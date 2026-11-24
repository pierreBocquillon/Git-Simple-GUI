//! Details of the selected commit: SHA, parents, message, author and changed files.

use crate::app::{Action, RepoTab};
use crate::avatars::Avatars;
use crate::git::DiffSource;
use crate::ui::paint::{self, text_fit};
use crate::ui::theme::{self, *};
use crate::ui::widgets::{avatar, file_row, panel_header};
use eframe::egui::{self, pos2, vec2, Frame, Margin, Rect, RichText, Sense, Stroke, Ui};

pub fn show(ui: &mut Ui, tab: &mut RepoTab, avatars: &Avatars, acts: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    panel_header(ui, "COMMIT", CYAN);
    let Some(c) = tab.details.clone() else {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(tr!("commit.unavailable")).color(TEXT_FAINT))
        });
        return;
    };
    let selected_path = tab.diff.as_ref().map(|(f, _)| f.path.clone());
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin {
                    left: 16,
                    right: 16,
                    top: 14,
                    bottom: 6,
                })
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 8.0);
                    ui.horizontal_wrapped(|ui| {
                        let sha = ui.add(
                            egui::Button::new(
                                RichText::new(&c.hash[..7.min(c.hash.len())])
                                    .font(theme::mono(12.5))
                                    .color(CYAN),
                            )
                            .fill(alpha(CYAN, 0.1))
                            .stroke(Stroke::new(1.0, alpha(CYAN, 0.4))),
                        );
                        if sha.on_hover_text(tr!("commit.copy_sha")).clicked() {
                            acts.push(Action::Copy(c.hash.clone()));
                        }
                        if !c.parents.is_empty() {
                            ui.label(
                                RichText::new(if c.parents.len() > 1 {
                                    tr!("commit.parents")
                                } else {
                                    tr!("commit.parent")
                                })
                                .color(TEXT_FAINT)
                                .font(theme::regular(12.0)),
                            );
                            for p in &c.parents {
                                let b = ui.add(
                                    egui::Button::new(
                                        RichText::new(&p[..7.min(p.len())])
                                            .font(theme::mono(12.0))
                                            .color(TEXT_DIM),
                                    )
                                    .fill(BG3),
                                );
                                if b.on_hover_text(tr!("commit.go_to_parent")).clicked() {
                                    acts.push(Action::Reveal(p.clone()));
                                }
                            }
                        }
                    });
                    ui.add_space(4.0);
                    let (subject, rest) = match c.message.split_once('\n') {
                        Some((s, r)) => (s.to_string(), r.trim().to_string()),
                        None => (c.message.clone(), String::new()),
                    };
                    ui.add(
                        egui::Label::new(
                            RichText::new(subject)
                                .font(theme::semibold(16.5))
                                .color(TEXT),
                        )
                        .wrap(),
                    );
                    if !rest.is_empty() {
                        ui.add(
                            egui::Label::new(
                                RichText::new(rest)
                                    .font(theme::regular(13.0))
                                    .color(TEXT_DIM),
                            )
                            .wrap(),
                        );
                    }
                    ui.add_space(6.0);
                    // Author.
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
                    let p = ui.painter();
                    paint::toon_box(
                        p,
                        Rect::from_min_size(rect.min, rect.size() - vec2(0.0, 4.0)),
                        14,
                        BG2,
                        OUTLINE,
                        4.0,
                    );
                    let github = tab
                        .data
                        .as_ref()
                        .and_then(|d| crate::avatars::github_repo(d));
                    let picture = avatars.get(&c.email, &tab.path, github.as_deref());
                    avatar(
                        p,
                        pos2(rect.left() + 26.0, rect.center().y),
                        15.0,
                        &c.author,
                        picture.as_ref(),
                    );
                    text_fit(
                        p,
                        rect.left() + 50.0,
                        rect.center().y - 9.0,
                        &c.author,
                        theme::semibold(13.0),
                        TEXT,
                        rect.width() - 60.0,
                    );
                    text_fit(
                        p,
                        rect.left() + 50.0,
                        rect.center().y + 10.0,
                        &format!("{}  ·  {}", c.author_date, c.email),
                        theme::regular(11.5),
                        TEXT_FAINT,
                        rect.width() - 60.0,
                    );
                    if c.committer != c.author || c.commit_date != c.author_date {
                        ui.label(
                            RichText::new(tr!(
                                "commit.committed_by",
                                name = c.committer,
                                date = c.commit_date
                            ))
                            .font(theme::regular(11.5))
                            .color(TEXT_FAINT),
                        );
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(if c.files.len() == 1 {
                                tr!("commit.files_one", n = c.files.len())
                            } else {
                                tr!("commit.files_other", n = c.files.len())
                            })
                            .font(theme::semibold(12.5))
                            .color(TEXT_DIM),
                        );
                        ui.label(
                            RichText::new(format!("+{}", c.additions))
                                .font(theme::semibold(12.5))
                                .color(ADDED),
                        );
                        ui.label(
                            RichText::new(format!("−{}", c.deletions))
                                .font(theme::semibold(12.5))
                                .color(REMOVED),
                        );
                    });
                });
            let parent = c.parents.first().cloned();
            for f in &c.files {
                let sel = selected_path.as_deref() == Some(f.path.as_str());
                let (resp, _) = file_row(ui, f, sel, &[]);
                let resp = match &f.orig_path {
                    Some(o) => resp.on_hover_text(format!("{o} → {}", f.path)),
                    None => resp,
                };
                if resp.clicked() {
                    acts.push(Action::OpenDiff(
                        DiffSource::Commit {
                            hash: c.hash.clone(),
                            parent: parent.clone(),
                        },
                        f.path.clone(),
                    ));
                }
            }
            ui.add_space(16.0);
        });
}
