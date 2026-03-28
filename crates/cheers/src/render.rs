use core::{
    fmt::{self, Debug, Formatter, Write},
    marker::PhantomData,
    ptr,
};
use std::{borrow::Cow, rc::Rc, sync::Arc};

use crate::context::{AttributeValue, Context, Element};

/// A raw pre-escaped HTML fragment or attribute value.
///
/// `Raw<T, Element>` is for already-sanitized HTML nodes. [`RawAttribute<T>`] is the same idea in
/// attribute context.
///
/// Most code should prefer [`html!`](crate::prelude::html) and normal [`Render`] implementations.
/// Reach for `Raw` only when you already have trusted, pre-escaped markup and need to insert it
/// without further escaping.
///
/// # Safety
///
/// `Raw` disables Cheers' normal escaping. Passing unsanitized user input here can create XSS
/// vulnerabilities.
///
/// # Example
///
/// ```
/// use cheers::{Raw, prelude::*};
///
/// // XSS SAFETY: this HTML is trusted and already sanitized.
/// let trusted = Raw::dangerously_create("<strong>trusted</strong>");
///
/// assert_eq!(
///     html! { div { (trusted) } }.render().into_inner(),
///     "<div><strong>trusted</strong></div>",
/// );
/// ```
#[derive(Clone, Copy, Default, Eq, Hash)]
pub struct Raw<T: AsRef<str>, C: Context = Element> {
    inner: T,
    context: PhantomData<C>,
}

impl<T: AsRef<str>, C: Context> Raw<T, C> {
    /// Creates a new [`Raw`] from the given string.
    ///
    /// It is recommended to add a `// XSS SAFETY` comment above the usage of
    /// this function to indicate why it is safe to directly use the
    /// contained raw HTML.
    #[inline]
    pub const fn dangerously_create(value: T) -> Self {
        Self {
            inner: value,
            context: PhantomData,
        }
    }

    /// Extracts the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        // SAFETY: `Raw<T, C>` has exactly one non-zero-sized field, which is `inner`.
        unsafe { const_precise_live_drops_hack!(self.inner) }
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub const fn as_inner(&self) -> &T {
        &self.inner
    }

    /// Gets a reference to the inner value as an [`&str`][str].
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_ref()
    }
}

impl<T: AsRef<str>> Raw<T> {
    /// Converts the [`Raw<T>`] into a [`Rendered<T>`].
    #[inline]
    #[must_use]
    pub const fn rendered(self) -> Rendered<T> {
        // SAFETY: `Raw<T>` has exactly one non-zero-sized field, which is `inner`.
        let value = unsafe { const_precise_live_drops_hack!(self.inner) };
        Rendered(value)
    }
}

impl<T: AsRef<str> + PartialEq<U>, C: Context, U: AsRef<str>> PartialEq<Raw<U, C>> for Raw<T, C> {
    #[inline]
    fn eq(&self, other: &Raw<U, C>) -> bool {
        self.inner == other.inner
    }
}

impl<T: AsRef<str>, C: Context> Debug for Raw<T, C> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Raw").field(&self.inner.as_ref()).finish()
    }
}

/// A raw pre-escaped attribute value.
///
/// This is a type alias for [`Raw<T, Attribute>`].
pub type RawAttribute<T> = Raw<T, AttributeValue>;

/// A rendered HTML string.
///
/// This type is returned by [`Render::render`] ([`Rendered<String>`]), as
/// well as [`Raw<T>::rendered`] ([`Rendered<T>`]).
///
/// This type intentionally does **not** implement [`Render`] to discourage
/// anti-patterns such as rendering to a string then embedding that HTML string
/// into another page. To do this, you should use [`RenderExt::memoize`], or
/// use [`Raw`] directly.
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// let rendered = html! { p { "Hello" } }.render();
///
/// assert_eq!(rendered.as_inner(), "<p>Hello</p>");
/// ```
#[derive(Debug, Clone, Copy, Default, Eq, Hash)]
pub struct Rendered<T>(T);

