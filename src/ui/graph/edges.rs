//! The lines between commit dots: straight runs joined by rounded elbows.

use super::{CORNER_R, EDGE_W};
use crate::graph::EdgeKind;
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Shape, Stroke};

pub fn circle_points(c: Pos2, r: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Rounded elbows: straight runs joined by a quarter circle of radius 8.
pub fn draw_edge(
    p: &egui::Painter,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    kind: EdgeKind,
    color: Color32,
) {
    let st = Stroke::new(EDGE_W, color);
    // Round ends hide the seam where one row's line meets the next.
    p.circle_filled(pos2(x0, y0), EDGE_W / 2.0, color);
    p.circle_filled(pos2(x1, y1), EDGE_W / 2.0, color);
    if (x0 - x1).abs() < 0.5 {
        p.line_segment([pos2(x0, y0), pos2(x1, y1)], st);
        return;
    }
    let s = (x1 - x0).signum();
    let r = CORNER_R.min((x1 - x0).abs()).min((y1 - y0).abs() / 2.0);
    let mut pts = Vec::new();
    match kind {
        EdgeKind::MergeIn => {
            // Down its own lane, then sideways into the next dot.
            pts.push(pos2(x0, y0));
            pts.push(pos2(x0, y1 - r));
            let c = pos2(x0 + s * r, y1 - r);
            for k in 1..=8 {
                let a = k as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                pts.push(pos2(c.x - s * r * a.cos(), c.y + r * a.sin()));
            }
            pts.push(pos2(x1, y1));
        }
        _ => {
            // Sideways out of the dot, then down the new lane.
            pts.push(pos2(x0, y0));
            pts.push(pos2(x1 - s * r, y0));
            let c = pos2(x1 - s * r, y0 + r);
            for k in 1..=8 {
                let a = k as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                pts.push(pos2(c.x + s * r * a.sin(), c.y - r * a.cos()));
            }
            pts.push(pos2(x1, y1));
        }
    }
    p.add(Shape::line(pts, st));
}
