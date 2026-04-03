//! avfs - Virtual filesystem CLI backed by embedded databases.

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use clap::{Parser, Subcommand};

use agentvfs::commands::{self, Output};

#[derive(Parser)]
#[command(name = "avfs")]
#[command(version, about = "Virtual filesystem CLI backed by embedded databases")]
struct Cli {
    /// Use a specific vault instead of the current one
    #[arg(long, global = true)]
    vault: Option<String>,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage vaults
    Vault(commands::vault::VaultArgs),

    /// List directory contents
    Ls(commands::ls::LsArgs),

    /// Read file contents
    Cat(commands::cat::CatArgs),

    /// Manage vault checkpoints
    Checkpoint(commands::checkpoint::CheckpointArgs),

    /// Write content to a file
    Write(commands::write::WriteArgs),

    /// Create a directory
    Mkdir(commands::mkdir::MkdirArgs),

    /// Remove files or directories
    Rm(commands::rm::RmArgs),

    /// Copy files or directories
    Cp(commands::cp::CpArgs),

    /// Move/rename files or directories
    Mv(commands::mv::MvArgs),

    /// Display directory tree
    Tree(commands::tree::TreeArgs),

    /// Show version history for a file
    Log(commands::log::LogArgs),

    /// Restore a file to a specific version
    Checkout(commands::checkout::CheckoutArgs),

    /// Revert a file to its previous version
    Revert(commands::revert::RevertArgs),

    /// Compare files or versions
    Diff(commands::diff::DiffArgs),

    /// Full-text search file contents
    Search(commands::search::SearchArgs),

    /// Search file contents with regex
    Grep(commands::grep::GrepArgs),

    /// Find files by name or attributes
    Find(commands::find::FindArgs),

    /// Add or manage tags on files
    Tag(commands::tag::TagArgs),

    /// Remove a tag from a file
    Untag(commands::untag::UntagArgs),

    /// Get or set file metadata
    Meta(commands::meta::MetaArgs),

    /// Import files from real filesystem
    Import(commands::import::ImportArgs),

    /// Export files to real filesystem
    Export(commands::export::ExportArgs),

    /// Run external command on virtual file
    Exec(commands::exec::ExecArgs),

    /// Show vault storage statistics
    Stats(commands::stats::StatsArgs),

    /// Prune old file versions
    Prune(commands::prune::PruneArgs),

    /// Run garbage collection on orphaned content
    Gc(commands::gc::GcArgs),

    /// Compact database to reclaim space
    Compact(commands::compact::CompactArgs),

    /// Run full maintenance routine
    Maintain(commands::maintain::MaintainArgs),

    /// Manage storage quotas
    Quota(commands::quota::QuotaArgs),

    /// View audit log
    Audit(commands::audit::AuditArgs),

    /// Manage vault snapshots
    Snapshot(commands::snapshot::SnapshotArgs),

    /// Launch interactive shell
    Shell(commands::shell::ShellArgs),

    /// Generate shell aliases
    Aliases(commands::aliases::AliasesArgs),

    /// Mount a vault as a FUSE filesystem
    #[cfg(feature = "fuse")]
    Mount(commands::mount::MountArgs),

    /// Unmount a FUSE filesystem
    #[cfg(feature = "fuse")]
    Unmount(commands::unmount::UnmountArgs),

    /// Auto-mount a vault and launch bash or another command inside it
    #[cfg(feature = "fuse")]
    Proxy(commands::proxy::ProxyArgs),

    /// Print current working directory (always /)
    Pwd,
}

fn main() {
    let cli = Cli::parse();
    let output = Output::new(cli.json);

    let result = match cli.command {
        Commands::Vault(args) => commands::vault::run(args, &output),
        Commands::Ls(args) => commands::ls::run(args, &output, cli.vault),
        Commands::Cat(args) => commands::cat::run(args, &output, cli.vault),
        Commands::Checkpoint(args) => commands::checkpoint::run(args, &output, cli.vault),
        Commands::Write(args) => commands::write::run(args, &output, cli.vault),
        Commands::Mkdir(args) => commands::mkdir::run(args, &output, cli.vault),
        Commands::Rm(args) => commands::rm::run(args, &output, cli.vault),
        Commands::Cp(args) => commands::cp::run(args, &output, cli.vault),
        Commands::Mv(args) => commands::mv::run(args, &output, cli.vault),
        Commands::Tree(args) => commands::tree::run(args, &output, cli.vault),
        Commands::Log(args) => commands::log::run(args, &output, cli.vault),
        Commands::Checkout(args) => commands::checkout::run(args, &output, cli.vault),
        Commands::Revert(args) => commands::revert::run(args, &output, cli.vault),
        Commands::Diff(args) => commands::diff::run(args, &output, cli.vault),
        Commands::Search(args) => commands::search::run(args, &output, cli.vault),
        Commands::Grep(args) => commands::grep::run(args, &output, cli.vault),
        Commands::Find(args) => commands::find::run(args, &output, cli.vault),
        Commands::Tag(args) => commands::tag::run(args, &output, cli.vault),
        Commands::Untag(args) => commands::untag::run(args, &output, cli.vault),
        Commands::Meta(args) => commands::meta::run(args, &output, cli.vault),
        Commands::Import(args) => commands::import::run(args, &output, cli.vault),
        Commands::Export(args) => commands::export::run(args, &output, cli.vault),
        Commands::Exec(args) => commands::exec::run(args, &output, cli.vault),
        Commands::Stats(args) => commands::stats::run(args, &output, cli.vault),
        Commands::Prune(args) => commands::prune::run(args, &output, cli.vault),
        Commands::Gc(args) => commands::gc::run(args, &output, cli.vault),
        Commands::Compact(args) => commands::compact::run(args, &output, cli.vault),
        Commands::Maintain(args) => commands::maintain::run(args, &output, cli.vault),
        Commands::Quota(args) => commands::quota::run(args, &output, cli.vault),
        Commands::Audit(args) => commands::audit::run(args, &output, cli.vault),
        Commands::Snapshot(args) => commands::snapshot::run(args, &output, cli.vault),
        Commands::Shell(args) => commands::shell::run(args, &output),
        Commands::Aliases(args) => commands::aliases::run(args, &output),
        #[cfg(feature = "fuse")]
        Commands::Mount(args) => commands::mount::run(args, &output),
        #[cfg(feature = "fuse")]
        Commands::Unmount(args) => commands::unmount::run(args, &output),
        #[cfg(feature = "fuse")]
        Commands::Proxy(args) => commands::proxy::run(args, &output, cli.vault),
        Commands::Pwd => {
            if cli.json {
                output.print_json(&serde_json::json!({"path": "/"}));
            } else {
                println!("/");
            }
            Ok(())
        }
    };

    if let Err(e) = result {
        output.print_error(&e);
        std::process::exit(1);
    }
}
