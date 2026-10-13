//! A drawing as a list of primitives in points (1/72 inch, y downward), rasterized with tiny-skia (preview and PNG) or
//! written as SVG. Both use the same geometry and the same glyph outlines, so the files look like the preview.

use super::text::{self, PathSink, TextLayout};
use std::fmt::Write as _;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub fn rgb(c: [u8; 3]) -> Rgba {
        Rgba(c[0], c[1], c[2], 255)
    }
    pub fn alpha(self, a: f64) -> Rgba {
        Rgba(self.0, self.1, self.2, (a.clamp(0.0, 1.0) * self.3 as f64).round() as u8)
    }
    fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path(pub Vec<Seg>);

impl Path {
    pub fn polyline(pts: &[(f32, f32)]) -> Path {
        let mut p = Path::default();
        for (k, &(x, y)) in pts.iter().enumerate() {
            p.0.push(if k == 0 { Seg::M(x, y) } else { Seg::L(x, y) });
        }
        p
    }
    pub fn polygon(pts: &[(f32, f32)]) -> Path {
        let mut p = Path::polyline(pts);
        p.0.push(Seg::Z);
        p
    }
    pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        Path::polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }
    pub fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        Path(vec![Seg::M(x0, y0), Seg::L(x1, y1)])
    }
    /// Circle of radius `r` around (`x`, `y`) from four cubic arcs.
    pub fn circle(x: f32, y: f32, r: f32) -> Path {
        let k = 0.552_284_8 * r;
        Path(vec![
            Seg::M(x + r, y),
            Seg::C(x + r, y + k, x + k, y + r, x, y + r),
            Seg::C(x - k, y + r, x - r, y + k, x - r, y),
            Seg::C(x - r, y - k, x - k, y - r, x, y - r),
            Seg::C(x + k, y - r, x + r, y - k, x + r, y),
            Seg::Z,
        ])
    }
    pub fn extend(&mut self, other: Path) {
        self.0.extend(other.0);
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl PathSink for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Seg::M(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Seg::L(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push(Seg::Q(x1, y1, x, y));
    }
    fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push(Seg::C(x1, y1, x2, y2, x, y));
    }
    fn close(&mut self) {
        self.0.push(Seg::Z);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub dash: Vec<f32>,
    pub cap: Cap,
}

impl Stroke {
    pub fn solid(width: f32) -> Stroke {
        Stroke { width, dash: vec![], cap: Cap::Butt }
    }
}

#[derive(Clone, Debug)]
pub enum Prim {
    Fill(Path, Rgba),
    Stroke(Path, Rgba, Stroke),
    /// text box with its top left corner at (`x`, `y`), rotated by `angle` (radians, counter-clockwise)
    Text { t: Arc<TextLayout>, x: f32, y: f32, angle: f32, color: Rgba },
    /// the following primitives are clipped to the rectangle until `Unclip`
    Clip(f32, f32, f32, f32),
    Unclip,
}

/// A figure: size [pt] and primitives in drawing order.
#[derive(Clone, Debug, Default)]
pub struct Drawing {
    pub width: f32,
    pub height: f32,
    /// background (drawn unless the export is transparent)
    pub background: Option<Rgba>,
    pub prims: Vec<Prim>,
}

impl Drawing {
    pub fn fill(&mut self, p: Path, c: Rgba) {
        if !p.is_empty() && c.3 > 0 {
            self.prims.push(Prim::Fill(p, c));
        }
    }
    pub fn stroke(&mut self, p: Path, c: Rgba, s: Stroke) {
        if !p.is_empty() && c.3 > 0 && s.width > 0.0 {
            self.prims.push(Prim::Stroke(p, c, s));
        }
    }
    pub fn text(&mut self, t: &Arc<TextLayout>, x: f32, y: f32, angle: f32, color: Rgba) {
        if !t.is_empty() {
            self.prims.push(Prim::Text { t: t.clone(), x, y, angle, color });
        }
    }
}

fn skia_path(p: &Path) -> Option<tiny_skia::Path> {
    let mut b = tiny_skia::PathBuilder::new();
    for s in &p.0 {
        match *s {
            Seg::M(x, y) => b.move_to(x, y),
            Seg::L(x, y) => b.line_to(x, y),
            Seg::Q(x1, y1, x, y) => b.quad_to(x1, y1, x, y),
            Seg::C(x1, y1, x2, y2, x, y) => b.cubic_to(x1, y1, x2, y2, x, y),
            Seg::Z => b.close(),
        }
    }
    b.finish()
}

fn paint(c: Rgba) -> tiny_skia::Paint<'static> {
    let mut p = tiny_skia::Paint::default();
    p.set_color_rgba8(c.0, c.1, c.2, c.3);
    p.anti_alias = true;
    p
}

