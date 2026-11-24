//! The app's side of updates (the GitHub part is in `crate::update`): check at launch or on
//! request, remember a newer release, install it and close.

use super::GitApp;
use crate::update::{self, CheckResult, Release};
use eframe::egui::{Context, ViewportCommand};
use std::sync::mpsc::Receiver;

/// Update state kept in `GitApp`.
#[derive(Default)]
pub struct Updates {
    /// A newer release found on GitHub.
    pub available: Option<Release>,
    /// The check running in the background, and whether the person asked for it (then the
    /// answer is shown even when there is nothing new).
    check: Option<(Receiver<CheckResult>, bool)>,
    /// The installer download running in the background.
    install: Option<Receiver<Result<(), String>>>,
}

impl Updates {
    pub fn checking(&self) -> bool {
        self.check.is_some()
    }

    pub fn installing(&self) -> bool {
        self.install.is_some()
    }
}

impl GitApp {
    /// Ask GitHub for a newer release. `asked`: the person clicked "check for updates".
    pub(super) fn check_for_update(&mut self, ctx: &Context, asked: bool) {
        if !update::can_check() {
            if asked {
                self.toast(false, tr!("update.none"), tr!("update.local_build_text"));
            }
            return;
        }
        if !self.updates.checking() {
            self.updates.check = Some((update::check(ctx), asked));
        }
    }

    /// Download and run the installer of the available release.
    pub(super) fn install_update(&mut self, ctx: &Context) {
        if let Some(release) = &self.updates.available {
            if !self.updates.installing() {
                self.updates.install = Some(update::install(release, ctx));
            }
        }
    }

    /// Collect the answers of the background check and download.
    pub(super) fn poll_updates(&mut self, ctx: &Context) {
        if let Some((rx, asked)) = &self.updates.check {
            let asked = *asked;
            if let Ok(result) = rx.try_recv() {
                self.updates.check = None;
                match result {
                    Ok(Some(release)) => {
                        if asked {
                            self.toast(true, tr!("update.available"), &release.title);
                        }
                        self.updates.available = Some(release);
                    }
                    Ok(None) if asked => self.toast(
                        true,
                        tr!("update.up_to_date"),
                        &tr!("update.version", version = update::current_version()),
                    ),
                    Err(e) if asked => self.toast(false, tr!("update.check_failed"), &e),
                    _ => {}
                }
            }
        }
        if let Some(rx) = &self.updates.install {
            if let Ok(result) = rx.try_recv() {
                self.updates.install = None;
                match result {
                    // The installer runs: close so it can replace the app (it reopens it).
                    Ok(()) => {
                        self.save_state();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    Err(e) => self.toast(false, tr!("update.install_failed"), &e),
                }
            }
        }
    }
}
