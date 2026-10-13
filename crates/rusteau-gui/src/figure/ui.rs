//! The window of the figure editor: the settings of the figure, its panels, layers, axes and annotations on the left,
//! the preview on the right. The figures belong to the project; the runs kept for comparison belong to the session
//! (and can be saved as files).

use super::canvas;
use super::data::{self, ColInfo, DataCtx, SavedRun};
use super::render::{self, Rendered, PT_PER_MM};
use super::templates::{self, TemplateCtx};
use crate::files::{DialogResult, FileDialog, Mode, Purpose};
use crate::tips::TipExt;
use crate::util::sig;
use crate::App;
use egui::{CollapsingHeader, ComboBox, DragValue, RichText, Ui};
use rusteau_core::figure::{Anchor, Annotation, AnnotationKind, Axis, Dash, Figure, Geom, Layer, LegendPos, Marker, Palette, Panel, Source, Theme};
use std::hash::{Hash, Hasher};

/// State of the figure window.
#[derive(Default)]
pub struct FigState {
    pub open: bool,
    /// selected figure, panel, layer and annotation
    pub fig: usize,
    pub panel: usize,
    pub layer: usize,
    pub ann: usize,
    /// axis shown in the axis section: 0 x, 1 y, 2 right y
    pub axis: usize,
    /// clicks in the preview place the selected annotation
    pub place: bool,
    /// runs kept for comparison
    pub runs: Vec<SavedRun>,
    pub run_name: String,
    pub status: String,
    /// the help window is shown in this window
    pub help_here: bool,
    /// opens or closes the section "Figure" once (screenshot tour)
    pub fold: Option<bool>,
    dialog: Option<FileDialog>,
    /// kept run a save dialog is for
    save_run: usize,
    tex: Option<egui::TextureHandle>,
    key: u64,
    rendered: Option<Rendered>,
    last_hash: Option<u64>,
    hover: String,
}

impl FigState {
    /// A project was loaded or created: its figures are not changes.
    pub fn project_changed(&mut self) {
        self.last_hash = None;
        self.fig = 0;
        self.panel = 0;
        self.layer = 0;
        self.ann = 0;
        self.key = 0;
    }
}

/// Changes that need the whole application (applied after the window is drawn).
enum Act {
    New(usize),
    Duplicate,
    Delete,
    ExportPng,
    ExportSvg,
    CopyImage,
    ImportData,
    KeepRun(String),
    LoadRun,
    SaveRun(usize),
    RemoveRun(usize),
    Help,
}

const THEMES: [(Theme, &str); 4] = [(Theme::Grid, "Grid"), (Theme::Classic, "Classic (axis lines)"), (Theme::Minimal, "Minimal"), (Theme::Dark, "Dark")];
const PALETTES: [(Palette, &str); 4] = [(Palette::OkabeIto, "Okabe–Ito (colour-blind safe)"), (Palette::Tableau, "Tableau 10"), (Palette::Viridis, "Viridis (ordered series)"), (Palette::Grey, "Grey")];
const LEGENDS: [(LegendPos, &str); 7] = [
    (LegendPos::TopRight, "Top right"),
    (LegendPos::TopLeft, "Top left"),
    (LegendPos::BottomRight, "Bottom right"),
    (LegendPos::BottomLeft, "Bottom left"),
    (LegendPos::Right, "Right, outside"),
    (LegendPos::Bottom, "Below, outside"),
    (LegendPos::None, "None"),
];
const GEOMS: [(Geom, &str); 6] = [(Geom::Line, "Line"), (Geom::Points, "Points"), (Geom::LinePoints, "Line and points"), (Geom::Steps, "Steps"), (Geom::Area, "Area"), (Geom::Bars, "Bars")];
const DASHES: [(Dash, &str); 4] = [(Dash::Solid, "Solid"), (Dash::Dashed, "Dashed"), (Dash::Dotted, "Dotted"), (Dash::DashDot, "Dash-dot")];
const MARKERS: [(Marker, &str); 9] = [
    (Marker::Circle, "● Circle"),
    (Marker::Square, "■ Square"),
    (Marker::Triangle, "▲ Triangle"),
    (Marker::Diamond, "◆ Diamond"),
    (Marker::Cross, "× Cross"),
    (Marker::Plus, "+ Plus"),
    (Marker::OpenCircle, "○ Open circle"),
    (Marker::OpenSquare, "□ Open square"),
    (Marker::OpenTriangle, "△ Open triangle"),
];
const ANCHORS: [(Anchor, &str); 3] = [(Anchor::Left, "Left"), (Anchor::Center, "Centre"), (Anchor::Right, "Right")];
const KINDS: [(AnnotationKind, &str); 4] = [(AnnotationKind::Text, "Text"), (AnnotationKind::HLine, "Horizontal line"), (AnnotationKind::VLine, "Vertical line"), (AnnotationKind::Arrow, "Arrow")];
const SIZES: [(&str, f64, f64); 5] = [("1 column (85 mm)", 85.0, 60.0), ("1.5 columns (120 mm)", 120.0, 80.0), ("2 columns (170 mm)", 170.0, 105.0), ("Slide 16:9 (254 × 143 mm)", 254.0, 143.0), ("A4 landscape (270 × 180 mm)", 270.0, 180.0)];
const SYMBOLS: &str = "θ α β γ δ ε η κ λ μ ν π ρ σ τ φ χ ψ ω Δ Θ Λ Σ Φ Ψ Ω ° ± × · − ≤ ≥ ≈ ≠ √ ∂ ∇ ∞ ∑ ∫ → ← ↑ ↓ ‰ µ ² ³ ⁻¹ ⁻² ⁻³ ₀ ₁ ₂ ₃ ₛ ᵣ";

