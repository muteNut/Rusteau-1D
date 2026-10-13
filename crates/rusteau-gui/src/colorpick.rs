//! A colour picker in the manner of the developer tools of web browsers: a swatch that opens a panel with a
//! saturation–brightness field, sliders for the hue and the opacity, the hex code and the RGBA values.

use egui::ecolor::Hsva;
use egui::{Color32, Id, Mesh, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

/// Draws a checkerboard (the background of transparent colours).
fn checkers(ui: &Ui, rect: Rect) {
    let p = ui.painter();
    p.rect_filled(rect, 2.0, Color32::from_gray(200));
    let n = 6.0;
    let mut y = rect.top();
    let mut row = 0;
    while y < rect.bottom() {
        let mut x = rect.left() + if row % 2 == 0 { 0.0 } else { n };
        while x < rect.right() {
            let r = Rect::from_min_max(Pos2::new(x, y), Pos2::new((x + n).min(rect.right()), (y + n).min(rect.bottom())));
            p.rect_filled(r, 0.0, Color32::from_gray(140));
            x += 2.0 * n;
        }
        y += n;
        row += 1;
    }
}

fn c32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// Swatch of a colour (with the checkerboard behind transparent colours).
pub fn swatch(ui: &mut Ui, c: [u8; 4], size: Vec2) -> Response {
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    checkers(ui, rect);
    ui.painter().rect_filled(rect, 2.0, c32(c));
    let stroke = if r.hovered() { ui.visuals().widgets.hovered.fg_stroke } else { ui.visuals().widgets.noninteractive.bg_stroke };
    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0_f32, stroke.color), StrokeKind::Outside);
    r
}

/// A swatch that opens the picker; returns true when the colour changed.
pub fn color_button(ui: &mut Ui, c: &mut [u8; 4]) -> Response {
    let mut r = swatch(ui, *c, Vec2::new(28.0, 18.0)).on_hover_text("Click to choose the colour");
    let popup = r.id.with("picker");
    if r.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let mut changed = false;
    egui::popup_below_widget(ui, popup, &r, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
        ui.set_min_width(250.0);
        changed = picker(ui, popup, c);
    });
    if changed {
        r.mark_changed();
    }
    r
}

