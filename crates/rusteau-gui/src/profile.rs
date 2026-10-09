//! Soil profile editor.
//!
//! The selected nodal quantity is drawn as a coloured column next to its profile curve. Node ranges are selected
//! with the mouse (drag; click on a layer selects the layer) or by depth, and the selection is edited: material,
//! sub-region, or a continuous quantity as constant, linear, hydrostatic equilibrium (initial pressure head) or root
//! distribution function. Observation nodes and fixed points are set by clicking; the discretization (depth, number
//! of nodes, fixed points with relative element sizes) regenerates the nodes and interpolates the nodal values. The
//! table view lists all nodal values.

use crate::util::*;
use crate::{App, Page};
use egui::{Align2, Color32, ComboBox, Pos2, RichText, Stroke, Ui, Vec2b};
use egui_extras::{Column, TableBuilder};
use egui_plot::{GridInput, GridMark, Line, LineStyle, MarkerShape, Plot, PlotPoint, PlotPoints, Points, Polygon, Text};
use rusteau_core::grid::generate_depths;
use rusteau_core::*;

/// Nodal quantity shown and edited.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Qty {
    Material,
    SubRegion,
    Init,
    Root,
    ScaleH,
    ScaleK,
    ScaleTh,
    Temp,
    Conc(usize),
    Sorb(usize),
}

impl Qty {
    pub fn categorical(self) -> bool {
        matches!(self, Qty::Material | Qty::SubRegion)
    }
    pub fn get(self, n: &ProfileNode) -> f64 {
        match self {
            Qty::Material => n.mat as f64,
            Qty::SubRegion => n.layer as f64,
            Qty::Init => n.h,
            Qty::Root => n.beta,
            Qty::ScaleH => n.ah,
            Qty::ScaleK => n.ak,
            Qty::ScaleTh => n.ath,
            Qty::Temp => n.temp,
            Qty::Conc(j) => n.conc.get(j).copied().unwrap_or(0.0),
            Qty::Sorb(j) => n.sorb.get(j).copied().unwrap_or(0.0),
        }
    }
    pub fn set(self, n: &mut ProfileNode, v: f64) {
        match self {
            Qty::Material => n.mat = v.round().max(1.0) as usize,
            Qty::SubRegion => n.layer = v.round().max(1.0) as usize,
            Qty::Init => n.h = v,
            Qty::Root => n.beta = v.max(0.0),
            Qty::ScaleH => n.ah = v,
            Qty::ScaleK => n.ak = v,
            Qty::ScaleTh => n.ath = v,
            Qty::Temp => n.temp = v,
            Qty::Conc(j) => {
                if n.conc.len() <= j {
                    n.conc.resize(j + 1, 0.0);
                }
                n.conc[j] = v;
            }
            Qty::Sorb(j) => {
                if n.sorb.len() <= j {
                    n.sorb.resize(j + 1, 0.0);
                }
                n.sorb[j] = v;
            }
        }
    }
    /// Short name (quantity buttons)
    pub fn short(self, p: &Project) -> String {
        match self {
            Qty::Material => "Material".into(),
            Qty::SubRegion => "Sub-regions".into(),
            Qty::Init => if p.water.init_in_water_content { "Initial θ".into() } else { "Initial h".into() },
            Qty::Root => "Root distribution".into(),
            Qty::ScaleH => "Scaling αh".into(),
            Qty::ScaleK => "Scaling αK".into(),
            Qty::ScaleTh => "Scaling αθ".into(),
            Qty::Temp => "Temperature".into(),
            Qty::Conc(j) => format!("Conc. {}", j + 1),
            Qty::Sorb(j) => format!("Non-eq. conc. {}", j + 1),
        }
    }
    pub fn unit(self, p: &Project) -> String {
        match self {
            Qty::Init if !p.water.init_in_water_content => p.units.length_str().to_string(),
            Qty::Temp => "°C".into(),
            Qty::Conc(_) => conc_unit(p),
            Qty::Sorb(_) => {
                if p.solute.materials.iter().any(|m| m.th_immobile > 0.0) {
                    conc_unit(p)
                } else {
                    let m = p.units.mass.trim();
                    format!("{}/g", if m.is_empty() || m == "-" { "M" } else { m })
                }
            }
            _ => "–".into(),
        }
    }
    /// Full name with unit (axis label)
    pub fn label(self, p: &Project) -> String {
        let name = match self {
            Qty::Material => "Material".to_string(),
            Qty::SubRegion => "Sub-region (mass balance)".to_string(),
            Qty::Init => if p.water.init_in_water_content { "Initial water content θ".into() } else { "Initial pressure head h".into() },
            Qty::Root => "Root distribution b(x)".into(),
            Qty::ScaleH => "Scaling factor αh (pressure head)".into(),
            Qty::ScaleK => "Scaling factor αK (conductivity)".into(),
            Qty::ScaleTh => "Scaling factor αθ (water content)".into(),
            Qty::Temp => "Initial temperature".into(),
            Qty::Conc(j) => format!("Initial concentration {}", j + 1),
            Qty::Sorb(j) => format!("Initial non-equilibrium concentration {}", j + 1),
        };
        format!("{name} ({})", self.unit(p))
    }
}

/// What a mouse click / drag in the profile does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Obs,
    Fixed,
}

/// How values are set in the selection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dist {
    Constant,
    Linear,
    /// Hydrostatic equilibrium from the lowest selected node (initial pressure head)
    Equilibrium,
    /// Hoffman & van Genuchten (1983) root distribution
    HoffmanVG,
    /// Vrugt et al. (2001) root distribution
    Vrugt,
}

pub struct ProfEdit {
    pub qty: Qty,
    pub tool: Tool,
    pub table: bool,
    pub table_sel_only: bool,
    /// Selected nodes (0-based, inclusive; top <= bottom)
    pub sel: (usize, usize),
    drag_anchor: Option<usize>,
    fill_key: Option<(Qty, usize, usize)>,
    pub dist: Dist,
    pub v_top: f64,
    pub v_bot: f64,
    /// Material and sub-region to assign (1-based)
    pub mat: usize,
    pub sub: usize,
    /// Root distribution: rooting depth (Hoffman & van Genuchten) / maximum rooting depth (Vrugt)
    pub root_depth: f64,
    /// Vrugt: depth of maximum intensity and shape parameter
    pub root_zstar: f64,
    pub root_pz: f64,
    pub obs_depth: f64,
    pub fixed_depth: f64,
    /// Discretization being edited (applied with "Generate nodes")
    pub g_depth: f64,
    pub g_nodes: usize,
    pub g_fixed: Vec<FixedPoint>,
    pub g_dirty: bool,
    pub msg: String,
    /// Pixels per length unit of the last drawn profile (for label and pick distances)
    px_per_unit: f64,
    /// Selection the table was last scrolled to
    table_scrolled: Option<(usize, usize)>,
    /// Print time of the results used as initial condition
    pub res_idx: usize,
}

