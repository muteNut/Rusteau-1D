//! Rosetta dialog: van Genuchten parameters and Ks predicted from texture, bulk density and two water contents
//! (the "Rosetta Lite" dialog of HYDRUS-1D, with Rosetta 3 as the default model).

use crate::util::*;
use crate::App;
use eframe::egui::{self, Color32, RichText};
use rusteau_core::rosetta::*;
use rusteau_core::SoilMaterial;

/// Where the predicted parameters go.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RosettaTarget {
    /// material of the soil page
    Material(usize),
    /// initial estimates of the parameter estimation (material)
    Inverse(usize),
}

pub struct RosettaDlg {
    pub target: RosettaTarget,
    pub version: RosettaVersion,
    /// model code 1 (textural classes) .. 5
    pub model: u8,
    /// textural class of model 1 (index in `TEXTURAL_CLASSES`)
    pub class: usize,
    pub sand: f64,
    pub silt: f64,
    pub clay: f64,
    pub bd: f64,
    pub th33: f64,
    pub th1500: f64,
    pub geometric: bool,
}

impl RosettaDlg {
    pub fn new(target: RosettaTarget) -> Self {
        RosettaDlg { target, version: RosettaVersion::V3, model: 2, class: 2, sand: 40.0, silt: 40.0, clay: 20.0, bd: 1.4, th33: 0.3, th1500: 0.1, geometric: false }
    }

    fn data(&self) -> SoilData {
        SoilData {
            sand: self.sand,
            silt: self.silt,
            clay: self.clay,
            bulk_density: (self.model >= 3).then_some(self.bd),
            th33: (self.model >= 4).then_some(self.th33),
            th1500: (self.model >= 5).then_some(self.th1500),
        }
    }

    fn prediction(&self) -> Result<Prediction, String> {
        if self.model == 1 {
            let (c, _) = TEXTURAL_CLASSES[self.class.min(TEXTURAL_CLASSES.len() - 1)];
            return class_average(c).ok_or_else(|| format!("no class averages for {c}"));
        }
        let est = if self.geometric { Estimate::Geometric } else { Estimate::Arithmetic };
        predict(self.version, &self.data(), Some(self.model), est)
    }
}

/// Writes θr, θs, α, n and Ks (α in 1/cm, Ks in cm/day) into a material in the project units.
fn apply(m: &mut SoilMaterial, model: rusteau_core::SoilModel, p: &Prediction, lf: f64, tf: f64) {
    m.qr = p.theta_r;
    m.qs = p.theta_s;
    m.alpha = p.alpha / lf;
    m.n = p.n;
    m.ks = p.ks * lf / tf;
    // wetting branch and model extensions keep their relation to the main curve
    m.qm = m.qm.max(m.qs);
    m.qs_w = m.qs;
    m.alpha_w = 2.0 * m.alpha;
    m.ks_w = m.ks;
    if model == rusteau_core::SoilModel::ModifiedVG {
        // θm, θa, θk, Kk of the unmodified curve
        m.extra[0] = m.qs;
        m.extra[1] = m.qr;
        m.extra[2] = m.qs;
        m.extra[3] = m.ks;
    }
}

