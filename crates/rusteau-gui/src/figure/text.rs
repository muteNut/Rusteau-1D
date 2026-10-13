//! Text of the figures: the markup of the help (`θ_{s}`, `cm^{3}`, `*italic*`, `**bold**`) laid out with the glyphs
//! of a font family. A character missing in the family is taken from Noto Sans, then from Noto Sans Math, so every
//! symbol of the labels can be drawn whatever family is chosen.

use crate::fonts;
use crate::help::markup::{self, Script};
use ab_glyph::{Font, FontArc, GlyphId, OutlineCurve};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Size of sub- and superscripts relative to the text.
const SCRIPT_SIZE: f32 = 0.7;
/// Baseline shifts of sub- (down) and superscripts (up) [em of the text].
const SUB_SHIFT: f32 = 0.22;
const SUP_SHIFT: f32 = 0.40;
/// Height of capitals and depth of descenders used for the text boxes [em].
pub const ASCENT: f32 = 0.78;
pub const DESCENT: f32 = 0.22;
/// Distance of the baselines of wrapped lines [em].
pub const LINE_SPACING: f32 = 1.25;

/// The fonts of a family: regular, bold, italic and bold italic (a missing style uses the regular font), the same
/// styles of Noto Sans and Noto Sans Math as fallbacks.
pub struct FontSet {
    pub family: String,
    fonts: Vec<FontArc>,
}

impl FontSet {
    /// Index of the font that draws `c` in style `style` (0 regular, 1 bold, 2 italic, 3 bold italic).
    fn pick(&self, c: char, style: usize) -> usize {
        for k in [style, 4 + style, 8] {
            if self.fonts[k].glyph_id(c).0 != 0 {
                return k;
            }
        }
        style
    }

    pub fn font(&self, k: u8) -> &FontArc {
        &self.fonts[k as usize]
    }

    /// Whether font `k` is Noto Sans Math (the last fallback).
    #[cfg(test)]
    pub fn is_math(&self, k: u8) -> bool {
        k == 8
    }
}

fn bundled() -> [FontArc; 5] {
    let f = |b: &'static [u8]| FontArc::try_from_slice(b).expect("bundled font");
    [f(fonts::NOTO_SANS), f(fonts::NOTO_SANS_BOLD), f(fonts::NOTO_SANS_ITALIC), f(fonts::NOTO_SANS_BOLD_ITALIC), f(fonts::NOTO_SANS_MATH)]
}

fn load(path: &Option<std::path::PathBuf>) -> Option<FontArc> {
    let data = std::fs::read(path.as_ref()?).ok()?;
    FontArc::try_from_vec(data).ok()
}

/// The fonts of `family` (loaded once); unknown families use Noto Sans.
pub fn font_set(family: &str) -> Arc<FontSet> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<FontSet>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(f) = cache.lock().unwrap().get(family) {
        return f.clone();
    }
    let [r, b, i, bi, math] = bundled();
    let noto = [r, b, i, bi];
    let mut own = noto.clone();
    let mut name = fonts::DEFAULT_FAMILY.to_string();
    if family != fonts::DEFAULT_FAMILY {
        if let Some(sf) = fonts::system_families().iter().find(|f| f.name == family) {
            if let Some(reg) = load(&sf.regular) {
                let bold = load(&sf.bold).unwrap_or_else(|| reg.clone());
                let italic = load(&sf.italic).unwrap_or_else(|| reg.clone());
                let bold_italic = load(&sf.bold_italic).unwrap_or_else(|| bold.clone());
                own = [reg, bold, italic, bold_italic];
                name = family.to_string();
            }
        }
    }
    let mut v: Vec<FontArc> = own.to_vec();
    v.extend(noto);
    v.push(math);
    let set = Arc::new(FontSet { family: name, fonts: v });
    cache.lock().unwrap().insert(family.to_string(), set.clone());
    set
}

/// A glyph placed in a text box: `x` from the left, `y` the baseline from the top [pt].
#[derive(Clone, Debug)]
pub struct Glyph {
    pub font: u8,
    pub id: GlyphId,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

/// A run of characters of one style for the text elements of SVG files.
#[derive(Clone, Debug)]
pub struct SpanOut {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

/// Laid-out text. The box reaches from 0 to `width` and from 0 (top of the first line) to `height`.
#[derive(Clone)]
pub struct TextLayout {
    pub glyphs: Vec<Glyph>,
    pub spans: Vec<SpanOut>,
    pub width: f32,
    pub height: f32,
    /// baseline of the first line from the top
    pub baseline: f32,
    pub fonts: Arc<FontSet>,
}

impl std::fmt::Debug for TextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextLayout({:?}, {}×{})", self.spans.iter().map(|s| s.text.as_str()).collect::<String>(), self.width, self.height)
    }
}

