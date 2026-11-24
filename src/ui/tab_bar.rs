//! The top bar, which is also the window's title bar: logo, settings, workspace switcher,
//! one tab per repository of the current workspace, "new tab", then the window buttons.

use crate::app::{Action, Dialog, GitApp};
use crate::git;
use crate::i18n::{self, Lang};
use crate::ui::icons::{self, Icon};
use crate::ui::menus;
use crate::ui::paint::text_fit;
use crate::ui::theme::{self, *};
use crate::ui::window_frame;
use crate::update;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, Context, CornerRadius, Frame, Pos2, Rect, Response, Sense,
    Stroke, Ui,
};

/// Height of the bar (it is also the window's title bar).
const BAR_H: f32 = 40.0;
/// Tabs sit on the bottom edge, so the active one flows into the toolbar below.
const TAB_H: f32 = 32.0;
/// Height of the small buttons on the left (settings, workspace, new tab).
const BUTTON_H: f32 = 26.0;

pub fn show(app: &GitApp, ctx: &Context, acts: &mut Vec<Action>) {
    egui::TopBottomPanel::top("tabbar")
        .exact_height(BAR_H)
        .frame(Frame::new().fill(BG0))
        .show(ctx, |ui| {
            let full = ui.max_rect();
            // The empty part of the bar moves the window (we draw our own title bar).
            window_frame::drag_area(ui, full);
            let p = ui.painter().clone();
            // Everything on the left is centered on the tabs' middle line.
            let mid = full.bottom() - TAB_H / 2.0;

            // Logo | settings | workspace | tabs.
            let logo = Rect::from_center_size(pos2(full.left() + 20.0, mid), vec2(22.0, 22.0));
            p.image(
                app.logo.id(),
                logo,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            let x = settings_button(ui, app, pos2(logo.right() + 6.0, mid), acts);
            let x = divider(&p, x + 8.0, mid);
            let x = workspace_switcher(ui, app, pos2(x + 6.0, mid), acts);
            let x = divider(&p, x + 6.0, mid);
            let x = tabs(ui, app, x + 6.0, acts);
            new_tab_button(ui, app, pos2(x + 6.0, mid), acts);

            let controls_left = window_frame::controls(ui, full);
            update_button(ui, app, controls_left - 8.0, mid, acts);
        });
}

/// A thin vertical line at `x`; returns `x`.
fn divider(p: &egui::Painter, x: f32, mid: f32) -> f32 {
    p.line_segment(
        [pos2(x, mid - 8.0), pos2(x, mid + 8.0)],
        Stroke::new(1.0, BORDER),
    );
    x
}

/// A square button with only an icon, filled when hovered or `on`.
/// `left_center` is the middle of its left edge.
fn ghost_button(ui: &mut Ui, id: &str, left_center: Pos2, icon: Icon, on: bool) -> Response {
    let rect = Rect::from_min_size(
        pos2(left_center.x, left_center.y - BUTTON_H / 2.0),
        vec2(BUTTON_H, BUTTON_H),
    );
    let resp = ui.interact(rect, ui.id().with(id), Sense::click());
    let p = ui.painter();
    if on || resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(7), if on { BG2 } else { BG1 });
    }
    icons::draw(
        p,
        icon,
        rect.shrink(6.5),
        if on || resp.hovered() { TEXT } else { TEXT_DIM },
    );
    resp
}

/// The ⚙ button and its menu: git identity and GitHub account. Returns its right edge.
fn settings_button(ui: &mut Ui, app: &GitApp, left_center: Pos2, acts: &mut Vec<Action>) -> f32 {
    let popup = ui.id().with("settingsmenu");
    let open = ui.memory(|m| m.is_popup_open(popup));
    let resp = ghost_button(ui, "gear", left_center, Icon::Gear, open);
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let resp = resp.on_hover_text(tr!("tabbar.settings"));
    egui::popup_below_widget(
        ui,
        popup,
        &resp,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(260.0);
            if menus::item(ui, Some(Icon::Gear), tr!("tabbar.git_identity")) {
                acts.push(Action::Dialog(Dialog::Identity {
                    name: git::config_get(None, "user.name", true),
                    email: git::config_get(None, "user.email", true),
                }));
            }
            let github = match &app.github_login {
                Some(login) => tr!("tabbar.github_account", login = login),
                None => tr!("tabbar.connect_github").to_string(),
            };
            if menus::item(ui, Some(Icon::Cloud), &github) {
                acts.push(Action::Dialog(Dialog::GitHub {
                    token: String::new(),
                }));
            }
            menus::sep(ui);
            menus::heading(ui, tr!("tabbar.language"));
            for lang in Lang::ALL {
                let icon = (lang == i18n::current()).then_some(Icon::Check);
                if menus::item(ui, icon, lang.native_name()) {
                    acts.push(Action::SetLanguage(lang));
                }
            }
            menus::sep(ui);
            menus::heading(
                ui,
                &tr!("tabbar.version", version = update::current_version()),
            );
            if app.updates.available.is_some() {
                if menus::item(ui, Some(Icon::Pull), tr!("tabbar.install_update")) {
                    acts.push(Action::Dialog(Dialog::Update));
                }
            } else if menus::item(ui, Some(Icon::Refresh), tr!("tabbar.check_update")) {
                acts.push(Action::CheckUpdate);
            }
        },
    );
    resp.rect.right()
}

