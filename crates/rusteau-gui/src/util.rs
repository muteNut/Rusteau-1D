use crate::tips::TipExt;
use egui::{Color32, DragValue, Ui};
use rusteau_core::{LengthUnit, Project, SoilMaterial, TimeUnit};

/// Canonical 10-color categorical palette (sRGB). Single source of truth in GUI.
pub const PALETTE: [Color32; 10] = [
    Color32::from_rgb(31, 119, 180),  // 0: Blue (Loam)
    Color32::from_rgb(255, 127, 14),  // 1: Orange (Sand)
    Color32::from_rgb(44, 160, 44),   // 2: Green
    Color32::from_rgb(214, 39, 40),   // 3: Red
    Color32::from_rgb(148, 103, 189), // 4: Purple
    Color32::from_rgb(140, 86, 75),   // 5: Brown
    Color32::from_rgb(227, 119, 194), // 6: Pink
    Color32::from_rgb(127, 127, 127), // 7: Gray
    Color32::from_rgb(188, 189, 34),  // 8: Olive
    Color32::from_rgb(23, 190, 207),  // 9: Cyan
];

/// Returns the color for a material index (1-based).
pub fn mat_color(m: usize) -> Color32 {
    PALETTE[(m.max(1) - 1) % PALETTE.len()]
}

/// Returns color based on class name, falling back to 1-based index cycling.
pub fn soil_color_for_name(name: &str, m_1based: usize) -> Color32 {
    let name_lower = name.trim().to_lowercase();
    if name_lower == "sand" {
        PALETTE[1] // Orange
    } else if name_lower == "loam" {
        PALETTE[0] // Blue
    } else if name_lower.contains("clay") {
        PALETTE[3] // Red
    } else if name_lower.contains("silt") {
        PALETTE[5] // Brown
    } else {
        mat_color(m_1based)
    }
}

/// Dynamic material color resolver from project and 1-based material index.
pub fn soil_mat_color(prj: &Project, m_1based: usize) -> Color32 {
    let idx = m_1based.saturating_sub(1);
    if let Some(mat) = prj.water.materials.get(idx) {
        soil_color_for_name(&mat.name, m_1based)
    } else {
        mat_color(m_1based)
    }
}

/// USDA soil textural classes, Carsel & Parrish (1988) as in the HYDRUS soil catalog (cm, day).
pub const SOIL_CATALOG: [(&str, f64, f64, f64, f64, f64); 12] = [
    ("Sand", 0.045, 0.43, 0.145, 2.68, 712.8),
    ("Loamy Sand", 0.057, 0.41, 0.124, 2.28, 350.2),
    ("Sandy Loam", 0.065, 0.41, 0.075, 1.89, 106.1),
    ("Loam", 0.078, 0.43, 0.036, 1.56, 24.96),
    ("Silt", 0.034, 0.46, 0.016, 1.37, 6.0),
    ("Silt Loam", 0.067, 0.45, 0.020, 1.41, 10.8),
    ("Sandy Clay Loam", 0.100, 0.39, 0.059, 1.48, 31.44),
    ("Clay Loam", 0.095, 0.41, 0.019, 1.31, 6.24),
    ("Silty Clay Loam", 0.089, 0.43, 0.010, 1.23, 1.68),
    ("Sandy Clay", 0.100, 0.38, 0.027, 1.23, 2.88),
    ("Silty Clay", 0.070, 0.36, 0.005, 1.09, 0.48),
    ("Clay", 0.068, 0.38, 0.008, 1.09, 4.8),
];

/// Factor converting a length given in cm to the project's length unit.
pub fn cm_to_unit(p: &Project) -> f64 {
    match p.units.length {
        LengthUnit::Mm => 10.0,
        LengthUnit::Cm => 1.0,
        LengthUnit::M => 0.01,
    }
}

