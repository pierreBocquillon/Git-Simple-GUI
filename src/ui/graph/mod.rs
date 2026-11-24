//! The commit graph table: BRANCH / TAG, GRAPH, COMMIT MESSAGE, AUTHOR, DATE, SHA.
//! - `edges`: drawing the lines between commits
//! - `labels`: the branch / tag pills on the left

mod edges;
mod labels;

use self::edges::{circle_points, draw_edge};
use self::labels::{draw_pill, group_labels};
use crate::app::{commands, Action, RepoTab, Selection};
use crate::avatars::Avatars;
use crate::git::{self, Commit};
use crate::ui::icons::{self, Icon};
use crate::ui::paint::{self, text_fit};
use crate::ui::theme::{self, *};
use crate::ui::{diff_view, menus, widgets};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    Ui,
};

// Row and lane geometry, in pixels.
pub const ROW_H: f32 = 34.0;
pub const ROW_INNER_H: f32 = 24.0;
pub const HEADER_H: f32 = 32.0;
pub const LANE_W: f32 = 24.0;
pub const NODE_R: f32 = 11.0;
pub const AVATAR_R: f32 = 9.0;
pub const MERGE_R: f32 = 6.0;
/// Line width; lines get round ends so no seam shows between rows.
pub const EDGE_W: f32 = 2.5;
pub const CORNER_R: f32 = 8.0;
pub const GRAPH_PAD: f32 = 3.0;
const SHA_W: f32 = 74.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Row(usize),
    Ref(usize),
}

struct Cols {
    refs: Rect,
    graph: Rect,
    msg: Rect,
    author: Rect,
    date: Rect,
    sha: Rect,
}

fn columns(full: Rect, tab: &RepoTab) -> Cols {
    let lanes = tab.graph.lanes.min(14) as f32;
    let graph_w = tab
        .col_graph
        .unwrap_or(lanes * LANE_W + 2.0 * GRAPH_PAD + 14.0)
        .max(60.0);
    let x0 = full.left();
    let refs = Rect::from_min_size(pos2(x0, full.top()), vec2(tab.col_refs, full.height()));
    let graph = Rect::from_min_size(pos2(refs.right(), full.top()), vec2(graph_w, full.height()));
    let sha = Rect::from_min_max(pos2(full.right() - SHA_W, full.top()), full.right_bottom());
    let date = Rect::from_min_max(
        pos2(sha.left() - tab.col_date, full.top()),
        pos2(sha.left(), full.bottom()),
    );
    let author = Rect::from_min_max(
        pos2(date.left() - tab.col_author, full.top()),
        pos2(date.left(), full.bottom()),
    );
    let msg = Rect::from_min_max(
        pos2(graph.right(), full.top()),
        pos2(author.left().max(graph.right() + 80.0), full.bottom()),
    );
    Cols {
        refs,
        graph,
        msg,
        author,
        date,
        sha,
    }
}

fn lane_x(cols: &Cols, lane: usize) -> f32 {
    cols.graph.left() + GRAPH_PAD + 7.0 + LANE_W * lane as f32 + LANE_W / 2.0
}

