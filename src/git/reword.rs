//! Change the message of a commit of the current branch, even an old one.
//!
//! Only messages change, never files: the commit is written again with its new message, then
//! every commit after it (up to HEAD) is written again on top of the new one, with the same
//! files, author and dates. Finally the branch is moved to the new last commit. Since the files
//! are identical, this cannot conflict and leaves the working folder and the index alone.

use super::command::git_bytes;
use std::collections::HashMap;
use std::path::Path;

/// Give `hash` the message `message`. Returns the new ID of that commit.
pub fn reword(repo: &Path, hash: &str, message: &str) -> Result<String, String> {
    let text = |args: &[&str]| -> Result<String, String> {
        git_bytes(repo, args, None).map(|o| String::from_utf8_lossy(&o).trim().to_string())
    };
    let head = text(&["rev-parse", "--verify", "HEAD"])?;
    let target = text(&["rev-parse", "--verify", &format!("{hash}^{{commit}}")])?;
    // The branch HEAD is on, or HEAD itself when it is detached.
    let reference = text(&["symbolic-ref", "-q", "HEAD"]).unwrap_or_else(|_| "HEAD".into());
    if git_bytes(repo, &["merge-base", "--is-ancestor", &target, &head], None).is_err() {
        return Err(tr!("git.reword.not_ancestor").into());
    }
    // The commits after the target, parents first.
    let after = text(&[
        "rev-list",
        "--reverse",
        "--topo-order",
        "--ancestry-path",
        &format!("{target}..{head}"),
    ])?;

    let mut rewritten: HashMap<String, String> = HashMap::new();
    let write = |old: &str, msg: Option<&str>, map: &HashMap<String, String>| {
        let raw = git_bytes(repo, &["cat-file", "commit", old], None)?;
        let object = rebuild(&raw, map, msg);
        let new = git_bytes(
            repo,
            &["hash-object", "-t", "commit", "-w", "--stdin"],
            Some(&object),
        )?;
        Ok::<String, String>(String::from_utf8_lossy(&new).trim().to_string())
    };
    let new_target = write(&target, Some(message), &rewritten)?;
    rewritten.insert(target.clone(), new_target.clone());
    for old in after.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let new = write(old, None, &rewritten)?;
        rewritten.insert(old.to_string(), new);
    }
    let new_head = rewritten.get(&head).cloned().unwrap_or(head.clone());
    // Only if HEAD did not move meanwhile.
    git_bytes(
        repo,
        &[
            "update-ref",
            "-m",
            "Git Simple GUI: edit commit message",
            &reference,
            &new_head,
            &head,
        ],
        None,
    )?;
    Ok(new_target)
}

