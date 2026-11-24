//! Diff of one file (from a commit, the working copy, the index or a stash) and its parser.

use super::command::git;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
    Hunk,
    Meta,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Clone, Debug)]
pub enum DiffSource {
    Commit {
        hash: String,
        parent: Option<String>,
    },
    Unstaged,
    Staged,
    Untracked,
    Stash {
        name: String,
    },
}

#[derive(Clone, Debug)]
pub struct FileDiff {
    pub path: String,
    pub title: String,
    pub lines: Vec<DiffLine>,
    pub binary: bool,
    pub added: u32,
    pub removed: u32,
}

pub fn file_diff(repo: &Path, source: &DiffSource, path: &str) -> FileDiff {
    let out = match source {
        DiffSource::Commit {
            hash,
            parent: Some(p),
        } => git(repo, &["diff", "-M", p, hash, "--", path]),
        DiffSource::Commit { hash, parent: None } => {
            git(repo, &["show", "--format=", "--root", hash, "--", path])
        }
        DiffSource::Unstaged => git(repo, &["diff", "--", path]),
        DiffSource::Staged => git(repo, &["diff", "--cached", "-M", "--", path]),
        DiffSource::Untracked => git(repo, &["diff", "--no-index", "--", "/dev/null", path]),
        DiffSource::Stash { name } => git(repo, &["stash", "show", "-p", name, "--", path]),
    };
    let title = match source {
        DiffSource::Commit { hash, .. } => {
            tr!("git.diff.commit", hash = &hash[..hash.len().min(7)])
        }
        DiffSource::Unstaged => tr!("git.diff.unstaged").into(),
        DiffSource::Staged => tr!("git.diff.staged").into(),
        DiffSource::Untracked => tr!("git.diff.untracked").into(),
        DiffSource::Stash { name } => name.clone(),
    };
    parse_diff(path, title, &out.stdout)
}

pub fn parse_diff(path: &str, title: String, text: &str) -> FileDiff {
    let mut lines = Vec::new();
    let mut binary = false;
    let (mut old_no, mut new_no) = (0u32, 0u32);
    let (mut added, mut removed) = (0, 0);
    let mut in_hunk = false;
    for raw in text.lines() {
        if raw.starts_with("Binary files") {
            binary = true;
            continue;
        }
        if raw.starts_with("@@") {
            in_hunk = true;
            // @@ -a,b +c,d @@ heading
            let mut parts = raw.split_whitespace().skip(1);
            let old = parts.next().unwrap_or("-0");
            let new = parts.next().unwrap_or("+0");
            old_no = old
                .trim_start_matches('-')
                .split(',')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            new_no = new
                .trim_start_matches('+')
                .split(',')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            lines.push(DiffLine {
                kind: LineKind::Hunk,
                old_no: None,
                new_no: None,
                text: raw.to_string(),
            });
            continue;
        }
        if !in_hunk {
            continue;
        }
        if let Some(rest) = raw.strip_prefix('+') {
            lines.push(DiffLine {
                kind: LineKind::Added,
                old_no: None,
                new_no: Some(new_no),
                text: rest.to_string(),
            });
            new_no += 1;
            added += 1;
        } else if let Some(rest) = raw.strip_prefix('-') {
            lines.push(DiffLine {
                kind: LineKind::Removed,
                old_no: Some(old_no),
                new_no: None,
                text: rest.to_string(),
            });
            old_no += 1;
            removed += 1;
        } else if let Some(rest) = raw.strip_prefix(' ') {
            lines.push(DiffLine {
                kind: LineKind::Context,
                old_no: Some(old_no),
                new_no: Some(new_no),
                text: rest.to_string(),
            });
            old_no += 1;
            new_no += 1;
        } else if raw.starts_with('\\') {
            lines.push(DiffLine {
                kind: LineKind::Meta,
                old_no: None,
                new_no: None,
                text: raw.to_string(),
            });
        } else if raw.starts_with("diff --git") {
            in_hunk = false;
        }
    }
    FileDiff {
        path: path.to_string(),
        title,
        lines,
        binary,
        added,
        removed,
    }
}
