use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone)]
pub struct VirtualModule {
    pub specifier: String,
    pub content: String,
}

impl VirtualModule {
    pub fn new(specifier: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            specifier: specifier.into(),
            content: content.into(),
        }
    }
}

struct PseudoResolver {
    virtual_modules: HashSet<String>,
}

fn resolve_datastar_alias(module_specifier: &str) -> Option<swc_ecma_loader::resolve::Resolution> {
    if module_specifier == "@engine" {
        return Some(swc_ecma_loader::resolve::Resolution {
            filename: FileName::Real("engine/engine".into()),
            slug: None,
        });
    }

    let resolved = if let Some(rest) = module_specifier.strip_prefix("@engine/") {
        Some(format!("engine/{rest}"))
    } else if let Some(rest) = module_specifier.strip_prefix("@utils/") {
        Some(format!("utils/{rest}"))
    } else {
        module_specifier
            .strip_prefix("@plugins/")
            .map(|rest| format!("plugins/{rest}"))
    };

    resolved.map(|filename| swc_ecma_loader::resolve::Resolution {
        filename: FileName::Real(filename.into()),
        slug: None,
    })
}

fn resolve_relative_specifier(base: &str, module_specifier: &str) -> Result<String, Error> {
    let mut parts = base
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let Some(_) = parts.pop() else {
        bail!(
            "resolve relative module specifier `{module_specifier}` from `{base}`: missing base module name"
        );
    };

    for segment in module_specifier.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                let Some(_) = parts.pop() else {
                    bail!(
                        "resolve relative module specifier `{module_specifier}` from `{base}`: walked past the virtual root"
                    );
                };
            }
            _ => parts.push(segment),
        }
    }

    if parts.is_empty() {
        bail!(
            "resolve relative module specifier `{module_specifier}` from `{base}`: resolved to an empty module name"
        );
    }

    Ok(parts.join("/"))
}

fn is_relative_specifier(module_specifier: &str) -> bool {
    module_specifier == "."
        || module_specifier == ".."
        || module_specifier.starts_with("./")
        || module_specifier.starts_with("../")
}

fn normalize_virtual_specifier(
    virtual_modules: &HashSet<String>,
    module_specifier: &str,
) -> Option<String> {
    if virtual_modules.contains(module_specifier) {
        return Some(module_specifier.to_owned());
    }

    if let Some(module_specifier) = module_specifier.strip_suffix(".ts")
        && virtual_modules.contains(module_specifier)
    {
        return Some(module_specifier.to_owned());
    }

    let module_specifier_with_ts = format!("{module_specifier}.ts");
    if virtual_modules.contains(&module_specifier_with_ts) {
        return Some(module_specifier_with_ts);
    }

    None
}