impl Default for ProfEdit {
    fn default() -> Self {
        ProfEdit {
            qty: Qty::Material,
            tool: Tool::Select,
            table: false,
            table_sel_only: false,
            sel: (0, usize::MAX),
            drag_anchor: None,
            fill_key: None,
            dist: Dist::Constant,
            v_top: 0.0,
            v_bot: 0.0,
            mat: 1,
            sub: 1,
            root_depth: 0.0,
            root_zstar: 0.0,
            root_pz: 1.0,
            obs_depth: 0.0,
            fixed_depth: 0.0,
            g_depth: 100.0,
            g_nodes: 101,
            g_fixed: vec![],
            g_dirty: false,
            msg: String::new(),
            px_per_unit: 0.0,
            table_scrolled: None,
            res_idx: usize::MAX,
        }
    }
}

impl ProfEdit {
    /// Forget the selection and the edited discretization (new project).
    pub fn reset(&mut self) {
        let keep = (self.qty, self.tool, self.table);
        *self = ProfEdit::default();
        (self.qty, self.tool, self.table) = keep;
    }
}

/// Viridis colour map (perceptually uniform, readable in both themes), t in [0, 1].
fn viridis(t: f64) -> Color32 {
    const S: [(f64, [f64; 3]); 5] = [
        (0.0, [68.0, 1.0, 84.0]),
        (0.25, [59.0, 82.0, 139.0]),
        (0.5, [33.0, 145.0, 140.0]),
        (0.75, [94.0, 201.0, 98.0]),
        (1.0, [253.0, 231.0, 37.0]),
    ];
    let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.5 };
    let k = S.iter().position(|s| s.0 >= t).unwrap_or(4).max(1);
    let (a, b) = (S[k - 1], S[k]);
    let f = (t - a.0) / (b.0 - a.0);
    let c = |i: usize| (a.1[i] + (b.1[i] - a.1[i]) * f).round() as u8;
    Color32::from_rgb(c(0), c(1), c(2))
}

fn sub_color(k: usize) -> Color32 {
    PALETTE[(k.max(1) + 3) % PALETTE.len()]
}

const OBS_COLOR: Color32 = Color32::from_rgb(235, 70, 70);
const FIXED_COLOR: Color32 = Color32::from_rgb(220, 60, 220);
const LADDER_COLOR: Color32 = Color32::from_rgb(70, 175, 95);

/// "Nice" step (1, 2, 2.5, 5 × 10^k) not smaller than `x`.
fn nice_step(x: f64) -> f64 {
    if !(x > 0.0 && x.is_finite()) {
        return 1.0;
    }
    let e = 10f64.powf(x.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if m * e >= x * 0.999 {
            return m * e;
        }
    }
    10.0 * e
}

/// Value range of a quantity, widened when constant.
fn value_range(vals: &[f64]) -> (f64, f64) {
    let (mut lo, mut hi) = vals.iter().filter(|v| v.is_finite()).fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    if lo > hi {
        return (0.0, 1.0);
    }
    if hi - lo <= 1e-12 * lo.abs().max(hi.abs()).max(1e-300) {
        let w = (lo.abs() * 0.1).max(if lo == 0.0 { 1.0 } else { 1e-12 });
        lo -= w;
        hi += w;
    }
    (lo, hi)
}

/// Contiguous runs of equal value (categorical quantities): (first node, last node, value).
fn runs(vals: &[f64]) -> Vec<(usize, usize, f64)> {
    let mut out: Vec<(usize, usize, f64)> = vec![];
    for (i, &v) in vals.iter().enumerate() {
        match out.last_mut() {
            Some(r) if r.2 == v => r.1 = i,
            _ => out.push((i, i, v)),
        }
    }
    out
}

impl App {
    fn qty_list(&self) -> Vec<Qty> {
        let p = &self.prj;
        let mut q = vec![Qty::Material, Qty::SubRegion, Qty::Init];
        if p.processes.root_water_uptake {
            q.push(Qty::Root);
        }
        q.extend([Qty::ScaleH, Qty::ScaleK, Qty::ScaleTh]);
        if p.processes.heat || p.processes.solute {
            q.push(Qty::Temp);
        }
        if p.processes.solute {
            let ns = p.solute.species.len();
            q.extend((0..ns).map(Qty::Conc));
            if !p.processes.equilibrium_adsorption {
                q.extend((0..ns).map(Qty::Sorb));
            }
        }
        q
    }

    fn mat_name(&self, m: usize) -> String {
        format!("{} {}", m, self.prj.water.materials.get(m.wrapping_sub(1)).map(|x| x.name.as_str()).unwrap_or(""))
    }

    pub fn page_profile(&mut self, ui: &mut Ui, ch: &mut bool) {
        self.sync();
        let n = self.prj.profile.nodes.len();
        if n < 2 {
            page_title(ui, "Soil profile");
            ui.label("The profile has fewer than two nodes.");
            return;
        }
        // keep the editor state consistent with the project
        if self.pe.sel.1 >= n || self.pe.sel.0 > self.pe.sel.1 {
            self.pe.sel = (0, n - 1);
        }
        if !self.pe.g_dirty {
            self.pe.g_depth = self.prj.profile.depth();
            self.pe.g_nodes = n;
            self.pe.g_fixed = self.prj.profile.fixed_points_or_ends();
        }
        let qtys = self.qty_list();
        if !qtys.contains(&self.pe.qty) {
            self.pe.qty = Qty::Material;
        }
        let q = self.pe.qty;
        let valid = match self.pe.dist {
            Dist::Equilibrium => q == Qty::Init && !self.prj.water.init_in_water_content,
            Dist::HoffmanVG | Dist::Vrugt => q == Qty::Root,
            _ => true,
        };
        if !valid {
            self.pe.dist = Dist::Constant;
        }
        // values of the edit fields follow the selection
        let key = (q, self.pe.sel.0, self.pe.sel.1);
        if self.pe.fill_key != Some(key) {
            let nodes = &self.prj.profile.nodes;
            self.pe.v_top = q.get(&nodes[self.pe.sel.0]);
            self.pe.v_bot = q.get(&nodes[self.pe.sel.1]);
            self.pe.mat = nodes[self.pe.sel.0].mat;
            self.pe.sub = nodes[self.pe.sel.0].layer;
            if self.pe.root_depth <= 0.0 {
                let d0 = nodes[0].depth;
                let deepest = nodes.iter().filter(|x| x.beta > 0.0).map(|x| x.depth - d0).fold(0.0, f64::max);
                self.pe.root_depth = if deepest > 0.0 { deepest } else { 0.5 * self.prj.profile.depth() };
                self.pe.root_zstar = 0.2 * self.pe.root_depth;
            }
            self.pe.fill_key = Some(key);
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A)) && !ui.ctx().wants_keyboard_input() {
            self.pe.sel = (0, n - 1);
        }

