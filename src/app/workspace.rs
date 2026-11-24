//! Workspaces: named groups of tabs. Only the tabs of the current workspace are shown; the
//! others stay open in the background and come back as they were when you switch to them.
//!
//! Every `RepoTab` belongs to exactly one workspace (`RepoTab::workspace` holds its id).

use super::GitApp;

pub struct Workspace {
    pub id: u64,
    pub name: String,
    /// The tab to show when coming back to this workspace.
    pub focus: Option<u64>,
}

impl GitApp {
    /// The workspace on screen.
    pub fn workspace(&self) -> &Workspace {
        &self.workspaces[self.ws]
    }

    /// Positions (in `self.tabs`) of the tabs of the current workspace, in display order.
    pub fn visible_tabs(&self) -> Vec<usize> {
        self.tabs_of(self.workspace().id)
    }

    /// Positions (in `self.tabs`) of the tabs of one workspace.
    pub fn tabs_of(&self, workspace: u64) -> Vec<usize> {
        (0..self.tabs.len())
            .filter(|&i| self.tabs[i].workspace == workspace)
            .collect()
    }

    /// Put workspace `i` on screen and show the tab it was on last time.
    pub(super) fn switch_workspace(&mut self, i: usize) {
        if i >= self.workspaces.len() {
            return;
        }
        let current = self.active.map(|a| self.tabs[a].id);
        self.workspaces[self.ws].focus = current;
        self.ws = i;
        self.show_focus();
        if let Some(t) = self.tab_mut() {
            t.reload();
        }
        self.save_state();
    }

    /// Make `active` point at the remembered tab of the current workspace (or its first tab).
    pub(super) fn show_focus(&mut self) {
        let ws = self.workspace();
        let visible = self.tabs_of(ws.id);
        let remembered = ws
            .focus
            .and_then(|id| visible.iter().copied().find(|&i| self.tabs[i].id == id));
        self.active = remembered.or_else(|| visible.first().copied());
    }

    /// Create a workspace and switch to it, optionally moving a tab into it.
    pub(super) fn new_workspace(&mut self, name: &str, with_tab: Option<u64>) {
        let id = self.next_id;
        self.next_id += 1;
        self.workspaces.push(Workspace {
            id,
            name: name.trim().to_string(),
            focus: with_tab,
        });
        if let Some(tab) = with_tab {
            self.move_tab(tab, id);
        }
        self.switch_workspace(self.workspaces.len() - 1);
    }

    pub(super) fn rename_workspace(&mut self, id: u64, name: &str) {
        if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == id) {
            w.name = name.trim().to_string();
        }
        self.save_state();
    }

    /// Remove a workspace and close its tabs (the repositories on disk are not touched).
    /// The last workspace cannot be removed.
    pub(super) fn delete_workspace(&mut self, id: u64) {
        if self.workspaces.len() < 2 {
            return;
        }
        let Some(i) = self.workspaces.iter().position(|w| w.id == id) else {
            return;
        };
        let active_id = self.active.map(|a| self.tabs[a].id);
        self.tabs.retain(|t| t.workspace != id);
        self.workspaces.remove(i);
        if i == self.ws {
            self.ws = i.min(self.workspaces.len() - 1);
            self.show_focus();
            if let Some(t) = self.tab_mut() {
                t.reload();
            }
        } else {
            if i < self.ws {
                self.ws -= 1;
            }
            // Tabs were removed from the list: find the active tab again.
            self.active = active_id.and_then(|id| self.tabs.iter().position(|t| t.id == id));
        }
        self.save_state();
    }

    /// Move a tab to another workspace. When it was the tab on screen, the next tab of the
    /// current workspace is shown instead.
    pub(super) fn move_tab(&mut self, tab: u64, workspace: u64) {
        let Some(i) = self.tabs.iter().position(|t| t.id == tab) else {
            return;
        };
        let was_active = self.active == Some(i);
        let local = self.visible_tabs().iter().position(|&j| j == i);
        self.tabs[i].workspace = workspace;
        if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == workspace) {
            w.focus = Some(tab);
        }
        if was_active {
            let visible = self.visible_tabs();
            self.active = local
                .and_then(|l| visible.get(l.min(visible.len().saturating_sub(1))))
                .copied();
        }
        self.save_state();
    }

    /// Move a tab of the current workspace to slot `to` (0 = first; the number of tabs = last),
    /// as when it is dragged and dropped in the tab bar. Other workspaces are not affected.
    pub(super) fn reorder_tab(&mut self, tab: u64, to: usize) {
        let visible = self.visible_tabs();
        let Some(from) = visible.iter().position(|&i| self.tabs[i].id == tab) else {
            return;
        };
        // Dropped on its own place (just before or just after itself): nothing to do.
        if to == from || to == from + 1 {
            return;
        }
        let active_id = self.active.map(|a| self.tabs[a].id);
        move_in_subset(&mut self.tabs, &visible, from, to);
        // `active` is a position in `self.tabs`, which just changed.
        self.active = active_id.and_then(|id| self.tabs.iter().position(|t| t.id == id));
        self.save_state();
    }
}

/// Move one item of a subset of `items` (the positions `subset`, in order) so it becomes the
/// `to`-th item of that subset. Items outside the subset keep their relative order.
fn move_in_subset<T>(items: &mut Vec<T>, subset: &[usize], from: usize, to: usize) {
    let moved = items.remove(subset[from]);
    // Positions of the other subset items, now that one item is gone.
    let rest: Vec<usize> = subset
        .iter()
        .filter(|&&i| i != subset[from])
        .map(|&i| if i > subset[from] { i - 1 } else { i })
        .collect();
    let slot = if to > from { to - 1 } else { to };
    let at = match rest.get(slot) {
        Some(&i) => i,
        None => rest.last().map_or(items.len(), |&i| i + 1),
    };
    items.insert(at, moved);
}

#[cfg(test)]
mod tests {
    use super::move_in_subset;

    #[test]
    fn moves_within_the_subset_only() {
        // a1 b1 a2 a3 b2: workspace "a" is at positions 0, 2, 3.
        let mut items = vec!["a1", "b1", "a2", "a3", "b2"];
        move_in_subset(&mut items, &[0, 2, 3], 0, 3); // a1 to the end of "a"
        assert_eq!(items, ["b1", "a2", "a3", "a1", "b2"]);

        let mut items = vec!["a1", "b1", "a2", "a3", "b2"];
        move_in_subset(&mut items, &[0, 2, 3], 2, 0); // a3 to the front of "a"
        assert_eq!(items, ["a3", "a1", "b1", "a2", "b2"]);

        let mut items = vec!["a1", "a2", "a3"];
        move_in_subset(&mut items, &[0, 1, 2], 0, 2); // a1 between a2 and a3
        assert_eq!(items, ["a2", "a1", "a3"]);
    }
}
