//! Work in progress: conflicts, unstaged and staged files, and the commit message box.

use crate::app::{args, commands, Action, Dialog, Job, RepoTab};
use crate::git::{self, DiffSource, RepoState};
use crate::ui::icons::{self, Icon};
use crate::ui::theme::{self, *};
use crate::ui::widgets::{file_row, hint_row, list_header, panel_header, primary_button};
use eframe::egui::{self, vec2, Align, Frame, Layout, Margin, RichText, Sense, TextEdit, Ui};

pub fn show(ui: &mut Ui, tab: &mut RepoTab, acts: &mut Vec<Action>) {
    let Some(d) = tab.data.clone() else { return };
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    panel_header(ui, tr!("wip.header"), MODIFIED);

    // Commit box at the bottom.
    egui::TopBottomPanel::bottom(egui::Id::new(("commitbox", tab.id)))
        .frame(Frame::new().fill(BG2).inner_margin(Margin::same(14)))
        .resizable(false)
        .show_inside(ui, |ui| commit_box(ui, tab, &d, acts));

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new(format!(
                        "{} ",
                        tr!("wip.files_changed_on", n = d.status.total())
                    ))
                    .color(TEXT_DIM),
                );
                ui.label(
                    RichText::new(tab.head_label())
                        .font(theme::semibold(13.5))
                        .color(CYAN),
                );
            });
            let selected_path = tab
                .diff
                .as_ref()
                .map(|(f, s)| (f.path.clone(), matches!(s, DiffSource::Staged)));

            if !d.status.conflicted.is_empty() {
                list_header(
                    ui,
                    tr!("wip.conflicts"),
                    d.status.conflicted.len(),
                    WARNING,
                    None,
                );
                for f in d.status.conflicted.iter().take(500) {
                    let (resp, b) = file_row(
                        ui,
                        f,
                        false,
                        &[
                            (Icon::Laptop, CYAN, tr!("wip.keep_ours")),
                            (Icon::Cloud, PINK, tr!("wip.keep_theirs")),
                            (Icon::Check, ADDED, tr!("wip.mark_resolved")),
                        ],
                    );
                    let path = f.path.clone();
                    match b {
                        Some(0) => acts.push(Action::Run(
                            Job::new(
                                tab,
                                tr!("wip.kept_ours_job"),
                                vec![
                                    args(&["checkout", "--ours", "--", &path]),
                                    args(&["add", "--", &path]),
                                ],
                            )
                            .quick(),
                        )),
                        Some(1) => acts.push(Action::Run(
                            Job::new(
                                tab,
                                tr!("wip.kept_theirs_job"),
                                vec![
                                    args(&["checkout", "--theirs", "--", &path]),
                                    args(&["add", "--", &path]),
                                ],
                            )
                            .quick(),
                        )),
                        Some(_) => acts.push(Action::Run(
                            Job::new(
                                tab,
                                tr!("wip.resolved_job"),
                                vec![args(&["add", "--", &path])],
                            )
                            .quick(),
                        )),
                        None => {
                            if resp.clicked() {
                                acts.push(Action::OpenDiff(DiffSource::Unstaged, path));
                            }
                        }
                    }
                }
            }

            let unstaged: Vec<String> = d.status.unstaged.iter().map(|f| f.path.clone()).collect();
            if list_header(
                ui,
                tr!("wip.unstaged"),
                d.status.unstaged.len(),
                MODIFIED,
                (!unstaged.is_empty()).then_some((tr!("wip.stage_all"), Icon::Plus, ADDED)),
            ) {
                acts.push(Action::Run(
                    Job::new(tab, tr!("wip.stage_all"), vec![args(&["add", "-A"])]).quick(),
                ));
            }
            if unstaged.is_empty() {
                hint_row(ui, tr!("wip.nothing_to_stage"));
            }
            for f in d.status.unstaged.iter().take(800) {
                let sel = selected_path
                    .as_ref()
                    .map(|(p, s)| p == &f.path && !s)
                    .unwrap_or(false);
                let (resp, b) = file_row(
                    ui,
                    f,
                    sel,
                    &[
                        (Icon::Undo, REMOVED, tr!("wip.discard_file")),
                        (Icon::Plus, ADDED, tr!("wip.stage")),
                    ],
                );
                match b {
                    Some(0) => acts.push(Action::Dialog(Dialog::Confirm {
                        title: tr!("wip.discard_title").into(),
                        text: tr!("wip.discard_text", path = f.path),
                        ok_label: tr!("wip.discard_ok").into(),
                        danger: true,
                        job: commands::discard_job(tab, &f.path, f.code == '?'),
                    })),
                    Some(_) => acts.push(Action::Run(commands::stage_job(tab, &[f.path.clone()]))),
                    None => {
                        if resp.double_clicked() {
                            acts.push(Action::Run(commands::stage_job(tab, &[f.path.clone()])));
                        } else if resp.clicked() {
                            let src = if f.code == '?' {
                                DiffSource::Untracked
                            } else {
                                DiffSource::Unstaged
                            };
                            acts.push(Action::OpenDiff(src, f.path.clone()));
                        }
                    }
                }
            }
            if d.status.unstaged.len() > 800 {
                hint_row(
                    ui,
                    &tr!("wip.more_files", n = d.status.unstaged.len() - 800),
                );
            }

            ui.add_space(8.0);
            let staged: Vec<String> = d.status.staged.iter().map(|f| f.path.clone()).collect();
            if list_header(
                ui,
                tr!("wip.staged"),
                staged.len(),
                ADDED,
                (!staged.is_empty()).then_some((tr!("wip.unstage_all"), Icon::Undo, MODIFIED)),
            ) {
                let job = if d.head_hash.is_some() {
                    Job::new(tab, tr!("wip.unstage_all"), vec![args(&["reset", "-q"])]).quick()
                } else {
                    Job::new(
                        tab,
                        tr!("wip.unstage_all"),
                        vec![args(&["rm", "--cached", "-r", "-q", "."])],
                    )
                    .quick()
                };
                acts.push(Action::Run(job));
            }
            if staged.is_empty() {
                hint_row(ui, tr!("wip.stage_hint"));
            }
            for f in d.status.staged.iter().take(800) {
                let sel = selected_path
                    .as_ref()
                    .map(|(p, s)| p == &f.path && *s)
                    .unwrap_or(false);
                let (resp, b) = file_row(ui, f, sel, &[(Icon::Undo, MODIFIED, tr!("wip.unstage"))]);
                if b.is_some() || resp.double_clicked() {
                    acts.push(Action::Run(commands::unstage_job(tab, &[f.path.clone()])));
                } else if resp.clicked() {
                    acts.push(Action::OpenDiff(DiffSource::Staged, f.path.clone()));
                }
            }
            ui.add_space(16.0);
        });
}

