use std::fmt::{Debug, Display};

use macros::Cheers;

use crate::{
    context::{AttributeValue, Context, Element},
    render::{Buffer, Render},
    router::{css_url, js_bundle_url, js_url, svg_sprite_url},
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
/// This includes the `datastar.js` runtime and the SSR streaming helper. When the router was
/// built with a [`crate::track::TrackConfig`], the served runtime also includes the tracking
/// plugin. In debug builds it also includes the WebSocket live-reload script. With the
/// `subsecond` feature enabled, that script morphs rebuilt HTML after Subsecond patches instead of
/// reloading the page.
///
/// Include this in pages that use Cheers client-side behaviors such as actions, signals and patching
///
/// # Example
///
/// ```
/// use cheers::{components::Scripts, prelude::*};
///
/// let rendered = html! {
///     Scripts;
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
        let script = format!(
            r#"<script data-cheers-runtime="datastar" src="{}"></script>"#,
            js_url()
        );
        // XSS SAFETY: JS URL is computed by us
        buffer.dangerously_get_string().push_str(&script);
        if cfg!(debug_assertions) {
            if !crate::subsecond::enabled() {
                buffer.dangerously_get_string().push_str(
                    r#"
<script data-cheers-runtime="live-reload">
(function() {
  let attempts = 0;
  let reconnectTimer = null;

  function connect() {
    const url = new URL("/cheers/live-reload", window.location.href);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(url.href);

    socket.onopen = function() {
      if (attempts !== 0) {
          window.location.reload();
      }
      console.log("Cheers live-reload connected");
      attempts = 0;
    };

    socket.onmessage = function(event) {
      let message = null;
      try { message = JSON.parse(event.data); } catch (_) {}
      if (message && message.kind === "reload") {
        console.log("Cheers reload event received, reloading page...");
        window.location.reload();
      }
    };

    socket.onclose = function() {
      const delay = Math.min(50 * Math.pow(1.25, attempts), 1000);
      attempts++;
      reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        connect();
      }, delay);
    };

    socket.onerror = function(err) {
      console.error("Cheers live-reload WebSocket error:", err);

      if (reconnectTimer === null) {
        socket.close();
      }
    };
  }

  connect();
})();
</script>
                "#,
                );
            } else {
                buffer.dangerously_get_string().push_str(
                    r#"
<script data-cheers-runtime="live-reload">
(function() {
  if (window.__cheersSubsecondLiveReloadStarted) return;
  window.__cheersSubsecondLiveReloadStarted = true;

  let attempts = 0;
  let reconnectTimer = null;
  let morphing = false;

  function delay(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function cacheBustedCurrentUrl() {
    const url = new URL(window.location.href);
    url.hash = "";
    url.searchParams.set("__cheers_subsecond_hot_reload", Date.now().toString(36) + Math.random().toString(36).slice(2));
    return url.toString();
  }

  async function fetchCurrentDocument() {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);

    try {
      const response = await fetch(cacheBustedCurrentUrl(), {
        cache: "no-store",
        headers: { "cache-control": "no-cache" },
        signal: controller.signal
      });
      if (!response.ok) throw new Error("app returned HTTP " + response.status);
      const html = await response.text();
      const doc = new DOMParser().parseFromString(html, "text/html");
      window.__cheersSettleSsrStreams(doc);
      return doc;
    } catch (error) {
      if (error && error.name === "AbortError") {
        const timeoutError = new Error("timed out waiting for rebuilt HTML");
        timeoutError.name = "CheersDocumentFetchTimeout";
        throw timeoutError;
      }
      throw error;
    } finally {
      clearTimeout(timeout);
    }
  }

  function applyDatastarPatch(selector, elements) {
    const target = selector ? document.querySelector(selector) : document.body;
    if (!target) return false;

    try {
      document.dispatchEvent(new CustomEvent("datastar-fetch", {
        detail: {
          type: "datastar-patch-elements",
          el: target,
          argsRaw: { selector, mode: "inner", elements }
        }
      }));
    } catch (_) {
      return false;
    }

    return true;
  }

  function replaceBodyChildrenFallback(doc) {
    const fragment = document.createDocumentFragment();
    for (const node of Array.from(doc.body.childNodes)) {
      fragment.appendChild(document.importNode(node, true));
    }
    document.body.replaceChildren(fragment);
  }

  function patchBody(doc) {
    for (const script of Array.from(doc.querySelectorAll('script[data-cheers-runtime]'))) {
      script.remove();
    }
    if (!applyDatastarPatch("body", doc.body.innerHTML)) {
      replaceBodyChildrenFallback(doc);
    }
  }

  async function patchAsyncIslands() {
    const roots = Array.from(document.querySelectorAll("[data-cheers-async-root]"));
    const keys = Array.from(new Set(
      roots.map((root) => root.getAttribute("data-cheers-async-root")).filter(Boolean)
    ));
    if (!keys.length) return { patched: false, complete: false };

    const response = await fetch("/cheers/async-islands/render", {
      method: "POST",
      cache: "no-store",
      headers: {
        "content-type": "application/json",
        "cache-control": "no-cache"
      },
      body: JSON.stringify({ keys })
    });
    if (!response.ok) return { patched: false, complete: false };

    const payload = await response.json();
    const islands = payload && Array.isArray(payload.islands) ? payload.islands : [];
    if (!islands.length) return { patched: false, complete: false };

    const expectedKeys = new Set(keys);
    const patchedKeys = new Set();
    for (const island of islands) {
      if (!island || !island.key || !expectedKeys.has(island.key)) continue;
      const selector = window.__cheersSsrAttrSelector("data-cheers-async-root", island.key);
      const matchingRoots = Array.from(document.querySelectorAll(selector));
      if (!matchingRoots.length) continue;
      if (!applyDatastarPatch(selector, island.html || "")) {
        for (const root of matchingRoots) {
          root.innerHTML = island.html || "";
        }
      }
      patchedKeys.add(island.key);
    }

    if (patchedKeys.size) {
      console.log("Cheers Subsecond hot reload patched async islands without restarting their streams");
    }
    return {
      patched: patchedKeys.size > 0,
      complete: keys.every((key) => patchedKeys.has(key))
    };
  }

  function bodyContainsOnlyAsyncIslands() {
    const visibleChildren = Array.from(document.body.children).filter((el) => {
      return !el.matches('script[data-cheers-runtime], script[data-ssr$="-s"], template[data-ssr$="-t"]');
    });
    return visibleChildren.length > 0 && visibleChildren.every((el) => {
      return el.hasAttribute("data-cheers-async-root") || !!el.closest("[data-cheers-async-root]");
    });
  }

  async function morphCurrentPage() {
    if (morphing) return;
    morphing = true;
    let lastMessage = "rebuilt output is not visible yet";

    try {
      const asyncPatch = await patchAsyncIslands();
      if (asyncPatch.patched && asyncPatch.complete && bodyContainsOnlyAsyncIslands()) return;

      for (let attempt = 0; attempt < 60; attempt += 1) {
        try {
          const doc = await fetchCurrentDocument();
          if (!doc || !doc.body) throw new Error("rebuilt response did not contain a body");
          patchBody(doc);
          console.log("Cheers Subsecond hot reload morphed rebuilt HTML without reloading");
          return;
        } catch (error) {
          const message = error && error.message ? error.message : String(error);
          lastMessage = message === "Failed to fetch"
            ? "app server is not reachable yet; waiting for the rebuild/restart"
            : message;
          if (error && error.name === "CheersDocumentFetchTimeout") {
            break;
          }
          if (attempt === 0) {
            console.log("Cheers Subsecond hot reload waiting to morph rebuilt HTML", lastMessage);
          }
          await delay(attempt < 3 ? 150 : 500);
        }
      }
    } finally {
      morphing = false;
    }

    console.warn("Cheers Subsecond hot reload could not morph rebuilt HTML; falling back to page reload", lastMessage);
    window.location.reload();
  }

  function parseMessage(data) {
    try { return JSON.parse(data); } catch (_) { return null; }
  }

  function connect() {
    const url = new URL("/cheers/live-reload", window.location.href);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(url.href);

    socket.onopen = function() {
      if (attempts !== 0) morphCurrentPage();
      console.log("Cheers Subsecond live-reload connected");
      attempts = 0;
    };

    socket.onmessage = function(event) {
      const message = parseMessage(event.data);
      if (message && message.kind === "patch_applied") {
        morphCurrentPage();
      } else if (message && message.kind === "reload") {
        console.log("Cheers Subsecond hot reload falling back to page reload", message.kind);
        window.location.reload();
      }
    };

    socket.onclose = function() {
      const retryDelay = Math.min(50 * Math.pow(1.25, attempts), 1000);
      attempts++;
      reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        connect();
      }, retryDelay);
    };

    socket.onerror = function(err) {
      console.error("Cheers Subsecond live-reload WebSocket error:", err);

      if (reconnectTimer === null) {
        socket.close();
      }
    };
  }

  connect();
})();
</script>
                "#,
            );
            }

            render_cheers_iterate_script_to(buffer);
        }
    }
}

