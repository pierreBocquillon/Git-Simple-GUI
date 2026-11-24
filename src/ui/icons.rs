//! Small vector icons drawn with lines and circles, so they stay crisp at any size.

use eframe::egui::{epaint::PathStroke, pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Pull,
    Push,
    Fetch,
    Branch,
    Stash,
    Pop,
    Undo,
    Redo,
    Refresh,
    Folder,
    Plus,
    Gear,
    Cloud,
    Laptop,
    Tag,
    Check,
    Search,
    Close,
    ChevronDown,
    ChevronRight,
    Commit,
    Copy,
    Remote,
    Box,
    Warning,
    Merge,
    Clone,
    Init,
    File,
}

fn pl(points: Vec<Pos2>, stroke: Stroke) -> Shape {
    Shape::line(points, PathStroke::new(stroke.width, stroke.color))
}

/// Draws `icon` inside `rect` (best square) with `color`.
pub fn draw(p: &Painter, icon: Icon, rect: Rect, color: Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let u = s / 16.0; // design grid: 16x16
    let at = |x: f32, y: f32| pos2(c.x + (x - 8.0) * u, c.y + (y - 8.0) * u);
    let w = (2.0 * u).max(1.5);
    let st = Stroke::new(w, color);

    match icon {
        Icon::Pull => {
            p.line_segment([at(8.0, 2.0), at(8.0, 11.0)], st);
            p.add(pl(vec![at(4.5, 7.5), at(8.0, 11.0), at(11.5, 7.5)], st));
            p.line_segment([at(3.0, 14.0), at(13.0, 14.0)], st);
        }
        Icon::Push => {
            p.line_segment([at(8.0, 14.0), at(8.0, 5.0)], st);
            p.add(pl(vec![at(4.5, 8.5), at(8.0, 5.0), at(11.5, 8.5)], st));
            p.line_segment([at(3.0, 2.0), at(13.0, 2.0)], st);
        }
        Icon::Fetch => {
            arc(
                p,
                at(8.0, 8.0),
                5.5 * u,
                200f32.to_radians(),
                470f32.to_radians(),
                st,
            );
            p.add(pl(vec![at(2.6, 3.8), at(2.7, 7.4), at(6.2, 6.9)], st));
        }
        Icon::Refresh => {
            arc(
                p,
                at(8.0, 8.0),
                5.5 * u,
                -60f32.to_radians(),
                230f32.to_radians(),
                st,
            );
            p.add(pl(vec![at(13.6, 2.4), at(13.0, 5.9), at(9.6, 5.4)], st));
        }
        Icon::Branch => {
            p.line_segment([at(5.0, 4.5), at(5.0, 11.5)], st);
            p.add(pl(
                bez(at(11.0, 6.5), at(11.0, 9.5), at(5.0, 8.5), at(5.0, 11.0)),
                st,
            ));
            p.circle_stroke(at(5.0, 3.0), 1.7 * u, st);
            p.circle_stroke(at(5.0, 13.0), 1.7 * u, st);
            p.circle_stroke(at(11.0, 5.0), 1.7 * u, st);
        }
        Icon::Merge => {
            p.line_segment([at(5.0, 4.5), at(5.0, 11.5)], st);
            p.add(pl(
                bez(at(5.0, 5.0), at(5.0, 9.0), at(11.0, 8.0), at(11.0, 11.5)),
                st,
            ));
            p.circle_stroke(at(5.0, 3.0), 1.7 * u, st);
            p.circle_stroke(at(5.0, 13.0), 1.7 * u, st);
            p.circle_stroke(at(11.0, 13.0), 1.7 * u, st);
        }
        Icon::Stash | Icon::Pop | Icon::Box => {
            p.add(pl(
                vec![at(2.5, 8.0), at(2.5, 13.5), at(13.5, 13.5), at(13.5, 8.0)],
                st,
            ));
            p.line_segment([at(2.5, 8.0), at(5.5, 8.0)], st);
            p.line_segment([at(10.5, 8.0), at(13.5, 8.0)], st);
            p.add(pl(
                vec![at(5.5, 8.0), at(6.5, 10.0), at(9.5, 10.0), at(10.5, 8.0)],
                st,
            ));
            if icon == Icon::Stash {
                p.line_segment([at(8.0, 1.5), at(8.0, 7.0)], st);
                p.add(pl(vec![at(5.8, 4.8), at(8.0, 7.0), at(10.2, 4.8)], st));
            } else if icon == Icon::Pop {
                p.line_segment([at(8.0, 7.0), at(8.0, 1.5)], st);
                p.add(pl(vec![at(5.8, 3.7), at(8.0, 1.5), at(10.2, 3.7)], st));
            }
        }
        Icon::Undo | Icon::Redo => {
            let flip = if icon == Icon::Redo { -1.0 } else { 1.0 };
            let f = |x: f32, y: f32| at(8.0 + (x - 8.0) * flip, y);
            let mut pts = vec![f(3.0, 6.0)];
            pts.extend(
                bez(f(3.0, 6.0), f(9.5, 6.0), f(14.0, 7.0), f(13.0, 11.0))
                    .into_iter()
                    .skip(1),
            );
            pts.extend(
                bez(f(13.0, 11.0), f(12.4, 13.0), f(10.5, 13.5), f(9.0, 13.5))
                    .into_iter()
                    .skip(1),
            );
            p.add(pl(pts, st));
            p.add(pl(vec![f(6.0, 3.0), f(3.0, 6.0), f(6.0, 9.0)], st));
        }
        Icon::Folder => {
            p.add(pl(
                vec![
                    at(2.0, 4.0),
                    at(6.0, 4.0),
                    at(7.5, 5.5),
                    at(14.0, 5.5),
                    at(14.0, 13.0),
                    at(2.0, 13.0),
                    at(2.0, 4.0),
                ],
                st,
            ));
            p.line_segment([at(2.0, 7.5), at(14.0, 7.5)], st);
        }
        Icon::Plus => {
            p.line_segment([at(8.0, 3.0), at(8.0, 13.0)], st);
            p.line_segment([at(3.0, 8.0), at(13.0, 8.0)], st);
        }
        Icon::Close => {
            p.line_segment([at(4.0, 4.0), at(12.0, 12.0)], st);
            p.line_segment([at(12.0, 4.0), at(4.0, 12.0)], st);
        }
        Icon::Check => {
            p.add(pl(vec![at(3.0, 8.5), at(6.5, 12.0), at(13.0, 4.5)], st));
        }
        Icon::ChevronDown => {
            p.add(pl(vec![at(4.0, 6.0), at(8.0, 10.0), at(12.0, 6.0)], st));
        }
        Icon::ChevronRight => {
            p.add(pl(vec![at(6.0, 4.0), at(10.0, 8.0), at(6.0, 12.0)], st));
        }
        Icon::Gear => {
            p.circle_stroke(at(8.0, 8.0), 2.2 * u, st);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment(
                    [at(8.0, 8.0) + d * 4.3 * u, at(8.0, 8.0) + d * 6.3 * u],
                    Stroke::new(w * 1.4, color),
                );
            }
            p.circle_stroke(at(8.0, 8.0), 4.4 * u, st);
        }
        Icon::Cloud => {
            let mut pts = Vec::new();
            pts.extend(bez(
                at(4.5, 12.5),
                at(1.0, 12.5),
                at(1.5, 7.5),
                at(4.8, 7.8),
            ));
            pts.extend(
                bez(at(4.8, 7.8), at(5.0, 3.0), at(11.5, 2.5), at(11.8, 7.0))
                    .into_iter()
                    .skip(1),
            );
            pts.extend(
                bez(at(11.8, 7.0), at(15.5, 7.0), at(15.5, 12.5), at(12.0, 12.5))
                    .into_iter()
                    .skip(1),
            );
            pts.push(at(4.5, 12.5));
            p.add(pl(pts, st));
        }
        Icon::Remote => {
            p.circle_stroke(at(8.0, 8.0), 6.0 * u, st);
            p.add(pl(
                bez(at(8.0, 2.0), at(4.5, 5.0), at(4.5, 11.0), at(8.0, 14.0)),
                st,
            ));
            p.add(pl(
                bez(at(8.0, 2.0), at(11.5, 5.0), at(11.5, 11.0), at(8.0, 14.0)),
                st,
            ));
            p.line_segment([at(2.2, 8.0), at(13.8, 8.0)], st);
        }
        Icon::Laptop => {
            p.add(pl(
                vec![at(3.0, 11.0), at(3.0, 3.5), at(13.0, 3.5), at(13.0, 11.0)],
                st,
            ));
            p.add(pl(
                vec![
                    at(1.0, 11.0),
                    at(15.0, 11.0),
                    at(14.0, 13.0),
                    at(2.0, 13.0),
                    at(1.0, 11.0),
                ],
                st,
            ));
        }
        Icon::Tag => {
            p.add(pl(
                vec![
                    at(2.0, 2.0),
                    at(8.5, 2.0),
                    at(14.0, 7.5),
                    at(7.5, 14.0),
                    at(2.0, 8.5),
                    at(2.0, 2.0),
                ],
                st,
            ));
            p.circle_filled(at(5.3, 5.3), 1.3 * u, color);
        }
        Icon::Search => {
            p.circle_stroke(at(7.0, 7.0), 4.5 * u, st);
            p.line_segment(
                [at(10.3, 10.3), at(14.0, 14.0)],
                Stroke::new(w * 1.2, color),
            );
        }
        Icon::Commit => {
            p.circle_stroke(at(8.0, 8.0), 3.2 * u, st);
            p.line_segment([at(1.0, 8.0), at(4.8, 8.0)], st);
            p.line_segment([at(11.2, 8.0), at(15.0, 8.0)], st);
        }
        Icon::Copy => {
            p.rect_stroke(
                Rect::from_min_max(at(5.5, 5.5), at(13.5, 14.0)),
                1.5 * u,
                st,
                eframe::egui::StrokeKind::Middle,
            );
            p.add(pl(
                vec![
                    at(10.5, 3.5),
                    at(10.5, 2.0),
                    at(2.5, 2.0),
                    at(2.5, 10.5),
                    at(4.0, 10.5),
                ],
                st,
            ));
        }
        Icon::Warning => {
            p.add(pl(
                vec![at(8.0, 2.0), at(14.5, 13.5), at(1.5, 13.5), at(8.0, 2.0)],
                st,
            ));
            p.line_segment([at(8.0, 6.0), at(8.0, 9.5)], st);
            p.circle_filled(at(8.0, 11.6), 0.9 * u, color);
        }
        Icon::Clone => {
            p.circle_stroke(at(4.0, 4.0), 1.8 * u, st);
            p.circle_stroke(at(12.0, 4.0), 1.8 * u, st);
            p.line_segment([at(4.0, 6.0), at(4.0, 9.0)], st);
            p.line_segment([at(12.0, 6.0), at(12.0, 9.0)], st);
            p.add(pl(vec![at(4.0, 9.0), at(8.0, 11.0), at(12.0, 9.0)], st));
            p.line_segment([at(8.0, 11.0), at(8.0, 15.0)], st);
        }
        Icon::Init => {
            p.add(pl(
                vec![
                    at(3.0, 2.0),
                    at(10.0, 2.0),
                    at(13.0, 5.0),
                    at(13.0, 14.0),
                    at(3.0, 14.0),
                    at(3.0, 2.0),
                ],
                st,
            ));
            p.line_segment([at(8.0, 6.5), at(8.0, 11.5)], st);
            p.line_segment([at(5.5, 9.0), at(10.5, 9.0)], st);
        }
        Icon::File => {
            p.add(pl(
                vec![
                    at(3.5, 1.5),
                    at(9.5, 1.5),
                    at(12.5, 4.5),
                    at(12.5, 14.5),
                    at(3.5, 14.5),
                    at(3.5, 1.5),
                ],
                st,
            ));
            p.add(pl(vec![at(9.5, 1.5), at(9.5, 4.5), at(12.5, 4.5)], st));
        }
    }
}

fn arc(p: &Painter, center: Pos2, r: f32, a0: f32, a1: f32, st: Stroke) {
    let n = 24;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            center + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    p.add(pl(pts, st));
}

/// Points along a cubic Bezier curve.
pub fn bez(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2) -> Vec<Pos2> {
    let n = 14;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let mt = 1.0 - t;
            let x = mt * mt * mt * p0.x
                + 3.0 * mt * mt * t * p1.x
                + 3.0 * mt * t * t * p2.x
                + t * t * t * p3.x;
            let y = mt * mt * mt * p0.y
                + 3.0 * mt * mt * t * p1.y
                + 3.0 * mt * t * t * p2.y
                + t * t * t * p3.y;
            pos2(x, y)
        })
        .collect()
}
