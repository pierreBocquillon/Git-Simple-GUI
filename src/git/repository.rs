//! Reading a repository: commits, refs, status, stashes, remotes and commit details.

use super::command::git;
use super::model::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn repo_root(path: &Path) -> Option<PathBuf> {
    let out = git(path, &["rev-parse", "--show-toplevel"]);
    if !out.ok {
        return None;
    }
    let p = out.stdout.trim();
    if p.is_empty() {
        None
    } else {
        Some(PathBuf::from(p))
    }
}

fn git_dir(repo: &Path) -> PathBuf {
    let out = git(repo, &["rev-parse", "--absolute-git-dir"]);
    PathBuf::from(out.stdout.trim())
}

pub fn load(repo: &Path, max_commits: usize) -> Result<RepoData, String> {
    let check = git(repo, &["rev-parse", "--is-inside-work-tree"]);
    if !check.ok {
        return Err(tr!("git.not_a_repo_folder", details = check.message()));
    }

    let head_hash = {
        let o = git(repo, &["rev-parse", "--verify", "-q", "HEAD"]);
        if o.ok {
            Some(o.stdout.trim().to_string())
        } else {
            None
        }
    };
    let head_branch = {
        let o = git(repo, &["symbolic-ref", "-q", "--short", "HEAD"]);
        if o.ok {
            Some(o.stdout.trim().to_string())
        } else {
            None
        }
    };

    let refs = load_refs(repo);
    let mut refs_by_commit: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, r) in refs.iter().enumerate() {
        refs_by_commit.entry(r.target.clone()).or_default().push(i);
    }

    let (commits, truncated) = if head_hash.is_some() || !refs.is_empty() {
        load_commits(repo, max_commits, head_hash.is_some())
    } else {
        (Vec::new(), false)
    };

    let gd = git_dir(repo);
    let state = if gd.join("rebase-merge").exists() || gd.join("rebase-apply").exists() {
        RepoState::Rebasing
    } else if gd.join("MERGE_HEAD").exists() {
        RepoState::Merging
    } else if gd.join("CHERRY_PICK_HEAD").exists() {
        RepoState::CherryPicking
    } else if gd.join("REVERT_HEAD").exists() {
        RepoState::Reverting
    } else {
        RepoState::Clean
    };

    Ok(RepoData {
        head_branch,
        head_hash,
        commits,
        truncated,
        refs,
        status: load_status(repo),
        stashes: load_stashes(repo),
        remotes: load_remotes(repo),
        state,
        refs_by_commit,
    })
}

fn load_commits(repo: &Path, max: usize, with_head: bool) -> (Vec<Commit>, bool) {
    let n = format!("-n{}", max + 1);
    let mut args = vec![
        "log",
        "--date-order",
        n.as_str(),
        "--date=format-local:%d/%m/%Y %H:%M",
        "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%ad%x1f%s%x1e",
        "--branches",
        "--remotes",
        "--tags",
    ];
    if with_head {
        args.push("HEAD");
    }
    let out = git(repo, &args);
    let mut commits: Vec<Commit> = out
        .stdout
        .split('\x1e')
        .filter_map(|rec| {
            let rec = rec.trim_start_matches(['\n', '\r']);
            if rec.is_empty() {
                return None;
            }
            let f: Vec<&str> = rec.split('\x1f').collect();
            if f.len() < 6 {
                return None;
            }
            Some(Commit {
                hash: f[0].to_string(),
                parents: f[1].split_whitespace().map(|s| s.to_string()).collect(),
                author: f[2].to_string(),
                email: f[3].to_string(),
                date: f[4].to_string(),
                subject: f[5].trim_end().to_string(),
            })
        })
        .collect();
    let truncated = commits.len() > max;
    commits.truncate(max);
    (commits, truncated)
}

