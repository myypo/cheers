//! The [`Context`] trait and its implementors.

/// A marker trait to represent the context that the value is being rendered to.
///
/// This can be [`Element`], [`AttributeValue`], or [`JsSource`]. [`Element`]
/// represents an HTML node, [`AttributeValue`] represents an attribute value
/// which will eventually be surrounded by double quotes, and [`JsSource`] represents
/// JavaScript source embedded inside a Datastar attribute value.
///
/// This is used to ensure that the correct rendering methods are called
/// for each context, and to prevent errors such as accidentally rendering
/// an HTML element into an attribute value.
pub trait Context: sealed::Sealed {}

/// A marker type to represent a complete element node.
///
/// All types and traits that are generic over the `Context` trait use
/// [`Element`] as the default for the generic type parameter.
///
/// Traits and types with this marker type expect complete HTML nodes. If
/// rendering string-like types, the value/implementation must escape `&` to
/// `&amp;`, `<` to `&lt;`, and `>` to `&gt;`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Element;

impl Context for Element {}

/// A marker type to represent an attribute value.
///
/// Traits and types with this marker type expect an attribute value which will
/// eventually be surrounded by double quotes. The value/implementation must
/// escape `&` to `&amp;`, `<` to `&lt;`, `>` to `&gt;`, and `"` to `&quot;`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AttributeValue;

impl Context for AttributeValue {}

/// A marker type to represent a JavaScript expression or value inside a
/// Datastar attribute (e.g. `data-signals`, `data-class`, `data-on:click`).
///
/// Values rendered with this context are ultimately embedded inside a
/// double-quoted HTML attribute, so implementations must ensure the output
/// is valid JavaScript source and is also safe for HTML attribute parsing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JsSource;

impl Context for JsSource {}

mod sealed {
    use super::{AttributeValue, Element, JsSource};

    pub trait Sealed {}
    impl Sealed for Element {}
    impl Sealed for AttributeValue {}
    impl Sealed for JsSource {}
}
