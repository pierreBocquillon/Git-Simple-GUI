//! Everything that talks to git. No UI code here.
//!
//! - `command`    runs the `git` executable
//! - `model`      the data types (commits, refs, status, stashes, remotes…)
//! - `repository` reads a repository into those types
//! - `diff`       file diffs and their parser
//! - `names`      initials and branch-name helpers

mod command;
mod diff;
mod model;
mod names;
mod repository;

pub use command::*;
pub use diff::*;
pub use model::*;
pub use names::*;
pub use repository::*;
