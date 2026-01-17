use std::{fmt::Display, marker::PhantomData};

use crate::{
    context::AttributeValue,
    render::{Buffer, Render},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InnerElementId {
    Static(&'static str),
    Dynamic(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

        s.render_to(buffer);
    }
}

#[derive(Debug)]
pub struct Signal<T> {
    path: String,
    ty: PhantomData<T>,
}

#[macro_export]
macro_rules! scoped_signal {
    ($name:literal) => {
        ::cheers::prelude::Signal::__scoped($name, ::std::file!(), ::std::line!(), ::std::column!())
    };
}

impl<T> Signal<T> {
    #[doc(hidden)]
    pub fn __scoped(name: &'static str, file: &'static str, line: u32, column: u32) -> Self {
        let hash = hash_location(file, line, column);
        // TODO: there should be a way to avoid the alloc
        let path = format!("{name}{hash}");
        Self {
            path,
            ty: PhantomData::<T>,
        }
    }

    #[doc(hidden)]
    pub fn __string(path: String) -> Self {
        Signal {
            path,
            ty: PhantomData::<T>,
        }
    }

    #[doc(hidden)]
    pub fn __path(&self) -> &str {
        &self.path
    }

    #[doc(hidden)]
    pub fn __computed_open(&self, buffer: &mut Buffer<AttributeValue>) -> usize {
        let path = self.__path();
        let segments: Vec<&str> = path.split('.').collect();

        if segments.is_empty() {
            return 0;
        }

        for (i, segment) in segments.iter().enumerate() {
            if i == 0 {
                segment.render_to(buffer);
            } else {
                // XSS SAFETY: statically opening the JS object
                buffer.dangerously_get_string().push_str(":{");
                segment.render_to(buffer);
            }
        }
        // XSS SAFETY: statically assigning a JS function - the execution is intentional
        buffer.dangerously_get_string().push_str(":()=>");

        segments.len() - 1
    }
}

impl Signal<()> {
    #[doc(hidden)]
    pub fn __computed_close(count: usize, buffer: &mut Buffer<AttributeValue>) {
        // XSS SAFETY: statically closing the JS object
        let buf = buffer.dangerously_get_string();
        for _ in 0..count {
            buf.push('}');
        }
    }
}

impl<T: Render<AttributeValue>> Signal<T> {
    #[doc(hidden)]
    pub fn __assign(&self, buffer: &mut Buffer<AttributeValue>, v: T) {
        let path = self.__path();
        let segments: Vec<&str> = path.split('.').collect();

        if segments.is_empty() {
            return;
        }

        {
            let mut first = true;
            for seg in segments.iter() {
                if first {
                    first = false;
                    seg.render_to(buffer);
                } else {
                    // XSS SAFETY: statically opening the JS object
                    buffer.dangerously_get_string().push_str(":{");
                    seg.render_to(buffer);
                }
            }
            // XSS SAFETY: static assignment
            buffer.dangerously_get_string().push(':');
        }

        v.render_to(buffer);

        // XSS SAFETY: statically closing the JS object
        let s = buffer.dangerously_get_string();
        for _ in 0..segments.len() - 1 {
            s.push('}');
        }
    }
}

impl<T> Render<AttributeValue> for Signal<T> {
    fn render_to(&self, buffer: &mut Buffer<AttributeValue>) {
        '$'.render_to(buffer);
        self.__path().render_to(buffer);
    }
}

#[derive(Debug)]
pub struct FormName(&'static str);

impl FormName {
    #[doc(hidden)]
    pub fn __static(s: &'static str) -> Self {
        Self(s)
    }
}

impl Render<AttributeValue> for FormName {
    fn render_to(&self, buffer: &mut Buffer<AttributeValue>) {
        self.0.render_to(buffer);
    }
}

/// Computes 32-bit FNV1a hash from a location
pub(crate) const fn hash_location(file: &'static str, line: u32, column: u32) -> u32 {
    const FNV_OFFSET_BASIS_32: u32 = 0x811c9dc5;
    const FNV_PRIME_32: u32 = 0x01000193;

    let bytes = file.as_bytes();
    let mut hash = FNV_OFFSET_BASIS_32;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(FNV_PRIME_32);
        i += 1;
    }

    let line_bytes = line.to_ne_bytes();
    i = 0;
    while i < 4 {
        hash ^= line_bytes[i] as u32;
        hash = hash.wrapping_mul(FNV_PRIME_32);
        i += 1;
    }

    let column_bytes = column.to_ne_bytes();
    i = 0;
    while i < 4 {
        hash ^= column_bytes[i] as u32;
        hash = hash.wrapping_mul(FNV_PRIME_32);
        i += 1;
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_object_string_value() {
        let signal = Signal::<&str>::__string("user.name".to_string());
        let mut buffer = Buffer::new();
        signal.__assign(&mut buffer, "'Nick'");
        assert_eq!(buffer.rendered().into_inner(), r#"user:{name:'Nick'}"#);
    }

    #[test]
    fn signal_object_number_value() {
        let signal = Signal::<f64>::__string("user.age".to_string());
        let mut buffer = Buffer::new();
        signal.__assign(&mut buffer, -42.0);
        assert_eq!(buffer.rendered().into_inner(), r#"user:{age:-42.0}"#);
    }

    #[test]
    fn hash_different_locations() {
        const HASH1: u32 = hash_location("src/main.rs", 10, 5);
        const HASH2: u32 = hash_location("src/main.rs", 10, 6);

        assert_ne!(HASH1, HASH2);
    }

    #[test]
    fn hash_same_locations() {
        const HASH1: u32 = hash_location("src/main.rs", 42, 13);
        const HASH2: u32 = hash_location("src/main.rs", 42, 13);

        assert_eq!(HASH1, HASH2);
    }
}
