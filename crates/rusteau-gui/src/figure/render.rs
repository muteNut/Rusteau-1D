//! Layout of a figure: title, subtitle and caption, the panels in a grid with their axes, ticks, data layers, legends
//! and annotations. The result is a [`Drawing`] in points and the geometry of the panels (for the coordinates under
//! the mouse in the preview).

use super::canvas::{Cap, Drawing, Path, Prim, Rgba, Stroke};
use super::data::Series;
use super::text::{self, TextLayout, ASCENT, DESCENT};
use rusteau_core::figure::{Anchor, AnnotationKind, Axis, Dash, Figure, Geom, Layer, LegendPos, Marker, Palette, Panel, Theme};
use std::sync::Arc;

/// Points per millimetre.
pub const PT_PER_MM: f64 = 72.0 / 25.4;

/// Mapping of data values to the page along one axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub lo: f64,
    pub hi: f64,
    pub log: bool,
    pub reverse: bool,
    /// page positions of `lo` and `hi` (before reversing) [pt]
    pub p0: f32,
    pub p1: f32,
}

impl Scale {
    fn frac(&self, v: f64) -> Option<f64> {
        if !v.is_finite() {
            return None;
        }
        let t = if self.log {
            if v <= 0.0 {
                return None;
            }
            (v.log10() - self.lo.log10()) / (self.hi.log10() - self.lo.log10())
        } else {
            (v - self.lo) / (self.hi - self.lo)
        };
        Some(if self.reverse { 1.0 - t } else { t })
    }

    /// Page position of value `v` (`None` for missing values and values ≤ 0 on a log axis).
    pub fn pos(&self, v: f64) -> Option<f32> {
        self.frac(v).map(|t| (self.p0 as f64 + t.clamp(-1e3, 1e3) * (self.p1 - self.p0) as f64) as f32)
    }

    /// Value at page position `p`.
    pub fn value(&self, p: f32) -> f64 {
        let mut t = ((p - self.p0) / (self.p1 - self.p0)) as f64;
        if self.reverse {
            t = 1.0 - t;
        }
        if self.log {
            10f64.powf(self.lo.log10() + t * (self.hi.log10() - self.lo.log10()))
        } else {
            self.lo + t * (self.hi - self.lo)
        }
    }
}

/// Plot area and scales of a panel.
#[derive(Clone, Debug)]
pub struct PanelGeom {
    /// plot area: left, top, right, bottom [pt]
    pub rect: [f32; 4],
    pub x: Scale,
    pub y: Scale,
    pub y2: Option<Scale>,
}

/// A rendered figure.
#[derive(Clone, Debug, Default)]
pub struct Rendered {
    pub drawing: Drawing,
    pub panels: Vec<PanelGeom>,
    /// notes about the data (e.g. values left out on log axes)
    pub notes: Vec<String>,
}

/// Colours and lines of a theme.
struct Look {
    bg: Rgba,
    fg: Rgba,
    muted: Rgba,
    axis: Rgba,
    grid: Rgba,
    minor: Rgba,
    frame: bool,
    axis_lines: bool,
    ticks: bool,
    legend_bg: Rgba,
    legend_border: Option<Rgba>,
}

fn look(t: Theme) -> Look {
    let white = Rgba(255, 255, 255, 255);
    match t {
        Theme::Grid => Look {
            bg: white,
            fg: Rgba(26, 26, 26, 255),
            muted: Rgba(85, 85, 85, 255),
            axis: Rgba(77, 77, 77, 255),
            grid: Rgba(222, 222, 222, 255),
            minor: Rgba(240, 240, 240, 255),
            frame: true,
            axis_lines: false,
            ticks: true,
            legend_bg: Rgba(255, 255, 255, 220),
            legend_border: Some(Rgba(190, 190, 190, 255)),
        },
        Theme::Classic => Look {
            bg: white,
            fg: Rgba(0, 0, 0, 255),
            muted: Rgba(80, 80, 80, 255),
            axis: Rgba(0, 0, 0, 255),
            grid: Rgba(225, 225, 225, 255),
            minor: Rgba(242, 242, 242, 255),
            frame: false,
            axis_lines: true,
            ticks: true,
            legend_bg: Rgba(255, 255, 255, 220),
            legend_border: None,
        },
        Theme::Minimal => Look {
            bg: white,
            fg: Rgba(40, 40, 40, 255),
            muted: Rgba(100, 100, 100, 255),
            axis: Rgba(120, 120, 120, 255),
            grid: Rgba(224, 224, 224, 255),
            minor: Rgba(240, 240, 240, 255),
            frame: false,
            axis_lines: false,
            ticks: false,
            legend_bg: Rgba(255, 255, 255, 200),
            legend_border: None,
        },
        Theme::Dark => Look {
            bg: Rgba(32, 33, 36, 255),
            fg: Rgba(232, 232, 232, 255),
            muted: Rgba(180, 180, 180, 255),
            axis: Rgba(150, 150, 150, 255),
            grid: Rgba(62, 63, 66, 255),
            minor: Rgba(45, 46, 49, 255),
            frame: true,
            axis_lines: false,
            ticks: true,
            legend_bg: Rgba(32, 33, 36, 220),
            legend_border: Some(Rgba(90, 90, 90, 255)),
        },
    }
}

const OKABE_ITO: [[u8; 3]; 8] = [[0, 114, 178], [213, 94, 0], [0, 158, 115], [230, 159, 0], [86, 180, 233], [204, 121, 167], [240, 228, 66], [0, 0, 0]];
const VIRIDIS: [[u8; 3]; 9] = [[68, 1, 84], [71, 45, 123], [59, 82, 139], [44, 114, 142], [33, 145, 140], [40, 174, 128], [94, 201, 98], [173, 220, 48], [253, 231, 37]];

/// Colour `k` of `n` series of a palette.
pub fn palette_color(p: Palette, k: usize, n: usize, dark: bool) -> [u8; 3] {
    let along = |a: f64| if n <= 1 { 0.0 } else { a * k as f64 / (n - 1) as f64 };
    match p {
        Palette::OkabeIto => {
            let c = OKABE_ITO[k % OKABE_ITO.len()];
            if dark && c == [0, 0, 0] {
                [200, 200, 200]
            } else {
                c
            }
        }
        Palette::Tableau => {
            let c = crate::util::PALETTE[k % crate::util::PALETTE.len()];
            [c.r(), c.g(), c.b()]
        }
        Palette::Viridis => {
            // up to 85 % of the map: the last yellow is hard to see on white
            let t = along(0.85) * (VIRIDIS.len() - 1) as f64;
            let i = (t.floor() as usize).min(VIRIDIS.len() - 2);
            let f = t - i as f64;
            let (a, b) = (VIRIDIS[i], VIRIDIS[i + 1]);
            [0, 1, 2].map(|j| (a[j] as f64 + f * (b[j] as f64 - a[j] as f64)).round() as u8)
        }
        Palette::Grey => {
            let v = if dark { 240.0 - along(150.0) } else { along(160.0) };
            let v = v.round() as u8;
            [v, v, v]
        }
    }
}

