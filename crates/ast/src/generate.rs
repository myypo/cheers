use std::collections::BTreeMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::{
    Error, LitStr, braced,
    parse::Parse,
    token::{Brace, Paren},
};

use super::{
    AttributeValueNode, DataModifierPart, DataModifiers, ElementNode, Nodes, SyntaxStatic,
    UnquotedName,
};
use crate::validation;

fn escape_script_source_literal(value: &str) -> std::borrow::Cow<'_, str> {
    let bytes = value.as_bytes();
    let script_end = b"</script";
    let mut i = 0;
    let mut start = 0;
    let mut escaped = None::<String>;

    while i + script_end.len() <= bytes.len() {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'/'
            && bytes[i + 2..i + script_end.len()].eq_ignore_ascii_case(b"script")
        {
            let escaped = escaped.get_or_insert_with(|| String::with_capacity(value.len() + 1));
            escaped.push_str(&value[start..i]);
            escaped.push_str("<\\/");
            escaped.push_str(&value[i + 2..i + script_end.len()]);
            i += script_end.len();
            start = i;
        } else {
            i += 1;
        }
    }

    if let Some(mut escaped) = escaped {
        escaped.push_str(&value[start..]);
        std::borrow::Cow::Owned(escaped)
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}

pub fn async_key_ident() -> Ident {
    Ident::new("__cheers_async_key", Span::mixed_site())
}

pub fn async_scope_ident() -> Ident {
    Ident::new("__cheers_async_scope", Span::mixed_site())
}

pub fn lazy<T: Parse + Generate + SyntaxStatic>(tokens: TokenStream) -> Result<TokenStream, Error> {
    lazy_with_flavour::<T>(tokens, NodeFlavour::Html)
}

pub fn lazy_with_flavour<T: Parse + Generate + SyntaxStatic>(
    tokens: TokenStream,
    flavour: NodeFlavour,
) -> Result<TokenStream, Error> {
    let mut state = GeneratorState::new();
    let mut g = Generator::new_closure(T::CONTEXT, flavour, &mut state);

    let mut input = syn::parse2::<T>(tokens)?;
    let syntax_static = input.is_static();
    g.push(&mut input);

    let block = g.finish();
    let borrow_captures = state.captures;
    let has_async = state.has_async;

    let buffer_ident = Generator::buffer_ident();

    let marker_ident = T::CONTEXT.marker_type();
    let rendered = if has_async {
        let async_scope_items = &block.async_scope_items;
        let async_scope_ident = async_scope_ident();

        quote! {
            {
                use ::cheers::validation::attributes::*;
                #(#borrow_captures)*
                #(#async_scope_items)*

                ::cheers::prelude::AsyncLazy::__new(
                    move |
                        #buffer_ident: &mut ::cheers::prelude::Buffer<#marker_ident>,
                        #async_scope_ident: &::cheers::__internal::async_streams::AsyncScope,
                    | {
                        ::cheers::__internal::subsecond::call(|| {
                            #block
                        })
                    }
                )
            }
        }
    } else {
        let lazy = if !syntax_static {
            // Dynamic render bodies can contain arbitrary Rust expressions. Keep them as normal
            // closures instead of guessing whether their paths capture caller locals.
            quote! {
                ::cheers::prelude::Lazy::<_, #marker_ident>::dangerously_create(
                    move |#buffer_ident: &mut ::cheers::prelude::Buffer<#marker_ident>| {
                        ::cheers::__internal::subsecond::call(|| {
                            #block
                        })
                    }
                )
            }
        } else {
            // Syntactically static render bodies cannot reference caller locals. Coerce the
            // generated closure into a real function pointer so Subsecond has a precise hot
            // boundary.
            quote! {
                {
                    let __cheers_subsecond_hot_render: fn(&mut ::cheers::prelude::Buffer<#marker_ident>) = |#buffer_ident| {
                        #block
                    };

                    ::cheers::prelude::Lazy::<_, #marker_ident>::dangerously_create(
                        move |#buffer_ident: &mut ::cheers::prelude::Buffer<#marker_ident>| {
                            ::cheers::__internal::subsecond::hot_call(
                                __cheers_subsecond_hot_render,
                                (#buffer_ident,),
                            );
                        }
                    )
                }
            }
        };

        quote! {
            {
                use ::cheers::validation::attributes::*;
                #(#borrow_captures)*

                #lazy
            }
        }
    };

    Ok(rendered)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeFlavour {
    Html,
    Xml(XmlFlavour),
}

