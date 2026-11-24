//! Drawing helpers used everywhere: gradients, glows, "sticker" boxes, outlined titles and
//! single-line text cut with an ellipsis.

use super::theme::{alpha, INK};
use eframe::egui::{
    self, pos2, text::LayoutJob, Color32, CornerRadius, FontId, Mesh, Pos2, Rect, Stroke, Vec2,
};

/// Single-line text cut with an ellipsis to `max_w`, vertically centered on `y`.
pub fn text_fit(
    p: &egui::Painter,
    x: f32,
    y: f32,
    text: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
) -> f32 {
    if max_w <= 4.0 {
        return 0.0;
    }
    let mut job = LayoutJob::simple_singleline(text.to_string(), font, color);
    job.wrap = egui::text::TextWrapping {
        max_width: max_w,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = p.layout_job(job);
    let w = galley.size().x;
    p.galley(pos2(x, y - galley.size().y / 2.0), galley, color);
    w
}

/// Horizontal gradient through `stops` (evenly spaced).
pub fn gradient_h(painter: &egui::Painter, rect: Rect, stops: &[Color32]) {
    if stops.len() < 2 {
        painter.rect_filled(
            rect,
            0.0,
            stops.first().copied().unwrap_or(Color32::TRANSPARENT),
        );
        return;
    }
    let mut mesh = Mesh::default();
    let n = stops.len() - 1;
    for (i, c) in stops.iter().enumerate() {
        let x = rect.left() + rect.width() * i as f32 / n as f32;
        mesh.colored_vertex(Pos2::new(x, rect.top()), *c);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), *c);
    }
    for i in 0..n as u32 {
        let k = i * 2;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Vertical gradient from `top` to `bottom`.
pub fn gradient_v(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
}

/// A soft round glow (stacked translucent circles).
pub fn glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, strength: f32) {
    // Radial gradient: a fan of triangles whose rings fade smoothly to transparent.
    let rings = [
        (0.0f32, 1.0f32),
        (0.5, 0.75),
        (1.0, 0.42),
        (1.5, 0.15),
        (2.0, 0.0),
    ];
    let seg = 48u32;
    let base = (strength * 0.32).clamp(0.0, 1.0);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, alpha(color, base));
    for &(r, a) in &rings[1..] {
        for k in 0..seg {
            let ang = k as f32 / seg as f32 * std::f32::consts::TAU;
            mesh.colored_vertex(
                center + Vec2::new(ang.cos(), ang.sin()) * radius * r,
                alpha(color, base * a),
            );
        }
    }
    for k in 0..seg {
        let k1 = (k + 1) % seg;
        mesh.add_triangle(0, 1 + k, 1 + k1);
    }
    for ring in 0..(rings.len() as u32 - 2) {
        let a0 = 1 + ring * seg;
        let b0 = a0 + seg;
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            mesh.add_triangle(a0 + k, b0 + k, b0 + k1);
            mesh.add_triangle(a0 + k, b0 + k1, a0 + k1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// A rounded rectangle filled with a left-to-right gradient.
pub fn gradient_rounded(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    left: Color32,
    right: Color32,
) {
    let r = radius.min(rect.height() / 2.0).min(rect.width() / 2.0);
    let corners = [
        (Pos2::new(rect.right() - r, rect.top() + r), -90.0f32),
        (Pos2::new(rect.right() - r, rect.bottom() - r), 0.0),
        (Pos2::new(rect.left() + r, rect.bottom() - r), 90.0),
        (Pos2::new(rect.left() + r, rect.top() + r), 180.0),
    ];
    let color_at = |x: f32| {
        let t = ((x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color32::from_rgba_premultiplied(
            l(left.r(), right.r()),
            l(left.g(), right.g()),
            l(left.b(), right.b()),
            l(left.a(), right.a()),
        )
    };
    let mut mesh = Mesh::default();
    let c = rect.center();
    mesh.colored_vertex(c, color_at(c.x));
    for (center, start) in corners {
        for k in 0..=6 {
            let a = (start + 90.0 * k as f32 / 6.0).to_radians();
            let p = center + Vec2::new(a.cos(), a.sin()) * r;
            mesh.colored_vertex(p, color_at(p.x));
        }
    }
    let n = mesh.vertices.len() as u32 - 1;
    for i in 1..=n {
        let j = if i == n { 1 } else { i + 1 };
        mesh.add_triangle(0, i, j);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// A "sticker": hard ink drop shadow, flat fill and a thick outline.
pub fn toon_box(
    p: &egui::Painter,
    rect: Rect,
    radius: u8,
    fill: Color32,
    outline: Color32,
    shadow: f32,
) {
    let cr = CornerRadius::same(radius);
    // Kept light: a soft short shadow and a thin outline.
    if shadow > 0.0 {
        p.rect_filled(
            rect.translate(Vec2::new(0.0, (shadow * 0.4).max(1.0))),
            cr,
            Color32::from_black_alpha(110),
        );
    }
    p.rect(
        rect,
        cr,
        fill,
        Stroke::new(1.5, outline),
        egui::StrokeKind::Inside,
    );
}

/// Text with a thick ink outline and a drop shadow, like a cartoon title.
pub fn toon_text(
    p: &egui::Painter,
    pos: Pos2,
    anchor: egui::Align2,
    text: &str,
    font: FontId,
    color: Color32,
    outline: f32,
) {
    let o = outline;
    p.text(
        pos + Vec2::new(0.0, o * 1.8),
        anchor,
        text,
        font.clone(),
        INK,
    );
    for (dx, dy) in [
        (-o, 0.0),
        (o, 0.0),
        (0.0, -o),
        (0.0, o),
        (-o * 0.7, -o * 0.7),
        (o * 0.7, -o * 0.7),
        (-o * 0.7, o * 0.7),
        (o * 0.7, o * 0.7),
    ] {
        p.text(pos + Vec2::new(dx, dy), anchor, text, font.clone(), INK);
    }
    p.text(pos, anchor, text, font, color);
}