pub fn commit_box(ui: &mut Ui, tab: &mut RepoTab, d: &git::RepoData, acts: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = vec2(6.0, 8.0);
    let merging = d.state == RepoState::Merging;
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        icons::draw(ui.painter(), Icon::Commit, r, PINK);
        ui.label(
            RichText::new(tr!("wip.commit_message"))
                .font(theme::semibold(13.5))
                .color(TEXT),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let n = tab.summary.chars().count();
            let c = if n > 72 { WARNING } else { TEXT_FAINT };
            ui.label(
                RichText::new(format!("{n}/72"))
                    .font(theme::regular(11.5))
                    .color(c),
            );
        });
    });
    let summary = ui.add(
        TextEdit::singleline(&mut tab.summary)
            .hint_text(if merging {
                tr!("wip.summary_merge_hint")
            } else {
                tr!("wip.summary_hint")
            })
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(10, 8))
            .font(theme::regular(14.0)),
    );
    ui.add(
        TextEdit::multiline(&mut tab.body)
            .hint_text(tr!("wip.description_hint"))
            .desired_width(f32::INFINITY)
            .desired_rows(3)
            .margin(Margin::symmetric(10, 8)),
    );
    let before = tab.amend;
    ui.checkbox(
        &mut tab.amend,
        RichText::new(tr!("wip.amend_checkbox")).color(TEXT_DIM),
    );
    if tab.amend && !before && tab.summary.is_empty() {
        if let Some(h) = &d.head_hash {
            if let Some(c) = d.commits.iter().find(|c| &c.hash == h) {
                tab.summary = c.subject.clone();
            }
        }
    }
    let staged = d.status.staged.len();
    let label = if tab.amend {
        tr!("wip.amend_button").to_string()
    } else if merging {
        tr!("wip.commit_merge").to_string()
    } else if staged == 0 {
        tr!("wip.stage_all_and_commit").to_string()
    } else if staged == 1 {
        tr!("wip.commit_files_one", n = staged)
    } else {
        tr!("wip.commit_files_other", n = staged)
    };
    let enabled =
        (!tab.summary.trim().is_empty() || tab.amend || merging) && d.status.conflicted.is_empty();
    let w = ui.available_width();
    let ctrl_enter =
        summary.has_focus() && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
    if primary_button(ui, &label, enabled, w, 40.0)
        .on_hover_text(tr!("wip.commit_shortcut"))
        .clicked()
        || (ctrl_enter && enabled)
    {
        acts.push(Action::Commit);
    }
}