/// Factor converting a time given in days to the project's time unit.
pub fn day_to_unit(p: &Project) -> f64 {
    match p.units.time {
        TimeUnit::Sec => 86400.0,
        TimeUnit::Min => 1440.0,
        TimeUnit::Hours => 24.0,
        TimeUnit::Days => 1.0,
        TimeUnit::Years => 1.0 / 365.0,
    }
}

pub fn apply_catalog(m: &mut SoilMaterial, idx: usize, lf: f64, tf: f64) {
    if let Some(c) = SOIL_CATALOG.get(idx) {
        m.name = c.0.to_string();
        m.qr = c.1;
        m.qs = c.2;
        m.alpha = c.3 / lf; // 1/cm -> 1/L
        m.n = c.4;
        m.ks = c.5 * lf / tf;
        m.l = 0.5;
        m.qm = c.2;
        m.qs_w = c.2;
        m.alpha_w = m.alpha * 2.0;
        m.ks_w = m.ks;

        // Reset extra parameters cleanly and configure defaults if ModifiedVG is active
        m.extra = [0.0; 5];
        m.extra[0] = m.qs;
        m.extra[1] = m.qr;
        m.extra[2] = m.qs;
        m.extra[3] = m.ks;
    }
}

pub fn log_space(a: f64, b: f64, n: usize) -> Vec<f64> {
    let (la, lb) = (a.log10(), b.log10());
    (0..n).map(|i| 10f64.powf(la + (lb - la) * i as f64 / (n as f64 - 1.0))).collect()
}

fn fmt_num(v: f64, _r: std::ops::RangeInclusive<usize>) -> String {
    let a = v.abs();
    if a != 0.0 && (a >= 1e6 || a < 1e-4) {
        format!("{:e}", v)
    } else {
        let s = format!("{:.6}", v);
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if s.is_empty() || s == "-" { "0".into() } else { s }
    }
}

fn parse_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").replace(['d', 'D'], "e").parse::<f64>().ok()
}

pub fn dv(ui: &mut Ui, ch: &mut bool, v: &mut f64) {
    let speed = (v.abs() * 0.01).max(1e-6);
    if ui.add(DragValue::new(v).speed(speed).custom_formatter(fmt_num).custom_parser(parse_num)).changed() {
        *ch = true;
    }
}

/// Number field that returns its response (for tooltips).
pub fn dv_r(ui: &mut Ui, ch: &mut bool, v: &mut f64) -> egui::Response {
    let speed = (v.abs() * 0.01).max(1e-6);
    let r = ui.add(DragValue::new(v).speed(speed).custom_formatter(fmt_num).custom_parser(parse_num));
    if r.changed() {
        *ch = true;
    }
    r
}

/// Number field with a unit that returns its response (for tooltips).
pub fn dv_unit_r(ui: &mut Ui, ch: &mut bool, v: &mut f64, unit: &str) -> egui::Response {
    let speed = (v.abs() * 0.01).max(1e-6);
    let r = ui.add(DragValue::new(v).speed(speed).custom_formatter(fmt_num).custom_parser(parse_num).suffix(format!(" {}", unit)));
    if r.changed() {
        *ch = true;
    }
    r
}

/// Integer field that returns its response (for tooltips).
pub fn di_r(ui: &mut Ui, ch: &mut bool, v: &mut usize) -> egui::Response {
    let r = ui.add(DragValue::new(v).speed(0.2));
    if r.changed() {
        *ch = true;
    }
    r
}

pub fn dv_unit(ui: &mut Ui, ch: &mut bool, v: &mut f64, unit: &str) {
    let speed = (v.abs() * 0.01).max(1e-6);
    if ui.add(DragValue::new(v).speed(speed).custom_formatter(fmt_num).custom_parser(parse_num).suffix(format!(" {}", unit))).changed() {
        *ch = true;
    }
}

/// Grid row: label and number field, both with the description of the label as tooltip.
pub fn field(ui: &mut Ui, ch: &mut bool, label: &str, v: &mut f64) {
    let r = ui.label(label).tip(label);
    crate::shots::note(ui.ctx(), label, r.rect);
    dv_r(ui, ch, v).tip(label);
    ui.end_row();
}