/// Text as filled glyph outlines.
pub fn text_path(t: &TextLayout, x: f32, y: f32, angle: f32) -> Path {
    let mut p = Path::default();
    text::outline(t, x, y, angle, &mut p);
    p
}

/// Largest image [pixels] (about 480 MB of memory).
pub const MAX_PIXELS: u64 = 120_000_000;

/// Rasterizes the drawing at `scale` pixels per point; an image larger than [`MAX_PIXELS`] is an error.
pub fn rasterize(d: &Drawing, scale: f32, transparent: bool) -> Result<tiny_skia::Pixmap, String> {
    let w = (d.width * scale).round().max(1.0);
    let h = (d.height * scale).round().max(1.0);
    if !(w.is_finite() && h.is_finite()) || (w as f64) * (h as f64) > MAX_PIXELS as f64 {
        return Err(format!("The image would have {w:.0} × {h:.0} pixels; lower the resolution or the size of the figure."));
    }
    let (w, h) = (w as u32, h as u32);
    let mut pm = tiny_skia::Pixmap::new(w, h).ok_or_else(|| format!("No memory for an image of {w} × {h} pixels."))?;
    if let (Some(bg), false) = (d.background, transparent) {
        pm.fill(tiny_skia::Color::from_rgba8(bg.0, bg.1, bg.2, bg.3));
    }
    let tr = tiny_skia::Transform::from_scale(scale, scale);
    let mut mask: Option<tiny_skia::Mask> = None;
    for prim in &d.prims {
        match prim {
            Prim::Fill(p, c) => {
                if let Some(sp) = skia_path(p) {
                    pm.fill_path(&sp, &paint(*c), tiny_skia::FillRule::Winding, tr, mask.as_ref());
                }
            }
            Prim::Stroke(p, c, s) => {
                if let Some(sp) = skia_path(p) {
                    let mut st = tiny_skia::Stroke { width: s.width, ..Default::default() };
                    st.line_cap = match s.cap {
                        Cap::Butt => tiny_skia::LineCap::Butt,
                        Cap::Round => tiny_skia::LineCap::Round,
                        Cap::Square => tiny_skia::LineCap::Square,
                    };
                    st.line_join = tiny_skia::LineJoin::Round;
                    if s.dash.len() >= 2 {
                        st.dash = tiny_skia::StrokeDash::new(s.dash.clone(), 0.0);
                    }
                    pm.stroke_path(&sp, &paint(*c), &st, tr, mask.as_ref());
                }
            }
            Prim::Text { t, x, y, angle, color } => {
                if let Some(sp) = skia_path(&text_path(t, *x, *y, *angle)) {
                    pm.fill_path(&sp, &paint(*color), tiny_skia::FillRule::Winding, tr, mask.as_ref());
                }
            }
            Prim::Clip(x0, y0, x1, y1) => {
                mask = tiny_skia::Mask::new(w, h).and_then(|mut m| {
                    let r = skia_path(&Path::rect(*x0, *y0, *x1, *y1))?;
                    m.fill_path(&r, tiny_skia::FillRule::Winding, true, tr);
                    Some(m)
                });
            }
            Prim::Unclip => mask = None,
        }
    }
    Ok(pm)
}

