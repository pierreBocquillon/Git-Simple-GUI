//! Right-click menus (branch, commit, WIP, stash, remote) and the menu entry widgets.

use crate::app::{args, commands, Action, Dialog, Job, Post, RepoTab, Selection, Undoable};
use crate::git::RefKind;
use crate::ui::icons::{self, Icon};
use crate::ui::paint::text_fit;
use crate::ui::theme::{self, *};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Ui};

/// A menu entry with an icon; closes the menu when clicked.
pub fn item(ui: &mut Ui, icon: Option<Icon>, text: &str) -> bool {
    item_colored(ui, icon, text, TEXT)
}

pub fn danger(ui: &mut Ui, icon: Option<Icon>, text: &str) -> bool {
    item_colored(ui, icon, text, REMOVED)
}

fn item_colored(ui: &mut Ui, icon: Option<Icon>, text: &str, color: Color32) -> bool {
    let w = ui.available_width().max(200.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), alpha(VIOLET, 0.22));
    }
    if let Some(i) = icon {
        icons::draw(
            ui.painter(),
            i,
            Rect::from_center_size(pos2(rect.left() + 15.0, rect.center().y), vec2(14.0, 14.0)),
            if color == TEXT { TEXT_DIM } else { color },
        );
    }
    text_fit(
        ui.painter(),
        rect.left() + 32.0,
        rect.center().y,
        text,
        theme::regular(13.0),
        color,
        w - 40.0,
    );
    if resp.clicked() {
        ui.close_menu();
    }
    resp.clicked()
}

