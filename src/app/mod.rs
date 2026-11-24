//! Application state and the frame loop.
//!
//! - `repo_tab`  one open repository (data, selection, undo history, view settings)
//! - `actions`   what the UI can ask for, and how each request is carried out
//! - `jobs`      git commands run in the background and what happens when they finish
//! - `commands`  ready-made jobs (checkout, stage, unstage…) shared by the UI
//! - `dialog`    the data typed into dialog windows
//! - `settings`  what is remembered between launches
//! - `workspace` named groups of tabs; only the current workspace's tabs are shown
//! - `updates`   check for a newer release and install it

mod actions;
pub mod commands;
mod dialog;
mod jobs;
mod repo_tab;
mod settings;
mod updates;
mod workspace;

pub use actions::Action;
pub use dialog::Dialog;
pub use jobs::{args, Job, JobResult, Post, Undoable};
pub use repo_tab::{RepoTab, Selection, PAGE_COMMITS};
pub use settings::PullMode;
pub use updates::Updates;
pub use workspace::Workspace;

use crate::git;
use crate::ui;
use eframe::egui::{self, Context, TextureHandle};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// A notification shown at the bottom right for a few seconds.
pub struct Toast {
    pub ok: bool,
    pub title: String,
    pub text: String,
    pub born: Instant,
}

pub struct GitApp {
    /// Open tabs of every workspace. Only those of the current workspace are shown.
    pub tabs: Vec<RepoTab>,
    /// Position in `tabs` of the tab on screen (`None`: the welcome page).
    pub active: Option<usize>,
    /// Always at least one.
    pub workspaces: Vec<Workspace>,
    /// Position in `workspaces` of the workspace on screen.
    pub ws: usize,
    pub logo: TextureHandle,
    pub recent: Vec<PathBuf>,
    pub pull_mode: PullMode,
    pub toasts: Vec<Toast>,
    pub dialog: Option<Dialog>,
    /// `None` when git is not installed.
    pub git_version: Option<String>,
    /// Name of the connected GitHub account (see `crate::github`).
    pub github_login: Option<String>,
    /// The language picked in the app (`None`: the installer's choice, else English).
    pub language: Option<crate::i18n::Lang>,
    /// Authors' profile pictures.
    pub avatars: crate::avatars::Avatars,
    /// Newer release available, check and download in progress.
    pub updates: Updates,
    /// Label of a job running from the welcome page (clone, init).
    pub welcome_busy: Option<String>,
    /// Background jobs send their result here.
    tx: Sender<JobResult>,
    rx: Receiver<JobResult>,
    next_id: u64,
    was_focused: bool,
    last_poll: Instant,
}

impl GitApp {
    pub fn new(cc: &eframe::CreationContext, open: Vec<PathBuf>, logo: egui::ColorImage) -> Self {
        ui::theme::install_fonts(&cc.egui_ctx);
        ui::theme::apply_style(&cc.egui_ctx);
        let logo = cc
            .egui_ctx
            .load_texture("logo", logo, egui::TextureOptions::LINEAR);
        let (tx, rx) = channel();
        let git_version = {
            let o = git::run_in(None, &["--version".to_string()]);
            o.ok.then(|| {
                o.stdout
                    .trim()
                    .trim_start_matches("git version ")
                    .to_string()
            })
        };
        let token = crate::github::saved_token();
        let mut app = GitApp {
            tabs: Vec::new(),
            active: None,
            workspaces: Vec::new(),
            ws: 0,
            logo,
            recent: Vec::new(),
            pull_mode: PullMode::Merge,
            toasts: Vec::new(),
            dialog: None,
            git_version,
            welcome_busy: None,
            github_login: None,
            language: None,
            avatars: crate::avatars::Avatars::new(&cc.egui_ctx, token.clone()),
            updates: Updates::default(),
            tx,
            rx,
            next_id: 1,
            was_focused: true,
            last_poll: Instant::now(),
        };
        // Language: the one picked in the app (read by load_state), else the installer's.
        if let Some(lang) = settings::installer_language() {
            crate::i18n::set(lang);
        }
        let (saved, current) = app.load_state();
        if token.is_none() {
            // The token was removed from the Credential Manager: no account any more.
            app.github_login = None;
        }
        for w in saved {
            let id = app.next_id;
            app.next_id += 1;
            app.workspaces.push(Workspace {
                id,
                name: w.name,
                focus: None,
            });
            app.ws = app.workspaces.len() - 1;
            if app.git_version.is_some() {
                let opened: Vec<u64> = w
                    .open
                    .iter()
                    .filter_map(|p| app.add_tab(p, false))
                    .collect();
                app.workspaces[app.ws].focus = w.focus.and_then(|f| opened.get(f).copied());
            }
        }
        if app.workspaces.is_empty() {
            let id = app.next_id;
            app.next_id += 1;
            app.workspaces.push(Workspace {
                id,
                name: tr!("workspace.default_name").into(),
                focus: None,
            });
        }
        app.ws = current.min(app.workspaces.len() - 1);
        app.show_focus();
        // Folders given on the command line open in the current workspace.
        if app.git_version.is_some() {
            for p in open {
                app.add_tab(&p, false);
            }
        }
        app.save_state();
        app.check_for_update(&cc.egui_ctx, false);
        app
    }