impl App {
    pub fn rosetta_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.ros.take() else { return };
        let mut open = true;
        let mut accept = false;
        let mut cancel = false;
        let (lf, tf) = (cm_to_unit(&self.prj), day_to_unit(&self.prj));
        let model = self.prj.water.model;
        egui::Window::new("Neural network prediction (Rosetta)").open(&mut open).collapsible(false).resizable(false).min_width(760.0).show(ctx, |ui| {
            let target = match d.target {
                RosettaTarget::Material(i) => format!("material {} ({})", i + 1, self.prj.water.materials.get(i).map(|m| m.name.as_str()).unwrap_or("")),
                RosettaTarget::Inverse(i) => format!("initial estimates of material {}", i + 1),
            };
            ui.label(RichText::new(format!("Predicts θr, θs, α, n and Ks for {target}.")).weak());
            ui.horizontal(|ui| {
                ui.label("Model");
                ui.add_enabled_ui(d.model >= 2, |ui| {
                    egui::ComboBox::from_id_salt("ros_ver").width(340.0).selected_text(d.version.name()).show_ui(ui, |ui| {
                        for v in RosettaVersion::ALL {
                            ui.selectable_value(&mut d.version, v, v.name());
                        }
                    });
                });
            });
            ui.add_space(4.0);
            ui.columns(2, |cols| {
                let ui = &mut cols[0];
                ui.label(RichText::new("Input data").strong());
                for code in 1..=5u8 {
                    let was = d.model;
                    ui.radio_value(&mut d.model, code, model_name(code));
                    if code == 1 && d.model == 1 && was != 1 {
                        // start from the class of the texture entered
                        let c = texture_class(d.sand, d.silt, d.clay);
                        d.class = TEXTURAL_CLASSES.iter().position(|(n, _)| *n == c).unwrap_or(d.class);
                    }
                }
                if d.model == 1 {
                    ui.horizontal(|ui| {
                        ui.label("Textural class");
                        egui::ComboBox::from_id_salt("ros_class").selected_text(TEXTURAL_CLASSES[d.class].0).show_ui(ui, |ui| {
                            for (k, (c, _)) in TEXTURAL_CLASSES.iter().enumerate() {
                                ui.selectable_value(&mut d.class, k, *c);
                            }
                        });
                    });
                    ui.label(RichText::new("Average parameters of the class in the Rosetta database (Schaap et al., 2001), as Rosetta Lite of HYDRUS-1D.").weak());
                }
                ui.add_space(4.0);
                egui::Grid::new("ros_in").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                    let row = |ui: &mut egui::Ui, label: &str, v: &mut f64, unit: &str, on: bool, speed: f64| {
                        ui.label(label);
                        ui.add_enabled(on, egui::DragValue::new(v).speed(speed).max_decimals(4));
                        ui.label(unit);
                        ui.end_row();
                    };
                    let tx = d.model >= 2;
                    row(ui, "Sand", &mut d.sand, "%", tx, 0.5);
                    row(ui, "Silt", &mut d.silt, "%", tx, 0.5);
                    row(ui, "Clay", &mut d.clay, "%", tx, 0.5);
                    row(ui, "Bulk density", &mut d.bd, "g/cm³", d.model >= 3, 0.01);
                    row(ui, "θ at 33 kPa (field capacity)", &mut d.th33, "cm³/cm³", d.model >= 4, 0.005);
                    row(ui, "θ at 1500 kPa (wilting point)", &mut d.th1500, "cm³/cm³", d.model >= 5, 0.005);
                });
                let sum = d.sand + d.silt + d.clay;
                if (99.0..=101.0).contains(&sum) {
                    ui.label(RichText::new(format!("USDA texture class: {}", texture_class(d.sand, d.silt, d.clay))).weak());
                } else {
                    ui.colored_label(Color32::from_rgb(230, 120, 50), format!("sand + silt + clay = {sum:.1} %"));
                }
                ui.add_enabled(d.model >= 2, egui::Checkbox::new(&mut d.geometric, "geometric means of α, n and Ks"))
                    .on_hover_text("Off: arithmetic means of the ensemble (as rosetta-soil and the Rosetta web site). On: 10 to the mean of the logarithms.");
                let ui = &mut cols[1];
                ui.label(RichText::new("Prediction").strong());
                match d.prediction() {
                    Ok(p) => {
                        egui::Grid::new("ros_out").num_columns(3).striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                            ui.strong("");
                            ui.strong("value");
                            ui.strong("± std. dev.");
                            ui.end_row();
                            let rows = [
                                ("θr [-]", p.theta_r, p.std[0]),
                                ("θs [-]", p.theta_s, p.std[1]),
                                ("α [1/cm]", p.alpha, p.std[2]),
                                ("n [-]", p.n, p.std[3]),
                                ("Ks [cm/day]", p.ks, p.std[4]),
                                ("K0 [cm/day]", p.k0, p.std[5]),
                                ("L [-]", p.l, p.std[6]),
                            ];
                            for (k, (name, v, s)) in rows.iter().enumerate() {
                                let t = |x: &str| if k >= 5 { RichText::new(x).weak() } else { RichText::new(x) };
                                let num = |x: f64| if x.is_finite() { sig(x) } else { "–".into() };
                                ui.label(t(name));
                                ui.label(t(&num(*v)));
                                ui.label(t(&num(*s)));
                                ui.end_row();
                            }
                        });
                        ui.label(
                            RichText::new("K0 and L belong to the conductivity function with a matching point (Schaap & Leij 2000); Accept takes θr, θs, α, n and Ks (l stays).")
                                .weak(),
                        );
                    }
                    Err(e) => {
                        ui.colored_label(Color32::from_rgb(230, 120, 50), e);
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                let ok = d.prediction().is_ok();
                if ui.add_enabled(ok, egui::Button::new("Accept")).clicked() {
                    accept = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if accept {
            if let Ok(p) = d.prediction() {
                match d.target {
                    RosettaTarget::Material(i) => {
                        if let Some(m) = self.prj.water.materials.get_mut(i) {
                            apply(m, model, &p, lf, tf);
                            self.dirty = true;
                            self.status = if d.model == 1 {
                                format!("Rosetta: class averages of {} for material {}", TEXTURAL_CLASSES[d.class].0, i + 1)
                            } else {
                                format!("Rosetta: parameters of material {} predicted ({}, {})", i + 1, texture_class(d.sand, d.silt, d.clay), d.version.name())
                            };
                        }
                    }
                    RosettaTarget::Inverse(mat) => {
                        let mut q = self.prj.clone();
                        if let Some(m) = q.water.materials.get_mut(mat) {
                            apply(m, model, &p, lf, tf);
                        }
                        if let Some(s) = self.prj.inverse.as_mut() {
                            for i in 0..s.params.len() {
                                let par = s.params[i].clone();
                                if par.group == rusteau_core::inverse::ParamGroup::Water && par.mat == mat && (1..=5).contains(&par.index) {
                                    s.params[i].value = s.get_value(&q, &par);
                                }
                            }
                            self.dirty = true;
                            self.status = format!("Rosetta: initial estimates of material {} predicted", mat + 1);
                        }
                    }
                }
            }
        }
        if open && !accept && !cancel {
            self.ros = Some(d);
        }
    }
}
