pub mod diff;
pub mod pull;
pub mod push;
pub mod status;

use crate::Result;

pub use diff::compute_diff;
pub use status::get_status;

pub fn diff() -> Result<()> {
    diff::show_diff()
}

pub fn status() -> Result<()> {
    status::show_status()
}

pub fn push(dry_run: bool, force: bool) -> Result<()> {
    push::execute(dry_run, force)
}

pub fn pull(dry_run: bool, force: bool) -> Result<()> {
    pull::execute(dry_run, force)
}
