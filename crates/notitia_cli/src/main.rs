mod commands;
mod config;
mod extract;
mod snapshot;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "notitia", about = "Notitia schema migration tool")]
struct Cli {
    /// Show full cargo output during schema extraction
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Use a temporary directory instead of .notitia/ (no build cache)
    #[arg(long, global = true)]
    tmp: bool,

    /// Workspace member crate to extract schemas from (e.g. `--crate my_app`)
    #[arg(short = 'c', long = "crate", global = true)]
    krate: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Save the current schema as a YAML snapshot
    Snapshot,
    /// Check compatibility of all snapshots against the current schema
    Check,
    /// Initialize migration scaffolding in the current project
    Init,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Snapshot => commands::snapshot::run(cli.verbose, cli.tmp, cli.krate.as_deref())?,
        Commands::Check => commands::check::run(cli.verbose, cli.tmp, cli.krate.as_deref())?,
        Commands::Init => commands::init::run()?,
    }
    Ok(())
}
