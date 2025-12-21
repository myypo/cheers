use std::{
    borrow::Cow,
    fmt::{Display, Write},
    marker::PhantomData,
};

use crate::{
    context::AttributeValue,
    render::{Buffer, Lazy, Render},
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

        html_escape::encode_double_quoted_attribute_to_string(s, buffer.dangerously_get_string());
    }
}

#[derive(Debug)]
pub struct Signal<T> {
    path: Path,
    ty: PhantomData<T>,
}

impl<T> Signal<T> {
    #[doc(hidden)]
    pub fn __string(s: String) -> Self {
        Signal {
            path: Path(s),
            ty: PhantomData::<T>,
        }
    }

    #[doc(hidden)]
    pub fn __path(&self) -> &str {
        &self.path.0
    }

    pub fn scoped(s: &'static str) -> Self {
        Self {
            path: Path(s.to_owned()),
            ty: PhantomData::<T>,
        }
    }

    #[doc(hidden)]
    pub fn __computed_open(&self, buffer: &mut Buffer<AttributeValue>) -> usize {
        let segments: Vec<&str> = self.path.0.split('.').collect();

        if segments.is_empty() {
            return 0;
        }

        let buf = buffer.dangerously_get_string();
        for (i, segment) in segments.iter().enumerate() {
            if i == 0 {
                buf.push_str(segment);
            } else {
                buf.push_str(":{");
                buf.push_str(segment);
            }
        }
        buf.push_str(":()=>");

        segments.len() - 1
    }
}

impl Signal<()> {
    #[doc(hidden)]
    pub fn __computed_close(count: usize, buffer: &mut Buffer<AttributeValue>) {
        let buf = buffer.dangerously_get_string();
        for _ in 0..count {
            buf.push('}');
        }
    }
}

impl<T: Render<AttributeValue>> Signal<T> {
    #[doc(hidden)]
    pub fn __assign(&self, buffer: &mut Buffer<AttributeValue>, v: T) {
        let segments: Vec<&str> = self.path.0.split('.').collect();

        if segments.is_empty() {
            return;
        }

        {
            let s = buffer.dangerously_get_string();
            let mut first = true;
            for seg in segments.iter() {
                if first {
                    first = false;
                    s.push_str(seg);
                } else {
                    s.push(':');
                    s.push('{');
                    s.push_str(seg);
                }
            }
            s.push(':');
        }

        v.render_to(buffer);

        let s = buffer.dangerously_get_string();
        for _ in 0..segments.len() - 1 {
            s.push('}');
        }
    }
}

impl<T> AsRef<Path> for Signal<T> {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl<T> Render<AttributeValue> for Signal<T> {
    fn render_to(&self, buffer: &mut Buffer<AttributeValue>) {
        buffer.dangerously_get_string().push('$');
        // FIXME: can I make this safe?
        buffer.dangerously_get_string().push_str(&self.path.0);
    }
}

// TODO: better name?
#[derive(Debug)]
pub struct Path(String);

impl Path {
    #[doc(hidden)]
    pub fn __string(&self) -> String {
        // TODO: use Cow?
        self.0.clone()
    }

    #[doc(hidden)]
    pub fn __empty() -> Self {
        Self(String::new())
    }
}

pub trait Component {
    fn component(&self) -> impl Render;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_object_string_value() {
        let signal = Signal::<&str>::__string("user.name".to_string());
        let mut buffer = Buffer::new();
        signal.__assign(&mut buffer, "'Nick'");
        assert_eq!(buffer.dangerously_get_string(), r#"user:{name:'Nick'}"#);
    }

    #[test]
    fn signal_object_number_value() {
        let signal = Signal::<f64>::__string("user.age".to_string());
        let mut buffer = Buffer::new();
        signal.__assign(&mut buffer, -42.0);
        assert_eq!(buffer.dangerously_get_string(), r#"user:{age:-42.0}"#);
    }
}
