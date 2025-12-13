use std::fmt::Display;

use crate::{
    context::AttributeValue,
    render::{Buffer, Render},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InnerElementId {
    Static(&'static str),
    Dynamic(String),
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ElementId(pub(crate) InnerElementId);

impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            InnerElementId::Static(s) => f.write_str(s),
            InnerElementId::Dynamic(s) => f.write_str(s),
        }
    }
}

impl AsRef<ElementId> for ElementId {
    fn as_ref(&self) -> &ElementId {
        self
    }
}

impl ElementId {
    #[doc(hidden)]
    pub fn __static(s: &'static str) -> Self {
        Self(InnerElementId::Static(s))
    }

    #[doc(hidden)]
    pub fn __dynamic(s: String) -> Self {
        Self(InnerElementId::Dynamic(s))
    }
}

impl Render<AttributeValue> for ElementId {
    fn render_to(&self, buffer: &mut Buffer<AttributeValue>) {
        let s = match &self.0 {
            InnerElementId::Static(s) => s,
            InnerElementId::Dynamic(s) => s.as_str(),
        };

        html_escape::encode_unquoted_attribute_to_string(s, buffer.dangerously_get_string());
    }
}

#[derive(Debug)]
pub struct Signal(String);

impl Signal {
    #[doc(hidden)]
    pub fn __string(s: String) -> Self {
        Self(s)
    }

    pub fn path(&self) -> &str {
        &self.0
    }
}