impl<T> Rendered<T> {
    /// Extracts the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        // SAFETY: `Rendered<T>` has only one field, which is `0`.
        unsafe { const_precise_live_drops_hack!(self.0) }
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub const fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T: PartialEq<U>, U> PartialEq<Rendered<U>> for Rendered<T> {
    #[inline]
    fn eq(&self, other: &Rendered<U>) -> bool {
        self.0 == other.0
    }
}

/// Workaround for [`const_precise_live_drops`](https://github.com/rust-lang/rust/issues/73255) being unstable.
///
/// # Safety
///
/// - `$self` must be a struct with exactly 1 non-zero-sized field
/// - `$field` must be the name/index of that field
macro_rules! const_precise_live_drops_hack {
    ($self:ident. $field:tt) => {{
        let this = core::mem::ManuallyDrop::new($self);
        (&raw const (*(&raw const this).cast::<Self>()).$field).read()
    }};
}
pub(crate) use const_precise_live_drops_hack;

/// The buffer used for rendering HTML.
///
/// This is a wrapper around [`String`] that prevents accidental XSS
/// vulnerabilities by disallowing direct rendering of raw HTML into the buffer
/// without clearly indicating the risk of doing so.
#[derive(Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Buffer<C: Context = Element> {
    inner: String,
    context: PhantomData<C>,
}

/// A buffer used for rendering attribute values.
///
/// This is a type alias for [`Buffer<AttributeValue>`].
pub type AttributeBuffer = Buffer<AttributeValue>;

#[expect(
    clippy::missing_const_for_fn,
    reason = "`Buffer` does not make sense in `const` contexts"
)]
impl<C: Context> Buffer<C> {
    /// Creates a new, empty [`Buffer`].
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        // XSS SAFETY: The buffer is empty and does not contain any HTML.
        Self::dangerously_from_string(String::new())
    }

    /// Creates a new [`Buffer`] from the given [`String`].
    ///
    /// It is recommended to add a `// XSS SAFETY` comment above the usage of
    /// this function to indicate why the original string is safe to be used as
    /// raw HTML.
    #[inline]
    #[must_use]
    pub fn dangerously_from_string(string: String) -> Self {
        Self {
            inner: string,
            context: PhantomData,
        }
    }

    /// Creates a new [`&mut Buffer`](Buffer) from the given [`&mut
    /// String`](String).
    ///
    /// It is recommended to add a `// XSS SAFETY` comment above the usage of
    /// this function to indicate why the original string is safe to be used as
    /// raw HTML.
    #[inline]
    #[must_use]
    pub fn dangerously_from_string_mut(string: &mut String) -> &mut Self {
        // SAFETY:
        // - `Buffer<C>` is a `#[repr(transparent)]` wrapper around `String`, differing
        //   only in the zero-sized `PhantomData` marker type.
        // - `PhantomData` does not affect memory layout, so the layout of `Buffer<C>`
        //   and `String` is guaranteed to be identical by Rust's type system.
        // - The lifetime of the reference is preserved, and there are no aliasing or
        //   validity issues, as both types are functionally identical at runtime.
        unsafe { &mut *ptr::from_mut(string).cast::<Self>() }
    }

    /// Converts this into an [`&mut AttributeBuffer`](AttributeBuffer).
    #[inline]
    pub fn as_attribute_buffer(&mut self) -> &mut AttributeBuffer {
        // SAFETY:
        // - Both `Buffer<C>` and `AttributeBuffer` are `#[repr(transparent)]` wrappers
        //   around `String`, differing only in the zero-sized `PhantomData` marker
        //   type.
        // - `PhantomData` does not affect memory layout, so the layout of `Buffer<C>`
        //   and `AttributeBuffer` is guaranteed to be identical by Rust's type system.
        // - This cast only changes the marker type and does not affect the actual data
        //   or its validity.
        // - The lifetime of the reference is preserved, and there are no aliasing or
        //   validity issues, as both types are functionally identical at runtime.
        unsafe { &mut *ptr::from_mut(self).cast::<AttributeBuffer>() }
    }

    /// Renders the buffer to a [`Rendered<String>`].
    #[inline]
    #[must_use]
    pub fn rendered(self) -> Rendered<String> {
        Rendered(self.inner)
    }
}

