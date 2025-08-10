use std::sync::{LazyLock, Mutex};

use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{ParserFlags, ParserOptions, StyleSheet},
};

use crate::router::Error;

pub struct CssBundler(Mutex<Vec<String>>);

fn printer_options<'a>() -> PrinterOptions<'a> {
    if cfg!(debug_assertions) {
        PrinterOptions::default()
    } else {
        PrinterOptions {
            minify: true,
            ..Default::default()
        }
    }
}

impl CssBundler {
    pub fn add(&self, s: &str) {
        let mut this = self.0.lock().expect("unlock css bundler");
        this.push(s.to_owned());
    }

    pub(crate) fn bundle(&self) -> Result<String, Error> {
        let this = self.0.lock().expect("unlock css bundler");

        let deps = this
            .iter()
            .map(|d| {
                let s = StyleSheet::parse(
                    d,
                    ParserOptions {
                        flags: ParserFlags::NESTING,
                        ..Default::default()
                    },
                )
                .map_err(|e| Error::Bundling(e.to_string()));
                let s = s.and_then(|s| {
                    s.to_css(printer_options())
                        .map_err(|e| Error::Bundling(e.to_string()))
                });

                s.map(|s| s.code)
            })
            .collect::<Result<Vec<String>, Error>>()?;

        let stylesheet = deps.join("\n");

        Ok(stylesheet)
    }
}

pub static BUNDLER: LazyLock<CssBundler> = LazyLock::new(|| CssBundler(Mutex::new(Vec::new())));
