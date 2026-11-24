//! The bottom bar: busy/ready state, repository path, upstream, commit count and git version.

use crate::app::RepoTab;
use crate::ui::paint::text_fit;
use crate::ui::theme::{self, *};
use crate::ui::widgets::badge;
use eframe::egui::{self, pos2, vec2, Align2, Context, Frame, Stroke};

pub fn show(ctx: &Context, tab: Option<&RepoTab>, git_version: Option<&str>) {
    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(26.0)
        .frame(Frame::new().fill(BG2))
        .show(ctx, |ui| {
            let full = ui.max_rect();
            let p = ui.painter();
            p.line_segment(
                [full.left_top(), full.right_top()],
                Stroke::new(1.0, BORDER),
            );
            let y = full.center().y;
            let mut x = full.left() + 14.0;
            match tab.and_then(|t| t.busy.clone()) {
                Some(b) => {
                    let a = ui.input(|i| i.time) as f32 * 6.0;
                    let c = pos2(x + 4.0, y);
                    p.circle_stroke(c, 5.0, Stroke::new(1.5, alpha(CYAN, 0.25)));
                    p.line_segment(
                        [
                            c + vec2(a.cos(), a.sin()) * 5.0,
                            c + vec2((a + 1.3).cos(), (a + 1.3).sin()) * 5.0,
                        ],
                        Stroke::new(2.0, CYAN),
                    );
                    x += 16.0;
                    x += p
                        .text(
                            pos2(x, y),
                            Align2::LEFT_CENTER,
                            format!("{b}…"),
                            theme::semibold(11.5),
                            CYAN,
                        )
                        .width()
                        + 18.0;
                }
                None => {
                    p.circle_filled(pos2(x + 4.0, y), 3.5, ADDED);
                    x += 14.0;
                    x += p
                        .text(
                            pos2(x, y),
                            Align2::LEFT_CENTER,
                            tr!("status.ready"),
                            theme::semibold(11.5),
                            TEXT_DIM,
                        )
                        .width()
                        + 18.0;
                }
            }
            if let Some(t) = tab {
                text_fit(
                    p,
                    x,
                    y,
                    &t.path.display().to_string(),
                    theme::regular(11.5),
                    TEXT_FAINT,
                    full.width() * 0.45,
                );
            }
            let mut rx = full.right() - 14.0;
            let put = |p: &egui::Painter, rx: &mut f32, s: String| {
                *rx -= p
                    .text(
                        pos2(*rx, y),
                        Align2::RIGHT_CENTER,
                        s,
                        theme::regular(11.5),
                        TEXT_FAINT,
                    )
                    .width()
                    + 14.0;
            };
            put(
                p,
                &mut rx,
                match git_version {
                    Some(v) => format!("git {v}"),
                    None => tr!("status.git_missing").into(),
                },
            );
            if let Some(d) = tab.and_then(|t| t.data.as_ref()) {
                put(
                    p,
                    &mut rx,
                    format!(
                        "{}   ·",
                        tr!(
                            "status.commits",
                            count = format!(
                                "{}{}",
                                d.commits.len(),
                                if d.truncated { "+" } else { "" }
                            )
                        )
                    ),
                );
                if let Some(h) = d.head_ref() {
                    if let Some(u) = &h.upstream {
                        rx = badge(p, rx, y, &format!("↓{}", h.behind), CYAN) - 4.0;
                        rx = badge(p, rx, y, &format!("↑{}", h.ahead), PINK) - 8.0;
                        put(p, &mut rx, tr!("status.tracking", upstream = u));
                    }
                }
            }
        });
}