fn load_refs(repo: &Path) -> Vec<RefInfo> {
    let out = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)%1f%(objectname)%1f%(*objectname)%1f%(upstream:short)%1f%(upstream:track,nobracket)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
    );
    let mut refs = Vec::new();
    for line in out.stdout.lines() {
        let f: Vec<&str> = line.split('\x1f').collect();
        if f.len() < 5 {
            continue;
        }
        let full = f[0];
        let (kind, name) = if let Some(n) = full.strip_prefix("refs/heads/") {
            (RefKind::Local, n)
        } else if let Some(n) = full.strip_prefix("refs/remotes/") {
            if n.ends_with("/HEAD") {
                continue;
            }
            (RefKind::Remote, n)
        } else if let Some(n) = full.strip_prefix("refs/tags/") {
            (RefKind::Tag, n)
        } else {
            continue;
        };
        let target = if !f[2].is_empty() { f[2] } else { f[1] };
        let (mut ahead, mut behind, mut gone) = (0, 0, false);
        for part in f[4].split(',') {
            let part = part.trim();
            if let Some(n) = part.strip_prefix("ahead ") {
                ahead = n.parse().unwrap_or(0);
            } else if let Some(n) = part.strip_prefix("behind ") {
                behind = n.parse().unwrap_or(0);
            } else if part == "gone" {
                gone = true;
            }
        }
        refs.push(RefInfo {
            name: name.to_string(),
            kind,
            target: target.to_string(),
            upstream: if f[3].is_empty() {
                None
            } else {
                Some(f[3].to_string())
            },
            ahead,
            behind,
            upstream_gone: gone,
        });
    }
    refs
}

pub fn load_status(repo: &Path) -> Status {
    let out = git(repo, &["status", "--porcelain=v1", "-z", "-uall"]);
    let mut st = Status::default();
    let mut parts = out.stdout.split('\0');
    while let Some(entry) = parts.next() {
        if entry.len() < 4 {
            continue;
        }
        let bytes = entry.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let path = entry[3..].to_string();
        let orig = if x == 'R' || x == 'C' {
            parts.next().map(|s| s.to_string())
        } else {
            None
        };
        let conflict = matches!(
            (x, y),
            ('D', 'D')
                | ('A', 'U')
                | ('U', 'D')
                | ('U', 'A')
                | ('D', 'U')
                | ('A', 'A')
                | ('U', 'U')
        );
        if conflict {
            st.conflicted.push(FileEntry {
                path,
                orig_path: None,
                code: 'U',
            });
            continue;
        }
        if x == '?' {
            st.unstaged.push(FileEntry {
                path,
                orig_path: None,
                code: '?',
            });
            continue;
        }
        if x == '!' {
            continue;
        }
        if x != ' ' {
            st.staged.push(FileEntry {
                path: path.clone(),
                orig_path: orig.clone(),
                code: x,
            });
        }
        if y != ' ' {
            st.unstaged.push(FileEntry {
                path,
                orig_path: None,
                code: y,
            });
        }
    }
    st
}

fn load_stashes(repo: &Path) -> Vec<Stash> {
    let out = git(repo, &["stash", "list", "--format=%gd%x1f%H%x1f%gs"]);
    out.stdout
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\x1f').collect();
            if f.len() < 3 {
                return None;
            }
            Some(Stash {
                name: f[0].to_string(),
                hash: f[1].to_string(),
                message: f[2].to_string(),
            })
        })
        .collect()
}

fn load_remotes(repo: &Path) -> Vec<Remote> {
    let out = git(repo, &["remote", "-v"]);
    let mut remotes: Vec<Remote> = Vec::new();
    for line in out.stdout.lines() {
        let mut it = line.split_whitespace();
        let (Some(name), Some(url), kind) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        let idx = match remotes.iter().position(|r| r.name == name) {
            Some(i) => i,
            None => {
                remotes.push(Remote {
                    name: name.into(),
                    fetch_url: String::new(),
                    push_url: String::new(),
                });
                remotes.len() - 1
            }
        };
        if kind == Some("(push)") {
            remotes[idx].push_url = url.into();
        } else {
            remotes[idx].fetch_url = url.into();
        }
    }
    remotes
}

pub fn commit_details(repo: &Path, hash: &str) -> Option<CommitDetails> {
    let out = git(
        repo,
        &[
            "show",
            "-s",
            "--date=format-local:%d/%m/%Y %H:%M",
            "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%ad%x1f%cn%x1f%cd%x1f%B",
            hash,
        ],
    );
    if !out.ok {
        return None;
    }
    let f: Vec<&str> = out.stdout.splitn(8, '\x1f').collect();
    if f.len() < 8 {
        return None;
    }
    let parents: Vec<String> = f[1].split_whitespace().map(String::from).collect();
    let (files_out, stat_out) = if parents.is_empty() {
        (
            git(
                repo,
                &[
                    "diff-tree",
                    "--root",
                    "--no-commit-id",
                    "-r",
                    "-M",
                    "--name-status",
                    "-z",
                    hash,
                ],
            ),
            git(
                repo,
                &[
                    "diff-tree",
                    "--root",
                    "--no-commit-id",
                    "-r",
                    "-M",
                    "--numstat",
                    hash,
                ],
            ),
        )
    } else {
        let base = format!("{hash}^1");
        (
            git(repo, &["diff", "--name-status", "-M", "-z", &base, hash]),
            git(repo, &["diff", "--numstat", "-M", &base, hash]),
        )
    };
    let files = parse_name_status_z(&files_out.stdout);
    let (mut additions, mut deletions) = (0, 0);
    for line in stat_out.stdout.lines() {
        let mut it = line.split('\t');
        additions += it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        deletions += it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    }
    Some(CommitDetails {
        hash: f[0].to_string(),
        parents,
        author: f[2].to_string(),
        email: f[3].to_string(),
        author_date: f[4].to_string(),
        committer: f[5].to_string(),
        commit_date: f[6].to_string(),
        message: f[7].trim().to_string(),
        files,
        additions,
        deletions,
    })
}

