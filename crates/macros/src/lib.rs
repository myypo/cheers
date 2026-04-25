#![expect(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod action;
mod cheers;
mod scoped_signal;
mod shared;

use ast::{
    AttributeValueNode, Document, JsSourceNodes, Nodes,
    generate::{NodeFlavour, XmlFlavour},
};
use syn::{ItemStruct, parse_macro_input};

use crate::{
    action::ActionArgs,
    shared::{MaybeItemFn, generate_field_bindings},
};

fn expand_document_lazy(
    tokens: proc_macro::TokenStream,
    flavour: NodeFlavour,
) -> proc_macro::TokenStream {
    ast::generate::lazy_with_flavour::<Document>(tokens.into(), flavour)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

fn expand_attribute_lazy(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    ast::generate::lazy::<Nodes<AttributeValueNode>>(tokens.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

fn expand_js_lazy(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    ast::generate::lazy::<JsSourceNodes>(tokens.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Cheers, attributes(id, signal, form, form_derive, prop))]
/// Derives id, signal, form, and component-prop helpers for a component struct.
///
/// This derive does **not** implement `Render`. A type becomes
///
/// usable as a component in `html!` by implementing `Render`; `#[derive(Cheers)]` only
/// adds the supporting APIs around that type.
///
/// # Generated APIs
///
/// Depending on which attributes you use, the derive generates:
///
/// - DOM id associated functions from `#[id]` and `#[id("namespace")]`; inside the component,
///   bind those ids with [`ids!`]
/// - signal associated functions and deserialization types from `#[signal]` and
///   `#[signal(name: Type)]`; inside the component, bind those signals with [`signals!`]
/// - form field-name bindings and a deserializable `...Form` type from `#[form]` and
///   `#[form_derive(...)]`; inside the component, bind those names with [`form_names!`]
/// - support for `#[prop(default(...))]`
///
/// Form names are component-local and are not meant to be referenced from outside the
/// component. Ids and signals can be referenced from outside through the generated associated
/// functions.
///
/// The companion macros [`ids!`], [`signals!`], and [`form_names!`] expose the generated
/// bindings inside methods on the component. They are intentionally exhaustive: if you derive
/// an id, signal, or form name, you are expected to bind it explicitly where you use it.
///
/// That behavior is intentional: component code should derive only the ids, signals, and form
/// names it actually uses, rather than generating extras and silently ignoring them.
///
/// # Supported attributes
///
/// - `#[id]` on at most one field marks the component instance id.
/// - `#[id("namespace")]` on the struct generates additional namespaced ids.
/// - `#[signal]` on a field generates a signal accessor for that field.
/// - `#[signal(nested)]` on a field nests another component's signal scope.
/// - `#[signal(name: Type)]` on the struct generates an extra signal that is not backed by a
///   field.
/// - `#[form]` on a field includes that field in the generated form type.
/// - `#[form(...)]` on a field forwards additional field attributes, such as serde
///   attributes, to the generated form field.
/// - `#[form(name: Type)]` on the struct adds an extra field to the generated form type.
/// - `#[form_derive(...)]` adds derives to the generated `...Form` type.
/// - `#[prop(default(...))]` marks a component field as optional when used from `html!`.
///   Optional props are provided through a trailing `(prop=value)` group; use `()` to opt into
///   defaults without overriding any optional props. This syntax cannot be combined with `..`
///   defaults syntax.
///
/// # Example
///
/// ```ignore
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// #[id("input")]
/// struct TodoItem {
///     #[id]
///     id: u64,
///     #[signal]
///     done: bool,
///     #[form]
///     title: String,
/// }
///
/// impl Render for TodoItem {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         ids!(id, id_input);
///         signals!(signal_done);
///         form_names!(form_title);
///
///         html! {
///             label for=id_input {
///                 input id=id type="checkbox" !bind(signal_done) name=(form_title);
///             }
///         }
///         .render_to(buffer);
///     }
/// }
///
/// let rendered = TodoItem {
///     id: 1,
///     done: false,
///     title: String::from("Write docs"),
/// }
/// .render()
/// .into_inner();
///
/// assert!(rendered.contains("id=\"todo_item-1\""));
/// assert!(rendered.contains("for=\"todo_item-1-input\""));
/// assert!(rendered.contains("data-bind=\"todo_item['1']['done']\""));
/// assert!(rendered.contains("name=\"title\""));
/// ```
pub fn cheers_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as ItemStruct);

    cheers::generate(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
/// Builds a lazily rendered HTML fragment.
///
/// Use `html!` for normal Cheers markup. It can render elements, text, components, control flow,
/// and interpolated values with `(expr)`.
///
/// `html!` captures interpolated values by value. Use `(@&expr)` in expression positions when
/// you want to borrow instead and keep using the original value after the macro invocation.
///
/// # Example
///
/// ```ignore
/// use cheers::prelude::*;
///
/// let name = String::from("Ferris");
/// let page = html! {
///     section {
///         h1 { "Hello" }
///         p { (name) }
///     }
/// };
///
/// assert_eq!(page.render().into_inner(), "<section><h1>Hello</h1><p>Ferris</p></section>");
/// ```
pub fn html(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_document_lazy(tokens, NodeFlavour::Html)
}

#[proc_macro]
/// Builds a lazily rendered SVG fragment or document.
///
/// Use `svg!` when generating standalone SVG, such as sprite bundles served as
/// `image/svg+xml`. Unlike [`html!`], this macro starts in SVG/XML mode from
/// the root, validates elements against the SVG validation table, and renders
/// childless SVG elements with XML-style self-closing syntax (`/>`).
///
/// To embed inline SVG inside HTML, prefer [`html!`] and a normal `<svg>`
/// element. Cheers will switch into SVG validation automatically inside that
/// subtree.
///
/// # Example
///
/// ```ignore
/// use cheers::prelude::*;
///
/// let sprite = svg! {
///     svg viewBox="0 0 16 16" xml:lang="en" {
///         defs {
///             symbol id="icon-check" viewBox="0 0 16 16" {
///                 path d="M6.5 11.2 3.3 8l-1.1 1.1 4.3 4.3L14 5.9l-1.1-1.1z";
///             }
///         }
///     }
/// };
///
/// assert!(sprite.render().into_inner().contains("<symbol id=\"icon-check\""));
/// ```
pub fn svg(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_document_lazy(tokens, NodeFlavour::Xml(XmlFlavour::Svg))
}

#[proc_macro]
/// Builds an attribute value from literal and interpolated fragments.
///
/// `attribute!` captures interpolated values by value. Use `(@&expr)` to borrow an
/// interpolated value instead.
///
/// # Example
///
/// ```ignore
/// use cheers::macros::attribute;
/// use cheers::prelude::*;
///
/// let kind = String::from("primary");
/// let class = attribute! { "btn btn-" (kind) };
/// let page = html! {
///     button class=class { "Save" }
/// };
///
/// assert_eq!(page.render().into_inner(), r#"<button class="btn btn-primary">Save</button>"#);
/// ```
pub fn attribute(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_attribute_lazy(tokens)
}

#[proc_macro]
/// Builds a JavaScript source fragment for Datastar attributes.
///
/// # Example
///
/// ```ignore
/// use cheers::prelude::*;
///
/// let onclick = js! {
///     "console.log("
///     (signal_name)
///     ")"
/// };
///
/// html! {
///     button !on:click(onclick) { "Log" }
/// };
/// ```
pub fn js(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_js_lazy(tokens)
}

#[proc_macro]
/// Creates a component-local signal whose name is scoped to the current component instance and call site.
///
/// Use this when defining local state inside a component method, not when referencing a
/// component signal from outside. Scoped signals are intentionally internal to the component
/// that creates them.
///
/// This is useful for component-local UI state such as loading spinners.
///
/// The type hint is optional.
///
/// ```ignore
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// struct Projects {
///     #[id]
///     id: u64,
/// }
///
/// impl Render for Projects {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         scoped_signal!(signal_fetching: bool);
///         scoped_signal!(signal_busy);
///
///         html! {
///             button !on:click("@get('/items')") !indicator(signal_fetching) { "Refresh" }
///             div !show(signal_fetching) { "Loading..." }
///             p !signals(signal_busy: true) {}
///         }
///         .render_to(buffer);
///     }
/// }
/// ```
pub fn scoped_signal(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    scoped_signal::expand(tokens)
}

#[proc_macro_attribute]
/// Generates a renderable action reference from an async handler function.
///
/// Applying `#[action(METHOD)]` generates a companion `...Action` type that renders to the
/// client-side action string used by Cheers attributes such as `!on:click`.
///
/// Path parameters are taken from `Path<_>` arguments. Form submission is enabled when the
/// handler takes a `Form<_>` argument or an argument marked with `#[form]`.
/// Register the generated route explicitly with `Router::action::<...Action>()`.
///
/// # Example
///
/// ```ignore
/// use cheers::prelude::*;
/// use axum::extract::Path;
///
/// #[action(POST)]
/// async fn save_user(Path(id): Path<u64>) {}
///
/// let action = SaveUserAction { id: 7 };
/// assert_eq!(action.render().into_inner(), "@post('/cheers/actions/save_user/7')");
/// ```
pub fn action(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as ActionArgs);
    let mut item = parse_macro_input!(item as MaybeItemFn);
    action::generate(args, &mut item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
/// Binds every signal generated by `#[derive(Cheers)]` for `self`.
///
/// This macro is the way to acquire generated component signals inside the component that
/// defines them.
///
/// For component-local state created inside a component, use `scoped_signal!`.
/// Outside the component, use the generated associated functions such as
/// `YourComponent::signal_name(...)` instead.
///
/// This macro is intentionally exhaustive: if your component derives signals, you are
/// expected to bind all of them explicitly.
///
/// That is intentional: component code is expected to derive only the signals it actually
/// uses. If you do not want to bind a generated signal, do not derive it.
///
/// This macro is intended for methods with a `self` receiver of your component.
///
/// ```ignore
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// struct Counter {
///     #[signal]
///     count: i32,
/// }
///
/// impl Render for Counter {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         signals!(signal_count);
///
///         html! {
///             span !text(signal_count) {}
///         }
///         .render_to(buffer);
///     }
/// }
///
/// assert_eq!(
///     Counter { count: 3 }.render().into_inner(),
///     r#"<span data-text="$counter['count']"></span>"#,
/// );
/// ```
pub fn signals(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    generate_field_bindings(
        tokens,
        "__signals",
        quote::quote!(::cheers::__internal::Signals),
    )
}

#[proc_macro]
/// Binds every id generated by `#[derive(Cheers)]` for `self`.
///
/// This macro is the way to acquire `ElementId` values inside the component that defines
/// those ids.
///
/// Outside the component, use the generated associated functions such as `YourComponent::id(...)`
/// and `YourComponent::id_name(...)` instead.
///
/// This macro is intentionally exhaustive: if your component derives ids, you are expected to
/// bind all of them explicitly.
///
/// That is intentional: ids should be derived only when they are actually used. If you do not
/// want to bind a particular generated id, do not derive it in the first place.
///
/// This macro is intended for methods with a `self` receiver of your component.
///
/// ```ignore
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// #[id("title")]
/// struct Panel {
///     #[id]
///     id: u64,
/// }
///
/// impl Render for Panel {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         ids!(id, id_title);
///
///         html! {
///             section id=id {
///                 h2 id=id_title { "Panel" }
///             }
///         }
///         .render_to(buffer);
///     }
/// }
///
/// assert_eq!(
///     Panel { id: 7 }.render().into_inner(),
///     r#"<section id="panel-7"><h2 id="panel-7-title">Panel</h2></section>"#,
/// );
/// ```
pub fn ids(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    generate_field_bindings(tokens, "__ids", quote::quote!(::cheers::__internal::Ids))
}

#[proc_macro]
/// Binds every form field name generated by `#[derive(Cheers)]` for `self`.
///
/// This macro is the only intended way to acquire generated form names. Form names are
/// component-local and are not meant to be referenced from outside the component.
///
/// This macro is intentionally exhaustive: if your component derives form names, you are
/// expected to bind all of them explicitly.
///
/// That is intentional: component code is expected to derive only the form fields it actually
/// uses. If you do not want to bind a generated form name, do not derive it.
///
/// This macro is intended for methods with a `self` receiver, typically inside `impl Render`
/// for your component.
///
/// ```ignore
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// struct LoginForm {
///     #[form]
///     email: String,
/// }
///
/// impl Render for LoginForm {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         form_names!(form_email);
///
///         html! {
///             input name=(form_email);
///         }
///         .render_to(buffer);
///     }
/// }
///
/// assert_eq!(
///     LoginForm {
///         email: String::from("hello@example.com"),
///     }
///     .render()
///     .into_inner(),
///     r#"<input name="email">"#,
/// );
/// ```
pub fn form_names(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    generate_field_bindings(
        tokens,
        "__form_names",
        quote::quote!(::cheers::__internal::FormNames),
    )
}