/// Grid row: label and number field with a unit, both with the description of the label as tooltip.
pub fn field_u(ui: &mut Ui, ch: &mut bool, label: &str, v: &mut f64, unit: &str) {
    let r = ui.label(label).tip(label);
    crate::shots::note(ui.ctx(), label, r.rect);
    dv_unit_r(ui, ch, v, unit).tip(label);
    ui.end_row();
}

/// Grid row: label and integer field with the description of the label as tooltip.
pub fn field_i(ui: &mut Ui, ch: &mut bool, label: &str, v: &mut usize) {
    ui.label(label).tip(label);
    di_r(ui, ch, v).tip(label);
    ui.end_row();
}

/// Check box with the description of its label as tooltip.
pub fn check(ui: &mut Ui, ch: &mut bool, v: &mut bool, label: &str) {
    if ui.checkbox(v, label).tip(label).changed() {
        *ch = true;
    }
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(title).strong().size(15.0));
    ui.separator();
}
/// Time for display: 8 significant digits, which removes the noise of the single-precision time steps
/// (119.99999999441206 -> 120).
pub fn fmt_t(t: f64) -> String {
    if t == 0.0 || !t.is_finite() {
        return format!("{t}");
    }
    let v: f64 = format!("{:.7e}", t).parse().unwrap_or(t);
    format!("{v}")
}

/// Value with 4 significant digits for hover labels (plain notation for moderate magnitudes).
pub fn sig(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let a = v.abs();
    if (1e-3..1e5).contains(&a) {
        let d = (3 - a.log10().floor() as i32).max(0) as usize;
        let s = format!("{v:.d$}");
        if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
    } else {
        format!("{v:.3e}")
    }
}

/// Page title (heading in the strong text colour) with a "?" button that opens the help of the page (as F1).
pub fn page_title(ui: &mut Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).heading().strong());
        help_button(ui);
    });
}

/// Small "?" button: help on the current page.
pub fn help_button(ui: &mut Ui) {
    if ui.small_button("?").on_hover_text("Help on this page (F1)").clicked() {
        crate::help::request_page_help(ui.ctx());
    }
}

/// Grid marks at whole decades for an axis that shows log10 values: thin lines at every decade, labelled lines at
/// 1, 2, 5, 10, ... decades, whichever keeps the labels apart.
pub fn decade_marks(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let (lo, hi) = input.bounds;
    if !(lo.is_finite() && hi.is_finite()) || hi - lo > 1000.0 || hi <= lo {
        return vec![];
    }
    let major = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0].into_iter().find(|s| *s >= 8.0 * input.base_step_size).unwrap_or(100.0);
    let mut marks: Vec<egui_plot::GridMark> = vec![];
    for step in [1.0, major] {
        if step == 1.0 && hi - lo > 200.0 {
            continue;
        }
        let mut k = (lo / step).ceil() * step;
        while k <= hi {
            match marks.iter_mut().find(|m| (m.value - k).abs() < 1e-9) {
                Some(m) => m.step_size = m.step_size.max(step),
                None => marks.push(egui_plot::GridMark { value: k, step_size: step }),
            }
            k += step;
        }
    }
    marks
}

/// Label of a decade mark of a log10 axis: 0.01, 0.1, 1, 10, … 10000, then 1e5.
pub fn decade_label(m: egui_plot::GridMark, _range: &std::ops::RangeInclusive<f64>) -> String {
    let k = m.value.round();
    if (m.value - k).abs() > 1e-9 {
        return String::new();
    }
    let k = k as i32;
    if (-3..=4).contains(&k) {
        let v = 10f64.powi(k);
        let s = format!("{:.3}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        format!("1e{k}")
    }
}

/// Unit text of a concentration: mass unit per length unit cubed.
pub fn conc_unit(p: &Project) -> String {
    let m = p.units.mass.trim();
    let m = if m.is_empty() || m == "-" { "M" } else { m };
    format!("{}/{}³", m, p.units.length_str())
}