/// The picker panel. The hue and saturation are kept while the panel is open, so they survive grey and black.
pub fn picker(ui: &mut Ui, id: Id, c: &mut [u8; 4]) -> bool {
    let key = id.with("hsva");
    let mut hsva = ui.data(|d| d.get_temp::<([u8; 4], Hsva)>(key)).filter(|(rgba, _)| rgba == c).map(|(_, h)| h).unwrap_or_else(|| Hsva::from_srgba_unmultiplied(*c));
    let before = *c;
    let w = 250.0;
    // saturation (left to right) and brightness (top to bottom) of the hue
    let (rect, r) = ui.allocate_exact_size(Vec2::new(w, 140.0), Sense::click_and_drag());
    let n = 24;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (s, v) = (i as f32 / n as f32, 1.0 - j as f32 / n as f32);
            let col: Color32 = Hsva::new(hsva.h, s, v, 1.0).into();
            mesh.colored_vertex(Pos2::new(rect.left() + s * rect.width(), rect.top() + (1.0 - v) * rect.height()), col);
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let c2 = a + (n + 1) as u32;
            let d = c2 + 1;
            mesh.add_triangle(a, b, d);
            mesh.add_triangle(a, d, c2);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
    if let Some(p) = r.interact_pointer_pos() {
        hsva.s = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsva.v = 1.0 - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    }
    let mark = Pos2::new(rect.left() + hsva.s * rect.width(), rect.top() + (1.0 - hsva.v) * rect.height());
    ui.painter().circle_stroke(mark, 5.0, Stroke::new(2.0_f32, Color32::WHITE));
    ui.painter().circle_stroke(mark, 6.5, Stroke::new(1.0_f32, Color32::BLACK));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        swatch(ui, hsva.to_srgba_unmultiplied(), Vec2::new(30.0, 30.0));
        ui.vertical(|ui| {
            let sw = w - 44.0;
            // hue
            let (hr, resp) = ui.allocate_exact_size(Vec2::new(sw, 12.0), Sense::click_and_drag());
            let mut m = Mesh::default();
            let k = 36;
            for i in 0..=k {
                let t = i as f32 / k as f32;
                let col: Color32 = Hsva::new(t, 1.0, 1.0, 1.0).into();
                m.colored_vertex(Pos2::new(hr.left() + t * hr.width(), hr.top()), col);
                m.colored_vertex(Pos2::new(hr.left() + t * hr.width(), hr.bottom()), col);
            }
            for i in 0..k as u32 {
                m.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
                m.add_triangle(2 * i + 1, 2 * i + 3, 2 * i + 2);
            }
            ui.painter().add(Shape::mesh(m));
            if let Some(p) = resp.interact_pointer_pos() {
                hsva.h = ((p.x - hr.left()) / hr.width()).clamp(0.0, 1.0);
            }
            handle(ui, hr, hsva.h);
            ui.add_space(4.0);
            // opacity
            let (ar, resp) = ui.allocate_exact_size(Vec2::new(sw, 12.0), Sense::click_and_drag());
            checkers(ui, ar);
            let opaque: Color32 = Hsva { a: 1.0, ..hsva }.into();
            let mut m = Mesh::default();
            m.colored_vertex(ar.left_top(), Color32::TRANSPARENT);
            m.colored_vertex(ar.left_bottom(), Color32::TRANSPARENT);
            m.colored_vertex(ar.right_top(), opaque);
            m.colored_vertex(ar.right_bottom(), opaque);
            m.add_triangle(0, 1, 2);
            m.add_triangle(1, 3, 2);
            ui.painter().add(Shape::mesh(m));
            if let Some(p) = resp.interact_pointer_pos() {
                hsva.a = ((p.x - ar.left()) / ar.width()).clamp(0.0, 1.0);
            }
            handle(ui, ar, hsva.a);
        });
    });
    let mut rgba = hsva.to_srgba_unmultiplied();
    ui.add_space(6.0);
    // hex code and RGBA values
    let hex_key = id.with("hex");
    let mut hex = ui.data(|d| d.get_temp::<String>(hex_key)).unwrap_or_else(|| rusteau_core::figure::Color(rgba).hex());
    let mut typed = false;
    ui.horizontal(|ui| {
        ui.label("Hex");
        let r = ui.add(egui::TextEdit::singleline(&mut hex).desired_width(90.0).char_limit(9).font(egui::TextStyle::Monospace));
        if r.changed() {
            hex.retain(|ch| ch == '#' || ch.is_ascii_hexdigit());
            if let Some(c) = rusteau_core::figure::Color::parse(&hex) {
                rgba = c.0;
                typed = true;
            }
        }
        if !r.has_focus() {
            hex = rusteau_core::figure::Color(rgba).hex();
        }
    });
    let mut dragged = false;
    ui.horizontal(|ui| {
        for (k, name) in ["R", "G", "B", "A"].iter().enumerate() {
            ui.label(*name);
            dragged |= crate::util::int(ui.add(egui::DragValue::new(&mut rgba[k]).range(0..=255).speed(1.0))).changed();
        }
    });
    if typed || dragged {
        hsva = Hsva::from_srgba_unmultiplied(rgba);
    } else {
        rgba = hsva.to_srgba_unmultiplied();
    }
    *c = rgba;
    ui.data_mut(|d| {
        d.insert_temp(key, (*c, hsva));
        d.insert_temp(hex_key, hex);
    });
    *c != before
}

fn handle(ui: &Ui, r: Rect, t: f32) {
    let x = r.left() + t * r.width();
    let h = Rect::from_center_size(Pos2::new(x, r.center().y), Vec2::new(5.0, r.height() + 4.0));
    ui.painter().rect_filled(h, 1.5, Color32::WHITE);
    ui.painter().rect_stroke(h, 1.5, Stroke::new(1.0_f32, Color32::BLACK), StrokeKind::Outside);
}
