use std::collections::HashMap;

use anyhow::{Context, Error, anyhow, bail};
use swc_bundler::{Bundler, Hook, Load, ModuleData, ModuleRecord};
use swc_common::{FileName, GLOBALS, Globals, Mark, SourceMap, Span, sync::Lrc};
use swc_ecma_ast::*;
use swc_ecma_codegen::{
    Emitter,
    text_writer::{JsWriter, WriteJs, omit_trailing_semi},
};
use swc_ecma_loader::{resolve::Resolve, resolvers::lru::CachingResolver};
use swc_ecma_minifier::option::{CompressOptions, ExtraOptions, MangleOptions, MinifyOptions};
use swc_ecma_parser::{Syntax, parse_file_as_module};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_typescript::typescript;
use swc_ecma_visit::VisitMutWith;

struct PseudoResolver {}

impl Resolve for PseudoResolver {
    fn resolve(
        &self,
        base: &FileName,
        module_specifier: &str,
    ) -> Result<swc_ecma_loader::resolve::Resolution, Error> {
        if let FileName::Custom(s) = base
            && s == "datastar-entry"
        {
            if let Some(rest) = module_specifier.strip_prefix("@plugins/") {
                return Ok(swc_ecma_loader::resolve::Resolution {
                    filename: FileName::Real(format!("plugins/{}", rest).into()),
                    slug: None,
                });
            }

            return Ok(swc_ecma_loader::resolve::Resolution {
                filename: FileName::Real(module_specifier.into()),
                slug: None,
            });
        }

        if let FileName::Real(_) = base {
            if module_specifier == "@engine" {
                return Ok(swc_ecma_loader::resolve::Resolution {
                    filename: FileName::Real("engine/engine".into()),
                    slug: None,
                });
            }

            if let Some(rest) = module_specifier.strip_prefix("@engine/") {
                return Ok(swc_ecma_loader::resolve::Resolution {
                    filename: FileName::Real(format!("engine/{}", rest).into()),
                    slug: None,
                });
            } else if let Some(rest) = module_specifier.strip_prefix("@utils/") {
                return Ok(swc_ecma_loader::resolve::Resolution {
                    filename: FileName::Real(format!("utils/{}", rest).into()),
                    slug: None,
                });
            } else if let Some(rest) = module_specifier.strip_prefix("@plugins/") {
                return Ok(swc_ecma_loader::resolve::Resolution {
                    filename: FileName::Real(format!("plugins/{}", rest).into()),
                    slug: None,
                });
            };

            bail!(
                "unsupported module specifier `{}` for base filename `{}`",
                module_specifier,
                base
            );
        }

        bail!("unsupported base filename type: {base}");
    }
}

struct NoopHook;

impl Hook for NoopHook {
    fn get_import_meta_props(&self, _: Span, _: &ModuleRecord) -> Result<Vec<KeyValueProp>, Error> {
        Ok(Vec::new())
    }
}

struct Loader {
    cm: Lrc<SourceMap>,
    entry_content: String,
}

include!(concat!(env!("OUT_DIR"), "/datastar_loader.rs"));

impl Load for Loader {
    fn load(&self, filename: &FileName) -> Result<ModuleData, Error> {
        let source_file = match filename {
            FileName::Real(path) => self.load_datastar_file(path),
            FileName::Custom(name) if name == "datastar-entry" => self
                .cm
                .new_source_file(filename.clone().into(), self.entry_content.clone()),
            _ => return Err(anyhow!("unexpected filename: {filename}")),
        };

        let module = parse_file_as_module(
            &source_file,
            Syntax::Typescript(Default::default()),
            EsVersion::Es2020,
            None,
            &mut Vec::new(),
        )
        .map_err(|e| anyhow!("parse: {:?}", e))?;

        let mut program = Program::Module(module);
        let mut ts_pass = typescript(Default::default(), Mark::new(), Mark::new());
        ts_pass.process(&mut program);

        let module = match program {
            Program::Module(m) => m,
            _ => unreachable!(),
        };

        Ok(ModuleData {
            fm: source_file,
            module,
            helpers: Default::default(),
        })
    }
}

pub fn bundle_and_minify(entry_content: String) -> Result<String, Error> {
    let cm = Lrc::new(SourceMap::new(swc_common::FilePathMapping::empty()));

    let loader = Loader {
        cm: cm.clone(),
        entry_content,
    };

    let globals = Globals::new();

    let mut bundler = Bundler::new(
        &globals,
        cm.clone(),
        &loader,
        CachingResolver::new(4096, PseudoResolver {}),
        swc_bundler::Config {
            require: false,
            disable_inliner: false,
            external_modules: Default::default(),
            disable_fixer: true,
            disable_hygiene: true,
            disable_dce: false,
            module: Default::default(),
        },
        Box::new(NoopHook),
    );

    let entries = HashMap::from([(
        "datastar".to_owned(),
        FileName::Custom("datastar-entry".to_owned()),
    )]);

    let mut bundles = bundler.bundle(entries)?.into_iter();
    let mut bundle = bundles.next().with_context(|| "create a single bundle")?;
    if bundles.next().is_some() {
        bail!("expected one bundle but got more");
    }

    bundle = GLOBALS.set(&globals, || {
        bundle.module = swc_ecma_minifier::optimize(
            bundle.module.into(),
            cm.clone(),
            None,
            None,
            &MinifyOptions {
                compress: Some(CompressOptions::default()),
                mangle: Some(MangleOptions {
                    props: Some(Default::default()),
                    top_level: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &ExtraOptions {
                unresolved_mark: Mark::new(),
                top_level_mark: Mark::new(),
                mangle_name_cache: None,
            },
        )
        .module()
        .expect("expected a module to come out of optimizing module");

        bundle.module.visit_mut_with(&mut fixer(None));
        bundle
    });

    let mut buf = Vec::new();
    {
        let wr = JsWriter::new(cm.clone(), "\n", &mut buf, None);
        let mut emitter = Emitter {
            cfg: swc_ecma_codegen::Config::default().with_minify(true),
            cm: cm.clone(),
            comments: None,
            wr: Box::new(omit_trailing_semi(wr)) as Box<dyn WriteJs>,
        };
        emitter.emit_module(&bundle.module)?;
    }

    Ok(String::from_utf8_lossy(&buf).to_string())
}
