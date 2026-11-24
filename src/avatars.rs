//! Profile pictures of commit authors, taken from GitHub when they can be found.
//!
//! Where a picture comes from, in order:
//! 1. a GitHub "noreply" e-mail (`12345+login@users.noreply.github.com`) names the account
//!    directly, no API call needed;
//! 2. otherwise, when the repository's remote is on github.com, the GitHub API is asked which
//!    account authored one of that person's pushed commits. Without a GitHub account this
//!    only works for public repositories (60 requests per hour); with an account connected
//!    in the app (see `github`), private ones too (5000 per hour). Answers are kept on disk.
//!
//! Anyone else keeps their initials. Downloads run on one background thread; pictures and
//! "no account" answers are cached in %LOCALAPPDATA%\GitSimpleGUI\avatars (a "no account"
//! answer is asked again after a week).

use crate::git::{self, RepoData};
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, SystemTime};

/// Pictures are stored and drawn at this size (pixels).
const SIZE: u32 = 64;
/// A "no account" answer is asked again after this long.
const RETRY_AFTER: Duration = Duration::from_secs(7 * 24 * 3600);

/// Everything the background thread needs to find one person's picture.
struct Request {
    email: String,
    /// The local repository, to find a commit by that person.
    repo_path: PathBuf,
    /// "owner/name" when the repository is on GitHub.
    github_repo: Option<String>,
}

/// Messages to the background thread.
enum ToWorker {
    Find(Request),
    /// The GitHub account changed (connected or disconnected).
    Token(Option<String>),
}

enum Slot {
    Loading,
    Ready(TextureHandle),
    Missing,
}

pub struct Avatars {
    ctx: Context,
    /// Keyed by lower-case e-mail.
    slots: RefCell<HashMap<String, Slot>>,
    to_worker: Sender<ToWorker>,
    from_worker: Receiver<(String, Option<ColorImage>)>,
}

impl Avatars {
    /// `token`: the GitHub token to ask the API with, if an account is connected.
    pub fn new(ctx: &Context, token: Option<String>) -> Self {
        let (to_worker, requests) = channel();
        let (results, from_worker) = channel();
        let worker_ctx = ctx.clone();
        std::thread::spawn(move || worker(requests, results, worker_ctx, token));
        Avatars {
            ctx: ctx.clone(),
            slots: RefCell::new(HashMap::new()),
            to_worker,
            from_worker,
        }
    }

    /// The picture for this author, or `None` while it loads or when there is none (draw the
    /// initials then). `repo_path` is the repository the author was seen in, `github_repo`
    /// its "owner/name" when it is on GitHub.
    pub fn get(
        &self,
        email: &str,
        repo_path: &Path,
        github_repo: Option<&str>,
    ) -> Option<TextureHandle> {
        let key = email.trim().to_lowercase();
        if key.is_empty() {
            return None;
        }
        self.receive();
        let mut slots = self.slots.borrow_mut();
        match slots.get(&key) {
            Some(Slot::Ready(t)) => Some(t.clone()),
            Some(_) => None,
            None => {
                slots.insert(key.clone(), Slot::Loading);
                let _ = self.to_worker.send(ToWorker::Find(Request {
                    email: key,
                    repo_path: repo_path.to_path_buf(),
                    github_repo: github_repo.map(str::to_string),
                }));
                None
            }
        }
    }

    /// Use another GitHub account (or none) from now on. Authors without a picture are
    /// looked up again, since the new account may see more repositories.
    pub fn set_token(&self, token: Option<String>) {
        let _ = self.to_worker.send(ToWorker::Token(token));
        self.receive();
        self.slots
            .borrow_mut()
            .retain(|_, s| !matches!(s, Slot::Missing));
    }