#[expect(
    clippy::missing_const_for_fn,
    reason = "`Buffer` does not make sense in `const` contexts"
)]
impl<C: Context> Buffer<C> {
    /// Gets a mutable reference to the inner [`String`].
    ///
    /// For [`Buffer<Node>`] (a.k.a. [`Buffer`]) writes, the caller must push
    /// complete HTML nodes. If rendering string-like types, the pushed contents
    /// must escape `&` to `&amp;`, `<` to `&lt;`, and `>` to `&gt;`.
    ///
    /// For [`Buffer<AttributeValue>`] (a.k.a. [`AttributeBuffer`]) writes, the
    /// caller must push attribute values which will eventually be surrounded by
    /// double quotes. The pushed contents must escape `&` to `&amp;`, `<` to
    /// `&lt;`, `>` to `&gt;`, and `"` to `&quot;`.
    ///
    /// It is recommended to add a `// XSS SAFETY` comment above the usage of
    /// this method to indicate why it is safe to directly write to the
    /// underlying buffer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cheers::prelude::*;
    ///
    /// fn get_some_html() -> String {
    ///     // get html from some source, such as a CMS
    ///     "<h2>Some HTML from the CMS</h2>".into()
    /// }
    ///
    /// let mut buffer = Buffer::new();
    ///
    /// html! {
    ///     h1 { "My Document!" }
    /// }
    /// .render_to(&mut buffer);
    ///
    /// // XSS SAFETY: The CMS sanitizes the HTML before returning it.
    /// buffer.dangerously_get_string().push_str(&get_some_html());
    ///
    /// assert_eq!(
    ///     buffer.rendered().as_inner(),
    ///     "<h1>My Document!</h1><h2>Some HTML from the CMS</h2>"
    /// )
    /// ```
    #[inline]
    pub fn dangerously_get_string(&mut self) -> &mut String {
        &mut self.inner
    }
}

impl Debug for Buffer {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Buffer").field(&self.inner).finish()
    }
}

/// A type that can be rendered by Cheers.
///
/// This is the core trait behind components. A type becomes usable as a component in `html!` by
/// implementing `Render`. `#[derive(Cheers)]` does not implement this trait; it only generates
/// helper APIs such as ids, signals, and form names.
///
/// For [`Render<Node>`] (a.k.a. [`Render`]) implementations, this
/// must render complete HTML nodes. If rendering string-like types, the
/// implementation must escape `&` to `&amp;`, `<` to `&lt;`, and `>` to `&gt;`.
///
/// For [`Render<AttributeValue>`] implementations, this must render an
/// attribute value which will eventually be surrounded by double quotes. The
/// implementation must escape `&` to `&amp;`, `<` to `&lt;`, `>` to `&gt;`, and
/// `"` to `&quot;`.
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// pub struct Person {
///     name: String,
///     age: u8,
/// }
///
/// impl Render for Person {
///     fn render_to(&self, buffer: &mut Buffer) {
///         html! {
///             div {
///                 h1 { (self.name) }
///                 p { "Age: " (self.age) }
///             }
///         }
///         .render_to(buffer);
///     }
/// }
///
/// let person = Person {
///     name: "Alice".into(),
///     age: 20,
/// };
///
/// assert_eq!(
///     html! { main { (person) } }.render().as_inner(),
///     r#"<main><div><h1>Alice</h1><p>Age: 20</p></div></main>"#,
/// );
/// ```
pub trait Render<C: Context = Element> {
    /// Renders this value to the buffer.
    fn render_to(&self, buffer: &mut Buffer<C>);

