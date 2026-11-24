//! The modal dialog window: new branch, rename, tag, remotes, stash, clone, init, identity
//! workspaces and confirmations. Most dialogs turn what was typed into a git `Job`; the
//! workspace ones produce an `Action` instead.

use crate::app::{args, Action, Dialog, GitApp, Job, Post, RepoTab, Undoable};
use crate::git;
use crate::ui::icons::{self, Icon};
use crate::ui::paint::{self};
use crate::ui::theme::{self, *};
use crate::ui::widgets::{primary_button, secondary_button};
use eframe::egui::{
    self, vec2, Align, Align2, Color32, Context, CornerRadius, Frame, Layout, Margin, Rect,
    Response, RichText, Sense, Stroke, TextEdit, Ui,
};
use std::path::PathBuf;

pub fn field(ui: &mut Ui, label: &str, value: &mut String, hint: &str, focus: bool) -> Response {
    ui.label(
        RichText::new(label)
            .font(theme::semibold(12.5))
            .color(TEXT_DIM),
    );
    let r = ui.add(
        TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(10, 8)),
    );
    if focus && ui.memory(|m| m.focused().is_none()) {
        r.request_focus();
    }
    ui.add_space(4.0);
    r
}

pub fn folder_field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.label(
        RichText::new(label)
            .font(theme::semibold(12.5))
            .color(TEXT_DIM),
    );
    ui.horizontal(|ui| {
        let w = ui.available_width() - 110.0;
        ui.add(
            TextEdit::singleline(value)
                .desired_width(w)
                .margin(Margin::symmetric(10, 8)),
        );
        if secondary_button(ui, tr!("dialog.browse"), Some(Icon::Folder), VIOLET).clicked() {
            if let Some(p) = rfd::FileDialog::new().set_directory(&*value).pick_folder() {
                *value = p.display().to_string();
            }
        }
    });
    ui.add_space(4.0);
}

pub fn tab_job(tabs: &[RepoTab], tab: u64, label: String, steps: Vec<Vec<String>>) -> Option<Job> {
    tabs.iter()
        .find(|t| t.id == tab)
        .map(|t| Job::new(t, label, steps))
}

pub fn global_job(label: &str, steps: Vec<Vec<String>>, post: Post, sync: bool) -> Job {
    Job {
        tab: 0,
        cwd: None,
        label: label.into(),
        steps,
        post,
        sync,
        quiet: false,
    }
}