        let lu = self.prj.units.length_str().to_string();
        ui.horizontal(|ui| {
            page_title(ui, "Soil profile");
            ui.add_space(16.0);
            ui.selectable_value(&mut self.pe.table, false, "Graphical");
            ui.selectable_value(&mut self.pe.table, true, "Table");
            ui.add_space(16.0);
            let nodes = &self.prj.profile.nodes;
            let dz: Vec<f64> = nodes.windows(2).map(|w| w[1].depth - w[0].depth).collect();
            let (dmin, dmax) = dz.iter().fold((f64::MAX, 0.0f64), |(a, b), &d| (a.min(d), b.max(d)));
            ui.label(
                RichText::new(format!(
                    "{} nodes · depth {} {lu} · Δz {}–{} {lu} · {} material(s) · {} observation node(s)",
                    n,
                    sig(self.prj.profile.depth()),
                    sig(dmin),
                    sig(dmax),
                    self.prj.water.materials.len(),
                    self.prj.profile.observation_nodes.len()
                ))
                .weak(),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Quantity:");
            for qq in &qtys {
                if ui.selectable_label(self.pe.qty == *qq, qq.short(&self.prj)).on_hover_text(qq.label(&self.prj)).clicked() {
                    self.pe.qty = *qq;
                }
            }
        });
        if self.pe.table {
            ui.separator();
            self.profile_table(ui, ch);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Mouse:");
            ui.selectable_value(&mut self.pe.tool, Tool::Select, "▭ Select nodes")
                .on_hover_text("Drag to select a range of nodes; click selects one node (a whole layer for material and sub-region); Shift+click extends the selection; Ctrl+A selects all");
            ui.selectable_value(&mut self.pe.tool, Tool::Obs, "◉ Observation nodes").on_hover_text("Click to add or remove an observation node");
            ui.selectable_value(&mut self.pe.tool, Tool::Fixed, "◆ Fixed points")
                .on_hover_text("Click to add or remove a fixed point of the discretization (applied with “Generate nodes”)");
            ui.label(RichText::new("Wheel: scroll · Ctrl+wheel: zoom · double-click: whole profile").weak());
        });
        ui.separator();
        let avail = ui.available_size();
        let ctrl_w = (avail.x * 0.40).clamp(330.0, 460.0);
        let plot_w = (avail.x - ctrl_w - 24.0).max(260.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(egui::vec2(plot_w, avail.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                self.profile_plot(ui, ch, avail.y - 34.0);
            });
            ui.separator();
            ui.allocate_ui_with_layout(egui::vec2(ctrl_w, avail.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("prof_ctl").auto_shrink([false, false]).show(ui, |ui| {
                    self.profile_controls(ui, ch);
                });
            });
        });
    }

    /// Colour scale or legend above the profile.
    fn profile_legend(&self, ui: &mut Ui, lo: f64, hi: f64) {
        let q = self.pe.qty;
        let p = &self.prj;
        if q.categorical() {
            ui.horizontal_wrapped(|ui| {
                let mut vals: Vec<usize> = p.profile.nodes.iter().map(|n| q.get(n) as usize).collect();
                vals.sort_unstable();
                vals.dedup();
                for v in vals {
                    let (c, t) = if q == Qty::Material { (soil_mat_color(p, v), self.mat_name(v)) } else { (sub_color(v), format!("Sub-region {v}")) };
                    ui.colored_label(c, "■");
                    ui.label(t);
                    ui.add_space(8.0);
                }
            });
            return;
        }
        ui.horizontal(|ui| {
            ui.label(sig(lo));
            let w = (ui.available_width() - 80.0).clamp(80.0, 360.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 12.0), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let k = 64;
            for i in 0..k {
                let x0 = rect.left() + rect.width() * i as f32 / k as f32;
                let x1 = rect.left() + rect.width() * (i + 1) as f32 / k as f32 + 0.5;
                painter.rect_filled(egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom())), 0.0, viridis((i as f64 + 0.5) / k as f64));
            }
            ui.label(sig(hi));
            let u = q.unit(p);
            if u != "–" {
                ui.label(RichText::new(u).weak());
            }
        });
    }

    fn profile_plot(&mut self, ui: &mut Ui, ch: &mut bool, height: f32) {
        let q = self.pe.qty;
        let cat = q.categorical();
        let p = &self.prj;
        let nodes = &p.profile.nodes;
        let n = nodes.len();
        let d: Vec<f64> = nodes.iter().map(|x| x.depth).collect();
        let vals: Vec<f64> = nodes.iter().map(|x| q.get(x)).collect();
        let (lo, hi) = value_range(&vals);
        self.profile_legend(ui, lo, hi);
        let norm = |v: f64| (v - lo) / (hi - lo);
        // node bands: node i owns the depths between the midpoints to its neighbours
        let band = |i: usize| -> (f64, f64) {
            let top = if i == 0 { d[0] } else { 0.5 * (d[i - 1] + d[i]) };
            let bot = if i + 1 == n { d[n - 1] } else { 0.5 * (d[i] + d[i + 1]) };
            (top, bot)
        };
        let color_of = |i: usize| -> Color32 {
            match q {
                Qty::Material => soil_mat_color(p, nodes[i].mat),
                Qty::SubRegion => sub_color(nodes[i].layer),
                _ => viridis((norm(vals[i]) * 63.0).round() / 63.0),
            }
        };
        let mut bands: Vec<(f64, f64, Color32)> = vec![];
        for i in 0..n {
            let (t, b) = band(i);
            let c = color_of(i);
            match bands.last_mut() {
                Some(l) if l.2 == c => l.1 = b,
                _ => bands.push((t, b, c)),
            }
        }
        let (cx0, cx1) = if cat { (-0.36, 1.0) } else { (-0.36, -0.06) };
        let sel = self.pe.sel;
        let (st, sb) = (band(sel.0).0, band(sel.1).1);
        let visuals = ui.visuals().clone();
        let sel_fill = visuals.selection.bg_fill.gamma_multiply(0.22);
        let sel_stroke = Stroke::new(1.5_f32, visuals.selection.bg_fill.gamma_multiply(1.6));
        let curve_col = visuals.strong_text_color();
        let lu = p.units.length_str().to_string();
        let labels: Vec<(f64, String)> = if cat {
            runs(&vals)
                .into_iter()
                .filter(|r| (band(r.1).1 - band(r.0).0) * self.pe.px_per_unit > 16.0 || self.pe.px_per_unit == 0.0)
                .map(|r| (0.5 * (band(r.0).0 + band(r.1).1), if q == Qty::Material { self.mat_name(r.2 as usize) } else { format!("Sub-region {}", r.2) }))
                .collect()
        } else {
            vec![]
        };
        let obs: Vec<(usize, f64)> = p.profile.observation_nodes.iter().enumerate().filter_map(|(k, &o)| d.get(o.wrapping_sub(1)).map(|&dd| (k + 1, dd))).collect();
        let fixed: Vec<f64> = self.pe.g_fixed.iter().map(|f| f.depth).collect();
        let fixed_pending = self.pe.g_dirty;
        let show_pts = n <= 150;

        let mut plot = Plot::new("prof_editor")
            .height(height.max(300.0))
            .allow_drag(false)
            .allow_boxed_zoom(false)
            .allow_zoom(Vec2b::new(false, true))
            .allow_scroll(Vec2b::new(false, true))
            .show_x(false)
            .show_y(false)
            .include_x(-0.68)
            .include_x(1.04)
            .include_y(-d[0])
            .include_y(-d[n - 1])
            .set_margin_fraction(egui::vec2(0.0, 0.02))
            .y_axis_label(format!("depth ({lu})"))
            .y_axis_formatter(|m, _| {
                let v = -m.value;
                sig(if v.abs() < 1e-12 { 0.0 } else { v })
            });
        if cat {
            plot = plot.show_axes([false, true]).show_grid([false, true]);
        } else {
            plot = plot
                .x_axis_label(q.label(p))
                .x_grid_spacer(move |_inp: GridInput| {
                    let span = hi - lo;
                    let step = nice_step(span / 4.0);
                    let mut marks = vec![];
                    let mut v = (lo / step).ceil() * step;
                    while v <= hi + 1e-9 * span {
                        marks.push(GridMark { value: (v - lo) / span, step_size: step / span });
                        v += step;
                    }
                    marks
                })
                .x_axis_formatter(move |m, _| {
                    if m.value < -1e-6 || m.value > 1.0 + 1e-6 {
                        String::new()
                    } else {
                        let v = lo + m.value * (hi - lo);
                        sig(if v.abs() < 1e-12 * (hi - lo).abs() { 0.0 } else { v })
                    }
                });
        }
        let resp = plot.show(ui, |pu| {
            // coloured column
            for (t, b, c) in &bands {
                pu.polygon(Polygon::new(PlotPoints::new(vec![[cx0, -t], [cx1, -t], [cx1, -b], [cx0, -b]])).fill_color(*c).stroke(Stroke::new(0.0_f32, *c)).allow_hover(false));
            }
            // node ladder
            let mut zig: Vec<[f64; 2]> = Vec::with_capacity(2 * n);
            for (i, &z) in d.iter().enumerate() {
                if i % 2 == 0 {
                    zig.push([-0.44, -z]);
                    zig.push([-0.38, -z]);
                } else {
                    zig.push([-0.38, -z]);
                    zig.push([-0.44, -z]);
                }
            }
            pu.line(Line::new(PlotPoints::new(zig)).color(LADDER_COLOR).width(1.0_f32).allow_hover(false));
            for x in [-0.44, -0.38] {
                pu.line(Line::new(PlotPoints::new(vec![[x, -d[0]], [x, -d[n - 1]]])).color(LADDER_COLOR).width(1.0_f32).allow_hover(false));
            }
            // value curve
            if !cat {
                let pts: Vec<[f64; 2]> = (0..n).map(|i| [norm(vals[i]), -d[i]]).collect();
                pu.line(Line::new(PlotPoints::new(pts.clone())).color(curve_col).width(2.0_f32).allow_hover(false));
                if show_pts {
                    pu.points(Points::new(pts).color(curve_col).radius(2.0_f32).allow_hover(false));
                }
            }
            for (y, t) in &labels {
                pu.text(Text::new(PlotPoint::new(0.5 * (cx0 + cx1), -y), RichText::new(t).size(15.0).strong().color(Color32::WHITE)).allow_hover(false));
            }
            // selection
            pu.polygon(
                Polygon::new(PlotPoints::new(vec![[-0.66, -st], [1.03, -st], [1.03, -sb], [-0.66, -sb]]))
                    .fill_color(sel_fill)
                    .stroke(sel_stroke)
                    .allow_hover(false),
            );
            // observation nodes
            for (k, z) in &obs {
                pu.points(Points::new(vec![[-0.55, -z]]).shape(MarkerShape::Right).radius(6.0_f32).filled(true).color(OBS_COLOR).allow_hover(false));
                pu.text(Text::new(PlotPoint::new(-0.59, -z), RichText::new(k.to_string()).color(OBS_COLOR).size(14.0).strong()).anchor(Align2::RIGHT_CENTER).allow_hover(false));
                if !cat {
                    pu.line(Line::new(PlotPoints::new(vec![[0.0, -z], [1.0, -z]])).color(OBS_COLOR.gamma_multiply(0.6)).style(LineStyle::dotted_dense()).allow_hover(false));
                }
            }
            // fixed points
            for z in &fixed {
                pu.points(Points::new(vec![[-0.48, -z]]).shape(MarkerShape::Right).radius(5.0_f32).filled(!fixed_pending).color(FIXED_COLOR).allow_hover(false));
            }
        });
        let tr = resp.transform;
        let frame_h = tr.frame().height() as f64;
        let b = tr.bounds();
        let span_y = (b.max()[1] - b.min()[1]).abs();
        if span_y > 0.0 {
            self.pe.px_per_unit = frame_h / span_y;
        }
        let response = resp.response;
        let depth_at = |pos: Pos2| -tr.value_from_position(pos).y;
        let nearest = |z: f64| -> usize {
            let mut best = 0;
            let mut bd = f64::MAX;
            for (i, &x) in d.iter().enumerate() {
                if (x - z).abs() < bd {
                    bd = (x - z).abs();
                    best = i;
                }
            }
            best
        };
        match self.pe.tool {
            Tool::Select => {
                if response.drag_started() {
                    // the drag is recognised after the pointer moved: start at the press position
                    if let Some(pos) = ui.input(|i| i.pointer.press_origin()).or(response.interact_pointer_pos()) {
                        let k = nearest(depth_at(pos));
                        self.pe.drag_anchor = Some(k);
                        self.pe.sel = (k, k);
                    }
                }
                if response.dragged() {
                    if let (Some(a), Some(pos)) = (self.pe.drag_anchor, response.interact_pointer_pos()) {
                        let k = nearest(depth_at(pos));
                        self.pe.sel = (a.min(k), a.max(k));
                    }
                }
                if response.drag_stopped() {
                    self.pe.drag_anchor = None;
                }
                if response.clicked() && !response.double_clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let k = nearest(depth_at(pos));
                        if ui.input(|i| i.modifiers.shift) {
                            let (a, b) = self.pe.sel;
                            self.pe.sel = (a.min(k), b.max(k));
                        } else if cat {
                            let r = runs(&vals).into_iter().find(|r| r.0 <= k && k <= r.1).unwrap_or((k, k, 0.0));
                            self.pe.sel = (r.0, r.1);
                        } else {
                            self.pe.sel = (k, k);
                        }
                    }
                }
            }
            Tool::Obs => {
                if response.clicked() && !response.double_clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let k = nearest(depth_at(pos)) + 1;
                        let obs = &mut self.prj.profile.observation_nodes;
                        if let Some(i) = obs.iter().position(|&o| o == k) {
                            obs.remove(i);
                        } else if obs.len() < 100 {
                            obs.push(k);
                            obs.sort_unstable();
                        }
                        *ch = true;
                    }
                }
            }
            Tool::Fixed => {
                if response.clicked() && !response.double_clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let z = depth_at(pos);
                        let pick = 8.0 / self.pe.px_per_unit.max(1e-9);
                        let total = self.pe.g_depth;
                        let hit = self.pe.g_fixed.iter().position(|f| (f.depth - z).abs() < pick);
                        match hit {
                            Some(i) => {
                                let f = self.pe.g_fixed[i].depth;
                                if f > 1e-9 * total && f < total * (1.0 - 1e-9) {
                                    self.pe.g_fixed.remove(i);
                                    self.pe.g_dirty = true;
                                }
                            }
                            None => {
                                let zz = d[nearest(z)];
                                self.add_fixed_point(zz);
                            }
                        }
                    }
                }
            }
        }
        // hover information
        if let Some(pos) = response.hover_pos() {
            let k = nearest(depth_at(pos));
            let nd = &self.prj.profile.nodes[k];
            let mut t = format!("node {} · depth {} {}", k + 1, sig(nd.depth), lu);
            t.push_str(&format!("\n{}", self.mat_name(nd.mat)));
            if !cat {
                t.push_str(&format!("\n{} = {}", q.short(&self.prj), sig(q.get(nd))));
            } else if q == Qty::SubRegion {
                t.push_str(&format!("\nsub-region {}", nd.layer));
            }
            if let Some((o, _)) = obs.iter().find(|(_, z)| (*z - nd.depth).abs() < 1e-12) {
                t.push_str(&format!("\nobservation node {o}"));
            }
            response.on_hover_text_at_pointer(t);
        }
    }

    /// Adds a fixed point at depth `z` (element sizes interpolated from the neighbouring fixed points).
    fn add_fixed_point(&mut self, z: f64) {
        let total = self.pe.g_depth;
        if !(z >= 0.0 && z <= total) {
            return;
        }
        let fx = &mut self.pe.g_fixed;
        if fx.iter().any(|f| (f.depth - z).abs() <= 1e-9 * total.max(1e-12)) {
            return;
        }
        fx.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        let above = fx.iter().rev().find(|f| f.depth < z).copied();
        let below = fx.iter().find(|f| f.depth > z).copied();
        let s = match (above, below) {
            (Some(a), Some(b)) => {
                let f = (z - a.depth) / (b.depth - a.depth);
                a.size_below + (b.size_above - a.size_below) * f
            }
            (Some(a), None) => a.size_below,
            (None, Some(b)) => b.size_above,
            _ => 1.0,
        };
        fx.push(FixedPoint { depth: z, size_above: s, size_below: s });
        fx.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        self.pe.g_dirty = true;
    }

    /// Applies the edited discretization: new nodes from depth, number of nodes and fixed points.
    fn generate_nodes(&mut self, ch: &mut bool) {
        match generate_depths(&self.pe.g_fixed, self.pe.g_depth, self.pe.g_nodes) {
            Ok(z) => {
                self.prj.profile.regrid(&z);
                let depth = self.pe.g_depth;
                let mut fx: Vec<FixedPoint> = self.pe.g_fixed.iter().filter(|f| f.depth >= 0.0 && f.depth <= depth).copied().collect();
                fx.sort_by(|a, b| a.depth.total_cmp(&b.depth));
                if fx.first().is_none_or(|f| f.depth > 1e-9 * depth) {
                    fx.insert(0, FixedPoint::new(0.0));
                }
                if fx.last().is_none_or(|f| f.depth < depth * (1.0 - 1e-9)) {
                    fx.push(FixedPoint::new(depth));
                }
                self.prj.profile.fixed_points = fx;
                self.pe.g_dirty = false;
                self.pe.sel = (0, z.len() - 1);
                self.pe.fill_key = None;
                self.pe.msg = format!("{} nodes generated.", z.len());
                *ch = true;
            }
            Err(e) => self.pe.msg = e,
        }
    }

    /// Sets the selected quantity in the selected nodes.
    fn apply_values(&mut self, ch: &mut bool) {
        let q = self.pe.qty;
        let (a, b) = self.pe.sel;
        let cos_a = self.prj.cos_alpha;
        let nodes = &mut self.prj.profile.nodes;
        let (da, db) = (nodes[a].depth, nodes[b].depth);
        let d0 = nodes[0].depth;
        let e = &self.pe;
        for nd in nodes[a..=b].iter_mut() {
            let z = nd.depth;
            let f = if db > da { (z - da) / (db - da) } else { 0.0 };
            let v = match q {
                Qty::Material => e.mat as f64,
                Qty::SubRegion => e.sub as f64,
                _ => match e.dist {
                    Dist::Constant => e.v_top,
                    Dist::Linear => e.v_top + (e.v_bot - e.v_top) * f,
                    Dist::Equilibrium => e.v_bot - (db - z) * cos_a,
                    Dist::HoffmanVG => {
                        let lr = e.root_depth;
                        let s = z - d0;
                        if lr <= 0.0 || s > lr {
                            0.0
                        } else if s < 0.2 * lr {
                            1.666667 / lr
                        } else {
                            2.083333 / lr * (1.0 - s / lr)
                        }
                    }
                    Dist::Vrugt => {
                        let (zm, zs, pz) = (e.root_depth, e.root_zstar, e.root_pz);
                        let s = z - d0;
                        if zm <= 0.0 || s > zm {
                            0.0
                        } else {
                            let p = if s > zs { 1.0 } else { pz };
                            (1.0 - s / zm) * (-(p / zm) * (zs - s).abs()).exp()
                        }
                    }
                },
            };
            q.set(nd, v);
        }
        self.pe.fill_key = None;
        *ch = true;
    }

    fn profile_controls(&mut self, ui: &mut Ui, ch: &mut bool) {
        let q = self.pe.qty;
        let lu = self.prj.units.length_str().to_string();
        let n = self.prj.profile.nodes.len();

        // ---- selection
        section(ui, "Selection");
        let (a, b) = self.pe.sel;
        let (da, db) = (self.prj.profile.nodes[a].depth, self.prj.profile.nodes[b].depth);
        ui.label(format!("Nodes {}–{} ({} nodes), depth {}–{} {lu}", a + 1, b + 1, b - a + 1, sig(da), sig(db)));
        ui.horizontal(|ui| {
            let (mut fa, mut fb) = (da, db);
            ui.label("From");
            let ra = ui.add(egui::DragValue::new(&mut fa).speed(0.2).suffix(format!(" {lu}")));
            ui.label("to");
            let rb = ui.add(egui::DragValue::new(&mut fb).speed(0.2).suffix(format!(" {lu}")));
            if ra.changed() || rb.changed() {
                let ka = self.prj.profile.nearest_node(fa);
                let kb = self.prj.profile.nearest_node(fb);
                self.pe.sel = (ka.min(kb), ka.max(kb));
            }
            if ui.button("All").on_hover_text("Ctrl+A").clicked() {
                self.pe.sel = (0, n - 1);
            }
        });

        // ---- values
        section(ui, &format!("Set {} in the selection", q.short(&self.prj).to_lowercase()));
        match q {
            Qty::Material => {
                ui.horizontal(|ui| {
                    ui.label("Material");
                    let nm = self.prj.water.materials.len();
                    self.pe.mat = self.pe.mat.clamp(1, nm);
                    let names: Vec<(String, Color32)> = (1..=nm).map(|m| (self.mat_name(m), soil_mat_color(&self.prj, m))).collect();
                    ComboBox::from_id_salt("pe_mat").selected_text(names[self.pe.mat - 1].0.clone()).width(180.0).show_ui(ui, |ui| {
                        for (m, (name, col)) in names.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.colored_label(*col, "■");
                                ui.selectable_value(&mut self.pe.mat, m + 1, name);
                            });
                        }
                    });
                    if ui.button("Apply").clicked() {
                        self.apply_values(ch);
                    }
                });
                if ui.link("Add, remove or rename materials (soil hydraulic parameters) →").clicked() {
                    self.page = Page::SoilParams;
                }
            }
            Qty::SubRegion => {
                ui.horizontal(|ui| {
                    ui.label("Sub-region");
                    ui.add(egui::DragValue::new(&mut self.pe.sub).range(1..=100));
                    if ui.button("Apply").clicked() {
                        self.apply_values(ch);
                    }
                });
                if ui.button("Sub-regions = materials").on_hover_text("Set the sub-region of every node to its material number").clicked() {
                    for nd in self.prj.profile.nodes.iter_mut() {
                        nd.layer = nd.mat;
                    }
                    self.pe.fill_key = None;
                    *ch = true;
                }
                ui.label(RichText::new("Sub-regions only divide the water and solute balance (Balance.out); they do not change the simulation.").weak());
            }
            _ => self.value_editor(ui, ch),
        }

        // ---- observation nodes
        section(ui, &format!("Observation nodes ({})", self.prj.profile.observation_nodes.len()));
        let mut remove = None;
        egui::Grid::new("pe_obs").num_columns(4).striped(true).show(ui, |ui| {
            for (k, &o) in self.prj.profile.observation_nodes.iter().enumerate() {
                ui.colored_label(OBS_COLOR, format!("{}", k + 1));
                ui.label(format!("node {o}"));
                ui.label(format!("{} {lu}", self.prj.profile.nodes.get(o.wrapping_sub(1)).map(|x| sig(x.depth)).unwrap_or_default()));
                if ui.small_button("✖").on_hover_text("remove").clicked() {
                    remove = Some(k);
                }
                ui.end_row();
            }
        });
        if let Some(k) = remove {
            self.prj.profile.observation_nodes.remove(k);
            *ch = true;
        }
        ui.horizontal(|ui| {
            ui.label("Add at depth");
            ui.add(egui::DragValue::new(&mut self.pe.obs_depth).speed(0.2).suffix(format!(" {lu}")));
            if ui.button("Add").clicked() {
                let k = self.prj.profile.nearest_node(self.pe.obs_depth) + 1;
                let obs = &mut self.prj.profile.observation_nodes;
                if !obs.contains(&k) && obs.len() < 100 {
                    obs.push(k);
                    obs.sort_unstable();
                    *ch = true;
                }
            }
            if ui.add_enabled(!self.prj.profile.observation_nodes.is_empty(), egui::Button::new("Delete all")).clicked() {
                self.prj.profile.observation_nodes.clear();
                *ch = true;
            }
        });
        ui.label(RichText::new("Or choose the mouse tool “Observation nodes” and click in the profile.").weak());

        // ---- discretization
        section(ui, "Profile depth & discretization");
        egui::Grid::new("pe_geo").num_columns(2).show(ui, |ui| {
            ui.label("Profile depth");
            if ui.add(egui::DragValue::new(&mut self.pe.g_depth).speed(0.5).range(1e-6..=1e9).suffix(format!(" {lu}"))).changed() {
                self.pe.g_dirty = true;
            }
            ui.end_row();
            ui.label("Number of nodes");
            if ui.add(egui::DragValue::new(&mut self.pe.g_nodes).range(2..=1001)).changed() {
                self.pe.g_dirty = true;
            }
            ui.end_row();
        });
        ui.label("Fixed points are always nodes; the relative element size above and below each point sets the node density.");
        let total = self.pe.g_depth;
        let mut remove = None;
        let mut dirty = false;
        egui::Grid::new("pe_fixed").num_columns(4).striped(true).show(ui, |ui| {
            ui.strong(format!("depth ({lu})"));
            ui.strong("size above");
            ui.strong("size below");
            ui.label("");
            ui.end_row();
            let m = self.pe.g_fixed.len();
            for (i, f) in self.pe.g_fixed.iter_mut().enumerate() {
                let end = f.depth <= 1e-9 * total || f.depth >= total * (1.0 - 1e-9);
                if end {
                    ui.label(sig(f.depth));
                } else if ui.add(egui::DragValue::new(&mut f.depth).speed(0.2).range(0.0..=total)).changed() {
                    dirty = true;
                }
                if i > 0 {
                    dirty |= ui.add(egui::DragValue::new(&mut f.size_above).speed(0.05).range(1e-3..=1e3)).changed();
                } else {
                    ui.label("–");
                }
                if i + 1 < m {
                    dirty |= ui.add(egui::DragValue::new(&mut f.size_below).speed(0.05).range(1e-3..=1e3)).changed();
                } else {
                    ui.label("–");
                }
                if !end && ui.small_button("✖").on_hover_text("remove").clicked() {
                    remove = Some(i);
                } else if end {
                    ui.label("");
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove {
            self.pe.g_fixed.remove(i);
            dirty = true;
        }
        if dirty {
            self.pe.g_dirty = true;
            self.pe.g_fixed.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        }
        ui.horizontal(|ui| {
            ui.label("Add fixed point at");
            ui.add(egui::DragValue::new(&mut self.pe.fixed_depth).speed(0.2).range(0.0..=total).suffix(format!(" {lu}")));
            if ui.button("Add").clicked() {
                self.add_fixed_point(self.pe.fixed_depth);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Fixed points at material boundaries").on_hover_text("Adds a fixed point at the first node of every material change").clicked() {
                let nodes = &self.prj.profile.nodes;
                let z: Vec<f64> = nodes.windows(2).filter(|w| w[0].mat != w[1].mat).map(|w| w[1].depth).collect();
                for zz in z {
                    self.add_fixed_point(zz);
                }
            }
            if ui.button("Uniform").on_hover_text("All relative element sizes 1").clicked() {
                for f in self.pe.g_fixed.iter_mut() {
                    f.size_above = 1.0;
                    f.size_below = 1.0;
                }
                self.pe.g_dirty = true;
            }
        });
        ui.horizontal(|ui| {
            let gen = egui::Button::new(RichText::new("Generate nodes").strong());
            if ui.add_enabled(self.pe.g_dirty, gen).on_hover_text("New nodes from depth, number of nodes and fixed points; nodal values are interpolated, observation nodes keep their depth").clicked() {
                self.generate_nodes(ch);
            }
            if self.pe.g_dirty {
                if ui.button("Discard").clicked() {
                    self.pe.g_dirty = false;
                    self.pe.msg.clear();
                }
                ui.colored_label(ui.visuals().warn_fg_color, "changes not applied");
            }
        });
        if !self.pe.msg.is_empty() {
            ui.label(RichText::new(&self.pe.msg).weak());
        }
        egui::Grid::new("pe_cos").num_columns(2).show(ui, |ui| {
            field(ui, ch, "cos α (1 = vertical flow)", &mut self.prj.cos_alpha);
        });
    }

    /// Editor of a continuous quantity: constant, linear, hydrostatic equilibrium, root distribution functions.
    fn value_editor(&mut self, ui: &mut Ui, ch: &mut bool) {
        let q = self.pe.qty;
        let unit = q.unit(&self.prj);
        let lu = self.prj.units.length_str().to_string();
        let theta = self.prj.water.init_in_water_content;
        if q == Qty::Init && ui.checkbox(&mut self.prj.water.init_in_water_content, "Initial condition as water content θ").changed() {
            *ch = true;
        }
        ui.horizontal_wrapped(|ui| {
            ui.radio_value(&mut self.pe.dist, Dist::Constant, "Constant");
            ui.radio_value(&mut self.pe.dist, Dist::Linear, "Linear");
            if q == Qty::Init && !theta {
                ui.radio_value(&mut self.pe.dist, Dist::Equilibrium, "Hydrostatic equilibrium")
                    .on_hover_text("h = h(bottom) − (z(bottom) − z)·cos α from the lowest selected node");
            }
            if q == Qty::Root {
                ui.radio_value(&mut self.pe.dist, Dist::HoffmanVG, "Hoffman & van Genuchten (1983)");
                ui.radio_value(&mut self.pe.dist, Dist::Vrugt, "Vrugt et al. (2001)");
            }
        });
        let dv_u = |ui: &mut Ui, v: &mut f64, u: &str| {
            let speed = (v.abs() * 0.01).max(1e-3);
            let suffix = if u == "–" { String::new() } else { format!(" {u}") };
            ui.add(egui::DragValue::new(v).speed(speed).max_decimals(6).suffix(suffix));
        };
        egui::Grid::new("pe_val").num_columns(2).show(ui, |ui| match self.pe.dist {
            Dist::Constant => {
                ui.label("Value");
                dv_u(ui, &mut self.pe.v_top, &unit);
                ui.end_row();
            }
            Dist::Linear => {
                ui.label("Top of the selection");
                dv_u(ui, &mut self.pe.v_top, &unit);
                ui.end_row();
                ui.label("Bottom of the selection");
                dv_u(ui, &mut self.pe.v_bot, &unit);
                ui.end_row();
            }
            Dist::Equilibrium => {
                ui.label("h at the lowest selected node");
                dv_u(ui, &mut self.pe.v_bot, &unit);
                ui.end_row();
            }
            Dist::HoffmanVG => {
                ui.label("Rooting depth Lr");
                dv_u(ui, &mut self.pe.root_depth, &lu);
                ui.end_row();
            }
            Dist::Vrugt => {
                ui.label("Maximum rooting depth zm");
                dv_u(ui, &mut self.pe.root_depth, &lu);
                ui.end_row();
                ui.label("Depth of maximum intensity z*");
                dv_u(ui, &mut self.pe.root_zstar, &lu);
                ui.end_row();
                ui.label("Shape parameter pz");
                dv_u(ui, &mut self.pe.root_pz, "–");
                ui.end_row();
            }
        });
        if ui.button(RichText::new("Apply to selection").strong()).clicked() {
            self.apply_values(ch);
        }
        // initial condition from the results of the last run (any print time), interpolated in depth
        let from_results = matches!(q, Qty::Init | Qty::Temp | Qty::Conc(_));
        if let (true, Some(r)) = (from_results, self.results.as_ref()) {
            if !r.profiles.is_empty() {
                let np = r.profiles.len();
                if self.pe.res_idx >= np {
                    self.pe.res_idx = np - 1;
                }
                let tu = self.prj.units.time_str();
                let mut use_it = false;
                ui.horizontal(|ui| {
                    ui.label("Or take the results at");
                    ComboBox::from_id_salt("pe_res").selected_text(format!("t = {} {tu}", fmt_t(r.profiles[self.pe.res_idx].t))).show_ui(ui, |ui| {
                        for (k, p) in r.profiles.iter().enumerate() {
                            ui.selectable_value(&mut self.pe.res_idx, k, format!("t = {} {tu}", fmt_t(p.t)));
                        }
                    });
                    use_it = ui.button("Use").on_hover_text("Sets the selected nodes to the simulated values at this print time (interpolated in depth)").clicked();
                });
                if use_it {
                    let prof = &r.profiles[self.pe.res_idx];
                    let theta = self.prj.water.init_in_water_content;
                    let val = |n: &rusteau_core::output::NodeOut| -> f64 {
                        match q {
                            Qty::Init if theta => n.theta,
                            Qty::Init => n.h,
                            Qty::Temp => n.temp,
                            Qty::Conc(j) => n.conc.get(j).copied().unwrap_or(0.0),
                            _ => 0.0,
                        }
                    };
                    let pts: Vec<(f64, f64)> = prof.nodes.iter().map(|n| (n.depth, val(n))).collect();
                    let at = |z: f64| -> f64 {
                        if pts.is_empty() {
                            return 0.0;
                        }
                        if z <= pts[0].0 {
                            return pts[0].1;
                        }
                        for w in pts.windows(2) {
                            if z <= w[1].0 {
                                let f = if w[1].0 > w[0].0 { (z - w[0].0) / (w[1].0 - w[0].0) } else { 0.0 };
                                return w[0].1 + (w[1].1 - w[0].1) * f;
                            }
                        }
                        pts[pts.len() - 1].1
                    };
                    let (a, b) = self.pe.sel;
                    for nd in self.prj.profile.nodes[a..=b].iter_mut() {
                        let v = at(nd.depth);
                        q.set(nd, v);
                    }
                    self.pe.fill_key = None;
                    *ch = true;
                }
            }
        }
        let (a, b) = self.pe.sel;
        let vs: Vec<f64> = self.prj.profile.nodes[a..=b].iter().map(|x| q.get(x)).collect();
        let (lo, hi) = vs.iter().fold((f64::MAX, f64::MIN), |(l, h), &v| (l.min(v), h.max(v)));
        let u = if unit == "–" { String::new() } else { format!(" {unit}") };
        ui.label(RichText::new(format!("In the selection: {} … {}{u}", sig(lo), sig(hi))).weak());
        let note = match q {
            Qty::Root => "b(x) is normalised by the program; only relative values matter. Root functions are measured from the soil surface.",
            Qty::ScaleH | Qty::ScaleK | Qty::ScaleTh => "Scaling factors of the similar-media scaling of the hydraulic functions; 1 = no scaling.",
            Qty::Temp if !self.prj.processes.heat => "Used by temperature-dependent solute parameters.",
            Qty::Init if theta => "Values are water contents; switching the option does not convert them.",
            _ => "",
        };
        if !note.is_empty() {
            ui.label(RichText::new(note).weak());
        }
    }

    /// Nodal table (all values; the selection is highlighted, a row click selects the node).
    fn profile_table(&mut self, ui: &mut Ui, ch: &mut bool) {
        let p = &self.prj;
        let lu = p.units.length_str().to_string();
        let cu = conc_unit(p);
        let ns = if p.processes.solute { p.solute.species.len() } else { 0 };
        let noneq = ns > 0 && !p.processes.equilibrium_adsorption;
        let temp = p.processes.heat || p.processes.solute;
        let root = p.processes.root_water_uptake;
        let nm = p.water.materials.len();
        let n = p.profile.nodes.len();
        let mut cols: Vec<String> = vec![
            "node".into(),
            format!("depth ({lu})"),
            if p.water.init_in_water_content { "θ (–)".into() } else { format!("h ({lu})") },
            "material".into(),
            "sub-region".into(),
        ];
        if root {
            cols.push("b(x)".into());
        }
        cols.extend(["αh".to_string(), "αK".into(), "αθ".into()]);
        if temp {
            cols.push("T (°C)".into());
        }
        for j in 0..ns {
            cols.push(format!("c{} ({cu})", j + 1));
        }
        if noneq {
            for j in 0..ns {
                cols.push(format!("s{}", j + 1));
            }
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.pe.table_sel_only, "Only the selected nodes");
            let (a, b) = self.pe.sel;
            ui.label(RichText::new(format!("Selection: nodes {}–{}. Click a row to select it, Shift+click to extend.", a + 1, b + 1)).weak());
        });
        let (a, b) = self.pe.sel;
        let (first, count) = if self.pe.table_sel_only { (a, b - a + 1) } else { (0, n) };
        let shift = ui.input(|i| i.modifiers.shift);
        let max_h = ui.available_height() - 30.0;
        let row_h = ui.spacing().interact_size.y + 2.0 * ui.spacing().button_padding.y + 2.0;
        let mut tb = TableBuilder::new(ui).striped(true).sense(egui::Sense::click()).max_scroll_height(max_h).auto_shrink([false, false]);
        if self.pe.table_scrolled != Some((a, b)) {
            // show the selection when it changed (e.g. in the graphical view)
            self.pe.table_scrolled = Some((a, b));
            if !self.pe.table_sel_only {
                tb = tb.scroll_to_row(a, Some(egui::Align::TOP));
            }
        }
        for (i, _) in cols.iter().enumerate() {
            tb = tb.column(if i == 0 { Column::auto().at_least(44.0) } else { Column::auto().at_least(70.0) });
        }
        let mut clicked: Option<usize> = None;
        let nodes = &mut self.prj.profile.nodes;
        tb.header(22.0, |mut h| {
            for c in &cols {
                h.col(|ui| {
                    ui.strong(c);
                });
            }
        })
        .body(|body| {
            body.rows(row_h, count, |mut row| {
                let i = first + row.index();
                row.set_selected(i >= a && i <= b);
                let nd = &mut nodes[i];
                row.col(|ui| {
                    ui.label(format!("{}", i + 1));
                });
                row.col(|ui| {
                    ui.label(sig(nd.depth));
                });
                row.col(|ui| dv(ui, ch, &mut nd.h));
                row.col(|ui| {
                    if ui.add(egui::DragValue::new(&mut nd.mat).range(1..=nm)).changed() {
                        *ch = true;
                    }
                });
                row.col(|ui| {
                    if ui.add(egui::DragValue::new(&mut nd.layer).range(1..=100)).changed() {
                        *ch = true;
                    }
                });
                if root {
                    row.col(|ui| dv(ui, ch, &mut nd.beta));
                }
                row.col(|ui| dv(ui, ch, &mut nd.ah));
                row.col(|ui| dv(ui, ch, &mut nd.ak));
                row.col(|ui| dv(ui, ch, &mut nd.ath));
                if temp {
                    row.col(|ui| dv(ui, ch, &mut nd.temp));
                }
                for j in 0..ns {
                    if nd.conc.len() <= j {
                        nd.conc.resize(j + 1, 0.0);
                    }
                    row.col(|ui| dv(ui, ch, &mut nd.conc[j]));
                }
                if noneq {
                    for j in 0..ns {
                        if nd.sorb.len() <= j {
                            nd.sorb.resize(j + 1, 0.0);
                        }
                        row.col(|ui| dv(ui, ch, &mut nd.sorb[j]));
                    }
                }
                if row.response().clicked() {
                    clicked = Some(i);
                }
            });
        });
        if let Some(k) = clicked {
            self.pe.sel = if shift { (a.min(k), b.max(k)) } else { (k, k) };
            self.pe.table_scrolled = Some(self.pe.sel);
        }
    }
}