    /// Renders this value to a string. This is a convenience method that
    /// calls [`render_to`] into a new [`Buffer`] and returns the result.
    ///
    /// This is useful for tests, debugging, and one-off rendering. For composition inside other
    /// markup, prefer rendering the value directly rather than round-tripping through a string.
    ///
    /// If overridden for performance reasons, this must match the
    /// implementation of [`render_to`].
    ///
    /// [`render_to`]: Render::render_to
    #[inline]
    fn render(&self) -> Rendered<String>
    where
        Self: Render<C>,
    {
        let mut buffer = Buffer::<C>::new();
        self.render_to(&mut buffer);
        buffer.rendered()
    }
}

/// Convenience methods for [`Render`] types.
///
/// This trait currently provides [`memoize`](RenderExt::memoize), which pre-renders a value into
/// reusable [`Raw`] HTML.
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// let cached = html! { span { "cached" } }.memoize();
/// let rendered = html! { div { (&cached) (&cached) } }.render().into_inner();
///
/// assert_eq!(
///     rendered,
///     "<div><span>cached</span><span>cached</span></div>"
/// );
/// ```
pub trait RenderExt: Render {
    /// Pre-renders the value and stores it in a [`Raw`] so that it can be
    /// re-used among multiple renderings without re-computing the value.
    ///
    /// This should generally be avoided to avoid unnecessary allocations, but
    /// may be useful if it is more expensive to compute and render the value.
    #[inline]
    fn memoize(&self) -> Raw<String> {
        // XSS SAFETY: The value has already been rendered and is assumed as safe.
        Raw::dangerously_create(self.render().into_inner())
    }
}

impl<T: Render> RenderExt for T {}

/// A value lazily rendered via a closure.
///
/// For [`Lazy<F, Node>`] (a.k.a. [`Lazy<F>`]), this must render complete
/// HTML nodes. If rendering string-like types, the closure must escape `&` to
/// `&amp;`, `<` to `&lt;`, and `>` to `&gt;`.
///
/// For [`Lazy<F, AttributeValue>`] (a.k.a. [`LazyAttribute<F>`]), this must
/// render an attribute value which will eventually be surrounded by double
/// quotes. The closure must escape `&` to `&amp;`, `<` to `&lt;`, `>` to
/// `&gt;`, and `"` to `&quot;`.
#[derive(Clone, Copy)]
#[must_use = "`Lazy` does nothing unless `.render()` or `.render_to()` is called"]
pub struct Lazy<F: Fn(&mut Buffer<C>), C: Context = Element> {
    f: F,
    context: PhantomData<C>,
}

/// An attribute value lazily rendered via a closure.
///
/// This is a type alias for [`Lazy<F, AttributeValue>`].
pub type LazyAttribute<F> = Lazy<F, AttributeValue>;

impl<F: Fn(&mut Buffer<C>), C: Context> Lazy<F, C> {
    /// Creates a new [`Lazy`] from the given closure.
    ///
    /// It is recommended to add a `// XSS SAFETY` comment above the usage of
    /// this function to indicate why it is safe to assume that the closure will
    /// not write possibly unsafe HTML to the buffer.
    #[inline]
    pub const fn dangerously_create(f: F) -> Self {
        Self {
            f,
            context: PhantomData,
        }
    }

    /// Extracts the inner closure.
    #[inline]
    pub const fn into_inner(self) -> F {
        // SAFETY: `Lazy<F, C>` has exactly one non-zero-sized field, which is `f`.
        unsafe { const_precise_live_drops_hack!(self.f) }
    }

    /// Gets a reference to the inner closure.
    #[inline]
    pub const fn as_inner(&self) -> &F {
        &self.f
    }
}

impl<F: Fn(&mut Buffer<C>), C: Context> Render<C> for Lazy<F, C> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        (self.f)(buffer);
    }
}

impl<F: Fn(&mut Buffer<C>), C: Context> Debug for Lazy<F, C> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Lazy").finish_non_exhaustive()
    }
}

impl<T: AsRef<str>, C: Context> Render<C> for Raw<T, C> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        // XSS SAFETY: `Raw` values are expected to be pre-escaped for
        // their respective rendering context.
        buffer.dangerously_get_string().push_str(self.as_str());
    }

    #[inline]
    fn render(&self) -> Rendered<String> {
        Rendered(self.as_str().into())
    }
}