/// "Update" at the right of the bar, before the window buttons, when a newer version is
/// available; "Downloading…" while its installer downloads. `right` is its right edge.
fn update_button(ui: &mut Ui, app: &GitApp, right: f32, mid: f32, acts: &mut Vec<Action>) {
    let installing = app.updates.installing();
    if app.updates.available.is_none() {
        return;
    }
    let label = if installing {
        tr!("tabbar.downloading")
    } else {
        tr!("tabbar.update")
    };
    let font = theme::semibold(12.5);
    let tw = ui.fonts(|f| f.layout_no_wrap(label.into(), font.clone(), TEXT).size().x);
    let rect = Rect::from_min_max(
        pos2(right - tw - 38.0, mid - BUTTON_H / 2.0),
        pos2(right, mid + BUTTON_H / 2.0),
    );
    let resp = ui.interact(rect, ui.id().with("update"), Sense::click());
    let p = ui.painter();
    let fill = theme::mix(BG0, CYAN, if resp.hovered() { 0.3 } else { 0.18 });
    p.rect(
        rect,
        CornerRadius::same(7),
        fill,
        Stroke::new(1.0, alpha(CYAN, 0.7)),
        egui::StrokeKind::Inside,
    );
    icons::draw(
        p,
        Icon::Pull,
        Rect::from_center_size(pos2(rect.left() + 15.0, mid), vec2(13.0, 13.0)),
        CYAN,
    );
    p.text(
        pos2(rect.left() + 28.0, mid),
        Align2::LEFT_CENTER,
        label,
        font,
        TEXT,
    );
    let tip = app
        .updates
        .available
        .as_ref()
        .map(|r| r.title.clone())
        .unwrap_or_default();
    if resp.on_hover_text(tip).clicked() && !installing {
        acts.push(Action::Dialog(Dialog::Update));
    }
}

/// Each workspace gets a color from its position, used for its badge.
fn workspace_color(index: usize) -> Color32 {
    LANES[index % LANES.len()]
}

