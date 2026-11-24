//! Right panel: what is selected in the graph.
//! - `wip`: the work in progress (staging area and commit box)
//! - `commit`: details and files of a commit
//! - `stash`: contents of a stash

mod commit;
mod stash;
mod wip;

use crate::app::{Action, RepoTab, Selection};
use crate::avatars::Avatars;
use crate::ui::theme::{self, *};
use eframe::egui::{self, Align2, Context, Frame};

pub fn show(ctx: &Context, tab: &mut RepoTab, avatars: &Avatars, acts: &mut Vec<Action>) {
    egui::SidePanel::right("details")
        .resizable(true)
        .default_width(380.0)
        .width_range(300.0..=680.0)
        .frame(Frame::new().fill(BG1))
        .show(ctx, |ui| match tab.selection.clone() {
            Selection::Wip => wip::show(ui, tab, acts),
            Selection::Commit(_) => commit::show(ui, tab, avatars, acts),
            Selection::Stash(h) => stash::show(ui, tab, &h, acts),
            Selection::None => {
                let r = ui.max_rect();
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    tr!("details.select_commit"),
                    theme::semibold(14.0),
                    TEXT_FAINT,
                );
            }
        });
}
