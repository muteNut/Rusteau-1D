//! Soil hydraulic property curves for the viewer: θ(h), K(h), K(θ), C(h) of the main drying curve and, with
//! hysteresis, of the main wetting curve (the scaled drying curve, as in the engine).

use crate::measured::{Measured, Target};
use crate::util::*;
use egui::Ui;
use egui_plot::{Legend, Line, LineStyle, MarkerShape, Plot, PlotPoints, Points};
use rusteau_core::material::{fc, fk, fq, par_of};
use rusteau_core::{Hysteresis, Project, SoilModel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    ThetaH,
    KH,
    KTheta,
    CH,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::ThetaH, Kind::KH, Kind::KTheta, Kind::CH];
    pub fn label(self) -> &'static str {
        match self {
            Kind::ThetaH => "Water content θ(h)",
            Kind::KH => "Hydraulic conductivity K(h)",
            Kind::KTheta => "Hydraulic conductivity K(θ)",
            Kind::CH => "Water capacity C(h)",
        }
    }
    /// The y axis can be logarithmic
    pub fn log_y_possible(self) -> bool {
        matches!(self, Kind::KH | Kind::KTheta | Kind::CH)
    }
    /// The x axis is a pressure head
    pub fn x_is_h(self) -> bool {
        !matches!(self, Kind::KTheta)
    }
}

pub struct HydView {
    pub kind: Kind,
    pub log_x: bool,
    pub log_y: bool,
    pub wetting: bool,
}

/// Curve values at the pressure heads |h| (positive).
pub struct Curve {
    pub h: Vec<f64>,
    pub th: Vec<f64>,
    pub k: Vec<f64>,
    pub c: Vec<f64>,
}

/// Main drying curve (the only curve without hysteresis) and, with hysteresis, the main wetting curve of material `mi`.
pub fn curves(p: &Project, mi: usize) -> (Curve, Option<Curve>) {
    let xc = p.units.x_conv();
    let model = p.water.model;
    let hs = log_space(1e-4 * xc, 1e5 * xc, 300);
    if model == SoilModel::Tabular {
        if let Some(tb) = p.water.tabs.as_ref().and_then(|t| t.get(mi)) {
            let c = Curve {
                h: tb.h.iter().map(|h| -h.min(-1e-12)).collect(),
                th: tb.the.clone(),
                k: tb.con.clone(),
                c: tb.cap.clone(),
            };
            return (c, None);
        }
    }
    let m = &p.water.materials[mi];
    let mut par = par_of(m, model, xc);
    let hyst = p.water.hysteresis != Hysteresis::None;
    if hyst {
        par[6] = m.qm.max(m.qs);
    }
    let eval = |a_h: f64, a_th: f64, a_k: f64| -> Curve {
        let mut c = Curve { h: vec![], th: vec![], k: vec![], c: vec![] };
        for &h in &hs {
            let hh = -h / a_h;
            c.h.push(h);
            c.th.push(par[0] + a_th * (fq(model, hh, &par) - par[0]));
            c.k.push(a_k * fk(model, hh, &par));
            c.c.push(a_th / a_h * fc(model, hh, &par));
        }
        c
    };
    let drying = eval(1.0, 1.0, 1.0);
    let wetting = if hyst && m.alpha_w > 0.0 && par[1] > par[0] {
        let a_h = par[2] / m.alpha_w;
        let a_th = (m.qs_w - par[0]) / (par[1] - par[0]);
        let a_k = if p.water.hysteresis == Hysteresis::Retention { 1.0 } else { m.ks_w / par[4].max(1e-300) };
        Some(eval(a_h, a_th, a_k))
    } else {
        None
    };
    (drying, wetting)
}

fn tr(v: f64, log: bool) -> Option<f64> {
    if log {
        (v > 0.0 && v.is_finite()).then(|| v.log10())
    } else {
        v.is_finite().then_some(v)
    }
}

