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
}