fn dash_pattern(d: Dash, width: f32) -> (Vec<f32>, Cap) {
    let f = (width / 1.2).max(0.6);
    match d {
        Dash::Solid => (vec![], Cap::Butt),
        Dash::Dashed => (vec![5.0 * f, 3.0 * f], Cap::Butt),
        Dash::Dotted => (vec![0.01, 2.4 * f], Cap::Round),
        Dash::DashDot => (vec![5.0 * f, 2.2 * f, 0.01, 2.2 * f], Cap::Round),
    }
}

const DASHES: [Dash; 4] = [Dash::Solid, Dash::Dashed, Dash::Dotted, Dash::DashDot];
const MARKERS: [Marker; 9] =
    [Marker::Circle, Marker::Square, Marker::Triangle, Marker::Diamond, Marker::Cross, Marker::Plus, Marker::OpenCircle, Marker::OpenSquare, Marker::OpenTriangle];

fn cycle<T: Copy + PartialEq>(all: &[T], base: T, k: usize) -> T {
    let b = all.iter().position(|x| *x == base).unwrap_or(0);
    all[(b + k) % all.len()]
}

/// How one series is drawn.
#[derive(Clone, Debug)]
struct Style {
    color: Rgba,
    geom: Geom,
    width: f32,
    dash: Dash,
    marker: Marker,
    marker_size: f32,
    alpha: f64,
}

/// Styles of the series of the layers of a panel: palette colours in order; a layer with several items and several
/// runs (comparison) keeps the colour of an item and changes the line style and marker with the run.
fn styles(fig: &Figure, panel: &Panel, series: &[Vec<Series>], dark: bool) -> Vec<Vec<Style>> {
    let empty = vec![];
    let of = |k: usize| series.get(k).unwrap_or(&empty);
    let plan: Vec<(bool, usize)> = panel
        .layers
        .iter()
        .enumerate()
        .map(|(k, l)| {
            let ss = of(k);
            let n_items = ss.iter().map(|s| s.item).max().map(|m| m + 1).unwrap_or(0);
            let n_runs = ss.iter().map(|s| s.run).max().map(|m| m + 1).unwrap_or(0);
            let by_item = n_items > 1 && n_runs > 1;
            let slots = if !l.visible || l.color.is_some() {
                0
            } else if by_item {
                n_items
            } else {
                ss.len()
            };
            (by_item, slots)
        })
        .collect();
    let total: usize = plan.iter().map(|p| p.1).sum();
    let mut next = 0;
    let mut out = vec![];
    for ((k, l), (by_item, slots)) in panel.layers.iter().enumerate().zip(plan) {
        let v = of(k)
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let slot = next + if by_item { s.item } else { k };
                let c = l.color.unwrap_or_else(|| palette_color(fig.palette, slot, total, dark));
                let (dash, marker) = if by_item { (cycle(&DASHES, l.dash, s.run), cycle(&MARKERS, l.marker, s.run)) } else { (l.dash, l.marker) };
                Style { color: Rgba::rgb(c), geom: l.geom, width: l.width as f32, dash, marker, marker_size: l.marker_size as f32, alpha: l.alpha }
            })
            .collect();
        next += slots;
        out.push(v);
    }
    out
}

/// Text in the font of the figure.
struct Typo<'a> {
    family: &'a str,
}

impl Typo<'_> {
    fn t(&self, s: &str, size: f64, bold: bool, wrap: Option<f32>) -> Arc<TextLayout> {
        Arc::new(text::layout(s, size as f32, self.family, bold, wrap))
    }
}

fn nice_step(range: f64, target: f64) -> f64 {
    let raw = range / target.max(1.0);
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    let f = raw / mag;
    let nf = if f < 1.5 {
        1.0
    } else if f < 2.25 {
        2.0
    } else if f < 3.5 {
        2.5
    } else if f < 7.5 {
        5.0
    } else {
        10.0
    };
    nf * mag
}

/// Number with a minus sign (U+2212).
fn minus(s: String) -> String {
    s.replace('-', "−")
}

/// Labels of linear ticks: decimals from the step; large and small numbers as a × 10^b.
fn tick_labels(vals: &[f64], step: f64) -> Vec<String> {
    let amax = vals.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let e = if amax > 0.0 { amax.log10().floor() as i32 } else { 0 };
    if e >= 5 || e <= -4 {
        let p = 10f64.powi(e);
        let d = (-(step / p).log10().floor() as i32).clamp(0, 6) as usize;
        vals.iter()
            .map(|v| {
                if v.abs() < step * 1e-6 {
                    "0".to_string()
                } else {
                    minus(format!("{:.d$}×10^{{{}}}", v / p, e))
                }
            })
            .collect()
    } else {
        let mut d = (-(step.log10() + 1e-9).floor() as i32).max(0) as usize;
        let mant = step / 10f64.powf(step.log10().floor());
        if (mant - 2.5).abs() < 1e-6 {
            d += 1;
        }
        vals.iter().map(|v| if v.abs() < step * 1e-6 { "0".to_string() } else { minus(format!("{v:.d$}")) }).collect()
    }
}

#[derive(Clone, Debug, Default)]
struct Ticks {
    major: Vec<(f64, String)>,
    minor: Vec<f64>,
}

/// Range and ticks of an axis of `len` pt with at least `spacing` pt between the major ticks.
fn axis_ticks(axis: &Axis, data: Option<(f64, f64)>, len: f32, spacing: f32) -> ((f64, f64), Ticks) {
    let target = ((len / spacing.max(1.0)).floor() as f64).clamp(2.0, 12.0);
    if axis.log {
        let (mut lo, mut hi) = if axis.auto || axis.min <= 0.0 || axis.max <= 0.0 { data.unwrap_or((1.0, 10.0)) } else { (axis.min, axis.max) };
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        if lo <= 0.0 {
            lo = hi / 1e3;
        }
        if hi <= 0.0 {
            (lo, hi) = (1.0, 10.0);
        }
        if axis.auto {
            lo = 10f64.powf((lo.log10() + 1e-9).floor());
            hi = 10f64.powf((hi.log10() - 1e-9).ceil());
        }
        if hi / lo < 1.0 + 1e-9 {
            hi = lo * 10.0;
        }
        let (k0, k1) = ((lo.log10() - 1e-9).ceil() as i32, (hi.log10() + 1e-9).floor() as i32);
        let decades = (hi / lo).log10();
        let m = if axis.step >= 1.0 { axis.step.round() as i32 } else { (decades / target).ceil().max(1.0) as i32 };
        let plain = lo >= 0.01 * 0.999 && hi <= 1000.0 * 1.001;
        let lab = |k: i32| if plain { crate::util::sig(10f64.powi(k)) } else { minus(format!("10^{{{k}}}")) };
        let mut t = Ticks::default();
        for k in k0..=k1 {
            if k.rem_euclid(m) == 0 {
                t.major.push((10f64.powi(k), lab(k)));
            } else {
                t.minor.push(10f64.powi(k));
            }
        }
        if m == 1 && decades <= 12.0 {
            for k in (k0 - 1)..=k1 {
                for j in 2..=9 {
                    let v = j as f64 * 10f64.powi(k);
                    if v >= lo * (1.0 - 1e-9) && v <= hi * (1.0 + 1e-9) {
                        t.minor.push(v);
                    }
                }
            }
        }
        if t.major.len() < 2 {
            // less than two decades in the range: 1, 2 and 5 times the powers of ten
            t.major.clear();
            for k in (k0 - 1)..=k1 {
                for j in [1.0, 2.0, 5.0] {
                    let v = j * 10f64.powi(k);
                    if v >= lo * (1.0 - 1e-9) && v <= hi * (1.0 + 1e-9) {
                        t.major.push((v, minus(crate::util::sig(v))));
                    }
                }
            }
        }
        return ((lo, hi), t);
    }
    let (mut lo, mut hi) = if axis.auto { data.unwrap_or((0.0, 1.0)) } else { (axis.min, axis.max) };
    if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }
    if hi - lo <= 1e-12 * hi.abs().max(lo.abs()).max(1e-300) {
        let d = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
        lo -= d;
        hi += d;
    }
    let mut step = if axis.step > 0.0 { axis.step } else { nice_step(hi - lo, target) };
    if (hi - lo) / step > 500.0 {
        step = nice_step(hi - lo, target);
    }
    if axis.auto {
        // data a millionth of a step beyond a tick (noise of single-precision times) do not add a step
        lo = (lo / step + 1e-6).floor() * step;
        hi = (hi / step - 1e-6).ceil() * step;
    }
    let (k0, k1) = ((lo / step - 1e-9).ceil() as i64, (hi / step + 1e-9).floor() as i64);
    let vals: Vec<f64> = (k0..=k1).map(|k| k as f64 * step).collect();
    let labels = tick_labels(&vals, step);
    let mant = step / 10f64.powf(step.log10().floor());
    let sub = if (mant - 2.0).abs() < 1e-6 { 4.0 } else { 5.0 };
    let ms = step / sub;
    let (m0, m1) = ((lo / ms - 1e-9).ceil() as i64, (hi / ms + 1e-9).floor() as i64);
    let minor = if m1 - m0 < 5000 { (m0..=m1).map(|k| k as f64 * ms).filter(|v| ((v / step).round() * step - v).abs() > ms * 1e-3).collect() } else { vec![] };
    ((lo, hi), Ticks { major: vals.into_iter().zip(labels).collect(), minor })
}