fn parse_name_status_z(s: &str) -> Vec<FileEntry> {
    let mut files = Vec::new();
    let mut it = s.split('\0').filter(|p| !p.is_empty());
    while let Some(status) = it.next() {
        let code = status.chars().next().unwrap_or('M');
        if code == 'R' || code == 'C' {
            let old = it.next().unwrap_or_default().to_string();
            let new = it.next().unwrap_or_default().to_string();
            files.push(FileEntry {
                path: new,
                orig_path: Some(old),
                code,
            });
        } else if let Some(path) = it.next() {
            files.push(FileEntry {
                path: path.to_string(),
                orig_path: None,
                code,
            });
        }
    }
    files
}

pub fn stash_files(repo: &Path, name: &str) -> Vec<FileEntry> {
    let out = git(repo, &["stash", "show", "--name-status", "-z", name]);
    parse_name_status_z(&out.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{file_diff, DiffSource, LineKind};

    fn run(repo: &Path, a: &[&str]) {
        let o = git(repo, a);
        assert!(o.ok, "git {:?} failed: {}", a, o.stderr);
    }

    #[test]
    fn reads_a_real_repository() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-repo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        run(&dir, &["init", "-q"]);
        run(&dir, &["config", "user.name", "Test Person"]);
        run(&dir, &["config", "user.email", "t@example.com"]);
        run(&dir, &["checkout", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        run(&dir, &["add", "-A"]);
        run(&dir, &["commit", "-q", "-m", "First", "-m", "Body text"]);
        run(&dir, &["branch", "feature/x"]);
        run(&dir, &["tag", "-a", "v1", "-m", "v1"]);
        std::fs::write(dir.join("a.txt"), "one\ntwo\n").unwrap();
        std::fs::write(dir.join("new file.txt"), "n\n").unwrap();
        std::fs::write(dir.join("b.txt"), "b\n").unwrap();
        run(&dir, &["add", "b.txt"]);

        let d = load(&dir, 100).unwrap();
        assert_eq!(d.head_branch.as_deref(), Some("main"));
        assert_eq!(d.commits.len(), 1);
        assert_eq!(d.commits[0].subject, "First");
        assert!(d
            .refs
            .iter()
            .any(|r| r.kind == RefKind::Local && r.name == "feature/x"));
        let tag = d.refs.iter().find(|r| r.kind == RefKind::Tag).unwrap();
        assert_eq!(
            tag.target, d.commits[0].hash,
            "annotated tags point at their commit"
        );
        assert_eq!(d.status.staged.len(), 1);
        assert_eq!(d.status.staged[0].code, 'A');
        assert!(d
            .status
            .unstaged
            .iter()
            .any(|f| f.path == "a.txt" && f.code == 'M'));
        assert!(d
            .status
            .unstaged
            .iter()
            .any(|f| f.path == "new file.txt" && f.code == '?'));

        let c = commit_details(&dir, &d.commits[0].hash).unwrap();
        assert_eq!(c.message, "First\n\nBody text");
        assert_eq!(c.files.len(), 1);
        assert_eq!(c.additions, 1);

        let diff = file_diff(&dir, &DiffSource::Unstaged, "a.txt");
        assert_eq!(diff.added, 1);
        assert!(diff
            .lines
            .iter()
            .any(|l| l.kind == LineKind::Added && l.text == "two"));
        let diff = file_diff(&dir, &DiffSource::Untracked, "new file.txt");
        assert_eq!(diff.added, 1);

        run(&dir, &["stash", "push", "-u", "-q", "-m", "keep"]);
        let d = load(&dir, 100).unwrap();
        assert!(d.status.is_clean());
        assert_eq!(d.stashes.len(), 1);
        assert_eq!(stash_files(&dir, &d.stashes[0].name).len(), 2);
    }
}
