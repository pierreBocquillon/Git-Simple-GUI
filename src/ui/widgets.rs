//! Reusable widgets: buttons, badges, list rows and panel headers.

use crate::git::{self, FileEntry};
use crate::ui::icons::{self, Icon};
use crate::ui::paint::{self, text_fit};
use crate::ui::theme::{self, *};
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Pos2, Rect, Response, RichText,
    Sense, Stroke, StrokeKind, TextureHandle, Ui,
};

/// The big violet-to-pink button.
pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool, width: f32, height: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        vec2(width, height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = ui.painter();
    if enabled {
        // Chunky cartoon button: a darker "lip" under it that it presses into.
        let down = resp.is_pointer_button_down_on();
        let lip = 3.0;
        let body = Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() - lip))
            .translate(vec2(0.0, if down { lip - 1.0 } else { 0.0 }));
        let base = Rect::from_min_size(
            rect.min + vec2(0.0, lip),
            vec2(rect.width(), rect.height() - lip),
        );
        p.rect_filled(
            base.expand(1.0),
            CornerRadius::same(13),
            Color32::from_black_alpha(120),
        );
        paint::gradient_rounded(
            p,
            base,
            12.0,
            theme::mix(VIOLET_DEEP, INK, 0.45),
            theme::mix(PINK, INK, 0.45),
        );

        let (l, r) = if resp.hovered() {
            (
                theme::mix(VIOLET_DEEP, Color32::WHITE, 0.12),
                theme::mix(PINK, Color32::WHITE, 0.12),
            )
        } else {
            (VIOLET_DEEP, PINK)
        };
        paint::gradient_rounded(p, body, 12.0, l, r);
        p.text(
            body.center() + vec2(0.0, 1.5),
            Align2::CENTER_CENTER,
            text,
            theme::bold(14.0),
            alpha(INK, 0.6),
        );
        p.text(
            body.center(),
            Align2::CENTER_CENTER,
            text,
            theme::bold(14.0),
            Color32::WHITE,
        );
    } else {
        paint::toon_box(
            p,
            Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() - 4.0)),
            12,
            BG3,
            OUTLINE,
            4.0,
        );
        p.text(
            rect.center() - vec2(0.0, 2.0),
            Align2::CENTER_CENTER,
            text,
            theme::bold(13.5),
            TEXT_FAINT,
        );
    }
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub fn secondary_button(ui: &mut Ui, text: &str, icon: Option<Icon>, accent: Color32) -> Response {
    let font = theme::bold(12.5);
    let tw = ui.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), TEXT)
            .size()
            .x
    });
    let w = tw + if icon.is_some() { 40.0 } else { 26.0 };
    let (outer, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let p = ui.painter();
    let down = resp.is_pointer_button_down_on();
    let rect = Rect::from_min_size(outer.min, vec2(w, 29.0))
        .translate(vec2(0.0, if down { 2.0 } else { 0.0 }));
    let fill = theme::mix(BG2, accent, if resp.hovered() { 0.26 } else { 0.12 });
    p.rect(
        rect,
        CornerRadius::same(12),
        fill,
        Stroke::new(1.5, alpha(accent, 0.85)),
        StrokeKind::Inside,
    );
    let mut x = rect.left() + 12.0;
    if let Some(i) = icon {
        icons::draw(
            p,
            i,
            Rect::from_center_size(pos2(x + 6.0, rect.center().y), vec2(13.0, 13.0)),
            accent,
        );
        x += 18.0;
    }
    p.text(
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        TEXT,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let color = if resp.hovered() { TEXT } else { TEXT_DIM };
    if resp.hovered() {
        paint::toon_box(ui.painter(), rect.shrink(1.0), 10, BG3, VIOLET, 2.0);
    }
    icons::draw(ui.painter(), icon, rect.shrink(size * 0.26), color);
    resp.on_hover_text(tip)
}

pub fn status_color(code: char) -> Color32 {
    match code {
        'A' | '?' => ADDED,
        'D' => REMOVED,
        'R' | 'C' => RENAMED,
        'U' => WARNING,
        _ => MODIFIED,
    }
}

pub fn status_badge(p: &egui::Painter, center: Pos2, code: char) {
    let c = status_color(code);
    let r = Rect::from_center_size(center, vec2(18.0, 18.0));
    p.rect(
        r,
        CornerRadius::same(6),
        theme::mix(BG1, c, 0.18),
        Stroke::new(1.0, alpha(c, 0.8)),
        StrokeKind::Inside,
    );
    let letter = match code {
        '?' => 'A',
        'T' => 'M',
        other => other,
    };
    p.text(
        center,
        Align2::CENTER_CENTER,
        letter.to_string(),
        theme::bold(10.5),
        c,
    );
}

/// A round author picture: the profile picture when there is one, else the initials.
pub fn avatar(
    p: &egui::Painter,
    center: Pos2,
    r: f32,
    name: &str,
    picture: Option<&TextureHandle>,
) {
    let rect = Rect::from_center_size(center, vec2(r * 2.0, r * 2.0));
    if let Some(t) = picture {
        picture_circle(p, center, r, t, Color32::WHITE);
        return;
    }
    paint::gradient_rounded(p, rect, r, VIOLET_DEEP, PINK);
    p.text(
        center,
        Align2::CENTER_CENTER,
        git::initials(name),
        theme::bold(r * 0.8),
        Color32::WHITE,
    );
}

/// Draw a texture clipped to a circle, multiplied by `tint`.
pub fn picture_circle(p: &egui::Painter, center: Pos2, r: f32, t: &TextureHandle, tint: Color32) {
    let rect = Rect::from_center_size(center, vec2(r * 2.0, r * 2.0));
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    p.add(
        egui::epaint::RectShape::filled(rect, CornerRadius::same(r.round() as u8), tint)
            .with_texture(t.id(), uv),
    );
}

/// A small rounded count badge ending at `right`; a leading ↑ or ↓ is drawn as an arrow.
/// Returns the badge's left edge.
pub fn badge(p: &egui::Painter, right: f32, y: f32, text: &str, c: Color32) -> f32 {
    let (arrow, rest) = match text.chars().next() {
        Some('↑') => (Some(true), &text['↑'.len_utf8()..]),
        Some('↓') => (Some(false), &text['↓'.len_utf8()..]),
        _ => (None, text),
    };
    let g = p.layout_no_wrap(rest.to_string(), theme::semibold(10.5), c);
    let aw = if arrow.is_some() { 9.0 } else { 0.0 };
    let b = Rect::from_min_max(
        pos2(right - g.size().x - 10.0 - aw, y - 8.0),
        pos2(right, y + 8.0),
    );
    p.rect_filled(b, CornerRadius::same(8), alpha(c, 0.16));
    if let Some(up) = arrow {
        arrow_glyph(p, pos2(b.left() + 8.0, y), up, c);
    }
    p.galley(pos2(b.left() + 5.0 + aw, y - g.size().y / 2.0), g, c);
    b.left()
}

/// A tiny up or down arrow centered on `c`.
pub fn arrow_glyph(p: &egui::Painter, c: Pos2, up: bool, color: Color32) {
    let s = if up { -1.0 } else { 1.0 };
    let st = Stroke::new(1.5, color);
    p.line_segment([c - vec2(0.0, 4.0 * s), c + vec2(0.0, 4.0 * s)], st);
    p.line_segment([c + vec2(0.0, 4.0 * s), c + vec2(-3.0, 1.0 * s)], st);
    p.line_segment([c + vec2(0.0, 4.0 * s), c + vec2(3.0, 1.0 * s)], st);
}

pub fn hint_row(ui: &mut Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    ui.painter().text(
        pos2(rect.left() + 30.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme::regular(12.0),
        TEXT_FAINT,
    );
}

pub fn panel_header(ui: &mut Ui, kind: &str, accent: Color32) -> Rect {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 48.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, BG1);
    paint::gradient_h(
        p,
        Rect::from_min_size(rect.left_bottom() - vec2(0.0, 1.5), vec2(rect.width(), 1.5)),
        &[alpha(PINK, 0.6), alpha(VIOLET, 0.6), alpha(CYAN, 0.6)],
    );
    // A tilted-looking label sticker.
    let tw = p
        .layout_no_wrap(kind.to_string(), theme::bold(11.5), TEXT)
        .size()
        .x;
    let tag = Rect::from_min_size(
        pos2(rect.left() + 14.0, rect.center().y - 12.0),
        vec2(tw + 34.0, 24.0),
    );
    p.rect(
        tag,
        CornerRadius::same(12),
        theme::mix(BG1, accent, 0.14),
        Stroke::new(1.0, alpha(accent, 0.6)),
        StrokeKind::Inside,
    );
    p.circle_filled(pos2(tag.left() + 13.0, tag.center().y), 4.0, accent);
    p.text(
        pos2(tag.left() + 24.0, tag.center().y),
        Align2::LEFT_CENTER,
        kind,
        theme::bold(11.5),
        Color32::WHITE,
    );
    rect
}

pub fn file_row(
    ui: &mut Ui,
    f: &FileEntry,
    selected: bool,
    buttons: &[(Icon, Color32, &str)],
) -> (Response, Option<usize>) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    let p = ui.painter().clone();
    let pill = Rect::from_min_max(
        pos2(rect.left() + 6.0, rect.top() + 1.0),
        pos2(rect.right() - 6.0, rect.bottom() - 2.0),
    );
    if selected {
        p.rect(
            pill,
            CornerRadius::same(10),
            theme::mix(BG1, VIOLET, 0.2),
            Stroke::new(1.0, alpha(VIOLET, 0.7)),
            StrokeKind::Inside,
        );
    } else if resp.hovered() {
        p.rect_filled(pill, CornerRadius::same(10), BG3);
    }
    status_badge(&p, pos2(rect.left() + 22.0, rect.center().y), f.code);
    let (dir, name) = match f.path.rsplit_once('/') {
        Some((d, n)) => (format!("{d}/"), n.to_string()),
        None => (String::new(), f.path.clone()),
    };
    let show_buttons = !buttons.is_empty() && (selected || ui.rect_contains_pointer(rect));
    let right_space = if show_buttons {
        buttons.len() as f32 * 26.0 + 6.0
    } else {
        6.0
    };
    let max = rect.width() - 44.0 - right_space;
    let nw = text_fit(
        &p,
        rect.left() + 38.0,
        rect.center().y,
        &name,
        theme::regular(13.0),
        TEXT,
        max,
    );
    if !dir.is_empty() {
        text_fit(
            &p,
            rect.left() + 44.0 + nw,
            rect.center().y,
            &dir,
            theme::regular(11.5),
            TEXT_FAINT,
            max - nw - 6.0,
        );
    }
    let mut clicked_button = None;
    if show_buttons {
        for (k, (icon, color, tip)) in buttons.iter().enumerate() {
            let b = Rect::from_center_size(
                pos2(
                    rect.right() - 16.0 - 26.0 * (buttons.len() - 1 - k) as f32,
                    rect.center().y,
                ),
                vec2(22.0, 22.0),
            );
            let br = ui
                .interact(b, ui.id().with(("filebtn", &f.path, k)), Sense::click())
                .on_hover_text(*tip);
            p.rect(
                b,
                CornerRadius::same(7),
                if br.hovered() {
                    alpha(*color, 0.3)
                } else {
                    alpha(*color, 0.12)
                },
                Stroke::new(1.0, alpha(*color, 0.5)),
                StrokeKind::Inside,
            );
            icons::draw(&p, *icon, b.shrink(5.5), *color);
            if br.clicked() {
                clicked_button = Some(k);
            }
        }
    }
    (resp, clicked_button)
}

pub fn list_header(
    ui: &mut Ui,
    title: &str,
    count: usize,
    color: Color32,
    button: Option<(&str, Icon, Color32)>,
) -> bool {
    ui.add_space(8.0);
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(14.0);
        ui.label(RichText::new(title).font(theme::bold(11.5)).color(TEXT_DIM));
        ui.label(
            RichText::new(count.to_string())
                .font(theme::semibold(11.0))
                .color(color),
        );
        if let Some((label, icon, accent)) = button {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                if secondary_button(ui, label, Some(icon), accent).clicked() {
                    clicked = true;
                }
            });
        }
    });
    ui.add_space(4.0);
    clicked
}