fn name_of<T: PartialEq + Copy>(list: &[(T, &'static str)], v: T) -> &'static str {
    list.iter().find(|x| x.0 == v).map(|x| x.1).unwrap_or("")
}

/// A combo box over a list of (value, name); returns whether the value changed.
fn combo<T: PartialEq + Copy>(ui: &mut Ui, id: &str, v: &mut T, list: &[(T, &'static str)], width: f32) -> egui::Response {
    let before = *v;
    let mut r = ComboBox::from_id_salt(id).width(width).selected_text(name_of(list, *v)).show_ui(ui, |ui| {
        for (x, n) in list {
            ui.selectable_value(v, *x, *n);
        }
    });
    if *v != before {
        r.response.mark_changed();
    }
    r.response
}

/// Label of a grid row with its description.
fn row(ui: &mut Ui, label: &str) {
    ui.label(label).tip(label);
}

fn speed(v: f64) -> f64 {
    (v.abs() * 0.01).max(1e-4)
}

fn num(ui: &mut Ui, v: &mut f64, lo: f64, hi: f64, suffix: &str) -> egui::Response {
    let s = speed(*v);
    ui.add(DragValue::new(v).range(lo..=hi).speed(s).max_decimals(6).suffix(suffix))
}

/// A coordinate: steps of 0.005 for relative coordinates, 1 % of the value otherwise.
fn coord(ui: &mut Ui, v: &mut f64, relative: bool) -> egui::Response {
    let s = if relative { 0.005 } else { speed(*v) };
    ui.add(DragValue::new(v).speed(s).max_decimals(6))
}

fn hash_of(v: &impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// Hash of the figures (imported tables by their size): changes mark the project as modified.
fn figures_hash(figs: &mut [Figure]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in figs.iter_mut() {
        let d = std::mem::take(&mut f.data);
        serde_json::to_vec(f).unwrap_or_default().hash(&mut h);
        for t in &d {
            (&t.name, &t.columns, t.values.iter().map(|c| c.len()).sum::<usize>()).hash(&mut h);
        }
        f.data = d;
    }
    h.finish()
}

fn file_name(s: &str, ext: &str) -> String {
    let base: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' }).collect();
    let base = base.trim();
    format!("{}.{ext}", if base.is_empty() { "figure" } else { base })
}

/// A column menu, grouped like the variables of the source.
fn column_combo(ui: &mut Ui, id: &str, key: &mut String, src: &Source, cols: &[ColInfo]) -> egui::Response {
    let before = key.clone();
    let text = cols.iter().find(|c| c.key == *key).map(|c| c.title()).unwrap_or_else(|| if key.is_empty() { "–".into() } else { format!("{key} (not available)") });
    let mut r = ComboBox::from_id_salt(id).width(250.0).selected_text(text).height(420.0).show_ui(ui, |ui| {
        let mut groups: Vec<&str> = vec![];
        for c in cols {
            let g = data::column_group(src, c);
            if !groups.contains(&g) {
                groups.push(g);
            }
        }
        for g in groups {
            ui.label(RichText::new(g).small().weak());
            for c in cols.iter().filter(|c| data::column_group(src, c) == g) {
                ui.selectable_value(key, c.key.clone(), format!("   {}", c.title()));
            }
        }
    });
    if *key != before {
        r.response.mark_changed();
    }
    r.response
}

/// Id of the figure window.
pub fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("rusteau_figures")
}

impl App {
    /// The figure window (a window of its own; inside the main window where the system does not support that).
    pub fn figure_window(&mut self, ctx: &egui::Context) {
        if !self.figs.open {
            return;
        }
        let title = match self.prj.figures.get(self.figs.fig) {
            Some(f) => format!("Rusteau-1D – Figures – {}", f.name),
            None => "Rusteau-1D – Figures".into(),
        };
        let vid = viewport_id();
        let builder = egui::ViewportBuilder::default().with_title(title).with_inner_size([1320.0, 860.0]).with_min_inner_size([900.0, 560.0]);
        ctx.show_viewport_immediate(vid, builder, |ctx, class| {
            if class == egui::ViewportClass::Embedded && crate::shots::ACTIVE.load(std::sync::atomic::Ordering::Relaxed) {
                // the screenshot tour: the window covers the main window
                let screen = ctx.screen_rect();
                egui::Area::new(egui::Id::new("fig_tour")).fixed_pos(screen.min).order(egui::Order::Foreground).show(ctx, |ui| {
                    egui::Frame::new().fill(ctx.style().visuals.panel_fill).show(ui, |ui| {
                        ui.set_min_size(screen.size());
                        ui.set_max_size(screen.size());
                        self.figure_body(ui);
                    });
                });
                return;
            }
            if class == egui::ViewportClass::Embedded {
                let mut open = true;
                egui::Window::new("Figures").open(&mut open).default_size([1150.0, 760.0]).show(ctx, |ui| self.figure_body(ui));
                if !open {
                    self.figs.open = false;
                }
                return;
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.figs.open = false;
                self.figs.help_here = false;
            }
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| self.figure_body(ui));
        });
    }

    fn figure_body(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let mut figures = std::mem::take(&mut self.prj.figures);
        let runs = std::mem::take(&mut self.figs.runs);
        let mut acts: Vec<Act> = vec![];
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            acts.push(Act::Help);
        }
        {
            let fit = match (&self.inv.settings, &self.inv.result) {
                (Some(s), Some(r)) => Some((s, r)),
                _ => None,
            };
            let dc = DataCtx { prj: &self.prj, results: self.results.as_ref(), runs: &runs, measured: &self.measured, fit };
            let tc = TemplateCtx { prj: &self.prj, runs: runs.iter().map(|r| r.name.clone()).collect(), has_fit: fit.is_some(), has_measured: !self.measured.is_empty() };
            let st = &mut self.figs;
            if figures.is_empty() {
                st.fig = 0;
            } else {
                st.fig = st.fig.min(figures.len() - 1);
            }
            egui::TopBottomPanel::top("fig_toolbar").show_inside(ui, |ui| toolbar(ui, st, &mut figures, &tc, &mut acts));
            egui::TopBottomPanel::bottom("fig_status").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&st.hover).monospace());
                    ui.separator();
                    ui.label(&st.status);
                });
            });
            if figures.is_empty() {
                egui::CentralPanel::default().show_inside(ui, |ui| start_page(ui, &tc, &mut acts));
            } else {
                let env = hash_of(&(
                    serde_json::to_vec(dc.prj).unwrap_or_default(),
                    dc.results.map(|r| (r.tlevel.as_ptr() as usize, r.tlevel.len(), r.profiles.len(), r.obs.len(), r.balance.len())),
                    runs.iter().map(|r| (r.name.clone(), r.results.tlevel.len())).collect::<Vec<_>>(),
                    self.measured.iter().map(|m| (m.name.clone(), m.pts.len())).collect::<Vec<_>>(),
                    fit.map(|(s, r)| (s.observations.len(), r.fitted.len())),
                ));
                egui::SidePanel::left("fig_side").resizable(true).default_width(400.0).min_width(330.0).show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        let fig = &mut figures[st.fig];
                        controls(ui, st, fig, &dc, &runs, &mut acts);
                    });
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let fig = &mut figures[st.fig];
                    preview(ui, st, fig, &dc, env);
                });
            }
            if !figures.is_empty() {
                let h = figures_hash(&mut figures);
                if st.last_hash.is_some_and(|x| x != h) {
                    self.dirty = true;
                }
                st.last_hash = Some(h);
            }
        }
        self.prj.figures = figures;
        self.figs.runs = runs;
        self.figure_acts(&ctx, acts);
        self.figure_dialog(&ctx);
        if self.figs.help_here {
            if let Some(crate::help::HelpEvent::OpenInBrowser(id)) = self.help.window(&ctx) {
                if let Ok(url) = crate::help::html_url(&id) {
                    ctx.open_url(egui::OpenUrl::new_tab(&url));
                }
            }
            if !self.help.open {
                self.figs.help_here = false;
            }
        }
    }

    fn figure_acts(&mut self, ctx: &egui::Context, acts: Vec<Act>) {
        for a in acts {
            match a {
                Act::New(k) => {
                    let tc = TemplateCtx {
                        prj: &self.prj,
                        runs: self.figs.runs.iter().map(|r| r.name.clone()).collect(),
                        has_fit: self.inv.result.is_some(),
                        has_measured: !self.measured.is_empty(),
                    };
                    let t = &templates::all()[k];
                    let mut f = (t.make)(&tc);
                    if k == 0 {
                        f.name = format!("Figure {}", self.prj.figures.len() + 1);
                    }
                    self.prj.figures.push(f);
                    self.figs.fig = self.prj.figures.len() - 1;
                    self.figs.panel = 0;
                    self.figs.layer = 0;
                    self.figs.ann = 0;
                    self.figs.status = format!("New figure from the template \"{}\"", t.name);
                }
                Act::Duplicate => {
                    if let Some(f) = self.prj.figures.get(self.figs.fig).cloned() {
                        let mut f = f;
                        f.name = format!("{} (copy)", f.name);
                        self.prj.figures.insert(self.figs.fig + 1, f);
                        self.figs.fig += 1;
                    }
                }
                Act::Delete => {
                    if self.figs.fig < self.prj.figures.len() {
                        let f = self.prj.figures.remove(self.figs.fig);
                        self.figs.fig = self.figs.fig.saturating_sub(1);
                        self.figs.status = format!("Figure \"{}\" deleted", f.name);
                        self.dirty = true;
                    }
                }
                Act::ExportPng | Act::ExportSvg => {
                    let Some(f) = self.prj.figures.get(self.figs.fig) else { continue };
                    let png = matches!(a, Act::ExportPng);
                    let (purpose, ext) = if png { (Purpose::FigurePng, "png") } else { (Purpose::FigureSvg, "svg") };
                    self.figs.dialog = Some(FileDialog::new(purpose, Mode::SaveFile, if png { "Export the figure as PNG" } else { "Export the figure as SVG" }, self.last_dir.clone(), &file_name(&f.name, ext), &[ext]));
                }
                Act::CopyImage => {
                    if let Some(pm) = self.figure_pixmap(4000.0) {
                        let img = egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &canvas::rgba(&pm));
                        ctx.copy_image(img);
                        self.figs.status = format!("Figure copied to the clipboard ({} × {} pixels)", pm.width(), pm.height());
                    }
                }
                Act::ImportData => {
                    self.figs.dialog = Some(FileDialog::new(Purpose::FigureData, Mode::OpenFile, "Import a data table (text or CSV)", self.last_dir.clone(), "", &["csv", "txt", "dat", "tsv", "prn", "out"]));
                }
                Act::KeepRun(name) => {
                    if let Some(r) = &self.results {
                        let mut name = name.trim().to_string();
                        if name.is_empty() {
                            name = format!("Run {}", self.figs.runs.len() + 1);
                        }
                        while self.figs.runs.iter().any(|x| x.name == name) {
                            name.push('\'');
                        }
                        self.figs.runs.push(SavedRun { name: name.clone(), project: self.prj.clone(), results: r.clone() });
                        self.figs.run_name.clear();
                        self.figs.status = format!("Results kept as run \"{name}\": choose it in the runs of a layer to compare");
                    }
                }
                Act::LoadRun => {
                    self.figs.dialog = Some(FileDialog::new(Purpose::FigureLoadRun, Mode::OpenFile, "Load a kept run or results (.json)", self.last_dir.clone(), "", &["json"]));
                }
                Act::SaveRun(k) => {
                    if let Some(r) = self.figs.runs.get(k) {
                        self.figs.save_run = k;
                        self.figs.dialog = Some(FileDialog::new(Purpose::FigureSaveRun, Mode::SaveFile, "Save the kept run", self.last_dir.clone(), &file_name(&r.name, "json"), &["json"]));
                    }
                }
                Act::RemoveRun(k) => {
                    if k < self.figs.runs.len() {
                        let r = self.figs.runs.remove(k);
                        self.figs.status = format!("Run \"{}\" removed", r.name);
                    }
                }
                Act::Help => {
                    self.help.show_topic("figures", None);
                    self.figs.help_here = true;
                }
            }
        }
    }

    /// The selected figure rasterized at its resolution (at most `max_px` pixels wide or high).
    fn figure_pixmap(&self, max_px: f64) -> Option<tiny_skia::Pixmap> {
        let f = self.prj.figures.get(self.figs.fig)?;
        let r = self.render_figure(f);
        let scale = (f.dpi / 72.0).min(max_px / (r.drawing.width.max(r.drawing.height) as f64)) as f32;
        Some(canvas::rasterize(&r.drawing, scale, f.transparent))
    }

    /// The selected figure rasterized at `dpi` (example images of the help).
    pub fn figure_pixmap_at(&self, dpi: f64) -> Option<tiny_skia::Pixmap> {
        let f = self.prj.figures.get(self.figs.fig)?;
        let r = self.render_figure(f);
        Some(canvas::rasterize(&r.drawing, (dpi / 72.0) as f32, false))
    }

    fn render_figure(&self, f: &Figure) -> Rendered {
        let fit = match (&self.inv.settings, &self.inv.result) {
            (Some(s), Some(r)) => Some((s, r)),
            _ => None,
        };
        let dc = DataCtx { prj: &self.prj, results: self.results.as_ref(), runs: &self.figs.runs, measured: &self.measured, fit };
        let series: Vec<Vec<Vec<data::Series>>> = f.panels.iter().map(|p| p.layers.iter().map(|l| dc.series(l, f)).collect()).collect();
        render::render(f, &series)
    }

    fn figure_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.figs.dialog.take() else { return };
        match d.show(ctx) {
            DialogResult::Open => self.figs.dialog = Some(d),
            DialogResult::Cancel => {}
            DialogResult::Chosen(path) => {
                self.last_dir = path.parent().map(|p| p.to_path_buf());
                let k = self.figs.fig;
                self.figs.status = match d.purpose {
                    Purpose::FigurePng => {
                        let p = if path.extension().is_none() { path.with_extension("png") } else { path };
                        match (self.figure_pixmap(20000.0), self.prj.figures.get(k)) {
                            (Some(pm), Some(f)) => match canvas::write_png(&pm, f.dpi, &p) {
                                Ok(()) => format!("PNG written: {} ({} × {} pixels, {} dpi)", p.display(), pm.width(), pm.height(), f.dpi),
                                Err(e) => format!("Could not write the PNG: {e}"),
                            },
                            _ => String::new(),
                        }
                    }
                    Purpose::FigureSvg => {
                        let p = if path.extension().is_none() { path.with_extension("svg") } else { path };
                        match self.prj.figures.get(k) {
                            Some(f) => {
                                let r = self.render_figure(f);
                                match std::fs::write(&p, canvas::to_svg(&r.drawing, f.svg_text_as_paths, f.transparent)) {
                                    Ok(()) => format!("SVG written: {}", p.display()),
                                    Err(e) => format!("Could not write the SVG: {e}"),
                                }
                            }
                            None => String::new(),
                        }
                    }
                    Purpose::FigureData => match data::read_table(&path) {
                        Ok(t) => {
                            let msg = format!("Imported {} with {} columns and {} rows: choose it as the source of a layer", t.name, t.columns.len(), t.values.first().map(|c| c.len()).unwrap_or(0));
                            if let Some(f) = self.prj.figures.get_mut(k) {
                                f.data.push(t);
                                let i = f.data.len() - 1;
                                if let Some(l) = f.panels.get_mut(self.figs.panel).and_then(|p| p.layers.get_mut(self.figs.layer)) {
                                    if l.source == Source::default() && l.x == "t" && l.y == "vTop" {
                                        // a fresh layer takes the new table
                                        l.source = Source::DataFile(i);
                                        l.x = "col0".into();
                                        l.y = "col1".into();
                                    }
                                }
                            }
                            msg
                        }
                        Err(e) => e,
                    },
                    Purpose::FigureLoadRun => match data::load_run(&path) {
                        Ok(mut r) => {
                            while self.figs.runs.iter().any(|x| x.name == r.name) {
                                r.name.push('\'');
                            }
                            if r.project.profile.nodes.is_empty() {
                                // results without their project: units and profile of the current project
                                r.project = self.prj.clone();
                                r.project.figures.clear();
                            }
                            let msg = format!("Run \"{}\" loaded from {}", r.name, path.display());
                            self.figs.runs.push(r);
                            msg
                        }
                        Err(e) => e,
                    },
                    Purpose::FigureSaveRun => match self.figs.runs.get(self.figs.save_run) {
                        Some(r) => {
                            let p = if path.extension().is_none() { path.with_extension("json") } else { path };
                            match data::save_run(r, &p) {
                                Ok(()) => format!("Run \"{}\" saved to {}", r.name, p.display()),
                                Err(e) => e,
                            }
                        }
                        None => String::new(),
                    },
                    _ => String::new(),
                };
            }
        }
    }
}

