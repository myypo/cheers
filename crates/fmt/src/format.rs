use std::ops::Range;

use anyhow::{Context, Result};
use ast::{DatastarSourceNodes, Document, ScriptSourceNodes};
use crop::Rope;
use syn::{
    parse::{Parse, ParseStream, Parser},
    spanned::Spanned,
};

use crate::{
    collect::MaudMacro,
    print::{print, print_datastar_source, print_js_script},
};

pub struct FormatOptions {
    pub line_length: usize,
    pub macro_names: Vec<String>,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            line_length: 100,
            macro_names: vec![
                String::from("html"),
                String::from("svg"),
                String::from("datastar_source"),
                String::from("js_script"),
            ],
        }
    }
}

#[derive(Debug)]
struct TextEdit {
    range: Range<usize>,
    new_text: String,
}

pub fn format_source(
    source: &mut Rope,
    macros: Vec<MaudMacro<'_>>,
    options: &FormatOptions,
) -> String {
    let mut edits = Vec::new();

    for maud_mac in macros {
        let mac = maud_mac.macro_;
        let start = mac.path.span().start();
        let end = mac.delimiter.span().close().end();
        let start_byte = line_column_to_byte(source, start);
        let end_byte = line_column_to_byte(source, end);

        match format_macro(&maud_mac, source, options) {
            Ok(new_text) => edits.push(TextEdit {
                range: start_byte..end_byte,
                new_text,
            }),
            Err(e) => eprintln!("{e}"),
        }
    }

    let mut last_offset: isize = 0;
    for edit in edits {
        let start = edit.range.start;
        let end = edit.range.end;
        let new_text = edit.new_text;

        source.replace(
            (start as isize + last_offset) as usize..(end as isize + last_offset) as usize,
            &new_text,
        );
        last_offset += new_text.len() as isize - (end as isize - start as isize);
    }

    source.to_string()
}

fn format_macro(mac: &MaudMacro, source: &Rope, options: &FormatOptions) -> Result<String> {
    if mac.macro_name == "datastar_source" {
        let document: DatastarSourceNodes = Parser::parse2(
            |input: ParseStream| DatastarSourceNodes::parse(input),
            mac.macro_.tokens.clone(),
        )
        .context("Failed to parse datastar_source macro")?;

        return Ok(print_datastar_source(document, mac, source, options));
    }

    if mac.macro_name == "js_script" {
        let document: ScriptSourceNodes = Parser::parse2(
            |input: ParseStream| ScriptSourceNodes::parse(input),
            mac.macro_.tokens.clone(),
        )
        .context("Failed to parse js_script macro")?;

        return Ok(print_js_script(document, mac, source, options));
    }

    let document: Document = Parser::parse2(
        |input: ParseStream| Document::parse(input),
        mac.macro_.tokens.clone(),
    )
    .context("Failed to parse maud macro")?;

    Ok(print(document, mac, source, options))
}

pub fn line_column_to_byte(source: &Rope, point: proc_macro2::LineColumn) -> usize {
    let line_byte = source.byte_of_line(point.line - 1);
    let line = source.line(point.line - 1);
    let char_byte: usize = line.chars().take(point.column).map(|c| c.len_utf8()).sum();
    line_byte + char_byte
}