    /// Turn the pictures the background thread finished into textures.
    fn receive(&self) {
        let mut slots = self.slots.borrow_mut();
        while let Ok((key, image)) = self.from_worker.try_recv() {
            let slot = match image {
                Some(img) => Slot::Ready(self.ctx.load_texture(
                    format!("avatar:{key}"),
                    img,
                    TextureOptions::LINEAR,
                )),
                None => Slot::Missing,
            };
            slots.insert(key, slot);
        }
    }
}

/// "owner/name" of the repository's GitHub remote ("origin" first), if it has one.
pub fn github_repo(data: &RepoData) -> Option<String> {
    let mut remotes: Vec<_> = data.remotes.iter().collect();
    remotes.sort_by_key(|r| r.name != "origin");
    remotes.iter().find_map(|r| parse_github_url(&r.fetch_url))
}

/// `https://github.com/o/r.git`, `git@github.com:o/r.git`, `ssh://git@github.com/o/r` → "o/r".
fn parse_github_url(url: &str) -> Option<String> {
    let at = url.find("github.com")?;
    let rest = url[at + "github.com".len()..].trim_start_matches([':', '/']);
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = rest.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    (parts.next().is_none() && !owner.is_empty() && !name.is_empty())
        .then(|| format!("{owner}/{name}"))
}

// ---------------------------------------------------------------------------------------
// Background thread.