fn toolbar(ui: &mut Ui, st: &mut FigState, figures: &mut [Figure], tc: &TemplateCtx, acts: &mut Vec<Act>) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Figures").heading().strong());
        if ui.small_button("?").on_hover_text("Help on the figure editor (F1)").clicked() {
            acts.push(Act::Help);
        }
        ui.add_space(12.0);
        if !figures.is_empty() {
            let cur = figures[st.fig].name.clone();
            let mut sel = st.fig;
            ComboBox::from_id_salt("fig_sel").width(220.0).selected_text(cur).show_ui(ui, |ui| {
                for (k, f) in figures.iter().enumerate() {
                    ui.selectable_value(&mut sel, k, &f.name);
                }
            });
            if sel != st.fig {
                st.fig = sel;
                st.panel = 0;
                st.layer = 0;
                st.ann = 0;
            }
        }
        ui.menu_button("New figure ▾", |ui| {
            template_menu(ui, tc, acts);
        })
        .response
        .tip("New figure");
        let has = !figures.is_empty();
        if ui.add_enabled(has, egui::Button::new("Duplicate")).tip("Duplicate").clicked() {
            acts.push(Act::Duplicate);
        }
        if ui.add_enabled(has, egui::Button::new("Delete")).tip("Delete figure").clicked() {
            acts.push(Act::Delete);
        }
        ui.separator();
        if ui.add_enabled(has, egui::Button::new("Export PNG…")).tip("Export PNG").clicked() {
            acts.push(Act::ExportPng);
        }
        if ui.add_enabled(has, egui::Button::new("Export SVG…")).tip("Export SVG").clicked() {
            acts.push(Act::ExportSvg);
        }
        if ui.add_enabled(has, egui::Button::new("Copy image")).tip("Copy image").clicked() {
            acts.push(Act::CopyImage);
        }
    });
    ui.add_space(2.0);
}

