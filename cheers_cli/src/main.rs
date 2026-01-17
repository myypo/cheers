mod fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::fmt::FmtArgs;

#[derive(Parser)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Format cheers macros in Rust files
    Fmt(FmtArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fmt(args) => fmt::run(args),
    }
}
