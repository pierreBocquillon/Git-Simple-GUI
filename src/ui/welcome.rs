//! The page shown when no repository is open: open / clone / create, and recent repositories.

use crate::app::{Action, Dialog, GitApp};
use crate::ui::icons::{self, Icon};
use crate::ui::paint::{self, text_fit};
use crate::ui::theme::{self, *};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, Context, CornerRadius, Frame, Rect, Sense, Stroke,
    StrokeKind,
};

pub fn show(app: &GitApp, ctx: &Context, acts: &mut Vec<Action>) {
    egui::CentralPanel::default()
        .frame(Frame::new().fill(BG0))
        .show(ctx, |ui| {
            let full = ui.max_rect();
            let p = ui.painter().clone();
            // Soft colored light behind the logo.
            paint::gradient_v(&p, full, BG0, theme::mix(BG0, VIOLET_DEEP, 0.08));
            let recent_n = if app.git_version.is_some() {
                app.recent.len().min(5)
            } else {
                0
            };
            let content_h = 200.0
                + 116.0
                + 128.0
                + if recent_n > 0 {
                    56.0 + recent_n as f32 * 44.0
                } else {
                    0.0
                };
            let y0 = full.top() + ((full.height() - content_h) / 2.0).max(16.0);
            let logo_c = pos2(full.center().x, y0 + 100.0);
            paint::glow(&p, logo_c + vec2(-60.0, 10.0), 120.0, VIOLET, 0.9);
            paint::glow(&p, logo_c + vec2(70.0, -20.0), 100.0, CYAN, 0.45);
            paint::glow(&p, logo_c + vec2(10.0, 60.0), 100.0, PINK, 0.45);
            let logo = Rect::from_center_size(logo_c, vec2(200.0, 200.0));
            p.image(
                app.logo.id(),
                logo,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            paint::toon_text(
                &p,
                pos2(full.center().x, logo.bottom() + 30.0),
                Align2::CENTER_CENTER,
                "Git Simple GUI",
                theme::bold(38.0),
                TEXT,
                2.5,
            );
            let bar = Rect::from_center_size(
                pos2(full.center().x, logo.bottom() + 58.0),
                vec2(120.0, 3.0),
            );
            paint::gradient_rounded(&p, bar, 1.5, PINK, CYAN);
            p.text(
                pos2(full.center().x, logo.bottom() + 82.0),
                Align2::CENTER_CENTER,
                tr!("welcome.tagline"),
                theme::regular(15.0),
                TEXT_DIM,
            );

            // Action cards.
            let cards = [
                (
                    Icon::Folder,
                    tr!("welcome.open.title"),
                    tr!("welcome.open.subtitle"),
                    VIOLET,
                ),
                (
                    Icon::Clone,
                    tr!("welcome.clone.title"),
                    tr!("welcome.clone.subtitle"),
                    CYAN,
                ),
                (
                    Icon::Init,
                    tr!("welcome.init.title"),
                    tr!("welcome.init.subtitle"),
                    PINK,
                ),
            ];
            let card_w = 220.0;
            let gap = 18.0;
            let total = card_w * 3.0 + gap * 2.0;
            let top = logo.bottom() + 116.0;
            for (k, (icon, title, sub, accent)) in cards.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(
                        full.center().x - total / 2.0 + k as f32 * (card_w + gap),
                        top,
                    ),
                    vec2(card_w, 128.0),
                );
                let resp = ui.interact(r, ui.id().with(("card", k)), Sense::click());
                let lift = if resp.hovered() { -3.0 } else { 0.0 };
                let r = r.translate(vec2(0.0, lift));
                if resp.hovered() {
                    paint::glow(&p, r.center(), 80.0, *accent, 0.5);
                }
                paint::toon_box(
                    &p,
                    r,
                    20,
                    if resp.hovered() {
                        theme::mix(BG2, *accent, 0.12)
                    } else {
                        BG2
                    },
                    if resp.hovered() { *accent } else { OUTLINE },
                    7.0 - lift,
                );
                let ic =
                    Rect::from_center_size(pos2(r.left() + 36.0, r.top() + 38.0), vec2(40.0, 40.0));
                paint::toon_box(&p, ic, 13, theme::mix(BG2, *accent, 0.35), *accent, 3.0);
                icons::draw(&p, *icon, ic.shrink(10.0), Color32::WHITE);
                p.text(
                    pos2(r.left() + 18.0, r.top() + 82.0),
                    Align2::LEFT_CENTER,
                    *title,
                    theme::semibold(15.0),
                    TEXT,
                );
                p.text(
                    pos2(r.left() + 18.0, r.top() + 104.0),
                    Align2::LEFT_CENTER,
                    *sub,
                    theme::regular(12.5),
                    TEXT_FAINT,
                );
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    match k {
                        0 => acts.push(Action::PickRepo),
                        1 => acts.push(Action::Dialog(Dialog::Clone {
                            url: String::new(),
                            parent: default_parent(),
                            name: String::new(),
                        })),
                        _ => acts.push(Action::Dialog(Dialog::Init {
                            parent: default_parent(),
                            name: String::new(),
                        })),
                    }
                }
            }

            if let Some(b) = &app.welcome_busy {
                p.text(
                    pos2(full.center().x, top + 150.0),
                    Align2::CENTER_CENTER,
                    format!("{b}…"),
                    theme::semibold(13.0),
                    CYAN,
                );
            }
            if app.git_version.is_none() {
                let r =
                    Rect::from_center_size(pos2(full.center().x, top + 170.0), vec2(total, 56.0));
                p.rect(
                    r,
                    CornerRadius::same(10),
                    alpha(WARNING, 0.12),
                    Stroke::new(1.0, alpha(WARNING, 0.5)),
                    StrokeKind::Inside,
                );
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    tr!("welcome.git_missing"),
                    theme::semibold(13.0),
                    TEXT,
                );
            }

            // Recent repositories.
            let recent_top = top + 168.0;
            if !app.recent.is_empty() && app.git_version.is_some() {
                p.text(
                    pos2(full.center().x - total / 2.0, recent_top),
                    Align2::LEFT_CENTER,
                    tr!("welcome.recent"),
                    theme::bold(11.5),
                    TEXT_FAINT,
                );
                let fit = ((full.bottom() - recent_top - 30.0) / 44.0).max(0.0) as usize;
                for (k, path) in app.recent.iter().take(fit).enumerate() {
                    let r = Rect::from_min_size(
                        pos2(
                            full.center().x - total / 2.0,
                            recent_top + 16.0 + k as f32 * 44.0,
                        ),
                        vec2(total, 40.0),
                    );
                    let resp = ui.interact(r, ui.id().with(("recent", k)), Sense::click());
                    if resp.hovered() {
                        p.rect(
                            r,
                            CornerRadius::same(9),
                            BG2,
                            Stroke::new(1.0, BORDER),
                            StrokeKind::Inside,
                        );
                    }
                    icons::draw(
                        &p,
                        Icon::Folder,
                        Rect::from_center_size(
                            pos2(r.left() + 20.0, r.center().y),
                            vec2(16.0, 16.0),
                        ),
                        VIOLET,
                    );
                    let name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let w = text_fit(
                        &p,
                        r.left() + 40.0,
                        r.center().y,
                        &name,
                        theme::semibold(14.0),
                        TEXT,
                        260.0,
                    );
                    text_fit(
                        &p,
                        r.left() + 52.0 + w,
                        r.center().y,
                        &path.display().to_string(),
                        theme::regular(12.0),
                        TEXT_FAINT,
                        r.width() - w - 100.0,
                    );
                    let x = Rect::from_center_size(
                        pos2(r.right() - 20.0, r.center().y),
                        vec2(22.0, 22.0),
                    );
                    let xr = ui.interact(x, ui.id().with(("forget", k)), Sense::click());
                    if resp.hovered() || xr.hovered() {
                        icons::draw(
                            &p,
                            Icon::Close,
                            x.shrink(5.0),
                            if xr.hovered() { REMOVED } else { TEXT_FAINT },
                        );
                    }
                    if xr.on_hover_text(tr!("welcome.forget")).clicked() {
                        acts.push(Action::ForgetRecent(path.clone()));
                    } else if resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        acts.push(Action::OpenRepo(path.clone()));
                    }
                }
            }
        });
}

pub fn default_parent() -> String {
    std::env::var("USERPROFILE")
        .map(|h| format!("{h}\\Documents"))
        .unwrap_or_default()
}