impl Resolve for PseudoResolver {
    fn resolve(
        &self,
        base: &FileName,
        module_specifier: &str,
    ) -> Result<swc_ecma_loader::resolve::Resolution, Error> {
        if let Some(resolution) = resolve_datastar_alias(module_specifier) {
            return Ok(resolution);
        }

        if is_relative_specifier(module_specifier) {
            let base = match base {
                FileName::Custom(name) => name.clone(),
                FileName::Real(path) => path.to_string_lossy().into_owned(),
                _ => bail!("unsupported base filename type: {base}"),
            };

            let resolved = resolve_relative_specifier(&base, module_specifier)?;
            if let Some(virtual_module) =
                normalize_virtual_specifier(&self.virtual_modules, &resolved)
            {
                return Ok(swc_ecma_loader::resolve::Resolution {
                    filename: FileName::Custom(virtual_module),
                    slug: None,
                });
            }

            return Ok(swc_ecma_loader::resolve::Resolution {
                filename: FileName::Real(resolved.into()),
                slug: None,
            });
        }

        if let Some(virtual_module) =
            normalize_virtual_specifier(&self.virtual_modules, module_specifier)
        {
            return Ok(swc_ecma_loader::resolve::Resolution {
                filename: FileName::Custom(virtual_module),
                slug: None,
            });
        }

        if let FileName::Custom(_) = base {
            return Ok(swc_ecma_loader::resolve::Resolution {
                filename: FileName::Real(module_specifier.into()),
                slug: None,
            });
        }

        bail!(
            "unsupported module specifier `{}` for base filename `{}`",
            module_specifier,
            base
        );
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
    virtual_modules: HashMap<String, String>,
}

include!(concat!(env!("OUT_DIR"), "/datastar_loader.rs"));

impl Load for Loader {
    fn load(&self, filename: &FileName) -> Result<ModuleData, Error> {
        let source_file = match filename {
            FileName::Real(path) => self.load_datastar_file(path),
            FileName::Custom(name) => {
                let content = self
                    .virtual_modules
                    .get(name)
                    .with_context(|| format!("unknown virtual module: {name}"))?;
                self.cm
                    .new_source_file(filename.clone().into(), content.clone())
            }
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

pub fn bundle_and_minify(
    entry_specifier: &str,
    virtual_modules: impl IntoIterator<Item = VirtualModule>,
) -> Result<String, Error> {
    let mut modules_by_specifier = HashMap::new();
    for module in virtual_modules {
        if modules_by_specifier
            .insert(module.specifier.clone(), module.content)
            .is_some()
        {
            bail!("duplicate virtual module specifier `{}`", module.specifier);
        }
    }

    if !modules_by_specifier.contains_key(entry_specifier) {
        bail!("entry virtual module `{entry_specifier}` not found");
    }

    let cm = Lrc::new(SourceMap::new(swc_common::FilePathMapping::empty()));

    let resolver = PseudoResolver {
        virtual_modules: modules_by_specifier.keys().cloned().collect(),
    };
    let loader = Loader {
        cm: cm.clone(),
        virtual_modules: modules_by_specifier,
    };

    let globals = Globals::new();

    let mut bundler = Bundler::new(
        &globals,
        cm.clone(),
        &loader,
        CachingResolver::new(4096, resolver),
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
        FileName::Custom(entry_specifier.to_owned()),
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

#[cfg(test)]
mod tests {
    use super::{VirtualModule, bundle_and_minify};

    #[test]
    fn bundles_virtual_modules_with_relative_imports() {
        let bundle = bundle_and_minify(
            "cheers/entry",
            [
                VirtualModule::new("cheers/entry", "import './dep'"),
                VirtualModule::new("cheers/dep", "globalThis.__cheers_bundle_test = 1"),
            ],
        )
        .expect("bundle should succeed");

        assert!(bundle.contains("__cheers_bundle_test"), "{bundle}");
    }

    #[test]
    fn resolves_virtual_modules_with_ts_extensions() {
        let bundle = bundle_and_minify(
            "cheers/entry",
            [
                VirtualModule::new("cheers/entry", "import './dep.ts'"),
                VirtualModule::new("cheers/dep", "globalThis.__cheers_bundle_test_ts = 1"),
            ],
        )
        .expect("bundle should succeed");

        assert!(bundle.contains("__cheers_bundle_test_ts"), "{bundle}");
    }

    #[test]
    fn preserves_http_and_json_property_names() {
        let bundle = bundle_and_minify(
            "cheers/entry",
            [VirtualModule::new(
                "cheers/entry",
                r#"
                const body = JSON.stringify({
                    sent_at_ms: 1,
                    items: [{ timestamp_ms: 2, props: { button_id: "main" } }],
                });

                fetch("/_track", {
                    method: "POST",
                    headers: {
                        "content-type": "application/json",
                    },
                    body,
                });
                "#,
            )],
        )
        .expect("bundle should succeed");

        assert!(bundle.contains("content-type"), "{bundle}");
        assert!(bundle.contains("sent_at_ms"), "{bundle}");
        assert!(bundle.contains("timestamp_ms"), "{bundle}");
        assert!(bundle.contains("props"), "{bundle}");
    }
}
