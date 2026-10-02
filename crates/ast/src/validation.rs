//! The element and attribute names that the Rust items in `cheers::validation` accept.
//!
//! A name found here passes the compile-time check `html!` would emit for it, so the check can be
//! skipped. Names missing here keep their check, which is the only place a typo, a custom element
//! or a `define_events!` event is resolved.

#[cfg(test)]
mod generate_table;
#[rustfmt::skip]
mod table;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::{
    UnquotedName,
    generate::{
        AttributeNameCheck, AttributeNameCheckKind, ElementCheck, ElementKind, ValidationModule,
        all_checks,
    },
};

pub struct Element {
    name: &'static str,
    kind: ElementKind,
    globals: &'static Globals,
    attributes: &'static [&'static str],
    namespaces: &'static [&'static str],
}

pub struct Globals {
    attributes: &'static [&'static str],
    namespaces: &'static [&'static str],
}

/// A module of `cheers::validation`, by its path relative to it.
pub struct Module {
    path: &'static str,
    namespace: bool,
    attributes: &'static [&'static str],
    modifiers: &'static [&'static str],
}

pub fn element(module: ValidationModule, name: &str) -> Option<&'static Element> {
    let elements = match module {
        ValidationModule::Html => table::HTML_ELEMENTS,
        ValidationModule::Svg => table::SVG_ELEMENTS,
        ValidationModule::MathMl => return None,
    };

    elements
        .binary_search_by(|element| element.name.cmp(name))
        .ok()
        .map(|index| &elements[index])
}

pub fn module(path: &str) -> Option<&'static Module> {
    table::MODULES
        .binary_search_by(|module| module.path.cmp(path))
        .ok()
        .map(|index| &table::MODULES[index])
}

fn contains(names: &[&str], name: &str) -> bool {
    names.binary_search(&name).is_ok()
}

impl Element {
    pub fn kind(&self) -> ElementKind {
        self.kind
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        contains(self.attributes, name) || contains(self.globals.attributes, name)
    }

    pub fn has_namespace(&self, name: &str) -> bool {
        // An inherent `<element>::name` attribute shadows a namespace of the same name.
        !contains(self.attributes, name)
            && (contains(self.namespaces, name) || contains(self.globals.namespaces, name))
    }
}

impl Module {
    pub fn has_namespace(&self) -> bool {
        self.namespace
    }

    pub fn has_attribute(&self, name: &str) -> bool {
        contains(self.attributes, name)
    }

    pub fn has_modifier(&self, name: &str) -> bool {
        contains(self.modifiers, name)
    }
}

