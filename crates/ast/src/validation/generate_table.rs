//! Derives `table.rs` from the Rust items in `crates/cheers/src/validation`.
//!
//! The derivation mirrors how rustc resolves the checks `html!` emits, and rejects any item it
//! does not understand, so a change to the validation items fails here instead of producing a
//! table that disagrees with them.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use syn::{
    Attribute, File, GenericParam, Ident, ImplItem, Item, Path as SynPath, TraitItem, Type,
    TypeParamBound, UseTree, Visibility, braced,
    ext::IdentExt,
    parse::{ParseStream, Parser},
    token::Brace,
};

use crate::generate::ValidationModule;

const UPDATE_ENV: &str = "UPDATE_VALIDATION_TABLE";

#[derive(Clone, PartialEq, Eq)]
enum ConstType {
    Attribute,
    Modifier,
    Namespace(String),
}

#[derive(Default)]
struct ModuleInfo {
    namespace: bool,
    attributes: BTreeSet<String>,
    modifiers: BTreeSet<String>,
}

struct ElementInfo {
    kind: &'static str,
    globals: String,
    attributes: BTreeSet<String>,
}

#[derive(Default)]
struct Source {
    traits: BTreeMap<String, Vec<(String, ConstType)>>,
    /// `(bound, implemented)` for `impl<T: bound> implemented for T {}`.
    blanket_impls: Vec<(String, String)>,
    /// `(implemented, module, element)` for `impl implemented for element {}`.
    element_impls: Vec<(String, ValidationModule, String)>,
    modules: BTreeMap<String, ModuleInfo>,
    elements: BTreeMap<ValidationModule, BTreeMap<String, ElementInfo>>,
}

fn name(ident: &Ident) -> String {
    ident.unraw().to_string()
}

/// Whether an item with `attrs` is behind `#[cfg]`. Attributes that could change what a name
/// means, like `deprecated` or `cfg_attr`, are rejected.
fn cfg_gated(attrs: &[Attribute]) -> bool {
    let mut gated = false;
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("cfg") {
            gated = true;
        } else if !["doc", "allow", "expect", "derive", "non_exhaustive"]
            .iter()
            .any(|name| path.is_ident(name))
        {
            panic!(
                "unsupported attribute `#[{}]` on a validation item",
                quote::quote!(#path)
            );
        }
    }
    gated
}

/// Items that add names must always be compiled: under another feature set, a gated one could
/// make a name the table lists ambiguous.
fn assert_ungated(attrs: &[Attribute]) {
    assert!(
        !cfg_gated(attrs),
        "`#[cfg]` is only supported on element lists and re-exports"
    );
}

fn use_names(tree: &UseTree, names: &mut Vec<String>) {
    match tree {
        UseTree::Path(tree) => use_names(&tree.tree, names),
        UseTree::Name(tree) => names.push(name(&tree.ident)),
        UseTree::Rename(tree) => names.push(name(&tree.rename)),
        UseTree::Glob(_) => {}
        UseTree::Group(group) => {
            for tree in &group.items {
                use_names(tree, names);
            }
        }
    }
}

fn last_segment(path: &SynPath) -> String {
    name(&path.segments.last().expect("paths are never empty").ident)
}

fn const_type(ty: &Type) -> ConstType {
    let Type::Path(ty) = ty else {
        panic!("unsupported validation const type");
    };
    let segments: Vec<_> = ty.path.segments.iter().map(|s| name(&s.ident)).collect();
    match segments.as_slice() {
        [.., last] if last == "Attribute" => ConstType::Attribute,
        [.., last] if last == "Modifier" => ConstType::Modifier,
        [namespace, last] if last == "Namespace" => ConstType::Namespace(namespace.clone()),
        _ => panic!(
            "unsupported validation const type `{}`",
            segments.join("::")
        ),
    }
}

fn element_path(path: &SynPath) -> (ValidationModule, String) {
    let segments: Vec<_> = path.segments.iter().map(|s| name(&s.ident)).collect();
    match segments.as_slice() {
        [krate, validation, elements, element]
            if krate == "crate" && validation == "validation" && elements == "elements" =>
        {
            (ValidationModule::Html, element.clone())
        }
        [krate, validation, svg, elements, element]
            if krate == "crate"
                && validation == "validation"
                && svg == "svg"
                && elements == "elements" =>
        {
            (ValidationModule::Svg, element.clone())
        }
        _ => panic!("unsupported element path `{}`", segments.join("::")),
    }
}