/// Range of the finite values (only positive values on a log axis).
fn range_of<'a>(vals: impl Iterator<Item = &'a f64>, log: bool) -> Option<(f64, f64)> {
    let mut r: Option<(f64, f64)> = None;
    for &v in vals {
        if !v.is_finite() || (log && v <= 0.0) {
            continue;
        }
        r = Some(match r {
            None => (v, v),
            Some((a, b)) => (a.min(v), b.max(v)),
        });
    }
    r
}

fn union(a: Option<(f64, f64)>, b: Option<(f64, f64)>) -> Option<(f64, f64)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.max(b.1))),
        (a, None) => a,
        (None, b) => b,
    }
}

/// Automatic axis title from the variables of the series: "name (unit)"; several names with the same unit share it.
pub fn auto_title(infos: &[&super::data::ColInfo]) -> String {
    let mut names: Vec<&str> = vec![];
    let mut units: Vec<&str> = vec![];
    for i in infos {
        if !names.contains(&i.name.as_str()) {
            names.push(&i.name);
        }
        if !units.contains(&i.unit.as_str()) {
            units.push(&i.unit);
        }
    }
    if names.is_empty() {
        return String::new();
    }
    let shown: Vec<&str> = names.iter().take(3).copied().collect();
    let more = if names.len() > 3 { ", …" } else { "" };
    if units.len() == 1 {
        let n = format!("{}{more}", shown.join(", "));
        if units[0].is_empty() {
            n
        } else {
            format!("{n} ({})", units[0])
        }
    } else {
        infos.iter().take(3).map(|i| i.title()).collect::<Vec<_>>().join(", ") + more
    }
}