fn template_menu(ui: &mut Ui, tc: &TemplateCtx, acts: &mut Vec<Act>) {
    for (k, t) in templates::all().iter().enumerate() {
        let ok = (t.fits)(tc);
        let r = ui.add_enabled(ok, egui::Button::new(t.name)).on_hover_text(t.desc).on_disabled_hover_text(format!("{} (needs other data: observation nodes, solutes, measured data, a parameter estimation or kept runs)", t.desc));
        if r.clicked() {
            acts.push(Act::New(k));
            ui.close_menu();
        }
    }
}

fn start_page(ui: &mut Ui, tc: &TemplateCtx, acts: &mut Vec<Act>) {
    ui.add_space(20.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("No figures yet").heading());
        ui.add_space(6.0);
        ui.label("Start from a template; everything can be changed afterwards. The figures are saved with the project.");
        ui.add_space(12.0);
    });
    egui::Grid::new("fig_templates").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
        for (k, t) in templates::all().iter().enumerate() {
            let ok = (t.fits)(tc);
            if ui.add_enabled(ok, egui::Button::new(RichText::new(t.name).strong()).min_size(egui::vec2(300.0, 0.0))).clicked() {
                acts.push(Act::New(k));
            }
            ui.label(RichText::new(t.desc).weak());
            ui.end_row();
        }
    });
}

fn controls(ui: &mut Ui, st: &mut FigState, fig: &mut Figure, dc: &DataCtx, runs: &[SavedRun], acts: &mut Vec<Act>) {
    ui.spacing_mut().item_spacing.y = 6.0;
    figure_section(ui, fig, st.fold.take());
    text_section(ui, fig);
    fig.sync_panels();
    st.panel = st.panel.min(fig.panels.len().saturating_sub(1));
    // panel
    CollapsingHeader::new(RichText::new("Panel").strong()).default_open(true).show(ui, |ui| {
        if fig.panels.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                ui.label("Panel:").tip("Panel");
                for k in 0..fig.panels.len() {
                    let t = &fig.panels[k].title;
                    let name = if t.is_empty() { format!("{}", k + 1) } else { format!("{} {}", k + 1, t.chars().take(18).collect::<String>()) };
                    if ui.selectable_label(st.panel == k, name).clicked() {
                        st.panel = k;
                        st.layer = 0;
                        st.ann = 0;
                    }
                }
            });
        }
        let p = &mut fig.panels[st.panel];
        egui::Grid::new("fig_panel").num_columns(2).show(ui, |ui| {
            row(ui, "Panel title");
            ui.add(egui::TextEdit::singleline(&mut p.title).desired_width(230.0).hint_text("e.g. (a) Loam")).tip("Panel title");
            ui.end_row();
            row(ui, "Legend");
            combo(ui, "fig_legend", &mut p.legend, &LEGENDS, 160.0).tip("Legend");
            ui.end_row();
        });
    });
    // the panels are edited while the figure (its data tables) is read
    let mut panels = std::mem::take(&mut fig.panels);
    let p = &mut panels[st.panel];
    layers_section(ui, st, p, fig, dc, runs);
    axes_section(ui, st, p);
    annotations_section(ui, st, p);
    fig.panels = panels;
    data_section(ui, fig, acts);
    runs_section(ui, st, dc, runs, acts);
    symbols_section(ui, st);
}