fn worker(
    messages: Receiver<ToWorker>,
    results: Sender<(String, Option<ColorImage>)>,
    ctx: Context,
    token: Option<String>,
) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent("git-simple-gui")
        .build();
    let dir = cache_dir();
    if let Some(d) = &dir {
        let _ = std::fs::create_dir_all(d);
    }
    let mut state = WorkerState {
        token,
        ..Default::default()
    };
    for msg in messages {
        let req = match msg {
            ToWorker::Find(req) => req,
            ToWorker::Token(token) => {
                // A new account: forget what the previous one could not see.
                state = WorkerState {
                    token,
                    ..Default::default()
                };
                continue;
            }
        };
        let image = find(&agent, dir.as_deref(), &req, &mut state);
        if results.send((req.email, image)).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

#[derive(Default)]
struct WorkerState {
    /// GitHub token of the connected account, if any.
    token: Option<String>,
    /// GitHub refused more API requests (rate limit): stop asking until the next launch.
    api_blocked: bool,
    /// Repositories the API cannot see (private or gone): no use asking again.
    unreachable: HashSet<String>,
}

enum ApiAnswer {
    Picture(String),
    NoAccount,
    Failed,
}

fn find(
    agent: &ureq::Agent,
    dir: Option<&Path>,
    req: &Request,
    state: &mut WorkerState,
) -> Option<ColorImage> {
    let name = file_name(&req.email);
    let picture_file = dir.map(|d| d.join(format!("{name}.img")));
    let none_file = dir.map(|d| d.join(format!("{name}.none")));

    // Already known?
    if let Some(bytes) = picture_file.as_ref().and_then(|f| std::fs::read(f).ok()) {
        return decode(&bytes);
    }
    let recently_missing = none_file
        .as_ref()
        .and_then(|f| std::fs::metadata(f).ok()?.modified().ok())
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < RETRY_AFTER);
    if recently_missing {
        return None;
    }

    // Where is the picture?
    let url = match noreply_picture(&req.email) {
        Some(url) => url,
        None => {
            let repo = req.github_repo.as_ref()?;
            if state.api_blocked || state.unreachable.contains(repo) {
                return None;
            }
            // GitHub only knows pushed commits: take one from the remote branches.
            let out = git::git(
                &req.repo_path,
                &[
                    "log",
                    "--remotes",
                    "-n1",
                    "--format=%H",
                    "--fixed-strings",
                    "--regexp-ignore-case",
                    &format!("--author=<{}>", req.email),
                ],
            );
            let sha = out.stdout.trim();
            if sha.is_empty() {
                return None;
            }
            match ask_api(agent, repo, sha, state) {
                ApiAnswer::Picture(url) => url,
                ApiAnswer::NoAccount => {
                    if let Some(f) = &none_file {
                        let _ = std::fs::write(f, "");
                    }
                    return None;
                }
                ApiAnswer::Failed => return None,
            }
        }
    };

    // Download it.
    let sep = if url.contains('?') { '&' } else { '?' };
    let resp = agent.get(&format!("{url}{sep}s={SIZE}")).call().ok()?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(2 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    let image = decode(&bytes)?;
    if let Some(f) = &picture_file {
        let _ = std::fs::write(f, &bytes);
    }
    Some(image)
}

/// The picture URL hidden in a GitHub "noreply" address, without any API call.
fn noreply_picture(email: &str) -> Option<String> {
    let local = email.strip_suffix("@users.noreply.github.com")?;
    match local.split_once('+') {
        // 12345+login@users.noreply.github.com
        Some((id, _)) if id.chars().all(|c| c.is_ascii_digit()) => {
            Some(format!("https://avatars.githubusercontent.com/u/{id}"))
        }
        // login@users.noreply.github.com (older accounts)
        _ => Some(format!("https://github.com/{local}.png")),
    }
}

/// Ask GitHub which account wrote commit `sha` of `repo`.
fn ask_api(agent: &ureq::Agent, repo: &str, sha: &str, state: &mut WorkerState) -> ApiAnswer {
    let url = format!("https://api.github.com/repos/{repo}/commits/{sha}");
    let mut request = agent.get(&url).set("Accept", "application/vnd.github+json");
    if let Some(token) = &state.token {
        request = request.set("Authorization", &format!("Bearer {token}"));
    }
    let resp = request.call();
    let json: serde_json::Value = match resp {
        Ok(r) => match serde_json::from_reader(r.into_reader().take(1024 * 1024)) {
            Ok(j) => j,
            Err(_) => return ApiAnswer::Failed,
        },
        // Private (and the account cannot see it) or deleted repository.
        Err(ureq::Error::Status(404, _)) => {
            state.unreachable.insert(repo.to_string());
            return ApiAnswer::Failed;
        }
        // Rate limit reached.
        Err(ureq::Error::Status(403 | 429, _)) => {
            state.api_blocked = true;
            return ApiAnswer::Failed;
        }
        // Commit not pushed yet (422), network error…
        Err(_) => return ApiAnswer::Failed,
    };
    match json["author"]["avatar_url"].as_str() {
        Some(url) => ApiAnswer::Picture(url.to_string()),
        // The e-mail is not linked to any GitHub account.
        None => ApiAnswer::NoAccount,
    }
}

fn decode(bytes: &[u8]) -> Option<ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let img = image::imageops::resize(&img, SIZE, SIZE, image::imageops::FilterType::Triangle);
    Some(ColorImage::from_rgba_unmultiplied(
        [SIZE as usize, SIZE as usize],
        img.as_raw(),
    ))
}

fn cache_dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("APPDATA"))?;
    Some(PathBuf::from(base).join("GitSimpleGUI").join("avatars"))
}

/// An e-mail turned into a safe file name.
fn file_name(email: &str) -> String {
    email
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_urls() {
        for url in [
            "https://github.com/rust-lang/rust.git",
            "https://github.com/rust-lang/rust",
            "git@github.com:rust-lang/rust.git",
            "ssh://git@github.com/rust-lang/rust",
        ] {
            assert_eq!(
                parse_github_url(url).as_deref(),
                Some("rust-lang/rust"),
                "{url}"
            );
        }
        assert_eq!(parse_github_url("https://gitlab.com/a/b.git"), None);
    }

    #[test]
    fn noreply_addresses() {
        assert_eq!(
            noreply_picture("123+bob@users.noreply.github.com").as_deref(),
            Some("https://avatars.githubusercontent.com/u/123")
        );
        assert_eq!(
            noreply_picture("bob@users.noreply.github.com").as_deref(),
            Some("https://github.com/bob.png")
        );
        assert_eq!(noreply_picture("bob@example.com"), None);
    }
}
