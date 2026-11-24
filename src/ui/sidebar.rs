//! Left panel: LOCAL branches, REMOTE branches, STASHES and TAGS, with a filter box.

use crate::app::{commands, Action, Dialog, RepoTab, Selection};
use crate::git::RefKind;
use crate::ui::icons::{self, Icon};
use crate::ui::menus;
use crate::ui::paint::text_fit;
use crate::ui::theme::{self, *};
use crate::ui::widgets::{badge, hint_row};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, Context, CornerRadius, Frame, Margin, Rect, Response, Sense,
    Stroke, StrokeKind, TextEdit, Ui,
};
use std::collections::BTreeMap;

#[allow(clippy::too_many_arguments)]
pub fn side_row(
    ui: &mut Ui,
    depth: usize,
    icon: Icon,
    icon_color: Color32,
    text: &str,
    strong: bool,
    active: bool,
    right: &[(String, Color32)],
) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    let p = ui.painter();
    let pill = Rect::from_min_max(
        pos2(rect.left() + 6.0, rect.top() + 1.0),
        pos2(rect.right() - 6.0, rect.bottom() - 2.0),
    );
    if active {
        p.rect(
            pill,
            CornerRadius::same(10),
            theme::mix(BG1, VIOLET, 0.2),
            Stroke::new(1.0, alpha(VIOLET, 0.7)),
            StrokeKind::Inside,
        );
    } else if resp.hovered() {
        p.rect_filled(pill, CornerRadius::same(10), BG3);
    }
    let x = rect.left() + 16.0 + depth as f32 * 16.0;
    icons::draw(
        p,
        icon,
        Rect::from_center_size(pos2(x + 7.0, rect.center().y), vec2(13.0, 13.0)),
        icon_color,
    );
    // Right side badges.
    let mut rx = rect.right() - 10.0;
    for (t, c) in right.iter().rev() {
        rx = badge(p, rx, rect.center().y, t, *c) - 4.0;
    }
    let font = if strong {
        theme::semibold(13.0)
    } else {
        theme::regular(13.0)
    };
    text_fit(
        p,
        x + 21.0,
        rect.center().y,
        text,
        font,
        if strong || active { TEXT } else { TEXT_DIM },
        rx - x - 26.0,
    );
    resp
}