fn figure_section(ui: &mut Ui, fig: &mut Figure, fold: Option<bool>) {
    CollapsingHeader::new(RichText::new("Figure").strong()).default_open(true).open(fold).show(ui, |ui| {
        egui::Grid::new("fig_main").num_columns(2).show(ui, |ui| {
            row(ui, "Name");
            ui.add(egui::TextEdit::singleline(&mut fig.name).desired_width(230.0)).tip("Name");
            ui.end_row();
            row(ui, "Title");
            ui.add(egui::TextEdit::singleline(&mut fig.title).desired_width(230.0).hint_text("none")).tip("Title");
            ui.end_row();
            row(ui, "Subtitle");
            ui.add(egui::TextEdit::singleline(&mut fig.subtitle).desired_width(230.0).hint_text("none")).tip("Subtitle");
            ui.end_row();
            row(ui, "Caption");
            ui.add(egui::TextEdit::multiline(&mut fig.caption).desired_width(230.0).desired_rows(2).hint_text("none")).tip("Caption");
            ui.end_row();
            row(ui, "Size (mm)");
            ui.horizontal(|ui| {
                num(ui, &mut fig.width_mm, 20.0, 1000.0, "").tip("Size (mm)");
                ui.label("×");
                num(ui, &mut fig.height_mm, 20.0, 1000.0, "").tip("Size (mm)");
                ui.menu_button("▾", |ui| {
                    for (n, w, h) in SIZES {
                        if ui.button(n).clicked() {
                            fig.width_mm = w;
                            fig.height_mm = h;
                            ui.close_menu();
                        }
                    }
                })
                .response
                .on_hover_text("Common sizes");
            });
            ui.end_row();
            row(ui, "Resolution (dpi)");
            num(ui, &mut fig.dpi, 50.0, 1200.0, "").tip("Resolution (dpi)");
            ui.end_row();
            row(ui, "Panels (rows × columns)");
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut fig.rows).range(1..=4)).tip("Panels (rows × columns)");
                ui.label("×");
                ui.add(DragValue::new(&mut fig.cols).range(1..=4)).tip("Panels (rows × columns)");
            });
            ui.end_row();
            row(ui, "Theme");
            combo(ui, "fig_theme", &mut fig.theme, &THEMES, 200.0).tip("Theme");
            ui.end_row();
            row(ui, "Palette");
            combo(ui, "fig_palette", &mut fig.palette, &PALETTES, 200.0).tip("Palette");
            ui.end_row();
        });
        ui.checkbox(&mut fig.transparent, "Transparent background (PNG, SVG)").tip("Transparent background (PNG, SVG)");
        ui.checkbox(&mut fig.svg_text_as_paths, "SVG: text as outlines").tip("SVG: text as outlines");
    });
}

fn text_section(ui: &mut Ui, fig: &mut Figure) {
    CollapsingHeader::new(RichText::new("Font and text sizes").strong()).show(ui, |ui| {
        egui::Grid::new("fig_text").num_columns(2).show(ui, |ui| {
            row(ui, "Font");
            let mut font = fig.font.clone();
            ComboBox::from_id_salt("fig_font").width(220.0).selected_text(&font).height(400.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut font, crate::fonts::DEFAULT_FAMILY.to_string(), "Noto Sans (bundled, all symbols)");
                ui.separator();
                for f in crate::fonts::system_families() {
                    ui.selectable_value(&mut font, f.name.clone(), &f.name);
                }
            })
            .response
            .tip("Font");
            fig.font = font;
            ui.end_row();
            let s = &mut fig.sizes;
            for (label, v) in [
                ("Title (pt)", &mut s.title),
                ("Subtitle (pt)", &mut s.subtitle),
                ("Caption (pt)", &mut s.caption),
                ("Panel titles (pt)", &mut s.panel_title),
                ("Axis titles (pt)", &mut s.axis_title),
                ("Tick labels (pt)", &mut s.ticks),
                ("Legend (pt)", &mut s.legend),
            ] {
                row(ui, label);
                ui.add(DragValue::new(v).range(4.0..=48.0).speed(0.1).max_decimals(1)).tip(label);
                ui.end_row();
            }
        });
        ui.checkbox(&mut fig.title_bold, "Bold title").tip("Bold title");
    });
}

fn layer_name(l: &Layer, cols: &[ColInfo]) -> String {
    let n = |k: &str| cols.iter().find(|c| c.key == k).map(|c| c.name.clone()).unwrap_or_else(|| k.to_string());
    format!("{} against {}", n(&l.y), n(&l.x))
}

fn layers_section(ui: &mut Ui, st: &mut FigState, p: &mut Panel, fig: &Figure, dc: &DataCtx, runs: &[SavedRun]) {
    CollapsingHeader::new(RichText::new("Layers (data)").strong()).default_open(true).show(ui, |ui| {
        let mut action: Option<(usize, i32)> = None;
        for k in 0..p.layers.len() {
            let cols = dc.columns(&p.layers[k].source, fig);
            ui.horizontal(|ui| {
                ui.checkbox(&mut p.layers[k].visible, "").tip("Visible");
                let name = format!("{} {}", k + 1, layer_name(&p.layers[k], &cols));
                if ui.selectable_label(st.layer == k, name).clicked() {
                    st.layer = k;
                }
                if ui.small_button("⬆").on_hover_text("Draw earlier (below the others)").clicked() {
                    action = Some((k, -1));
                }
                if ui.small_button("⬇").on_hover_text("Draw later (on top)").clicked() {
                    action = Some((k, 1));
                }
                if ui.small_button("🗑").on_hover_text("Remove the layer").clicked() {
                    action = Some((k, 0));
                }
            });
        }
        match action {
            Some((k, 0)) => {
                p.layers.remove(k);
                st.layer = st.layer.min(p.layers.len().saturating_sub(1));
            }
            Some((k, d)) => {
                let j = k as i32 + d;
                if j >= 0 && (j as usize) < p.layers.len() {
                    p.layers.swap(k, j as usize);
                    st.layer = j as usize;
                }
            }
            None => {}
        }
        ui.horizontal(|ui| {
            if ui.button("+ Add layer").tip("Add layer").clicked() {
                let l = p.layers.get(st.layer).cloned().map(|l| Layer { color: None, label: String::new(), ..l }).unwrap_or_default();
                p.layers.push(l);
                st.layer = p.layers.len() - 1;
            }
        });
        let Some(l) = p.layers.get_mut(st.layer) else { return };
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Layer {}", st.layer + 1)).strong());
        layer_editor(ui, l, fig, dc, runs);
    });
}

