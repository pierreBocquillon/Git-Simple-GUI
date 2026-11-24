// Git Simple GUI - a fast, dark Git client for Windows.
// Copyright (C) 2026 Pierre Bocquillon - https://github.com/pierreBocquillon/Git-Simple-GUI
//
// This program is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details (LICENSE file).

//! Git Simple GUI: a clean, dark git client. Commit graph, branches, remotes and everyday git actions.
//!
//! Layout of the code:
//! - `git/`   talks to the installed `git` command and reads a repository (no UI here)
//! - `graph`  lays the commits out in lanes for the graph (pure logic)
//! - `avatars` authors' GitHub profile pictures, downloaded in the background
//! - `github` the optional GitHub account (token kept in the Windows Credential Manager)
//! - `update` newer releases on GitHub: check, download and run the installer
//! - `i18n`   every text shown on screen, in English, French, German and Spanish
//! - `app/`   application state: open tabs, background jobs, actions, undo/redo, settings
//! - `ui/`    everything drawn on screen (panels, graph, dialogs, menus, theme)
//!
//! Each frame, the UI draws itself from the state and pushes `Action`s; the app applies them after.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod i18n;

mod app;
mod avatars;
mod git;
mod github;
mod graph;
mod ui;
mod update;

use eframe::egui;
use std::path::PathBuf;

const LOGO_PNG: &[u8] = include_bytes!("../assets/logo.png");

fn main() -> eframe::Result<()> {
    // Folders given on the command line are opened as tabs.
    let open: Vec<PathBuf> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .collect();

    let logo = image::load_from_memory(LOGO_PNG).expect("logo").to_rgba8();
    let texture = image::imageops::resize(&logo, 320, 320, image::imageops::FilterType::Triangle);
    let icon = image::imageops::resize(&logo, 128, 128, image::imageops::FilterType::Triangle);
    let texture_image = egui::ColorImage::from_rgba_unmultiplied([320, 320], texture.as_raw());

    let viewport = egui::ViewportBuilder::default()
        .with_title("Git Simple GUI")
        .with_app_id("git-simple-gui")
        .with_inner_size([1480.0, 900.0])
        .with_min_inner_size([980.0, 600.0])
        // No Windows title bar: the top bar draws its own (see ui::window_frame).
        .with_decorations(false)
        .with_icon(egui::IconData {
            rgba: icon.into_raw(),
            width: 128,
            height: 128,
        });
    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Git Simple GUI",
        options,
        Box::new(move |cc| Ok(Box::new(app::GitApp::new(cc, open, texture_image)))),
    )
}