impl Render for fmt::Arguments<'_> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        struct ElementEscaper<'a>(&'a mut String);

        impl Write for ElementEscaper<'_> {
            #[inline]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                html_escape::encode_text_to_string(s, self.0);
                Ok(())
            }
        }

        // XSS SAFETY: `ElementEscaper` will escape special characters.
        _ = ElementEscaper(buffer.dangerously_get_string()).write_fmt(*self);
    }
}

impl Render<AttributeValue> for fmt::Arguments<'_> {
    #[inline]
    fn render_to(&self, buffer: &mut AttributeBuffer) {
        struct AttributeEscaper<'a>(&'a mut String);

        impl Write for AttributeEscaper<'_> {
            #[inline]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                html_escape::encode_double_quoted_attribute_to_string(s, self.0);
                Ok(())
            }
        }

        // XSS SAFETY: `AttributeEscaper` will escape special characters.
        _ = AttributeEscaper(buffer.dangerously_get_string()).write_fmt(*self);
    }
}

impl Render for char {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        // XSS SAFETY: manual escaping
        let s = buffer.dangerously_get_string();
        match *self {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            c => s.push(c),
        }
    }

    #[inline]
    fn render(&self) -> Rendered<String> {
        Rendered(match *self {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            c => c.into(),
        })
    }
}

impl Render<AttributeValue> for char {
    #[inline]
    fn render_to(&self, buffer: &mut AttributeBuffer) {
        // XSS SAFETY: we are manually performing escaping here
        let s = buffer.dangerously_get_string();

        match *self {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            c => s.push(c),
        }
    }
}

impl Render for str {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        // XSS SAFETY: we use `html_escape` to ensure the text is properly escaped
        html_escape::encode_text_to_string(self, buffer.dangerously_get_string());
    }

    #[inline]
    fn render(&self) -> Rendered<String> {
        Rendered(html_escape::encode_text(self).into_owned())
    }
}

impl Render<AttributeValue> for str {
    #[inline]
    fn render_to(&self, buffer: &mut AttributeBuffer) {
        // XSS SAFETY: we use `html_escape` to ensure the text is properly escaped
        html_escape::encode_double_quoted_attribute_to_string(
            self,
            buffer.dangerously_get_string(),
        );
    }
}

impl Render for String {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        self.as_str().render_to(buffer);
    }

    #[inline]
    fn render(&self) -> Rendered<String> {
        Render::<Element>::render(self.as_str())
    }
}

impl Render<AttributeValue> for String {
    #[inline]
    fn render_to(&self, buffer: &mut AttributeBuffer) {
        self.as_str().render_to(buffer);
    }
}

impl<C: Context> Render<C> for bool {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        // XSS SAFETY: "true" and "false" are safe strings
        buffer
            .dangerously_get_string()
            .push_str(if *self { "true" } else { "false" });
    }

    #[inline]
    fn render(&self) -> Rendered<String> {
        Rendered(if *self { "true" } else { "false" }.into())
    }
}

macro_rules! render_via_itoa {
    ($($Ty:ty)*) => {
        $(
            impl<C: Context> Render<C> for $Ty {
                #[inline]
                fn render_to(&self, buffer: &mut Buffer<C>) {
                    // XSS SAFETY: integers are safe
                    buffer.dangerously_get_string().push_str(itoa::Buffer::new().format(*self));
                }

                #[inline]
                fn render(&self) -> Rendered<String> {
                    Rendered(itoa::Buffer::new().format(*self).into())
                }
            }
        )*
    };
}

render_via_itoa! {
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
}

macro_rules! render_via_zmij {
    ($($Ty:ty)*) => {
        $(
            impl<C: Context> Render<C> for $Ty {
                #[inline]
                fn render_to(&self, buffer: &mut Buffer<C>) {
                    // XSS SAFETY: floats are safe
                    buffer.dangerously_get_string().push_str(zmij::Buffer::new().format(*self));
                }

                #[inline]
                fn render(&self) -> Rendered<String> {
                    Rendered(zmij::Buffer::new().format(*self).into())
                }
            }
        )*
    };
}

