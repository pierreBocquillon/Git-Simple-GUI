//! Plain data read from a repository: commits, refs, file status, stashes, remotes.

use std::collections::HashMap;

/// One commit as shown in the graph.
#[derive(Clone, Debug)]
pub struct Commit {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    /// Already formatted as dd/mm/yyyy hh:mm.
    pub date: String,
    pub subject: String,
}

impl Commit {
    pub fn short(&self) -> &str {
        &self.hash[..self.hash.len().min(7)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Local,
    Remote,
    Tag,
}

#[derive(Clone, Debug)]
pub struct RefInfo {
    /// Short name: `main`, `origin/main`, `v1.0`.
    pub name: String,
    pub kind: RefKind,
    /// The commit it points at (tags are peeled).
    pub target: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub upstream_gone: bool,
}

impl RefInfo {
    /// For a remote branch, `origin/feature/x` -> ("origin", "feature/x").
    pub fn remote_parts(&self) -> Option<(&str, &str)> {
        if self.kind != RefKind::Remote {
            return None;
        }
        self.name.split_once('/')
    }
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub orig_path: Option<String>,
    /// One of M A D R C T U ? (U = conflict, ? = new untracked file)
    pub code: char,
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub staged: Vec<FileEntry>,
    pub unstaged: Vec<FileEntry>,
    pub conflicted: Vec<FileEntry>,
}

impl Status {
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty() && self.unstaged.is_empty() && self.conflicted.is_empty()
    }
    pub fn total(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.conflicted.len()
    }
}

#[derive(Clone, Debug)]
pub struct Stash {
    pub name: String,
    pub message: String,
    pub hash: String,
}

#[derive(Clone, Debug)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoState {
    Clean,
    Merging,
    Rebasing,
    CherryPicking,
    Reverting,
}

#[derive(Clone, Debug)]
pub struct RepoData {
    pub head_branch: Option<String>,
    pub head_hash: Option<String>,
    pub commits: Vec<Commit>,
    pub truncated: bool,
    pub refs: Vec<RefInfo>,
    pub status: Status,
    pub stashes: Vec<Stash>,
    pub remotes: Vec<Remote>,
    pub state: RepoState,
    /// commit hash -> indexes into `refs`
    pub refs_by_commit: HashMap<String, Vec<usize>>,
}

impl RepoData {
    pub fn head_ref(&self) -> Option<&RefInfo> {
        let name = self.head_branch.as_ref()?;
        self.refs
            .iter()
            .find(|r| r.kind == RefKind::Local && &r.name == name)
    }
    pub fn locals(&self) -> impl Iterator<Item = &RefInfo> {
        self.refs.iter().filter(|r| r.kind == RefKind::Local)
    }
    pub fn remotes_refs(&self) -> impl Iterator<Item = &RefInfo> {
        self.refs.iter().filter(|r| r.kind == RefKind::Remote)
    }
}

#[derive(Clone, Debug)]
pub struct CommitDetails {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub author_date: String,
    pub committer: String,
    pub commit_date: String,
    pub message: String,
    pub files: Vec<FileEntry>,
    pub additions: u32,
    pub deletions: u32,
}