pub fn section(
    ui: &mut Ui,
    tab: &mut RepoTab,
    key: &str,
    title: &str,
    icon: Icon,
    count: usize,
    add_tip: &str,
) -> (bool, bool) {
    let w = ui.available_width();
    ui.add_space(6.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
    let p = ui.painter().clone();
    let id = format!("section:{key}");
    let open = !tab.collapsed.contains(&id);
    if resp.hovered() {
        p.rect_filled(rect, 0.0, alpha(BG3, 0.6));
    }
    icons::draw(
        &p,
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        Rect::from_center_size(pos2(rect.left() + 14.0, rect.center().y), vec2(10.0, 10.0)),
        TEXT_FAINT,
    );
    icons::draw(
        &p,
        icon,
        Rect::from_center_size(pos2(rect.left() + 32.0, rect.center().y), vec2(14.0, 14.0)),
        VIOLET,
    );
    p.text(
        pos2(rect.left() + 46.0, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        theme::bold(11.5),
        TEXT_DIM,
    );
    let tw = ui.fonts(|f| {
        f.layout_no_wrap(title.into(), theme::bold(11.5), TEXT)
            .size()
            .x
    });
    let chip = Rect::from_min_size(
        pos2(rect.left() + 52.0 + tw, rect.center().y - 8.0),
        vec2(if count > 99 { 30.0 } else { 22.0 }, 16.0),
    );
    p.rect_filled(chip, CornerRadius::same(8), BG3);
    p.text(
        chip.center(),
        Align2::CENTER_CENTER,
        count.to_string(),
        theme::semibold(10.5),
        TEXT_DIM,
    );
    let plus = Rect::from_center_size(pos2(rect.right() - 18.0, rect.center().y), vec2(22.0, 22.0));
    let presp = ui.interact(plus, ui.id().with(("add", key)), Sense::click());
    let mut add = false;
    if resp.hovered() || presp.hovered() {
        if presp.hovered() {
            p.rect_filled(plus, CornerRadius::same(5), BG4);
        }
        icons::draw(
            &p,
            Icon::Plus,
            plus.shrink(5.0),
            if presp.hovered() { CYAN } else { TEXT_DIM },
        );
    }
    if presp.on_hover_text(add_tip).clicked() {
        add = true;
    } else if resp.clicked() {
        if open {
            tab.collapsed.insert(id);
        } else {
            tab.collapsed.remove(&id);
        }
    }
    (open, add)
}

pub fn show(ctx: &Context, tab: &mut RepoTab, acts: &mut Vec<Action>) {
    egui::SidePanel::left("sidebar")
        .resizable(true)
        .default_width(262.0)
        .width_range(190.0..=460.0)
        .frame(Frame::new().fill(BG1).inner_margin(Margin {
            left: 0,
            right: 0,
            top: 10,
            bottom: 6,
        }))
        .show(ctx, |ui| {
            let Some(d) = tab.data.clone() else { return };
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                let edit = TextEdit::singleline(&mut tab.sidebar_filter)
                    .hint_text(tr!("sidebar.filter_hint"))
                    .desired_width(ui.available_width() - 10.0)
                    .margin(Margin::symmetric(8, 6));
                ui.add(edit);
            });
            ui.add_space(8.0);
            let filter = tab.sidebar_filter.trim().to_lowercase();
            let keep = |s: &str| filter.is_empty() || s.to_lowercase().contains(&filter);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // LOCAL
                    let locals: Vec<(String, usize)> = d
                        .refs
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.kind == RefKind::Local && keep(&r.name))
                        .map(|(k, r)| (r.name.clone(), k))
                        .collect();
                    let (open, add) = section(
                        ui,
                        tab,
                        "local",
                        tr!("sidebar.local"),
                        Icon::Laptop,
                        locals.len(),
                        tr!("sidebar.new_branch"),
                    );
                    if add {
                        acts.push(Action::Dialog(Dialog::NewBranch {
                            tab: tab.id,
                            from: "HEAD".into(),
                            from_label: tab.head_label(),
                            name: String::new(),
                            checkout: true,
                        }));
                    }
                    if open {
                        if locals.is_empty() {
                            hint_row(ui, tr!("sidebar.no_branches"));
                        }
                        branch_tree(ui, tab, &locals, 0, "local:", !filter.is_empty(), acts);
                    }
                    ui.add_space(4.0);

                    // REMOTE
                    let (open, add) = section(
                        ui,
                        tab,
                        "remote",
                        tr!("sidebar.remote"),
                        Icon::Cloud,
                        d.remotes.len(),
                        tr!("sidebar.add_remote"),
                    );
                    if add {
                        acts.push(Action::Dialog(Dialog::AddRemote {
                            tab: tab.id,
                            name: String::new(),
                            url: String::new(),
                        }));
                    }
                    if open {
                        if d.remotes.is_empty() {
                            hint_row(ui, tr!("sidebar.no_remotes"));
                        }
                        for rm in &d.remotes {
                            let key = format!("remote:{}", rm.name);
                            let ropen = !tab.collapsed.contains(&key) || !filter.is_empty();
                            let branches: Vec<(String, usize)> = d
                                .refs
                                .iter()
                                .enumerate()
                                .filter(|(_, r)| r.kind == RefKind::Remote && keep(&r.name))
                                .filter_map(|(k, r)| {
                                    let (remote, b) = r.remote_parts()?;
                                    (remote == rm.name).then(|| (b.to_string(), k))
                                })
                                .collect();
                            let resp = side_row(
                                ui,
                                0,
                                if ropen {
                                    Icon::ChevronDown
                                } else {
                                    Icon::ChevronRight
                                },
                                TEXT_FAINT,
                                &rm.name,
                                true,
                                false,
                                &[(branches.len().to_string(), TEXT_DIM)],
                            );
                            let resp = resp.on_hover_text(&rm.fetch_url);
                            if resp.clicked() {
                                if ropen {
                                    tab.collapsed.insert(key.clone());
                                } else {
                                    tab.collapsed.remove(&key);
                                }
                            }
                            let name = rm.name.clone();
                            resp.context_menu(|ui| {
                                ui.set_min_width(240.0);
                                menus::remote_menu(ui, tab, &name, acts);
                            });
                            if ropen {
                                branch_tree(
                                    ui,
                                    tab,
                                    &branches,
                                    1,
                                    &format!("{key}/"),
                                    !filter.is_empty(),
                                    acts,
                                );
                            }
                        }
                    }
                    ui.add_space(4.0);

                    // STASHES
                    let (open, add) = section(
                        ui,
                        tab,
                        "stashes",
                        tr!("sidebar.stashes"),
                        Icon::Box,
                        d.stashes.len(),
                        tr!("sidebar.stash_changes"),
                    );
                    if add {
                        acts.push(Action::Dialog(Dialog::Stash {
                            tab: tab.id,
                            message: String::new(),
                            untracked: true,
                        }));
                    }
                    if open {
                        if d.stashes.is_empty() {
                            hint_row(ui, tr!("sidebar.no_stashes"));
                        }
                        for s in &d.stashes {
                            let active = tab.selection == Selection::Stash(s.hash.clone());
                            let msg = s.message.trim_start_matches("On ").to_string();
                            let resp =
                                side_row(ui, 0, Icon::Box, MODIFIED, &msg, false, active, &[])
                                    .on_hover_text(&s.name);
                            if resp.clicked() {
                                acts.push(Action::Select(Selection::Stash(s.hash.clone())));
                            }
                            let name = s.name.clone();
                            resp.context_menu(|ui| {
                                ui.set_min_width(240.0);
                                menus::stash_menu(ui, tab, &name, acts);
                            });
                        }
                    }
                    ui.add_space(4.0);

                    // TAGS
                    let tags: Vec<(String, usize)> = d
                        .refs
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.kind == RefKind::Tag && keep(&r.name))
                        .map(|(k, r)| (r.name.clone(), k))
                        .collect();
                    let (open, add) = section(
                        ui,
                        tab,
                        "tags",
                        tr!("sidebar.tags"),
                        Icon::Tag,
                        tags.len(),
                        tr!("sidebar.new_tag"),
                    );
                    if add {
                        if let Some(h) = d.head_hash.clone() {
                            acts.push(Action::Dialog(Dialog::NewTag {
                                tab: tab.id,
                                at: h,
                                name: String::new(),
                                message: String::new(),
                                annotated: false,
                            }));
                        }
                    }
                    if open || !filter.is_empty() {
                        for (name, k) in tags.iter().rev() {
                            let r = &d.refs[*k];
                            let resp =
                                side_row(ui, 0, Icon::Tag, MODIFIED, name, false, false, &[]);
                            ref_row_events(&resp, tab, *k, &r.target.clone(), acts);
                        }
                    }
                    ui.add_space(20.0);
                });
        });
}