render_via_zmij! {
    f32 f64
}

macro_rules! render_via_deref {
    ($($Ty:ty)*) => {
        $(
            impl<T: Render + ?Sized> Render for $Ty {
                #[inline]
                fn render_to(&self, buffer: &mut Buffer) {
                    T::render_to(&**self, buffer);
                }

                #[inline]
                fn render(&self) -> Rendered<String> {
                    T::render(&**self)
                }
            }

            impl<T: Render<AttributeValue> + ?Sized> Render<AttributeValue> for $Ty {
                #[inline]
                fn render_to(&self, buffer: &mut AttributeBuffer) {
                    T::render_to(&**self, buffer);
                }
            }
        )*
    };
}

render_via_deref! {
    &T
    &mut T
    Box<T>
    Rc<T>
    Arc<T>
}

impl<'a, B: 'a + Render + ToOwned + ?Sized> Render for Cow<'a, B> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        B::render_to(&**self, buffer);
    }

    #[inline]
    fn render(&self) -> Rendered<String> {
        B::render(&**self)
    }
}

impl<'a, B: 'a + Render<AttributeValue> + ToOwned + ?Sized> Render<AttributeValue> for Cow<'a, B> {
    #[inline]
    fn render_to(&self, buffer: &mut AttributeBuffer) {
        B::render_to(&**self, buffer);
    }
}

impl<T: Render> Render for [T] {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        for item in self {
            item.render_to(buffer);
        }
    }
}

impl<T: Render, const N: usize> Render for [T; N] {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        self.as_slice().render_to(buffer);
    }
}

impl<T: Render> Render for Vec<T> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer) {
        self.as_slice().render_to(buffer);
    }
}

impl<T: Render<C>, C: Context> Render<C> for Option<T> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        if let Some(value) = self {
            value.render_to(buffer);
        }
    }
}

impl<T: Render<C>, E: Render<C>, C: Context> Render<C> for Result<T, E> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        match self {
            Ok(value) => value.render_to(buffer),
            Err(err) => err.render_to(buffer),
        }
    }
}

macro_rules! impl_tuple {
    () => {
        impl<C: Context> Render<C> for () {
            #[inline]
            fn render_to(&self, _: &mut Buffer<C>) {}
        }
    };
    (($i:tt $T:ident)) => {
        #[cfg_attr(docsrs, doc(fake_variadic))]
        #[cfg_attr(docsrs, doc = "This trait is implemented for tuples up to twelve items long.")]
        impl<$T: Render<C>, C: Context> Render<C> for ($T,) {
            #[inline]
            fn render_to(&self, buffer: &mut Buffer<C>) {
                self.$i.render_to(buffer);
            }
        }
    };
    (($i0:tt $T0:ident) $(($i:tt $T:ident))+) => {
        #[cfg_attr(docsrs, doc(hidden))]
        impl<$T0: Render<C>, $($T: Render<C>),*, C: Context> Render<C> for ($T0, $($T,)*) {
            #[inline]
            fn render_to(&self, buffer: &mut Buffer<C>) {
                self.$i0.render_to(buffer);
                $(self.$i.render_to(buffer);)*
            }
        }
    }
}

impl_tuple!();
impl_tuple!((0 T));
impl_tuple!((0 T0) (1 T1));
impl_tuple!((0 T0) (1 T1) (2 T2));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5) (6 T6));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5) (6 T6) (7 T7));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5) (6 T6) (7 T7) (8 T8));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5) (6 T6) (7 T7) (8 T8) (9 T9));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5) (6 T6) (7 T7) (8 T8) (9 T9) (10 T10));
impl_tuple!((0 T0) (1 T1) (2 T2) (3 T3) (4 T4) (5 T5) (6 T6) (7 T7) (8 T8) (9 T9) (10 T10) (11 T11));
