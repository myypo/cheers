use anyhow::{Context, Result};
use crop::Rope;

mod collect;
mod format;
mod line_length;
mod print;
mod trivia;
mod unparse;

#[cfg(test)]
mod testing;

pub use format::FormatOptions;

pub fn try_fmt_file(source: &str, options: &format::FormatOptions) -> Result<String> {
    let ast = syn::parse_file(source).context("Failed to parse source")?;
    let rope = Rope::from(source);
    let (mut rope, macros) = collect::collect_macros_from_file(&ast, rope, &options.macro_names);
    let formatted_processed = format::format_source(&mut rope, macros, options);

    Ok(formatted_processed)
}
