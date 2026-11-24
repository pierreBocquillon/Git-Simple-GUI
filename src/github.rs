//! The GitHub account the app talks to GitHub with (optional).
//!
//! Without an account, GitHub only answers about public repositories, 60 times per hour.
//! With a personal access token, it also answers about the private repositories that account
//! can read, 5000 times per hour. The token is only used to ask GitHub who wrote a commit
//! (for profile pictures, see `avatars`).
//!
//! The token is kept in the Windows Credential Manager ("git-simple-gui"), never in a file.

use std::io::Read;
use std::time::Duration;

/// GitHub page that creates a token with the right permission, already filled in.
pub const NEW_TOKEN_URL: &str =
    "https://github.com/settings/tokens/new?scopes=repo&description=Git%20Simple%20GUI";

const SERVICE: &str = "git-simple-gui";
const ACCOUNT: &str = "github-token";

/// The saved token, if the person connected an account.
pub fn saved_token() -> Option<String> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT).ok()?;
    entry.get_password().ok().filter(|t| !t.is_empty())
}

/// Check a token with GitHub and save it. Returns the account name ("login").
pub fn connect(token: &str) -> Result<String, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err(tr!("github.empty_token").into());
    }
    let login = account_name(token)?;
    keyring::Entry::new(SERVICE, ACCOUNT)
        .and_then(|e| e.set_password(token))
        .map_err(|e| tr!("github.save_token_failed", error = e))?;
    Ok(login)
}

/// Forget the saved token.
pub fn disconnect() {
    if let Ok(e) = keyring::Entry::new(SERVICE, ACCOUNT) {
        let _ = e.delete_credential();
    }
}

/// Ask GitHub which account a token belongs to.
fn account_name(token: &str) -> Result<String, String> {
    let resp = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent("git-simple-gui")
        .build()
        .get("https://api.github.com/user")
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &format!("Bearer {token}"))
        .call();
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(401, _)) => return Err(tr!("github.token_refused").into()),
        Err(ureq::Error::Status(code, _)) => return Err(tr!("github.status", code = code)),
        Err(e) => return Err(tr!("github.unreachable", error = e)),
    };
    let json: serde_json::Value = serde_json::from_reader(resp.into_reader().take(1024 * 1024))
        .map_err(|_| tr!("github.bad_response").to_string())?;
    json["login"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| tr!("github.bad_response").into())
}
