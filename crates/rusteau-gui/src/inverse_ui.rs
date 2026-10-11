//! Parameter estimation (inverse problem): the Fit.in settings, the run with its iteration table and the results of
//! Fit.out (fitted parameters with their confidence limits, goodness of fit, observed against fitted values).

use crate::files::{FileDialog, Mode, Purpose};
use crate::tips::{hdr, lbl, TipExt};
use crate::util::*;
use crate::{App, Page};
use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Legend, Line, MarkerShape, Plot, PlotPoints, Points};
use rusteau_core::inverse::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

/// Messages of the optimization thread.
pub enum InvMsg {
    Progress(Progress),
    Done(Box<Result<InverseResult, String>>),
}

pub struct InvHandle {
    pub rx: Receiver<InvMsg>,
    pub cancel: Arc<AtomicBool>,
    pub started: Instant,
    /// iteration rows received so far (first starting point)
    pub rows: Vec<Progress>,
    /// final values of the starting points finished so far
    pub finished: Vec<Progress>,
    /// number of starting points
    pub n_starts: usize,
}

/// GUI state of the inverse pages.
#[derive(Default)]
pub struct InvState {
    pub run: Option<InvHandle>,
    pub result: Option<InverseResult>,
    /// settings the result belongs to
    pub settings: Option<InverseSettings>,
    pub mat: usize,
    /// observed-vs-fitted plot: 0 scatter, 1 against time / pressure head
    pub plot_kind: usize,
    pub plot_set: usize,
}

pub const TYPE_NAMES: [&str; 16] = [
    "0 cumulative boundary flux",
    "1 pressure head h(t) at obs. node",
    "2 water content θ(t) at obs. node",
    "3 boundary flux",
    "4 concentration c(t) at obs. node",
    "5 retention θ(h)",
    "6 conductivity K(h)",
    "7 prior α",
    "8 prior n",
    "9 prior θr",
    "10 prior θs",
    "11 prior Ks",
    "12 h(x) at print time",
    "13 θ(x) at print time",
    "14 c(x) at print time",
    "15 sorbed c(x) at print time",
];

/// Name of an observation type; type 4 holds temperatures when heat parameters are fitted (HYDRUS 4.17).
pub fn type_name(kind: i32, heat: bool) -> &'static str {
    match kind {
        4 if heat => "4 temperature T(t) at obs. node",
        k => TYPE_NAMES.get(k as usize).copied().unwrap_or("?"),
    }
}

const WEIGHTING: [&str; 3] = ["No internal weighting", "Weighting by mean ratio", "Weighting by standard deviation"];
const CONC_TYPES: [&str; 5] = ["Resident concentrations", "Log. resident concentrations", "Flux concentrations", "Total concentrations (liquid + solid)", "Resident concentrations (mobile + immobile)"];
const Q_SAME: [&str; 3] = ["θs (drying) > θs (wetting)", "θm = θs", "θm = θs(wetting) = θs"];

fn set_color(kind: i32) -> Color32 {
    const C: [Color32; 8] = [
        Color32::from_rgb(70, 140, 230),
        Color32::from_rgb(230, 120, 50),
        Color32::from_rgb(60, 180, 90),
        Color32::from_rgb(200, 70, 160),
        Color32::from_rgb(180, 160, 40),
        Color32::from_rgb(120, 100, 220),
        Color32::from_rgb(40, 180, 180),
        Color32::from_rgb(200, 60, 60),
    ];
    C[kind.rem_euclid(8) as usize]
}

/// Name of the abscissa of an observation type.
fn x_name(kind: i32) -> &'static str {
    match kind {
        0..=4 => "time",
        5 | 6 => "h",
        12..=15 => "depth",
        _ => "–",
    }
}

impl App {
    /// Starts the parameter estimation in a background thread.
    pub fn start_inverse(&mut self) {
        let Some(s) = self.prj.inverse.clone() else {
            self.status = "No inverse settings – see the Parameter estimation page.".into();
            self.page = Page::Inverse;
            return;
        };
        let problems = s.validate(&self.prj);
        if !problems.is_empty() {
            self.problems = problems;
            self.status = "The inverse settings have problems – see the Parameter estimation page.".into();
            self.page = Page::Inverse;
            return;
        }
        self.problems.clear();
        let prj = self.prj.clone();
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        let s2 = s.clone();
        std::thread::spawn(move || {
            let tx2 = tx.clone();
            let r = optimize(&prj, &s2, &MarqOptions::default(), Some(&c2), |p| {
                let _ = tx2.send(InvMsg::Progress(p.clone()));
            })
            .map_err(|e| e.to_string());
            let _ = tx.send(InvMsg::Done(Box::new(r)));
        });
        let n_starts = if s.hydrus_stop { 1 } else { 1 + s.extra_starts };
        self.inv.run = Some(InvHandle { rx, cancel, started: Instant::now(), rows: vec![], finished: vec![], n_starts });
        self.inv.settings = Some(s);
        self.status = "Parameter estimation running…".into();
        self.page = Page::InverseResults;
    }