/// The current workspace: a colored badge with its initial, its name and a chevron.
/// A click opens the list of workspaces. Returns its right edge.
fn workspace_switcher(ui: &mut Ui, app: &GitApp, left_center: Pos2, acts: &mut Vec<Action>) -> f32 {
    let p = ui.painter().clone();
    let name = &app.workspace().name;
    let font = theme::semibold(13.0);
    let tw = ui.fonts(|f| f.layout_no_wrap(name.clone(), font.clone(), TEXT).size().x);
    let w = (tw + 52.0).min(220.0);
    let rect = Rect::from_min_size(
        pos2(left_center.x, left_center.y - BUTTON_H / 2.0),
        vec2(w, BUTTON_H),
    );
    let resp = ui.interact(rect, ui.id().with("workspace"), Sense::click());
    let popup = ui.id().with("workspacemenu");
    let open = ui.memory(|m| m.is_popup_open(popup));
    if open || resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(7), if open { BG2 } else { BG1 });
    }
    // Badge.
    let color = workspace_color(app.ws);
    let badge = Rect::from_center_size(pos2(rect.left() + 15.0, rect.center().y), vec2(18.0, 18.0));
    p.rect(
        badge,
        CornerRadius::same(5),
        theme::mix(BG0, color, 0.25),
        Stroke::new(1.0, alpha(color, 0.8)),
        egui::StrokeKind::Inside,
    );
    let initial: String = name.chars().next().unwrap_or('?').to_uppercase().collect();
    p.text(
        badge.center(),
        Align2::CENTER_CENTER,
        initial,
        theme::bold(10.5),
        color,
    );
    // Name and chevron.
    text_fit(
        &p,
        rect.left() + 30.0,
        rect.center().y,
        name,
        font,
        TEXT,
        w - 48.0,
    );
    icons::draw(
        &p,
        Icon::ChevronDown,
        Rect::from_center_size(pos2(rect.right() - 11.0, rect.center().y), vec2(8.0, 8.0)),
        TEXT_FAINT,
    );
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let resp = resp.on_hover_text(tr!("tabbar.switch_workspace"));
    egui::popup_below_widget(
        ui,
        popup,
        &resp,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(280.0);
            menus::heading(ui, tr!("tabbar.workspaces"));
            for (i, ws) in app.workspaces.iter().enumerate() {
                let n = app.tabs_of(ws.id).len();
                let count = if n == 1 {
                    tr!("tabbar.tabs_one", n = n)
                } else {
                    tr!("tabbar.tabs_other", n = n)
                };
                let mut label = format!("{}   · {count}", ws.name);
                if i < 9 {
                    label += &format!("   (Ctrl+{})", i + 1);
                }
                let icon = (i == app.ws).then_some(Icon::Check);
                if menus::item(ui, icon, &label) {
                    acts.push(Action::SwitchWorkspace(i));
                }
            }
            menus::sep(ui);
            if menus::item(ui, Some(Icon::Plus), tr!("tabbar.new_workspace")) {
                acts.push(Action::Dialog(Dialog::Workspace {
                    id: None,
                    name: String::new(),
                    with_tab: None,
                }));
            }
            let current = app.workspace();
            if menus::item(
                ui,
                Some(Icon::Box),
                &tr!("tabbar.rename_workspace", name = current.name),
            ) {
                acts.push(Action::Dialog(Dialog::Workspace {
                    id: Some(current.id),
                    name: current.name.clone(),
                    with_tab: None,
                }));
            }
            if app.workspaces.len() > 1
                && menus::danger(
                    ui,
                    Some(Icon::Close),
                    &tr!("tabbar.delete_workspace", name = current.name),
                )
            {
                let tabs = app.tabs_of(current.id).len();
                acts.push(if tabs == 0 {
                    Action::DeleteWorkspace(current.id)
                } else {
                    Action::Dialog(Dialog::DeleteWorkspace {
                        id: current.id,
                        name: current.name.clone(),
                        tabs,
                    })
                });
            }
        },
    );
    rect.right()
}

