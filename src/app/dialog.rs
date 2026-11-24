//! The data behind each dialog window (what the person is typing). Drawn by `ui::dialogs`.

use super::Job;

#[derive(Clone, Debug)]
pub enum Dialog {
    NewBranch {
        tab: u64,
        from: String,
        from_label: String,
        name: String,
        checkout: bool,
    },
    RenameBranch {
        tab: u64,
        old: String,
        name: String,
    },
    NewTag {
        tab: u64,
        at: String,
        name: String,
        message: String,
        annotated: bool,
    },
    AddRemote {
        tab: u64,
        name: String,
        url: String,
    },
    EditRemote {
        tab: u64,
        old: String,
        name: String,
        url: String,
    },
    Stash {
        tab: u64,
        message: String,
        untracked: bool,
    },
    Clone {
        url: String,
        parent: String,
        name: String,
    },
    Init {
        parent: String,
        name: String,
    },
    /// Create (`id: None`) or rename a workspace. `with_tab` moves that tab into the new one.
    Workspace {
        id: Option<u64>,
        name: String,
        with_tab: Option<u64>,
    },
    DeleteWorkspace {
        id: u64,
        name: String,
        tabs: usize,
    },
    /// Connect a GitHub account (paste a token) or disconnect it.
    /// A newer version is available: what is new, and install it.
    Update,
    GitHub {
        token: String,
    },
    Identity {
        name: String,
        email: String,
    },
    /// Change the message of a commit of the current branch.
    EditCommit {
        tab: u64,
        hash: String,
        summary: String,
        body: String,
        /// Already on a remote branch: pushing it again will need a force push.
        pushed: bool,
    },
    Confirm {
        title: String,
        text: String,
        ok_label: String,
        danger: bool,
        job: Job,
    },
}
