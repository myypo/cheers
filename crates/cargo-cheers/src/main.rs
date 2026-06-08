mod collect;
mod fmt;
mod format;
mod line_length;
mod print;
mod subsecond;
#[cfg(test)]
mod testing;
mod trivia;
mod unparse;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crop::Rope;

use crate::fmt::FmtArgs;
use crate::subsecond::SubsecondArgs;

pub use format::FormatOptions;

pub fn try_fmt_file(source: &str, options: &format::FormatOptions) -> Result<String> {
    let ast = syn::parse_file(source).context("Failed to parse source")?;
    let rope = Rope::from(source);
    let (mut rope, macros) = collect::collect_macros_from_file(&ast, rope, &options.macro_names);
    let formatted_processed = format::format_source(&mut rope, macros, options);

    Ok(formatted_processed)
}

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
