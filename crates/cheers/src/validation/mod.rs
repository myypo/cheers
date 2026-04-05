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
