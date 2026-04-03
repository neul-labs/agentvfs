//! unmount command - unmount a FUSE filesystem.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::mount::util::unmount_path;

#[derive(Args)]
pub struct UnmountArgs {
    /// Mount point to unmount
    pub mountpoint: String,

    /// Lazy unmount (detach now, cleanup later)
    #[arg(short, long)]
    pub lazy: bool,
}

#[derive(Serialize)]
struct UnmountOutput {
    mountpoint: String,
    success: bool,
}

pub fn run(args: UnmountArgs, output: &Output) -> Result<()> {
    unmount_path(std::path::Path::new(&args.mountpoint), args.lazy)?;

    if output.is_json() {
        output.print_json(&UnmountOutput {
            mountpoint: args.mountpoint,
            success: true,
        });
    } else {
        println!("Unmounted {}", args.mountpoint);
    }

    Ok(())
}
