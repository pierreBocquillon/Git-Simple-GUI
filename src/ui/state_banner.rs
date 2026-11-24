//! The orange banner shown above the graph while a merge, rebase, cherry-pick or revert is
//! in progress, with Continue / Abort buttons.

use crate::app::{commands, Action, Job, RepoTab};
use crate::git::RepoState;
use crate::ui::icons::{self, Icon};
use crate::ui::theme::{self, *};
use crate::ui::widgets::secondary_button;
use eframe::egui::{self, pos2, vec2, Align, Align2, Layout, Rect, Sense, Stroke, Ui};

pub fn show(ui: &mut Ui, tab: &RepoTab, acts: &mut Vec<Action>) {
    let Some(d) = &tab.data else { return };
    let Some((title, cont, abort)) = commands::state_commands(d.state) else {
        return;
    };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, alpha(WARNING, 0.12));
    p.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, alpha(WARNING, 0.5)),
    );
    icons::draw(
        p,
        Icon::Warning,
        Rect::from_center_size(pos2(rect.left() + 22.0, rect.center().y), vec2(18.0, 18.0)),
        WARNING,
    );
    // The git operation's name, the same English word in every language (used in job labels).
    let op = match d.state {
        RepoState::Merging => "Merge",
        RepoState::Rebasing => "Rebase",
        RepoState::CherryPicking => "Cherry-pick",
        RepoState::Reverting => "Revert",
        RepoState::Clean => "",
    };
    let n = d.status.conflicted.len();
    let text = if n > 0 {
        tr!("banner.conflicts", title = title, count = n)
    } else {
        tr!("banner.resolved", title = title)
    };
    p.text(
        pos2(rect.left() + 40.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme::semibold(13.0),
        TEXT,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(12.0, 6.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    if secondary_button(&mut child, tr!("banner.abort"), Some(Icon::Close), REMOVED).clicked() {
        acts.push(Action::Run(Job::new(
            tab,
            tr!("banner.aborted", op = op),
            vec![abort],
        )));
    }
    if n == 0
        && secondary_button(&mut child, tr!("banner.continue"), Some(Icon::Check), ADDED).clicked()
    {
        acts.push(Action::Run(Job::new(
            tab,
            tr!("banner.continued", op = op),
            vec![cont],
        )));
    }
}
