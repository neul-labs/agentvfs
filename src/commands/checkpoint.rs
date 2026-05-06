//! checkpoint command - alias for snapshot management.

pub use crate::commands::snapshot::SnapshotArgs as CheckpointArgs;

use crate::commands::Output;
use crate::error::Result;

pub fn run(args: CheckpointArgs, output: &Output, vault: Option<String>) -> Result<()> {
    crate::commands::snapshot::run(args, output, vault)
}