impl TextLayout {
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

#[derive(Clone, Copy)]
struct Atom {
    c: char,
    font: u8,
    id: GlyphId,
    size: f32,
    shift: f32,
    adv: f32,
    bold: bool,
    italic: bool,
}

/// Lays out `text` (markup of the help, lines separated by `\n`) at `size` pt; lines longer than `max_width` are
/// wrapped at blanks.
pub fn layout(text: &str, size: f32, family: &str, bold: bool, max_width: Option<f32>) -> TextLayout {
    layout_aligned(text, size, family, bold, max_width, false)
}

/// As [`layout`], with every line centred in the box.
pub fn layout_centred(text: &str, size: f32, family: &str, bold: bool, max_width: Option<f32>) -> TextLayout {
    layout_aligned(text, size, family, bold, max_width, true)
}

fn layout_aligned(text: &str, size: f32, family: &str, bold: bool, max_width: Option<f32>, centre: bool) -> TextLayout {
    let fs = font_set(family);
    let mut lines: Vec<Vec<Atom>> = vec![];
    for par in text.split('\n') {
        let mut atoms = vec![];
        for sp in markup::inline(par) {
            let b = bold || sp.bold;
            let style = (b as usize) + 2 * (sp.italic as usize);
            let (sz, shift) = match sp.script {
                Script::Normal => (size, 0.0),
                Script::Sub => (size * SCRIPT_SIZE, SUB_SHIFT * size),
                Script::Sup => (size * SCRIPT_SIZE, -SUP_SHIFT * size),
            };
            for c in sp.text.chars() {
                let c = if c == '\t' { ' ' } else { c };
                let k = fs.pick(c, style);
                let f = &fs.fonts[k];
                let id = f.glyph_id(c);
                let em = f.units_per_em().unwrap_or(1000.0);
                let adv = f.h_advance_unscaled(id) * sz / em;
                atoms.push(Atom { c, font: k as u8, id, size: sz, shift, adv, bold: b, italic: sp.italic });
            }
        }
        // wrap at blanks
        match max_width {
            Some(mw) if mw > 0.0 => {
                let mut line: Vec<Atom> = vec![];
                let mut w = 0.0;
                let mut word: Vec<Atom> = vec![];
                let mut ww = 0.0;
                let flush_word = |line: &mut Vec<Atom>, w: &mut f32, word: &mut Vec<Atom>, ww: &mut f32, lines: &mut Vec<Vec<Atom>>| {
                    if *w + *ww > mw && !line.is_empty() {
                        while line.last().is_some_and(|a| a.c == ' ') {
                            line.pop();
                        }
                        lines.push(std::mem::take(line));
                        *w = 0.0;
                        while word.first().is_some_and(|a| a.c == ' ') {
                            *ww -= word[0].adv;
                            word.remove(0);
                        }
                    }
                    *w += *ww;
                    line.append(word);
                    *ww = 0.0;
                };
                for a in atoms {
                    if a.c == ' ' && word.iter().any(|x| x.c != ' ') {
                        flush_word(&mut line, &mut w, &mut word, &mut ww, &mut lines);
                    }
                    ww += a.adv;
                    word.push(a);
                }
                flush_word(&mut line, &mut w, &mut word, &mut ww, &mut lines);
                lines.push(line);
            }
            _ => lines.push(atoms),
        }
    }
    let mut out = TextLayout { glyphs: vec![], spans: vec![], width: 0.0, height: 0.0, baseline: 0.0, fonts: fs.clone() };
    let mut y = 0.0;
    // first glyph, first span and width of every line
    let mut starts: Vec<(usize, usize, f32)> = vec![];
    for (li, line) in lines.iter().enumerate() {
        let (g0, s0) = (out.glyphs.len(), out.spans.len());
        // the first line: room for superscripts above the capitals
        let asc = line.iter().map(|a| ASCENT * a.size - a.shift).fold(ASCENT * size, f32::max);
        let desc = line.iter().map(|a| DESCENT * a.size + a.shift).fold(DESCENT * size, f32::max);
        y += if li == 0 { asc } else { LINE_SPACING * size };
        if li == 0 {
            out.baseline = y;
        }
        let mut x = 0.0;
        let mut prev: Option<(u8, GlyphId, f32)> = None;
        for a in line {
            if let Some((pf, pid, psz)) = prev {
                if pf == a.font && psz == a.size {
                    let f = fs.font(a.font);
                    x += f.kern_unscaled(pid, a.id) * a.size / f.units_per_em().unwrap_or(1000.0);
                }
            }
            if a.c != ' ' {
                out.glyphs.push(Glyph { font: a.font, id: a.id, x, y: y + a.shift, size: a.size });
            }
            match out.spans.last_mut() {
                // a span ends at a change of style or line (another baseline)
                Some(s) if s.size == a.size && s.bold == a.bold && s.italic == a.italic && s.y == y + a.shift => s.text.push(a.c),
                _ => out.spans.push(SpanOut { text: a.c.to_string(), x, y: y + a.shift, size: a.size, bold: a.bold, italic: a.italic }),
            }
            x += a.adv;
            prev = Some((a.font, a.id, a.size));
        }
        out.width = out.width.max(x);
        out.height = y + desc;
        starts.push((g0, s0, x));
    }
    if centre {
        for (k, &(g0, s0, w)) in starts.iter().enumerate() {
            let (g1, s1) = starts.get(k + 1).map(|n| (n.0, n.1)).unwrap_or((out.glyphs.len(), out.spans.len()));
            let dx = (out.width - w) / 2.0;
            out.glyphs[g0..g1].iter_mut().for_each(|g| g.x += dx);
            out.spans[s0..s1].iter_mut().for_each(|s| s.x += dx);
        }
    }
    // blanks at the ends of the spans do not matter; empty texts have no height
    if out.glyphs.is_empty() {
        out.height = 0.0;
        out.width = 0.0;
    }
    out
}

/// Receives the outlines of glyphs and other paths.
pub trait PathSink {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32);
    fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32);
    fn close(&mut self);
}