fn layer_editor(ui: &mut Ui, l: &mut Layer, fig: &Figure, dc: &DataCtx, runs: &[SavedRun]) {
    egui::Grid::new("fig_layer").num_columns(2).show(ui, |ui| {
        row(ui, "Data source");
        let before = l.source.clone();
        ComboBox::from_id_salt("fig_source").width(250.0).selected_text(data::source_name(&l.source, fig)).height(420.0).show_ui(ui, |ui| {
            let mut group = "";
            for s in data::all_sources(fig) {
                let g = data::source_group(&s);
                if g != group {
                    ui.label(RichText::new(g).small().weak());
                    group = g;
                }
                let name = format!("   {}", data::source_name(&s, fig));
                ui.selectable_value(&mut l.source, s, name);
            }
        })
        .response
        .tip("Data source");
        ui.end_row();
        let cols = dc.columns(&l.source, fig);
        if l.source != before {
            // keep the variables when the new source has them, otherwise its first columns
            l.items.clear();
            let has = |k: &str| cols.iter().any(|c| c.key == k);
            if !has(&l.x) {
                l.x = cols.first().map(|c| c.key.clone()).unwrap_or_default();
            }
            if !has(&l.y) {
                l.y = cols.get(1).or(cols.first()).map(|c| c.key.clone()).unwrap_or_default();
            }
        }
        row(ui, "x variable");
        column_combo(ui, "fig_x", &mut l.x, &l.source, &cols).tip("x variable");
        ui.end_row();
        row(ui, "y variable");
        column_combo(ui, "fig_y", &mut l.y, &l.source, &cols).tip("y variable");
        ui.end_row();
        row(ui, "Scale factors x, y");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut l.x_scale).speed(0.01).max_decimals(6)).tip("Scale factors x, y");
            ui.add(DragValue::new(&mut l.y_scale).speed(0.01).max_decimals(6)).tip("Scale factors x, y");
        });
        ui.end_row();
    });
    // items: nodes, print times, materials, sets
    if data::has_items(&l.source) {
        let items = dc.items(&l.source);
        let what = match l.source {
            Source::ObsNodes => "Observation nodes",
            Source::Profiles => "Print times",
            Source::Hydraulic => "Materials",
            _ => "Data sets",
        };
        ui.horizontal(|ui| {
            ui.label(what).tip("Items");
            let mut all = l.items.is_empty();
            if ui.checkbox(&mut all, "all").tip("Items").changed() {
                l.items = if all { vec![] } else { vec![0] };
            }
        });
        if items.is_empty() {
            ui.label(RichText::new("none (run the calculation or load the data first)").weak());
        } else if !l.items.is_empty() {
            egui::ScrollArea::vertical().id_salt("fig_items").max_height(150.0).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (k, name) in items.iter().enumerate() {
                        let mut on = l.items.contains(&k);
                        if ui.checkbox(&mut on, name).changed() {
                            if on {
                                l.items.push(k);
                                l.items.sort_unstable();
                            } else {
                                l.items.retain(|x| *x != k);
                            }
                        }
                    }
                });
            });
        }
    }
    // runs
    if data::per_run(&l.source) && !runs.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label("Runs").tip("Runs");
            let mut sel: Vec<String> = if l.runs.is_empty() { vec![String::new()] } else { l.runs.clone() };
            let names: Vec<(String, String)> = std::iter::once((String::new(), "current results".to_string())).chain(runs.iter().map(|r| (r.name.clone(), r.name.clone()))).collect();
            let mut changed = false;
            for (key, label) in names {
                let mut on = sel.contains(&key);
                if ui.checkbox(&mut on, label).changed() {
                    changed = true;
                    if on {
                        sel.push(key);
                    } else {
                        sel.retain(|x| *x != key);
                    }
                }
            }
            if changed {
                l.runs = if sel == vec![String::new()] { vec![] } else { sel };
            }
        });
    }
    ui.add_space(4.0);
    egui::Grid::new("fig_style").num_columns(2).show(ui, |ui| {
        row(ui, "Geometry");
        combo(ui, "fig_geom", &mut l.geom, &GEOMS, 160.0).tip("Geometry");
        ui.end_row();
        row(ui, "Colour");
        ui.horizontal(|ui| {
            let mut auto = l.color.is_none();
            if ui.checkbox(&mut auto, "palette").tip("Colour").changed() {
                l.color = if auto { None } else { Some([0, 114, 178]) };
            }
            if let Some(c) = &mut l.color {
                egui::color_picker::color_edit_button_srgb(ui, c).tip("Colour");
            }
        });
        ui.end_row();
        if l.geom != Geom::Points {
            row(ui, "Line width (pt)");
            ui.add(DragValue::new(&mut l.width).range(0.1..=10.0).speed(0.05).max_decimals(2)).tip("Line width (pt)");
            ui.end_row();
            row(ui, "Line style");
            combo(ui, "fig_dash", &mut l.dash, &DASHES, 160.0).tip("Line style");
            ui.end_row();
        }
        if matches!(l.geom, Geom::Points | Geom::LinePoints) {
            row(ui, "Marker");
            ui.horizontal(|ui| {
                combo(ui, "fig_marker", &mut l.marker, &MARKERS, 130.0).tip("Marker");
                ui.add(DragValue::new(&mut l.marker_size).range(1.0..=20.0).speed(0.1).max_decimals(1).suffix(" pt")).tip("Marker");
            });
            ui.end_row();
        }
        if matches!(l.geom, Geom::Area | Geom::Bars) {
            row(ui, "Fill opacity");
            ui.add(egui::Slider::new(&mut l.alpha, 0.0..=1.0)).tip("Fill opacity");
            ui.end_row();
        }
        row(ui, "Legend label");
        ui.add(egui::TextEdit::singleline(&mut l.label).desired_width(230.0).hint_text("automatic; {item} {run} {y}")).tip("Legend label");
        ui.end_row();
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut l.in_legend, "In the legend").tip("In the legend");
        ui.checkbox(&mut l.y2, "Right y axis").tip("Right y axis");
    });
}

fn axis_editor(ui: &mut Ui, a: &mut Axis, id: &str) {
    egui::Grid::new(id).num_columns(2).show(ui, |ui| {
        row(ui, "Axis title");
        ui.horizontal(|ui| {
            ui.checkbox(&mut a.label_auto, "auto").tip("Axis title");
            if !a.label_auto {
                ui.add(egui::TextEdit::singleline(&mut a.label).desired_width(190.0).hint_text("e.g. θ (cm^{3} cm^{−3})")).tip("Axis title");
            }
        });
        ui.end_row();
        row(ui, "Range");
        ui.horizontal(|ui| {
            ui.checkbox(&mut a.auto, "auto").tip("Range");
            if !a.auto {
                coord(ui, &mut a.min, false).tip("Range");
                ui.label("to");
                coord(ui, &mut a.max, false).tip("Range");
            }
        });
        ui.end_row();
        row(ui, "Tick step");
        num(ui, &mut a.step, 0.0, f64::MAX, "").on_hover_text("0: automatic").tip("Tick step");
        ui.end_row();
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut a.log, "Logarithmic").tip("Logarithmic");
        ui.checkbox(&mut a.reverse, "Reversed").tip("Reversed");
        ui.checkbox(&mut a.grid, "Grid lines").tip("Grid lines");
        ui.checkbox(&mut a.minor, "Minor ticks").tip("Minor ticks");
    });
}

fn axes_section(ui: &mut Ui, st: &mut FigState, p: &mut Panel) {
    CollapsingHeader::new(RichText::new("Axes").strong()).default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            for (k, n) in ["x axis", "y axis", "Right y axis"].iter().enumerate() {
                ui.selectable_value(&mut st.axis, k, *n);
            }
        });
        match st.axis {
            0 => axis_editor(ui, &mut p.x, "fig_ax_x"),
            1 => axis_editor(ui, &mut p.y, "fig_ax_y"),
            _ => {
                if !p.layers.iter().any(|l| l.y2) {
                    ui.label(RichText::new("Shown when a layer uses the right y axis.").weak());
                }
                axis_editor(ui, &mut p.y2, "fig_ax_y2");
            }
        }
    });
}

