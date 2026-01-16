use std::fmt::{Debug, Display};

use crate::{
    context::Context,
    render::{Buffer, Render},
    router::css_url,
};

pub struct Doctype;

impl Render for Doctype {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        // XSS SAFETY: static doctype string
        buffer.dangerously_get_string().push_str("<!DOCTYPE html>");
    }
}

pub struct Scripts;

impl Render for Scripts {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        // XSS SAFETY: static HTML streaming script
        buffer.dangerously_get_string().push_str("<script>function __ssrStream(key){const t=document.querySelector(`[data-ssr='${key}-t']`),s=document.querySelector(`[data-ssr='${key}']`);s.replaceWith(t.content);t.remove();document.querySelector(`[data-ssr='${key}-s']`).remove()}</script>");
        // XSS SAFETY: static inclusion of datastar
        buffer
            .dangerously_get_string()
            .push_str(r#"<script src="/cheers/assets/datastar.js"></script>"#);
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

pub struct Css;

impl Render for Css {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        let link = format!(r#"<link rel="stylesheet" href="/cheers{}">"#, css_url());
        // XSS SAFETY: CSS URL is computed by us
        buffer.dangerously_get_string().push_str(&link);
    }
}

/// A value rendered via its [`Display`] implementation.
///
/// This will handle escaping special characters for you.
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
