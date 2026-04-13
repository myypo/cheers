use std::fmt::{Debug, Display};

use macros::Cheers;

use crate::{
    context::{AttributeValue, Context, Element},
    render::{Buffer, Render},
    router::{css_url, js_url, svg_sprite_url},
};

/// Renders `<!DOCTYPE html>`.
///
/// This is the first item in a full HTML document response.
///
/// # Example
///
/// ```
/// use cheers::{components::Doctype, prelude::*};
///
/// let page = html! {
///     (Doctype)
///     html {
///         body { "Hello" }
///     }
/// };
///
/// assert_eq!(
///     page.render().into_inner(),
///     "<!DOCTYPE html><html><body>Hello</body></html>"
/// );
/// ```
#[derive(Cheers)]
pub struct Doctype;

impl Render for Doctype {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        // XSS SAFETY: static doctype string
        buffer.dangerously_get_string().push_str("<!DOCTYPE html>");
    }
}

/// Renders the core Cheers client-side runtime scripts.
///
/// This includes the streaming helper script and the `datastar.js` runtime. When the router was
/// built with a [`crate::track::TrackConfig`], the served runtime also includes the tracking
/// plugin. In debug builds it also includes the live-reload script.
///
/// Include this in pages that use Cheers client-side behaviors such as actions, signals and patching
///
/// # Example
///
/// ```
/// use cheers::{components::Scripts, prelude::*};
///
/// let rendered = html! {
///     Scripts ();
/// }
/// .render()
/// .into_inner();
///
/// assert!(rendered.contains("/cheers/assets/"));
/// ```
#[derive(Cheers, Default)]
pub struct Scripts;

impl Render for Scripts {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        // XSS SAFETY: static HTML streaming script
        buffer.dangerously_get_string().push_str("<script>function __ssrStream(key){const t=document.querySelector(`[data-ssr='${key}-t']`),s=document.querySelector(`[data-ssr='${key}']`);s.replaceWith(t.content);t.remove();document.querySelector(`[data-ssr='${key}-s']`).remove()}</script>");
        let script = format!(r#"<script src="{}"></script>"#, js_url());
        // XSS SAFETY: JS URL is computed by us
        buffer.dangerously_get_string().push_str(&script);
        if cfg!(debug_assertions) {
            // XSS SAFETY: static reload script
            buffer.dangerously_get_string().push_str(
                r#"
<script>
(function() {
  let attempts = 0;

  function connect() {
    const source = new EventSource("/cheers/live-reload");

    source.onopen = function() {
      if (attempts !== 0) {
          window.location.reload();
      }
      console.log("Cheers live-reload connected");
      attempts = 0;
    };

    source.onmessage = function(event) {
      if (event.data === "reload") {
        console.log("Cheers reload event received, reloading page...");
        window.location.reload();
      }
    };

    source.onerror = function(err) {
      console.error("Cheers live-reload connection error:", err);

      const delay = Math.min(50 * Math.pow(1.25, attempts), 1000);
      source.close();
      attempts++;
      setTimeout(() => {
        connect();
      }, delay);
    };
  }

  connect();
})();
</script>
                "#,
            );
        }
    }
}

/// Renders the `<link rel="stylesheet">` tag for the Cheers CSS bundle.
///
/// This links to the framework-managed stylesheet path produced by the router.
#[derive(Cheers)]
pub struct CssStylesheet;

impl Render for CssStylesheet {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        let link = format!(r#"<link rel="stylesheet" href="{}">"#, css_url());
        // XSS SAFETY: CSS URL is computed by us
        buffer.dangerously_get_string().push_str(&link);
    }
}

/// A reference to a symbol inside the global Cheers SVG sprite sheet.
///
/// In attribute context, this renders the `href` value for a `<use>` element. In element context,
/// it renders a minimal `<svg><use ...></use></svg>` wrapper.
///
/// Register the sprite sheet first with
/// [`include_svg_sprite!`](crate::include_svg_sprite).
///
/// # Example
///
/// ```
/// use cheers::{components::SvgSymbol, prelude::*};
///
/// include_svg_sprite! {
///     svg viewBox="0 0 16 16" {
///         symbol id="icon-check" viewBox="0 0 16 16" {
///             path d="M6.5 11.2 3.3 8l-1.1 1.1 4.3 4.3L14 5.9l-1.1-1.1z";
///         }
///     }
/// }
///
/// let rendered = html! {
///     svg {
///         use href=(SvgSymbol("icon-check"));
///     }
/// }
/// .render()
/// .into_inner();
///
/// assert!(rendered.contains("#icon-check"));
///
/// let rendered = html! {
///     (SvgSymbol("icon-check"))
/// }
/// .render()
/// .into_inner();
///
/// assert!(rendered.starts_with("<svg><use href=\"/cheers/assets/"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SvgSymbol<T: Display>(pub T);

impl<T: Display> Render<AttributeValue> for SvgSymbol<T>
where
    for<'a> std::fmt::Arguments<'a>: Render<AttributeValue>,
{
    fn render_to(&self, buffer: &mut Buffer<AttributeValue>) {
        format_args!("{}#{}", svg_sprite_url(), self.0).render_to(buffer);
    }
}

impl<T: Display> Render<Element> for SvgSymbol<T>
where
    for<'a> std::fmt::Arguments<'a>: Render<AttributeValue>,
{
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        // XSS SAFETY: sprite URL is computed by us and symbol id is escaped in attribute context.
        buffer
            .dangerously_get_string()
            .push_str("<svg><use href=\"");
        format_args!("{}#{}", svg_sprite_url(), self.0).render_to(buffer.as_attribute_buffer());
        // XSS SAFETY: static SVG wrapper
        buffer.dangerously_get_string().push_str("\"></use></svg>");
    }
}

/// A value rendered via its [`Display`] implementation.
///
/// This will handle escaping special characters for you.
///
/// # Example
///
/// ```
/// use cheers::{components::Displayed, prelude::*};
///
/// let rendered = html! { p { (Displayed("<hello>")) } }.render().into_inner();
///
/// assert_eq!(rendered, "<p>&lt;hello&gt;</p>");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Displayed<T: Display>(pub T);

impl<C: Context, T: Display> Render<C> for Displayed<T>
where
    for<'a> std::fmt::Arguments<'a>: Render<C>,
{
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        format_args!("{}", self.0).render_to(buffer);
    }
}

/// A value rendered via its [`Debug`] implementation.
///
/// This will handle escaping special characters for you.
///
/// # Example
///
/// ```
/// use cheers::{components::Debugged, prelude::*};
///
/// let rendered = html! { pre { (Debugged(vec![1, 2, 3])) } }
///     .render()
///     .into_inner();
///
/// assert_eq!(rendered, "<pre>[1, 2, 3]</pre>");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Debugged<T: Debug>(pub T);

impl<C: Context, T: Debug> Render<C> for Debugged<T>
where
    for<'a> std::fmt::Arguments<'a>: Render<C>,
{
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        format_args!("{:?}", self.0).render_to(buffer);
    }
}