fn annotations_section(ui: &mut Ui, st: &mut FigState, p: &mut Panel) {
    CollapsingHeader::new(RichText::new("Annotations").strong()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (kind, n) in KINDS {
                if ui.button(format!("+ {n}")).tip("Add annotation").clicked() {
                    let a = match kind {
                        AnnotationKind::Text => Annotation::default(),
                        AnnotationKind::HLine => Annotation { kind, text: String::new(), y: 0.5, ..Default::default() },
                        AnnotationKind::VLine => Annotation { kind, text: String::new(), x: 0.5, ..Default::default() },
                        AnnotationKind::Arrow => Annotation { kind, x: 0.75, y: 0.75, x2: 0.55, y2: 0.55, ..Default::default() },
                    };
                    p.annotations.push(a);
                    st.ann = p.annotations.len() - 1;
                }
            }
        });
        let mut remove = None;
        for (k, a) in p.annotations.iter().enumerate() {
            ui.horizontal(|ui| {
                let t = if a.text.is_empty() { name_of(&KINDS, a.kind).to_string() } else { format!("{}: {}", name_of(&KINDS, a.kind), a.text.chars().take(28).collect::<String>()) };
                if ui.selectable_label(st.ann == k, format!("{} {t}", k + 1)).clicked() {
                    st.ann = k;
                }
                if ui.small_button("🗑").on_hover_text("Remove the annotation").clicked() {
                    remove = Some(k);
                }
            });
        }
        if let Some(k) = remove {
            p.annotations.remove(k);
        }
        st.ann = st.ann.min(p.annotations.len().saturating_sub(1));
        let Some(a) = p.annotations.get_mut(st.ann) else { return };
        ui.add_space(4.0);
        egui::Grid::new("fig_ann").num_columns(2).show(ui, |ui| {
            row(ui, "Kind");
            combo(ui, "fig_ann_kind", &mut a.kind, &KINDS, 160.0).tip("Kind");
            ui.end_row();
            row(ui, "Text");
            ui.add(egui::TextEdit::singleline(&mut a.text).desired_width(230.0).hint_text("markup: x_{s}, x^{2}, *italic*")).tip("Text");
            ui.end_row();
            row(ui, "Position x, y");
            ui.horizontal(|ui| {
                coord(ui, &mut a.x, a.relative).tip("Position x, y");
                coord(ui, &mut a.y, a.relative).tip("Position x, y");
            });
            ui.end_row();
            if a.kind == AnnotationKind::Arrow {
                row(ui, "Arrow end x, y");
                ui.horizontal(|ui| {
                    coord(ui, &mut a.x2, a.relative).tip("Arrow end x, y");
                    coord(ui, &mut a.y2, a.relative).tip("Arrow end x, y");
                });
                ui.end_row();
            }
            row(ui, "Font size (pt)");
            ui.add(DragValue::new(&mut a.size).range(0.0..=48.0).speed(0.1).max_decimals(1)).on_hover_text("0: size of the tick labels").tip("Font size (pt)");
            ui.end_row();
            row(ui, "Colour");
            egui::color_picker::color_edit_button_srgb(ui, &mut a.color).tip("Annotation colour");
            ui.end_row();
            if a.kind != AnnotationKind::Arrow {
                row(ui, "Text alignment");
                combo(ui, "fig_ann_anchor", &mut a.anchor, &ANCHORS, 120.0).tip("Text alignment");
                ui.end_row();
            }
            if a.kind != AnnotationKind::Text {
                row(ui, "Line");
                ui.horizontal(|ui| {
                    combo(ui, "fig_ann_dash", &mut a.dash, &DASHES, 100.0).tip("Line");
                    ui.add(DragValue::new(&mut a.width).range(0.1..=8.0).speed(0.05).max_decimals(2).suffix(" pt")).tip("Line");
                });
                ui.end_row();
            }
        });
        ui.horizontal_wrapped(|ui| {
            let before = a.relative;
            ui.checkbox(&mut a.relative, "Relative to the plot area (0–1)").tip("Relative to the plot area (0–1)");
            if a.relative != before {
                // keep it on the plot: the other system has another scale
                (a.x, a.y, a.x2, a.y2) = if a.relative { (0.05, 0.95, 0.3, 0.7) } else { (a.x, a.y, a.x2, a.y2) };
            }
            ui.checkbox(&mut a.on_y2, "Right y axis").tip("Annotation on the right y axis");
        });
        ui.toggle_value(&mut st.place, "🖱 Place with the mouse").tip("Place with the mouse");
    });
}

fn data_section(ui: &mut Ui, fig: &mut Figure, acts: &mut Vec<Act>) {
    CollapsingHeader::new(RichText::new("Data files").strong()).show(ui, |ui| {
        ui.label(RichText::new("Tables from text or CSV files (a header line with the column names, units in brackets), e.g. measurements or results of other programs. They are stored in the project.").weak());
        if ui.button("Import table…").tip("Import table").clicked() {
            acts.push(Act::ImportData);
        }
        let mut remove = None;
        for (k, t) in fig.data.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}: {} columns × {} rows", t.name, t.columns.len(), t.values.first().map(|c| c.len()).unwrap_or(0)));
                if ui.small_button("🗑").on_hover_text("Remove the table").clicked() {
                    remove = Some(k);
                }
            });
        }
        if let Some(k) = remove {
            fig.data.remove(k);
            // layers of later tables keep their table; layers of the removed one show the boundary fluxes
            for p in &mut fig.panels {
                for l in &mut p.layers {
                    if let Source::DataFile(i) = l.source {
                        if i == k {
                            *l = Layer { visible: l.visible, ..Layer::default() };
                        } else if i > k {
                            l.source = Source::DataFile(i - 1);
                        }
                    }
                }
            }
        }
    });
}

fn runs_section(ui: &mut Ui, st: &mut FigState, dc: &DataCtx, runs: &[SavedRun], acts: &mut Vec<Act>) {
    CollapsingHeader::new(RichText::new("Runs for comparison").strong()).show(ui, |ui| {
        ui.label(RichText::new("Keep the current results under a name, change the input, run again and show both: choose the runs in the layer settings. Kept runs are not saved with the project; save them as files.").weak());
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.run_name).desired_width(150.0).hint_text(format!("Run {}", runs.len() + 1))).tip("Run name");
            if ui.add_enabled(dc.results.is_some(), egui::Button::new("Keep current results")).tip("Keep current results").clicked() {
                acts.push(Act::KeepRun(st.run_name.clone()));
            }
        });
        for (k, r) in runs.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}  ({}, {} time levels)", r.name, r.project.title, r.results.tlevel.len()));
                if ui.small_button("Save…").on_hover_text("Save the run as a file (JSON)").clicked() {
                    acts.push(Act::SaveRun(k));
                }
                if ui.small_button("🗑").on_hover_text("Remove the run").clicked() {
                    acts.push(Act::RemoveRun(k));
                }
            });
        }
        if ui.button("Load run…").tip("Load run").clicked() {
            acts.push(Act::LoadRun);
        }
    });
}

