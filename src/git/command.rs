//! Running the installed `git` command line.
//!
//! We drive the real `git` executable, so the person's own settings (identity, credential
//! helper, SSH keys) keep working.

use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Clone, Debug)]
pub struct GitOutput {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

impl GitOutput {
    /// The most useful message to show a person after a command.
    pub fn message(&self) -> String {
        let text = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        let lines: Vec<&str> = text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("hint:"))
            .collect();
        let start = lines.len().saturating_sub(6);
        lines[start..].join("\n")
    }
}

fn base_command(repo: Option<&Path>) -> Command {
    let mut cmd = Command::new("git");
    if let Some(repo) = repo {
        cmd.arg("-C").arg(repo);
    }
    cmd.args([
        "-c",
        "core.quotepath=false",
        "-c",
        "color.ui=false",
        "-c",
        "core.pager=cat",
    ]);
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: never flash a console
    cmd
}

pub fn run_in(repo: Option<&Path>, args: &[String]) -> GitOutput {
    let mut cmd = base_command(repo);
    cmd.args(args);
    match cmd.output() {
        Ok(out) => GitOutput {
            ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => GitOutput {
            ok: false,
            stdout: String::new(),
            stderr: tr!("git.cannot_run", error = e),
        },
    }
}

pub fn git(repo: &Path, args: &[&str]) -> GitOutput {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_in(Some(repo), &args)
}

pub fn config_get(repo: Option<&Path>, key: &str, global: bool) -> String {
    let mut args = vec!["config".to_string()];
    if global {
        args.push("--global".into());
    }
    args.push("--get".into());
    args.push(key.into());
    run_in(repo, &args).stdout.trim().to_string()
}