/// Colours of the pixels without premultiplied alpha (RGBA, row by row).
pub fn rgba(pm: &tiny_skia::Pixmap) -> Vec<u8> {
    let mut v = Vec::with_capacity(pm.data().len());
    for p in pm.pixels() {
        let c = p.demultiply();
        v.extend([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    v
}

/// Writes a PNG file with the resolution `dpi` stored in the file (pHYs chunk).
pub fn write_png(pm: &tiny_skia::Pixmap, dpi: f64, path: &std::path::Path) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), pm.width(), pm.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let ppm = (dpi / 0.0254).round() as u32;
    enc.set_pixel_dims(Some(png::PixelDimensions { xppu: ppm, yppu: ppm, unit: png::Unit::Meter }));
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(&rgba(pm)).map_err(|e| e.to_string())?;
    w.finish().map_err(|e| e.to_string())
}

fn num(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn svg_d(p: &Path) -> String {
    let mut s = String::new();
    for seg in &p.0 {
        match *seg {
            Seg::M(x, y) => write!(s, "M{} {}", num(x), num(y)),
            Seg::L(x, y) => write!(s, "L{} {}", num(x), num(y)),
            Seg::Q(a, b, x, y) => write!(s, "Q{} {} {} {}", num(a), num(b), num(x), num(y)),
            Seg::C(a, b, c, d, x, y) => write!(s, "C{} {} {} {} {} {}", num(a), num(b), num(c), num(d), num(x), num(y)),
            Seg::Z => write!(s, "Z"),
        }
        .ok();
    }
    s
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn color_attr(kind: &str, c: Rgba) -> String {
    if c.3 == 255 {
        format!("{kind}=\"{}\"", c.hex())
    } else {
        format!("{kind}=\"{}\" {kind}-opacity=\"{}\"", c.hex(), num(c.3 as f32 / 255.0))
    }
}

/// The drawing as an SVG document; the size is given in millimetres, the coordinates are points. Text is written as
/// text elements (editable; the fonts must be installed where the file is shown) or as glyph outlines.
pub fn to_svg(d: &Drawing, text_as_paths: bool, transparent: bool) -> String {
    let mm = |pt: f32| pt / 72.0 * 25.4;
    let mut s = String::new();
    writeln!(s, r#"<?xml version="1.0" encoding="UTF-8"?>"#).ok();
    writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="0 0 {} {}">"#,
        num(mm(d.width)),
        num(mm(d.height)),
        num(d.width),
        num(d.height)
    )
    .ok();
    writeln!(s, "<!-- {} {} -->", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).ok();
    if let (Some(bg), false) = (d.background, transparent) {
        writeln!(s, r#"<rect width="{}" height="{}" {}/>"#, num(d.width), num(d.height), color_attr("fill", bg)).ok();
    }
    let mut clips = 0;
    let mut open = false;
    for prim in &d.prims {
        match prim {
            Prim::Fill(p, c) => {
                writeln!(s, r#"<path d="{}" {}/>"#, svg_d(p), color_attr("fill", *c)).ok();
            }
            Prim::Stroke(p, c, st) => {
                let mut a = format!(r#"fill="none" {} stroke-width="{}" stroke-linejoin="round""#, color_attr("stroke", *c), num(st.width));
                match st.cap {
                    Cap::Butt => {}
                    Cap::Round => a.push_str(r#" stroke-linecap="round""#),
                    Cap::Square => a.push_str(r#" stroke-linecap="square""#),
                }
                if st.dash.len() >= 2 {
                    write!(a, r#" stroke-dasharray="{}""#, st.dash.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" ")).ok();
                }
                writeln!(s, r#"<path d="{}" {a}/>"#, svg_d(p)).ok();
            }
            Prim::Text { t, x, y, angle, color } => {
                if text_as_paths {
                    writeln!(s, r#"<path d="{}" {}/>"#, svg_d(&text_path(t, *x, *y, *angle)), color_attr("fill", *color)).ok();
                } else {
                    let rot = if *angle != 0.0 { format!(r#" transform="rotate({} {} {})""#, num(-angle.to_degrees()), num(*x), num(*y)) } else { String::new() };
                    let family = if t.fonts.family == crate::fonts::DEFAULT_FAMILY {
                        "'Noto Sans', 'Noto Sans Math', sans-serif".to_string()
                    } else {
                        format!("'{}', 'Noto Sans', 'Noto Sans Math', sans-serif", esc(&t.fonts.family))
                    };
                    write!(s, r#"<text xml:space="preserve" font-family="{family}" {}{rot}>"#, color_attr("fill", *color)).ok();
                    for sp in &t.spans {
                        let mut a = format!(r#"x="{}" y="{}" font-size="{}""#, num(x + sp.x), num(y + sp.y), num(sp.size));
                        if sp.bold {
                            a.push_str(r#" font-weight="bold""#);
                        }
                        if sp.italic {
                            a.push_str(r#" font-style="italic""#);
                        }
                        write!(s, "<tspan {a}>{}</tspan>", esc(&sp.text)).ok();
                    }
                    writeln!(s, "</text>").ok();
                }
            }
            Prim::Clip(x0, y0, x1, y1) => {
                if open {
                    writeln!(s, "</g>").ok();
                }
                clips += 1;
                writeln!(
                    s,
                    r#"<clipPath id="clip{clips}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath><g clip-path="url(#clip{clips})">"#,
                    num(*x0),
                    num(*y0),
                    num(x1 - x0),
                    num(y1 - y0)
                )
                .ok();
                open = true;
            }
            Prim::Unclip => {
                if open {
                    writeln!(s, "</g>").ok();
                    open = false;
                }
            }
        }
    }
    if open {
        writeln!(s, "</g>").ok();
    }
    writeln!(s, "</svg>").ok();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_png_and_svg() {
        let mut d = Drawing { width: 100.0, height: 50.0, background: Some(Rgba(255, 255, 255, 255)), prims: vec![] };
        d.fill(Path::rect(10.0, 10.0, 30.0, 30.0), Rgba(255, 0, 0, 255));
        d.prims.push(Prim::Clip(0.0, 0.0, 50.0, 50.0));
        d.stroke(Path::line(0.0, 40.0, 100.0, 40.0), Rgba(0, 0, 255, 255), Stroke { width: 2.0, dash: vec![4.0, 2.0], cap: Cap::Butt });
        d.prims.push(Prim::Unclip);
        let t = Arc::new(text::layout("θ_{s} & <x>", 10.0, "Noto Sans", false, None));
        d.text(&t, 40.0, 5.0, 0.0, Rgba(0, 0, 0, 255));
        d.text(&t, 90.0, 45.0, std::f32::consts::FRAC_PI_2, Rgba(0, 0, 0, 255));
        let pm = rasterize(&d, 2.0, false).unwrap();
        assert_eq!((pm.width(), pm.height()), (200, 100));
        // red square, white background, the dashed line clipped at x = 50 pt
        let px = |x: u32, y: u32| pm.pixel(x, y).unwrap().demultiply();
        assert_eq!((px(40, 40).red(), px(40, 40).green()), (255, 0));
        assert_eq!(px(150, 5).green(), 255);
        assert_eq!(px(180, 80).blue(), 255);
        assert_eq!(px(180, 80).red(), 255);
        // the text has dark pixels
        let dark = (80..200).flat_map(|x| (10..30).map(move |y| (x, y))).filter(|&(x, y)| px(x, y).red() < 100).count();
        assert!(dark > 20, "{dark}");
        // transparent: no background
        assert_eq!(rasterize(&d, 1.0, true).unwrap().pixel(90, 2).unwrap().alpha(), 0);
        // too large an image is an error, not a crash
        assert!(rasterize(&d, 1e4, false).is_err());
        let dir = std::env::temp_dir().join(format!("rusteau-canvas-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_png(&pm, 300.0, &dir.join("a.png")).unwrap();
        let bytes = std::fs::read(dir.join("a.png")).unwrap();
        assert!(bytes.windows(4).any(|w| w == b"pHYs"));
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!(img.width(), 200);
        let _ = std::fs::remove_dir_all(&dir);
        let svg = to_svg(&d, false, false);
        assert!(svg.contains("<tspan") && svg.contains("&amp; &lt;x&gt;") && svg.contains("clip-path") && svg.contains("rotate(-90"));
        assert!(svg.contains(r#"width="35.28mm""#), "{}", &svg[..200]);
        let paths = to_svg(&d, true, true);
        assert!(!paths.contains("<text") && !paths.contains("<rect width"));
    }
}
