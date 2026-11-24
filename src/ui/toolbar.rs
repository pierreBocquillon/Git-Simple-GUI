//! The toolbar under the tabs: Undo/Redo, Pull, Push, Branch, Stash, Pop on the left,
//! search and refresh on the right.

use crate::app::{args, Action, Dialog, Job, PullMode, RepoTab, Selection};
use crate::ui::icons::{self, Icon};
use crate::ui::menus;
use crate::ui::theme::{self, *};
use crate::ui::widgets::icon_button;
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, Context, CornerRadius, Frame, Layout, Margin, Pos2,
    Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui,
};

pub fn tool_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    badge: Option<(u32, Color32)>,
    spinning: bool,
) -> Response {
    // At least 58 px, wider when the label needs it (labels differ by language).
    let label_w = ui.fonts(|f| {
        f.layout_no_wrap(label.to_string(), theme::bold(11.5), TEXT)
            .size()
            .x
    });
    let (rect, resp) = ui.allocate_exact_size(
        vec2((label_w + 12.0).max(58.0), 52.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = ui.painter();
    // Each action gets its own sticker color.
    let accent = match icon {
        Icon::Pull | Icon::Fetch => CYAN,
        Icon::Push => PINK,
        Icon::Branch => LANES[4],
        Icon::Stash => MODIFIED,
        Icon::Pop => LANES[5],
        _ => VIOLET,
    };
    let down = enabled && resp.is_pointer_button_down_on();
    let lift = if enabled && resp.hovered() && !down {
        -2.0
    } else if down {
        2.0
    } else {
        0.0
    };
    let tile = Rect::from_center_size(
        pos2(rect.center().x, rect.top() + 16.0 + lift),
        vec2(38.0, 28.0),
    );
    let color = if !enabled { TEXT_FAINT } else { Color32::WHITE };
    if enabled {
        let fill = theme::mix(BG2, accent, if resp.hovered() { 0.4 } else { 0.2 });
        p.rect(
            tile,
            CornerRadius::same(11),
            fill,
            Stroke::new(1.5, alpha(accent, 0.9)),
            StrokeKind::Inside,
        );
    } else {
        p.rect(
            tile,
            CornerRadius::same(11),
            BG2,
            Stroke::new(1.0, OUTLINE),
            StrokeKind::Inside,
        );
    }
    let ic = Rect::from_center_size(tile.center(), vec2(18.0, 18.0));
    if spinning {
        let a = ui.input(|i| i.time) as f32 * 6.0;
        p.circle_stroke(ic.center(), 8.0, Stroke::new(2.0, alpha(CYAN, 0.2)));
        let pts: Vec<Pos2> = (0..12)
            .map(|k| {
                let t = a + k as f32 * 0.12;
                ic.center() + vec2(t.cos(), t.sin()) * 8.0
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(2.0, CYAN)));
    } else {
        icons::draw(p, icon, ic, color);
    }
    p.text(
        pos2(rect.center().x, rect.top() + 43.0),
        Align2::CENTER_CENTER,
        label,
        theme::bold(11.5),
        if enabled { TEXT } else { TEXT_FAINT },
    );
    if let Some((n, c)) = badge {
        if n > 0 {
            let b = Rect::from_center_size(
                pos2(rect.center().x + 19.0, rect.top() + 4.0),
                vec2(if n > 9 { 24.0 } else { 18.0 }, 17.0),
            );
            p.rect(
                b,
                CornerRadius::same(10),
                c,
                Stroke::new(2.0, BG2),
                StrokeKind::Outside,
            );
            p.text(
                b.center(),
                Align2::CENTER_CENTER,
                n.to_string(),
                theme::bold(10.0),
                BG0,
            );
        }
    }
    resp
}

pub fn tool_sep(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(13.0, 52.0), Sense::hover());
    ui.painter().line_segment(
        [
            pos2(rect.center().x, rect.top() + 10.0),
            pos2(rect.center().x, rect.bottom() - 10.0),
        ],
        Stroke::new(1.0, BORDER),
    );
}

pub fn show(ctx: &Context, tab: &mut RepoTab, pull_mode: PullMode, acts: &mut Vec<Action>) {
    egui::TopBottomPanel::top("toolbar")
        .exact_height(70.0)
        .frame(
            Frame::new()
                .fill(BG2)
                .inner_margin(Margin::symmetric(16, 9)),
        )
        .show(ctx, |ui| {
            let full = ui.max_rect();
            ui.painter().line_segment(
                [
                    pos2(full.left() - 16.0, full.bottom() + 9.0),
                    pos2(full.right() + 16.0, full.bottom() + 9.0),
                ],
                Stroke::new(1.0, BORDER),
            );
            let busy = tab.busy.clone().unwrap_or_default();
            let (ahead, behind) = tab
                .data
                .as_ref()
                .and_then(|d| d.head_ref())
                .map(|r| (r.ahead, r.behind))
                .unwrap_or((0, 0));
            let has_data = tab.data.is_some();

            // Left: actions.
            let actions = Rect::from_min_max(full.min, pos2(full.right() - 300.0, full.bottom()));
            let mut c = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(actions)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            c.spacing_mut().item_spacing.x = 2.0;
            let undo_tip = tab
                .undo
                .last()
                .map(|u| tr!("toolbar.undo_tip", label = u.label))
                .unwrap_or_else(|| tr!("toolbar.nothing_to_undo").into());
            let redo_tip = tab
                .redo
                .last()
                .map(|u| tr!("toolbar.redo_tip", label = u.label))
                .unwrap_or_else(|| tr!("toolbar.nothing_to_redo").into());
            if tool_button(
                &mut c,
                Icon::Undo,
                tr!("toolbar.undo"),
                !tab.undo.is_empty(),
                None,
                false,
            )
            .on_hover_text(undo_tip)
            .clicked()
            {
                acts.push(Action::Undo);
            }
            if tool_button(
                &mut c,
                Icon::Redo,
                tr!("toolbar.redo"),
                !tab.redo.is_empty(),
                None,
                false,
            )
            .on_hover_text(redo_tip)
            .clicked()
            {
                acts.push(Action::Redo);
            }
            tool_sep(&mut c);
            let pulling = busy.starts_with("Pull") || busy.starts_with("Fetch");
            let pull_icon = if pull_mode == PullMode::FetchAll {
                Icon::Fetch
            } else {
                Icon::Pull
            };
            let pull_label = if pull_mode == PullMode::FetchAll {
                "Fetch"
            } else {
                "Pull"
            };
            if tool_button(
                &mut c,
                pull_icon,
                pull_label,
                has_data,
                Some((behind, CYAN)),
                pulling,
            )
            .on_hover_text(pull_mode.label())
            .clicked()
            {
                acts.push(Action::Pull(None));
            }
            // Pull options.
            let (arrow, aresp) = c.allocate_exact_size(vec2(16.0, 52.0), Sense::click());
            if aresp.hovered() {
                c.painter().rect_filled(arrow, CornerRadius::same(4), BG3);
            }
            icons::draw(
                c.painter(),
                Icon::ChevronDown,
                Rect::from_center_size(arrow.center() - vec2(0.0, 8.0), vec2(10.0, 10.0)),
                TEXT_DIM,
            );
            let popup = c.id().with("pullmenu");
            if aresp.clicked() {
                c.memory_mut(|m| m.toggle_popup(popup));
            }
            egui::popup_below_widget(
                &c,
                popup,
                &aresp,
                egui::PopupCloseBehavior::CloseOnClick,
                |ui| {
                    ui.set_min_width(280.0);
                    menus::heading(ui, tr!("toolbar.pull_menu_heading"));
                    for m in [
                        PullMode::FetchAll,
                        PullMode::Merge,
                        PullMode::FfOnly,
                        PullMode::Rebase,
                    ] {
                        let icon = if m == pull_mode {
                            Some(Icon::Check)
                        } else {
                            None
                        };
                        if menus::item(ui, icon, m.label()) {
                            acts.push(Action::Pull(Some(m)));
                        }
                    }
                },
            );
            if tool_button(
                &mut c,
                Icon::Push,
                "Push",
                has_data,
                Some((ahead, PINK)),
                busy.starts_with("Push"),
            )
            .on_hover_text(tr!("toolbar.push_tip"))
            .clicked()
            {
                acts.push(Action::Push { force: false });
            }
            tool_sep(&mut c);
            if tool_button(
                &mut c,
                Icon::Branch,
                tr!("toolbar.branch"),
                has_data,
                None,
                false,
            )
            .on_hover_text(tr!("toolbar.branch_tip"))
            .clicked()
            {
                acts.push(Action::Dialog(Dialog::NewBranch {
                    tab: tab.id,
                    from: "HEAD".into(),
                    from_label: tab.head_label(),
                    name: String::new(),
                    checkout: true,
                }));
            }
            tool_sep(&mut c);
            let dirty = tab.has_wip_row();
            let stashes = tab.data.as_ref().map(|d| d.stashes.len()).unwrap_or(0);
            if tool_button(&mut c, Icon::Stash, "Stash", dirty, None, false)
                .on_hover_text(tr!("toolbar.stash_tip"))
                .clicked()
            {
                acts.push(Action::StashQuick);
            }
            if tool_button(&mut c, Icon::Pop, "Pop", stashes > 0, None, false)
                .on_hover_text(tr!("toolbar.pop_tip"))
                .clicked()
            {
                acts.push(Action::Run(Job::new(
                    tab,
                    tr!("toolbar.pop_job"),
                    vec![args(&["stash", "pop"])],
                )));
            }

            // Right: search and refresh.
            let right = Rect::from_min_max(pos2(full.right() - 300.0, full.top()), full.max);
            let mut r = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(right)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            r.spacing_mut().item_spacing.x = 6.0;
            if icon_button(&mut r, Icon::Refresh, 32.0, tr!("toolbar.refresh_tip")).clicked() {
                acts.push(Action::Refresh);
            }
            if tab.search_open {
                if icon_button(&mut r, Icon::Close, 28.0, tr!("toolbar.close_search")).clicked() {
                    acts.push(Action::ToggleSearch);
                }
                let edit = TextEdit::singleline(&mut tab.search)
                    .hint_text(tr!("toolbar.search_hint"))
                    .desired_width(200.0)
                    .margin(Margin::symmetric(8, 6));
                let resp = r.add(edit);
                if tab.search_focus {
                    resp.request_focus();
                    tab.search_focus = false;
                }
                if resp.lost_focus() && r.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Jump to the next match.
                    if let Some(d) = &tab.data {
                        let q = tab.search.trim().to_lowercase();
                        let start = match &tab.selection {
                            Selection::Commit(h) => tab.commit_index(h).map(|i| i + 1).unwrap_or(0),
                            _ => 0,
                        };
                        let n = d.commits.len();
                        if let Some(c) = (0..n).map(|k| &d.commits[(start + k) % n]).find(|c| {
                            c.subject.to_lowercase().contains(&q)
                                || c.author.to_lowercase().contains(&q)
                                || c.hash.starts_with(&q)
                        }) {
                            acts.push(Action::Reveal(c.hash.clone()));
                        }
                    }
                    resp.request_focus();
                }
            } else if icon_button(&mut r, Icon::Search, 32.0, tr!("toolbar.search_tip")).clicked() {
                acts.push(Action::ToggleSearch);
            }
        });
}
