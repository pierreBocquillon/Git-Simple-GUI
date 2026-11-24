//! The file diff that replaces the graph when a file is opened.

use crate::app::{commands, Action, RepoTab};
use crate::git::{DiffSource, LineKind};
use crate::ui::icons::{self, Icon};
use crate::ui::paint::{self, text_fit};
use crate::ui::theme::{self, *};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui,
};

pub fn show(ui: &mut Ui, tab: &mut RepoTab, acts: &mut Vec<Action>) {
    let Some((diff, source)) = &tab.diff else {
        return;
    };
    let full = ui.available_rect_before_wrap();
    let header = Rect::from_min_size(full.min, vec2(full.width(), 48.0));
    let p = ui.painter().clone();
    p.rect_filled(full, 0.0, BG0);
    p.rect_filled(header, 0.0, BG2);
    paint::gradient_h(
        &p,
        Rect::from_min_size(
            header.left_bottom() - vec2(0.0, 2.0),
            vec2(header.width(), 2.0),
        ),
        &[PINK, VIOLET, CYAN],
    );

    let (dir, name) = match diff.path.rsplit_once('/') {
        Some((d, n)) => (format!("{d}/"), n.to_string()),
        None => (String::new(), diff.path.clone()),
    };
    icons::draw(
        &p,
        Icon::File,
        Rect::from_center_size(
            pos2(header.left() + 22.0, header.center().y),
            vec2(16.0, 16.0),
        ),
        CYAN,
    );
    let name_y = if dir.is_empty() {
        header.center().y
    } else {
        text_fit(
            &p,
            header.left() + 40.0,
            header.center().y - 8.0,
            &dir,
            theme::regular(11.5),
            TEXT_FAINT,
            header.width() - 420.0,
        );
        header.center().y + 8.0
    };
    text_fit(
        &p,
        header.left() + 40.0,
        name_y,
        &name,
        theme::semibold(14.5),
        TEXT,
        header.width() - 420.0,
    );
    let info = format!("{}   +{}  −{}", diff.title, diff.added, diff.removed);
    p.text(
        pos2(header.right() - 250.0, header.center().y),
        Align2::RIGHT_CENTER,
        info,
        theme::regular(12.0),
        TEXT_DIM,
    );

    // Header buttons.
    let mut bx = header.right() - 12.0;
    let mut button = |ui: &mut Ui, label: &str, icon: Icon, accent: Color32| -> bool {
        let w = 34.0
            + ui.fonts(|f| {
                f.layout_no_wrap(label.into(), theme::semibold(12.0), TEXT)
                    .size()
                    .x
            });
        let r = Rect::from_min_size(pos2(bx - w, header.center().y - 14.0), vec2(w, 28.0));
        bx -= w + 8.0;
        let resp = ui.interact(r, ui.id().with(("diffbtn", label)), Sense::click());
        let fill = if resp.hovered() {
            alpha(accent, 0.28)
        } else {
            alpha(accent, 0.14)
        };
        ui.painter().rect(
            r,
            CornerRadius::same(6),
            fill,
            Stroke::new(1.0, alpha(accent, 0.5)),
            StrokeKind::Inside,
        );
        icons::draw(
            ui.painter(),
            icon,
            Rect::from_center_size(pos2(r.left() + 15.0, r.center().y), vec2(13.0, 13.0)),
            accent,
        );
        ui.painter().text(
            pos2(r.left() + 27.0, r.center().y),
            Align2::LEFT_CENTER,
            label,
            theme::semibold(12.0),
            TEXT,
        );
        resp.clicked()
    };
    if button(ui, tr!("diff.close"), Icon::Close, TEXT_DIM)
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        acts.push(Action::CloseDiff);
    }
    let path = diff.path.clone();
    match source {
        DiffSource::Unstaged | DiffSource::Untracked => {
            if button(ui, tr!("diff.stage"), Icon::Plus, ADDED) {
                acts.push(Action::Run(commands::stage_job(tab, &[path.clone()])));
            }
        }
        DiffSource::Staged => {
            if button(ui, tr!("diff.unstage"), Icon::Undo, MODIFIED) {
                acts.push(Action::Run(commands::unstage_job(tab, &[path.clone()])));
            }
        }
        _ => {}
    }

    let body = Rect::from_min_max(pos2(full.left(), header.bottom()), full.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
    if diff.binary {
        child.painter().text(
            body.center(),
            Align2::CENTER_CENTER,
            tr!("diff.binary"),
            theme::semibold(14.0),
            TEXT_DIM,
        );
        return;
    }
    if diff.lines.is_empty() {
        child.painter().text(
            body.center(),
            Align2::CENTER_CENTER,
            tr!("diff.no_changes"),
            theme::semibold(14.0),
            TEXT_DIM,
        );
        return;
    }
    let line_h = 20.0;
    let font = theme::mono(12.5);
    let char_w = child.fonts(|f| f.glyph_width(&font, 'M'));
    let longest = diff
        .lines
        .iter()
        .map(|l| l.text.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let gutter = 100.0;
    let content_w = (gutter + 24.0 + longest * char_w + 40.0).max(body.width());
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    egui::ScrollArea::both()
        .id_salt(("diff", tab.id, &diff.path))
        .auto_shrink([false, false])
        .show_rows(&mut child, line_h, diff.lines.len(), |ui, range| {
            let block = Rect::from_min_size(
                ui.cursor().min,
                vec2(content_w, line_h * range.len() as f32),
            );
            ui.allocate_rect(block, Sense::hover());
            let p = ui.painter();
            let vis_left = ui.clip_rect().left();
            for (k, i) in range.clone().enumerate() {
                let l = &diff.lines[i];
                let row = Rect::from_min_size(
                    pos2(block.left(), block.top() + k as f32 * line_h),
                    vec2(content_w, line_h),
                );
                let (bg, fg, mark) = match l.kind {
                    LineKind::Added => (alpha(ADDED, 0.11), TEXT, "+"),
                    LineKind::Removed => (alpha(REMOVED, 0.12), TEXT, "−"),
                    LineKind::Hunk => (alpha(VIOLET, 0.10), theme::mix(VIOLET, TEXT, 0.4), ""),
                    LineKind::Meta => (Color32::TRANSPARENT, TEXT_FAINT, ""),
                    LineKind::Context => (Color32::TRANSPARENT, TEXT_DIM, ""),
                };
                p.rect_filled(row, 0.0, bg);
                let y = row.center().y;
                let gx = vis_left.max(block.left());
                // Line numbers stay visible when scrolling sideways.
                p.rect_filled(
                    Rect::from_min_size(pos2(gx, row.top()), vec2(gutter, line_h)),
                    0.0,
                    theme::mix(BG0, BG2, 0.6),
                );
                if let Some(n) = l.old_no {
                    p.text(
                        pos2(gx + 42.0, y),
                        Align2::RIGHT_CENTER,
                        n.to_string(),
                        font.clone(),
                        TEXT_FAINT,
                    );
                }
                if let Some(n) = l.new_no {
                    p.text(
                        pos2(gx + 90.0, y),
                        Align2::RIGHT_CENTER,
                        n.to_string(),
                        font.clone(),
                        TEXT_FAINT,
                    );
                }
                let mark_color = match l.kind {
                    LineKind::Added => ADDED,
                    LineKind::Removed => REMOVED,
                    _ => TEXT_FAINT,
                };
                if l.kind == LineKind::Added || l.kind == LineKind::Removed {
                    p.rect_filled(
                        Rect::from_min_size(pos2(gx + gutter - 3.0, row.top()), vec2(3.0, line_h)),
                        0.0,
                        mark_color,
                    );
                }
                let tx = block.left() + gutter + 10.0;
                p.text(
                    pos2(tx, y),
                    Align2::LEFT_CENTER,
                    mark,
                    font.clone(),
                    mark_color,
                );
                let text = l.text.replace('\t', "    ");
                p.with_clip_rect(Rect::from_min_max(
                    pos2(gx + gutter, row.top()),
                    pos2(row.right().max(gx + gutter), row.bottom()),
                ))
                .text(
                    pos2(tx + 14.0, y),
                    Align2::LEFT_CENTER,
                    text,
                    font.clone(),
                    fg,
                );
            }
        });
}