/// Markup characters in automatic titles are shown as they are.
fn escape_markup(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if matches!(c, '*' | '`' | '\\' | '[') {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

fn marker_path(m: Marker, x: f32, y: f32, d: f32) -> (Path, bool) {
    let r = d / 2.0;
    match m {
        Marker::Circle | Marker::OpenCircle => (Path::circle(x, y, r), m == Marker::Circle),
        Marker::Square | Marker::OpenSquare => {
            let h = r * 0.88;
            (Path::rect(x - h, y - h, x + h, y + h), m == Marker::Square)
        }
        Marker::Triangle | Marker::OpenTriangle => {
            let h = r * 1.12;
            (Path::polygon(&[(x, y - h), (x + h * 0.95, y + h * 0.6), (x - h * 0.95, y + h * 0.6)]), m == Marker::Triangle)
        }
        Marker::Diamond => (Path::polygon(&[(x, y - r * 1.15), (x + r * 0.85, y), (x, y + r * 1.15), (x - r * 0.85, y)]), true),
        Marker::Cross => {
            let h = r * 0.8;
            let mut p = Path::line(x - h, y - h, x + h, y + h);
            p.extend(Path::line(x - h, y + h, x + h, y - h));
            (p, false)
        }
        Marker::Plus => {
            let mut p = Path::line(x - r, y, x + r, y);
            p.extend(Path::line(x, y - r, x, y + r));
            (p, false)
        }
    }
}

fn draw_marker(d: &mut Drawing, m: Marker, x: f32, y: f32, size: f32, c: Rgba, bg: Rgba) {
    let (p, filled) = marker_path(m, x, y, size);
    let lw = (size / 6.5).max(0.6);
    if filled {
        d.fill(p, c);
    } else {
        if matches!(m, Marker::OpenCircle | Marker::OpenSquare | Marker::OpenTriangle) {
            d.fill(p.clone(), bg);
        }
        d.stroke(p, c, Stroke { width: lw, dash: vec![], cap: Cap::Round });
    }
}

/// Draws a series in the plot area.
fn draw_series(d: &mut Drawing, s: &Series, st: &Style, sx: &Scale, sy: &Scale, bg: Rgba) {
    let pts: Vec<Option<(f32, f32)>> = s.x.iter().zip(&s.y).map(|(&x, &y)| Some((sx.pos(x)?, sy.pos(y)?))).collect();
    // runs of consecutive valid points
    let mut runs: Vec<Vec<(f32, f32)>> = vec![];
    let mut cur = vec![];
    for p in &pts {
        match p {
            Some(q) => cur.push(*q),
            None => {
                if !cur.is_empty() {
                    runs.push(std::mem::take(&mut cur));
                }
            }
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    let (dash, cap) = dash_pattern(st.dash, st.width);
    let stroke = Stroke { width: st.width, dash, cap };
    // the base of areas and bars: y = 0, or the edge of the axis when 0 is not on it
    let base = sy.pos(0.0).filter(|_| !sy.log).unwrap_or(if sy.reverse { sy.p1 } else { sy.p0 });
    let base = base.clamp(sy.p0.min(sy.p1), sy.p0.max(sy.p1));
    match st.geom {
        Geom::Line | Geom::LinePoints => {
            for r in &runs {
                if r.len() > 1 {
                    d.stroke(Path::polyline(r), st.color, stroke.clone());
                }
            }
        }
        Geom::Steps => {
            for r in &runs {
                if r.len() > 1 {
                    // the value of a point holds from the previous x (the records of the boundary data)
                    let mut v = vec![r[0]];
                    for w in r.windows(2) {
                        v.push((w[0].0, w[1].1));
                        v.push(w[1]);
                    }
                    d.stroke(Path::polyline(&v), st.color, stroke.clone());
                }
            }
        }
        Geom::Area => {
            for r in &runs {
                if r.len() > 1 {
                    let mut poly = r.clone();
                    poly.push((r[r.len() - 1].0, base));
                    poly.push((r[0].0, base));
                    d.fill(Path::polygon(&poly), st.color.alpha(st.alpha));
                    d.stroke(Path::polyline(r), st.color, stroke.clone());
                }
            }
        }
        Geom::Bars => {
            let xs: Vec<f32> = runs.iter().flatten().map(|p| p.0).collect();
            for r in &runs {
                for p in r {
                    // 80 % of the distance to the nearest neighbour
                    let gap = xs.iter().filter(|x| (**x - p.0).abs() > 1e-3).map(|x| (x - p.0).abs()).fold(f32::INFINITY, f32::min);
                    let w = if gap.is_finite() { 0.8 * gap } else { 8.0 }.max(0.5);
                    let rect = Path::rect(p.0 - w / 2.0, p.1.min(base), p.0 + w / 2.0, p.1.max(base));
                    d.fill(rect.clone(), st.color.alpha(st.alpha));
                    d.stroke(rect, st.color, Stroke::solid((st.width * 0.5).max(0.4)));
                }
            }
        }
        Geom::Points => {}
    }
    if matches!(st.geom, Geom::Points | Geom::LinePoints) {
        for p in pts.iter().flatten() {
            draw_marker(d, st.marker, p.0, p.1, st.marker_size, st.color, bg);
        }
    }
}

/// An entry of a legend.
struct Entry {
    style: Style,
    label: Arc<TextLayout>,
}

const SAMPLE_W: f32 = 20.0;

fn entry_size(e: &Entry) -> (f32, f32) {
    let sw = if e.style.geom == Geom::Points { e.style.marker_size.max(8.0) } else { SAMPLE_W };
    (sw + 4.0 + e.label.width, e.label.height.max(e.style.marker_size).max(6.0))
}

fn draw_sample(d: &mut Drawing, st: &Style, x: f32, yc: f32, bg: Rgba) {
    let sw = if st.geom == Geom::Points { st.marker_size.max(8.0) } else { SAMPLE_W };
    match st.geom {
        Geom::Area | Geom::Bars => {
            let r = Path::rect(x + 1.0, yc - 4.0, x + sw - 1.0, yc + 4.0);
            d.fill(r.clone(), st.color.alpha(st.alpha));
            d.stroke(r, st.color, Stroke::solid(if st.geom == Geom::Area { st.width.min(2.0) } else { 0.6 }));
        }
        Geom::Points => {}
        _ => {
            let (dash, cap) = dash_pattern(st.dash, st.width);
            d.stroke(Path::line(x, yc, x + sw, yc), st.color, Stroke { width: st.width, dash, cap });
        }
    }
    if matches!(st.geom, Geom::Points | Geom::LinePoints) {
        draw_marker(d, st.marker, x + sw / 2.0, yc, st.marker_size, st.color, bg);
    }
}

/// Size of a legend: entries in a column, or in rows of at most `max_w` pt (legend below the plot).
fn legend_size(entries: &[Entry], rows_of: Option<f32>) -> (f32, f32) {
    const PAD: f32 = 4.0;
    match rows_of {
        None => {
            let w = entries.iter().map(|e| entry_size(e).0).fold(0.0, f32::max);
            let h: f32 = entries.iter().map(|e| entry_size(e).1 + 2.0).sum();
            (w + 2.0 * PAD, h - 2.0 + 2.0 * PAD)
        }
        Some(max_w) => {
            let rows = flow(entries, max_w - 2.0 * PAD);
            let w = rows.iter().map(|r| r.1).fold(0.0, f32::max);
            let h: f32 = rows.iter().map(|r| r.2 + 2.0).sum();
            (w + 2.0 * PAD, h - 2.0 + 2.0 * PAD)
        }
    }
}

/// Entries in rows: (indices, width, height).
fn flow(entries: &[Entry], max_w: f32) -> Vec<(Vec<usize>, f32, f32)> {
    let mut rows: Vec<(Vec<usize>, f32, f32)> = vec![];
    for (k, e) in entries.iter().enumerate() {
        let (w, h) = entry_size(e);
        match rows.last_mut() {
            Some(r) if r.1 + 12.0 + w <= max_w => {
                r.0.push(k);
                r.1 += 12.0 + w;
                r.2 = r.2.max(h);
            }
            _ => rows.push((vec![k], w, h)),
        }
    }
    rows
}

fn draw_legend(d: &mut Drawing, entries: &[Entry], x: f32, y: f32, rows_of: Option<f32>, lk: &Look, boxed: bool) {
    const PAD: f32 = 4.0;
    let (w, h) = legend_size(entries, rows_of);
    if boxed {
        let r = Path::rect(x, y, x + w, y + h);
        d.fill(r.clone(), lk.legend_bg);
        if let Some(b) = lk.legend_border {
            d.stroke(r, b, Stroke::solid(0.5));
        }
    }
    let item = |d: &mut Drawing, e: &Entry, ex: f32, ey: f32, eh: f32| {
        let sw = if e.style.geom == Geom::Points { e.style.marker_size.max(8.0) } else { SAMPLE_W };
        let yc = ey + eh / 2.0;
        draw_sample(d, &e.style, ex, yc, lk.bg);
        d.text(&e.label, ex + sw + 4.0, yc - e.label.height / 2.0, 0.0, lk.fg);
    };
    match rows_of {
        None => {
            let mut ey = y + PAD;
            for e in entries {
                let (_, eh) = entry_size(e);
                item(d, e, x + PAD, ey, eh);
                ey += eh + 2.0;
            }
        }
        Some(max_w) => {
            let mut ey = y + PAD;
            for (idx, rw, rh) in flow(entries, max_w - 2.0 * PAD) {
                let mut ex = x + PAD + (w - 2.0 * PAD - rw) / 2.0;
                for k in idx {
                    item(d, &entries[k], ex, ey, rh);
                    ex += entry_size(&entries[k]).0 + 12.0;
                }
                ey += rh + 2.0;
            }
        }
    }
}

/// Renders a figure: `series[panel][layer]` are the series of the layers of the panels.
pub fn render(fig: &Figure, series: &[Vec<Vec<Series>>]) -> Rendered {
    let w = (fig.width_mm.clamp(10.0, 2000.0) * PT_PER_MM) as f32;
    let h = (fig.height_mm.clamp(10.0, 2000.0) * PT_PER_MM) as f32;
    let lk = look(fig.theme);
    let dark = fig.theme == Theme::Dark;
    let ty = Typo { family: &fig.font };
    let sz = &fig.sizes;
    let mut d = Drawing { width: w, height: h, background: Some(lk.bg), prims: vec![] };
    let mut out_panels = vec![];
    let mut notes = vec![];
    let pad = 8.0f32;
    // title, subtitle
    let mut top = pad;
    let inner_w = w - 2.0 * pad;
    if !fig.title.trim().is_empty() {
        let t = ty.t(&fig.title, sz.title, fig.title_bold, Some(inner_w));
        d.text(&t, pad, top, 0.0, lk.fg);
        top += t.height + 3.0;
    }
    if !fig.subtitle.trim().is_empty() {
        let t = ty.t(&fig.subtitle, sz.subtitle, false, Some(inner_w));
        d.text(&t, pad, top, 0.0, lk.muted);
        top += t.height + 3.0;
    }
    if top > pad {
        top += 4.0;
    }
    // caption
    let mut bottom = h - pad;
    if !fig.caption.trim().is_empty() {
        let t = ty.t(&fig.caption, sz.caption, false, Some(inner_w));
        bottom -= t.height;
        d.text(&t, pad, bottom, 0.0, lk.muted);
        bottom -= 6.0;
    }
    let rows = fig.rows.max(1);
    let cols = fig.cols.max(1);
    let gap = 12.0f32;
    let cw = (inner_w - gap * (cols - 1) as f32) / cols as f32;
    let ch = (bottom - top - gap * (rows - 1) as f32) / rows as f32;
    let empty: Vec<Vec<Series>> = vec![];
    for (pi, panel) in fig.panels.iter().take(rows * cols).enumerate() {
        let (r, c) = (pi / cols, pi % cols);
        let cell = [pad + c as f32 * (cw + gap), top + r as f32 * (ch + gap), pad + c as f32 * (cw + gap) + cw, top + r as f32 * (ch + gap) + ch];
        let ps = series.get(pi).unwrap_or(&empty);
        let g = render_panel(&mut d, fig, panel, ps, cell, &lk, dark, &ty, &mut notes);
        out_panels.push(g);
    }
    notes.dedup();
    Rendered { drawing: d, panels: out_panels, notes }
}

#[allow(clippy::too_many_arguments)]
fn render_panel(d: &mut Drawing, fig: &Figure, panel: &Panel, series: &[Vec<Series>], cell: [f32; 4], lk: &Look, dark: bool, ty: &Typo, notes: &mut Vec<String>) -> PanelGeom {
    let sz = &fig.sizes;
    let [cx0, cy0, cx1, cy1] = cell;
    let mut top = cy0;
    if !panel.title.trim().is_empty() {
        let t = ty.t(&panel.title, sz.panel_title, false, Some(cx1 - cx0));
        d.text(&t, cx0, top, 0.0, lk.fg);
        top += t.height + 4.0;
    }
    let empty = vec![];
    let layer_series = |k: usize| series.get(k).unwrap_or(&empty);
    let visible: Vec<(usize, &Layer)> = panel.layers.iter().enumerate().filter(|(_, l)| l.visible).collect();
    let styles = styles(fig, panel, series, dark);
    let has_y2 = visible.iter().any(|(k, l)| l.y2 && !layer_series(*k).is_empty());
    // data ranges
    let (mut rx, mut ry, mut ry2) = (None, None, None);
    let mut any = false;
    for (k, l) in &visible {
        for s in layer_series(*k) {
            any |= !s.x.is_empty();
            rx = union(rx, range_of(s.x.iter(), panel.x.log));
            let ya = if l.y2 { &panel.y2 } else { &panel.y };
            let mut yr = range_of(s.y.iter(), ya.log);
            if matches!(l.geom, Geom::Area | Geom::Bars) && !ya.log {
                yr = union(yr, Some((0.0, 0.0)));
            }
            if l.y2 {
                ry2 = union(ry2, yr);
            } else {
                ry = union(ry, yr);
            }
            let bad = |v: &[f64]| v.iter().any(|x| x.is_finite() && *x <= 0.0);
            if (panel.x.log && bad(&s.x)) || (ya.log && bad(&s.y)) {
                notes.push("Values ≤ 0 are left out on logarithmic axes.".into());
            }
        }
    }
    // axis titles
    let infos = |x: bool, right: bool| {
        let mut v = vec![];
        for (k, l) in &visible {
            if !x && l.y2 != right {
                continue;
            }
            for s in layer_series(*k).iter().take(1) {
                v.push(if x { &s.x_info } else { &s.y_info });
            }
        }
        v
    };
    let title_of = |a: &Axis, x: bool, right: bool| if a.label_auto { escape_markup(&auto_title(&infos(x, right))) } else { a.label.clone() };
    let xt = ty.t(&title_of(&panel.x, true, false), sz.axis_title, false, None);
    let yt = ty.t(&title_of(&panel.y, false, false), sz.axis_title, false, None);
    let y2t = ty.t(&title_of(&panel.y2, false, true), sz.axis_title, false, None);
    // legend
    let mut entries = vec![];
    for (k, l) in &visible {
        if !l.in_legend {
            continue;
        }
        for (s, st) in layer_series(*k).iter().zip(&styles[*k]) {
            if entries.len() < 40 {
                entries.push(Entry { style: st.clone(), label: ty.t(&s.label, sz.legend, false, Some(200.0)) });
            }
        }
    }
    let legend = if entries.is_empty() { LegendPos::None } else { panel.legend };
    let tick_len = if lk.ticks { 3.0 } else { 0.0 };
    let tick_h = (ASCENT + DESCENT) * sz.ticks as f32;
    // margins depend on the tick labels, which depend on the length of the axes: two passes
    let mut rect = [cx0 + 40.0, top + 4.0, cx1 - 10.0, cy1 - 30.0];
    let mut x_spacing = 50.0f32;
    let mut res = None;
    for _ in 0..3 {
        let (xr, xticks) = axis_ticks(&panel.x, rx, rect[2] - rect[0], x_spacing);
        let (yr, yticks) = axis_ticks(&panel.y, ry, rect[3] - rect[1], 3.6 * sz.ticks as f32);
        let (y2r, y2ticks) = axis_ticks(&panel.y2, ry2, rect[3] - rect[1], 3.6 * sz.ticks as f32);
        let lab = |t: &Ticks| t.major.iter().map(|(_, s)| ty.t(s, sz.ticks, false, None)).collect::<Vec<_>>();
        let (xl, yl, y2l) = (lab(&xticks), lab(&yticks), if has_y2 { lab(&y2ticks) } else { vec![] });
        let yw = yl.iter().map(|t| t.width).fold(0.0, f32::max);
        let y2w = y2l.iter().map(|t| t.width).fold(0.0, f32::max);
        let mut left = cx0 + yw + tick_len + 3.0;
        if !yt.is_empty() {
            left += yt.height + 4.0;
        }
        let mut right = cx1 - 4.0;
        if has_y2 {
            right = cx1 - (y2w + tick_len + 3.0);
            if !y2t.is_empty() {
                right -= y2t.height + 4.0;
            }
        }
        let mut bot = cy1 - (tick_h + tick_len + 3.0);
        if !xt.is_empty() {
            bot -= xt.height + 4.0;
        }
        match legend {
            LegendPos::Right => {
                right -= legend_size(&entries, None).0 + 8.0;
            }
            LegendPos::Bottom => {
                bot -= legend_size(&entries, Some(cx1 - cx0)).1 + 6.0;
            }
            _ => {}
        }
        // room for the half of the top tick label above the plot area
        let new = [left, top + tick_h / 2.0, right.max(left + 10.0), bot.max(top + 10.0)];
        x_spacing = (xl.iter().map(|t| t.width).fold(0.0, f32::max) + 20.0).max(45.0);
        let done = (new[0] - rect[0]).abs() < 0.5 && (new[2] - rect[2]).abs() < 0.5 && (new[3] - rect[3]).abs() < 0.5;
        rect = new;
        res = Some((xr, xticks, xl, yr, yticks, yl, y2r, y2ticks, y2l));
        if done {
            break;
        }
    }
    let (xr, xticks, xl, yr, yticks, yl, y2r, y2ticks, y2l) = res.unwrap();
    // final ticks for the final lengths (labels measured above stay valid when the ticks do not change)
    let [x0, y0, x1, y1] = rect;
    let sx = Scale { lo: xr.0, hi: xr.1, log: panel.x.log, reverse: panel.x.reverse, p0: x0, p1: x1 };
    let sy = Scale { lo: yr.0, hi: yr.1, log: panel.y.log, reverse: panel.y.reverse, p0: y1, p1: y0 };
    let sy2 = Scale { lo: y2r.0, hi: y2r.1, log: panel.y2.log, reverse: panel.y2.reverse, p0: y1, p1: y0 };
    // grid
    let grid = |d: &mut Drawing, a: &Axis, t: &Ticks, s: &Scale, vertical: bool| {
        if !a.grid {
            return;
        }
        if a.minor {
            let mut p = Path::default();
            for v in &t.minor {
                if let Some(q) = s.pos(*v) {
                    p.extend(if vertical { Path::line(q, y0, q, y1) } else { Path::line(x0, q, x1, q) });
                }
            }
            d.stroke(p, lk.minor, Stroke::solid(0.3));
        }
        let mut p = Path::default();
        for (v, _) in &t.major {
            if let Some(q) = s.pos(*v) {
                p.extend(if vertical { Path::line(q, y0, q, y1) } else { Path::line(x0, q, x1, q) });
            }
        }
        d.stroke(p, lk.grid, Stroke::solid(0.45));
    };
    grid(d, &panel.x, &xticks, &sx, true);
    grid(d, &panel.y, &yticks, &sy, false);
    if has_y2 {
        grid(d, &panel.y2, &y2ticks, &sy2, false);
    }
    // data
    d.prims.push(Prim::Clip(x0, y0, x1, y1));
    for (k, l) in &visible {
        let ys = if l.y2 { &sy2 } else { &sy };
        for (s, st) in layer_series(*k).iter().zip(&styles[*k]) {
            draw_series(d, s, st, &sx, ys, lk.bg);
        }
    }
    d.prims.push(Prim::Unclip);
    if !any {
        let t = ty.t("No data: choose the source and the variables of a layer, or run the simulation", sz.ticks, false, Some((x1 - x0) * 0.8));
        d.text(&t, (x0 + x1 - t.width) / 2.0, (y0 + y1 - t.height) / 2.0, 0.0, lk.muted);
    }
    // frame and axis lines
    if lk.frame {
        d.stroke(Path::rect(x0, y0, x1, y1), lk.axis, Stroke::solid(0.6));
    }
    if lk.axis_lines {
        let mut p = Path::polyline(&[(x0, y0), (x0, y1), (x1, y1)]);
        if has_y2 {
            p.extend(Path::line(x1, y0, x1, y1));
        }
        d.stroke(p, lk.axis, Stroke { width: 0.7, dash: vec![], cap: Cap::Square });
    }
    // ticks and their labels
    let mut tick_path = Path::default();
    let minor_len = tick_len * 0.6;
    for ((v, _), t) in xticks.major.iter().zip(&xl) {
        if let Some(q) = sx.pos(*v) {
            tick_path.extend(Path::line(q, y1, q, y1 + tick_len));
            d.text(t, q - t.width / 2.0, y1 + tick_len + 2.0, 0.0, lk.fg);
        }
    }
    for ((v, _), t) in yticks.major.iter().zip(&yl) {
        if let Some(q) = sy.pos(*v) {
            tick_path.extend(Path::line(x0 - tick_len, q, x0, q));
            d.text(t, x0 - tick_len - 2.0 - t.width, q - t.height / 2.0, 0.0, lk.fg);
        }
    }
    if has_y2 {
        for ((v, _), t) in y2ticks.major.iter().zip(&y2l) {
            if let Some(q) = sy2.pos(*v) {
                tick_path.extend(Path::line(x1, q, x1 + tick_len, q));
                d.text(t, x1 + tick_len + 2.0, q - t.height / 2.0, 0.0, lk.fg);
            }
        }
    }
    if lk.ticks {
        let mut minor = |a: &Axis, t: &Ticks, s: &Scale, f: &dyn Fn(f32) -> Path| {
            if a.minor {
                for v in &t.minor {
                    if let Some(q) = s.pos(*v) {
                        tick_path.extend(f(q));
                    }
                }
            }
        };
        minor(&panel.x, &xticks, &sx, &|q| Path::line(q, y1, q, y1 + minor_len));
        minor(&panel.y, &yticks, &sy, &|q| Path::line(x0 - minor_len, q, x0, q));
        if has_y2 {
            minor(&panel.y2, &y2ticks, &sy2, &|q| Path::line(x1, q, x1 + minor_len, q));
        }
        d.stroke(tick_path, lk.axis, Stroke::solid(0.6));
    }
    // axis titles
    let x_title_y = y1 + tick_len + 3.0 + tick_h + 3.0;
    if !xt.is_empty() {
        d.text(&xt, (x0 + x1 - xt.width) / 2.0, x_title_y, 0.0, lk.fg);
    }
    if !yt.is_empty() {
        let yw = yl.iter().map(|t| t.width).fold(0.0, f32::max);
        let x = x0 - tick_len - 3.0 - yw - 4.0 - yt.height;
        d.text(&yt, x, (y0 + y1 + yt.width) / 2.0, std::f32::consts::FRAC_PI_2, lk.fg);
    }
    if has_y2 && !y2t.is_empty() {
        let w2 = y2l.iter().map(|t| t.width).fold(0.0, f32::max);
        let x = x1 + tick_len + 3.0 + w2 + 4.0 + y2t.height;
        d.text(&y2t, x, (y0 + y1 - y2t.width) / 2.0, -std::f32::consts::FRAC_PI_2, lk.fg);
    }
    // annotations
    for a in &panel.annotations {
        let ys = if a.on_y2 && has_y2 { &sy2 } else { &sy };
        let px = |x: f64| if a.relative { Some(x0 + x as f32 * (x1 - x0)) } else { sx.pos(x) };
        let py = |y: f64| if a.relative { Some(y1 - y as f32 * (y1 - y0)) } else { ys.pos(y) };
        let color = if a.color == [0, 0, 0] { lk.fg } else { Rgba::rgb(a.color) };
        let size = if a.size > 0.0 { a.size } else { sz.ticks };
        let t = ty.t(&a.text, size, false, None);
        let (dash, cap) = dash_pattern(a.dash, a.width as f32);
        let stroke = Stroke { width: a.width as f32, dash, cap };
        let hx = |x: f32| match a.anchor {
            Anchor::Left => x,
            Anchor::Center => x - t.width / 2.0,
            Anchor::Right => x - t.width,
        };
        match a.kind {
            AnnotationKind::Text => {
                if let (Some(x), Some(y)) = (px(a.x), py(a.y)) {
                    d.text(&t, hx(x), y - t.height / 2.0, 0.0, color);
                }
            }
            AnnotationKind::HLine => {
                if let Some(y) = py(a.y) {
                    d.prims.push(Prim::Clip(x0, y0, x1, y1));
                    d.stroke(Path::line(x0, y, x1, y), color, stroke);
                    d.prims.push(Prim::Unclip);
                    let x = match a.anchor {
                        Anchor::Left => x0 + 3.0,
                        Anchor::Center => (x0 + x1 - t.width) / 2.0,
                        Anchor::Right => x1 - 3.0 - t.width,
                    };
                    d.text(&t, x, y - 2.0 - t.height, 0.0, color);
                }
            }
            AnnotationKind::VLine => {
                if let Some(x) = px(a.x) {
                    d.prims.push(Prim::Clip(x0, y0, x1, y1));
                    d.stroke(Path::line(x, y0, x, y1), color, stroke);
                    d.prims.push(Prim::Unclip);
                    let tx = match a.anchor {
                        Anchor::Left => x + 3.0,
                        Anchor::Center => x - t.width / 2.0,
                        Anchor::Right => x - 3.0 - t.width,
                    };
                    d.text(&t, tx, y0 + 3.0, 0.0, color);
                }
            }
            AnnotationKind::Arrow => {
                if let (Some(xa), Some(ya), Some(xb), Some(yb)) = (px(a.x), py(a.y), px(a.x2), py(a.y2)) {
                    let (dx, dy) = (xb - xa, yb - ya);
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 0.5 {
                        let (ux, uy) = (dx / len, dy / len);
                        let head = (3.5 + 2.0 * a.width as f32).min(len);
                        let (bx, by) = (xb - ux * head, yb - uy * head);
                        d.stroke(Path::line(xa, ya, bx, by), color, stroke);
                        let hw = head * 0.42;
                        d.fill(Path::polygon(&[(xb, yb), (bx - uy * hw, by + ux * hw), (bx + uy * hw, by - ux * hw)]), color);
                    }
                    // the text at the start of the arrow, on the side away from it
                    let tx = if dx >= 0.0 { xa - 3.0 - t.width } else { xa + 3.0 };
                    d.text(&t, tx, ya - t.height / 2.0, 0.0, color);
                }
            }
        }
    }
    // legend
    match legend {
        LegendPos::None => {}
        LegendPos::Right => {
            let lw = legend_size(&entries, None).0;
            let mut x = x1 + 8.0;
            if has_y2 {
                let w2 = y2l.iter().map(|t| t.width).fold(0.0, f32::max);
                x += tick_len + 3.0 + w2 + if y2t.is_empty() { 0.0 } else { y2t.height + 4.0 };
            }
            draw_legend(d, &entries, x.min(cx1 - lw), y0, None, lk, false);
        }
        LegendPos::Bottom => {
            let (lw, lh) = legend_size(&entries, Some(cx1 - cx0));
            draw_legend(d, &entries, (x0 + x1 - lw) / 2.0, cy1 - lh, Some(cx1 - cx0), lk, false);
        }
        inside => {
            let (lw, lh) = legend_size(&entries, None);
            let inset = 5.0;
            let x = if matches!(inside, LegendPos::TopLeft | LegendPos::BottomLeft) { x0 + inset } else { x1 - inset - lw };
            let y = if matches!(inside, LegendPos::TopLeft | LegendPos::TopRight) { y0 + inset } else { y1 - inset - lh };
            draw_legend(d, &entries, x, y, None, lk, true);
        }
    }
    PanelGeom { rect, x: sx, y: sy, y2: if has_y2 { Some(sy2) } else { None } }
}

#[cfg(test)]
mod tests {
    use super::super::data::ColInfo;
    use super::*;
    use rusteau_core::figure::Annotation;

    fn series(n: usize, f: impl Fn(f64) -> f64, label: &str) -> Series {
        let x: Vec<f64> = (0..n).map(|k| k as f64).collect();
        Series {
            label: label.into(),
            y: x.iter().map(|v| f(*v)).collect(),
            x,
            x_info: ColInfo { key: "t".into(), name: "Time".into(), unit: "d".into() },
            y_info: ColInfo { key: "theta".into(), name: "Water content θ".into(), unit: "–".into() },
            item: 0,
            run: 0,
        }
    }

    #[test]
    fn ticks_are_nice() {
        let a = Axis::default();
        let ((lo, hi), t) = axis_ticks(&a, Some((0.08, 0.43)), 300.0, 40.0);
        assert!((lo - 0.05).abs() < 1e-12 && (hi - 0.45).abs() < 1e-12, "{lo} {hi}");
        assert_eq!(t.major.first().unwrap().1, "0.05");
        assert!(t.major.iter().any(|m| m.1 == "0.25"));
        // negative values with a minus sign
        let ((lo, _), t) = axis_ticks(&a, Some((-100.0, 0.0)), 200.0, 40.0);
        assert_eq!(lo, -100.0);
        assert_eq!(t.major[0].1, "−100");
        // fixed range and step
        let f = Axis { auto: false, min: 0.0, max: 30.0, step: 10.0, ..Default::default() };
        let (_, t) = axis_ticks(&f, Some((3.0, 7.0)), 300.0, 40.0);
        assert_eq!(t.major.iter().map(|m| m.1.as_str()).collect::<Vec<_>>(), vec!["0", "10", "20", "30"]);
        // large numbers: powers of ten
        let (_, t) = axis_ticks(&a, Some((0.0, 3e6)), 300.0, 40.0);
        assert!(t.major[1].1.contains("×10^{6}"), "{:?}", t.major);
        // logarithmic
        let l = Axis { log: true, ..Default::default() };
        let ((lo, hi), t) = axis_ticks(&l, Some((0.3, 15000.0)), 300.0, 30.0);
        assert_eq!((lo, hi), (0.1, 1e5));
        assert_eq!(t.major[0].1, "10^{−1}");
        assert!(t.minor.contains(&0.2));
        let (_, t) = axis_ticks(&l, Some((1.0, 100.0)), 300.0, 30.0);
        assert_eq!(t.major.iter().map(|m| m.1.as_str()).collect::<Vec<_>>(), vec!["1", "10", "100"]);
        // noise above a tick does not extend the axis
        let ((_, hi), _) = axis_ticks(&a, Some((0.0, 30.000001)), 300.0, 60.0);
        assert_eq!(hi, 30.0);
        // one value
        let ((lo, hi), _) = axis_ticks(&a, Some((5.0, 5.0)), 300.0, 40.0);
        assert!(lo < 5.0 && hi > 5.0);
        // scale
        let s = Scale { lo: 0.0, hi: 10.0, log: false, reverse: true, p0: 100.0, p1: 0.0 };
        assert_eq!(s.pos(0.0), Some(0.0));
        assert!((s.value(25.0) - 2.5).abs() < 1e-9);
        let s = Scale { lo: 1.0, hi: 100.0, log: true, reverse: false, p0: 0.0, p1: 100.0 };
        assert_eq!(s.pos(10.0), Some(50.0));
        assert_eq!(s.pos(-1.0), None);
    }

    #[test]
    fn figures_render_with_all_parts() {
        let mut fig = Figure { title: "Infiltration and **evaporation**".into(), subtitle: "loam, θ_{s} = 0.43".into(), caption: "Simulated with Rusteau".into(), rows: 1, cols: 2, ..Default::default() };
        fig.sync_panels();
        fig.panels[0].layers[0].geom = Geom::LinePoints;
        fig.panels[0].layers.push(Layer { y2: true, geom: Geom::Bars, ..Default::default() });
        fig.panels[0].annotations = vec![
            Annotation::default(),
            Annotation { kind: AnnotationKind::HLine, relative: false, y: 0.2, text: "field capacity".into(), ..Default::default() },
            Annotation { kind: AnnotationKind::Arrow, text: "ponding".into(), ..Default::default() },
        ];
        fig.panels[1].legend = LegendPos::Bottom;
        fig.panels[1].y.log = true;
        let s = vec![vec![vec![series(20, |t| 0.1 + 0.01 * t, "a")], vec![series(20, |t| (t * 0.5).sin(), "b")]], vec![vec![series(20, |t| 10f64.powf(t / 5.0), "c")]]];
        let r = render(&fig, &s);
        assert_eq!(r.panels.len(), 2);
        assert!(r.panels[0].y2.is_some() && r.panels[1].y2.is_none());
        let g = &r.panels[0];
        assert!(g.rect[0] > 20.0 && g.rect[2] < r.drawing.width / 2.0);
        // the hline annotation lies at y = 0.2
        let q = g.y.pos(0.2).unwrap();
        assert!(q > g.rect[1] && q < g.rect[3]);
        let texts = r.drawing.prims.iter().filter(|p| matches!(p, Prim::Text { .. })).count();
        assert!(texts > 30, "{texts}");
        // all themes and palettes render; an empty panel shows a hint
        for theme in [Theme::Grid, Theme::Classic, Theme::Minimal, Theme::Dark] {
            for palette in [Palette::OkabeIto, Palette::Tableau, Palette::Viridis, Palette::Grey] {
                let f = Figure { theme, palette, ..fig.clone() };
                let r = render(&f, &[]);
                assert!(r.drawing.prims.iter().any(|p| matches!(p, Prim::Text { t, .. } if t.spans.iter().any(|s| s.text.contains("No data")))));
                let pm = super::super::canvas::rasterize(&r.drawing, 0.5, false);
                assert!(pm.width() > 10);
            }
        }
        // comparison: two items in two runs → colours by item, line styles by run
        let a = series(5, |t| t, "n1 A");
        let mut b = series(5, |t| t, "n2 A");
        let mut c = series(5, |t| t, "n1 B");
        b.item = 1;
        c.run = 1;
        let panel = Panel::default();
        let st = styles(&fig, &panel, &[vec![a, b, c]], false);
        assert_eq!(st[0][0].color, st[0][2].color);
        assert_ne!(st[0][0].color, st[0][1].color);
        assert_ne!(st[0][0].dash, st[0][2].dash);
        // log axis notes
        let mut f = Figure::default();
        f.panels[0].y.log = true;
        let r = render(&f, &[vec![vec![series(5, |t| t - 1.0, "x")]]]);
        assert!(!r.notes.is_empty());
    }

    #[test]
    fn automatic_titles() {
        let a = ColInfo { key: "a".into(), name: "Actual surface flux".into(), unit: "cm/d".into() };
        let b = ColInfo { key: "b".into(), name: "Bottom flux".into(), unit: "cm/d".into() };
        let c = ColInfo { key: "c".into(), name: "Water content θ".into(), unit: "–".into() };
        assert_eq!(auto_title(&[&a]), "Actual surface flux (cm/d)");
        assert_eq!(auto_title(&[&a, &b]), "Actual surface flux, Bottom flux (cm/d)");
        assert_eq!(auto_title(&[&a, &c]), "Actual surface flux (cm/d), Water content θ (–)");
        assert_eq!(escape_markup("a*b"), "a\\*b");
    }
}

#[cfg(test)]
mod visual {
    use super::super::data::{DataCtx, SavedRun};
    use super::*;
    use rusteau_core::figure::{Annotation, Source};

    /// Writes example figures to `$RUSTEAU_FIG_OUT` (`cargo test -- --ignored visual`).
    #[test]
    #[ignore]
    fn visual_examples() {
        let out = std::path::PathBuf::from(std::env::var("RUSTEAU_FIG_OUT").unwrap_or_else(|_| "/tmp".into()));
        let p = rusteau_core::examples::infiltration_evaporation();
        let mut sim = rusteau_core::Simulation::new(p.clone()).unwrap();
        sim.run(|_| true);
        let r = std::mem::take(&mut sim.res);
        let mut p2 = p.clone();
        p2.water.materials[0].ks *= 0.3;
        let mut sim = rusteau_core::Simulation::new(p2.clone()).unwrap();
        sim.run(|_| true);
        let runs = vec![SavedRun { name: "Ks × 0.3".into(), project: p2, results: std::mem::take(&mut sim.res) }];
        let ctx = DataCtx { prj: &p, results: Some(&r), runs: &runs, measured: &[], fit: None };
        let mut fig = Figure {
            title: "Infiltration and evaporation".into(),
            subtitle: "Water content θ and pressure head h, loam".into(),
            caption: "Simulated with Rusteau 1.0; Noto Sans and Noto Sans Math (SIL OFL 1.1).".into(),
            cols: 2,
            ..Default::default()
        };
        fig.sync_panels();
        fig.panels[0] = Panel {
            title: "(a) Profiles".into(),
            layers: vec![Layer { source: Source::Profiles, x: "theta".into(), y: "depth".into(), ..Default::default() }],
            legend: LegendPos::BottomRight,
            ..Default::default()
        };
        fig.panels[1] = Panel {
            title: "(b) Surface flux, comparison".into(),
            layers: vec![
                Layer { source: Source::TimeLevel, runs: vec![String::new(), "Ks × 0.3".into()], x: "t".into(), y: "vTop".into(), ..Default::default() },
                Layer { source: Source::AtmInput, x: "t".into(), y: "prec".into(), geom: Geom::Steps, y2: true, color: Some([120, 120, 120]), ..Default::default() },
            ],
            annotations: vec![Annotation { kind: AnnotationKind::Arrow, text: "rain".into(), x: 0.55, y: 0.8, x2: 0.4, y2: 0.6, ..Default::default() }],
            legend: LegendPos::Bottom,
            ..Default::default()
        };
        fig.panels[1].y.label_auto = false;
        fig.panels[1].y.label = "Surface flux v_{top} (cm d^{−1})".into();
        let series: Vec<Vec<Vec<Series>>> = fig.panels.iter().map(|p| p.layers.iter().map(|l| ctx.series(l, &fig)).collect()).collect();
        for theme in [Theme::Grid, Theme::Classic, Theme::Dark] {
            let f = Figure { theme, ..fig.clone() };
            let rd = render(&f, &series);
            let pm = super::super::canvas::rasterize(&rd.drawing, 150.0 / 72.0, false);
            super::super::canvas::write_png(&pm, 150.0, &out.join(format!("fig_{theme:?}.png"))).unwrap();
            std::fs::write(out.join(format!("fig_{theme:?}.svg")), super::super::canvas::to_svg(&rd.drawing, false, false)).unwrap();
        }
    }
}