impl NodeFlavour {
    pub const fn void_close(self) -> &'static str {
        match self {
            Self::Html => ">",
            Self::Xml(_) => "/>",
        }
    }

    pub const fn elements_module(self) -> ValidationModule {
        match self {
            Self::Html => ValidationModule::Html,
            Self::Xml(XmlFlavour::Svg) => ValidationModule::Svg,
            Self::Xml(XmlFlavour::MathMl) => ValidationModule::MathMl,
        }
    }

    pub const fn element_kind(self, is_void: bool) -> ElementKind {
        match self {
            Self::Html => {
                if is_void {
                    ElementKind::Void
                } else {
                    ElementKind::Normal
                }
            }
            Self::Xml(_) => ElementKind::Xml,
        }
    }

    pub fn child_flavour(self, element_name: &UnquotedName) -> Self {
        match self {
            Self::Html => match element_name {
                name if name == &"svg" => Self::Xml(XmlFlavour::Svg),
                name if name == &"math" => Self::Xml(XmlFlavour::MathMl),
                _ => self,
            },
            Self::Xml(XmlFlavour::Svg) => match element_name {
                name if name == &"foreignObject" => Self::Html,
                _ => self,
            },
            Self::Xml(XmlFlavour::MathMl) => self,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlFlavour {
    Svg,
    MathMl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationModule {
    Html,
    Svg,
    MathMl,
}

impl ToTokens for ValidationModule {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Html => quote!(::cheers::validation::elements),
            Self::Svg => quote!(::cheers::validation::svg::elements),
            Self::MathMl => quote!(::cheers::validation::mathml::elements),
        }
        .to_tokens(tokens);
    }
}

struct GeneratorState {
    captures: Vec<TokenStream>,
    counter: usize,
    /// Names bound by template control flow (`@let`, `@for`, `@if let`, ...) that are in scope
    /// at the current generation point. They do not exist where borrows are hoisted to.
    local_bindings: Vec<Ident>,
    async_item_counter: usize,
    has_async: bool,
}

impl GeneratorState {
    fn new() -> Self {
        Self {
            captures: Vec::new(),
            counter: 0,
            local_bindings: Vec::new(),
            async_item_counter: 0,
            has_async: false,
        }
    }

    fn hoist_ref_expr(
        &mut self,
        paren_token: Paren,
        expr: impl ToTokens,
        root: Option<&Ident>,
    ) -> TokenStream {
        let mut ref_expr = TokenStream::new();
        paren_token.surround(&mut ref_expr, |tokens| expr.to_tokens(tokens));

        let reference = quote_spanned!(paren_token.span=> &);

        if root.is_some_and(|root| self.local_bindings.contains(root)) {
            return quote!(#reference #ref_expr);
        }

        let ref_idx = self.counter;
        self.counter += 1;

        let ref_ident = format_ident!("__cheers_ref_{ref_idx}", span = Span::mixed_site());

        self.captures.push(quote! {
            let #ref_ident = #reference #ref_expr;
        });

        ref_ident.into_token_stream()
    }
}

pub struct Generator<'a> {
    context: Context,
    flavour: NodeFlavour,
    brace_token: Brace,
    parts: Vec<Part>,
    checks: Checks,
    async_scope_items: Vec<TokenStream>,
    in_loop: bool,
    /// Whether the code generated here runs inside a closure that may be called more than once,
    /// and so cannot move values it captures.
    in_shared_closure: bool,
    context_override: Option<Context>,
    state: &'a mut GeneratorState,
}

impl<'a> Generator<'a> {
    pub fn buffer_ident() -> Ident {
        Ident::new("__hypertext_buffer", Span::mixed_site())
    }

    fn new_closure(context: Context, flavour: NodeFlavour, state: &'a mut GeneratorState) -> Self {
        Self::new_root_with_brace(context, Brace::default(), flavour, state)
    }

    fn new_root_with_brace(
        context: Context,
        brace_token: Brace,
        flavour: NodeFlavour,
        state: &'a mut GeneratorState,
    ) -> Self {
        Self {
            context,
            flavour,
            brace_token,
            parts: Vec::new(),
            checks: Checks::new(),
            async_scope_items: Vec::new(),
            in_loop: false,
            in_shared_closure: true,
            context_override: None,
            state,
        }
    }

    fn new_child_with_brace<'b>(
        &'b mut self,
        brace_token: Brace,
        flavour: NodeFlavour,
    ) -> Generator<'b> {
        Generator {
            context: self.context,
            flavour,
            brace_token,
            parts: Vec::new(),
            checks: Checks::new(),
            async_scope_items: Vec::new(),
            in_loop: self.in_loop,
            in_shared_closure: self.in_shared_closure,
            context_override: self.context_override,
            state: &mut *self.state,
        }
    }

    fn finish(self) -> AnyBlock {
        let buffer_ident = Self::buffer_ident();
        let mut stmts = TokenStream::new();
        let mut parts = self.parts.into_iter();
        let mut size_estimate = 0;

        while let Some(part) = parts.next() {
            match part {
                Part::Static(lit) => {
                    let mut dynamic_stmt = None;
                    let mut static_str = lit.value();
                    for part in parts.by_ref() {
                        match part {
                            Part::Static(lit) => static_str.push_str(&lit.value()),
                            Part::Dynamic(stmt) => {
                                dynamic_stmt = Some(stmt);
                                break;
                            }
                        }
                    }
                    size_estimate += static_str.len();
                    let static_lit = LitStr::new(&static_str, lit.span());

                    // XSS SAFETY: static parts are literal strings pushed by us
                    stmts.extend(quote! {
                        #buffer_ident.dangerously_get_string().push_str(#static_lit);
                    });
                    stmts.extend(dynamic_stmt);
                }
                Part::Dynamic(stmt) => {
                    stmts.extend(stmt);
                }
            }
        }

        // XSS SAFETY: prealoc does not add any content
        let render = quote! {
            #buffer_ident.dangerously_get_string().reserve(#size_estimate);
            #stmts
        };

        let checks = self.checks;

        AnyBlock {
            brace_token: self.brace_token,
            stmts: quote! {
                #checks
                #render
            },
            async_scope_items: self.async_scope_items,
        }
    }

    pub fn block_with(
        &mut self,
        brace_token: Brace,
        f: impl for<'b> FnOnce(&mut Generator<'b>),
        append_async: bool,
    ) -> AnyBlock {
        self.block_with_flavour(brace_token, self.flavour, f, append_async)
    }

    pub fn block_with_flavour(
        &mut self,
        brace_token: Brace,
        flavour: NodeFlavour,
        f: impl for<'b> FnOnce(&mut Generator<'b>),
        append_async: bool,
    ) -> AnyBlock {
        let local_bindings_len = self.state.local_bindings.len();
        let (mut child_checks, mut block) = {
            let mut g = self.new_child_with_brace(brace_token, flavour);

            f(&mut g);

            let child_checks = std::mem::replace(&mut g.checks, Checks::new());
            let block = g.finish();

            (child_checks, block)
        };
        self.state.local_bindings.truncate(local_bindings_len);

        self.checks.append(&mut child_checks);
        if append_async {
            self.async_scope_items.append(&mut block.async_scope_items);
        }

        block
    }

    pub fn push_with_flavour(
        &mut self,
        flavour: NodeFlavour,
        f: impl for<'b> FnOnce(&mut Generator<'b>),
    ) {
        let block = self.block_with_flavour(Brace::default(), flavour, f, true);
        self.push_stmt(block);
    }

    pub fn push_in_block(
        &mut self,
        brace_token: Brace,
        f: impl for<'b> FnOnce(&mut Generator<'b>),
    ) {
        let block = self.block_with(brace_token, f, true);
        self.push_stmt(block);
    }

    pub fn push_str(&mut self, s: &'static str) {
        self.push_spanned_str(s, Span::mixed_site());
    }

    pub fn push_spanned_str(&mut self, s: &'static str, span: Span) {
        self.parts.push(Part::Static(LitStr::new(s, span)));
    }

    pub fn push_escaped_literal(&mut self, context: Context, lit: &LitStr) {
        let value = lit.value();
        let effective_context = self.context_override.unwrap_or(context);
        let escaped_value = match effective_context {
            Context::Element => html_escape::encode_text(&value),
            Context::AttributeValue | Context::DatastarSource => {
                html_escape::encode_double_quoted_attribute(&value)
            }
            Context::ScriptSource => escape_script_source_literal(&value),
        };

        self.parts
            .push(Part::Static(LitStr::new(&escaped_value, lit.span())));
    }

    pub fn push_literals(&mut self, literals: Vec<LitStr>) {
        for lit in literals {
            self.parts.push(Part::Static(lit));
        }
    }

    pub fn push_literal(&mut self, lit: LitStr) {
        self.parts.push(Part::Static(lit));
    }

    #[cfg(feature = "pi-extension")]
    pub fn push_element_source_hint(&mut self, source: LitStr) {
        let buffer_ident = Self::buffer_ident();
        self.push_stmt(quote! {
            #[cfg(debug_assertions)]
            {
                ::cheers::__internal::pi_extension::__push_element_source_hint(
                    #buffer_ident,
                    #source,
                );
            }
        });
    }

    pub fn with_context_override<R>(
        &mut self,
        context: Context,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let prev = self.context_override.replace(context);
        let result = f(self);
        self.context_override = prev;
        result
    }

    pub fn push_expr(&mut self, paren_token: Paren, context: Context, expr: impl ToTokens) {
        let effective_context = self.context_override.unwrap_or(context);
        let buffer_ident = Self::buffer_ident();
        let buffer_expr = match (self.context, effective_context) {
            (Context::Element, Context::Element)
            | (Context::AttributeValue, Context::AttributeValue)
            | (Context::DatastarSource, Context::DatastarSource)
            | (Context::ScriptSource, Context::ScriptSource) => {
                quote!(#buffer_ident)
            }
            (Context::Element, Context::AttributeValue) => {
                quote!(#buffer_ident.as_attribute_buffer())
            }
            (Context::Element, Context::DatastarSource) => {
                quote!(#buffer_ident.as_datastar_buffer())
            }
            (Context::Element, Context::ScriptSource) => {
                quote!(#buffer_ident.as_script_buffer())
            }
            (Context::AttributeValue, Context::DatastarSource) => {
                quote!(#buffer_ident.as_datastar_buffer())
            }
            (Context::AttributeValue, Context::ScriptSource) => unreachable!(),
            (Context::DatastarSource, Context::Element) => unreachable!(),
            (Context::DatastarSource, Context::AttributeValue) => {
                quote!(#buffer_ident.as_attribute_buffer())
            }
            (Context::DatastarSource, Context::ScriptSource) => unreachable!(),
            (Context::AttributeValue, Context::Element) => unreachable!(),
            (Context::ScriptSource, Context::Element)
            | (Context::ScriptSource, Context::AttributeValue)
            | (Context::ScriptSource, Context::DatastarSource) => unreachable!(),
        };

        let mut paren_expr = TokenStream::new();
        paren_token.surround(&mut paren_expr, |tokens| expr.to_tokens(tokens));
        let reference = quote_spanned!(paren_token.span=> &);
        self.push_stmt(quote! {
            ::cheers::prelude::Render::render_to(
                #reference #paren_expr,
                #buffer_expr
            );
        });
    }

    pub fn push_js_value_node(&mut self, node: &mut AttributeValueNode) {
        self.with_context_override(Context::DatastarSource, |g| g.push(node));
    }

    /// Borrows `expr` outside the render closure so the closure captures only the reference.
    /// Expressions rooted at a template-local binding are borrowed in place instead, since the
    /// binding does not exist outside the closure. `root` is the variable `expr` borrows from.
    pub fn hoist_ref_expr(
        &mut self,
        paren_token: Paren,
        expr: impl ToTokens,
        root: Option<&Ident>,
    ) -> TokenStream {
        self.state.hoist_ref_expr(paren_token, expr, root)
    }

    pub fn push_ref_expr(
        &mut self,
        paren_token: Paren,
        context: Context,
        expr: impl ToTokens,
        root: Option<&Ident>,
    ) {
        let ref_expr = self.hoist_ref_expr(paren_token, expr, root);
        self.push_expr(Paren::default(), context, ref_expr);
    }

    pub fn declare_local_bindings(&mut self, bindings: impl IntoIterator<Item = Ident>) {
        self.state.local_bindings.extend(bindings);
    }

    pub fn nodes_use_local_bindings(&self, nodes: &Nodes<ElementNode>) -> bool {
        let bindings = &self.state.local_bindings;
        !bindings.is_empty() && crate::uses::nodes_use_any(nodes, bindings)
    }

    pub const fn in_loop(&self) -> bool {
        self.in_loop
    }

    pub fn with_in_loop<R>(&mut self, in_loop: bool, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = std::mem::replace(&mut self.in_loop, in_loop);
        let result = f(self);
        self.in_loop = prev;
        result
    }

    pub const fn in_shared_closure(&self) -> bool {
        self.in_shared_closure
    }

    pub fn with_in_shared_closure<R>(
        &mut self,
        in_shared_closure: bool,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let prev = std::mem::replace(&mut self.in_shared_closure, in_shared_closure);
        let result = f(self);
        self.in_shared_closure = prev;
        result
    }

    pub fn push_async_slot(&mut self, stream: impl ToTokens) -> Ident {
        let slot_ident = self.next_async_item_ident("slot");
        let key_ident = async_key_ident();

        self.async_scope_items.push(quote! {
            let #slot_ident = ::cheers::__internal::async_streams::AsyncSlot::new(
                move |#key_ident: ::cheers::__internal::async_streams::AsyncKey| -> ::cheers::__internal::async_streams::AsyncStream {
                    ::std::boxed::Box::pin(#stream)
                }
            );
        });

        slot_ident
    }

    pub fn push_async_instances(&mut self) -> Ident {
        let instances_ident = self.next_async_item_ident("instances");

        self.async_scope_items.push(quote! {
            let #instances_ident = ::cheers::__internal::async_streams::AsyncInstances::new();
        });

        instances_ident
    }

    fn next_async_item_ident(&mut self, kind: &str) -> Ident {
        let idx = self.state.async_item_counter;
        self.state.async_item_counter += 1;
        format_ident!("__cheers_async_{kind}_{idx}", span = Span::mixed_site())
    }

    pub fn mark_async(&mut self) {
        self.state.has_async = true;
    }

    pub fn push_stmt(&mut self, stmt: impl ToTokens) {
        self.parts.push(Part::Dynamic(stmt.to_token_stream()));
    }

    pub fn push_conditional(
        &mut self,
        cond: impl ToTokens,
        f: impl for<'b> FnOnce(&mut Generator<'b>),
    ) {
        let then_block = self.block_with(Brace::default(), f, true);
        self.push_stmt(quote! {
            if #cond #then_block
        });
    }

    pub fn push(&mut self, mut value: impl Generate) {
        value.generate(self);
    }

    pub fn record_element(&mut self, el_checks: ElementCheck) {
        self.checks.push_element(el_checks);
    }

    pub fn push_diagnostic(&mut self, diagnostic: impl ToTokens) {
        self.checks.push_diagnostic(diagnostic.to_token_stream());
    }

    pub const fn node_flavour(&self) -> NodeFlavour {
        self.flavour
    }

    pub fn push_all(&mut self, values: impl IntoIterator<Item = impl Generate>) {
        for value in values {
            self.push(value);
        }
    }
}

enum Part {
    Static(LitStr),
    Dynamic(TokenStream),
}

#[derive(Debug, Clone, Copy)]
pub enum Context {
    Element,
    AttributeValue,
    DatastarSource,
    ScriptSource,
}

impl Context {
    pub fn marker_type(self) -> TokenStream {
        let ident = match self {
            Self::Element => Ident::new("Element", Span::mixed_site()),
            Self::AttributeValue => Ident::new("AttributeValue", Span::mixed_site()),
            Self::DatastarSource => Ident::new("DatastarSource", Span::mixed_site()),
            Self::ScriptSource => Ident::new("ScriptSource", Span::mixed_site()),
        };

        quote!(::cheers::prelude::#ident)
    }
}

pub trait Generate {
    const CONTEXT: Context;
    fn generate(&mut self, g: &mut Generator<'_>);
}

impl<T: Generate> Generate for &mut T {
    const CONTEXT: Context = T::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        (*self).generate(g);
    }
}

struct Checks {
    elements: Vec<ElementCheck>,
    recovered_errors: Vec<TokenStream>,
}

impl Checks {
    const fn new() -> Self {
        Self {
            elements: Vec::new(),
            recovered_errors: Vec::new(),
        }
    }

    fn append(&mut self, other: &mut Self) {
        self.elements.append(&mut other.elements);
        self.recovered_errors.append(&mut other.recovered_errors);
    }

    fn push_element(&mut self, element: ElementCheck) {
        self.elements.push(element);
    }

    fn push_diagnostic(&mut self, diagnostic: TokenStream) {
        self.recovered_errors.push(diagnostic);
    }

    fn block(module: ValidationModule, checks: &TokenStream) -> TokenStream {
        quote! {
            const _: fn() = || {
                #[allow(unused_imports)]
                use #module::*;

                #[doc(hidden)]
                /// Used by the `html!`, `svg!`, and `attribute!` macros to
                /// trigger compile-time element
                /// validation.
                fn check_element<
                    K: ::cheers::validation::ElementKind
                >(_: impl ::cheers::validation::Element<Kind = K>) {}

                #checks
            };
        }
    }
}

fn by_module(elements: &[ElementCheck]) -> BTreeMap<ValidationModule, Vec<&ElementCheck>> {
    let mut by_module: BTreeMap<ValidationModule, Vec<&ElementCheck>> = BTreeMap::new();
    for check in elements {
        by_module.entry(check.module).or_default().push(check);
    }
    by_module
}

/// Emits every check in `elements`, including those [`validation`] knows would pass.
pub(crate) fn all_checks(elements: &[ElementCheck]) -> TokenStream {
    by_module(elements)
        .into_iter()
        .map(|(module, checks)| {
            let checks: TokenStream = checks.iter().map(|check| check.checks()).collect();
            Checks::block(module, &checks)
        })
        .collect()
}

/// Emits the checks rustc needs, skipping those [`validation`] knows would pass. The names of
/// skipped checks are still referenced for rust-analyzer, which needs them for hover and
/// go-to-definition on element and attribute names.
impl ToTokens for Checks {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for diagnostic in &self.recovered_errors {
            diagnostic.to_tokens(tokens);
        }

        for (module, checks) in by_module(&self.elements) {
            let mut needed = TokenStream::new();
            let mut references = TokenStream::new();
            for check in checks {
                check.split_checks(&mut needed, &mut references);
            }

            if !needed.is_empty() {
                Self::block(module, &needed).to_tokens(tokens);
            }
            if !references.is_empty() {
                quote! {
                    #[allow(unexpected_cfgs)]
                    {
                        #[cfg(rust_analyzer)]
                        {
                            #[allow(unused_imports)]
                            use #module::*;
                            #references
                        }
                    }
                }
                .to_tokens(tokens);
            }
        }
    }
}

pub struct ElementCheck {
    module: ValidationModule,
    ident: UnquotedName,
    kind: ElementKind,
    attributes: Vec<AttributeNameCheck>,
}

impl ElementCheck {
    pub fn new(
        el_name: &UnquotedName,
        element_kind: ElementKind,
        module: ValidationModule,
    ) -> Self {
        Self {
            module,
            ident: el_name.clone(),
            kind: element_kind,
            attributes: Vec::new(),
        }
    }

    pub fn push_attribute(&mut self, attr: AttributeNameCheck) {
        self.attributes.push(attr);
    }

    fn element_check(&self) -> TokenStream {
        let el = &self.ident;
        let kind = self.kind;
        quote! {
            check_element::<#kind>(#el);
        }
    }

    fn checks(&self) -> TokenStream {
        let el_check = self.element_check();
        let attr_checks = self
            .attributes
            .iter()
            .map(|attr| attr.to_token_stream_with_el(&self.ident));

        quote! {
            #el_check
            #(#attr_checks)*
        }
    }

    /// Splits the checks into those [`validation`] does not know to pass, and plain references to
    /// the names of the others.
    fn split_checks(&self, needed: &mut TokenStream, references: &mut TokenStream) {
        let element = validation::element(self.module, &self.ident.name());

        if element.is_some_and(|element| element.kind() == self.kind) {
            let el = &self.ident;
            references.extend(quote!(#el;));
        } else {
            needed.extend(self.element_check());
        }

        for attr in &self.attributes {
            if attr.is_known(element) {
                references.extend(attr.reference_with_el(&self.ident));
            } else {
                needed.extend(attr.to_token_stream_with_el(&self.ident));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Normal,
    Void,
    Xml,
}

impl ToTokens for ElementKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Normal => quote!(::cheers::validation::Normal),
            Self::Void => quote!(::cheers::validation::Void),
            Self::Xml => quote!(::cheers::validation::Xml),
        }
        .to_tokens(tokens);
    }
}

pub struct AttributeNameCheck {
    kind: AttributeNameCheckKind,
    ident: UnquotedName,
    data: bool,
    data_modifiers: Vec<UnquotedName>,
}

struct DataModifierNameCheck<'a>(&'a UnquotedName);

impl DataModifierNameCheck<'_> {
    /// `self` cannot be used as an item name, so the validation table stores it as `self_`
    /// while rendering still emits the Datastar modifier name `self`.
    fn item_name(&self) -> String {
        if self.0 == &"self" {
            "self_".to_owned()
        } else {
            self.0.name()
        }
    }
}

impl ToTokens for DataModifierNameCheck<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        UnquotedName(Ident::new(&self.item_name(), self.0.span())).to_tokens(tokens);
    }
}

impl AttributeNameCheck {
    pub fn new(kind: AttributeNameCheckKind, ident: UnquotedName, data: bool) -> Self {
        Self {
            kind,
            ident,
            data,
            data_modifiers: Vec::new(),
        }
    }

    pub fn push_data_modifier(&mut self, modifier: UnquotedName) {
        self.data_modifiers.push(modifier);
    }

    pub fn push_data_modifiers(&mut self, modifiers: Option<&DataModifiers>) {
        if let Some(modifiers) = modifiers {
            self.data_modifiers
                .extend(
                    modifiers
                        .modifiers
                        .iter()
                        .filter_map(|modifier| match &modifier.name {
                            DataModifierPart::Ident(ident) => Some(ident.clone()),
                            DataModifierPart::Literal(_) => None,
                        }),
                );
        }
    }

    fn data_plugin(&self) -> &UnquotedName {
        match &self.kind {
            AttributeNameCheckKind::Normal => &self.ident,
            AttributeNameCheckKind::Namespace(namespace) => namespace,
        }
    }

    fn data_modifier_checks(&self) -> TokenStream {
        if !self.data || self.data_modifiers.is_empty() {
            return TokenStream::new();
        }

        let plugin = self.data_plugin();
        let modifiers = self.data_modifiers.iter().map(DataModifierNameCheck);

        quote! {
            #(
                let _: ::cheers::validation::data::Modifier = ::cheers::validation::data::modifiers::#plugin::#modifiers;
            )*
        }
    }

    /// Whether this check passes for `element`, as far as [`validation`] knows.
    fn is_known(&self, element: Option<&validation::Element>) -> bool {
        let name = self.ident.name();
        let known = match (&self.kind, self.data) {
            (AttributeNameCheckKind::Normal, false) => {
                element.is_some_and(|element| element.has_attribute(&name))
            }
            (AttributeNameCheckKind::Namespace(namespace), false) => {
                let namespace = namespace.name();
                element.is_some_and(|element| element.has_namespace(&namespace))
                    && validation::module(&namespace)
                        .is_some_and(|module| module.has_namespace() && module.has_attribute(&name))
            }
            (AttributeNameCheckKind::Normal, true) => {
                validation::module("data").is_some_and(|module| module.has_attribute(&name))
            }
            (AttributeNameCheckKind::Namespace(namespace), true) => {
                validation::module(&format!("data::{}", namespace.name()))
                    .is_some_and(|module| module.has_namespace() && module.has_attribute(&name))
            }
        };

        known && self.data_modifiers_known()
    }

    fn data_modifiers_known(&self) -> bool {
        if !self.data || self.data_modifiers.is_empty() {
            return true;
        }

        let plugin = self.data_plugin().name();
        let Some(module) = validation::module(&format!("data::modifiers::{plugin}")) else {
            return false;
        };

        self.data_modifiers
            .iter()
            .all(|modifier| module.has_modifier(&DataModifierNameCheck(modifier).item_name()))
    }

    fn reference_with_el(&self, el: &UnquotedName) -> TokenStream {
        let ident = &self.ident;
        let name = match &self.kind {
            AttributeNameCheckKind::Namespace(namespace) if self.data => {
                quote!(::cheers::validation::data::#namespace::#ident;)
            }
            AttributeNameCheckKind::Namespace(namespace) => {
                quote!(<#el>::#namespace; ::cheers::validation::#namespace::#ident;)
            }
            AttributeNameCheckKind::Normal if self.data => {
                quote!(::cheers::validation::data::#ident;)
            }
            AttributeNameCheckKind::Normal => quote!(<#el>::#ident;),
        };
        if !self.data {
            return name;
        }
        let plugin = self.data_plugin();
        let modifiers = self.data_modifiers.iter().map(DataModifierNameCheck);

        quote! {
            #name
            #(::cheers::validation::data::modifiers::#plugin::#modifiers;)*
        }
    }

    fn to_token_stream_with_el(&self, el: &UnquotedName) -> TokenStream {
        let data_modifier_checks = self.data_modifier_checks();

        match &self.kind {
            AttributeNameCheckKind::Namespace(namespace) => {
                let ident = &self.ident;

                if self.data {
                    quote! {
                        {
                            let _: ::cheers::validation::data::#namespace::Namespace = ::cheers::validation::data::#namespace::Namespace;
                            #[allow(unused_imports)]
                            use ::cheers::validation::data::#namespace::*;
                            let _: ::cheers::validation::Attribute = #ident;
                            #data_modifier_checks
                        }
                    }
                } else {
                    quote! {
                        let _: ::cheers::validation::#namespace::Namespace = <#el>::#namespace;
                        let _: ::cheers::validation::Attribute = ::cheers::validation::#namespace::#ident;
                    }
                }
            }
            AttributeNameCheckKind::Normal => {
                let ident = &self.ident;
                if self.data {
                    quote! {
                        let _: ::cheers::validation::Attribute = ::cheers::validation::data::#ident;
                        #data_modifier_checks
                    }
                } else {
                    quote! {
                        let _: ::cheers::validation::Attribute = <#el>::#ident;
                    }
                }
            }
        }
    }
}

pub enum AttributeNameCheckKind {
    Normal,
    Namespace(UnquotedName),
}

pub struct AnyBlock {
    pub brace_token: Brace,
    pub stmts: TokenStream,
    /// `@async` slots and instance counters that must be defined at the root of the enclosing
    /// async scope.
    pub async_scope_items: Vec<TokenStream>,
}

impl Parse for AnyBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            brace_token: braced!(content in input),
            stmts: content.parse()?,
            async_scope_items: Vec::new(),
        })
    }
}

impl ToTokens for AnyBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.brace_token.surround(tokens, |tokens| {
            self.stmts.to_tokens(tokens);
        });
    }
}
