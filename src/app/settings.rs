//! Remembered between launches: recent repositories, workspaces with their open tabs, the
//! current workspace and the default Pull mode. Saved as a small text file in
//! %APPDATA%\GitSimpleGUI\state.txt, one `key<TAB>value` per line:
//!
//! ```text
//! recent   C:\Projects\app    (repeated)
//! ws       Work                starts a workspace; the lines below belong to it
//! open     C:\Projects\app    a tab of that workspace (repeated)
//! focus    0                   the tab shown in that workspace (position among its tabs)
//! current  1                   the workspace on screen
//! pull     merge
//! github   login               the connected GitHub account (its token is not in this file)
//! lang     fr                  the language picked in the app (see `installer_language`)
//! ```

use super::GitApp;
use crate::i18n::Lang;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullMode {
    FetchAll,
    Merge,
    FfOnly,
    Rebase,
}

impl PullMode {
    pub fn label(self) -> &'static str {
        match self {
            PullMode::FetchAll => tr!("app.pull_mode.fetch_all"),
            PullMode::Merge => tr!("app.pull_mode.merge"),
            PullMode::FfOnly => tr!("app.pull_mode.ff_only"),
            PullMode::Rebase => tr!("app.pull_mode.rebase"),
        }
    }
    fn key(self) -> &'static str {
        match self {
            PullMode::FetchAll => "fetch",
            PullMode::Merge => "merge",
            PullMode::FfOnly => "ffonly",
            PullMode::Rebase => "rebase",
        }
    }
    fn from_key(s: &str) -> PullMode {
        match s {
            "fetch" => PullMode::FetchAll,
            "ffonly" => PullMode::FfOnly,
            "rebase" => PullMode::Rebase,
            _ => PullMode::Merge,
        }
    }
}

/// A workspace as read from the state file, before its tabs are opened.
pub(super) struct SavedWorkspace {
    pub name: String,
    pub open: Vec<PathBuf>,
    /// Position of the remembered tab among `open`.
    pub focus: Option<usize>,
}

impl SavedWorkspace {
    fn named(name: &str) -> Self {
        SavedWorkspace {
            name: name.to_string(),
            open: Vec::new(),
            focus: None,
        }
    }
}

impl GitApp {
    fn state_file() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
        Some(PathBuf::from(base).join("GitSimpleGUI").join("state.txt"))
    }

    /// Read the saved state. Fills `recent` and `pull_mode` directly; returns the workspaces
    /// to reopen and the position of the current one.
    pub(super) fn load_state(&mut self) -> (Vec<SavedWorkspace>, usize) {
        let mut workspaces: Vec<SavedWorkspace> = Vec::new();
        let mut current = 0;
        let Some(text) = Self::state_file().and_then(|f| std::fs::read_to_string(f).ok()) else {
            return (workspaces, current);
        };
        // Files written before workspaces existed have "open" lines without a "ws" line.
        let last = |w: &mut Vec<SavedWorkspace>| -> usize {
            if w.is_empty() {
                w.push(SavedWorkspace::named(tr!("workspace.default_name")));
            }
            w.len() - 1
        };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('\t') else {
                continue;
            };
            match k {
                "recent" => self.recent.push(PathBuf::from(v)),
                "ws" => workspaces.push(SavedWorkspace::named(v)),
                "open" => {
                    let i = last(&mut workspaces);
                    workspaces[i].open.push(PathBuf::from(v));
                }
                // "active" is the old name of "focus".
                "focus" | "active" => {
                    let i = last(&mut workspaces);
                    workspaces[i].focus = v.parse().ok();
                }
                "current" => current = v.parse().unwrap_or(0),
                "pull" => self.pull_mode = PullMode::from_key(v),
                "github" => self.github_login = Some(v.to_string()),
                "lang" => {
                    if let Some(lang) = Lang::from_code(v) {
                        self.language = Some(lang);
                        crate::i18n::set(lang);
                    }
                }
                _ => {}
            }
        }
        (workspaces, current)
    }

    pub fn save_state(&self) {
        let Some(f) = Self::state_file() else { return };
        let mut out = String::new();
        // First, so the language is known while the rest is read.
        if let Some(lang) = self.language {
            out += &format!("lang\t{}\n", lang.code());
        }
        for r in &self.recent {
            out += &format!("recent\t{}\n", r.display());
        }
        for w in &self.workspaces {
            out += &format!("ws\t{}\n", w.name);
            // The tab on screen counts as the remembered one for the current workspace.
            let focus = if w.id == self.workspace().id {
                self.active.map(|a| self.tabs[a].id)
            } else {
                w.focus
            };
            for (local, &i) in self.tabs_of(w.id).iter().enumerate() {
                out += &format!("open\t{}\n", self.tabs[i].path.display());
                if focus == Some(self.tabs[i].id) {
                    out += &format!("focus\t{local}\n");
                }
            }
        }
        out += &format!("current\t{}\n", self.ws);
        out += &format!("pull\t{}\n", self.pull_mode.key());
        if let Some(login) = &self.github_login {
            out += &format!("github\t{login}\n");
        }
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(f, out);
    }
}

/// The language chosen in the installer, written next to the app in `language.ini`
/// (`language=fr`). Used until a language is picked in the app.
pub(super) fn installer_language() -> Option<Lang> {
    let exe = std::env::current_exe().ok()?;
    let text = std::fs::read_to_string(exe.parent()?.join("language.ini")).ok()?;
    text.lines()
        .find_map(|l| l.trim().strip_prefix("language="))
        .and_then(Lang::from_code)
}
