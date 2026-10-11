//! Drawing of help topics with egui: wrapped paragraphs with clickable links and citations, sub- and superscripts,
//! formulas, tables, lists, notes and images.

use super::markup::{Block, Doc, Inline, Link, Script};
use super::{refs, topics};
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontFamily, FontId, Galley, Pos2, Rect, Sense, Stroke, TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use std::sync::Arc;

/// What a click in a topic asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Go { id: String, anchor: Option<String> },
    Url(String),
    /// Show an image at full size
    Zoom(String),
}

/// Textures of the images (`None`: the image could not be decoded).
pub type ImageCache = HashMap<String, Option<TextureHandle>>;

pub struct Renderer<'a> {
    pub images: &'a mut ImageCache,
    pub action: Option<Action>,
    /// Anchor to scroll to (consumed when it is drawn)
    pub scroll_to: Option<String>,
    /// Size of the body text
    pub size: f32,
}

/// Section index → link of a laid-out inline run.
type Links = Vec<Option<Link>>;

fn script_size(size: f32, s: Script) -> f32 {
    match s {
        Script::Normal => size,
        _ => size * 0.72,
    }
}

impl Renderer<'_> {
    /// Lays out an inline run.
    fn layout(&self, ui: &Ui, inl: &Inline, size: f32, color: Color32, wrap: f32, strong: bool) -> (Arc<Galley>, Links) {
        let v = ui.visuals();
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap;
        let mut links = vec![];
        for s in inl {
            if s.text.is_empty() {
                continue;
            }
            let family = if s.code { FontFamily::Monospace } else { FontFamily::Proportional };
            let fsize = script_size(if s.code { size * 0.92 } else { size }, s.script);
            let color = match (&s.link, s.bold || strong) {
                (Some(_), _) => v.hyperlink_color,
                (None, true) => v.strong_text_color(),
                _ => color,
            };
            let format = TextFormat {
                font_id: FontId::new(fsize, family),
                color,
                italics: s.italic,
                background: if s.code { v.faint_bg_color } else { Color32::TRANSPARENT },
                valign: match s.script {
                    Script::Sup => Align::TOP,
                    Script::Sub => Align::BOTTOM,
                    Script::Normal => Align::BOTTOM,
                },
                line_height: Some(size * 1.42),
                ..Default::default()
            };
            job.append(&s.text, 0.0, format);
            links.push(s.link.clone());
        }
        (ui.fonts(|f| f.layout_job(job)), links)
    }

    /// The link under `pointer` (screen) of a galley painted at `pos`.
    fn link_at(galley: &Galley, links: &Links, pos: Pos2, pointer: Pos2) -> Option<Link> {
        let p = pointer - pos;
        for row in &galley.rows {
            if p.y < row.rect.min.y || p.y > row.rect.max.y {
                continue;
            }
            for g in &row.glyphs {
                if p.x >= g.pos.x && p.x <= g.max_x() {
                    return links.get(g.section_index as usize).cloned().flatten();
                }
            }
        }
        None
    }

    /// Paints a laid-out run at `pos` and handles hovering and clicks on its links.
    fn paint_galley(&mut self, ui: &mut Ui, galley: Arc<Galley>, links: &Links, rect: Rect, resp: &egui::Response) {
        let pos = rect.min;
        ui.painter().galley(pos, galley.clone(), ui.visuals().text_color());
        if !links.iter().any(|l| l.is_some()) {
            return;
        }
        let Some(pointer) = resp.hover_pos() else { return };
        let Some(link) = Self::link_at(&galley, links, pos, pointer) else { return };
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let tip = match &link {
            Link::Cite(k) => refs::find(k).map(|r| {
                let mut t = super::markup::plain(&super::markup::inline(r.full));
                if let Some(d) = r.doi {
                    t.push_str(&format!("\ndoi:{d}"));
                }
                t
            }),
            Link::Url(u) => Some(u.clone()),
            Link::Topic { id, .. } => Some(format!("→ {}", topics::title(id))),
        };
        if let Some(t) = tip {
            resp.clone().on_hover_ui_at_pointer(|ui| {
                ui.set_max_width(460.0);
                ui.label(t);
            });
        }
        if resp.clicked() {
            self.action = Some(match link {
                Link::Topic { id, anchor } => Action::Go { id, anchor },
                Link::Url(u) => Action::Url(u),
                Link::Cite(k) => Action::Go { id: "references".into(), anchor: Some(k) },
            });
        }
    }

    /// An inline run wrapped to the available width.
    fn inline(&mut self, ui: &mut Ui, inl: &Inline, size: f32, color: Color32, strong: bool) -> Rect {
        let w = ui.available_width().max(60.0);
        let (galley, links) = self.layout(ui, inl, size, color, w, strong);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, galley.size().y), Sense::click());
        self.paint_galley(ui, galley, &links, rect, &resp);
        rect
    }

    fn check_anchor(&mut self, ui: &Ui, anchor: &str, rect: Rect) {
        if self.scroll_to.as_deref() == Some(anchor) {
            ui.scroll_to_rect(rect, Some(Align::TOP));
            self.scroll_to = None;
        }
    }

    pub fn doc(&mut self, ui: &mut Ui, doc: &Doc) {
        let size = self.size;
        let text_color = ui.visuals().text_color();
        ui.spacing_mut().item_spacing.y = 6.0;
        let r = self.inline(ui, &super::markup::inline(&doc.title), size * 1.6, text_color, true);
        if self.scroll_to.as_deref() == Some("") {
            ui.scroll_to_rect(r, Some(Align::TOP));
            self.scroll_to = None;
        }
        ui.add_space(2.0);
        for b in &doc.blocks {
            self.block(ui, b);
        }
        ui.add_space(24.0);
    }

    fn block(&mut self, ui: &mut Ui, b: &Block) {
        let size = self.size;
        let text_color = ui.visuals().text_color();
        match b {
            Block::Heading { level, text, anchor } => {
                ui.add_space(if *level == 2 { 10.0 } else { 6.0 });
                let s = if *level == 2 { size * 1.3 } else { size * 1.1 };
                let r = self.inline(ui, text, s, text_color, true);
                if *level == 2 {
                    let y = r.bottom() + 2.0;
                    ui.painter().hline(r.x_range(), y, Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color));
                    ui.add_space(4.0);
                }
                self.check_anchor(ui, anchor, r);
            }
            Block::Anchor(a) => {
                let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 0.0), Sense::hover());
                self.check_anchor(ui, a, r);
            }
            Block::Para(inl) => {
                self.inline(ui, inl, size, text_color, false);
            }
            Block::List { ordered, items } => {
                for (k, it) in items.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let mark = if *ordered { format!("{}.", k + 1) } else { "•".to_string() };
                        let (r, _) = ui.allocate_exact_size(Vec2::new(22.0, size * 1.42), Sense::hover());
                        ui.painter().text(r.right_top() + Vec2::new(-6.0, 0.0), egui::Align2::RIGHT_TOP, mark, FontId::proportional(size), text_color);
                        ui.vertical(|ui| {
                            self.inline(ui, it, size, text_color, false);
                        });
                    });
                }
            }
            Block::Note(inl) => {
                let accent = ui.visuals().hyperlink_color;
                let fill = ui.visuals().faint_bg_color;
                let resp = egui::Frame::new().fill(fill).inner_margin(egui::Margin { left: 12, right: 8, top: 6, bottom: 6 }).corner_radius(4.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.inline(ui, inl, size, text_color, false);
                });
                let r = resp.response.rect;
                ui.painter().vline(r.left() + 1.5, r.y_range(), Stroke::new(3.0_f32, accent));
            }
            Block::Formula(lines) => {
                let fill = ui.visuals().extreme_bg_color;
                egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(16, 8)).corner_radius(4.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for l in lines {
                        self.inline(ui, l, size * 1.12, text_color, false);
                    }
                });
            }
            Block::Rule => {
                ui.separator();
            }
            Block::Table { head, rows } => self.table(ui, head, rows),
            Block::Image { src, caption } => self.image(ui, src, caption),
        }
    }

    fn table(&mut self, ui: &mut Ui, head: &[Inline], rows: &[Vec<Inline>]) {
        let size = self.size;
        let n = rows.iter().map(|r| r.len()).chain(std::iter::once(head.len())).max().unwrap_or(0);
        if n == 0 {
            return;
        }
        let pad = 6.0;
        let total = ui.available_width().max(100.0);
        let text_color = ui.visuals().text_color();
        // natural widths of the columns (unwrapped), then shared out so that the table fits
        let mut natural = vec![0.0f32; n];
        for (r, row) in std::iter::once(head).chain(rows.iter().map(|r| r.as_slice())).enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let (g, _) = self.layout(ui, cell, size, text_color, f32::INFINITY, r == 0 && !head.is_empty());
                natural[c] = natural[c].max(g.size().x + 2.0 * pad + 2.0);
            }
        }
        let mut width = natural.clone();
        if natural.iter().sum::<f32>() > total {
            let mut free: Vec<usize> = (0..n).collect();
            let mut left = total;
            loop {
                let share = left / free.len() as f32;
                let fits: Vec<usize> = free.iter().copied().filter(|&c| natural[c] <= share).collect();
                if fits.is_empty() {
                    for &c in &free {
                        width[c] = share.max(40.0);
                    }
                    break;
                }
                for c in fits {
                    width[c] = natural[c];
                    left -= natural[c];
                    free.retain(|&x| x != c);
                }
                if free.is_empty() {
                    break;
                }
            }
        }
        let grid = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let stripe = ui.visuals().faint_bg_color;
        let head_fill = ui.visuals().widgets.inactive.weak_bg_fill;
        let all_rows: Vec<(bool, &[Inline])> = if head.is_empty() { vec![] } else { vec![(true, head)] }.into_iter().chain(rows.iter().map(|r| (false, r.as_slice()))).collect();
        ui.spacing_mut().item_spacing.y = 0.0;
        for (k, (is_head, row)) in all_rows.iter().enumerate() {
            let cells: Vec<(Arc<Galley>, Links)> = (0..n)
                .map(|c| {
                    let empty = vec![];
                    let cell = row.get(c).unwrap_or(&empty);
                    self.layout(ui, cell, size, text_color, width[c] - 2.0 * pad, *is_head)
                })
                .collect();
            let h = cells.iter().map(|(g, _)| g.size().y).fold(0.0, f32::max) + 2.0 * pad;
            let w: f32 = width.iter().sum();
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
            let fill = if *is_head {
                head_fill
            } else if k % 2 == 0 {
                stripe
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 0.0, fill);
            let mut x = rect.left();
            for (c, (g, links)) in cells.into_iter().enumerate() {
                let cell_rect = Rect::from_min_size(Pos2::new(x + pad, rect.top() + pad), g.size());
                self.paint_galley(ui, g, &links, cell_rect, &resp);
                x += width[c];
                if c + 1 < n {
                    ui.painter().vline(x, rect.y_range(), Stroke::new(0.5_f32, grid));
                }
            }
            ui.painter().hline(rect.x_range(), rect.bottom(), Stroke::new(0.5_f32, grid));
            if k == 0 {
                ui.painter().hline(rect.x_range(), rect.top(), Stroke::new(0.5_f32, grid));
            }
        }
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.add_space(6.0);
    }

    fn texture(&mut self, ui: &Ui, src: &str) -> Option<TextureHandle> {
        if !self.images.contains_key(src) {
            let tex = topics::image(src).and_then(|bytes| image::load_from_memory(bytes).ok()).map(|img| {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                ui.ctx().load_texture(format!("help:{src}"), ci, egui::TextureOptions::LINEAR)
            });
            self.images.insert(src.to_string(), tex);
        }
        self.images.get(src).cloned().flatten()
    }

    fn image(&mut self, ui: &mut Ui, src: &str, caption: &Inline) {
        let size = self.size;
        match self.texture(ui, src) {
            Some(tex) => {
                let px = tex.size_vec2();
                // screenshots are taken at 1.15 points per pixel; never wider than the page
                let w = (px.x / 1.15).min(ui.available_width());
                let sz = Vec2::new(w, px.y * w / px.x);
                let resp = ui.add(egui::Image::new(&tex).fit_to_exact_size(sz).sense(Sense::click()));
                ui.painter().rect_stroke(resp.rect, 2.0, Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Outside);
                if resp.on_hover_text("Click to show the image at full size").clicked() {
                    self.action = Some(Action::Zoom(src.to_string()));
                }
            }
            None => {
                ui.colored_label(ui.visuals().warn_fg_color, format!("[image {src}]"));
            }
        }
        let weak = ui.visuals().weak_text_color();
        let mut cap = caption.clone();
        for s in cap.iter_mut() {
            s.italic = true;
        }
        self.inline(ui, &cap, size * 0.92, weak, false);
        ui.add_space(4.0);
    }

    /// Full-size view of an image.
    pub fn zoomed(&mut self, ui: &mut Ui, src: &str) {
        if let Some(tex) = self.texture(ui, src) {
            let px = tex.size_vec2() / ui.ctx().pixels_per_point();
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::Image::new(&tex).fit_to_exact_size(px));
            });
        }
    }
}
