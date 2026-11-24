//! Everything the interface can ask for. The UI only pushes `Action`s; they are applied here,
//! once per frame, after drawing.

use super::{args, Dialog, GitApp, Job, Post, PullMode, Selection, PAGE_COMMITS};
use crate::git::{self, DiffSource, RepoState};
use eframe::egui::Context;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Action {
    Select(Selection),
    /// Select a commit and scroll the graph to it.
    Reveal(String),
    OpenDiff(DiffSource, String),
    CloseDiff,
    Run(Job),
    Dialog(Dialog),
    Copy(String),
    PickRepo,
    OpenRepo(PathBuf),
    CloseTab(usize),
    ActivateTab(Option<usize>),
    /// Put the workspace at this position on screen.
    SwitchWorkspace(usize),
    /// Create a workspace (and move a tab into it) then switch to it.
    NewWorkspace {
        name: String,
        with_tab: Option<u64>,
    },
    RenameWorkspace {
        id: u64,
        name: String,
    },
    /// Remove a workspace and close its tabs.
    DeleteWorkspace(u64),
    /// Move the tab with this id to another workspace.
    MoveTab {
        tab: u64,
        workspace: u64,
    },
    /// Move the tab with this id so it becomes the `to`-th tab of its workspace (drag and drop).
    ReorderTab {
        tab: u64,
        to: usize,
    },
    ForgetRecent(PathBuf),
    Refresh,
    LoadMore,
    Undo,
    Redo,
    Pull(Option<PullMode>),
    Push {
        force: bool,
    },
    Commit,
    /// Open the dialog to change the message of this commit (of the current branch).
    EditCommit(String),
    StashQuick,
    ShowInExplorer(PathBuf),
    /// Open a web page in the browser.
    OpenUrl(String),
    /// Check a GitHub token, save it and use it.
    GitHubConnect(String),
    GitHubDisconnect,
    /// Ask GitHub for a newer version (and say so even when there is none).
    CheckUpdate,
    /// Show the app in another language (remembered).
    SetLanguage(crate::i18n::Lang),
    /// Download the newer version's installer, run it and close.
    InstallUpdate,
    ToggleSearch,
}

impl GitApp {
    pub fn apply(&mut self, actions: Vec<Action>, ctx: &Context) {
        for a in actions {
            self.apply_one(a, ctx);
        }
    }

