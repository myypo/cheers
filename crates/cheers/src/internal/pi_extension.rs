use crate::{context::Element, render::Buffer};

#[inline]
pub fn __push_element_source_hint(buffer: &mut Buffer<Element>, source: &str) {
    crate::pi_extension::push_element_source_hint(buffer, source);
}