    pub fn tab(&self) -> Option<&RepoTab> {
        self.active.and_then(|i| self.tabs.get(i))
    }

    pub fn tab_mut(&mut self) -> Option<&mut RepoTab> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }

    /// Open a repository in a new tab of the current workspace (or switch to it if it is
    /// already open there).
    pub fn open_repo(&mut self, path: &Path, report: bool) {
        if self.add_tab(path, report).is_some() {
            self.save_state();
        }
    }

    /// Same as `open_repo` without saving. Returns the id of the tab now on screen.
    fn add_tab(&mut self, path: &Path, report: bool) -> Option<u64> {
        let Some(root) = git::repo_root(path) else {
            if report {
                self.toast(
                    false,
                    tr!("app.not_a_repo"),
                    &tr!("app.not_a_repo_text", path = path.display()),
                );
            }
            self.recent.retain(|r| r != path);
            return None;
        };
        let root = PathBuf::from(root.to_string_lossy().replace('/', "\\"));
        let workspace = self.workspace().id;
        let existing = self
            .tabs
            .iter()
            .position(|t| t.workspace == workspace && same_path(&t.path, &root));
        if let Some(i) = existing {
            self.active = Some(i);
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.tabs.push(RepoTab::new(id, workspace, root.clone()));
            self.active = Some(self.tabs.len() - 1);
        }
        self.recent.retain(|r| !same_path(r, &root));
        self.recent.insert(0, root);
        self.recent.truncate(12);
        self.active.map(|a| self.tabs[a].id)
    }

    pub fn toast(&mut self, ok: bool, title: &str, text: &str) {
        self.toasts.push(Toast {
            ok,
            title: title.into(),
            text: text.into(),
            born: Instant::now(),
        });
        if self.toasts.len() > 5 {
            self.toasts.remove(0);
        }
    }

    /// Collect finished jobs, reload the repository when it changed on disk, expire toasts.
    fn poll(&mut self, ctx: &Context) {
        while let Ok(r) = self.rx.try_recv() {
            self.finish(r, ctx);
        }
        self.poll_updates(ctx);
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let regained = focused && !self.was_focused;
        self.was_focused = focused;
        let due = self.last_poll.elapsed() > Duration::from_millis(2000);
        if regained || due {
            self.last_poll = Instant::now();
            if let Some(t) = self.tab_mut() {
                if t.busy.is_none() && (regained || t.changed_on_disk()) {
                    t.reload();
                }
            }
        }
        self.toasts
            .retain(|t| t.born.elapsed() < Duration::from_secs(if t.ok { 4 } else { 12 }));
    }
}

impl eframe::App for GitApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        let mut actions = Vec::new();
        ui::draw(self, ctx, &mut actions);
        self.apply(actions, ctx);
        let animating = self.tabs.iter().any(|t| t.busy.is_some())
            || self.welcome_busy.is_some()
            || !self.toasts.is_empty();
        ctx.request_repaint_after(Duration::from_millis(if animating { 100 } else { 2000 }));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_state();
    }
}

/// Windows paths compared without caring about slashes, case or a trailing separator.
fn same_path(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    n(a) == n(b)
}