impl Source {
    fn read(dir: &Path) -> Self {
        let mut source = Self::default();
        source.attribute_items(&parse(&dir.join("attributes.rs")).items, "");
        source.element_items(
            &parse(&dir.join("elements.rs")).items,
            ValidationModule::Html,
        );

        let svg = parse(&dir.join("svg.rs"));
        let [Item::Mod(elements)] = svg.items.as_slice() else {
            panic!("`svg.rs` should only contain `mod elements`");
        };
        let (_, items) = elements
            .content
            .as_ref()
            .expect("`svg::elements` is inline");
        source.element_items(items, ValidationModule::Svg);
        source.check_root_items(&parse(&dir.join("mod.rs")).items, true);

        source
    }

    /// Rejects items in `validation/mod.rs` that would change what the checks resolve to without
    /// this table knowing: validation trait impls, element lists, and items that shadow a module
    /// the `pub use attributes::*` glob provides.
    fn check_root_items(&self, items: &[Item], top_level: bool) {
        for item in items {
            match item {
                Item::Impl(item) => {
                    if let Some((_, implemented, _)) = &item.trait_ {
                        assert!(
                            !self.traits.contains_key(&last_segment(implemented)),
                            "validation trait impls belong in `attributes.rs`"
                        );
                    }
                }
                Item::Macro(item) => assert!(
                    !item.mac.path.is_ident("define_validation_elements"),
                    "element lists belong in `elements.rs` or `svg.rs`"
                ),
                Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        self.check_root_items(items, false);
                    }
                }
                _ => {}
            }

            if !top_level {
                continue;
            }
            let mut names = Vec::new();
            match item {
                Item::Use(item) => use_names(&item.tree, &mut names),
                Item::Mod(item) => names.push(name(&item.ident)),
                Item::Struct(item) => names.push(name(&item.ident)),
                Item::Enum(item) => names.push(name(&item.ident)),
                Item::Const(item) => names.push(name(&item.ident)),
                Item::Static(item) => names.push(name(&item.ident)),
                Item::Fn(item) => names.push(name(&item.sig.ident)),
                Item::Trait(item) => names.push(name(&item.ident)),
                Item::Type(item) => names.push(name(&item.ident)),
                _ => {}
            }
            for name in names {
                assert!(
                    !self.modules.contains_key(&name),
                    "`validation::{name}` shadows the attribute module of the same name"
                );
            }
        }
    }

    fn attribute_items(&mut self, items: &[Item], module: &str) {
        self.modules.entry(module.to_owned()).or_default();

        for item in items {
            match item {
                Item::Use(item) => {
                    assert!(
                        cfg_gated(&item.attrs) || matches!(item.vis, Visibility::Inherited),
                        "re-exports from validation attributes are not supported"
                    );
                }
                Item::Trait(item) => {
                    assert_ungated(&item.attrs);
                    assert!(
                        item.generics.params.is_empty() && item.generics.where_clause.is_none(),
                        "generic validation traits are not supported"
                    );
                    let consts = item
                        .items
                        .iter()
                        .map(|item| match item {
                            TraitItem::Const(item) => {
                                assert_ungated(&item.attrs);
                                (name(&item.ident), const_type(&item.ty))
                            }
                            _ => panic!("validation traits should only contain consts"),
                        })
                        .collect();
                    self.traits.insert(name(&item.ident), consts);
                }
                Item::Impl(item) => {
                    assert_ungated(&item.attrs);
                    self.attribute_impl(item);
                }
                Item::Mod(item) => {
                    assert_ungated(&item.attrs);
                    assert!(
                        matches!(item.vis, Visibility::Public(_)),
                        "validation modules should be public"
                    );
                    let path = if module.is_empty() {
                        name(&item.ident)
                    } else {
                        format!("{module}::{}", name(&item.ident))
                    };
                    let (_, items) = item.content.as_ref().expect("modules are inline");
                    self.attribute_items(items, &path);
                }
                Item::Struct(item) => {
                    assert_ungated(&item.attrs);
                    if item.ident == "Namespace" {
                        self.modules.entry(module.to_owned()).or_default().namespace = true;
                    }
                }
                Item::Const(item) => {
                    assert_ungated(&item.attrs);
                    assert!(
                        matches!(item.vis, Visibility::Public(_)),
                        "validation consts should be public"
                    );
                    let info = self.modules.entry(module.to_owned()).or_default();
                    let set = match const_type(&item.ty) {
                        ConstType::Attribute => &mut info.attributes,
                        ConstType::Modifier => &mut info.modifiers,
                        ConstType::Namespace(_) => panic!("namespace consts belong in traits"),
                    };
                    set.insert(name(&item.ident));
                }
                _ => panic!("unsupported item in validation attributes"),
            }
        }
    }

    fn attribute_impl(&mut self, item: &syn::ItemImpl) {
        assert!(
            item.items
                .iter()
                .all(|item| !matches!(item, ImplItem::Const(_))),
            "validation impls should not override consts"
        );
        let (_, implemented, _) = item
            .trait_
            .as_ref()
            .expect("only trait impls are supported");
        let implemented = last_segment(implemented);

        match item.generics.params.iter().collect::<Vec<_>>().as_slice() {
            [] => {
                let Type::Path(self_ty) = &*item.self_ty else {
                    panic!("unsupported impl target");
                };
                let (module, element) = element_path(&self_ty.path);
                self.element_impls.push((implemented, module, element));
            }
            [GenericParam::Type(param)] => {
                let [TypeParamBound::Trait(bound)] =
                    param.bounds.iter().collect::<Vec<_>>().as_slice()
                else {
                    panic!("blanket impls should have one trait bound");
                };
                assert!(
                    matches!(&*item.self_ty, Type::Path(ty) if ty.path.is_ident(&param.ident)),
                    "only blanket impls for the type parameter are supported"
                );
                self.blanket_impls
                    .push((last_segment(&bound.path), implemented));
            }
            _ => panic!("unsupported impl generics"),
        }
    }

    fn element_items(&mut self, items: &[Item], module: ValidationModule) {
        for item in items {
            match item {
                Item::Use(_) => {}
                Item::Macro(item) if cfg_gated(&item.attrs) => {}
                Item::Macro(item) if item.mac.path.is_ident("define_validation_elements") => {
                    let elements = self.elements.entry(module).or_default();
                    (|input: ParseStream<'_>| parse_elements(input, elements))
                        .parse2(item.mac.tokens.clone())
                        .expect("`define_validation_elements!` invocations should parse");
                }
                _ => panic!("unsupported item in validation elements"),
            }
        }
    }

    /// The traits a type implementing all of `traits` implements through blanket impls.
    fn implied_traits(&self, traits: impl IntoIterator<Item = String>) -> BTreeSet<String> {
        let mut implied: BTreeSet<String> = traits.into_iter().collect();
        loop {
            let before = implied.len();
            for (bound, implemented) in &self.blanket_impls {
                if implied.contains(bound) {
                    implied.insert(implemented.clone());
                }
            }
            if implied.len() == before {
                return implied;
            }
        }
    }

    fn trait_consts<'a>(
        &'a self,
        traits: &'a BTreeSet<String>,
    ) -> impl Iterator<Item = &'a (String, ConstType)> {
        traits.iter().flat_map(|implemented| {
            self.traits
                .get(implemented)
                .unwrap_or_else(|| panic!("unknown validation trait `{implemented}`"))
        })
    }

    /// The attribute and namespace names `<element>::name` resolves to through `traits`, leaving
    /// out names more than one of them declares, which rustc rejects as ambiguous.
    fn trait_names(&self, traits: &BTreeSet<String>) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut declared = BTreeMap::<&str, usize>::new();
        for (name, _) in self.trait_consts(traits) {
            *declared.entry(name).or_default() += 1;
        }

        let mut attributes = BTreeSet::new();
        let mut namespaces = BTreeSet::new();
        for (name, ty) in self.trait_consts(traits) {
            if declared[name.as_str()] > 1 {
                continue;
            }
            match ty {
                ConstType::Attribute => {
                    attributes.insert(name.clone());
                }
                ConstType::Namespace(namespace)
                    if namespace == name
                        && self.modules.get(namespace).is_some_and(|m| m.namespace) =>
                {
                    namespaces.insert(name.clone());
                }
                ConstType::Namespace(_) | ConstType::Modifier => {}
            }
        }
        (attributes, namespaces)
    }
}

