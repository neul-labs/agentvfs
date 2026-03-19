//! shell command - launch interactive shell.

use clap::Args;

use crate::commands::Output;
use crate::error::Result;
use crate::shell::Shell;

#[derive(Args)]
pub struct ShellArgs {
    /// Disable command history persistence
    #[arg(long)]
    pub no_history: bool,
}

pub fn run(args: ShellArgs, _output: &Output) -> Result<()> {
    let persist_history = !args.no_history;
    let mut shell = Shell::new(persist_history)?;
    shell.run()
}