/// Writes the outlines of the glyphs of `t`, its box placed with its top left corner at (`x`, `y`) and rotated by
/// `angle` (radians, counter-clockwise on the page) around that corner.
pub fn outline(t: &TextLayout, x: f32, y: f32, angle: f32, sink: &mut dyn PathSink) {
    let (s, c) = angle.sin_cos();
    // page y grows downward: a counter-clockwise rotation on the page
    let tr = |px: f32, py: f32| (x + px * c + py * s, y - px * s + py * c);
    for g in &t.glyphs {
        let f = t.fonts.font(g.font);
        let Some(o) = f.outline(g.id) else { continue };
        let k = g.size / f.units_per_em().unwrap_or(1000.0);
        let p = |q: ab_glyph::Point| tr(g.x + q.x * k, g.y - q.y * k);
        let mut last: Option<ab_glyph::Point> = None;
        let mut open = false;
        for cv in &o.curves {
            let start = match cv {
                OutlineCurve::Line(a, _) | OutlineCurve::Quad(a, _, _) | OutlineCurve::Cubic(a, _, _, _) => *a,
            };
            if last != Some(start) {
                if open {
                    sink.close();
                }
                let (mx, my) = p(start);
                sink.move_to(mx, my);
                open = true;
            }
            match cv {
                OutlineCurve::Line(_, b) => {
                    let (bx, by) = p(*b);
                    sink.line_to(bx, by);
                    last = Some(*b);
                }
                OutlineCurve::Quad(_, b, e) => {
                    let (bx, by) = p(*b);
                    let (ex, ey) = p(*e);
                    sink.quad_to(bx, by, ex, ey);
                    last = Some(*e);
                }
                OutlineCurve::Cubic(_, b, d, e) => {
                    let (bx, by) = p(*b);
                    let (dx, dy) = p(*d);
                    let (ex, ey) = p(*e);
                    sink.cubic_to(bx, by, dx, dy, ex, ey);
                    last = Some(*e);
                }
            }
        }
        if open {
            sink.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Count(usize);
    impl PathSink for Count {
        fn move_to(&mut self, _: f32, _: f32) {
            self.0 += 1;
        }
        fn line_to(&mut self, _: f32, _: f32) {}
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
        fn cubic_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
        fn close(&mut self) {}
    }

    #[test]
    fn rich_text_with_scripts_fallback_and_wrapping() {
        let t = layout("θ_{s} (cm^{3} cm^{−3})", 10.0, "Noto Sans", false, None);
        assert!(t.width > 40.0 && t.width < 150.0, "{}", t.width);
        // the subscript is smaller and lower than the θ
        assert!(t.glyphs[1].size < t.glyphs[0].size && t.glyphs[1].y > t.glyphs[0].y);
        // the superscript is above the baseline
        let sup = t.glyphs.iter().find(|g| g.y < t.baseline).expect("superscript");
        assert!(sup.size < 10.0);
        // ∂ and → are not in Noto Sans: drawn with Noto Sans Math
        let m = layout("∂θ/∂t → ∞", 10.0, "Noto Sans", false, None);
        assert!(m.glyphs.iter().any(|g| m.fonts.is_math(g.font)));
        assert!(m.glyphs.iter().all(|g| g.id.0 != 0));
        // bold and italic markup choose other fonts
        let b = layout("a **b** *c*", 10.0, "Noto Sans", false, None);
        let fonts: Vec<u8> = b.glyphs.iter().map(|g| g.font).collect();
        assert_eq!(fonts, vec![0, 1, 2]);
        // wrapping
        let long = "word ".repeat(40);
        let w = layout(&long, 10.0, "Noto Sans", false, Some(100.0));
        assert!(w.width <= 100.0 + 1e-3, "{}", w.width);
        assert!(w.height > 5.0 * LINE_SPACING * 10.0);
        // explicit line breaks
        let two = layout("a\nb", 10.0, "Noto Sans", false, None);
        assert!(two.glyphs[1].y - two.glyphs[0].y > 10.0);
        // outlines: one contour for "l", several for "θ"
        let mut n = Count(0);
        outline(&layout("l", 10.0, "Noto Sans", false, None), 0.0, 0.0, 0.0, &mut n);
        assert_eq!(n.0, 1);
        // empty text
        assert!(layout("", 10.0, "Noto Sans", false, None).is_empty());
        // an unknown family falls back to Noto Sans
        assert_eq!(font_set("No such family").family, "Noto Sans");
    }
}
