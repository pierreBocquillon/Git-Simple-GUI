//! Everything drawn on screen. The UI reads the app state and pushes `Action`s; it never runs
//! git itself.
//!
//! Window layout (top to bottom, left to right):
//! - `tab_bar`      logo, workspace switcher and the tabs of the current workspace
//! - `toolbar`      Undo/Redo, Pull, Push, Branch, Stash… (left), search and refresh (right)
//! - `sidebar`      LOCAL / REMOTE / STASHES / TAGS
//! - `graph`        the commit graph table (or `diff_view` when a file is opened)
//! - `details`      right panel: WIP staging, commit details or stash contents
//! - `status_bar`   bottom line
//! - `welcome`      the page shown when no repository is open
//! - `dialogs`, `menus`, `toasts`   pop-ups
//!
//! - `window_frame` window buttons, moving and resizing (no Windows title bar)
//!
//! Shared building blocks: `theme` (colors, fonts), `paint` (drawing helpers), `icons`, `widgets`.

mod details;
mod dialogs;
mod diff_view;
pub mod graph;
mod icons;
mod menus;
mod paint;
mod sidebar;
mod state_banner;
mod status_bar;
mod tab_bar;
pub mod theme;
mod toasts;
mod toolbar;
mod welcome;
mod widgets;
mod window_frame;

use crate::app::{Action, GitApp};
use eframe::egui::{self, Context, Frame, RichText};
use theme::{BG0, TEXT, TEXT_DIM};

/// Draw the whole window for this frame and collect what the person asked for.
pub fn draw(app: &mut GitApp, ctx: &Context, acts: &mut Vec<Action>) {
    shortcuts(app, ctx, acts);
    tab_bar::show(app, ctx, acts);
    let Some(i) = app.active else {
        status_bar::show(ctx, None, app.git_version.as_deref());
        welcome::show(app, ctx, acts);
        dialogs::show(app, ctx, acts);
        toasts::show(app, ctx);
        window_frame::resize_edges(ctx);
        return;
    };

    let pull_mode = app.pull_mode;
    let git_version = app.git_version.clone();
    let avatars = &app.avatars;
    let tab = &mut app.tabs[i];
    toolbar::show(ctx, tab, pull_mode, acts);
    status_bar::show(ctx, Some(tab), git_version.as_deref());
    if tab.data.is_some() {
        sidebar::show(ctx, tab, acts);
        details::show(ctx, tab, avatars, acts);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG0))
            .show(ctx, |ui| {
                state_banner::show(ui, tab, acts);
                graph::show(ui, tab, avatars, acts);
            });
    } else {
        // The folder could not be read as a repository.
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG0))
            .show(ctx, |ui| {
                let msg = tab.error.clone().unwrap_or_default();
                ui.add_space(80.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(tr!("ui.cannot_read_repo"))
                            .font(theme::semibold(18.0))
                            .color(TEXT),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new(msg).color(TEXT_DIM));
                    ui.add_space(16.0);
                    if ui.button(tr!("ui.retry")).clicked() {
                        acts.push(Action::Refresh);
                    }
                });
            });
    }
    dialogs::show(app, ctx, acts);
    toasts::show(app, ctx);
    window_frame::resize_edges(ctx);
}

/// Ctrl+1…9 switch workspace; with a repository open: F5 refresh, Ctrl+F search, Ctrl+O open.
fn shortcuts(app: &GitApp, ctx: &Context, acts: &mut Vec<Action>) {
    if app.dialog.is_some() {
        return;
    }
    const DIGITS: [egui::Key; 9] = [
        egui::Key::Num1,
        egui::Key::Num2,
        egui::Key::Num3,
        egui::Key::Num4,
        egui::Key::Num5,
        egui::Key::Num6,
        egui::Key::Num7,
        egui::Key::Num8,
        egui::Key::Num9,
    ];
    let has_tab = app.tab().is_some();
    ctx.input_mut(|i| {
        // Match the physical key too, and accept Shift: on AZERTY the "1" key types "&".
        let mut switch = None;
        i.events.retain(|e| {
            let egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                modifiers,
                ..
            } = e
            else {
                return true;
            };
            if !modifiers.command || modifiers.alt {
                return true;
            }
            let digit = DIGITS
                .iter()
                .position(|d| d == key || Some(*d) == *physical_key);
            match digit {
                Some(n) if n < app.workspaces.len() => {
                    switch = Some(n);
                    false
                }
                _ => true,
            }
        });
        if let Some(n) = switch {
            acts.push(Action::SwitchWorkspace(n));
        }
        if !has_tab {
            return;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::F5) {
            acts.push(Action::Refresh);
        }
        if i.consume_key(egui::Modifiers::COMMAND, egui::Key::F) {
            acts.push(Action::ToggleSearch);
        }
        if i.consume_key(egui::Modifiers::COMMAND, egui::Key::O) {
            acts.push(Action::PickRepo);
        }
    });
}