/// A commit object with its parents replaced through `map` and, when given, a new message.
/// Signatures are dropped: they would no longer match.
fn rebuild(raw: &[u8], map: &HashMap<String, String>, message: Option<&str>) -> Vec<u8> {
    let split = raw
        .windows(2)
        .position(|w| w == b"\n\n")
        .unwrap_or(raw.len());
    let (headers, body) = (&raw[..split], raw.get(split + 2..).unwrap_or(&[]));
    let mut out = Vec::with_capacity(raw.len());
    let mut skipping = false;
    for line in headers.split(|&b| b == b'\n') {
        // A line starting with a space continues the previous header (signatures, mergetag).
        if line.first() == Some(&b' ') {
            if !skipping {
                out.extend_from_slice(line);
                out.push(b'\n');
            }
            continue;
        }
        skipping = line.starts_with(b"gpgsig ")
            || line.starts_with(b"gpgsig-sha256 ")
            || (message.is_some() && line.starts_with(b"encoding "));
        if skipping {
            continue;
        }
        if let Some(parent) = line.strip_prefix(b"parent ") {
            let parent = String::from_utf8_lossy(parent).to_string();
            let parent = map.get(&parent).unwrap_or(&parent);
            out.extend_from_slice(format!("parent {parent}\n").as_bytes());
        } else {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
    }
    out.push(b'\n');
    match message {
        Some(m) => {
            out.extend_from_slice(m.trim_end().as_bytes());
            out.push(b'\n');
        }
        None => out.extend_from_slice(body),
    }
    out
}

/// "Summary" and "Description" back into one commit message.
pub fn join_message(summary: &str, body: &str) -> String {
    let (summary, body) = (summary.trim(), body.trim());
    if body.is_empty() {
        summary.to_string()
    } else {
        format!("{summary}\n\n{body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &[u8] = b"tree 1111\nparent aaaa\nparent bbbb\nauthor A <a@x> 1 +0100\ncommitter A <a@x> 1 +0100\nencoding ISO-8859-1\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n abc\n -----END PGP SIGNATURE-----\n\nOld subject\n\nOld body\n";

    #[test]
    fn keeps_everything_but_parents_message_and_signature() {
        let map = HashMap::from([("aaaa".to_string(), "cccc".to_string())]);
        let same = rebuild(RAW, &map, None);
        assert_eq!(
            String::from_utf8(same).unwrap(),
            "tree 1111\nparent cccc\nparent bbbb\nauthor A <a@x> 1 +0100\ncommitter A <a@x> 1 +0100\nencoding ISO-8859-1\n\nOld subject\n\nOld body\n"
        );
        let new = rebuild(RAW, &HashMap::new(), Some("New\n\nText\n\n"));
        assert_eq!(
            String::from_utf8(new).unwrap(),
            "tree 1111\nparent aaaa\nparent bbbb\nauthor A <a@x> 1 +0100\ncommitter A <a@x> 1 +0100\n\nNew\n\nText\n"
        );
    }

    /// A real repository: a1 - a2 - a3 (merge of side) - a4, with uncommitted changes.
    /// Rewording a2 keeps every file, the merge, the side branch and the working folder.
    #[test]
    fn rewords_an_old_commit_in_a_real_repository() {
        let dir = std::env::temp_dir().join(format!("gsg-reword-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let o = crate::git::git(&dir, args);
            assert!(o.ok, "git {args:?}: {}", o.stderr);
            o.stdout.trim().to_string()
        };
        let commit = |file: &str, msg: &str| {
            std::fs::write(dir.join(file), msg).unwrap();
            git(&["add", "-A"]);
            git(&["commit", "-q", "-m", msg]);
            git(&["rev-parse", "HEAD"])
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.name", "Test"]);
        git(&["config", "user.email", "test@example.com"]);
        commit("a.txt", "a1");
        let a2 = commit("a.txt", "a2");
        git(&["checkout", "-q", "-b", "side"]);
        let side = commit("side.txt", "side");
        git(&["checkout", "-q", "main"]);
        git(&["merge", "-q", "--no-ff", "-m", "a3 merge", "side"]);
        commit("a.txt", "a4");
        std::fs::write(dir.join("a.txt"), "uncommitted").unwrap();
        let tree_before = git(&["rev-parse", "HEAD^{tree}"]);

        let new_a2 = reword(&dir, &a2, "a2 fixed\n\nwith details").unwrap();

        assert_ne!(new_a2, a2);
        assert_eq!(
            git(&["log", "-1", "--format=%B", &new_a2]),
            "a2 fixed\n\nwith details"
        );
        assert_eq!(git(&["rev-parse", "HEAD^{tree}"]), tree_before);
        assert_eq!(git(&["rev-parse", "HEAD~2"]), new_a2);
        // The merged side commit started from a2, so it is written again on the new a2; the
        // `side` branch label itself stays where it was.
        assert_ne!(git(&["rev-parse", "HEAD~1^2"]), side);
        assert_eq!(git(&["rev-parse", "HEAD~1^2^"]), new_a2);
        assert_eq!(git(&["rev-parse", "side"]), side);
        assert_eq!(git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
        assert_eq!(
            git(&["log", "--topo-order", "--format=%s", "main"]),
            "a4\na3 merge\nside\na2 fixed\na1"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "uncommitted"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn joins_summary_and_body() {
        assert_eq!(join_message(" Fix ", ""), "Fix");
        assert_eq!(join_message("Fix", " details \n"), "Fix\n\ndetails");
    }
}
