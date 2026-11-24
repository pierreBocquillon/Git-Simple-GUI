//! Ready-made git jobs shared by several parts of the interface
//! (checkout, stage, unstage, discard, continue/abort a merge or rebase).

use super::{args, Job, Post, RepoTab, Undoable};
use crate::git::{RefKind, RepoState};

pub fn prev_head(tab: &RepoTab) -> Option<String> {
    let d = tab.data.as_ref()?;
    d.head_branch.clone().or_else(|| d.head_hash.clone())
}

pub fn checkout_job(tab: &RepoTab, target: &str, label: String) -> Job {
    let mut job = Job::new(tab, label, vec![args(&["checkout", target])]);
    if let Some(prev) = prev_head(tab) {
        job = job.post(Post::Undo(Undoable {
            label: format!("checkout {target}"),
            undo: vec![args(&["checkout", &prev])],
            redo: vec![args(&["checkout", target])],
        }));
    }
    job
}

/// Checking out a label: a local branch directly, a remote one through a local tracking branch.
pub fn checkout_ref_job(tab: &RepoTab, k: usize) -> Option<Job> {
    let d = tab.data.as_ref()?;
    let r = d.refs.get(k)?;
    match r.kind {
        RefKind::Local => {
            if d.head_branch.as_deref() == Some(r.name.as_str()) {
                return None;
            }
            Some(checkout_job(
                tab,
                &r.name,
                tr!("cmd.checkout_branch", branch = r.name),
            ))
        }
        RefKind::Remote => {
            let (_, branch) = r.remote_parts()?;
            if d.locals().any(|l| l.name == branch) {
                return Some(checkout_job(
                    tab,
                    branch,
                    tr!("cmd.checkout_branch", branch = branch),
                ));
            }
            let mut job = Job::new(
                tab,
                tr!("cmd.checkout_branch", branch = branch),
                vec![args(&["checkout", "-b", branch, "--track", &r.name])],
            );
            if let Some(prev) = prev_head(tab) {
                job = job.post(Post::Undo(Undoable {
                    label: format!("checkout {branch}"),
                    undo: vec![args(&["checkout", &prev]), args(&["branch", "-D", branch])],
                    redo: vec![args(&["checkout", "-b", branch, "--track", &r.name])],
                }));
            }
            Some(job)
        }
        RefKind::Tag => Some(checkout_job(
            tab,
            &r.name,
            tr!("cmd.checkout_tag", tag = r.name),
        )),
    }
}

pub fn stage_job(tab: &RepoTab, paths: &[String]) -> Job {
    let mut a = args(&["add", "-A", "--"]);
    a.extend(paths.iter().cloned());
    Job::new(tab, tr!("cmd.stage"), vec![a]).quick()
}

pub fn unstage_job(tab: &RepoTab, paths: &[String]) -> Job {
    let has_head = tab
        .data
        .as_ref()
        .map(|d| d.head_hash.is_some())
        .unwrap_or(false);
    let mut a = if has_head {
        args(&["reset", "-q", "HEAD", "--"])
    } else {
        args(&["rm", "--cached", "-q", "-r", "--"])
    };
    a.extend(paths.iter().cloned());
    Job::new(tab, tr!("cmd.unstage"), vec![a]).quick()
}

pub fn discard_job(tab: &RepoTab, path: &str, untracked: bool) -> Job {
    let step = if untracked {
        args(&["clean", "-f", "--", path])
    } else {
        args(&["checkout", "--", path])
    };
    Job::new(tab, tr!("cmd.discard", path = path), vec![step])
}

/// Merge / rebase / cherry-pick in progress: the matching continue and abort commands.
pub fn state_commands(state: RepoState) -> Option<(&'static str, Vec<String>, Vec<String>)> {
    match state {
        RepoState::Merging => Some((
            tr!("cmd.state.merging"),
            args(&["commit", "--no-edit"]),
            args(&["merge", "--abort"]),
        )),
        RepoState::Rebasing => Some((
            tr!("cmd.state.rebasing"),
            args(&["rebase", "--continue"]),
            args(&["rebase", "--abort"]),
        )),
        RepoState::CherryPicking => Some((
            tr!("cmd.state.cherry_picking"),
            args(&["cherry-pick", "--continue"]),
            args(&["cherry-pick", "--abort"]),
        )),
        RepoState::Reverting => Some((
            tr!("cmd.state.reverting"),
            args(&["revert", "--continue"]),
            args(&["revert", "--abort"]),
        )),
        RepoState::Clean => None,
    }
}
