//! Git jobs: a list of git commands run in the background (or on the spot for quick ones),
//! and what to do once they finish (undo history, toasts, reload).

use super::{Dialog, GitApp, RepoTab, Selection};
use crate::git::{self, GitOutput};
use eframe::egui::Context;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Undoable {
    pub label: String,
    pub undo: Vec<Vec<String>>,
    pub redo: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
pub enum Post {
    None,
    Undo(Undoable),
    Commit {
        prev: Option<String>,
    },
    Push {
        remote: String,
        branch: String,
        target: String,
    },
    Open(PathBuf),
    History {
        item: Undoable,
        was_undo: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Job {
    pub tab: u64,
    pub cwd: Option<PathBuf>,
    pub label: String,
    pub steps: Vec<Vec<String>>,
    pub post: Post,
    /// Run on the spot (quick local commands such as staging) instead of in the background.
    pub sync: bool,
    /// No toast when it succeeds.
    pub quiet: bool,
}

impl Job {
    pub fn new(tab: &RepoTab, label: impl Into<String>, steps: Vec<Vec<String>>) -> Job {
        Job {
            tab: tab.id,
            cwd: Some(tab.path.clone()),
            label: label.into(),
            steps,
            post: Post::None,
            sync: false,
            quiet: false,
        }
    }
    pub fn quick(mut self) -> Job {
        self.sync = true;
        self.quiet = true;
        self
    }
    pub fn post(mut self, post: Post) -> Job {
        self.post = post;
        self
    }
}

/// Build an argument list: `args(&["branch", "-D", name])`.
pub fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

pub struct JobResult {
    pub job: Job,
    pub output: GitOutput,
}

impl GitApp {
    pub fn run_job(&mut self, job: Job, ctx: &Context) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == job.tab) {
            if let Some(b) = &tab.busy {
                let b = b.clone();
                self.toast(false, tr!("job.wait"), &tr!("job.wait_text", label = b));
                return;
            }
            if !job.sync {
                tab.busy = Some(job.label.clone());
            }
        } else if job.tab == 0 {
            self.welcome_busy = Some(job.label.clone());
        }
        if job.sync {
            let output = run_steps(&job);
            self.finish(JobResult { job, output }, ctx);
            return;
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let output = run_steps(&job);
            let _ = tx.send(JobResult { job, output });
            ctx.request_repaint();
        });
    }

    pub(super) fn finish(&mut self, r: JobResult, ctx: &Context) {
        let JobResult { job, output } = r;
        if job.tab == 0 {
            self.welcome_busy = None;
        }
        let tab_idx = self.tabs.iter().position(|t| t.id == job.tab);
        if let Some(i) = tab_idx {
            self.tabs[i].busy = None;
        }

        if output.ok {
            match job.post.clone() {
                Post::Undo(u) => {
                    if let Some(i) = tab_idx {
                        self.tabs[i].undo.push(u);
                        self.tabs[i].redo.clear();
                    }
                }
                Post::Commit { prev } => {
                    if let Some(i) = tab_idx {
                        let t = &mut self.tabs[i];
                        t.summary.clear();
                        t.body.clear();
                        t.amend = false;
                        let new = git::git(&t.path, &["rev-parse", "HEAD"])
                            .stdout
                            .trim()
                            .to_string();
                        let undo = match &prev {
                            Some(p) => vec![args(&["reset", "--soft", p])],
                            None => vec![args(&["update-ref", "-d", "HEAD"])],
                        };
                        t.undo.push(Undoable {
                            label: "commit".into(),
                            undo,
                            redo: vec![args(&["reset", "--soft", &new])],
                        });
                        t.redo.clear();
                        t.selection = Selection::Commit(new);
                        t.scroll_to_row = Some(0);
                    }
                }
                Post::History { item, was_undo } => {
                    if let Some(i) = tab_idx {
                        if was_undo {
                            self.tabs[i].redo.push(item);
                        } else {
                            self.tabs[i].undo.push(item);
                        }
                    }
                }
                Post::Open(path) => {
                    self.open_repo(&path, true);
                }
                Post::Push { .. } | Post::None => {}
            }
            if !job.quiet {
                let msg = output.message();
                self.toast(
                    true,
                    &job.label,
                    if msg.len() > 300 {
                        tr!("job.done")
                    } else {
                        &msg
                    },
                );
            }
        } else {
            let msg = output.message();
            let rejected = msg.contains("rejected")
                || msg.contains("non-fast-forward")
                || msg.contains("fetch first");
            match (&job.post, rejected) {
                (
                    Post::Push {
                        remote,
                        branch,
                        target,
                    },
                    true,
                ) => {
                    let steps = vec![args(&[
                        "push",
                        "--force-with-lease",
                        remote,
                        &format!("{branch}:{target}"),
                    ])];
                    let mut force = job.clone();
                    force.steps = steps;
                    // Starts with "Push" in every language: the toolbar looks for it.
                    force.label = tr!("job.force_push_label", branch = branch);
                    force.post = Post::None;
                    self.dialog = Some(Dialog::Confirm {
                        title: tr!("job.push_rejected").into(),
                        text: tr!("job.push_rejected_text", message = msg),
                        ok_label: tr!("job.force_push").into(),
                        danger: true,
                        job: force,
                    });
                }
                _ => self.toast(
                    false,
                    &tr!("job.failed", label = job.label),
                    &friendly(&msg),
                ),
            }
        }

        if let Some(i) = tab_idx {
            let t = &mut self.tabs[i];
            t.reload();
            t.load_selection();
        }
        ctx.request_repaint();
    }
}

fn run_steps(job: &Job) -> GitOutput {
    let mut out = GitOutput {
        ok: true,
        stdout: String::new(),
        stderr: String::new(),
    };
    for step in &job.steps {
        let o = git::run_in(job.cwd.as_deref(), step);
        out.stdout += &o.stdout;
        out.stderr = o.stderr.clone();
        out.ok = o.ok;
        if !o.ok {
            out.stdout = o.stdout;
            break;
        }
    }
    out
}

/// Explain the most common git failures in plain words, keeping git's own text after.
/// The matches are on git's own (English) output, not on the app's texts.
fn friendly(msg: &str) -> String {
    let hint = if msg.contains("Please tell me who you are") || msg.contains("user.email") {
        tr!("job.hint.identity")
    } else if msg.contains("would be overwritten") {
        tr!("job.hint.overwritten")
    } else if msg.contains("CONFLICT") || msg.contains("conflict") {
        tr!("job.hint.conflict")
    } else if msg.contains("Authentication failed")
        || msg.contains("could not read Username")
        || msg.contains("Permission denied")
    {
        tr!("job.hint.auth")
    } else if msg.contains("not fully merged") {
        tr!("job.hint.not_merged")
    } else {
        return msg.to_string();
    };
    format!("{hint}\n\n{msg}")
}
