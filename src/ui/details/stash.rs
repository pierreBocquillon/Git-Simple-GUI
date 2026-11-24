//! Contents of the selected stash, with Pop / Apply / Delete.

use crate::app::{args, Action, Dialog, Job, RepoTab};
use crate::git::DiffSource;
use crate::ui::icons::Icon;
use crate::ui::theme::{self, *};
use crate::ui::widgets::{file_row, panel_header, secondary_button};
use eframe::egui::{self, vec2, Frame, Margin, RichText, Ui};

pub fn show(ui: &mut Ui, tab: &mut RepoTab, hash: &str, acts: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    panel_header(ui, "STASH", MODIFIED);
    let Some(s) = tab
        .data
        .as_ref()
        .and_then(|d| d.stashes.iter().find(|s| s.hash == hash).cloned())
    else {
        return;
    };
    Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
        ui.label(
            RichText::new(&s.name)
                .font(theme::mono(12.5))
                .color(MODIFIED),
        );
        ui.add(
            egui::Label::new(
                RichText::new(&s.message)
                    .font(theme::semibold(15.5))
                    .color(TEXT),
            )
            .wrap(),
        );
        ui.horizontal(|ui| {
            if secondary_button(ui, "Pop", Some(Icon::Pop), CYAN).clicked() {
                acts.push(Action::Run(Job::new(
                    tab,
                    tr!("stash.pop_job", name = s.name),
                    vec![args(&["stash", "pop", &s.name])],
                )));
            }
            if secondary_button(ui, tr!("stash.apply"), Some(Icon::Box), VIOLET).clicked() {
                acts.push(Action::Run(Job::new(
                    tab,
                    tr!("stash.apply_job", name = s.name),
                    vec![args(&["stash", "apply", &s.name])],
                )));
            }
            if secondary_button(ui, tr!("stash.delete"), Some(Icon::Close), REMOVED).clicked() {
                acts.push(Action::Dialog(Dialog::Confirm {
                    title: tr!("stash.delete_title").into(),
                    text: tr!("stash.delete_text", name = s.name),
                    ok_label: tr!("stash.delete").into(),
                    danger: true,
                    job: Job::new(
                        tab,
                        tr!("stash.delete_job", name = s.name),
                        vec![args(&["stash", "drop", &s.name])],
                    ),
                }));
            }
        });
    });
    let selected_path = tab.diff.as_ref().map(|(f, _)| f.path.clone());
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for f in &tab.stash_files {
                let sel = selected_path.as_deref() == Some(f.path.as_str());
                let (resp, _) = file_row(ui, f, sel, &[]);
                if resp.clicked() {
                    acts.push(Action::OpenDiff(
                        DiffSource::Stash {
                            name: s.name.clone(),
                        },
                        f.path.clone(),
                    ));
                }
            }
        });
}
