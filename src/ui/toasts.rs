//! Notifications stacked at the bottom right after each git job.

use crate::app::GitApp;
use crate::ui::icons::{self, Icon};
use crate::ui::paint::{self, text_fit};
use crate::ui::theme::{self, *};
use eframe::egui::{self, pos2, vec2, Context, Rect, Sense};

pub fn show(app: &mut GitApp, ctx: &Context) {
    let screen = ctx.screen_rect();
    let mut remove = None;
    let mut y = screen.bottom() - 40.0;
    for (k, t) in app.toasts.iter().enumerate().rev() {
        let text: String = t.text.chars().take(600).collect();
        let w = 380.0;
        let galley =
            ctx.fonts(|f| f.layout(text.clone(), theme::regular(12.5), TEXT_DIM, w - 62.0));
        let h = 44.0
            + if text.is_empty() {
                0.0
            } else {
                galley.size().y + 6.0
            };
        let rect = Rect::from_min_size(pos2(screen.right() - w - 18.0, y - h), vec2(w, h));
        y -= h + 10.0;
        let accent = if t.ok { CYAN } else { PINK };
        egui::Area::new(egui::Id::new(("toast", k)))
            .order(egui::Order::Tooltip)
            .fixed_pos(rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(rect.size(), Sense::click());
                let p = ui.painter();
                paint::toon_box(
                    p,
                    Rect::from_min_size(r.min, r.size() - vec2(0.0, 5.0)),
                    16,
                    BG3,
                    accent,
                    5.0,
                );
                p.rect_filled(
                    Rect::from_min_size(r.min + vec2(0.0, 10.0), vec2(3.0, r.height() - 20.0)),
                    2.0,
                    accent,
                );
                let ic =
                    Rect::from_center_size(pos2(r.left() + 26.0, r.top() + 22.0), vec2(22.0, 22.0));
                p.circle_filled(ic.center(), 11.0, alpha(accent, 0.18));
                icons::draw(
                    p,
                    if t.ok { Icon::Check } else { Icon::Warning },
                    ic.shrink(5.0),
                    accent,
                );
                text_fit(
                    p,
                    r.left() + 46.0,
                    r.top() + 22.0,
                    &t.title,
                    theme::semibold(13.5),
                    TEXT,
                    w - 70.0,
                );
                if !text.is_empty() {
                    p.galley(pos2(r.left() + 46.0, r.top() + 38.0), galley, TEXT_DIM);
                }
                if resp.on_hover_text(tr!("toasts.click_to_close")).clicked() {
                    remove = Some(k);
                }
            });
    }
    if let Some(k) = remove {
        app.toasts.remove(k);
    }
}
