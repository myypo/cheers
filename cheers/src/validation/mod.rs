//! Types and traits used for validation of HTML elements and attributes.
pub mod attributes;
pub mod elements;
#[cfg(feature = "mathml")]
mod mathml;

// Re-export attribute namespace modules at the validation level
pub use attributes::*;

/// A marker trait for type-checked elements.
pub trait Element {
    /// The kind of this element.
    type Kind: ElementKind;
}
/// A marker trait to represent the kind of an element.
///
/// This can be either [`Normal`] or [`Void`]. A [`Normal`] element will always
/// have a closing tag, and can have children. A [`Void`] element will never
/// have a closing tag, and cannot have children.
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

mod sealed {
    use super::{Normal, Void};

    pub trait Sealed {}
    impl Sealed for Normal {}
    impl Sealed for Void {}
}

/// A standard attribute.
#[derive(Debug, Clone, Copy)]
pub struct Attribute;

/// An attribute namespace.
#[derive(Debug, Clone, Copy)]
pub struct AttributeNamespace;

/// An attribute prefixed by a symbol.
#[derive(Debug, Clone, Copy)]
pub struct AttributeSymbol;