/// Emits the check `html!` would for every name in the table, so compiling the output proves
/// that rustc accepts all of them.
pub fn table_checks() -> TokenStream {
    let name = |name: &str| UnquotedName(Ident::new(name, Span::call_site()));
    let modifiers = |plugin: &str, check: &mut AttributeNameCheck| {
        for modifier in
            module(&format!("data::modifiers::{plugin}")).map_or(&[][..], |m| m.modifiers)
        {
            check.push_data_modifier(name(modifier));
        }
    };
    let mut checks = Vec::new();

    for (validation_module, elements) in [
        (ValidationModule::Html, table::HTML_ELEMENTS),
        (ValidationModule::Svg, table::SVG_ELEMENTS),
    ] {
        for element in elements {
            let mut check = ElementCheck::new(&name(element.name), element.kind, validation_module);
            for attribute in element.attributes.iter().chain(element.globals.attributes) {
                check.push_attribute(AttributeNameCheck::new(
                    AttributeNameCheckKind::Normal,
                    name(attribute),
                    false,
                ));
            }
            let namespaces = element.namespaces.iter().chain(element.globals.namespaces);
            for namespace in namespaces.filter(|namespace| element.has_namespace(namespace)) {
                let attributes = module(namespace).map_or(&[][..], |m| m.attributes);
                for attribute in attributes {
                    check.push_attribute(AttributeNameCheck::new(
                        AttributeNameCheckKind::Namespace(name(namespace)),
                        name(attribute),
                        false,
                    ));
                }
            }
            checks.push(check);
        }
    }

    let mut data = ElementCheck::new(&name("div"), ElementKind::Normal, ValidationModule::Html);
    for module in table::MODULES {
        if module.path == "data" {
            for attribute in module.attributes {
                let mut check =
                    AttributeNameCheck::new(AttributeNameCheckKind::Normal, name(attribute), true);
                modifiers(attribute, &mut check);
                data.push_attribute(check);
            }
        } else if let Some(namespace) = module.path.strip_prefix("data::")
            && module.namespace
        {
            for attribute in module.attributes {
                let mut check = AttributeNameCheck::new(
                    AttributeNameCheckKind::Namespace(name(namespace)),
                    name(attribute),
                    true,
                );
                modifiers(namespace, &mut check);
                data.push_attribute(check);
            }
        }
    }
    checks.push(data);

    let checks = all_checks(&checks);
    quote! {
        const _: () = {
            use ::cheers::validation::attributes::*;
            #checks
        };
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{element, module, table};
    use crate::{
        Document,
        generate::{ElementKind, ValidationModule, lazy},
    };

    fn expand(tokens: proc_macro2::TokenStream) -> String {
        lazy::<Document>(tokens)
            .expect("document should generate")
            .to_string()
    }

    #[test]
    fn known_names_are_only_referenced_for_rust_analyzer() {
        let expanded = expand(quote! {
            div id="a" aria:label="b" !on:click[prevent]("c") { a href="/" {} input; }
        });
        let (_, references) = expanded
            .split_once("# [cfg (rust_analyzer)]")
            .expect("known names should be referenced for rust-analyzer");

        assert!(!expanded.contains("check_element"), "{expanded}");
        assert!(references.contains("< r#div > :: r#id ;"), "{references}");
        assert!(
            references.contains("data :: r#on :: r#click ;")
                && references.contains("modifiers :: r#on :: r#prevent ;"),
            "{references}"
        );
    }

    #[test]
    fn unknown_names_keep_their_checks() {
        let expanded = expand(quote! {
            div id="a" hx_get="b" !on:my_event("c") { img {} }
        });
        let (needed, references) = expanded
            .split_once("# [cfg (rust_analyzer)]")
            .expect("known names should be referenced for rust-analyzer");

        assert!(
            needed.contains("r#hx_get") && needed.contains("r#my_event"),
            "{needed}"
        );
        assert!(needed.contains("(r#img)"), "{needed}");
        assert!(
            !needed.contains("r#id") && !needed.contains("(r#div)"),
            "{needed}"
        );
        assert!(
            references.contains("r#div ;") && references.contains("r#id ;"),
            "{references}"
        );
    }

    #[test]
    fn unknown_elements_are_checked_without_cfg() {
        let expanded = expand(quote! { my_widget size="1" {} });

        assert!(!expanded.contains("rust_analyzer"), "{expanded}");
        assert!(
            expanded.contains("(r#my_widget)") && expanded.contains("r#size"),
            "{expanded}"
        );
    }

    #[test]
    fn adjacent_static_parts_are_pushed_as_one_literal() {
        let expanded = expand(quote! { div class="a" { "b" } });

        assert!(
            expanded.contains(r#"push_str ("<div class=\"a\">b</div>")"#),
            "{expanded}"
        );
    }

    #[test]
    fn tables_are_sorted() {
        let is_sorted = |names: &[&str]| names.windows(2).all(|pair| pair[0] < pair[1]);
        for elements in [table::HTML_ELEMENTS, table::SVG_ELEMENTS] {
            assert!(elements.windows(2).all(|pair| pair[0].name < pair[1].name));
            for element in elements {
                assert!(is_sorted(element.attributes) && is_sorted(element.namespaces));
                assert!(
                    is_sorted(element.globals.attributes) && is_sorted(element.globals.namespaces)
                );
            }
        }
        assert!(
            table::MODULES
                .windows(2)
                .all(|pair| pair[0].path < pair[1].path)
        );
        for module in table::MODULES {
            assert!(is_sorted(module.attributes) && is_sorted(module.modifiers));
        }
    }

    #[test]
    fn looks_up_names() {
        let div = element(ValidationModule::Html, "div").expect("`div` is an HTML element");
        assert_eq!(div.kind(), ElementKind::Normal);
        assert!(div.has_attribute("id") && div.has_attribute("role"));
        assert!(!div.has_attribute("href") && !div.has_attribute("aria"));
        assert!(div.has_namespace("aria") && !div.has_namespace("on"));

        let object = element(ValidationModule::Html, "object").expect("`object` is an element");
        assert!(object.has_attribute("data") && !object.has_namespace("data"));

        let meta = element(ValidationModule::Html, "meta").expect("`meta` is an HTML element");
        assert_eq!(meta.kind(), ElementKind::Void);
        assert!(meta.has_attribute("property"));

        assert!(element(ValidationModule::Html, "dvi").is_none());
        assert!(element(ValidationModule::Svg, "div").is_none());
        assert!(element(ValidationModule::MathMl, "math").is_none());
        assert!(element(ValidationModule::Html, "math").is_none());

        let on = module("data::on").expect("`data::on` is a module");
        assert!(on.has_namespace() && on.has_attribute("click"));
        assert!(module("data::modifiers::ignore").is_some_and(|m| m.has_modifier("self_")));
    }
}