pub fn show(ui: &mut Ui, tab: &mut RepoTab, avatars: &Avatars, acts: &mut Vec<Action>) {
    if tab.diff.is_some() {
        diff_view::show(ui, tab, acts);
        return;
    }
    let Some(data_rc) = tab.data.clone() else {
        return;
    };
    let data: &git::RepoData = &data_rc;
    let github = crate::avatars::github_repo(data);
    let full = ui.available_rect_before_wrap();
    let cols = columns(full, tab);

    // ---- header -------------------------------------------------------------------------
    let header = Rect::from_min_size(full.min, vec2(full.width(), HEADER_H));
    {
        let p = ui.painter();
        p.rect_filled(header, 0.0, BG2);
        p.line_segment(
            [header.left_bottom(), header.right_bottom()],
            Stroke::new(1.0, BORDER),
        );
        let title = |r: Rect, t: &str| {
            p.text(
                pos2(r.left() + 10.0, header.center().y),
                Align2::LEFT_CENTER,
                t,
                theme::semibold(10.5),
                TEXT_FAINT,
            );
        };
        title(cols.refs, tr!("graph.col_refs"));
        title(cols.graph, tr!("graph.col_graph"));
        title(cols.msg, tr!("graph.col_message"));
        title(cols.author, tr!("graph.col_author"));
        title(cols.date, tr!("graph.col_date"));
        title(cols.sha, "SHA");
        for x in [
            cols.refs.right(),
            cols.graph.right(),
            cols.author.left(),
            cols.date.left(),
            cols.sha.left(),
        ] {
            p.line_segment(
                [pos2(x, header.top() + 7.0), pos2(x, header.bottom() - 7.0)],
                Stroke::new(1.0, BORDER),
            );
        }
    }
    // Column resizing by dragging the header separators.
    let grips = [
        (0, cols.refs.right()),
        (1, cols.graph.right()),
        (2, cols.author.left()),
        (3, cols.date.left()),
    ];
    for (k, x) in grips {
        let r = Rect::from_center_size(pos2(x, header.center().y), vec2(8.0, HEADER_H));
        let resp = ui.interact(r, ui.id().with(("grip", k)), Sense::drag());
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            ui.painter().line_segment(
                [pos2(x, header.top()), pos2(x, header.bottom())],
                Stroke::new(2.0, VIOLET),
            );
        }
        if resp.dragged() {
            let dx = resp.drag_delta().x;
            match k {
                0 => tab.col_refs = (tab.col_refs + dx).clamp(60.0, 420.0),
                1 => tab.col_graph = Some((cols.graph.width() + dx).clamp(60.0, 600.0)),
                2 => tab.col_author = (tab.col_author - dx).clamp(60.0, 320.0),
                _ => tab.col_date = (tab.col_date - dx).clamp(80.0, 260.0),
            }
        }
        if resp.double_clicked() && k == 1 {
            tab.col_graph = None;
        }
    }

    // ---- rows -----------------------------------------------------------------------------
    let wip = !data.status.is_clean();
    let offset = if wip { 1 } else { 0 };
    let n_rows = data.commits.len() + offset + if data.truncated { 1 } else { 0 };
    let body = Rect::from_min_max(pos2(full.left(), header.bottom()), full.max);
    ui.painter().rect_filled(body, 0.0, BG0);

    // Keyboard: arrows move through the history.
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if !typing {
        let (up, down) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if up || down {
            let cur = match &tab.selection {
                Selection::Wip => Some(0),
                Selection::Commit(h) => tab.commit_index(h).map(|i| i + offset),
                _ => None,
            };
            let next = match cur {
                Some(c) if up => c.saturating_sub(1),
                Some(c) => (c + 1).min(data.commits.len() + offset - 1),
                None => 0,
            };
            acts.push(Action::Select(row_selection(tab, next, offset)));
            tab.scroll_to_row = Some(next);
        }
    }

    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(("graph", tab.id))
        .auto_shrink([false, false])
        .max_height(body.height());
    if let Some(row) = tab.scroll_to_row.take() {
        let y = row as f32 * ROW_H;
        let view = tab.view_height.max(ROW_H);
        if y < tab.scroll_offset {
            scroll = scroll.vertical_scroll_offset(y);
        } else if y + ROW_H > tab.scroll_offset + view {
            scroll = scroll.vertical_scroll_offset((y + ROW_H - view + ROW_H).max(0.0));
        }
    }

    let search = tab.search.trim().to_lowercase();
    let mut hits: Vec<(Rect, Hit)> = Vec::new();
    let mut clicked: Option<(Hit, bool, bool)> = None; // (hit, double, secondary)

    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    body_ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let out = scroll.show_rows(&mut body_ui, ROW_H, n_rows, |ui, range| {
        let block = Rect::from_min_size(
            ui.cursor().min,
            vec2(full.width(), ROW_H * range.len() as f32),
        );
        let resp = ui.allocate_rect(block, Sense::click());
        let p = ui.painter().clone();
        let top = block.top() - range.start as f32 * ROW_H; // y of row 0
        let yc = |r: usize| top + r as f32 * ROW_H + ROW_H / 2.0;
        let hover_row = resp.hover_pos().map(|pp| ((pp.y - top) / ROW_H) as usize);
        let selected_row = match &tab.selection {
            Selection::Wip => Some(0),
            Selection::Commit(h) => tab.commit_index(h).map(|i| i + offset),
            _ => None,
        };
        let head_idx = data.head_hash.as_ref().and_then(|h| tab.commit_index(h));
        let commit_at = |r: usize| -> Option<(&Commit, usize)> {
            if r < offset {
                None
            } else {
                data.commits.get(r - offset).map(|c| (c, r - offset))
            }
        };
        let matches = |c: &Commit| -> bool {
            search.is_empty()
                || c.subject.to_lowercase().contains(&search)
                || c.author.to_lowercase().contains(&search)
                || c.hash.starts_with(&search)
        };

        // Row backgrounds.
        for r in range.clone() {
            let row = Rect::from_min_size(
                pos2(block.left(), top + r as f32 * ROW_H),
                vec2(block.width(), ROW_H),
            );
            let lane_color = match commit_at(r) {
                Some((_, i)) => theme::lane(tab.graph.rows[i].lane),
                None => head_idx
                    .map(|i| theme::lane(tab.graph.rows[i].lane))
                    .unwrap_or(VIOLET),
            };
            if Some(r) == selected_row {
                p.rect_filled(row, 0.0, theme::mix(BG0, lane_color, 0.09));
                p.rect_filled(
                    Rect::from_min_size(row.min + vec2(0.0, 6.0), vec2(3.0, ROW_H - 12.0)),
                    CornerRadius::same(2),
                    lane_color,
                );
            } else if Some(r) == hover_row {
                p.rect_filled(row, 0.0, theme::mix(BG0, Color32::WHITE, 0.025));
            }
            if !search.is_empty() {
                if let Some((c, _)) = commit_at(r) {
                    if matches(c) {
                        p.rect_filled(row, 0.0, alpha(CYAN, 0.07));
                    }
                }
            }
            // The commit's colored band, from its dot to the message column.
            if let Some((_, i)) = commit_at(r) {
                let x = lane_x(&cols, tab.graph.rows[i].lane);
                let band = Rect::from_min_max(
                    pos2(x, yc(r) - ROW_INNER_H / 2.0),
                    pos2(cols.graph.right(), yc(r) + ROW_INNER_H / 2.0),
                );
                p.rect_filled(
                    band,
                    CornerRadius::same(12),
                    theme::mix(
                        BG0,
                        lane_color,
                        if Some(r) == selected_row { 0.2 } else { 0.07 },
                    ),
                );
            }
        }

        // Graph lines (clipped to the graph column).
        let gp = p.with_clip_rect(cols.graph.intersect(p.clip_rect()));
        let first = range.start.saturating_sub(1).max(offset);
        for r in first..range.end {
            let Some((_, i)) = commit_at(r) else { continue };
            for e in &tab.graph.rows[i].edges {
                let color = theme::lane(e.color_lane());
                draw_edge(
                    &gp,
                    lane_x(&cols, e.from),
                    yc(r),
                    lane_x(&cols, e.to),
                    yc(r + 1),
                    e.kind,
                    color,
                );
            }
        }
        // WIP dashed link to the checked-out commit.
        if wip {
            if let Some(hi) = head_idx {
                let x = lane_x(&cols, tab.graph.rows[hi].lane);
                let (y0, y1) = (yc(0), yc(hi + offset));
                if range.start <= hi + offset {
                    let color = theme::lane(tab.graph.rows[hi].lane);
                    for s in Shape::dashed_line(
                        &[pos2(x, y0), pos2(x, y1)],
                        Stroke::new(EDGE_W, alpha(color, 0.8)),
                        4.0,
                        3.0,
                    ) {
                        gp.add(s);
                    }
                }
            }
        }

        // Rows: dots, labels and text.
        for r in range.clone() {
            let y = yc(r);
            let dim = |c: Color32, ok: bool| if ok { c } else { alpha(c, 0.3) };
            if r < offset {
                // Work in progress.
                let lane = head_idx.map(|i| tab.graph.rows[i].lane).unwrap_or(0);
                let color = theme::lane(lane);
                let x = lane_x(&cols, lane);
                gp.circle_filled(pos2(x, y), NODE_R + 1.0, BG0);
                for s in Shape::dashed_line(
                    &circle_points(pos2(x, y), NODE_R - 1.0, 28),
                    Stroke::new(EDGE_W, color),
                    3.0,
                    2.5,
                ) {
                    gp.add(s);
                }
                icons::draw(
                    &gp,
                    Icon::Plus,
                    Rect::from_center_size(pos2(x, y), vec2(10.0, 10.0)),
                    color,
                );
                let st = &data.status;
                let mut x = cols.msg.left() + 10.0;
                let w = text_fit(
                    &p,
                    x,
                    y,
                    "// WIP",
                    theme::mono(12.5),
                    TEXT_FAINT,
                    cols.msg.width() - 20.0,
                );
                x += w + 14.0;
                for (n, color, icon) in [
                    (
                        st.unstaged.len() + st.conflicted.len(),
                        MODIFIED,
                        Icon::File,
                    ),
                    (st.staged.len(), ADDED, Icon::Check),
                ] {
                    if n == 0 || x > cols.msg.right() - 50.0 {
                        continue;
                    }
                    let chip = Rect::from_min_size(pos2(x, y - 9.0), vec2(42.0, 18.0));
                    p.rect(
                        chip,
                        CornerRadius::same(9),
                        theme::mix(BG0, color, 0.16),
                        Stroke::new(1.0, alpha(color, 0.6)),
                        StrokeKind::Inside,
                    );
                    icons::draw(
                        &p,
                        icon,
                        Rect::from_center_size(pos2(chip.left() + 11.0, y), vec2(11.0, 11.0)),
                        color,
                    );
                    p.text(
                        pos2(chip.left() + 21.0, y),
                        Align2::LEFT_CENTER,
                        n.to_string(),
                        theme::semibold(11.5),
                        color,
                    );
                    x += 48.0;
                }
                hits.push((
                    Rect::from_min_size(
                        pos2(block.left(), y - ROW_H / 2.0),
                        vec2(block.width(), ROW_H),
                    ),
                    Hit::Row(r),
                ));
                continue;
            }
            let Some((c, i)) = commit_at(r) else {
                // "Load more" row.
                p.text(
                    pos2(cols.msg.left() + 10.0, y),
                    Align2::LEFT_CENTER,
                    tr!("graph.load_more"),
                    theme::semibold(12.5),
                    CYAN,
                );
                hits.push((
                    Rect::from_min_size(
                        pos2(block.left(), y - ROW_H / 2.0),
                        vec2(block.width(), ROW_H),
                    ),
                    Hit::Row(r),
                ));
                continue;
            };
            let ok = matches(c);
            let g = &tab.graph.rows[i];
            let color = theme::lane(g.lane);
            let x = lane_x(&cols, g.lane);
            let center = pos2(x, y);
            let is_head = Some(i) == head_idx;
            if is_head {
                paint::glow(&gp, center, NODE_R, color, 1.6);
            }
            if c.parents.len() > 1 {
                gp.circle_filled(center, MERGE_R + 1.5, BG0);
                gp.circle_filled(center, MERGE_R, dim(color, ok));
                gp.circle_filled(
                    center + vec2(-1.6, -1.8),
                    1.8,
                    dim(Color32::from_white_alpha(170), ok),
                );
            } else {
                // Dot: a thin gap ring, a colored ring, then the author's profile picture,
                // or a face with initials and a small highlight.
                gp.circle_filled(center, NODE_R + 1.5, BG0);
                gp.circle_filled(center, NODE_R, dim(color, ok));
                if let Some(t) = avatars.get(&c.email, &tab.path, github.as_deref()) {
                    widgets::picture_circle(&gp, center, AVATAR_R, &t, dim(Color32::WHITE, ok));
                } else {
                    gp.circle_filled(center, AVATAR_R, dim(theme::mix(BG0, color, 0.55), ok));
                    gp.circle_filled(
                        center + vec2(-6.6, -5.6),
                        1.6,
                        dim(Color32::from_white_alpha(150), ok),
                    );
                    gp.text(
                        center + vec2(0.0, 0.5),
                        Align2::CENTER_CENTER,
                        git::initials(&c.author),
                        theme::bold(9.5),
                        dim(Color32::WHITE, ok),
                    );
                }
            }
            if Some(r) == selected_row {
                gp.circle_stroke(center, NODE_R + 4.0, Stroke::new(2.5, Color32::WHITE));
            }

            // Branch and tag labels.
            if let Some(idx) = data.refs_by_commit.get(&c.hash) {
                let labels = group_labels(data, idx);
                if let Some(first) = labels.first() {
                    let more = labels.len() - 1;
                    let max_w = cols.refs.width() - 14.0 - if more > 0 { 30.0 } else { 0.0 };
                    let pill =
                        draw_pill(&p, pos2(cols.refs.left() + 6.0, y), first, color, max_w, ok);
                    hits.push((pill, Hit::Ref(first.ref_index)));
                    if more > 0 {
                        let chip = Rect::from_min_size(
                            pos2(pill.right() + 4.0, y - 9.0),
                            vec2(26.0, 18.0),
                        );
                        p.rect(
                            chip,
                            CornerRadius::same(9),
                            theme::mix(BG0, color, 0.2),
                            Stroke::new(1.0, alpha(color, 0.7)),
                            StrokeKind::Inside,
                        );
                        p.text(
                            chip.center(),
                            Align2::CENTER_CENTER,
                            format!("+{more}"),
                            theme::semibold(10.5),
                            TEXT,
                        );
                        hits.push((chip, Hit::Ref(labels[1].ref_index)));
                        if resp
                            .hover_pos()
                            .map(|pp| chip.contains(pp))
                            .unwrap_or(false)
                        {
                            let names: Vec<String> =
                                labels.iter().map(|l| l.text.clone()).collect();
                            resp.clone().on_hover_ui_at_pointer(|ui| {
                                for n in names {
                                    ui.label(n);
                                }
                            });
                        }
                    }
                    // Thin connector from the label to the dot.
                    let from = if more > 0 {
                        pill.right() + 32.0
                    } else {
                        pill.right() + 2.0
                    };
                    let to = x - NODE_R - 1.0;
                    if to > from {
                        p.line_segment(
                            [pos2(from, y), pos2(to, y)],
                            Stroke::new(1.5, alpha(color, if ok { 0.65 } else { 0.2 })),
                        );
                    }
                }
            }

            // Text columns.
            let tc = dim(TEXT, ok);
            text_fit(
                &p,
                cols.msg.left() + 10.0,
                y,
                &c.subject,
                theme::regular(13.0),
                tc,
                cols.msg.width() - 18.0,
            );
            text_fit(
                &p,
                cols.author.left() + 10.0,
                y,
                &c.author,
                theme::regular(12.5),
                dim(TEXT_DIM, ok),
                cols.author.width() - 16.0,
            );
            text_fit(
                &p,
                cols.date.left() + 10.0,
                y,
                &c.date,
                theme::regular(12.5),
                dim(TEXT_DIM, ok),
                cols.date.width() - 16.0,
            );
            text_fit(
                &p,
                cols.sha.left() + 10.0,
                y,
                c.short(),
                theme::mono(12.0),
                dim(TEXT_FAINT, ok),
                SHA_W - 14.0,
            );
            hits.push((
                Rect::from_min_size(
                    pos2(block.left(), y - ROW_H / 2.0),
                    vec2(block.width(), ROW_H),
                ),
                Hit::Row(r),
            ));
        }

        // Which element was clicked: labels first (they sit on top of their row).
        let pick = |pos: Pos2| -> Option<Hit> {
            hits.iter()
                .filter(|(r, h)| matches!(h, Hit::Ref(_)) && r.contains(pos))
                .map(|(_, h)| h.clone())
                .next()
                .or_else(|| {
                    hits.iter()
                        .find(|(r, _)| r.contains(pos))
                        .map(|(_, h)| h.clone())
                })
        };
        if let Some(pos) = resp.interact_pointer_pos() {
            if resp.clicked() || resp.double_clicked() || resp.secondary_clicked() {
                if let Some(h) = pick(pos) {
                    clicked = Some((h, resp.double_clicked(), resp.secondary_clicked()));
                }
            }
        }
        if resp.hovered() {
            if let Some(pos) = resp.hover_pos() {
                if pick(pos).is_some() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        }
        resp
    });
    tab.scroll_offset = out.state.offset.y;
    tab.view_height = out.inner_rect.height();
    let resp = out.inner;

    if let Some((hit, double, secondary)) = clicked {
        let row_of = |h: &Hit| -> Option<usize> {
            match h {
                Hit::Row(r) => Some(*r),
                Hit::Ref(k) => data
                    .refs
                    .get(*k)
                    .and_then(|rf| tab.commit_index(&rf.target))
                    .map(|i| i + offset),
            }
        };
        if let Some(r) = row_of(&hit) {
            if r >= data.commits.len() + offset {
                acts.push(Action::LoadMore);
            } else {
                acts.push(Action::Select(row_selection(tab, r, offset)));
            }
        }
        if secondary {
            tab.ctx_target = Some(hit.clone());
        }
        if double {
            if let Hit::Ref(k) = hit {
                if let Some(job) = commands::checkout_ref_job(tab, k) {
                    acts.push(Action::Run(job));
                }
            }
        }
    }
    let target = tab.ctx_target.clone();
    resp.context_menu(|ui| {
        ui.set_min_width(250.0);
        match &target {
            Some(Hit::Ref(k)) => menus::ref_menu(ui, tab, *k, acts),
            Some(Hit::Row(r)) if *r < offset => menus::wip_menu(ui, tab, acts),
            Some(Hit::Row(r)) => {
                if let Some(c) = data.commits.get(r - offset) {
                    menus::commit_menu(ui, tab, &c.hash.clone(), acts);
                }
            }
            None => {
                ui.close_menu();
            }
        }
    });

    if data.commits.is_empty() && !wip {
        let c = body.center();
        ui.painter().text(
            c - vec2(0.0, 12.0),
            Align2::CENTER_CENTER,
            tr!("graph.no_commits"),
            theme::semibold(16.0),
            TEXT_DIM,
        );
        ui.painter().text(
            c + vec2(0.0, 14.0),
            Align2::CENTER_CENTER,
            tr!("graph.no_commits_hint"),
            theme::regular(13.0),
            TEXT_FAINT,
        );
    }
}

fn row_selection(tab: &RepoTab, r: usize, offset: usize) -> Selection {
    if r < offset {
        Selection::Wip
    } else {
        tab.data
            .as_ref()
            .and_then(|d| d.commits.get(r - offset))
            .map(|c| Selection::Commit(c.hash.clone()))
            .unwrap_or(Selection::None)
    }
}
