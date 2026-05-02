mod fmt;
mod subsecond;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::fmt::FmtArgs;
use crate::subsecond::SubsecondArgs;

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
    /// Run a Cheers app through Subsecond hot-patching and morph hot reloads
    Subsecond(SubsecondArgs),
}

fn main() -> Result<()> {
    let args = std::env::args().enumerate().filter_map(|(i, arg)| {
        if i == 1 && arg == "cheers" {
            None
        } else {
            Some(arg)
        }
    });
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::Fmt(args) => fmt::run(args),
        Commands::Subsecond(args) => subsecond::run(args),
    }
}