fn parse(path: &Path) -> File {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read `{}`: {error}", path.display()));
    syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("failed to parse `{}`: {error}", path.display()))
}

fn parse_doc_attrs(input: ParseStream<'_>) -> syn::Result<()> {
    for attr in Attribute::parse_outer(input)? {
        if !attr.path().is_ident("doc") {
            return Err(syn::Error::new_spanned(
                attr,
                "only doc attributes are supported in validation tables",
            ));
        }
    }
    Ok(())
}

fn parse_elements(
    input: ParseStream<'_>,
    elements: &mut BTreeMap<String, ElementInfo>,
) -> syn::Result<()> {
    let path_arg = |key: &str| -> syn::Result<String> {
        let ident: Ident = input.parse()?;
        if ident != key {
            return Err(syn::Error::new(ident.span(), format!("expected `{key}`")));
        }
        input.parse::<syn::Token![=]>()?;
        let path: SynPath = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        Ok(last_segment(&path))
    };
    let kind_span = input.span();
    let kind = match path_arg("kind")?.as_str() {
        "Normal" => "Normal",
        "Void" => "Void",
        "Xml" => "Xml",
        kind => {
            return Err(syn::Error::new(
                kind_span,
                format!("unknown element kind `{kind}`"),
            ));
        }
    };
    let globals = path_arg("globals")?;

    let content;
    braced!(content in input);
    while !content.is_empty() {
        parse_doc_attrs(&content)?;
        let element = name(&content.call(Ident::parse_any)?);
        let mut attributes = BTreeSet::new();
        if content.peek(Brace) {
            let attrs;
            braced!(attrs in content);
            while !attrs.is_empty() {
                parse_doc_attrs(&attrs)?;
                attributes.insert(name(&attrs.call(Ident::parse_any)?));
            }
        }
        let info = ElementInfo {
            kind,
            globals: globals.clone(),
            attributes,
        };
        if elements.insert(element.clone(), info).is_some() {
            return Err(syn::Error::new(
                content.span(),
                format!("element `{element}` is defined twice"),
            ));
        }
    }
    Ok(())
}

