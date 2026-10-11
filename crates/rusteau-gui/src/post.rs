//! Post-processing pages.

use crate::hydro::{hyd_plot, HydView, Kind};
use crate::measured::{Measured, Target};
use crate::tips::{hdr, lbl, TipExt};
use crate::util::*;
use crate::App;
use egui::{Color32, ComboBox, Ui};
use egui_plot::{Legend, Line, MarkerShape, Plot, PlotPoints, Points};

fn pl(pts: Vec<[f64; 2]>, name: &str, c: Color32) -> Line<'static> {
    Line::new(PlotPoints::from(pts)).name(name).color(c)
}

fn mpoints(pts: Vec<[f64; 2]>, name: &str, c: Color32) -> Points<'static> {
    Points::new(pts).name(name).shape(MarkerShape::Circle).radius(3.5_f32).filled(true).color(c)
}

impl App {
    fn no_results(&self, ui: &mut Ui) -> bool {
        if self.results.is_none() {
            ui.label("No results yet – run the calculation first (F5).");
            true
        } else {
            false
        }
    }

    /// "Measured data" row: load a two-column text file for `target`, list the loaded sets of the targets accepted by
    /// `belongs`, with a switch to show / hide and a button to remove them.
    fn measured_row(&mut self, ui: &mut Ui, target: Target, what: &str, belongs: impl Fn(&Target) -> bool, obs_nodes: Option<&[String]>) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Measured data:");
            if let Some(names) = obs_nodes {
                // observation node the next loaded file belongs to
                ComboBox::from_id_salt("obs_meas").selected_text(names.get(self.post.obs_meas).map(|s| s.as_str()).unwrap_or("")).show_ui(ui, |ui| {
                    for (k, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut self.post.obs_meas, k, n);
                    }
                });
            }
            if ui.button("Load…").on_hover_text(format!("Text or CSV file with two columns: {what}")).clicked() {
                self.measured_target = Some(target);
                self.dialog = Some(crate::files::FileDialog::new(
                    crate::files::Purpose::ImportMeasured,
                    crate::files::Mode::OpenFile,
                    &format!("Measured data ({what})"),
                    self.last_dir.clone(),
                    "",
                    &[],
                ));
            }
            let mut remove = None;
            for (i, m) in self.measured.iter_mut().enumerate() {
                if !belongs(&m.target) {
                    continue;
                }
                let at = match m.target {
                    Target::Profile { t, .. } => format!(", t = {}", fmt_t(t)),
                    Target::Obs { obs, .. } => obs_nodes.and_then(|n| n.get(obs)).map(|n| format!(", {n}")).unwrap_or_default(),
                    _ => String::new(),
                };
                ui.checkbox(&mut m.show, format!("{}{at} ({} points)", m.name, m.pts.len()));
                if ui.small_button("✖").on_hover_text("remove").clicked() {
                    remove = Some(i);
                }
            }
            if let Some(i) = remove {
                self.measured.remove(i);
            }
        });
    }

    pub fn page_obs(&mut self, ui: &mut Ui) {
        page_title(ui, "Observation points");
        if self.no_results(ui) {
            return;
        }
        let r = self.results.as_ref().unwrap();
        if r.obs.is_empty() {
            ui.label("No observation nodes were defined (add them on the page “Soil profile”).");
            return;
        }
        let nobs = r.obs[0].points.len();
        if self.post.obs_sel.len() != nobs {
            self.post.obs_sel = vec![true; nobs];
        }
        let lu = self.prj.units.length_str();
        let tu = self.prj.units.time_str();
        let cu = conc_unit(&self.prj);
        let mut vars = vec![format!("Pressure head ({lu})"), "Water content (–)".into(), "Temperature (°C)".into(), format!("Flux ({lu}/{tu})")];
        for j in 0..r.n_solutes {
            vars.push(format!("Concentration {} ({cu})", j + 1));
        }
        self.post.obs_var = self.post.obs_var.min(vars.len() - 1);
        ui.horizontal(|ui| {
            lbl(ui, "Variable");
            ComboBox::from_id_salt("ov").width(260.0).selected_text(&vars[self.post.obs_var]).show_ui(ui, |ui| {
                for (i, v) in vars.iter().enumerate() {
                    ui.selectable_value(&mut self.post.obs_var, i, v);
                }
            });
        });
        let nodes: Vec<(usize, f64)> = r.obs[0].points.iter().map(|p| (p.node, self.prj.profile.nodes.get(p.node - 1).map(|n| n.depth).unwrap_or(0.0))).collect();
        ui.horizontal_wrapped(|ui| {
            for (k, (node, d)) in nodes.iter().enumerate() {
                ui.checkbox(&mut self.post.obs_sel[k], format!("node {} ({} {})", node, fmt_t(*d), lu));
            }
        });
        let v = self.post.obs_var;
        let names: Vec<String> = nodes.iter().map(|(n, _)| format!("node {n}")).collect();
        self.post.obs_meas = self.post.obs_meas.min(nobs - 1);
        let k_sel = self.post.obs_meas;
        let node_txt = format!("time ({tu}), {} at observation node {}", vars[v], nodes[k_sel].0);
        self.measured_row(ui, Target::Obs { var: v, obs: k_sel }, &node_txt, |t| matches!(t, Target::Obs { var, .. } if *var == v), Some(&names));
        let r = self.results.as_ref().unwrap();
        Plot::new("obs")
            .height((ui.available_height() - 24.0).max(400.0))
            .legend(Legend::default().follow_insertion_order(true))
            .x_axis_label(format!("time ({tu})"))
            .y_axis_label(vars[v].clone())
            .show(ui, |pu| {
                for k in 0..nobs {
                    if !self.post.obs_sel[k] {
                        continue;
                    }
                    let node = r.obs[0].points[k].node;
                    let pts: Vec<[f64; 2]> = r
                        .obs
                        .iter()
                        .map(|o| {
                            let p = &o.points[k];
                            let y = match v {
                                0 => p.h,
                                1 => p.theta,
                                2 => p.temp,
                                3 => p.flux,
                                j => p.conc.get(j - 4).copied().unwrap_or(0.0),
                            };
                            [o.t, y]
                        })
                        .collect();
                    let col = PALETTE[k % PALETTE.len()];
                    pu.line(pl(pts, &format!("node {}", node), col));
                    for m in self.measured.iter().filter(|m| m.show && m.target == Target::Obs { var: v, obs: k }) {
                        pu.points(mpoints(m.pts.clone(), &m.name, col));
                    }
                }
            });
    }

    pub fn page_profile_info(&mut self, ui: &mut Ui) {
        page_title(ui, "Profiles");
        if self.no_results(ui) {
            return;
        }
        let r = self.results.as_ref().unwrap();
        let np = r.profiles.len();
        if np == 0 {
            ui.label("No profiles were stored (print times).");
            return;
        }
        if self.post.prof_sel.len() != np {
            self.post.prof_sel = (0..np).map(|i| np <= 12 || i == 0 || i + 1 == np || i % (np / 8 + 1) == 0).collect();
            self.post.prof_idx = 0;
        }
        self.post.prof_idx = self.post.prof_idx.min(np - 1);
        let lu = self.prj.units.length_str();
        let tu = self.prj.units.time_str();
        let cu = conc_unit(&self.prj);
        let mut vars: Vec<String> = vec![
            format!("Pressure head ({lu})"),
            "Water content (–)".into(),
            format!("Hydraulic conductivity ({lu}/{tu})"),
            format!("Water capacity (1/{lu})"),
            format!("Flux ({lu}/{tu})"),
            format!("Root water uptake (1/{tu})"),
            "v / Ks(top) (–)".into(),
            "Temperature (°C)".into(),
        ];
        for j in 0..r.n_solutes {
            vars.push(format!("Concentration {} ({cu})", j + 1));
        }
        for j in 0..r.n_solutes {
            vars.push(format!("Sorbed / immobile conc. {}", j + 1));
        }
        self.post.prof_var = self.post.prof_var.min(vars.len() - 1);
        let times: Vec<f64> = r.profiles.iter().map(|p| p.t).collect();
        ui.horizontal(|ui| {
            lbl(ui, "Variable");
            ComboBox::from_id_salt("pvv").width(260.0).selected_text(&vars[self.post.prof_var]).show_ui(ui, |ui| {
                for (i, v) in vars.iter().enumerate() {
                    ui.selectable_value(&mut self.post.prof_var, i, v);
                }
            });
            ui.separator();
            ui.selectable_value(&mut self.post.prof_single, false, "Several times").tip("Several times");
            ui.selectable_value(&mut self.post.prof_single, true, "Time slider").tip("Time slider");
        });
        if self.post.prof_single {
            ui.horizontal(|ui| {
                let play = if self.post.prof_play { "⏸" } else { "▶" };
                if ui.button(play).on_hover_text("play through the print times").clicked() {
                    self.post.prof_play = !self.post.prof_play;
                    if self.post.prof_play && self.post.prof_idx + 1 == np {
                        self.post.prof_idx = 0;
                    }
                }
                if ui.button("|◀").on_hover_text("previous print time").clicked() {
                    self.post.prof_idx = self.post.prof_idx.saturating_sub(1);
                    self.post.prof_play = false;
                }
                if ui.button("▶|").on_hover_text("next print time").clicked() {
                    self.post.prof_idx = (self.post.prof_idx + 1).min(np - 1);
                    self.post.prof_play = false;
                }
                let w = (ui.available_width() - 180.0).clamp(120.0, 600.0);
                ui.spacing_mut().slider_width = w;
                if ui.add(egui::Slider::new(&mut self.post.prof_idx, 0..=np - 1).show_value(false)).changed() {
                    self.post.prof_play = false;
                }
                ui.strong(format!("t = {} {}", fmt_t(times[self.post.prof_idx]), tu));
            });
            if self.post.prof_play {
                let now = ui.input(|i| i.time);
                if now - self.post.prof_tick >= 0.35 {
                    self.post.prof_tick = now;
                    if self.post.prof_idx + 1 < np {
                        self.post.prof_idx += 1;
                    } else {
                        self.post.prof_play = false;
                    }
                }
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                if ui.button("All times").clicked() {
                    self.post.prof_sel.iter_mut().for_each(|s| *s = true);
                }
                if ui.button("None").clicked() {
                    self.post.prof_sel.iter_mut().for_each(|s| *s = false);
                }
                for (k, t) in times.iter().enumerate() {
                    ui.checkbox(&mut self.post.prof_sel[k], format!("t = {}", fmt_t(*t)));
                }
            });
        }
        let v = self.post.prof_var;
        let shown: Vec<usize> = if self.post.prof_single { vec![self.post.prof_idx] } else { (0..np).filter(|k| self.post.prof_sel[*k]).collect() };
        // measured profiles are loaded for the first shown time
        let t_meas = shown.first().map(|k| times[*k]).unwrap_or(times[0]);
        let what = format!("depth ({lu}), {} at t = {}", vars[v], fmt_t(t_meas));
        self.measured_row(ui, Target::Profile { var: v, t: t_meas }, &what, |t| matches!(t, Target::Profile { var, .. } if *var == v), None);
        let r = self.results.as_ref().unwrap();
        let ns = r.n_solutes;
        let same_t = |a: f64, b: f64| (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0);
        let value = |n: &rusteau_core::output::NodeOut| -> f64 {
            match v {
                0 => n.h,
                1 => n.theta,
                2 => n.k,
                3 => n.c,
                4 => n.flux,
                5 => n.sink,
                6 => n.v_over_ks,
                7 => n.temp,
                j if j < 8 + ns => n.conc.get(j - 8).copied().unwrap_or(0.0),
                j => n.sorb.get(j - 8 - ns).copied().unwrap_or(0.0),
            }
        };
        let mut plot = Plot::new("profplot")
            .height((ui.available_height() - 24.0).max(400.0))
            .legend(Legend::default().follow_insertion_order(true))
            .x_axis_label(vars[v].clone())
            .y_axis_label(format!("depth ({lu})"))
            .y_axis_formatter(|m, _| {
                let d = -m.value;
                sig(if d.abs() < 1e-12 { 0.0 } else { d })
            })
            .label_formatter(|name, pt| {
                let head = if name.is_empty() { String::new() } else { format!("{name}\n") };
                format!("{head}value = {}\ndepth = {}", sig(pt.x), sig(-pt.y))
            });
        if self.post.prof_single {
            // fixed axes over all print times, so that the profile moves and the axes stay
            let (lo, hi) = r.profiles.iter().flat_map(|p| p.nodes.iter().map(value)).filter(|x| x.is_finite()).fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
            if lo <= hi {
                let pad = 0.03 * (hi - lo).max(1e-12 * hi.abs().max(1e-30));
                plot = plot.include_x(lo - pad).include_x(hi + pad);
            }
        }
        plot.show(ui, |pu| {
            for (ci, &k) in shown.iter().enumerate() {
                let p = &r.profiles[k];
                let pts: Vec<[f64; 2]> = p.nodes.iter().map(|n| [value(n), -n.depth]).collect();
                let col = if self.post.prof_single { PALETTE[0] } else { PALETTE[ci % PALETTE.len()] };
                pu.line(pl(pts, &format!("t = {}", fmt_t(p.t)), col));
                for m in self.measured.iter().filter(|m| m.show && matches!(m.target, Target::Profile { var, t } if var == v && same_t(t, p.t))) {
                    let pts = m.pts.iter().map(|q| [q[1], -q[0].abs()]).collect();
                    pu.points(mpoints(pts, &m.name, col));
                }
            }
        });
    }

    pub fn page_fluxes(&mut self, ui: &mut Ui) {
        page_title(ui, "Boundary fluxes, heads and cumulative quantities");
        if self.no_results(ui) {
            return;
        }
        let r = self.results.as_ref().unwrap();
        let lu = self.prj.units.length_str();
        let tu = self.prj.units.time_str();
        let mu = {
            let m = self.prj.units.mass.trim();
            if m.is_empty() || m == "-" {
                "M".to_string()
            } else {
                m.to_string()
            }
        };
        let mut groups: Vec<(String, String)> = vec![
            ("Water fluxes: rTop, vTop, vBot, vRoot".into(), format!("flux ({lu}/{tu})")),
            ("Cumulative water fluxes".into(), format!("cumulative flux ({lu})")),
            ("Pressure heads: top, root zone, bottom".into(), format!("pressure head ({lu})")),
            ("Surface: runoff, cumulative infiltration/evaporation".into(), format!("rate ({lu}/{tu}) / cumulative ({lu})")),
            ("Soil water storage".into(), format!("water volume ({lu})")),
            ("Temperature at top and bottom".into(), "temperature (°C)".into()),
        ];
        for j in 0..r.n_solutes {
            groups.push((format!("Solute {}: boundary fluxes", j + 1), format!("solute flux ({mu}/{lu}²/{tu})")));
            groups.push((format!("Solute {}: cumulative fluxes", j + 1), format!("cumulative solute flux ({mu}/{lu}²)")));
            groups.push((format!("Solute {}: concentrations top / bottom", j + 1), format!("concentration ({})", conc_unit(&self.prj))));
        }
        self.post.flux_var = self.post.flux_var.min(groups.len() - 1);
        ComboBox::from_id_salt("fg").width(420.0).selected_text(&groups[self.post.flux_var].0).show_ui(ui, |ui| {
            for (i, g) in groups.iter().enumerate() {
                // "Solute 2: boundary fluxes" has the description of "Solute: boundary fluxes"
                let key = match g.0.split_once(':') {
                    Some((a, b)) if a.starts_with("Solute ") => format!("Solute:{b}"),
                    _ => g.0.clone(),
                };
                ui.selectable_value(&mut self.post.flux_var, i, &g.0).tip(&key);
            }
        });
        let g = self.post.flux_var;
        let what = format!("time ({tu}), {}", groups[g].1);
        self.measured_row(ui, Target::Flux { group: g }, &what, |t| *t == Target::Flux { group: g }, None);
        let r = self.results.as_ref().unwrap();
        let series = |f: &dyn Fn(&rusteau_core::output::TLevel) -> f64| -> Vec<[f64; 2]> { r.tlevel.iter().map(|t| [t.t, f(t)]).collect() };
        let mcol = ui.visuals().strong_text_color();
        Plot::new("fluxplot")
            .height((ui.available_height() - 24.0).max(400.0))
            .legend(Legend::default().follow_insertion_order(true))
            .x_axis_label(format!("time ({tu})"))
            .y_axis_label(groups[g].1.clone())
            .show(ui, |pu| {
                if g == 0 {
                    pu.line(pl(series(&|t| t.r_top), "rTop (potential)", PALETTE[0]));
                    pu.line(pl(series(&|t| t.v_top), "vTop (actual)", PALETTE[1]));
                    pu.line(pl(series(&|t| t.v_bot), "vBot", PALETTE[2]));
                    pu.line(pl(series(&|t| t.v_root), "vRoot (actual)", PALETTE[3]));
                    pu.line(pl(series(&|t| t.r_root), "rRoot (potential)", PALETTE[4]));
                } else if g == 1 {
                    pu.line(pl(series(&|t| t.cum_r_top), "Σ rTop", PALETTE[0]));
                    pu.line(pl(series(&|t| t.cum_v_top), "Σ vTop", PALETTE[1]));
                    pu.line(pl(series(&|t| t.cum_v_bot), "Σ vBot", PALETTE[2]));
                    pu.line(pl(series(&|t| t.cum_v_root), "Σ vRoot", PALETTE[3]));
                    pu.line(pl(series(&|t| t.cum_r_root), "Σ rRoot", PALETTE[4]));
                } else if g == 2 {
                    pu.line(pl(series(&|t| t.h_top), "h top", PALETTE[0]));
                    pu.line(pl(series(&|t| t.h_root), "h root zone", PALETTE[1]));
                    pu.line(pl(series(&|t| t.h_bot), "h bottom", PALETTE[2]));
                } else if g == 3 {
                    pu.line(pl(series(&|t| t.run_off), "runoff rate", PALETTE[0]));
                    pu.line(pl(series(&|t| t.cum_run_off), "Σ runoff", PALETTE[1]));
                    pu.line(pl(series(&|t| t.cum_infil), "Σ infiltration", PALETTE[2]));
                    pu.line(pl(series(&|t| t.cum_evap), "Σ evaporation", PALETTE[3]));
                } else if g == 4 {
                    pu.line(pl(series(&|t| t.volume), "water in profile", PALETTE[0]));
                } else if g == 5 {
                    pu.line(pl(series(&|t| t.temp_top), "T top", PALETTE[0]));
                    pu.line(pl(series(&|t| t.temp_bot), "T bottom", PALETTE[1]));
                } else {
                    let j = (g - 6) / 3;
                    let kind = (g - 6) % 3;
                    let sr = |f: &dyn Fn(&rusteau_core::output::SoluteTLevel) -> f64| -> Vec<[f64; 2]> { r.tlevel.iter().filter_map(|t| t.solutes.get(j).map(|s| [t.t, f(s)])).collect() };
                    match kind {
                        0 => {
                            pu.line(pl(sr(&|s| s.cv_top), "top flux", PALETTE[0]));
                            pu.line(pl(sr(&|s| s.cv_bot), "bottom flux", PALETTE[1]));
                            pu.line(pl(sr(&|s| s.cv_root), "root uptake", PALETTE[2]));
                        }
                        1 => {
                            pu.line(pl(sr(&|s| s.cum_top), "Σ top", PALETTE[0]));
                            pu.line(pl(sr(&|s| s.cum_bot), "Σ bottom", PALETTE[1]));
                            pu.line(pl(sr(&|s| s.cum_ch0), "Σ zero-order production", PALETTE[2]));
                            pu.line(pl(sr(&|s| s.cum_ch1), "Σ first-order decay", PALETTE[3]));
                            pu.line(pl(sr(&|s| s.cum_root), "Σ root uptake", PALETTE[4]));
                            pu.line(pl(sr(&|s| s.cum_neq), "Σ non-equilibrium exchange", PALETTE[5]));
                        }
                        _ => {
                            pu.line(pl(sr(&|s| s.c_top), "c top node", PALETTE[0]));
                            pu.line(pl(sr(&|s| s.c_bot), "c bottom node", PALETTE[1]));
                            pu.line(pl(sr(&|s| s.c_root), "c root zone", PALETTE[2]));
                        }
                    }
                }
                for m in self.measured.iter().filter(|m| m.show && m.target == Target::Flux { group: g }) {
                    pu.points(mpoints(m.pts.clone(), &m.name, mcol));
                }
            });
    }

    pub fn page_soil_props(&mut self, ui: &mut Ui) {
        page_title(ui, "Soil hydraulic properties");
        let nm = self.prj.water.materials.len();
        if nm == 0 {
            ui.label("No materials defined.");
            return;
        }
        if self.post.prop_mats.len() != nm {
            self.post.prop_mats = (0..nm).map(|i| i == 0).collect();
        }
        let kind = Kind::ALL[self.post.prop_kind.min(Kind::ALL.len() - 1)];
        let hyst = self.prj.water.hysteresis != rusteau_core::Hysteresis::None;
        ui.horizontal_wrapped(|ui| {
            ComboBox::from_id_salt("propkind").width(240.0).selected_text(kind.label()).show_ui(ui, |ui| {
                for (i, k) in Kind::ALL.iter().enumerate() {
                    ui.selectable_value(&mut self.post.prop_kind, i, k.label()).tip(k.label());
                }
            });
            if kind.x_is_h() {
                ui.checkbox(&mut self.post.prop_log_x, "log |h|").tip("log |h|");
            }
            if kind.log_y_possible() {
                ui.checkbox(&mut self.post.prop_log_y, "log y").tip("log y");
            }
            if hyst {
                ui.checkbox(&mut self.post.prop_wetting, "main wetting curve (dashed)").tip("main wetting curve (dashed)");
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Materials:");
            for i in 0..nm {
                ui.colored_label(soil_mat_color(&self.prj, i + 1), "■");
                ui.checkbox(&mut self.post.prop_mats[i], format!("{} {}", i + 1, self.prj.water.materials[i].name));
            }
        });
        let lu = self.prj.units.length_str();
        match kind {
            Kind::ThetaH => self.measured_row(ui, Target::Retention, &format!("h ({lu}), θ (–)"), |t| *t == Target::Retention, None),
            Kind::KH => {
                let tu = self.prj.units.time_str();
                self.measured_row(ui, Target::Conductivity, &format!("h ({lu}), K ({lu}/{tu})"), |t| *t == Target::Conductivity, None)
            }
            _ => {}
        }
        let mats: Vec<usize> = (0..nm).filter(|i| self.post.prop_mats[*i]).collect();
        let view = HydView { kind, log_x: self.post.prop_log_x, log_y: self.post.prop_log_y, wetting: self.post.prop_wetting };
        let h = (ui.available_height() - 40.0).max(360.0);
        hyd_plot(ui, &self.prj, &mats, &view, "props", h, &self.measured);
        if hyst {
            ui.label(egui::RichText::new("Main drying (solid) and main wetting (dashed) curves; scanning curves depend on the history and are not shown.").weak());
        }
    }

    pub fn page_runtime(&mut self, ui: &mut Ui) {
        page_title(ui, "Run-time information");
        if self.no_results(ui) {
            return;
        }
        let r = self.results.as_ref().unwrap();
        ui.label(format!(
            "{} time levels stored; wall-clock time {:.2} s; cumulative iterations: {}.",
            r.tlevel.len(),
            self.run_seconds,
            r.tlevel.last().map(|t| t.it_cum).unwrap_or(0)
        ));
        for m in &r.messages {
            ui.colored_label(egui::Color32::LIGHT_RED, m);
        }
        let tu = self.prj.units.time_str();
        let s = |f: &dyn Fn(&rusteau_core::output::TLevel) -> f64| -> Vec<[f64; 2]> { r.tlevel.iter().map(|t| [t.t, f(t)]).collect() };
        ui.columns(2, |c| {
            Plot::new("dtplot").height(260.0).x_axis_label(format!("time ({tu})")).y_axis_label(format!("time step Δt ({tu})")).show(&mut c[0], |pu| pu.line(pl(s(&|t| t.dt), "dt", PALETTE[0])));
            Plot::new("itplot").height(260.0).x_axis_label(format!("time ({tu})")).y_axis_label("iterations per step").show(&mut c[1], |pu| pu.line(pl(s(&|t| t.iter_w as f64), "iterations", PALETTE[1])));
        });
        if r.n_solutes > 0 {
            ui.columns(2, |c| {
                Plot::new("peplot").height(260.0).x_axis_label(format!("time ({tu})")).y_axis_label("max. Peclet number").show(&mut c[0], |pu| pu.line(pl(s(&|t| t.peclet), "Pe", PALETTE[2])));
                Plot::new("coplot").height(260.0).x_axis_label(format!("time ({tu})")).y_axis_label("max. Courant number").show(&mut c[1], |pu| pu.line(pl(s(&|t| t.courant), "Cr", PALETTE[3])));
            });
        }
    }

    pub fn page_balance(&mut self, ui: &mut Ui) {
        page_title(ui, "Mass balance");
        if self.no_results(ui) {
            return;
        }
        let r = self.results.as_ref().unwrap();
        let lu = self.prj.units.length_str();
        let tu = self.prj.units.time_str();
        egui::Grid::new("bal").striped(true).show(ui, |ui| {
            for h in [format!("time ({tu})"), format!("water volume ({lu})"), format!("WatBalT ({lu})"), "WatBalR (%)".into()] {
                hdr(ui, &h);
            }
            for j in 0..r.n_solutes {
                hdr(ui, &format!("mass {}", j + 1));
                hdr(ui, &format!("CncBalT {}", j + 1));
                hdr(ui, &format!("CncBalR {} (%)", j + 1));
            }
            ui.end_row();
            for b in &r.balance {
                ui.label(fmt_t(b.t));
                ui.label(format!("{:.5}", b.total.volume));
                ui.label(format!("{:.3e}", b.wat_bal_t));
                ui.label(format!("{:.4}", b.wat_bal_r));
                for j in 0..r.n_solutes {
                    ui.label(format!("{:.5}", b.total.c_vol.get(j).copied().unwrap_or(0.0)));
                    ui.label(format!("{:.3e}", b.sol_bal_t.get(j).copied().unwrap_or(0.0)));
                    ui.label(format!("{:.4}", b.sol_bal_r.get(j).copied().unwrap_or(0.0)));
                }
                ui.end_row();
            }
        });
        if let Some(b) = r.balance.last() {
            section(ui, &format!("Sub-regions at t = {} {tu}", fmt_t(b.t)));
            egui::Grid::new("sub").striped(true).show(ui, |ui| {
                for h in ["region".to_string(), format!("length ({lu})"), format!("water volume ({lu})"), format!("inflow ({lu}/{tu})"), format!("mean h ({lu})")] {
                    hdr(ui, &h);
                }
                for j in 0..r.n_solutes {
                    ui.strong(format!("solute {} mass", j + 1));
                    ui.strong(format!("solute {} mean c", j + 1));
                }
                ui.end_row();
                for (i, s) in b.sub.iter().enumerate() {
                    ui.label(format!("{}", i + 1));
                    ui.label(format!("{:.4}", s.area));
                    ui.label(format!("{:.5}", s.volume));
                    ui.label(format!("{:.4e}", s.change));
                    ui.label(format!("{:.3}", s.h_mean));
                    for j in 0..r.n_solutes {
                        ui.label(format!("{:.5}", s.c_vol.get(j).copied().unwrap_or(0.0)));
                        ui.label(format!("{:.5}", s.c_mean.get(j).copied().unwrap_or(0.0)));
                    }
                    ui.end_row();
                }
            });
        }
    }

    /// Adds measured data read from `path` for the pending target.
    pub fn import_measured(&mut self, path: &std::path::Path) {
        let Some(target) = self.measured_target.take() else { return };
        match crate::measured::read_xy(path) {
            Ok(pts) => {
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "measured".into());
                self.status = format!("Loaded {} measured points from {}", pts.len(), path.display());
                self.measured.push(Measured { name, target, pts, show: true });
            }
            Err(e) => self.status = e,
        }
    }
}
