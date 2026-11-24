//! One open repository (a tab): its loaded data, selection, commit message being written,
//! undo history and view settings.

use super::jobs::Undoable;
use crate::git::{self, CommitDetails, DiffSource, FileDiff, FileEntry, RepoData};
use crate::graph::{self, Graph};
use crate::ui::graph::Hit;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

/// Commits loaded at once ("Load more commits…" loads this many more).
pub const PAGE_COMMITS: usize = 2000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    None,
    Wip,
    Commit(String),
    Stash(String),
}

/// Modification times of a few files in `.git`, to notice changes made outside the app.
#[derive(Default, Clone, PartialEq)]
struct Stamp(Vec<Option<SystemTime>>);

pub struct RepoTab {
    pub id: u64,
    /// Id of the workspace this tab belongs to.
    pub workspace: u64,
    pub path: PathBuf,
    pub name: String,
    pub data: Option<Rc<RepoData>>,
    pub graph: Graph,
    pub error: Option<String>,
    pub selection: Selection,
    pub details: Option<CommitDetails>,
    pub stash_files: Vec<FileEntry>,
    pub diff: Option<(FileDiff, DiffSource)>,
    pub summary: String,
    pub body: String,
    pub amend: bool,
    pub search: String,
    pub search_open: bool,
    pub search_focus: bool,
    pub busy: Option<String>,
    pub max_commits: usize,
    pub scroll_to_row: Option<usize>,
    pub scroll_offset: f32,
    pub view_height: f32,
    pub collapsed: HashSet<String>,
    pub sidebar_filter: String,
    pub undo: Vec<Undoable>,
    pub redo: Vec<Undoable>,
    pub col_refs: f32,
    pub col_graph: Option<f32>,
    pub col_author: f32,
    pub col_date: f32,
    pub ctx_target: Option<Hit>,
    stamp: Stamp,
}

impl RepoTab {
    pub fn new(id: u64, workspace: u64, path: PathBuf) -> RepoTab {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let mut tab = RepoTab {
            id,
            workspace,
            path,
            name,
            data: None,
            graph: Graph::default(),
            error: None,
            selection: Selection::None,
            details: None,
            stash_files: Vec::new(),
            diff: None,
            summary: String::new(),
            body: String::new(),
            amend: false,
            search: String::new(),
            search_open: false,
            search_focus: false,
            busy: None,
            max_commits: PAGE_COMMITS,
            scroll_to_row: None,
            scroll_offset: 0.0,
            view_height: 600.0,
            collapsed: HashSet::new(),
            sidebar_filter: String::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            col_refs: 170.0,
            col_graph: None,
            col_author: 130.0,
            col_date: 130.0,
            ctx_target: None,
            stamp: Stamp::default(),
        };
        tab.collapsed.insert("section:tags".into());
        tab.reload();
        if let Some(d) = &tab.data {
            tab.selection = if !d.status.is_clean() {
                Selection::Wip
            } else if let Some(h) = &d.head_hash {
                Selection::Commit(h.clone())
            } else {
                Selection::None
            };
            tab.scroll_to_row = Some(0);
        }
        tab.load_selection();
        tab
    }

    pub fn head_label(&self) -> String {
        match &self.data {
            Some(d) => match &d.head_branch {
                Some(b) => b.clone(),
                None => match &d.head_hash {
                    Some(h) => tr!("tab.detached_head", hash = &h[..7.min(h.len())]),
                    None => tr!("tab.no_commit").into(),
                },
            },
            None => String::new(),
        }
    }

    pub fn reload(&mut self) {
        match git::load(&self.path, self.max_commits) {
            Ok(d) => {
                self.graph = graph::layout(&d.commits);
                self.data = Some(Rc::new(d));
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
                self.data = None;
                self.graph = Graph::default();
            }
        }
        self.stamp = self.read_stamp();
        // Keep what was selected when it still exists.
        if let Some(d) = &self.data {
            let keep = match &self.selection {
                Selection::Wip => !d.status.is_clean(),
                Selection::Commit(h) => d.commits.iter().any(|c| &c.hash == h),
                Selection::Stash(h) => d.stashes.iter().any(|s| &s.hash == h),
                Selection::None => true,
            };
            if !keep {
                self.selection = match (&self.selection, &d.head_hash) {
                    (Selection::Wip, Some(h)) => Selection::Commit(h.clone()),
                    _ => Selection::None,
                };
                self.load_selection();
            }
        }
        self.refresh_diff();
    }

    fn read_stamp(&self) -> Stamp {
        let gd = self.path.join(".git");
        let files = [
            "HEAD",
            "index",
            "logs/HEAD",
            "FETCH_HEAD",
            "packed-refs",
            "refs/stash",
            "MERGE_HEAD",
        ];
        Stamp(
            files
                .iter()
                .map(|f| {
                    std::fs::metadata(gd.join(f))
                        .and_then(|m| m.modified())
                        .ok()
                })
                .collect(),
        )
    }

    pub fn changed_on_disk(&self) -> bool {
        self.read_stamp() != self.stamp
    }

    pub fn load_selection(&mut self) {
        self.details = None;
        self.stash_files.clear();
        match self.selection.clone() {
            Selection::Commit(h) => self.details = git::commit_details(&self.path, &h),
            Selection::Stash(h) => {
                if let Some(s) = self
                    .data
                    .as_ref()
                    .and_then(|d| d.stashes.iter().find(|s| s.hash == h))
                {
                    self.stash_files = git::stash_files(&self.path, &s.name);
                }
            }
            _ => {}
        }
    }

    /// Re-read the open diff of a work-in-progress file after staging or editing.
    fn refresh_diff(&mut self) {
        let Some((diff, source)) = &self.diff else {
            return;
        };
        let path = diff.path.clone();
        let source = source.clone();
        let status = self
            .data
            .as_ref()
            .map(|d| d.status.clone())
            .unwrap_or_default();
        let in_list = |l: &Vec<FileEntry>| l.iter().any(|f| f.path == path);
        let new_source = match source {
            DiffSource::Unstaged | DiffSource::Untracked => {
                if let Some(f) = status.unstaged.iter().find(|f| f.path == path) {
                    Some(if f.code == '?' {
                        DiffSource::Untracked
                    } else {
                        DiffSource::Unstaged
                    })
                } else if in_list(&status.staged) {
                    Some(DiffSource::Staged)
                } else {
                    None
                }
            }
            DiffSource::Staged => {
                if in_list(&status.staged) {
                    Some(DiffSource::Staged)
                } else if let Some(f) = status.unstaged.iter().find(|f| f.path == path) {
                    Some(if f.code == '?' {
                        DiffSource::Untracked
                    } else {
                        DiffSource::Unstaged
                    })
                } else {
                    None
                }
            }
            other => Some(other),
        };
        self.diff = new_source.map(|s| (git::file_diff(&self.path, &s, &path), s));
    }

    /// Index of a commit in the graph rows (the WIP row is not counted).
    pub fn commit_index(&self, hash: &str) -> Option<usize> {
        self.data
            .as_ref()?
            .commits
            .iter()
            .position(|c| c.hash == hash)
    }

    pub fn has_wip_row(&self) -> bool {
        self.data
            .as_ref()
            .map(|d| !d.status.is_clean())
            .unwrap_or(false)
    }
}
