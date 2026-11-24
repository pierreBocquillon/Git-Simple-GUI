//! Our own window frame, replacing the Windows title bar (the window has no decorations):
//! - `drag_area`     the empty part of the top bar moves the window; double-click maximizes
//! - `controls`      minimize, maximize/restore and close buttons at the top right
//! - `resize_edges`  the window edges resize it

use crate::ui::theme::*;
use eframe::egui::{
    self, pos2, vec2, Color32, Context, CursorIcon, Painter, Pos2, Rect, ResizeDirection, Sense,
    Stroke, Ui, ViewportCommand,
};

/// Width of one window button.
const BUTTON_W: f32 = 46.0;
/// How close to the window edge the pointer must be to resize.
const EDGE: f32 = 5.0;
/// Windows' red for the close button.
const CLOSE_RED: Color32 = Color32::from_rgb(0xc4, 0x2b, 0x1c);

/// Make `rect` move the window when dragged and maximize it on double-click.
/// Call it before drawing the bar's buttons, so they stay clickable on top of it.
pub fn drag_area(ui: &mut Ui, rect: Rect) {
    let resp = ui.interact(rect, ui.id().with("window_drag"), Sense::click_and_drag());
    if resp.double_clicked() {
        toggle_maximized(ui.ctx());
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// Minimize, maximize/restore and close, right-aligned in `bar`. Returns their left edge.
pub fn controls(ui: &mut Ui, bar: Rect) -> f32 {
    let ctx = ui.ctx().clone();
    let maximized = is_maximized(&ctx);
    let mut right = bar.right();
    let mut button = |kind: Button| {
        let rect = Rect::from_min_max(pos2(right - BUTTON_W, bar.top()), pos2(right, bar.bottom()));
        right -= BUTTON_W;
        let resp = ui.interact(
            rect,
            ui.id().with(("window_button", kind as u8)),
            Sense::click(),
        );
        let p = ui.painter();
        let color = if resp.hovered() {
            let fill = if kind == Button::Close {
                CLOSE_RED
            } else {
                BG2
            };
            p.rect_filled(rect, 0.0, fill);
            TEXT
        } else {
            TEXT_DIM
        };
        draw_symbol(p, rect.center(), kind, maximized, color);
        let tip = match kind {
            Button::Close => tr!("window.close"),
            Button::Maximize if maximized => tr!("window.restore"),
            Button::Maximize => tr!("window.maximize"),
            Button::Minimize => tr!("window.minimize"),
        };
        resp.on_hover_text(tip).clicked()
    };
    if button(Button::Close) {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
    if button(Button::Maximize) {
        toggle_maximized(&ctx);
    }
    if button(Button::Minimize) {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
    right
}

/// Resize the window from its edges and corners (not while maximized).
/// Call it last in the frame so its cursor wins.
pub fn resize_edges(ctx: &Context) {
    if is_maximized(ctx) {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let Some(dir) = edge_at(ctx.screen_rect(), pos) else {
        return;
    };
    ctx.set_cursor_icon(match dir {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Button {
    Minimize,
    Maximize,
    Close,
}

fn is_maximized(ctx: &Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

fn toggle_maximized(ctx: &Context) {
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized(ctx)));
}

/// Which edge or corner of `screen` the pointer is on, if any.
fn edge_at(screen: Rect, pos: Pos2) -> Option<ResizeDirection> {
    let left = pos.x < screen.left() + EDGE;
    let right = pos.x > screen.right() - EDGE;
    let top = pos.y < screen.top() + EDGE;
    let bottom = pos.y > screen.bottom() - EDGE;
    Some(match (left, right, top, bottom) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::West,
        (_, true, ..) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        (_, _, _, true) => ResizeDirection::South,
        _ => return None,
    })
}

/// The thin line symbols of the window buttons, 10 px wide like Windows'.
fn draw_symbol(p: &Painter, c: Pos2, kind: Button, maximized: bool, color: Color32) {
    let s = Stroke::new(1.0, color);
    let h = 5.0;
    match kind {
        Button::Minimize => {
            p.line_segment([c + vec2(-h, 0.0), c + vec2(h, 0.0)], s);
        }
        Button::Maximize if maximized => {
            // Two overlapping squares.
            let back = Rect::from_min_size(c + vec2(-h + 2.0, -h), vec2(8.0, 8.0));
            p.line_segment([back.left_top(), back.right_top()], s);
            p.line_segment([back.right_top(), back.right_bottom()], s);
            let front = Rect::from_min_size(c + vec2(-h, -h + 2.0), vec2(8.0, 8.0));
            p.rect_stroke(front, 0.0, s, egui::StrokeKind::Inside);
        }
        Button::Maximize => {
            let r = Rect::from_center_size(c, vec2(2.0 * h, 2.0 * h));
            p.rect_stroke(r, 0.0, s, egui::StrokeKind::Inside);
        }
        Button::Close => {
            p.line_segment([c + vec2(-h, -h), c + vec2(h, h)], s);
            p.line_segment([c + vec2(-h, h), c + vec2(h, -h)], s);
        }
    }
}