    fn apply_one(&mut self, action: Action, ctx: &Context) {
        match action {
            Action::Select(sel) => {
                if let Some(t) = self.tab_mut() {
                    if t.selection != sel {
                        t.selection = sel;
                        t.load_selection();
                        t.diff = None;
                    }
                }
            }
            Action::Reveal(hash) => {
                if let Some(t) = self.tab_mut() {
                    if let Some(i) = t.commit_index(&hash) {
                        let off = if t.has_wip_row() { 1 } else { 0 };
                        t.scroll_to_row = Some(i + off);
                        if t.selection != Selection::Commit(hash.clone()) {
                            t.selection = Selection::Commit(hash);
                            t.load_selection();
                            t.diff = None;
                        }
                    } else {
                        self.toast(
                            false,
                            tr!("action.commit_out_of_view"),
                            tr!("action.commit_out_of_view_text"),
                        );
                    }
                }
            }
            Action::OpenDiff(source, path) => {
                if let Some(t) = self.tab_mut() {
                    let d = git::file_diff(&t.path, &source, &path);
                    t.diff = Some((d, source));
                }
            }
            Action::CloseDiff => {
                if let Some(t) = self.tab_mut() {
                    t.diff = None;
                }
            }
            Action::Run(job) => self.run_job(job, ctx),
            Action::Dialog(d) => self.dialog = Some(d),
            Action::Copy(s) => {
                ctx.copy_text(s.clone());
                self.toast(true, tr!("action.copied"), &s);
            }
            Action::PickRepo => {
                if let Some(p) = rfd::FileDialog::new()
                    .set_title(tr!("action.open_repo_title"))
                    .pick_folder()
                {
                    self.open_repo(&p, true);
                }
            }
            Action::OpenRepo(p) => self.open_repo(&p, true),
            Action::CloseTab(i) => {
                if i < self.tabs.len() {
                    // Show the neighbour of the closed tab, within the same workspace.
                    let active_id = self.active.map(|a| self.tabs[a].id);
                    let local = self.visible_tabs().iter().position(|&j| j == i);
                    let closed_id = self.tabs.remove(i).id;
                    let visible = self.visible_tabs();
                    self.active = if active_id == Some(closed_id) {
                        local
                            .and_then(|l| visible.get(l.min(visible.len().saturating_sub(1))))
                            .copied()
                    } else {
                        active_id.and_then(|id| self.tabs.iter().position(|t| t.id == id))
                    };
                    self.save_state();
                }
            }
            Action::ActivateTab(i) => {
                self.active = i;
                if let Some(t) = self.tab_mut() {
                    t.reload();
                }
                self.save_state();
            }
            Action::SwitchWorkspace(i) => self.switch_workspace(i),
            Action::NewWorkspace { name, with_tab } => self.new_workspace(&name, with_tab),
            Action::RenameWorkspace { id, name } => self.rename_workspace(id, &name),
            Action::DeleteWorkspace(id) => self.delete_workspace(id),
            Action::ReorderTab { tab, to } => self.reorder_tab(tab, to),
            Action::MoveTab { tab, workspace } => {
                self.move_tab(tab, workspace);
                let name = self
                    .workspaces
                    .iter()
                    .find(|w| w.id == workspace)
                    .map(|w| w.name.clone())
                    .unwrap_or_default();
                self.toast(
                    true,
                    tr!("action.tab_moved"),
                    &tr!("action.tab_moved_text", name = name),
                );
            }
            Action::ForgetRecent(p) => {
                self.recent.retain(|r| r != &p);
                self.save_state();
            }
            Action::Refresh => {
                if let Some(t) = self.tab_mut() {
                    t.reload();
                    t.load_selection();
                }
            }
            Action::LoadMore => {
                if let Some(t) = self.tab_mut() {
                    t.max_commits += PAGE_COMMITS;
                    t.reload();
                }
            }
            Action::Undo | Action::Redo => {
                let undo = matches!(action, Action::Undo);
                let Some(t) = self.tab_mut() else { return };
                let item = if undo { t.undo.pop() } else { t.redo.pop() };
                if let Some(item) = item {
                    let label = if undo {
                        tr!("action.undo_label", label = item.label)
                    } else {
                        tr!("action.redo_label", label = item.label)
                    };
                    let steps = if undo {
                        item.undo.clone()
                    } else {
                        item.redo.clone()
                    };
                    let job = Job::new(t, label, steps).post(Post::History {
                        item,
                        was_undo: undo,
                    });
                    self.run_job(job, ctx);
                }
            }
            Action::Pull(mode) => {
                let mode = mode.unwrap_or(self.pull_mode);
                if let Some(m) = Some(mode) {
                    if self.pull_mode != m {
                        self.pull_mode = m;
                        self.save_state();
                    }
                }
                self.pull(mode, ctx);
            }
            Action::Push { force } => self.push(force, ctx),
            Action::Commit => self.commit(ctx),
            Action::EditCommit(hash) => self.edit_commit(&hash),
            Action::StashQuick => {
                let Some(t) = self.tab() else { return };
                let label = tr!("action.stash_message", branch = t.head_label());
                let job = Job::new(
                    t,
                    tr!("action.stash_changes"),
                    vec![args(&["stash", "push", "-u", "-m", &label])],
                );
                self.run_job(job, ctx);
            }
            Action::OpenUrl(url) => {
                // Not `explorer`: it mishandles URLs with `?` or `&` and opens Documents instead.
                let _ = std::process::Command::new("rundll32")
                    .args(["url.dll,FileProtocolHandler", &url])
                    .spawn();
            }
            Action::GitHubConnect(token) => match crate::github::connect(&token) {
                Ok(login) => {
                    self.toast(
                        true,
                        tr!("action.github_connected"),
                        &tr!("action.github_connected_text", login = login),
                    );
                    self.github_login = Some(login);
                    self.avatars.set_token(Some(token.trim().to_string()));
                    self.save_state();
                }
                Err(e) => self.toast(false, tr!("action.github_connect_failed"), &e),
            },
            Action::CheckUpdate => self.check_for_update(ctx, true),
            Action::InstallUpdate => self.install_update(ctx),
            Action::SetLanguage(lang) => {
                crate::i18n::set(lang);
                self.language = Some(lang);
                self.save_state();
            }
            Action::GitHubDisconnect => {
                crate::github::disconnect();
                self.github_login = None;
                self.avatars.set_token(None);
                self.save_state();
                self.toast(
                    true,
                    tr!("action.github_disconnected"),
                    tr!("action.github_disconnected_text"),
                );
            }
            Action::ShowInExplorer(path) => {
                let _ = std::process::Command::new("explorer").arg(path).spawn();
            }
            Action::ToggleSearch => {
                if let Some(t) = self.tab_mut() {
                    t.search_open = !t.search_open;
                    t.search_focus = t.search_open;
                    if !t.search_open {
                        t.search.clear();
                    }
                }
            }
        }
    }