pub fn ref_row_events(
    resp: &Response,
    tab: &RepoTab,
    k: usize,
    target: &str,
    acts: &mut Vec<Action>,
) {
    if resp.double_clicked() {
        if let Some(job) = commands::checkout_ref_job(tab, k) {
            acts.push(Action::Run(job));
        }
    } else if resp.clicked() {
        acts.push(Action::Reveal(target.to_string()));
    }
    resp.context_menu(|ui| {
        ui.set_min_width(250.0);
        menus::ref_menu(ui, tab, k, acts);
    });
}

/// Branches grouped into folders by the `/` in their names (feature/login -> feature > login).
pub fn branch_tree(
    ui: &mut Ui,
    tab: &mut RepoTab,
    items: &[(String, usize)],
    depth: usize,
    key: &str,
    force_open: bool,
    acts: &mut Vec<Action>,
) {
    let Some(d) = tab.data.clone() else { return };
    let mut folders: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
    let mut leaves: Vec<(String, usize)> = Vec::new();
    for (name, k) in items {
        match name.split_once('/') {
            Some((f, rest)) => folders
                .entry(f.to_string())
                .or_default()
                .push((rest.to_string(), *k)),
            None => leaves.push((name.clone(), *k)),
        }
    }
    for (folder, children) in &folders {
        let fkey = format!("{key}{folder}/");
        let open = force_open || !tab.collapsed.contains(&fkey);
        let resp = side_row(
            ui,
            depth,
            if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            TEXT_FAINT,
            folder,
            false,
            false,
            &[(children.len().to_string(), TEXT_FAINT)],
        );
        if resp.clicked() {
            if open {
                tab.collapsed.insert(fkey.clone());
            } else {
                tab.collapsed.remove(&fkey);
            }
        }
        if open {
            branch_tree(ui, tab, children, depth + 1, &fkey, force_open, acts);
        }
    }
    for (name, k) in &leaves {
        let r = &d.refs[*k];
        let current = r.kind == RefKind::Local && d.head_branch.as_deref() == Some(r.name.as_str());
        let mut right = Vec::new();
        if r.ahead > 0 {
            right.push((format!("↑{}", r.ahead), PINK));
        }
        if r.behind > 0 {
            right.push((format!("↓{}", r.behind), CYAN));
        }
        if r.upstream_gone {
            right.push((tr!("sidebar.upstream_gone").to_string(), WARNING));
        }
        let (icon, color) = match (r.kind, current) {
            (_, true) => (Icon::Check, CYAN),
            (RefKind::Remote, _) => (Icon::Branch, TEXT_FAINT),
            _ => (Icon::Branch, TEXT_DIM),
        };
        let mut resp = side_row(ui, depth, icon, color, name, current, current, &right);
        if let Some(u) = &r.upstream {
            resp = resp.on_hover_text(format!("{}  →  {u}", r.name));
        }
        ref_row_events(&resp, tab, *k, &r.target.clone(), acts);
    }
}