/// Plots the selected curve of the materials `mats` (0-based) with the measured θ(h) / K(h) data.
pub fn hyd_plot(ui: &mut Ui, p: &Project, mats: &[usize], v: &HydView, id: &str, height: f32, measured: &[Measured]) {
    let lu = p.units.length_str();
    let tu = p.units.time_str();
    let log_x = v.log_x && v.kind.x_is_h();
    let log_y = v.log_y && v.kind.log_y_possible();
    let x_label = if v.kind.x_is_h() { format!("|h| ({lu})") } else { "θ (–)".to_string() };
    let y_label = match v.kind {
        Kind::ThetaH => "θ (–)".to_string(),
        Kind::KH | Kind::KTheta => format!("K ({lu}/{tu})"),
        Kind::CH => format!("C (1/{lu})"),
    };
    let mut plot = Plot::new(id).height(height).legend(Legend::default().follow_insertion_order(true)).x_axis_label(x_label).y_axis_label(y_label);
    if log_x {
        plot = plot.x_grid_spacer(decade_marks).x_axis_formatter(decade_label);
    }
    if log_y {
        plot = plot.y_grid_spacer(decade_marks).y_axis_formatter(decade_label);
    }
    // the hover label shows the values, not their logarithms
    let (xn, yn) = (if v.kind.x_is_h() { "|h|" } else { "θ" }, if v.kind == Kind::ThetaH { "θ" } else if v.kind == Kind::CH { "C" } else { "K" });
    plot = plot.label_formatter(move |name, pt| {
        let x = if log_x { 10f64.powf(pt.x) } else { pt.x };
        let y = if log_y { 10f64.powf(pt.y) } else { pt.y };
        let head = if name.is_empty() { String::new() } else { format!("{name}\n") };
        format!("{head}{xn} = {}\n{yn} = {}", sig(x), sig(y))
    });
    let points = |c: &Curve| -> Vec<[f64; 2]> {
        (0..c.h.len())
            .filter_map(|i| {
                let (x, y) = match v.kind {
                    Kind::ThetaH => (c.h[i], c.th[i]),
                    Kind::KH => (c.h[i], c.k[i]),
                    Kind::KTheta => (c.th[i], c.k[i]),
                    // C = 0 at saturation is not drawn on a log axis
                    Kind::CH => (c.h[i], c.c[i]),
                };
                let y = if log_y && y < 1e-30 { return None } else { y };
                Some([tr(x, log_x)?, tr(y, log_y)?])
            })
            .collect()
    };
    let mcol = ui.visuals().strong_text_color();
    plot.show(ui, |pu| {
        for &mi in mats {
            let col = soil_mat_color(p, mi + 1);
            let name = format!("{} {}", mi + 1, p.water.materials.get(mi).map(|m| m.name.as_str()).unwrap_or(""));
            let (d, w) = curves(p, mi);
            let suffix = if w.is_some() && v.wetting { " (drying)" } else { "" };
            pu.line(Line::new(PlotPoints::from(points(&d))).name(format!("{name}{suffix}")).color(col).width(2.0_f32));
            if let (Some(w), true) = (w, v.wetting) {
                pu.line(Line::new(PlotPoints::from(points(&w))).name(format!("{name} (wetting)")).color(col).style(LineStyle::dashed_dense()));
            }
        }
        for m in measured.iter().filter(|m| m.show) {
            let ok = matches!((m.target, v.kind), (Target::Retention, Kind::ThetaH) | (Target::Conductivity, Kind::KH));
            if !ok {
                continue;
            }
            let pts: Vec<[f64; 2]> = m.pts.iter().filter_map(|q| Some([tr(q[0].abs(), log_x)?, tr(q[1], log_y)?])).collect();
            pu.points(Points::new(pts).name(&m.name).shape(MarkerShape::Circle).radius(3.5_f32).filled(true).color(mcol));
        }
    });
}