pub fn show(app: &mut GitApp, ctx: &Context, acts: &mut Vec<Action>) {
    let Some(dialog) = app.dialog.as_mut() else {
        return;
    };
    let tabs = &app.tabs;
    let github_login = app.github_login.clone();
    let update = app.updates.available.clone();
    let mut close = false;
    let mut submit: Option<Job> = None;
    let mut submit_action: Option<Action> = None;
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);

    let (title, icon, accent) = match dialog {
        Dialog::NewBranch { .. } => (tr!("dialog.new_branch.title"), Icon::Branch, CYAN),
        Dialog::RenameBranch { .. } => (tr!("dialog.rename_branch.title"), Icon::Branch, CYAN),
        Dialog::NewTag { .. } => (tr!("dialog.new_tag.title"), Icon::Tag, MODIFIED),
        Dialog::AddRemote { .. } => (tr!("dialog.add_remote.title"), Icon::Remote, CYAN),
        Dialog::EditRemote { .. } => (tr!("dialog.edit_remote.title"), Icon::Remote, CYAN),
        Dialog::Stash { .. } => (tr!("dialog.stash.title"), Icon::Stash, VIOLET),
        Dialog::Clone { .. } => (tr!("dialog.clone.title"), Icon::Clone, CYAN),
        Dialog::Init { .. } => (tr!("dialog.init.title"), Icon::Init, PINK),
        Dialog::Workspace { id: None, .. } => (tr!("dialog.workspace.new_title"), Icon::Box, PINK),
        Dialog::Workspace { .. } => (tr!("dialog.workspace.rename_title"), Icon::Box, PINK),
        Dialog::DeleteWorkspace { .. } => {
            (tr!("dialog.delete_workspace.title"), Icon::Warning, REMOVED)
        }
        Dialog::Identity { .. } => (tr!("dialog.identity.title"), Icon::Gear, VIOLET),
        Dialog::GitHub { .. } => (tr!("dialog.github.title"), Icon::Cloud, CYAN),
        Dialog::Update => (tr!("dialog.update.title"), Icon::Pull, CYAN),
        Dialog::Confirm { danger, .. } => {
            ("", Icon::Warning, if *danger { REMOVED } else { VIOLET })
        }
    };

    let modal = egui::Modal::new(egui::Id::new("dialog"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(
            Frame::new()
                .fill(BG2)
                .corner_radius(CornerRadius::same(20))
                .stroke(Stroke::new(1.5, alpha(VIOLET, 0.7)))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 14],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(160),
                })
                .inner_margin(Margin::same(22)),
        )
        .show(ctx, |ui| {
            ui.set_width(460.0);
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            let heading = match dialog {
                Dialog::Confirm { title, .. } => title.clone(),
                _ => title.to_string(),
            };
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                ui.painter()
                    .rect_filled(r, CornerRadius::same(9), alpha(accent, 0.16));
                icons::draw(ui.painter(), icon, r.shrink(9.0), accent);
                ui.add_space(4.0);
                ui.label(
                    RichText::new(heading)
                        .font(theme::semibold(18.0))
                        .color(TEXT),
                );
            });
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
            paint::gradient_h(
                ui.painter(),
                Rect::from_min_size(bar.min + vec2(0.0, 6.0), vec2(bar.width(), 2.0)),
                &[PINK, VIOLET, CYAN],
            );
            ui.add_space(14.0);

            let ok_label: String;
            let mut ok_enabled = true;
            let mut danger = false;
            match dialog {
                Dialog::NewBranch {
                    tab,
                    from,
                    from_label,
                    name,
                    checkout,
                } => {
                    ui.label(
                        RichText::new(tr!("dialog.new_branch.from", from = from_label))
                            .color(TEXT_FAINT),
                    );
                    ui.add_space(6.0);
                    field(
                        ui,
                        tr!("dialog.new_branch.name"),
                        name,
                        tr!("dialog.new_branch.hint"),
                        true,
                    );
                    ui.checkbox(checkout, tr!("dialog.new_branch.checkout"));
                    let clean = git::sanitize_ref_name(name);
                    if &clean != name && !clean.is_empty() {
                        ui.label(
                            RichText::new(tr!("dialog.new_branch.will_be_named", name = clean))
                                .color(TEXT_FAINT)
                                .font(theme::regular(12.0)),
                        );
                    }
                    ok_label = tr!("dialog.new_branch.ok").into();
                    ok_enabled = !clean.is_empty();
                    if ok_enabled {
                        let steps = if *checkout {
                            vec![args(&["checkout", "-b", &clean, from])]
                        } else {
                            vec![args(&["branch", &clean, from])]
                        };
                        let prev = tabs
                            .iter()
                            .find(|t| t.id == *tab)
                            .and_then(|t| t.data.as_ref())
                            .and_then(|d| d.head_branch.clone().or(d.head_hash.clone()));
                        let mut undo = Vec::new();
                        if *checkout {
                            if let Some(p) = prev {
                                undo.push(args(&["checkout", &p]));
                            }
                        }
                        undo.push(args(&["branch", "-D", &clean]));
                        submit = tab_job(
                            tabs,
                            *tab,
                            tr!("dialog.new_branch.job", name = clean),
                            steps.clone(),
                        )
                        .map(|j| {
                            j.post(Post::Undo(Undoable {
                                label: tr!("dialog.new_branch.undo", name = clean),
                                undo,
                                redo: steps,
                            }))
                        });
                    }
                }
                Dialog::RenameBranch { tab, old, name } => {
                    field(ui, tr!("dialog.rename_branch.name"), name, "", true);
                    let clean = git::sanitize_ref_name(name);
                    ok_label = tr!("dialog.rename").into();
                    ok_enabled = !clean.is_empty() && &clean != old;
                    if ok_enabled {
                        submit = tab_job(
                            tabs,
                            *tab,
                            tr!("dialog.rename_branch.job", old = old, name = clean),
                            vec![args(&["branch", "-m", old, &clean])],
                        )
                        .map(|j| {
                            j.post(Post::Undo(Undoable {
                                label: tr!("dialog.rename_branch.undo", old = old),
                                undo: vec![args(&["branch", "-m", &clean, old])],
                                redo: vec![args(&["branch", "-m", old, &clean])],
                            }))
                        });
                    }
                }
                Dialog::NewTag {
                    tab,
                    at,
                    name,
                    message,
                    annotated,
                } => {
                    ui.label(
                        RichText::new(tr!(
                            "dialog.new_tag.on_commit",
                            hash = &at[..7.min(at.len())]
                        ))
                        .color(TEXT_FAINT),
                    );
                    ui.add_space(6.0);
                    field(
                        ui,
                        tr!("dialog.new_tag.name"),
                        name,
                        tr!("dialog.new_tag.hint"),
                        true,
                    );
                    ui.checkbox(annotated, tr!("dialog.new_tag.annotated"));
                    if *annotated {
                        field(
                            ui,
                            tr!("dialog.new_tag.message"),
                            message,
                            tr!("dialog.new_tag.message_hint"),
                            false,
                        );
                    }
                    let clean = git::sanitize_ref_name(name);
                    ok_label = tr!("dialog.new_tag.ok").into();
                    ok_enabled = !clean.is_empty() && (!*annotated || !message.trim().is_empty());
                    if ok_enabled {
                        let steps = if *annotated {
                            vec![args(&["tag", "-a", &clean, "-m", message.trim(), at])]
                        } else {
                            vec![args(&["tag", &clean, at])]
                        };
                        submit = tab_job(
                            tabs,
                            *tab,
                            tr!("dialog.new_tag.job", name = clean),
                            steps.clone(),
                        )
                        .map(|j| {
                            j.post(Post::Undo(Undoable {
                                label: tr!("dialog.new_tag.undo", name = clean),
                                undo: vec![args(&["tag", "-d", &clean])],
                                redo: steps,
                            }))
                        });
                    }
                }
                Dialog::AddRemote { tab, name, url } => {
                    field(ui, tr!("dialog.name"), name, "origin", true);
                    field(ui, tr!("dialog.url"), url, tr!("dialog.url_hint"), false);
                    ui.label(
                        RichText::new(tr!("dialog.add_remote.note"))
                            .color(TEXT_FAINT)
                            .font(theme::regular(12.0)),
                    );
                    let n = git::sanitize_ref_name(name);
                    ok_label = tr!("dialog.add_remote.ok").into();
                    ok_enabled = !n.is_empty() && !url.trim().is_empty();
                    if ok_enabled {
                        submit = tab_job(
                            tabs,
                            *tab,
                            tr!("dialog.add_remote.job", name = n),
                            vec![
                                args(&["remote", "add", &n, url.trim()]),
                                args(&["fetch", &n]),
                            ],
                        );
                    }
                }
                Dialog::EditRemote {
                    tab,
                    old,
                    name,
                    url,
                } => {
                    field(ui, tr!("dialog.name"), name, "", true);
                    field(ui, tr!("dialog.url"), url, "", false);
                    let n = git::sanitize_ref_name(name);
                    ok_label = tr!("dialog.save").into();
                    ok_enabled = !n.is_empty() && !url.trim().is_empty();
                    if ok_enabled {
                        let mut steps = Vec::new();
                        if &n != old {
                            steps.push(args(&["remote", "rename", old, &n]));
                        }
                        steps.push(args(&["remote", "set-url", &n, url.trim()]));
                        submit =
                            tab_job(tabs, *tab, tr!("dialog.edit_remote.job", name = n), steps);
                    }
                }
                Dialog::Stash {
                    tab,
                    message,
                    untracked,
                } => {
                    field(
                        ui,
                        tr!("dialog.stash.message"),
                        message,
                        tr!("dialog.stash.message_hint"),
                        true,
                    );
                    ui.checkbox(untracked, tr!("dialog.stash.untracked"));
                    ok_label = tr!("dialog.stash.ok").into();
                    let mut a = args(&["stash", "push"]);
                    if *untracked {
                        a.push("-u".into());
                    }
                    if !message.trim().is_empty() {
                        a.push("-m".into());
                        a.push(message.trim().to_string());
                    }
                    submit = tab_job(tabs, *tab, tr!("dialog.stash.title").into(), vec![a]);
                }
                Dialog::Clone { url, parent, name } => {
                    let before = url.clone();
                    field(
                        ui,
                        tr!("dialog.clone.url"),
                        url,
                        tr!("dialog.url_hint"),
                        true,
                    );
                    if *url != before {
                        *name = url
                            .trim()
                            .trim_end_matches('/')
                            .rsplit(['/', ':'])
                            .next()
                            .unwrap_or("")
                            .trim_end_matches(".git")
                            .to_string();
                    }
                    folder_field(ui, tr!("dialog.parent_folder"), parent);
                    field(
                        ui,
                        tr!("dialog.clone.folder_name"),
                        name,
                        tr!("dialog.clone.folder_hint"),
                        false,
                    );
                    let dest = PathBuf::from(parent.trim()).join(name.trim());
                    ui.label(
                        RichText::new(tr!("dialog.clone.destination", path = dest.display()))
                            .color(TEXT_FAINT)
                            .font(theme::regular(12.0)),
                    );
                    ok_label = tr!("dialog.clone.ok").into();
                    ok_enabled = !url.trim().is_empty()
                        && !name.trim().is_empty()
                        && !parent.trim().is_empty();
                    if ok_enabled {
                        let label = tr!("dialog.clone.job", name = name.trim());
                        submit = Some(global_job(
                            &label,
                            vec![args(&["clone", url.trim(), &dest.display().to_string()])],
                            Post::Open(dest),
                            false,
                        ));
                    }
                }
                Dialog::Init { parent, name } => {
                    folder_field(ui, tr!("dialog.parent_folder"), parent);
                    field(
                        ui,
                        tr!("dialog.init.name"),
                        name,
                        tr!("dialog.init.hint"),
                        true,
                    );
                    let dest = PathBuf::from(parent.trim()).join(name.trim());
                    ui.label(
                        RichText::new(tr!("dialog.init.will_be_created", path = dest.display()))
                            .color(TEXT_FAINT)
                            .font(theme::regular(12.0)),
                    );
                    ok_label = tr!("dialog.create").into();
                    ok_enabled = !name.trim().is_empty() && !parent.trim().is_empty();
                    if ok_enabled {
                        let label = tr!("dialog.init.job", name = name.trim());
                        submit = Some(global_job(
                            &label,
                            vec![args(&["init", &dest.display().to_string()])],
                            Post::Open(dest),
                            true,
                        ));
                    }
                }
                Dialog::Workspace { id, name, with_tab } => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr!("dialog.workspace.explain")).color(TEXT_FAINT),
                        )
                        .wrap(),
                    );
                    ui.add_space(8.0);
                    field(
                        ui,
                        tr!("dialog.workspace.name"),
                        name,
                        tr!("dialog.workspace.hint"),
                        true,
                    );
                    ok_label = if id.is_some() {
                        tr!("dialog.rename")
                    } else {
                        tr!("dialog.create")
                    }
                    .into();
                    ok_enabled = !name.trim().is_empty();
                    submit_action = Some(match id {
                        Some(id) => Action::RenameWorkspace {
                            id: *id,
                            name: name.clone(),
                        },
                        None => Action::NewWorkspace {
                            name: name.clone(),
                            with_tab: *with_tab,
                        },
                    });
                }
                Dialog::DeleteWorkspace { id, name, tabs } => {
                    let text = tr!("dialog.delete_workspace.text", name = name, tabs = tabs);
                    ui.add(
                        egui::Label::new(
                            RichText::new(text)
                                .color(TEXT_DIM)
                                .font(theme::regular(13.5)),
                        )
                        .wrap(),
                    );
                    ok_label = tr!("dialog.delete_workspace.ok").into();
                    danger = true;
                    submit_action = Some(Action::DeleteWorkspace(*id));
                }
                Dialog::Update => {
                    let new = update.as_ref().map(|r| r.title.clone()).unwrap_or_default();
                    ui.label(RichText::new(new).font(theme::semibold(15.0)).color(TEXT));
                    ui.label(
                        RichText::new(tr!(
                            "dialog.update.current",
                            version = crate::update::current_version()
                        ))
                        .color(TEXT_FAINT),
                    );
                    if let Some(notes) = update
                        .as_ref()
                        .map(|r| r.notes.as_str())
                        .filter(|n| !n.is_empty())
                    {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(tr!("dialog.update.whats_new"))
                                .font(theme::semibold(12.5))
                                .color(TEXT_DIM),
                        );
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .show(ui, |ui| {
                                ui.add(egui::Label::new(RichText::new(notes).color(TEXT)).wrap());
                            });
                    }
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr!("dialog.update.explain"))
                                .color(TEXT_FAINT)
                                .font(theme::regular(12.0)),
                        )
                        .wrap(),
                    );
                    ok_label = tr!("dialog.update.ok").into();
                    ok_enabled = update.is_some();
                    submit_action = Some(Action::InstallUpdate);
                }
                Dialog::GitHub { token } => match &github_login {
                    Some(login) => {
                        ui.label(
                            RichText::new(tr!("dialog.github.connected_as", login = login))
                                .font(theme::semibold(14.0))
                                .color(TEXT),
                        );
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(tr!("dialog.github.connected_note"))
                                    .color(TEXT_FAINT),
                            )
                            .wrap(),
                        );
                        ok_label = tr!("dialog.github.disconnect").into();
                        danger = true;
                        submit_action = Some(Action::GitHubDisconnect);
                    }
                    None => {
                        ui.add(
                            egui::Label::new(
                                RichText::new(tr!("dialog.github.explain")).color(TEXT_FAINT),
                            )
                            .wrap(),
                        );
                        ui.add_space(8.0);
                        ui.label(RichText::new(tr!("dialog.github.step1")).color(TEXT_DIM));
                        if secondary_button(
                            ui,
                            tr!("dialog.github.create_token"),
                            Some(Icon::Cloud),
                            CYAN,
                        )
                        .clicked()
                        {
                            acts.push(Action::OpenUrl(crate::github::NEW_TOKEN_URL.into()));
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(tr!("dialog.github.step2")).color(TEXT_DIM));
                        let r = ui.add(
                            TextEdit::singleline(token)
                                .password(true)
                                .hint_text("ghp_…")
                                .desired_width(f32::INFINITY)
                                .margin(Margin::symmetric(10, 8)),
                        );
                        if ui.memory(|m| m.focused().is_none()) {
                            r.request_focus();
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(tr!("dialog.github.token_note"))
                                .color(TEXT_FAINT)
                                .font(theme::regular(12.0)),
                        );
                        ok_label = tr!("dialog.github.connect").into();
                        ok_enabled = !token.trim().is_empty();
                        submit_action = Some(Action::GitHubConnect(token.clone()));
                    }
                },
                Dialog::Identity { name, email } => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr!("dialog.identity.explain")).color(TEXT_FAINT),
                        )
                        .wrap(),
                    );
                    ui.add_space(8.0);
                    field(
                        ui,
                        tr!("dialog.name"),
                        name,
                        tr!("dialog.identity.name_hint"),
                        true,
                    );
                    field(
                        ui,
                        tr!("dialog.identity.email"),
                        email,
                        tr!("dialog.identity.email_hint"),
                        false,
                    );
                    ok_label = tr!("dialog.save").into();
                    ok_enabled = !name.trim().is_empty() && !email.trim().is_empty();
                    if ok_enabled {
                        submit = Some(global_job(
                            tr!("dialog.identity.job"),
                            vec![
                                args(&["config", "--global", "user.name", name.trim()]),
                                args(&["config", "--global", "user.email", email.trim()]),
                            ],
                            Post::None,
                            true,
                        ));
                    }
                }
                Dialog::Confirm {
                    text,
                    ok_label: l,
                    danger: d,
                    job,
                    ..
                } => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(text.as_str())
                                .color(TEXT_DIM)
                                .font(theme::regular(13.5)),
                        )
                        .wrap(),
                    );
                    ok_label = l.clone();
                    danger = *d;
                    submit = Some(job.clone());
                }
            }

            ui.add_space(18.0);
            let mut confirmed = false;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let font = theme::semibold(13.5);
                let tw = ui.fonts(|f| {
                    f.layout_no_wrap(ok_label.clone(), font.clone(), TEXT)
                        .size()
                        .x
                });
                let resp = if danger {
                    let (r, resp) = ui.allocate_exact_size(vec2(tw + 36.0, 38.0), Sense::click());
                    ui.painter().rect_filled(
                        r,
                        CornerRadius::same(8),
                        if resp.hovered() {
                            theme::mix(REMOVED, Color32::WHITE, 0.12)
                        } else {
                            REMOVED
                        },
                    );
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        &ok_label,
                        font,
                        Color32::WHITE,
                    );
                    resp
                } else {
                    primary_button(ui, &ok_label, ok_enabled, tw + 40.0, 38.0)
                };
                if resp.clicked() || (enter && ok_enabled && !danger) {
                    confirmed = true;
                }
                ui.add_space(4.0);
                if secondary_button(ui, tr!("dialog.cancel"), None, TEXT_DIM).clicked() {
                    close = true;
                }
            });
            if !confirmed || !ok_enabled {
                submit = None;
                submit_action = None;
            }
        });
    if modal.should_close() {
        close = true;
    }
    if let Some(job) = submit {
        acts.push(Action::Run(job));
        close = true;
    }
    if let Some(action) = submit_action {
        acts.push(action);
        close = true;
    }
    if close {
        app.dialog = None;
    }
}
