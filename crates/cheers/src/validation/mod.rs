//! Types and traits used for validation of HTML elements and attributes.
pub mod attributes;
pub mod elements;
#[cfg(feature = "mathml")]
pub mod mathml;
pub mod svg;

// Re-export attribute namespace modules at the validation level
pub use attributes::*;

macro_rules! define_validation_elements {
    (
        kind = $kind:path,
        globals = $globals:path,
        {
            $(
                $(#[$meta:meta])*
                $name:ident $(
                    {
                        $(
                            $(#[$attr_meta:meta])*
                            $attr:ident
                        )*
                    }
                )?
            )*
        }
    ) => {
        $(
            $(#[$meta])*
            #[expect(
                non_camel_case_types,
                reason = "camel case types will be interpreted as components"
            )]
            #[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::marker::Copy)]
            pub struct $name;

            $(
                #[allow(non_upper_case_globals)]
                impl $name {
                    $(
                        $(#[$attr_meta])*
                        pub const $attr: $crate::validation::Attribute = $crate::validation::Attribute;
                    )*
                }
            )?

            impl $crate::validation::Element for $name {
                type Kind = $kind;
            }

            impl $globals for $name {}
        )*
    };
}

use define_validation_elements;

/// A marker trait for type-checked elements.
pub trait Element {
    /// The kind of this element.
    type Kind: ElementKind;
}
/// A marker trait to represent the kind of an element.
///
/// This can be either [`Normal`], [`Void`], or [`Xml`]. A [`Normal`] element
/// will always have a closing tag and can have children. A [`Void`] element
/// will never have a closing tag and cannot have children. An [`Xml`] element
/// can either have children and a closing tag or render as a self-closing
/// element.
pub trait ElementKind: sealed::Sealed {}

/// A marker type to represent a normal element.
///
/// This element has a closing tag and can have children.
///
/// # Example
///
/// ```html
/// <div>
///   Hello, world!
/// </div>
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Normal;

impl ElementKind for Normal {}

/// A marker type to represent a void element.
///
/// This element does not have a closing tag and cannot have children.
///
/// # Example
///
/// ```html
/// <img src="image.png" alt="An image">
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Void;

impl ElementKind for Void {}

/// A marker type to represent an XML element.
///
/// XML-flavoured elements, such as SVG and MathML elements embedded inside
/// `html!`, can either have children and a closing tag or render as a
/// self-closing element.
#[derive(Debug, Clone, Copy)]
pub struct Xml;

impl ElementKind for Xml {}

mod sealed {
    use super::{Normal, Void, Xml};

    pub trait Sealed {}
    impl Sealed for Normal {}
    impl Sealed for Void {}
    impl Sealed for Xml {}
}

/// A standard attribute.
#[derive(Debug, Clone, Copy)]
pub struct Attribute;

/// Defines custom Datastar event names for `!on:<event>` validation.
///
/// The generated constants must be in scope where `html!` is invoked. Defining them at module
/// scope makes them available to markup in that module; otherwise import the module containing
/// them with `use your_events::*`.
///
/// An event can also generate a component that emits that browser event when it is mounted. The
/// component's event-options props are optional, so use Cheers' optional-prop syntax (`[]`) when
/// rendering it.
///
/// Generated event components have these optional props:
///
/// - `target`: the DOM target to dispatch from. Defaults to [`EventTarget::This`], the generated
///   self-removing `data-init` element.
/// - `bubbles`: forwarded to `CustomEventInit.bubbles`. Defaults to `true`, so ancestor
///   `!on:<event>` listeners can observe the event when `target` is [`EventTarget::This`].
/// - `cancelable`: forwarded to `CustomEventInit.cancelable`. Defaults to `false`.
/// - `composed`: forwarded to `CustomEventInit.composed`. Defaults to `false`.
///
/// [`EventTarget::This`]: crate::EventTarget::This
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// cheers::define_events! {
///     emoji_click,
///     emoji_selected => EmojiSelected {
///         unicode: &'static str,
///     },
/// }
///
/// # #[derive(Cheers)]
/// # #[id]
/// # struct Composer;
/// # impl Render for Composer {
/// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
/// scoped_signal!(signal_message: String);
///
/// html! {
///     div !signals(signal_message: String::new()) !on:emoji_selected({ (signal_message) " += evt.detail.unicode" }) {
///         textarea !bind(signal_message) {}
///         EmojiSelected unicode="👍" [];
///     }
/// }
/// #         .render_to(buffer);
/// #     }
/// # }
/// # let rendered = Composer.render();
/// # assert!(rendered.as_inner().contains("data-on:emoji-selected="));
/// # assert!(rendered.as_inner().contains("data-init="));
/// # assert!(rendered.as_inner().contains(" += evt.detail.unicode"));
/// ```
#[macro_export]
macro_rules! define_events {
    () => {};

    ($event:ident $(,)?) => {
        #[doc(hidden)]
        #[allow(missing_docs, non_upper_case_globals)]
        pub const $event: $crate::validation::Attribute = $crate::validation::Attribute;
    };

    ($event:ident, $($rest:tt)*) => {
        $crate::define_events! { $event }
        $crate::define_events! { $($rest)* }
    };

    (
        $event:ident => $component:ident {
            $(
                $(#[$field_meta:meta])*
                $field:ident : $ty:ty $(= $default:expr)?
            ),* $(,)?
        }
        $(, $($rest:tt)*)?
    ) => {
        $crate::define_events! { $event }

        #[allow(missing_docs)]
        #[derive($crate::macros::Cheers)]
        pub struct $component<'target> {
            $(
                $(#[$field_meta])*
                $(#[prop(default($default))])?
                pub $field: $ty,
            )*
            /// The DOM target to dispatch the custom event from.
            ///
            /// Defaults to `EventTarget::This`, the generated self-removing `data-init` element.
            /// Use `EventTarget::Document`, `EventTarget::Window`, `EventTarget::Id`, or
            /// `EventTarget::Selector` when the event should originate elsewhere.
            ///
            #[prop(default($crate::EventTarget::This))]
            pub target: $crate::EventTarget<'target>,
            /// Whether the custom event bubbles through the DOM.
            ///
            /// Defaults to `true`, so ancestor `!on:<event>` listeners can observe the event when
            /// `target` is `EventTarget::This`.
            #[prop(default(true))]
            pub bubbles: bool,
            /// Whether the custom event is cancelable.
            ///
            /// Defaults to `false`.
            #[prop(default(false))]
            pub cancelable: bool,
            /// Whether the custom event crosses shadow DOM boundaries.
            ///
            /// Defaults to `false`.
            #[prop(default(false))]
            pub composed: bool,
        }

        impl<'target> $crate::prelude::Render<$crate::prelude::JsSource> for $component<'target> {
            fn render_to(&self, buffer: &mut $crate::prelude::Buffer<$crate::prelude::JsSource>) {
                #[derive($crate::__internal::serde::Serialize)]
                struct EventDetail<'detail> {
                    $(
                        $field: &'detail $ty,
                    )*
                }

                let detail = EventDetail {
                    $(
                        $field: &self.$field,
                    )*
                };

                $crate::__internal::__render_custom_event_to_js(
                    buffer,
                    stringify!($event),
                    Some(&detail),
                    &self.target,
                    self.bubbles,
                    self.cancelable,
                    self.composed,
                );
            }
        }

        impl<'target> $crate::prelude::Render for $component<'target> {
            fn render_to(&self, buffer: &mut $crate::prelude::Buffer<$crate::prelude::Element>) {
                $crate::__internal::__render_custom_event_component(self, buffer);
            }
        }

        $(
            $crate::define_events! { $($rest)* }
        )?
    };

    ($event:ident => $component:ident $(, $($rest:tt)*)?) => {
        $crate::define_events! { $event }

        #[allow(missing_docs)]
        #[derive($crate::macros::Cheers)]
        pub struct $component<'target> {
            /// The DOM target to dispatch the custom event from.
            ///
            /// Defaults to `EventTarget::This`, the generated self-removing `data-init` element.
            /// Use `EventTarget::Document`, `EventTarget::Window`, `EventTarget::Id`, or
            /// `EventTarget::Selector` when the event should originate elsewhere.
            ///
            #[prop(default($crate::EventTarget::This))]
            pub target: $crate::EventTarget<'target>,
            /// Whether the custom event bubbles through the DOM.
            ///
            /// Defaults to `true`, so ancestor `!on:<event>` listeners can observe the event when
            /// `target` is `EventTarget::This`.
            #[prop(default(true))]
            pub bubbles: bool,
            /// Whether the custom event is cancelable.
            ///
            /// Defaults to `false`.
            #[prop(default(false))]
            pub cancelable: bool,
            /// Whether the custom event crosses shadow DOM boundaries.
            ///
            /// Defaults to `false`.
            #[prop(default(false))]
            pub composed: bool,
        }

        impl<'target> $crate::prelude::Render<$crate::prelude::JsSource> for $component<'target> {
            fn render_to(&self, buffer: &mut $crate::prelude::Buffer<$crate::prelude::JsSource>) {
                $crate::__internal::__render_custom_event_to_js::<()>(
                    buffer,
                    stringify!($event),
                    None,
                    &self.target,
                    self.bubbles,
                    self.cancelable,
                    self.composed,
                );
            }
        }

        impl<'target> $crate::prelude::Render for $component<'target> {
            fn render_to(&self, buffer: &mut $crate::prelude::Buffer<$crate::prelude::Element>) {
                $crate::__internal::__render_custom_event_component(self, buffer);
            }
        }

        $(
            $crate::define_events! { $($rest)* }
        )?
    };
}