pub fn heading(ui: &mut Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    ui.painter().text(
        pos2(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme::semibold(10.5),
        TEXT_FAINT,
    );
}

pub fn sep(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
    ui.painter().line_segment(
        [
            pos2(rect.left() + 6.0, rect.center().y),
            pos2(rect.right() - 6.0, rect.center().y),
        ],
        egui::Stroke::new(1.0, BORDER),
    );
}

fn short(h: &str) -> &str {
    &h[..h.len().min(7)]
}

fn confirm(title: &str, text: String, ok: &str, job: Job) -> Action {
    Action::Dialog(Dialog::Confirm {
        title: title.into(),
        text,
        ok_label: ok.into(),
        danger: true,
        job,
    })
}

pub fn ref_menu(ui: &mut Ui, tab: &RepoTab, k: usize, acts: &mut Vec<Action>) {
    let Some(d) = tab.data.as_ref() else { return };
    let Some(r) = d.refs.get(k).cloned() else {
        return;
    };
    let current = d.head_branch.clone();
    let cur = current.clone().unwrap_or_else(|| "HEAD".into());
    let is_current = r.kind == RefKind::Local && current.as_deref() == Some(r.name.as_str());
    heading(
        ui,
        &match r.kind {
            RefKind::Local => tr!("menu.ref.heading_local", name = r.name),
            RefKind::Remote => tr!("menu.ref.heading_remote", name = r.name),
            RefKind::Tag => tr!("menu.ref.heading_tag", name = r.name),
        },
    );

    if !is_current {
        if let Some(job) = commands::checkout_ref_job(tab, k) {
            if item(
                ui,
                Some(Icon::Check),
                &tr!("menu.ref.checkout", name = r.name),
            ) {
                acts.push(Action::Run(job));
            }
        }
    }
    if r.kind != RefKind::Tag && !is_current && current.is_some() {
        if item(
            ui,
            Some(Icon::Merge),
            &tr!("menu.ref.merge_into", branch = r.name, into = cur),
        ) {
            acts.push(Action::Run(Job::new(
                tab,
                tr!("menu.ref.merge_job", branch = r.name, into = cur),
                vec![args(&["merge", "--no-edit", &r.name])],
            )));
        }
        if item(
            ui,
            Some(Icon::Branch),
            &tr!("menu.ref.rebase_onto", cur = cur, branch = r.name),
        ) {
            acts.push(confirm(
                tr!("menu.rebase"),
                tr!("menu.ref.rebase_confirm", cur = cur, branch = r.name),
                tr!("menu.rebase"),
                Job::new(
                    tab,
                    tr!("menu.ref.rebase_job", cur = cur, branch = r.name),
                    vec![args(&["rebase", &r.name])],
                ),
            ));
        }
    }

    if r.kind == RefKind::Local {
        sep(ui);
        if let Some(up) = &r.upstream {
            if r.behind > 0 && r.ahead == 0 {
                let label = tr!("menu.ref.fast_forward", name = r.name, upstream = up);
                if item(ui, Some(Icon::Pull), &label) {
                    let step = if is_current {
                        args(&["merge", "--ff-only", up])
                    } else {
                        args(&["fetch", ".", &format!("{up}:{}", r.name)])
                    };
                    acts.push(Action::Run(Job::new(tab, label, vec![step])));
                }
            }
        }
        if !d.remotes.is_empty() {
            let (remote, target) = match r.upstream.as_deref().and_then(|u| u.split_once('/')) {
                Some((rm, b)) if !r.upstream_gone => (rm.to_string(), b.to_string()),
                _ => (
                    if d.remotes.iter().any(|x| x.name == "origin") {
                        "origin".to_string()
                    } else {
                        d.remotes[0].name.clone()
                    },
                    r.name.clone(),
                ),
            };
            if item(
                ui,
                Some(Icon::Push),
                &tr!(
                    "menu.ref.push_to",
                    name = r.name,
                    target = format!("{remote}/{target}")
                ),
            ) {
                let mut a = args(&["push"]);
                if r.upstream.is_none() || r.upstream_gone {
                    a.push("-u".into());
                }
                a.push(remote.clone());
                a.push(format!("{}:{target}", r.name));
                acts.push(Action::Run(
                    Job::new(
                        tab,
                        tr!("menu.ref.push_job", name = r.name, remote = remote),
                        vec![a],
                    )
                    .post(Post::Push {
                        remote: remote.clone(),
                        branch: r.name.clone(),
                        target: target.clone(),
                    }),
                ));
            }
            let remote_branches: Vec<String> = d.remotes_refs().map(|x| x.name.clone()).collect();
            if !remote_branches.is_empty() {
                ui.menu_button(format!("      {}", tr!("menu.ref.set_upstream")), |ui| {
                    ui.set_min_width(220.0);
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .show(ui, |ui| {
                            for rb in &remote_branches {
                                if item(ui, Some(Icon::Cloud), rb) {
                                    acts.push(Action::Run(Job::new(
                                        tab,
                                        tr!(
                                            "menu.ref.set_upstream_job",
                                            name = r.name,
                                            upstream = rb
                                        ),
                                        vec![args(&[
                                            "branch",
                                            &format!("--set-upstream-to={rb}"),
                                            &r.name,
                                        ])],
                                    )));
                                }
                            }
                        });
                });
            }
        }
        sep(ui);
        if item(ui, Some(Icon::Branch), tr!("menu.create_branch_here")) {
            acts.push(Action::Dialog(Dialog::NewBranch {
                tab: tab.id,
                from: r.name.clone(),
                from_label: r.name.clone(),
                name: String::new(),
                checkout: true,
            }));
        }
        if item(ui, None, &tr!("menu.ref.rename", name = r.name)) {
            acts.push(Action::Dialog(Dialog::RenameBranch {
                tab: tab.id,
                old: r.name.clone(),
                name: r.name.clone(),
            }));
        }
        if !is_current
            && danger(
                ui,
                Some(Icon::Close),
                &tr!("menu.ref.delete", name = r.name),
            )
        {
            let job = Job::new(
                tab,
                tr!("menu.delete_job", name = r.name),
                vec![args(&["branch", "-D", &r.name])],
            )
            .post(Post::Undo(Undoable {
                label: tr!("menu.ref.delete_undo", name = r.name),
                undo: vec![args(&["branch", &r.name, &r.target])],
                redo: vec![args(&["branch", "-D", &r.name])],
            }));
            acts.push(confirm(
                tr!("menu.ref.delete_title"),
                tr!("menu.ref.delete_confirm", name = r.name),
                tr!("menu.delete"),
                job,
            ));
        }
    }

    if r.kind == RefKind::Remote {
        if let Some((remote, branch)) = r.remote_parts() {
            sep(ui);
            if item(ui, Some(Icon::Branch), tr!("menu.create_branch_here")) {
                acts.push(Action::Dialog(Dialog::NewBranch {
                    tab: tab.id,
                    from: r.name.clone(),
                    from_label: r.name.clone(),
                    name: branch.to_string(),
                    checkout: true,
                }));
            }
            if danger(
                ui,
                Some(Icon::Close),
                &tr!(
                    "menu.remote_branch.delete",
                    branch = branch,
                    remote = remote
                ),
            ) {
                acts.push(confirm(
                    tr!("menu.remote_branch.delete_title"),
                    tr!(
                        "menu.remote_branch.delete_confirm",
                        branch = branch,
                        remote = remote
                    ),
                    tr!("menu.remote_branch.delete_ok"),
                    Job::new(
                        tab,
                        tr!(
                            "menu.remote_branch.delete_job",
                            branch = branch,
                            remote = remote
                        ),
                        vec![args(&["push", remote, "--delete", branch])],
                    ),
                ));
            }
        }
    }

    if r.kind == RefKind::Tag {
        sep(ui);
        for rm in &d.remotes {
            if item(
                ui,
                Some(Icon::Push),
                &tr!("menu.tag.push", remote = rm.name),
            ) {
                acts.push(Action::Run(Job::new(
                    tab,
                    tr!("menu.tag.push_job", name = r.name, remote = rm.name),
                    vec![args(&["push", &rm.name, &format!("refs/tags/{}", r.name)])],
                )));
            }
        }
        if danger(
            ui,
            Some(Icon::Close),
            &tr!("menu.tag.delete", name = r.name),
        ) {
            let job = Job::new(
                tab,
                tr!("menu.tag.delete_job", name = r.name),
                vec![args(&["tag", "-d", &r.name])],
            )
            .post(Post::Undo(Undoable {
                label: tr!("menu.tag.delete_undo", name = r.name),
                undo: vec![args(&["tag", &r.name, &r.target])],
                redo: vec![args(&["tag", "-d", &r.name])],
            }));
            acts.push(Action::Run(job));
        }
        for rm in &d.remotes {
            if danger(
                ui,
                None,
                &tr!("menu.tag.delete_on_remote", remote = rm.name),
            ) {
                acts.push(confirm(
                    tr!("menu.tag.delete_remote_title"),
                    tr!(
                        "menu.tag.delete_remote_confirm",
                        name = r.name,
                        remote = rm.name
                    ),
                    tr!("menu.delete"),
                    Job::new(
                        tab,
                        tr!(
                            "menu.tag.delete_remote_job",
                            name = r.name,
                            remote = rm.name
                        ),
                        vec![args(&[
                            "push",
                            &rm.name,
                            "--delete",
                            &format!("refs/tags/{}", r.name),
                        ])],
                    ),
                ));
            }
        }
    }
    sep(ui);
    if item(ui, Some(Icon::Copy), tr!("menu.copy_name")) {
        acts.push(Action::Copy(r.name.clone()));
    }
}

pub fn commit_menu(ui: &mut Ui, tab: &RepoTab, hash: &str, acts: &mut Vec<Action>) {
    let Some(d) = tab.data.as_ref() else { return };
    let Some(c) = d.commits.iter().find(|c| c.hash == hash) else {
        return;
    };
    let cur = d.head_branch.clone().unwrap_or_else(|| "HEAD".into());
    let is_head = d.head_hash.as_deref() == Some(hash);
    heading(ui, &tr!("menu.commit.heading", hash = short(hash)));

    // Branch labels on this commit get their own sub-menus.
    if let Some(idx) = d.refs_by_commit.get(hash) {
        for &k in idx {
            let r = &d.refs[k];
            let icon = match r.kind {
                RefKind::Local => "💻",
                RefKind::Remote => "☁",
                RefKind::Tag => "🏷",
            };
            ui.menu_button(format!("{icon}  {}", r.name), |ui| {
                ui.set_min_width(250.0);
                ref_menu(ui, tab, k, acts);
            });
        }
        sep(ui);
    }

    if !is_head && item(ui, Some(Icon::Check), tr!("menu.commit.checkout")) {
        acts.push(Action::Run(commands::checkout_job(
            tab,
            hash,
            tr!("menu.commit.checkout_job", hash = short(hash)),
        )));
    }
    if item(ui, Some(Icon::Branch), tr!("menu.create_branch_here")) {
        acts.push(Action::Dialog(Dialog::NewBranch {
            tab: tab.id,
            from: hash.to_string(),
            from_label: tr!("menu.commit.from_label", hash = short(hash)),
            name: String::new(),
            checkout: true,
        }));
    }
    if item(ui, Some(Icon::Tag), tr!("menu.create_tag_here")) {
        acts.push(Action::Dialog(Dialog::NewTag {
            tab: tab.id,
            at: hash.to_string(),
            name: String::new(),
            message: String::new(),
            annotated: false,
        }));
    }
    sep(ui);
    if !is_head {
        if item(ui, Some(Icon::Commit), tr!("menu.commit.cherry_pick")) {
            let mut a = args(&["cherry-pick"]);
            if c.parents.len() > 1 {
                a.extend(args(&["-m", "1"]));
            }
            a.push(hash.to_string());
            acts.push(Action::Run(Job::new(
                tab,
                tr!("menu.commit.cherry_pick_job", hash = short(hash)),
                vec![a],
            )));
        }
        if d.head_branch.is_some()
            && item(
                ui,
                Some(Icon::Branch),
                &tr!("menu.commit.rebase_onto", cur = cur),
            )
        {
            acts.push(confirm(
                tr!("menu.rebase"),
                tr!("menu.commit.rebase_confirm", cur = cur, hash = short(hash)),
                tr!("menu.rebase"),
                Job::new(
                    tab,
                    tr!("menu.commit.rebase_job", cur = cur),
                    vec![args(&["rebase", hash])],
                ),
            ));
        }
    }
    if item(ui, Some(Icon::Undo), tr!("menu.commit.revert")) {
        let mut a = args(&["revert", "--no-edit"]);
        if c.parents.len() > 1 {
            a.extend(args(&["-m", "1"]));
        }
        a.push(hash.to_string());
        acts.push(Action::Run(Job::new(
            tab,
            tr!("menu.commit.revert_job", hash = short(hash)),
            vec![a],
        )));
    }
    if !is_head {
        if let Some(prev) = d.head_hash.clone() {
            ui.menu_button(
                format!("      {}", tr!("menu.commit.reset_to", cur = cur)),
                |ui| {
                    ui.set_min_width(300.0);
                    for (mode, text) in [
                        ("--soft", tr!("menu.commit.reset_soft")),
                        ("--mixed", tr!("menu.commit.reset_mixed")),
                        ("--hard", tr!("menu.commit.reset_hard")),
                    ] {
                        let clicked = if mode == "--hard" {
                            danger(ui, None, text)
                        } else {
                            item(ui, None, text)
                        };
                        if clicked {
                            let job = Job::new(
                                tab,
                                tr!("menu.commit.reset_job", mode = mode, hash = short(hash)),
                                vec![args(&["reset", mode, hash])],
                            )
                            .post(Post::Undo(Undoable {
                                label: tr!("menu.commit.reset_undo", hash = short(hash)),
                                undo: vec![args(&["reset", mode, &prev])],
                                redo: vec![args(&["reset", mode, hash])],
                            }));
                            if mode == "--hard" {
                                acts.push(confirm(
                                    tr!("menu.commit.reset_hard_title"),
                                    tr!(
                                        "menu.commit.reset_hard_confirm",
                                        cur = cur,
                                        hash = short(hash)
                                    ),
                                    tr!("menu.commit.reset_hard_title"),
                                    job,
                                ));
                            } else {
                                acts.push(Action::Run(job));
                            }
                        }
                    }
                },
            );
        }
    }
    sep(ui);
    if item(ui, Some(Icon::Copy), tr!("menu.commit.copy_sha")) {
        acts.push(Action::Copy(hash.to_string()));
    }
    if item(ui, None, tr!("menu.commit.copy_message")) {
        acts.push(Action::Copy(c.subject.clone()));
    }
}

pub fn wip_menu(ui: &mut Ui, tab: &RepoTab, acts: &mut Vec<Action>) {
    heading(ui, tr!("menu.wip.heading"));
    if item(ui, Some(Icon::Plus), tr!("menu.wip.stage_all")) {
        acts.push(Action::Run(
            Job::new(tab, tr!("menu.wip.stage_all"), vec![args(&["add", "-A"])]).quick(),
        ));
    }
    if item(ui, Some(Icon::Stash), tr!("menu.wip.stash")) {
        acts.push(Action::Dialog(Dialog::Stash {
            tab: tab.id,
            message: String::new(),
            untracked: true,
        }));
    }
    if item(ui, Some(Icon::Branch), tr!("menu.create_branch_here")) {
        acts.push(Action::Dialog(Dialog::NewBranch {
            tab: tab.id,
            from: "HEAD".into(),
            from_label: "HEAD".into(),
            name: String::new(),
            checkout: true,
        }));
    }
    sep(ui);
    if danger(ui, Some(Icon::Close), tr!("menu.wip.discard_all")) {
        acts.push(confirm(
            tr!("menu.wip.discard_title"),
            tr!("menu.wip.discard_confirm").into(),
            tr!("menu.wip.discard_ok"),
            Job::new(
                tab,
                tr!("menu.wip.discard_job"),
                vec![
                    args(&["reset", "--hard", "-q"]),
                    args(&["clean", "-fd", "-q"]),
                ],
            ),
        ));
    }
    let _ = Selection::Wip;
}

pub fn stash_menu(ui: &mut Ui, tab: &RepoTab, name: &str, acts: &mut Vec<Action>) {
    heading(ui, &tr!("menu.stash.heading", name = name));
    if item(ui, Some(Icon::Pop), tr!("menu.stash.pop")) {
        acts.push(Action::Run(Job::new(
            tab,
            tr!("menu.stash.pop_job", name = name),
            vec![args(&["stash", "pop", name])],
        )));
    }
    if item(ui, Some(Icon::Box), tr!("menu.stash.apply")) {
        acts.push(Action::Run(Job::new(
            tab,
            tr!("menu.stash.apply_job", name = name),
            vec![args(&["stash", "apply", name])],
        )));
    }
    sep(ui);
    if danger(ui, Some(Icon::Close), tr!("menu.delete")) {
        acts.push(confirm(
            tr!("menu.stash.delete_title"),
            tr!("menu.stash.delete_confirm", name = name),
            tr!("menu.delete"),
            Job::new(
                tab,
                tr!("menu.delete_job", name = name),
                vec![args(&["stash", "drop", name])],
            ),
        ));
    }
}

pub fn remote_menu(ui: &mut Ui, tab: &RepoTab, name: &str, acts: &mut Vec<Action>) {
    let Some(d) = tab.data.as_ref() else { return };
    let Some(rm) = d.remotes.iter().find(|r| r.name == name).cloned() else {
        return;
    };
    heading(ui, &tr!("menu.remote.heading", name = name));
    if item(
        ui,
        Some(Icon::Fetch),
        &tr!("menu.remote.fetch", name = name),
    ) {
        acts.push(Action::Run(Job::new(
            tab,
            tr!("menu.remote.fetch_job", name = name),
            vec![args(&["fetch", name, "--prune"])],
        )));
    }
    if item(ui, Some(Icon::Gear), tr!("menu.remote.edit")) {
        acts.push(Action::Dialog(Dialog::EditRemote {
            tab: tab.id,
            old: rm.name.clone(),
            name: rm.name.clone(),
            url: rm.fetch_url.clone(),
        }));
    }
    if item(ui, Some(Icon::Copy), tr!("menu.remote.copy_url")) {
        acts.push(Action::Copy(rm.fetch_url.clone()));
    }
    sep(ui);
    if danger(
        ui,
        Some(Icon::Close),
        &tr!("menu.remote.remove", name = name),
    ) {
        acts.push(confirm(
            tr!("menu.remote.remove_title"),
            tr!(
                "menu.remote.remove_confirm",
                name = name,
                url = rm.fetch_url
            ),
            tr!("menu.remote.remove_ok"),
            Job::new(
                tab,
                tr!("menu.remote.remove_job", name = name),
                vec![args(&["remote", "remove", name])],
            ),
        ));
    }
}
