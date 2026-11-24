//! Updates: find out whether GitHub has a newer release, then download and run its installer.
//!
//! A release is published when the version in Cargo.toml changes, tagged `v<version>` (e.g.
//! `v1.2.0`, see .github/workflows/release.yml). The workflow builds the app with
//! `GITHUB_REPOSITORY = owner/name`, so a released copy knows where to look; it compares the
//! latest release's version with its own. A copy built locally does not know the repository
//! and never checks.
//!
//! Installing: the installer (GitSimpleGUI-Setup.exe) is downloaded to the temp folder and
//! started with /SILENT; the app then closes, and the installer reopens it when done.

use eframe::egui::Context;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// "owner/name" of the GitHub repository the releases come from, set by GitHub Actions.
const REPO: Option<&str> = option_env!("GITHUB_REPOSITORY");
/// Name of the installer in each release.
const INSTALLER_ASSET: &str = "GitSimpleGUI-Setup.exe";

/// A release on GitHub.
#[derive(Clone, Debug)]
pub struct Release {
    /// e.g. "1.2.0".
    pub version: String,
    /// e.g. "Git Simple GUI 1.2.0".
    pub title: String,
    /// The release notes (the commit message of the push).
    pub notes: String,
    /// API address of the installer, if the release has one.
    installer: Option<String>,
}

/// What a check found.
pub type CheckResult = Result<Option<Release>, String>;

/// This copy's version, e.g. "1.2.0" (from Cargo.toml).
pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Whether this copy can check for updates (it was built by the release workflow).
pub fn can_check() -> bool {
    REPO.is_some()
}

/// "1.2.0" or "v1.2.0" → (1, 2, 0). Missing parts count as 0; anything else is not a version.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let mut next = || -> Option<u64> { parts.next().map_or(Some(0), |p| p.parse().ok()) };
    Some((next()?, next()?, next()?))
}

/// Ask GitHub, in the background, for a release newer than this copy. The answer is
/// `Ok(Some(release))` when there is one, `Ok(None)` when this copy is up to date.
pub fn check(ctx: &Context) -> Receiver<CheckResult> {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let current = parse_version(env!("CARGO_PKG_VERSION"));
        let result = match REPO {
            Some(repo) => {
                latest_release(repo).map(|r| r.filter(|r| parse_version(&r.version) > current))
            }
            None => Err(tr!("update.local_build_error").into()),
        };
        let _ = tx.send(result);
        ctx.request_repaint();
    });
    rx
}

/// Download the release's installer and start it, in the background. `Ok` means the
/// installer is running: the app must close so it can be replaced.
pub fn install(release: &Release, ctx: &Context) -> Receiver<Result<(), String>> {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    let release = release.clone();
    std::thread::spawn(move || {
        let _ = tx.send(download_and_run(&release));
        ctx.request_repaint();
    });
    rx
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(120))
        .user_agent("git-simple-gui")
        .build()
}

/// A GitHub API request, with the connected account's token when there is one
/// (needed when the repository is private).
fn github_get(url: &str, accept: &str) -> ureq::Request {
    let mut req = agent().get(url).set("Accept", accept);
    if let Some(token) = crate::github::saved_token() {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    req
}

fn latest_release(repo: &str) -> Result<Option<Release>, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = match github_get(&url, "application/vnd.github+json").call() {
        Ok(r) => r,
        // No release yet (or a private repository without a connected account).
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(ureq::Error::Status(code, _)) => return Err(tr!("update.github_status", code = code)),
        Err(e) => return Err(tr!("update.github_unreachable", error = e)),
    };
    let json: serde_json::Value = serde_json::from_reader(resp.into_reader().take(1024 * 1024))
        .map_err(|_| tr!("update.github_bad_response").to_string())?;
    // Tags are "v<version>"; anything else (older "build-N" tags) is ignored.
    let Some(version) = json["tag_name"]
        .as_str()
        .filter(|t| t.starts_with('v') && parse_version(t).is_some())
        .map(|t| t.trim_start_matches('v').to_string())
    else {
        return Ok(None);
    };
    let installer = json["assets"].as_array().and_then(|assets| {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(INSTALLER_ASSET))
            .and_then(|a| a["url"].as_str())
            .map(str::to_string)
    });
    Ok(Some(Release {
        version,
        title: json["name"].as_str().unwrap_or_default().to_string(),
        notes: json["body"].as_str().unwrap_or_default().trim().to_string(),
        installer,
    }))
}

fn download_and_run(release: &Release) -> Result<(), String> {
    let url = release
        .installer
        .as_deref()
        .ok_or(tr!("update.no_installer"))?;
    let resp = github_get(url, "application/octet-stream")
        .call()
        .map_err(|e| tr!("update.download_failed", error = e))?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(200 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| tr!("update.download_interrupted", error = e))?;
    let path: PathBuf =
        std::env::temp_dir().join(format!("GitSimpleGUI-Setup-{}.exe", release.version));
    std::fs::write(&path, &bytes).map_err(|e| tr!("update.write_installer_failed", error = e))?;
    std::process::Command::new(&path)
        .args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
        .spawn()
        .map_err(|e| tr!("update.run_installer_failed", error = e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn versions_compare_by_number() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("2.0"), Some((2, 0, 0)));
        assert!(parse_version("1.10.0") > parse_version("1.9.9"));
        assert_eq!(parse_version("build-42"), None);
    }
}
