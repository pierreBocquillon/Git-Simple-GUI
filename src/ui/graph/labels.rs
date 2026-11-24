//! Branch and tag pills shown left of each commit. A local branch and its remote copies
//! share one pill (with a laptop and a cloud icon).

use crate::git::{self, RefKind};
use crate::ui::icons::{self, Icon};
use crate::ui::paint::text_fit;
use crate::ui::theme::{self, *};
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind};

pub struct Label {
    pub text: String,
    pub local: bool,
    pub remote: bool,
    pub tag: bool,
    pub head: bool,
    pub ref_index: usize,
}

/// One label per branch name: a local branch and its remote copies share a label.
pub fn group_labels(data: &git::RepoData, idx: &[usize]) -> Vec<Label> {
    let mut out: Vec<Label> = Vec::new();
    for &k in idx {
        let r = &data.refs[k];
        match r.kind {
            RefKind::Local => {
                let head = data.head_branch.as_deref() == Some(r.name.as_str());
                if let Some(l) = out.iter_mut().find(|l| !l.tag && l.text == r.name) {
                    l.local = true;
                    l.head |= head;
                    l.ref_index = k;
                } else {
                    out.push(Label {
                        text: r.name.clone(),
                        local: true,
                        remote: false,
                        tag: false,
                        head,
                        ref_index: k,
                    });
                }
            }
            RefKind::Remote => {
                let short = r
                    .remote_parts()
                    .map(|(_, b)| b.to_string())
                    .unwrap_or_else(|| r.name.clone());
                if let Some(l) = out
                    .iter_mut()
                    .find(|l| !l.tag && (l.text == short || l.text == r.name))
                {
                    l.remote = true;
                } else {
                    out.push(Label {
                        text: r.name.clone(),
                        local: false,
                        remote: true,
                        tag: false,
                        head: false,
                        ref_index: k,
                    });
                }
            }
            RefKind::Tag => out.push(Label {
                text: r.name.clone(),
                local: false,
                remote: false,
                tag: true,
                head: false,
                ref_index: k,
            }),
        }
    }
    // Rename "origin/x" labels that merged with a local "x".
    out.sort_by_key(|l| (!l.head, l.tag, !l.local));
    out
}

pub fn draw_pill(
    p: &egui::Painter,
    left_center: Pos2,
    l: &Label,
    color: Color32,
    max_w: f32,
    ok: bool,
) -> Rect {
    let font = if l.head {
        theme::bold(12.0)
    } else {
        theme::semibold(12.0)
    };
    let icons_n = (l.head as usize) + (l.local as usize) + (l.remote as usize) + (l.tag as usize);
    let icons_w = icons_n as f32 * 15.0;
    let galley = p.layout_no_wrap(l.text.clone(), font.clone(), TEXT);
    let w = (galley.size().x + icons_w + 18.0).min(max_w).max(40.0);
    let rect = Rect::from_min_size(pos2(left_center.x, left_center.y - 10.0), vec2(w, 20.0));
    let a = if ok { 1.0 } else { 0.35 };
    // Light rounded label: soft fill and a thin colored outline.
    let fill = theme::mix(BG0, color, if l.head { 0.36 } else { 0.18 });
    p.rect(
        rect.expand(1.0),
        CornerRadius::same(11),
        alpha(fill, a),
        Stroke::new(1.5, alpha(color, 0.85 * a)),
        StrokeKind::Inside,
    );
    let mut x = rect.left() + 6.0;
    let mut icon = |i: Icon, c: Color32| {
        icons::draw(
            p,
            i,
            Rect::from_center_size(pos2(x + 6.0, rect.center().y), vec2(12.0, 12.0)),
            alpha(c, a),
        );
        x += 15.0;
    };
    if l.head {
        icon(Icon::Check, Color32::WHITE);
    }
    if l.local {
        icon(Icon::Laptop, TEXT);
    }
    if l.remote {
        icon(Icon::Cloud, TEXT);
    }
    if l.tag {
        icon(Icon::Tag, TEXT);
    }
    text_fit(
        p,
        x + 1.0,
        rect.center().y,
        &l.text,
        font,
        alpha(Color32::WHITE, a),
        rect.right() - x - 6.0,
    );
    rect
}