    fn pull(&mut self, mode: PullMode, ctx: &Context) {
        let Some(t) = self.tab() else { return };
        let Some(d) = &t.data else { return };
        if d.remotes.is_empty() {
            self.toast(false, tr!("action.no_remote"), tr!("action.no_remote_text"));
            return;
        }
        let steps = match mode {
            PullMode::FetchAll => vec![args(&["fetch", "--all", "--prune"])],
            _ => {
                if d.head_ref().and_then(|r| r.upstream.as_ref()).is_none() {
                    self.toast(
                        false,
                        tr!("action.no_upstream"),
                        tr!("action.no_upstream_text"),
                    );
                    return;
                }
                match mode {
                    PullMode::Merge => vec![args(&["pull", "--ff", "--no-edit"])],
                    PullMode::FfOnly => vec![args(&["pull", "--ff-only"])],
                    _ => vec![args(&["pull", "--rebase"])],
                }
            }
        };
        let label = match mode {
            // The toolbar recognises a running pull or fetch by this first word.
            PullMode::FetchAll => tr!("action.fetch_label").to_string(),
            _ => tr!("action.pull_label", branch = t.head_label()),
        };
        let job = Job::new(t, label, steps);
        self.run_job(job, ctx);
    }

    fn push(&mut self, force: bool, ctx: &Context) {
        let Some(t) = self.tab() else { return };
        let Some(d) = &t.data else { return };
        let Some(branch) = d.head_branch.clone() else {
            self.toast(
                false,
                tr!("action.detached_head"),
                tr!("action.detached_head_text"),
            );
            return;
        };
        if d.remotes.is_empty() {
            self.toast(false, tr!("action.no_remote"), tr!("action.no_remote_text"));
            return;
        }
        let upstream = d.head_ref().and_then(|r| {
            if r.upstream_gone {
                None
            } else {
                r.upstream.clone()
            }
        });
        let (remote, target, set_upstream) =
            match upstream.as_deref().and_then(|u| u.split_once('/')) {
                Some((r, b)) => (r.to_string(), b.to_string(), false),
                None => {
                    let r = if d.remotes.iter().any(|r| r.name == "origin") {
                        "origin".to_string()
                    } else {
                        d.remotes[0].name.clone()
                    };
                    (r, branch.clone(), true)
                }
            };
        let mut a = vec!["push".to_string()];
        if set_upstream {
            a.push("-u".into());
        }
        if force {
            a.push("--force-with-lease".into());
        }
        a.push(remote.clone());
        a.push(format!("{branch}:{target}"));
        // The toolbar recognises a running push by the first word of this label.
        let label = tr!("action.push_label", branch = branch, remote = remote);
        let job = Job::new(t, label, vec![a]).post(Post::Push {
            remote,
            branch,
            target,
        });
        self.run_job(job, ctx);
    }

    /// Open the "edit commit message" dialog, filled with the commit's current message.
    fn edit_commit(&mut self, hash: &str) {
        let Some(t) = self.tab() else { return };
        let Some(d) = &t.data else { return };
        if d.state != RepoState::Clean {
            self.toast(
                false,
                tr!("action.edit_commit_busy"),
                tr!("action.edit_commit_busy_text"),
            );
            return;
        }
        let message = git::git(&t.path, &["log", "-1", "--format=%B", hash]).stdout;
        let (summary, body) = match message.trim().split_once('\n') {
            Some((s, b)) => (s.trim().to_string(), b.trim().to_string()),
            None => (message.trim().to_string(), String::new()),
        };
        let pushed = !git::git(&t.path, &["branch", "-r", "--contains", hash])
            .stdout
            .trim()
            .is_empty();
        self.dialog = Some(Dialog::EditCommit {
            tab: t.id,
            hash: hash.to_string(),
            summary,
            body,
            pushed,
        });
    }

    fn commit(&mut self, ctx: &Context) {
        let Some(t) = self.tab() else { return };
        let Some(d) = &t.data else { return };
        let summary = t.summary.trim().to_string();
        let body = t.body.trim().to_string();
        let merging = d.state == RepoState::Merging;
        if !d.status.conflicted.is_empty() {
            self.toast(false, tr!("action.conflicts"), tr!("action.conflicts_text"));
            return;
        }
        if summary.is_empty() && !t.amend && !merging {
            self.toast(
                false,
                tr!("action.missing_message"),
                tr!("action.missing_message_text"),
            );
            return;
        }
        let mut steps = Vec::new();
        if d.status.staged.is_empty() && !t.amend && !merging {
            if d.status.unstaged.is_empty() {
                self.toast(
                    false,
                    tr!("action.nothing_to_commit"),
                    tr!("action.nothing_to_commit_text"),
                );
                return;
            }
            steps.push(args(&["add", "-A"]));
        }
        let mut c = vec!["commit".to_string()];
        if t.amend {
            c.push("--amend".into());
        }
        if summary.is_empty() {
            c.push("--no-edit".into());
        } else {
            c.push("-m".into());
            c.push(summary.clone());
            if !body.is_empty() {
                c.push("-m".into());
                c.push(body);
            }
        }
        steps.push(c);
        let label = if t.amend {
            tr!("action.commit_amended").to_string()
        } else {
            tr!("action.commit_label", summary = summary)
        };
        let job = Job::new(t, label, steps).post(Post::Commit {
            prev: d.head_hash.clone(),
        });
        self.run_job(job, ctx);
    }
}