/// The tabs of the current workspace, from `left`. Returns the right edge of the last one.
fn tabs(ui: &mut Ui, app: &GitApp, left: f32, acts: &mut Vec<Action>) -> f32 {
    let full = ui.max_rect();
    let p = ui.painter().clone();
    let top = full.bottom() - TAB_H;
    let round_top = CornerRadius {
        nw: 10,
        ne: 10,
        sw: 0,
        se: 0,
    };
    let mut x = left;
    // (rect, highlighted) of each tab, to place the separators afterwards.
    let mut drawn: Vec<(Rect, bool)> = Vec::new();
    for i in app.visible_tabs() {
        let t = &app.tabs[i];
        let active = app.active == Some(i);
        let font = theme::semibold(12.5);
        let tw = ui.fonts(|f| {
            f.layout_no_wrap(t.name.clone(), font.clone(), TEXT)
                .size()
                .x
        });
        let w = (tw + 58.0).clamp(110.0, 220.0);
        let rect = Rect::from_min_size(pos2(x, top), vec2(w, TAB_H));
        x += w + 2.0;
        let resp = ui.interact(rect, ui.id().with(("tab", t.id)), Sense::click());
        let hovered = resp.hovered();
        drawn.push((rect, active || hovered));

        // Background: the active tab has the toolbar's color and a thin outline on the top
        // and sides (the bottom edge is clipped away so it flows into the toolbar). A hovered
        // tab gets a soft rounded fill that does not touch the toolbar.
        if active {
            p.rect_filled(rect, round_top, BG2);
            p.with_clip_rect(rect).rect_stroke(
                rect.with_max_y(rect.bottom() + 2.0),
                round_top,
                Stroke::new(1.0, BORDER),
                egui::StrokeKind::Inside,
            );
        } else if hovered {
            let soft = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - 4.0));
            p.rect_filled(soft, CornerRadius::same(8), BG1);
        }

        // Icon (a spinner while a git command runs), name, close button.
        let c = pos2(rect.left() + 16.0, rect.center().y);
        if t.busy.is_some() {
            let a = ui.input(|i| i.time) as f32 * 6.0;
            p.circle_stroke(c, 5.5, Stroke::new(1.5, alpha(CYAN, 0.25)));
            p.line_segment(
                [
                    c + vec2(a.cos(), a.sin()) * 5.5,
                    c + vec2((a + 1.2).cos(), (a + 1.2).sin()) * 5.5,
                ],
                Stroke::new(2.0, CYAN),
            );
        } else {
            icons::draw(
                &p,
                Icon::Folder,
                Rect::from_center_size(c, vec2(13.0, 13.0)),
                if active { CYAN } else { TEXT_FAINT },
            );
        }
        text_fit(
            &p,
            rect.left() + 29.0,
            rect.center().y,
            &t.name,
            font,
            if active { TEXT } else { TEXT_DIM },
            rect.width() - 52.0,
        );
        let close =
            Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(16.0, 16.0));
        let close_resp = ui.interact(close, ui.id().with(("tabclose", t.id)), Sense::click());
        if active || hovered || close_resp.hovered() {
            if close_resp.hovered() {
                p.rect_filled(close, CornerRadius::same(4), BG4);
            }
            icons::draw(
                &p,
                Icon::Close,
                close.shrink(4.0),
                if close_resp.hovered() {
                    TEXT
                } else {
                    TEXT_FAINT
                },
            );
        }
        if close_resp.clicked() || resp.middle_clicked() {
            acts.push(Action::CloseTab(i));
        } else if resp.clicked() {
            acts.push(Action::ActivateTab(Some(i)));
        }
        resp.context_menu(|ui| tab_menu(ui, app, i, acts));
        resp.on_hover_text(t.path.display().to_string());
    }
    // Thin separators between two tabs that are neither active nor hovered.
    for pair in drawn.windows(2) {
        let ((a, a_on), (_, b_on)) = (pair[0], pair[1]);
        if !a_on && !b_on {
            let sx = a.right() + 1.0;
            p.line_segment(
                [pos2(sx, a.center().y - 7.0), pos2(sx, a.center().y + 7.0)],
                Stroke::new(1.0, BORDER),
            );
        }
    }
    drawn.last().map(|(r, _)| r.right()).unwrap_or(left)
}

/// The "+" after the tabs: opens the welcome page (open, clone or create a repository).
fn new_tab_button(ui: &mut Ui, app: &GitApp, left_center: Pos2, acts: &mut Vec<Action>) {
    let on_welcome = app.active.is_none();
    if ghost_button(ui, "newtab", left_center, Icon::Plus, on_welcome)
        .on_hover_text(tr!("tabbar.new_tab"))
        .clicked()
    {
        acts.push(Action::ActivateTab(None));
    }
}

/// Right-click on a tab: move it to another workspace, show it in Explorer, copy its path,
/// or close it.
fn tab_menu(ui: &mut Ui, app: &GitApp, i: usize, acts: &mut Vec<Action>) {
    let tab = &app.tabs[i];
    menus::heading(ui, tr!("tabbar.move_to_workspace"));
    for ws in app.workspaces.iter().filter(|w| w.id != tab.workspace) {
        if menus::item(ui, Some(Icon::Box), &ws.name) {
            acts.push(Action::MoveTab {
                tab: tab.id,
                workspace: ws.id,
            });
        }
    }
    if menus::item(ui, Some(Icon::Plus), tr!("tabbar.new_workspace_with_tab")) {
        acts.push(Action::Dialog(Dialog::Workspace {
            id: None,
            name: String::new(),
            with_tab: Some(tab.id),
        }));
    }
    menus::sep(ui);
    if menus::item(ui, Some(Icon::Folder), tr!("tabbar.show_in_explorer")) {
        acts.push(Action::ShowInExplorer(tab.path.clone()));
    }
    if menus::item(ui, Some(Icon::Copy), tr!("tabbar.copy_path")) {
        acts.push(Action::Copy(tab.path.display().to_string()));
    }
    if menus::item(ui, Some(Icon::Close), tr!("tabbar.close_tab")) {
        acts.push(Action::CloseTab(i));
    }
}