fn symbols_section(ui: &mut Ui, st: &mut FigState) {
    CollapsingHeader::new(RichText::new("Symbols and markup").strong()).show(ui, |ui| {
        ui.label(RichText::new("Click a symbol to copy it, then paste it into a text with Ctrl+V.").weak());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            for s in SYMBOLS.split(' ') {
                if ui.button(RichText::new(s).size(15.0)).clicked() {
                    ui.ctx().copy_text(s.to_string());
                    st.status = format!("{s} copied to the clipboard: paste it with Ctrl+V");
                }
            }
        });
        ui.add_space(4.0);
        egui::Grid::new("fig_markup").num_columns(2).striped(true).show(ui, |ui| {
            for (m, r) in [
                ("θ_{s}", "subscript: θₛ"),
                ("cm^{3}", "superscript: cm³"),
                ("*K*", "italic"),
                ("**Loam**", "bold"),
                ("\\*", "a literal *"),
                ("line 1 ⏎ line 2", "a new line (title, caption)"),
            ] {
                ui.monospace(m);
                ui.label(r);
                ui.end_row();
            }
        });
    });
}

fn preview(ui: &mut Ui, st: &mut FigState, fig: &mut Figure, dc: &DataCtx, env: u64) {
    let ctx = ui.ctx().clone();
    let wpt = (fig.width_mm.clamp(10.0, 2000.0) * PT_PER_MM) as f32;
    let hpt = (fig.height_mm.clamp(10.0, 2000.0) * PT_PER_MM) as f32;
    let avail = ui.available_size() - egui::vec2(16.0, 16.0);
    let scale = (avail.x / wpt).min(avail.y / hpt).max(0.05);
    let ppp = ctx.pixels_per_point();
    let px_per_pt = scale * ppp;
    let tables = std::mem::take(&mut fig.data);
    let json = serde_json::to_vec(&*fig).unwrap_or_default();
    fig.data = tables;
    let key = hash_of(&(env, json, fig.data.iter().map(|t| (t.name.clone(), t.columns.len())).collect::<Vec<_>>(), (px_per_pt * 1000.0) as i64));
    if key != st.key || st.tex.is_none() {
        let series: Vec<Vec<Vec<data::Series>>> = fig.panels.iter().map(|p| p.layers.iter().map(|l| dc.series(l, fig)).collect()).collect();
        let r = render::render(fig, &series);
        let pm = canvas::rasterize(&r.drawing, px_per_pt, false);
        let img = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data());
        match &mut st.tex {
            Some(t) => t.set(img, egui::TextureOptions::LINEAR),
            None => st.tex = Some(ctx.load_texture("figure_preview", img, egui::TextureOptions::LINEAR)),
        }
        st.rendered = Some(r);
        st.key = key;
    }
    let (Some(tex), Some(r)) = (&st.tex, &st.rendered) else { return };
    let size = egui::vec2(wpt * scale, hpt * scale);
    let resp = ui
        .vertical_centered(|ui| {
            ui.add_space(4.0);
            ui.add(egui::Image::new((tex.id(), size)).sense(egui::Sense::click()))
        })
        .inner;
    ui.painter().rect_stroke(resp.rect.expand(1.0), 0.0, egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Outside);
    let to_pt = |p: egui::Pos2| ((p.x - resp.rect.min.x) / scale, (p.y - resp.rect.min.y) / scale);
    let panel_at = |x: f32, y: f32| r.panels.iter().position(|g| x >= g.rect[0] && x <= g.rect[2] && y >= g.rect[1] && y <= g.rect[3]);
    st.hover = match resp.hover_pos() {
        Some(pos) => {
            let (x, y) = to_pt(pos);
            match panel_at(x, y) {
                Some(k) => {
                    let g = &r.panels[k];
                    let rel = ((x - g.rect[0]) / (g.rect[2] - g.rect[0]), (g.rect[3] - y) / (g.rect[3] - g.rect[1]));
                    let y2 = g.y2.map(|s| format!(", right y = {}", sig(s.from_pt(y)))).unwrap_or_default();
                    format!("panel {}: x = {}, y = {}{y2}  (relative {:.2}, {:.2})", k + 1, sig(g.x.from_pt(x)), sig(g.y.from_pt(y)), rel.0, rel.1)
                }
                None => format!("{:.1} × {:.1} mm", x as f64 / PT_PER_MM, y as f64 / PT_PER_MM),
            }
        }
        None => {
            let px = |pt: f32| (pt as f64 * fig.dpi / 72.0).round();
            let mut s = format!("{:.0} × {:.0} mm, PNG {} × {} pixels at {} dpi", fig.width_mm, fig.height_mm, px(wpt), px(hpt), fig.dpi);
            if !r.notes.is_empty() {
                s.push_str(&format!(" – {}", r.notes.join(" ")));
            }
            s
        }
    };
    // clicks: select the panel, or place the selected annotation (right click: the end of an arrow)
    let clicked = if resp.clicked() { Some(false) } else if resp.secondary_clicked() { Some(true) } else { None };
    if let (Some(second), Some(pos)) = (clicked, resp.interact_pointer_pos()) {
        let (x, y) = to_pt(pos);
        if let Some(k) = panel_at(x, y) {
            let g = r.panels[k].clone();
            if st.place && k == st.panel {
                if let Some(a) = fig.panels[k].annotations.get_mut(st.ann) {
                    let ys = if a.on_y2 { g.y2.unwrap_or(g.y) } else { g.y };
                    let (vx, vy) = if a.relative {
                        (((x - g.rect[0]) / (g.rect[2] - g.rect[0])) as f64, ((g.rect[3] - y) / (g.rect[3] - g.rect[1])) as f64)
                    } else {
                        (g.x.from_pt(x), ys.from_pt(y))
                    };
                    let round = |v: f64| if a.relative { (v * 1000.0).round() / 1000.0 } else { sig(v).parse().unwrap_or(v) };
                    if second {
                        (a.x2, a.y2) = (round(vx), round(vy));
                    } else {
                        (a.x, a.y) = (round(vx), round(vy));
                    }
                }
            } else if k != st.panel {
                st.panel = k;
                st.layer = 0;
                st.ann = 0;
            }
        }
    }
    if st.place {
        ctx.set_cursor_icon(if resp.hovered() { egui::CursorIcon::Crosshair } else { egui::CursorIcon::Default });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_and_hashes() {
        assert_eq!(file_name("Figure 1: θ/h", "png"), "Figure 1_ θ_h.png");
        assert_eq!(file_name("///", "svg"), "___.svg");
        let mut a = vec![Figure::default()];
        let h = figures_hash(&mut a);
        a[0].title = "x".into();
        assert_ne!(h, figures_hash(&mut a));
    }
}