fn render_cheers_iterate_script_to(buffer: &mut Buffer<crate::context::Element>) {
    let Some(script_src) = crate::devtools::cheers_iterate_script_src().unwrap_or_default() else {
        return;
    };

    let script_src = html_escape::encode_double_quoted_attribute(&script_src);
    let script = format!(r#"<script data-cheers-dev-tool="iterate" src="{script_src}"></script>"#);
    // XSS SAFETY: script src is generated from a validated localhost origin and URL-safe token,
    // then escaped in attribute context. It is loaded only in debug builds.
    buffer.dangerously_get_string().push_str(&script);
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

/// A framework-served application JavaScript bundle.
///
/// Declare const values with [`include_js_bundle!`](crate::include_js_bundle) and render them on
/// pages that need the bundle:
///
/// ```ignore
/// use cheers::prelude::*;
///
/// const CHAT_JS: cheers::components::JsBundle = cheers::include_js_bundle!("./chat.js");
///
/// html! {
///     (CHAT_JS)
/// };
/// ```
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct JsBundle {
    pub(crate) location: crate::__internal::assets::AssetSourceLocation,
    pub(crate) js_file: &'static str,
    pub(crate) contents: &'static str,
}

impl JsBundle {
    #[doc(hidden)]
    pub const fn __new(
        location: crate::__internal::assets::AssetSourceLocation,
        js_file: &'static str,
        contents: &'static str,
    ) -> Self {
        Self {
            location,
            js_file,
            contents,
        }
    }
}

impl Render for JsBundle {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let script = format!(r#"<script src="{}"></script>"#, js_bundle_url(self));
        // XSS SAFETY: JS bundle URL is computed by us
        buffer.dangerously_get_string().push_str(&script);
    }
}

/// A reference to a symbol inside the global Cheers SVG sprite sheet.
///
/// In attribute context, this renders the `href` value for a `<use>` element. In element context,
/// it renders a minimal `<svg><use ...></use></svg>` wrapper.
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