    pub fn poll_inverse(&mut self, ctx: &egui::Context) {
        let mut done = None;
        if let Some(h) = self.inv.run.as_mut() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            while let Ok(m) = h.rx.try_recv() {
                match m {
                    InvMsg::Progress(p) if p.finished => {
                        h.finished.push(p);
                        self.status = format!("Parameter estimation: {} of {} starting points finished", h.finished.len(), h.n_starts);
                    }
                    InvMsg::Progress(p) => {
                        self.status = format!("Parameter estimation: iteration {}, SSQ {:.4e} ({} runs)", p.iteration, p.ssq, p.runs);
                        h.rows.push(p);
                    }
                    InvMsg::Done(r) => done = Some((*r, h.started.elapsed().as_secs_f64())),
                }
            }
        }
        if let Some((r, secs)) = done {
            self.inv.run = None;
            match r {
                Ok(res) => {
                    self.status = format!(
                        "Parameter estimation finished in {:.1} s: {} iterations, SSQ {:.4e}, R² {:.5} ({}).",
                        secs,
                        res.iterations.len() - 1,
                        res.ssq,
                        res.measures.r_squared,
                        stop_text(res.stop)
                    );
                    if let Some(fr) = res.final_results.clone() {
                        self.post.obs_sel.clear();
                        self.post.prof_sel.clear();
                        self.results = Some(fr);
                    }
                    if let Some(s) = &self.inv.settings {
                        let sets = crate::measured::from_fit(s);
                        self.measured.retain(|m| !m.name.starts_with("Fit.in"));
                        self.measured.extend(sets);
                    }
                    self.inv.result = Some(res);
                }
                Err(e) => self.status = format!("Parameter estimation failed: {e}"),
            }
        }
    }

    fn inverse_running(&self) -> bool {
        self.inv.run.is_some()
    }

    /// Input page: settings, parameters to fit and observations.
    pub fn page_inverse(&mut self, ui: &mut Ui, ch: &mut bool) {
        page_title(ui, "Parameter estimation (inverse problem)");
        ui.label("Soil hydraulic, solute or heat transport parameters are estimated by the Marquardt-Levenberg method from measured data, as with the Fit.in file of HYDRUS-1D.");
        ui.add_space(4.0);
        check(ui, ch, &mut self.prj.processes.inverse, "Inverse problem: estimate parameters when the calculation is run");
        if self.prj.inverse.is_none() {
            ui.add_space(6.0);
            ui.label("There are no inverse settings yet.");
            ui.horizontal(|ui| {
                if ui.button("Create from the current project").tip("Create from the current project").clicked() {
                    self.prj.inverse = Some(InverseSettings::from_project(&self.prj));
                    *ch = true;
                }
                if ui.button("Import Fit.in…").tip("Import Fit.in…").clicked() {
                    self.dialog = Some(FileDialog::new(Purpose::ImportFitIn, Mode::OpenFile, "Select a Fit.in", self.last_dir.clone(), "Fit.in", &["in"]));
                }
            });
            return;
        }
        if !self.problems.is_empty() {
            ui.add_space(4.0);
            for p in &self.problems {
                ui.colored_label(Color32::from_rgb(230, 90, 70), p);
            }
        }
        let n_mat = self.prj.water.materials.len();
        let solute = self.prj.processes.solute && !self.prj.solute.species.is_empty();
        let hyst = self.prj.water.hysteresis.code();
        let mut s = self.prj.inverse.take().unwrap();
        // keep the parameter records in line with the project (materials added or removed, solute switched on)
        sync_records(&mut s, &self.prj);

        ui.horizontal(|ui| {
            if ui.button("Import Fit.in…").tip("Import Fit.in…").clicked() {
                self.dialog = Some(FileDialog::new(Purpose::ImportFitIn, Mode::OpenFile, "Select a Fit.in", self.last_dir.clone(), "Fit.in", &["in"]));
            }
            if ui.button("Export Fit.in…").tip("Export Fit.in…").clicked() {
                self.dialog = Some(FileDialog::new(Purpose::ExportFitIn, Mode::SaveFile, "Save Fit.in", self.last_dir.clone(), "Fit.in", &["in"]));
            }
        });
        let heat = self.prj.processes.heat;
        // the groups of the 4.17 dialog "Inverse Solution": solute and heat parameters exclude each other
        section(ui, "Estimate");
        ui.horizontal(|ui| {
            check(ui, ch, &mut s.fit_water, "Soil hydraulic parameters");
            if hyst > 0 && s.fit_water {
                ui.add_space(12.0);
                ui.label(RichText::new("constraints:").weak()).tip("constraints:");
                egui::ComboBox::from_id_salt("inv_qs").selected_text(Q_SAME[s.q_same.clamp(0, 2) as usize]).show_ui(ui, |ui| {
                    for (k, n) in Q_SAME.iter().enumerate() {
                        if ui.selectable_value(&mut s.q_same, k as i32, *n).tip(n).changed() {
                            *ch = true;
                        }
                    }
                });
                check(ui, ch, &mut s.alpha_w_2d, "αw = 2 α");
                check(ui, ch, &mut s.ks_same, "Ksw = Ks");
            }
        });
        ui.add_enabled_ui(solute && !s.fit_heat, |ui| {
            check(ui, ch, &mut s.fit_solute, "Solute transport parameters (solute 1)");
        });
        let conc_used = s.fit_solute || (!s.fit_heat && s.observations.iter().any(|o| o.kind == 4 || o.kind == 14));
        ui.add_enabled_ui(conc_used, |ui| {
            ui.indent("inv_from", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("from");
                    for (k, n) in CONC_TYPES.iter().enumerate() {
                        if ui.radio_value(&mut s.conc_type, k as i32, *n).tip(n).changed() {
                            *ch = true;
                        }
                    }
                });
            });
        });
        ui.add_enabled_ui(heat && !s.fit_solute, |ui| {
            check(ui, ch, &mut s.fit_heat, "Heat transport parameters");
        });
        if s.fit_heat {
            ui.indent("inv_heat_note", |ui| {
                ui.label(RichText::new("Observations of type 4 are temperatures at observation nodes (HYDRUS uses its concentration data type for them).").weak());
            });
        }
        if !solute && !heat {
            ui.label(RichText::new("Solute and heat transport parameters can be estimated when the project simulates solute or heat transport.").weak());
        }

        section(ui, "Weighting of the measurement sets");
        ui.horizontal_wrapped(|ui| {
            for (k, n) in WEIGHTING.iter().enumerate() {
                if ui.radio_value(&mut s.weighting, k as i32, *n).tip(n).changed() {
                    *ch = true;
                }
            }
        });

        section(ui, "Iteration");
        ui.horizontal(|ui| {
            lbl(ui, "Maximum number of iterations (MIT)");
            if ui.add(egui::DragValue::new(&mut s.max_iter).range(0..=1000)).tip("Maximum number of iterations (MIT)").changed() {
                *ch = true;
            }
        });
        let mut hydrus = s.hydrus_stop;
        let rusteau_help = "The iteration runs in stages: within a stage every run uses the time levels of the run at its start, so the \
                            objective changes smoothly with the parameters (with the adaptive time steps of every run it jumps by a few \
                            per cent, and the iteration can stop far from the minimum). A new stage starts from the end point of the last \
                            one while that lowers the objective. The estimation also runs from further starting points (in parallel) and \
                            keeps the lowest objective, for problems with several local minima.";
        if ui.radio_value(&mut hydrus, false, "Rusteau: stages on frozen time levels, several starting points").on_hover_text(rusteau_help).changed() {
            *ch = true;
        }
        ui.horizontal(|ui| {
            ui.add_space(28.0);
            ui.add_enabled_ui(!hydrus, |ui| {
                ui.label("Additional starting points");
                if ui
                    .add(egui::DragValue::new(&mut s.extra_starts).range(0..=64))
                    .on_hover_text("Number of starting points besides the initial estimates (0: only the initial estimates). They run in parallel threads.")
                    .changed()
                {
                    *ch = true;
                }
            });
        });
        if ui.radio_value(&mut hydrus, true, "HYDRUS 4.17: adaptive time steps in every run, one starting point").on_hover_text("The iteration of HYDRUS 4.17, for comparisons.").changed() {
            *ch = true;
        }
        s.hydrus_stop = hydrus;
        section(ui, "Parameters");
        ui.label(RichText::new("Initial estimates, constraints (0 = no constraint) and the parameters to fit, per material.").weak());
        ui.horizontal(|ui| {
            if n_mat > 1 {
                ui.label("Material");
                let names: Vec<String> = self.prj.water.materials.iter().enumerate().map(|(k, m)| format!("{} – {}", k + 1, m.name)).collect();
                self.inv.mat = self.inv.mat.min(n_mat - 1);
                egui::ComboBox::from_id_salt("inv_mat").selected_text(names[self.inv.mat].clone()).show_ui(ui, |ui| {
                    for (k, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut self.inv.mat, k, n);
                    }
                });
            }
            let nf = s.fitted().len();
            ui.label(RichText::new(format!("{nf} parameters fitted in total")).weak());
            if ui.button("Take the initial estimates from the project").on_hover_text("Sets every initial estimate to the value of the material parameter in the project").clicked() {
                let p = s.params.clone();
                for (k, q) in p.iter().enumerate() {
                    s.params[k].value = s.get_value(&self.prj, q);
                }
                *ch = true;
            }
        });
        let mat = self.inv.mat.min(n_mat.saturating_sub(1));
        let groups = [(ParamGroup::Water, s.fit_water, "Soil hydraulic parameters"), (ParamGroup::Solute, s.fit_solute, "Solute transport parameters"), (ParamGroup::Heat, s.fit_heat, "Heat transport parameters")];
        for (g, on, title) in groups {
            let idx: Vec<usize> = s.params.iter().enumerate().filter(|(_, p)| p.group == g && p.mat == mat).map(|(i, _)| i).collect();
            if !on || idx.is_empty() {
                continue;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                if g == ParamGroup::Water {
                    ui.add_space(16.0);
                    ui.label("initial estimates from the soil catalog (Carsel & Parrish 1988):");
                    egui::ComboBox::from_id_salt("inv_cat").selected_text("choose…").show_ui(ui, |ui| {
                        for (k, c) in SOIL_CATALOG.iter().enumerate() {
                            if ui.selectable_label(false, c.0).clicked() {
                                let mut m = self.prj.water.materials.get(mat).cloned().unwrap_or_default();
                                apply_catalog(&mut m, k, cm_to_unit(&self.prj), day_to_unit(&self.prj));
                                let mut q = self.prj.clone();
                                if let Some(slot) = q.water.materials.get_mut(mat) {
                                    *slot = m;
                                }
                                for &i in &idx {
                                    if s.params[i].index <= 6 {
                                        s.params[i].value = s.get_value(&q, &s.params[i]);
                                    }
                                }
                                *ch = true;
                            }
                        }
                    });
                    if ui.button("Rosetta…").on_hover_text("Initial estimates predicted from the soil texture (Rosetta)").clicked() {
                        self.ros = Some(crate::rosetta_ui::RosettaDlg::new(crate::rosetta_ui::RosettaTarget::Inverse(mat)));
                    }
                }
            });
            egui::ScrollArea::horizontal().id_salt(("inv_par_scroll", g as u8)).show(ui, |ui| {
                egui::Grid::new(("inv_par", g as u8)).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label("");
                    for &i in &idx {
                        let p = &s.params[i];
                        ui.strong(column_label(&s, p)).on_hover_text(s.param_name(p));
                    }
                    ui.end_row();
                    lbl(ui, "Initial estimate");
                    for &i in &idx {
                        dv(ui, ch, &mut s.params[i].value);
                    }
                    ui.end_row();
                    lbl(ui, "Minimum value");
                    for &i in &idx {
                        let f = s.params[i].fit;
                        ui.add_enabled_ui(f, |ui| dv(ui, ch, &mut s.params[i].min));
                    }
                    ui.end_row();
                    lbl(ui, "Maximum value");
                    for &i in &idx {
                        let f = s.params[i].fit;
                        ui.add_enabled_ui(f, |ui| dv(ui, ch, &mut s.params[i].max));
                    }
                    ui.end_row();
                    lbl(ui, "Fitted?");
                    for &i in &idx {
                        check(ui, ch, &mut s.params[i].fit, "");
                    }
                    ui.end_row();
                });
            });
        }

        section(ui, &format!("Observations ({})", s.observations.len()));
        observation_summary(ui, &s);
        ui.add_space(4.0);
        let mut remove: Option<usize> = None;
        let heat_obs = s.observes_temperature();
        let row_h = ui.spacing().interact_size.y + 8.0;
        let n = s.observations.len();
        ui.push_id("inv_obs", |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .max_scroll_height(320.0)
                .auto_shrink([false, true])
                .column(Column::auto().at_least(40.0))
                .column(Column::auto().at_least(230.0))
                .column(Column::auto().at_least(70.0))
                .columns(Column::auto().at_least(90.0), 3)
                .column(Column::auto())
                .header(22.0, |mut h| {
                    for c in ["#", "Type", "Position", "Time / h / x", "Value", "Weight", ""] {
                        h.col(|ui| {
                            hdr(ui, c);
                        });
                    }
                })
                .body(|body| {
                    body.rows(row_h, n, |mut row| {
                        let i = row.index();
                        let o = &mut s.observations[i];
                        row.col(|ui| {
                            ui.label(format!("{}", i + 1));
                        });
                        row.col(|ui| {
                            let cur = type_name(o.kind, heat_obs);
                            egui::ComboBox::from_id_salt(("ot", i)).width(220.0).selected_text(cur).show_ui(ui, |ui| {
                                for k in 0..TYPE_NAMES.len() as i32 {
                                    if ui.selectable_value(&mut o.kind, k, type_name(k, heat_obs)).changed() {
                                        *ch = true;
                                    }
                                }
                            });
                        });
                        row.col(|ui| {
                            if ui.add(egui::DragValue::new(&mut o.pos).range(-99..=999)).on_hover_text(pos_help(o.kind)).changed() {
                                *ch = true;
                            }
                        });
                        row.col(|ui| dv(ui, ch, &mut o.x));
                        row.col(|ui| dv(ui, ch, &mut o.value));
                        row.col(|ui| dv(ui, ch, &mut o.weight));
                        row.col(|ui| {
                            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    });
                });
        });
        if let Some(i) = remove {
            s.observations.remove(i);
            *ch = true;
        }
        ui.horizontal(|ui| {
            if ui.button("+ Add observation").tip("+ Add observation").clicked() {
                let o = s.observations.last().cloned().unwrap_or(Observation { x: self.prj.time.t_max, value: 0.0, kind: 2, pos: 1, weight: 1.0 });
                s.observations.push(o);
                *ch = true;
            }
            if ui.button("Paste observations").on_hover_text("Lines “x value type position weight” from the clipboard (as in Fit.in) are appended").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                self.inv_paste = true;
            }
            if !s.observations.is_empty() && ui.button("Remove all").clicked() {
                s.observations.clear();
                *ch = true;
            }
        });
        if self.inv_paste {
            let pasted: Option<String> = ui.input(|i| i.events.iter().find_map(|e| if let egui::Event::Paste(t) = e { Some(t.clone()) } else { None }));
            if let Some(t) = pasted {
                self.inv_paste = false;
                let mut k = 0;
                for line in t.lines() {
                    let v: Vec<f64> = line.split(|c: char| c.is_whitespace() || c == ';' || c == '\t').filter(|x| !x.is_empty()).filter_map(|x| x.replace(',', ".").parse().ok()).collect();
                    if v.len() >= 2 {
                        let kind = v.get(2).copied().unwrap_or(2.0) as i32;
                        s.observations.push(Observation { x: v[0], value: v[1], kind, pos: v.get(3).copied().unwrap_or(1.0) as i32, weight: v.get(4).copied().unwrap_or(1.0) });
                        k += 1;
                    }
                }
                self.status = format!("{k} observations pasted");
                *ch |= k > 0;
            }
        }

        section(ui, "Run");
        let problems = s.validate(&self.prj);
        for p in &problems {
            ui.colored_label(Color32::from_rgb(230, 150, 60), p);
        }
        ui.horizontal(|ui| {
            let running = self.inverse_running() || self.run.is_some();
            if ui.add_enabled(!running && problems.is_empty(), egui::Button::new(RichText::new("▶ Run parameter estimation").strong())).clicked() {
                self.prj.inverse = Some(s.clone());
                self.start_inverse();
            }
            if self.inv.result.is_some() && ui.button("Show results →").clicked() {
                self.page = Page::InverseResults;
            }
        });
        if self.prj.inverse.is_none() {
            self.prj.inverse = Some(s);
        }
    }

    /// Results page (the content of Fit.out).
    pub fn page_inverse_results(&mut self, ui: &mut Ui) {
        page_title(ui, "Parameter estimation results");
        if let Some(h) = &self.inv.run {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Running for {:.0} s …", h.started.elapsed().as_secs_f64()));
                if ui.button("■ Stop").clicked() {
                    h.cancel.store(true, Ordering::Relaxed);
                }
            });
            let names = self.inv.settings.as_ref().map(|s| s.fitted().iter().map(|&i| s.param_name(&s.params[i])).collect::<Vec<_>>()).unwrap_or_default();
            if h.n_starts > 1 {
                ui.label(format!(
                    "{} starting points run in parallel; the iterations of the first (the initial estimates) are listed. Finished: {} of {}.",
                    h.n_starts,
                    h.finished.len(),
                    h.n_starts
                ));
            }
            iteration_grid(ui, "inv_it_run", &names, h.rows.iter().map(|p| (p.iteration, p.ssq, p.params.clone())).collect());
            if !h.finished.is_empty() {
                section(ui, "Finished starting points");
                let mut f = h.finished.clone();
                f.sort_by_key(|p| p.start);
                egui::Grid::new("inv_starts_run").striped(true).spacing([18.0, 3.0]).show(ui, |ui| {
                    ui.strong("Start");
                    ui.strong("SSQ");
                    for n in &names {
                        ui.strong(n);
                    }
                    ui.end_row();
                    for p in &f {
                        ui.label(format!("{}", p.start));
                        ui.label(if p.ssq.is_finite() { format!("{:.4e}", p.ssq) } else { "failed".into() });
                        for v in &p.params {
                            ui.label(format!("{v:.4e}"));
                        }
                        ui.end_row();
                    }
                });
            }
            return;
        }
        let Some(r) = self.inv.result.as_ref() else {
            ui.label("No parameter estimation has been run yet. Set it up on the Parameter estimation page.");
            return;
        };
        let s = self.inv.settings.clone().unwrap_or_default();
        ui.label(format!(
            "{} – {} iterations{}, {} model runs, {:.1} s. Water mass balance error of the final run {:.4} %.",
            stop_text(r.stop),
            r.iterations.len() - 1,
            if r.stages > 1 { format!(" in {} stages on frozen time levels", r.stages) } else { String::new() },
            r.runs,
            r.elapsed_secs,
            r.mass_balance_error
        ));
        ui.horizontal(|ui| {
            if ui.button("Apply the fitted parameters to the project").on_hover_text("Writes the fitted values into the materials and makes them the initial estimates of a new estimation").clicked() {
                self.apply_fitted();
            }
            if ui.button("Save Fit.out…").tip("Save Fit.out…").clicked() {
                self.dialog = Some(FileDialog::new(Purpose::SaveFitOut, Mode::SaveFile, "Save Fit.out", self.last_dir.clone(), "Fit.out", &["out"]));
            }
        });
        let r = self.inv.result.as_ref().unwrap();

        section(ui, "Fitted parameters");
        egui::Grid::new("inv_res").striped(true).num_columns(6).spacing([18.0, 4.0]).show(ui, |ui| {
            for h in ["Parameter", "Initial", "Fitted", "S.E.", "Lower 95 %", "Upper 95 %"] {
                hdr(ui, h);
            }
            ui.end_row();
            for (j, p) in r.params.iter().enumerate() {
                // enough digits to show the confidence interval
                let digits = if p.se > 0.0 && p.value != 0.0 { (2.0 + (p.value.abs() / p.se).log10()).ceil().clamp(4.0, 10.0) as usize } else { 4 };
                let fmt = |v: f64| fmt_digits(v, digits);
                ui.label(&p.name);
                // the initial estimates of the settings (start 0), not those of the best starting point
                let init = r.starts.first().and_then(|st| st.initial.get(j)).or_else(|| r.iterations.first().and_then(|it| it.params.get(j)));
                ui.label(init.map(|v| sig(*v)).unwrap_or_default());
                ui.label(RichText::new(fmt(p.value)).strong());
                ui.label(sig(p.se));
                ui.label(fmt(p.lower_95));
                ui.label(fmt(p.upper_95));
                ui.end_row();
            }
        });

        section(ui, "Goodness of fit");
        let m = &r.measures;
        egui::Grid::new("inv_gof").num_columns(2).spacing([18.0, 4.0]).show(ui, |ui| {
            for (l, v) in [
                ("Objective function SSQ", r.ssq),
                ("R² (observed vs fitted)", m.r_squared),
                ("Mean weighted error ME", m.me),
                ("Mean weighted absolute error MAE", m.mae),
                ("Root mean square weighted error RMSE", m.rmse),
                ("Akaike information criterion AIC", m.aic),
                ("Bayesian information criterion BIC", m.bic),
            ] {
                lbl(ui, l);
                ui.label(sig(v));
                ui.end_row();
            }
            for (k, c) in &r.contributions {
                ui.label(format!("Contribution of set {}", type_name(*k, s.observes_temperature())))
                    .tip_text("Part of the objective function SSQ contributed by this measurement set.");
                ui.label(sig(*c));
                ui.end_row();
            }
        });

        section(ui, "Observed and fitted values");
        let mut kinds: Vec<(i32, i32)> = s.observations.iter().map(|o| (o.kind, o.pos)).collect();
        kinds.sort_unstable();
        kinds.dedup();
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.inv.plot_kind, 0, "Fitted against observed").tip("Fitted against observed");
            ui.radio_value(&mut self.inv.plot_kind, 1, "Against time / pressure head").tip("Against time / pressure head");
            if self.inv.plot_kind == 1 && !kinds.is_empty() {
                self.inv.plot_set = self.inv.plot_set.min(kinds.len() - 1);
                let heat = s.observes_temperature();
                let label = |k: &(i32, i32)| format!("{} (position {})", type_name(k.0, heat), k.1);
                egui::ComboBox::from_id_salt("inv_ps").selected_text(label(&kinds[self.inv.plot_set])).show_ui(ui, |ui| {
                    for (k, kk) in kinds.iter().enumerate() {
                        ui.selectable_value(&mut self.inv.plot_set, k, label(kk));
                    }
                });
            }
        });
        let r = self.inv.result.as_ref().unwrap();
        let plot = Plot::new(("inv_plot", self.inv.plot_kind, self.inv.plot_set)).height(340.0).legend(Legend::default());
        if self.inv.plot_kind == 0 {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            plot.x_axis_label("observed").y_axis_label("fitted").data_aspect(1.0).show(ui, |pu| {
                for (g, &(kind, pos)) in kinds.iter().enumerate() {
                    let pts: Vec<[f64; 2]> = s.observations.iter().zip(&r.fitted).filter(|(o, _)| o.kind == kind && o.pos == pos).map(|(o, f)| [o.value, *f]).collect();
                    for p in &pts {
                        lo = lo.min(p[0]).min(p[1]);
                        hi = hi.max(p[0]).max(p[1]);
                    }
                    pu.points(Points::new(PlotPoints::from(pts)).radius(3.0_f32).shape(MarkerShape::Circle).color(set_color(g as i32)).name(format!("{}, position {pos}", type_name(kind, s.observes_temperature()))));
                }
                if lo.is_finite() {
                    pu.line(Line::new(PlotPoints::from(vec![[lo, lo], [hi, hi]])).color(Color32::GRAY).name("1:1"));
                }
            });
        } else if let Some(&(kind, pos)) = kinds.get(self.inv.plot_set) {
            let mut pairs: Vec<(f64, f64, f64)> = s.observations.iter().zip(&r.fitted).filter(|(o, _)| o.kind == kind && o.pos == pos).map(|(o, f)| (o.x, o.value, *f)).collect();
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            plot.x_axis_label(x_name(kind)).show(ui, |pu| {
                pu.points(Points::new(PlotPoints::from(pairs.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>())).radius(3.5_f32).color(set_color(kind)).name("observed"));
                pu.line(Line::new(PlotPoints::from(pairs.iter().map(|p| [p.0, p.2]).collect::<Vec<_>>())).color(Color32::from_rgb(230, 120, 50)).width(2.0_f32).name("fitted"));
            });
        }

        section(ui, "Correlation matrix");
        egui::Grid::new("inv_corr").striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.label("");
            for nm in &r.names {
                ui.strong(nm);
            }
            ui.end_row();
            for (i, nm) in r.names.iter().enumerate() {
                ui.strong(nm);
                for j in 0..r.names.len() {
                    if j <= i {
                        let c = r.correlation[i][j];
                        let t = RichText::new(format!("{c:.4}"));
                        ui.label(if i != j && c.abs() > 0.95 { t.color(Color32::from_rgb(230, 120, 50)) } else { t });
                    } else {
                        ui.label("");
                    }
                }
                ui.end_row();
            }
        });

        if r.starts.len() > 1 {
            section(ui, "Starting points");
            ui.label(RichText::new(format!(
                "The estimation was run from {} starting points; the result of start {} (lowest SSQ) is shown.",
                r.starts.len(),
                r.best_start
            )).weak());
            egui::ScrollArea::horizontal().id_salt("inv_starts_scroll").show(ui, |ui| {
                egui::Grid::new("inv_starts").striped(true).spacing([18.0, 3.0]).show(ui, |ui| {
                    hdr(ui, "Start");
                    for n in &r.names {
                        ui.strong(format!("{n} (initial)"));
                    }
                    hdr(ui, "Final SSQ");
                    hdr(ui, "Iterations");
                    hdr(ui, "End");
                    ui.end_row();
                    for (k, st) in r.starts.iter().enumerate() {
                        let mark = |t: String| if k == r.best_start { RichText::new(t).strong() } else { RichText::new(t) };
                        ui.label(mark(format!("{k}{}", if k == 0 { " (initial estimates)" } else { "" })));
                        for v in &st.initial {
                            ui.label(mark(format!("{v:.4e}")));
                        }
                        match &st.error {
                            Some(e) => {
                                ui.label("–");
                                ui.label("–");
                                ui.label(RichText::new(e.lines().next().unwrap_or("")).color(Color32::from_rgb(230, 120, 50)));
                            }
                            None => {
                                ui.label(mark(format!("{:.4e}", st.ssq)));
                                ui.label(mark(format!("{}", st.iterations)));
                                ui.label(mark(stop_text(st.stop).to_string()));
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        }

        section(ui, "Iterations");
        if r.starts.len() > 1 {
            ui.label(RichText::new(format!("Iterations of start {}.", r.best_start)).weak());
        }
        iteration_grid(ui, "inv_it", &r.names, r.iterations.iter().enumerate().map(|(k, it)| (k, it.ssq, it.params.clone())).collect());
    }

    /// Writes the fitted values into the project and makes them the initial estimates.
    pub fn apply_fitted(&mut self) {
        let Some(r) = self.inv.result.as_ref() else { return };
        let Some(mut s) = self.prj.inverse.clone() else { return };
        let idx = s.fitted();
        if idx.len() != r.params.len() {
            self.status = "The parameters to fit have changed since the estimation – nothing applied.".into();
            return;
        }
        let vals: Vec<f64> = r.params.iter().map(|p| p.value).collect();
        s.apply(&mut self.prj, &vals);
        for (j, &i) in idx.iter().enumerate() {
            s.params[i].value = vals[j];
        }
        self.prj.inverse = Some(s);
        self.dirty = true;
        self.status = "The fitted parameters were written into the project.".into();
    }
}

/// Value with `digits` significant digits (plain notation for moderate magnitudes).
fn fmt_digits(v: f64, digits: usize) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let a = v.abs();
    if (1e-3..1e6).contains(&a) {
        let d = (digits as i32 - 1 - a.log10().floor() as i32).max(0) as usize;
        let t = format!("{v:.d$}");
        if t.contains('.') { t.trim_end_matches('0').trim_end_matches('.').to_string() } else { t }
    } else {
        format!("{v:.prec$e}", prec = digits.saturating_sub(1))
    }
}

fn stop_text(s: StopReason) -> &'static str {
    match s {
        StopReason::Converged => "converged (no parameter changed by more than 1 %)",
        StopReason::MaxIterations => "maximum number of iterations reached",
        StopReason::NoReduction => "no further reduction in SSQ obtained",
        StopReason::Cancelled => "cancelled",
    }
}

/// Column title of a parameter (as in the parameter dialogs of HYDRUS).
fn column_label(s: &InverseSettings, p: &FitParameter) -> String {
    match p.group {
        ParamGroup::Water => {
            const W: [&str; 6] = ["θr", "θs", "α", "n", "Ks", "l"];
            match p.index {
                1..=6 => W[p.index - 1].into(),
                7..=10 if s.hysteresis > 0 && s.model != 1 => ["θm", "θsW", "αW", "KsW"][p.index - 7].into(),
                7..=10 => ["θm", "θa", "θk", "Kk"][p.index - 7].into(),
                k => format!("Par {k}"),
            }
        }
        ParamGroup::Solute => SOLUTE_LABELS.get(p.index.wrapping_sub(1)).map(|x| x.to_string()).unwrap_or_default(),
        ParamGroup::Heat => HEAT_LABELS.get(p.index.wrapping_sub(1)).map(|x| x.to_string()).unwrap_or_default(),
    }
}

fn pos_help(kind: i32) -> &'static str {
    match kind {
        0 | 3 => "1 top, 2 bottom boundary",
        1 | 4 => "observation node (negative: fracture / second solute)",
        2 => "observation node; 0 mean of the profile, -k mean of sub-region k",
        5..=11 => "material",
        12..=15 => "print time",
        _ => "",
    }
}

/// Iteration table: (iteration, SSQ, parameters).
fn iteration_grid(ui: &mut Ui, id: &str, names: &[String], rows: Vec<(usize, f64, Vec<f64>)>) {
    egui::Grid::new(id).striped(true).spacing([18.0, 3.0]).show(ui, |ui| {
        hdr(ui, "Iteration");
        hdr(ui, "SSQ");
        for n in names {
            ui.strong(n);
        }
        ui.end_row();
        for (k, ssq, p) in &rows {
            ui.label(format!("{k}"));
            ui.label(format!("{ssq:.4e}"));
            for v in p {
                ui.label(format!("{v:.4e}"));
            }
            ui.end_row();
        }
    });
}

/// Counts and weights of the measurement sets.
fn observation_summary(ui: &mut Ui, s: &InverseSettings) {
    let w = s.effective_weights();
    let mut sets: Vec<i32> = s.observations.iter().map(|o| o.kind).collect();
    sets.sort_unstable();
    sets.dedup();
    for k in sets {
        let idx: Vec<usize> = (0..s.observations.len()).filter(|&i| s.observations[i].kind == k).collect();
        let v = idx.iter().map(|&i| w[i] / s.observations[i].weight.abs().max(1e-300)).next().unwrap_or(1.0);
        ui.label(RichText::new(format!("{}: {} values, set weight {}", type_name(k, s.observes_temperature()), idx.len(), sig(v))).weak());
    }
}

/// Parameter records for every material (added or removed materials, solute transport switched on).
fn sync_records(s: &mut InverseSettings, prj: &rusteau_core::Project) {
    let n_mat = prj.water.materials.len();
    s.n_mat = n_mat;
    s.params.retain(|p| p.mat < n_mat);
    let template = InverseSettings::from_project(prj);
    for p in template.params {
        if !s.params.iter().any(|q| q.group == p.group && q.index == p.index && q.mat == p.mat) {
            s.params.push(p);
        }
    }
    let rank = |g: ParamGroup| match g {
        ParamGroup::Water => 0,
        ParamGroup::Solute => 1,
        ParamGroup::Heat => 2,
    };
    s.params.sort_by_key(|p| (rank(p.group), p.mat, p.index));
    if s.n_water_par < 10 && s.params.iter().any(|p| p.group == ParamGroup::Water && p.index > s.n_water_par) {
        s.params.retain(|p| p.group != ParamGroup::Water || p.index <= s.n_water_par);
    }
}
