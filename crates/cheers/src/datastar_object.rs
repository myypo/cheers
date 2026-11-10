use std::fmt::{self, Debug, Formatter};

use crate::{
    context::DatastarSource,
    render::{Buffer, Render},
};

/// A JavaScript object built by [`datastar_object!`](crate::macros::datastar_object).
///
/// `!attr(..)`, `!style(..)` and `datastar_object!` spread it with a `..value` entry. Only this
/// type can be spread, since spreading anything other than an object produces broken attributes.
#[derive(Clone, Copy)]
#[must_use = "a `DatastarObject` does nothing unless spread into an attribute or rendered"]
pub struct DatastarObject<R>(R);

impl<R: Render<DatastarSource>> DatastarObject<R> {
    #[doc(hidden)]
    /// Used by `datastar_object!`. `source` must render a JavaScript object literal.
    pub const fn __new(source: R) -> Self {
        Self(source)
    }
}

impl<R: Render<DatastarSource>> Render<DatastarSource> for DatastarObject<R> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<DatastarSource>) {
        self.0.render_to(buffer);
    }
}

impl<R> Debug for DatastarObject<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DatastarObject").finish_non_exhaustive()
    }
}

/// A value that a `..value` entry can spread: it renders a JavaScript object.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be spread with `..`",
    label = "not a `datastar_object!` value",
    note = "build the entries with `datastar_object! {{ .. }}` and spread that"
)]
pub trait Spread: sealed::Sealed + Render<DatastarSource> {}

impl<R: Render<DatastarSource>> Spread for DatastarObject<R> {}
impl<T: Spread + ?Sized> Spread for &T {}

mod sealed {
    pub trait Sealed {}

    impl<R> Sealed for super::DatastarObject<R> {}
    impl<T: Sealed + ?Sized> Sealed for &T {}
}

#[doc(hidden)]
pub fn __spread<T: Spread + ?Sized>(value: &T) -> &T {
    value
}