fn list<'a>(names: impl IntoIterator<Item = &'a String>) -> String {
    let names: Vec<_> = names
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect();
    format!("&[{}]", names.join(", "))
}

fn globals_const(globals: &str) -> String {
    let mut out = String::new();
    for (index, c) in globals.chars().enumerate() {
        if c.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

fn render(source: &Source) -> String {
    let mut out = String::from(
        "// @generated from `crates/cheers/src/validation` by the `validation_table_is_up_to_date`\n\
         // test. Regenerate it with `UPDATE_VALIDATION_TABLE=1 cargo test -p cheers-ast`.\n\n\
         use super::{Element, Globals, Module};\n\
         use crate::generate::ElementKind::{Normal, Void, Xml};\n",
    );

    let globals: BTreeSet<&String> = source
        .elements
        .values()
        .flat_map(BTreeMap::values)
        .map(|element| &element.globals)
        .collect();
    for globals in globals {
        let (attributes, namespaces) =
            source.trait_names(&source.implied_traits([globals.clone()]));
        writeln!(
            out,
            "\nconst {}: Globals = Globals {{\n    attributes: {},\n    namespaces: {},\n}};",
            globals_const(globals),
            list(&attributes),
            list(&namespaces),
        )
        .expect("writing to a String cannot fail");
    }

    for (module, table) in [
        (ValidationModule::Html, "HTML_ELEMENTS"),
        (ValidationModule::Svg, "SVG_ELEMENTS"),
    ] {
        writeln!(out, "\npub(super) const {table}: &[Element] = &[").expect("infallible");
        for (element, info) in &source.elements[&module] {
            let globals = source.implied_traits([info.globals.clone()]);
            let implemented = source
                .element_impls
                .iter()
                .filter(|(_, m, e)| *m == module && e == element)
                .map(|(implemented, _, _)| implemented.clone());
            let implemented: BTreeSet<_> = source
                .implied_traits(implemented)
                .difference(&globals)
                .cloned()
                .collect();
            for (name, _) in source.trait_consts(&implemented) {
                assert!(
                    source
                        .trait_consts(&globals)
                        .all(|(global, _)| global != name),
                    "`{element}` gets `{name}` from more than one trait"
                );
            }

            let (mut attributes, namespaces) = source.trait_names(&implemented);
            attributes.extend(info.attributes.iter().cloned());
            writeln!(
                out,
                "    Element {{ name: \"{element}\", kind: {}, globals: &{}, attributes: {}, namespaces: {} }},",
                info.kind,
                globals_const(&info.globals),
                list(&attributes),
                list(&namespaces),
            )
            .expect("infallible");
        }
        out.push_str("];\n");
    }

    out.push_str("\npub(super) const MODULES: &[Module] = &[\n");
    for (path, info) in &source.modules {
        if path.is_empty() {
            continue;
        }
        writeln!(
            out,
            "    Module {{ path: \"{path}\", namespace: {}, attributes: {}, modifiers: {} }},",
            info.namespace,
            list(&info.attributes),
            list(&info.modifiers),
        )
        .expect("infallible");
    }
    out.push_str("];\n");

    out
}

#[test]
fn validation_table_is_up_to_date() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validation_dir = manifest_dir.join("../cheers/src/validation");
    if !validation_dir.exists() {
        eprintln!("skipped: the `cheers` sources are only available in the workspace");
        return;
    }
    let source = Source::read(&validation_dir);
    for module in [ValidationModule::Html, ValidationModule::Svg] {
        for element in source.element_impls.iter().filter(|(_, m, _)| *m == module) {
            assert!(
                source.elements[&module].contains_key(&element.2),
                "impl for unknown element `{}`",
                element.2
            );
        }
    }

    let generated = render(&source);
    let table_path = manifest_dir.join("src/validation/table.rs");

    if std::env::var_os(UPDATE_ENV).is_some() {
        fs::write(&table_path, generated).expect("failed to write the validation table");
        return;
    }

    let current = fs::read_to_string(&table_path).unwrap_or_default();
    assert!(
        current == generated,
        "`src/validation/table.rs` is out of date with `crates/cheers/src/validation`; \
         regenerate it with `{UPDATE_ENV}=1 cargo test -p cheers-ast validation_table`"
    );
}
