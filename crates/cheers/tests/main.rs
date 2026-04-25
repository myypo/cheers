#![expect(missing_docs, reason = "Test binary")]

use std::{
    fmt::{self, Debug, Display, Formatter},
    marker::Sync,
    sync::Arc,
    time::Duration,
};

use axum::{
    Form,
    body::Body,
    extract::{FromRequest, FromRequestParts, Path},
    http::StatusCode,
    response::IntoResponse,
};
use cheers::{
    ActionDef, RouterExt,
    components::{CssStylesheet, Debugged, Displayed, Doctype, Scripts, SvgSymbol},
    prelude::*,
};
use tokio::sync::{Barrier, Mutex};
use tower::ServiceExt;

use crate::test_utils::read_axum_body;

#[path = "../src/test_utils.rs"]
mod test_utils;

#[test]
fn can_render_vec() {
    let groceries = ["milk", "eggs", "bread"]
        .into_iter()
        .map(|s| {
            html! {
                li { (s) }
            }
        })
        .collect::<Vec<_>>();

    let result = html! {
        ul { (groceries) }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<ul><li>milk</li><li>eggs</li><li>bread</li></ul>"
    );
}

fn extract_href(rendered: &str) -> &str {
    rendered
        .split("href=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("rendered output should contain href attribute")
}

fn extract_src(rendered: &str) -> &str {
    rendered
        .split("src=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("rendered output should contain src attribute")
}

fn cheers_router() -> axum::Router<()> {
    cheers::router::new(axum::Router::<()>::new(), cheers::router::Config::default())
        .expect("router should build")
}

#[tokio::test]
async fn css_component_points_to_served_bundle() {
    let app = cheers_router();

    let rendered = CssStylesheet.render();
    let href = extract_href(rendered.as_inner());

    assert!(href.starts_with("/cheers/assets/"));

    let request = axum::http::Request::builder()
        .uri(href)
        .body(Body::empty())
        .expect("request should build");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should return a response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/css");
}

#[tokio::test]
async fn serves_registered_svg_sprite_sheet() {
    include_svg_sprite! {
        svg viewBox="0 0 16 16" {
            symbol id="icon-check" viewBox="0 0 16 16" {
                path d="M6.5 11.2 3.3 8l-1.1 1.1 4.3 4.3L14 5.9l-1.1-1.1z";
            }
        }
    }

    let app = cheers_router();

    let rendered = html! {
        svg {
            use href=(SvgSymbol("icon-check"));
        }
    }
    .render();

    let href = extract_href(rendered.as_inner());
    assert!(href.starts_with("/cheers/assets/"));
    assert!(href.ends_with("#icon-check"));

    let sprite_url = href
        .split_once('#')
        .map(|(url, _)| url)
        .expect("sprite symbol href should contain a fragment");

    let request = axum::http::Request::builder()
        .uri(sprite_url)
        .body(Body::empty())
        .expect("request should build");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should return a response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "image/svg+xml");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = String::from_utf8(body.into()).expect("response body should be valid UTF-8");

    assert!(body.contains(r#"<symbol id="icon-check""#));
}

#[tokio::test]
async fn js_bundle_omits_track_runtime_without_tracking_config() {
    let app = cheers_router();
    let rendered = Scripts.render();
    let src = extract_src(rendered.as_inner());

    let request = axum::http::Request::builder()
        .uri(src)
        .body(Body::empty())
        .expect("request should build");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should return a response");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = String::from_utf8(body.into()).expect("response body should be valid UTF-8");

    assert!(!body.contains("/_track"));
}

#[test]
fn correct_attr_escape() {
    let xss = r#""alert('XSS')"#;

    let result = html! {
        div "data-code"=(xss) {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div data-code="&quot;alert('XSS')"></div>"#
    );
}

#[test]
fn control() {
    let cond = true;

    let result = html! {
        div {
            @if cond {
                span { "branch 1" }
            } @else {
                span { "branch 2" }
            }
            @match !cond {
                true => span { "branch 1" }
                false => span { "branch 2" }
            }
            @for i in 0..3 {
                span { (i) }
            }
            @let mut i = 3;
            @while i < 6 {
                span { (i) }
                (i += 1)
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<div><span>branch 1</span><span>branch 2</span><span>0</span><span>1</span><span>2</span><span>3</span><span>4</span><span>5</span></div>"
    );
}

#[test]
fn component_fns() {
    fn component() -> impl Render {
        html! {
            span { "Hello, world!" }
        }
    }

    fn wrapping_component_html(c: impl Render) -> impl Render {
        html! {
            div { (c) }
        }
    }

    let result = html! {
        div { (component()) (wrapping_component_html(component())) }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r"<div><span>Hello, world!</span><div><span>Hello, world!</span></div></div>"
    );
}

#[test]
fn borrow() {
    let s = "Hello, world!".to_owned();
    let result = html! {
        span { (@&s) }
    };
    let expected = format!("<span>{s}</span>");

    assert_eq!(result.render().into_inner(), expected);
    assert_eq!(s, "Hello, world!");
}

#[test]
fn ref_expr_keeps_outer_value_available() {
    let s = "Hello!".to_owned();
    let result = html! {
        span { (@&s) }
    };
    let expected = format!("<span>{s}</span>");

    assert_eq!(result.render().into_inner(), expected);
    assert_eq!(s, "Hello!");
}

#[test]
fn ref_expr_keeps_outer_value_available_in_attribute_values() {
    let title = "Hello!".to_owned();
    let result = html! {
        div title=(@&title) {}
    };

    assert_eq!(
        result.render().into_inner(),
        r#"<div title="Hello!"></div>"#
    );
    assert_eq!(title, "Hello!");
}

#[test]
fn ref_expr_keeps_outer_value_available_in_js_attribute_values() {
    let value = "Hello!".to_owned();
    let result = html! {
        div !text((@&value)) {}
    };

    assert_eq!(
        result.render().into_inner(),
        r#"<div data-text="'Hello!'"></div>"#
    );
    assert_eq!(value, "Hello!");
}

#[test]
fn ref_expr_keeps_outer_values_available_across_nested_blocks() {
    let title = "Hello".to_owned();
    let subtitle = "World".to_owned();
    let show_subtitle = true;
    let result = html! {
        div {
            span { (@&title) }
            @if show_subtitle {
                strong { (@&subtitle) }
            } @else {}
        }
    };

    assert_eq!(
        result.render().into_inner(),
        "<div><span>Hello</span><strong>World</strong></div>"
    );
    assert_eq!(title, "Hello");
    assert_eq!(subtitle, "World");
}

#[test]
fn ref_expr_keeps_outer_value_available_in_component_prop_builders() {
    #[derive(Cheers)]
    struct Feedback<'a> {
        text: &'a str,
        #[prop(default("anonymous"))]
        author: &'a str,
    }

    impl<'a> Render for Feedback<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                h3 { (self.author) }
                p { (self.text) }
            }
            .render_to(buffer);
        }
    }

    let text = "Great".to_owned();
    let author = "myypo".to_owned();
    let result = html! {
        Feedback text=(@&text) [author=(@&author)];
    };

    assert_eq!(
        result.render().into_inner(),
        r#"<h3>myypo</h3><p>Great</p>"#
    );
    assert_eq!(text, "Great");
    assert_eq!(author, "myypo");
}

#[test]
fn ref_expr_keeps_outer_value_available_in_plain_component_props() {
    struct Badge<'a> {
        label: &'a str,
    }

    impl<'a> Render for Badge<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                span { (self.label) }
            }
            .render_to(buffer);
        }
    }

    let label = "Info".to_owned();
    let result = html! {
        Badge label=(@&label);
    };

    assert_eq!(result.render().into_inner(), "<span>Info</span>");
    assert_eq!(label, "Info");
}

#[test]
fn void_elements() {
    let result = html! {
        div {
            input type="text" name="username";
            input type="password" name="password";
            input type="submit" value="Login";
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div><input type="text" name="username"><input type="password" name="password"><input type="submit" value="Login"></div>"#
    );
}

#[test]
fn opengraph_meta_property_attribute() {
    let result = html! {
        head {
            meta property="og:title" content="Cheers";
            meta property="og:type" content="website";
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<head><meta property="og:title" content="Cheers"><meta property="og:type" content="website"></head>"#
    );
}

#[test]
fn component() {
    #[derive(Cheers)]
    struct Repeater<R> {
        count: usize,
        children: R,
    }

    impl<R: Render> Render for Repeater<R> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                @for _ in 0..self.count { (self.children) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        div {
            Repeater count=3 {
                span { "Hello, world!" }
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<div><span>Hello, world!</span><span>Hello, world!</span><span>Hello, world!</span></div>"
    );
}

#[test]
fn component_without_cheers_derive() {
    struct Card<'a, R> {
        title: &'a str,
        children: R,
    }

    impl<'a, R: Render> Render for Card<'a, R> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                section {
                    h2 { (self.title) }
                    div { (self.children) }
                }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Card title="Welcome" {
            span { "Hello" }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<section><h2>Welcome</h2><div><span>Hello</span></div></section>"#
    );
}

#[test]
fn unindent() {
    let result = html! {
        div title="multiline\ntitle" { "in\n    out\nin" }
        "\n"
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<div title=\"multiline\ntitle\">in\n    out\nin</div>\n"
    );
}

#[test]
fn displayed_debugged() {
    #[derive(Debug)]
    struct Greeting(&'static str);

    impl Display for Greeting {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "Hello, {}! <script>", self.0)
        }
    }

    let result = html! {
        div { (Displayed(Greeting("World"))) }
        div { (Debugged(Greeting("World"))) }
        div { (format_args!("{:#X}", 3_735_928_559_u32)) }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<div>Hello, World! &lt;script&gt;</div><div>Greeting(\"World\")</div><div>0xDEADBEEF</div>"
    );
}

#[test]
fn aria() {
    let result = html! {
        div aria:label="Hello, world!" { "Hello, world!" }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div aria-label="Hello, world!">Hello, world!</div>"#
    );
}

#[test]
fn aria_multiple_attributes() {
    let result = html! {
        button aria:pressed="false" aria:label="Toggle button" aria:hidden="false" role="button" {
            "Toggle"
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<button aria-pressed="false" aria-label="Toggle button" aria-hidden="false" role="button">Toggle</button>"#
    );
}

#[test]
#[cfg(feature = "mathml")]
fn mathml() {
    let result = html! {
        math {
            mi { "x" }
            mo { "+" }
            mn { "1" }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>"
    );
}

#[test]
fn svg_embedded_in_html() {
    let result = html! {
        div {
            svg width="100" height="100" {
                circle cx="50" cy="50" r="40" fill="red";
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div><svg width="100" height="100"><circle cx="50" cy="50" r="40" fill="red"/></svg></div>"#
    );
}

#[test]
fn svg_root_self_closing_children() {
    let result = html! {
        svg viewBox="0 0 100 100" {
            rect x="10" y="10" width="80" height="80";
            line x1="0" y1="0" x2="100" y2="100" stroke="black";
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<svg viewBox="0 0 100 100"><rect x="10" y="10" width="80" height="80"/><line x1="0" y1="0" x2="100" y2="100" stroke="black"/></svg>"#
    );
}

#[test]
fn svg_nested_children() {
    let result = html! {
        div {
            svg viewBox="0 0 200 200" {
                g transform="translate(10,10)" {
                    circle cx="50" cy="50" r="40";
                }
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div><svg viewBox="0 0 200 200"><g transform="translate(10,10)"><circle cx="50" cy="50" r="40"/></g></svg></div>"#
    );
}

#[test]
fn svg_root_xmlns_attribute_in_html_mode() {
    let result = html! {
        svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" {
            circle cx="5" cy="5" r="4";
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#
    );
}

#[test]
fn svg_foreign_object_switches_back_to_html() {
    let result = html! {
        svg width="200" height="200" {
            foreignObject x="10" y="10" width="180" height="180" {
                div {
                    p { "Hello from HTML inside SVG" }
                    input type="text";
                }
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<svg width="200" height="200"><foreignObject x="10" y="10" width="180" height="180"><div><p>Hello from HTML inside SVG</p><input type="text"></div></foreignObject></svg>"#
    );
}

#[test]
fn toggles() {
    let option_some = Some("value");

    let result = html! {
        input id=[option_some] type="checkbox" checked;
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<input id="value" type="checkbox" checked>"#
    );
}

#[derive(Cheers)]
struct Base<T> {
    children: T,
}

impl<T: Render> Render for Base<T> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! {
            Doctype;
            html {
                head {
                    Scripts;
                }
                body {
                    main { (self.children) }
                }
            }
        }
        .render_to(buffer);
    }
}

async fn next_axum_chunk(body: &mut axum::body::BodyDataStream) -> String {
    use futures::StreamExt;

    let ch = body
        .next()
        .await
        .expect("body stream should yield a chunk")
        .expect("body chunk should be readable");
    String::from_utf8(ch.to_vec()).expect("body chunk should be valid UTF-8")
}

#[tokio::test]
async fn page_is_rendered() {
    async fn main_page() -> impl IntoResponse {
        html! {
            Base {
                article {
                    p { "Data" }
                }
            }
        }
    }

    let result = main_page().await;
    let result = read_axum_body(result).await;
    assert!(
        result.contains(r#"<article><p>Data</p></article>"#),
        "{result}"
    );
}

#[tokio::test]
async fn page_async_block_is_streamed() {
    async fn main_page() -> cheers::prelude::AsyncLazy<cheers::prelude::Lazy<impl Fn(&mut Buffer)>>
    {
        html! {
            Base {
                article {
                    @async {
                        @let data = async { "Here!" };
                        div { (data.await) }
                    } @else {
                        div {
                            "Wait for it..."
                            p {}
                        }
                    }
                }
            }
        }
    }

    let mut result = main_page()
        .await
        .into_response()
        .into_body()
        .into_data_stream();
    let got = next_axum_chunk(&mut result).await;
    assert!(got.contains("Wait for it..."), "{got}");
    let got = next_axum_chunk(&mut result).await;
    assert!(got.contains("<div>Here!</div>"), "{got}");
}

#[derive(Cheers)]
struct ScopedSignalProbe {
    #[id]
    id: u32,
}

impl ScopedSignalProbe {
    fn render_signals(&self) -> String {
        scoped_signal!(signal_toggle);
        scoped_signal!(signal_typed: bool);
        html! {
            div !on:interval("@get('/')") {}
            p !signals(signal_toggle: true, signal_typed: false) {}
        }
        .render()
        .into_inner()
    }
}

#[test]
fn scoped_signal_hash() {
    let first_rendered = ScopedSignalProbe { id: 7 }.render_signals();
    let second_rendered = ScopedSignalProbe { id: 8 }.render_signals();

    let prefix = r#"<div data-on:interval="@get('/')"></div><p data-signals="{signal_toggle"#;
    let (first_toggle_hash, rest) = first_rendered
        .strip_prefix(prefix)
        .and_then(|rest| rest.split_once(":true,signal_typed"))
        .expect(&first_rendered);
    let (first_typed_hash, suffix) = rest.split_once(r#":false}"></p>"#).expect(&first_rendered);

    assert!(!first_toggle_hash.is_empty() && first_toggle_hash.chars().all(|c| c.is_ascii_digit()));
    assert!(!first_typed_hash.is_empty() && first_typed_hash.chars().all(|c| c.is_ascii_digit()));
    assert_ne!(first_toggle_hash, first_typed_hash);
    assert!(suffix.is_empty(), "unexpected trailing output: {suffix}");

    let (second_toggle_hash, _) = second_rendered
        .strip_prefix(prefix)
        .and_then(|rest| rest.split_once(":true,signal_typed"))
        .expect(&second_rendered);

    assert_ne!(first_toggle_hash, second_toggle_hash);
}

#[test]
fn svg_macro_foreign_object_switches_back_to_html() {
    let result = svg! {
        svg width="200" height="200" {
            foreignObject x="10" y="10" width="180" height="180" {
                div {
                    p { "Hello from HTML inside SVG" }
                    input type="text";
                }
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<svg width="200" height="200"><foreignObject x="10" y="10" width="180" height="180"><div><p>Hello from HTML inside SVG</p><input type="text"></div></foreignObject></svg>"#
    );
}

#[test]
fn svg_ref_expr_captures_by_reference() {
    let label = String::from("Icon");

    let result = svg! {
        svg viewBox="0 0 16 16" {
            title { (@&label) }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<svg viewBox="0 0 16 16"><title>Icon</title></svg>"#
    );
    assert_eq!(label, "Icon");
}

#[test]
fn svg_supports_fragments() {
    let result = svg! {
        circle cx="50" cy="50" r="40";
    }
    .render();

    assert_eq!(result.as_inner(), r#"<circle cx="50" cy="50" r="40"/>"#);
}

#[tokio::test]
async fn async_can_render_concurrently_in_order() {
    struct SyncPrimitives {
        barrier: Arc<Barrier>,
        mutex_a: Arc<Mutex<()>>,
        mutex_b: Arc<Mutex<()>>,
        mutex_c: Arc<Mutex<()>>,
    }

    async fn test_page(
        user: String,
        title: String,
        content: String,
        outages_today: i32,
        sync: SyncPrimitives,
    ) -> AsyncLazy<Lazy<impl Fn(&mut Buffer)>> {
        let post_html = {
            let barrier = sync.barrier.clone();
            let mutex_a = sync.mutex_a.clone();
            let mutex_b = sync.mutex_b.clone();
            async move {
                let _guard_a = mutex_a.lock().await;
                barrier.wait().await;
                let _guard_b = mutex_b.lock().await;
                format!("Title: {} Content: {}", title, content)
            }
        };
        let status_data = {
            let barrier = sync.barrier.clone();
            let mutex_a = sync.mutex_a.clone();
            let mutex_c = sync.mutex_c.clone();
            async move {
                let _guard_c = mutex_c.lock().await;
                barrier.wait().await;
                let _guard_a = mutex_a.lock().await;
                outages_today.to_string()
            }
        };

        html! {
            body {
                "Home of "
                (user)
                "Latest post:"
                @async {
                    div { (post_html.await) }
                } @else {
                    div { "Loading post..." }
                }
                "Status:"
                @async { (status_data.await) } @else {
                    div { "Loading status..." }
                }
            }
        }
    }

    let user = "myypo".to_owned();
    let title = "Hello".to_owned();
    let content = "World".to_owned();
    let outages_today = 1;

    let barrier = Arc::new(Barrier::new(2));

    let mutex_a = Arc::new(Mutex::new(()));
    let mutex_b = Arc::new(Mutex::new(()));
    let mutex_c = Arc::new(Mutex::new(()));

    let h = test_page(
        user,
        title.clone(),
        content.clone(),
        outages_today,
        SyncPrimitives {
            barrier,
            mutex_a,
            mutex_b,
            mutex_c,
        },
    )
    .await;

    let h = h.into_response();
    let mut h = h.into_body().into_data_stream();
    tokio::time::timeout(Duration::from_secs(1), async {
        next_axum_chunk(&mut h).await;
        next_axum_chunk(&mut h).await;
        next_axum_chunk(&mut h).await;
    })
    .await
    .expect("deadlock");
}

#[test]
fn component_dotdot_default() {
    #[derive(Default)]
    struct Feedback<'a> {
        name: Option<&'a str>,
        text: &'a str,
    }

    impl<'a> Render for Feedback<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                @if let Some(name) = self.name {
                    h3 { (name) }
                } @else {}
                p { (self.text) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Feedback text="Great" ..;
    }
    .render();

    assert_eq!(result.as_inner(), r#"<p>Great</p>"#);
}

#[test]
fn component_default_prop_without_override() {
    #[derive(Cheers)]
    struct Feedback<'a> {
        text: &'a str,
        #[prop(default("anonymous"))]
        author: &'a str,
    }

    impl<'a> Render for Feedback<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                h3 { (self.author) }
                p { (self.text) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Feedback text="Great" [];
    }
    .render();

    assert_eq!(result.as_inner(), r#"<h3>anonymous</h3><p>Great</p>"#);
}

#[test]
fn component_default_prop_with_override() {
    #[derive(Cheers)]
    struct Feedback<'a> {
        text: &'a str,
        #[prop(default("anonymous"))]
        author: &'a str,
    }

    impl<'a> Render for Feedback<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                h3 { (self.author) }
                p { (self.text) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Feedback text="Great" [author="myypo"];
    }
    .render();

    assert_eq!(result.as_inner(), r#"<h3>myypo</h3><p>Great</p>"#);
}

#[test]
fn component_default_prop_with_children() {
    #[derive(Cheers)]
    struct Card<'a, R> {
        title: &'a str,
        #[prop(default("note"))]
        kind: &'a str,
        children: R,
    }

    impl<'a, R: Render> Render for Card<'a, R> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                section {
                    h2 { (self.title) }
                    p { (self.kind) }
                    div { (self.children) }
                }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Card title="Greetings" [kind="warning"] {
            span { "Hello" }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<section><h2>Greetings</h2><p>warning</p><div><span>Hello</span></div></section>"#
    );
}

#[test]
fn component_required_props_can_be_out_of_order() {
    #[derive(Cheers)]
    struct Pair<'a> {
        a: &'a str,
        b: &'a str,
    }

    impl<'a> Render for Pair<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                p { (self.a) "-" (self.b) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Pair b="B" a="A";
    }
    .render();

    assert_eq!(result.as_inner(), r#"<p>A-B</p>"#);
}

#[test]
fn component_default_prop_with_filtered_where_clause() {
    #[derive(Cheers)]
    struct Message<T, U>
    where
        U: Clone,
    {
        value: T,
        #[prop(default(None))]
        extra: Option<U>,
    }

    impl<T, U> Render for Message<T, U>
    where
        T: Display,
        U: Clone + Display,
    {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            let extra = self
                .extra
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string());

            html! {
                p { (self.value.to_string()) " / " (extra) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        Message value=1 [extra=(Some("bonus"))];
    }
    .render();

    assert_eq!(result.as_inner(), r#"<p>1 / bonus</p>"#);
}

#[test]
fn component_default_prop_can_use_old_builder_method_names() {
    #[derive(Cheers)]
    struct BuilderNames<'a> {
        #[prop(default("one"))]
        build: &'a str,
        #[prop(default("two"))]
        build_with_children: &'a str,
    }

    impl<'a> Render for BuilderNames<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                p { (self.build) ":" (self.build_with_children) }
            }
            .render_to(buffer);
        }
    }

    let result = html! {
        BuilderNames [build="left" build_with_children="right"];
    }
    .render();

    assert_eq!(result.as_inner(), r#"<p>left:right</p>"#);
}

#[test]
fn component_default_only_props() {
    #[derive(Cheers)]
    struct Badge<'a> {
        #[prop(default("info"))]
        kind: &'a str,
    }

    impl<'a> Render for Badge<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                span { (self.kind) }
            }
            .render_to(buffer);
        }
    }

    let default_result = html! {
        Badge [];
    }
    .render();

    let overridden_result = html! {
        Badge [kind="warning"];
    }
    .render();

    assert_eq!(default_result.as_inner(), r#"<span>info</span>"#);
    assert_eq!(overridden_result.as_inner(), r#"<span>warning</span>"#);
}

#[test]
fn ids_with_id() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    #[id("number")]
    #[id("location")]
    struct House<'a> {
        #[id]
        id: u32,
        city: &'a str,
        street: &'a str,
    }

    impl<'a> House<'a> {
        fn assert_ids(&self) {
            ids!(id, id_number, id_location);

            assert_eq!(id.to_string(), "house-7");
            assert_eq!(id_number.to_string(), "house-7-number");
            assert_eq!(id_location.to_string(), "house-7-location");
        }
    }

    let instance_id = 7;
    assert_eq!(House::id(instance_id).to_string(), "house-7");
    assert_eq!(House::id_number(instance_id).to_string(), "house-7-number");
    assert_eq!(
        House::id_location(instance_id).to_string(),
        "house-7-location"
    );

    let house = House {
        id: 7,
        city: "whatever",
        street: "it is",
    };

    house.assert_ids();
}

#[test]
fn id_without_id() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    #[id("name")]
    #[id("price")]
    struct Steak<'a, M> {
        name: &'a str,
        dollars: M,
        cents: M,
    }

    impl<'a, M> Steak<'a, M> {
        fn assert_ids(&self) {
            ids!(id, id_name, id_price);

            assert_eq!(id.to_string(), "steak");
            assert_eq!(id_name.to_string(), "steak-name");
            assert_eq!(id_price.to_string(), "steak-price");
        }
    }

    assert_eq!(Steak::<i32>::id().to_string(), "steak");
    assert_eq!(Steak::<i32>::id_name().to_string(), "steak-name");
    assert_eq!(Steak::<i32>::id_price().to_string(), "steak-price");

    let steak = Steak {
        name: "porter",
        dollars: 10,
        cents: 99,
    };

    steak.assert_ids();
}

#[test]
fn data_indicator() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Something {
        #[signal]
        fetching: bool,
    }

    let fetching = Something::signal_fetching();
    let result = html! {
        button !indicator(fetching) !json_signals {}
        div !show({ "!" (fetching) " || true" }) { "Loaded!" }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<button data-indicator="something['fetching']" data-json-signals></button><div data-show="!$something['fetching'] || true">Loaded!</div>"#
    );
}

#[test]
fn data_text_escapes_rust_strings_for_js_and_html() {
    let value = "hi \"there\" <tag> & more\n\u{2028}\u{2029}\\ 'done'";

    let result = html! {
        div !text(value) {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "<div data-text=\"'hi &quot;there&quot; &lt;tag&gt; &amp; more\\n\\u2028\\u2029\\\\ \\\'done\\\''\"></div>"
    );
}

#[test]
fn data_signals() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Counter {
        #[signal]
        count: i32,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Other {
        #[signal]
        value: i32,
    }

    let count = Counter::signal_count();
    let value = Other::signal_value();

    let multiple_nested = html! {
        div !signals((@&count): 5, (@&value): 100) !text((@&count)) {}
    }
    .render();

    assert_eq!(
        multiple_nested.as_inner(),
        r#"<div data-signals="{counter:{count:5},other:{value:100}}" data-text="$counter['count']"></div>"#
    );
}

#[test]
fn data_signals_render_vecs_as_js_arrays() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Example {
        #[signal]
        values: Vec<String>,
    }

    let result = html! {
        div !signals(Example::signal_values(): vec!["bar".to_owned(), "baz".to_owned()]) {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div data-signals="{example:{values:['bar','baz']}}"></div>"#
    );
}

#[test]
fn data_style() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Options {
        #[signal]
        hiding: bool,
    }

    let hiding = Options::signal_hiding();
    let name = "color";
    let result = html! {
        pre !style("display": { (hiding) " ? 'none' : 'flex'" }, name: "'red'") {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<pre data-style="{display:$options['hiding'] ? 'none' : 'flex',color:'red'}"></pre>"#
    )
}

#[test]
fn control_flow_inside_js_attributes_uses_js_context() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Options {
        #[signal]
        hiding: bool,
    }

    let hiding = Options::signal_hiding();
    let cond = true;

    let result = html! {
        div !show({ @if cond { (hiding) } @else { "false" } }) {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div data-show="$options['hiding']"></div>"#
    );
}

#[allow(dead_code)]
#[test]
fn signal_computed() {
    #[derive(Cheers)]
    struct Input {
        #[signal]
        a: i32,
        #[signal]
        b: i32,
        #[signal]
        c: i32,
        #[signal]
        d: i32,
    }

    impl Render for Input {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            signals!(signal_a, signal_b, signal_c, signal_d);

            html! {
                p   !computed((@&signal_c): { (@&signal_a) "+" (@&signal_b) }, (@&signal_d): {
                            (@&signal_c)
                            "- 1"
                        }) {}
            }
            .render_to(buffer);
        }
    }

    #[derive(Cheers)]
    struct Calculator {
        #[signal(nested)]
        input: Input,
    }

    impl Render for Calculator {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            let Input { a, b, c, d } = self.input;
            html! {
                div {
                    Input a b c d;
                }
            }
            .render_to(buffer);
        }
    }

    let result = Calculator {
        input: Input {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
        },
    }
    .render()
    .into_inner();

    assert_eq!(
        result,
        r#"<div><p data-computed="{input:{c:()=>$input['a']+$input['b']}}" data-computed="{input:{d:()=>$input['c']- 1}}"></p></div>"#
    )
}

#[test]
fn signal_outer_without_id() {
    #[derive(Cheers)]
    #[signal(keepsake: String)]
    struct Ghost {
        name: String,
    }

    impl Render for Ghost {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            signals!(signal_keepsake);

            html! {
                p !bind((@&signal_keepsake)) !on:close({ (@&signal_keepsake) " + 'noooo'" }) {
                    (self.name)
                }
            }
            .render_to(buffer);
        }
    }

    let result = Ghost {
        name: "El".to_owned(),
    }
    .render()
    .into_inner();

    assert_eq!(
        result,
        r#"<p data-bind="ghost['keepsake']" data-on:close="$ghost['keepsake'] + 'noooo'">El</p>"#
    )
}

#[test]
fn signal_outer_with_id() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    #[signal(outside: String)]
    struct Outer {
        #[id]
        id: i32,
        name: String,
    }

    impl Outer {
        fn assert_signals(&self) {
            signals!(signal_outside);
            assert_eq!(
                signal_outside.render().into_inner(),
                "$outer['42']['outside']"
            );
        }
    }

    let outer = Outer {
        id: 42,
        name: "skipped".to_owned(),
    };
    outer.assert_signals();
    assert_eq!(
        Outer::signal_outside(42).render().into_inner(),
        "$outer['42']['outside']"
    );
}

#[test]
fn js_macro_literals_are_raw_js_source() {
    let rendered = js! {
        "console.log('wowzers')"
    }
    .render()
    .into_inner();

    assert_eq!(rendered, "console.log('wowzers')");
}

#[test]
fn js_macro_literals_are_attribute_escaped() {
    let rendered = js! {
        "if (x < \"&\") {}"
    }
    .render()
    .into_inner();

    assert_eq!(rendered, "if (x &lt; &quot;&amp;&quot;) {}");
}

#[test]
fn js_macro_interpolated_string_is_js_string_literal() {
    let name = "Ferris";

    let rendered = js! {
        "console.log("
        name
        ")"
    }
    .render()
    .into_inner();

    assert_eq!(rendered, "console.log('Ferris')");
}

#[test]
fn signal_id() {
    #[derive(Cheers)]
    #[expect(dead_code)]
    struct Ghost {
        #[id]
        id: i32,
        #[signal]
        name: String,
    }

    impl Render for Ghost {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            signals!(signal_name);

            html! {
                p
                    !bind((@&signal_name))
                    !text((@&signal_name))
                    !on:click({ "console.log(" (@&signal_name) ")" })
                    {}
            }
            .render_to(buffer);
        }
    }

    let result = Ghost {
        id: 69,
        name: "Ole".to_owned(),
    }
    .render()
    .into_inner();

    assert_eq!(
        result,
        r#"<p data-bind="ghost['69']['name']" data-text="$ghost['69']['name']" data-on:click="console.log($ghost['69']['name'])"></p>"#
    )
}

#[test]
fn signal_id_with_inline_js_macro() {
    #[derive(Cheers)]
    #[expect(dead_code)]
    struct Ghost {
        #[id]
        id: i32,
        #[signal]
        name: String,
    }

    impl Render for Ghost {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            signals!(signal_name);

            html! {
                p !bind((@&signal_name)) !on:click({ "console.log(" (@&signal_name) ")" }) {}
            }
            .render_to(buffer);
        }
    }

    let result = Ghost {
        id: 69,
        name: "Ole".to_owned(),
    }
    .render()
    .into_inner();

    assert_eq!(
        result,
        r#"<p data-bind="ghost['69']['name']" data-on:click="console.log($ghost['69']['name'])"></p>"#
    )
}

#[test]
fn signal_id_with_unsafe_segment() {
    #[derive(Cheers)]
    #[expect(dead_code)]
    struct GhostUser {
        #[id]
        id: &'static str,
        #[signal]
        name: String,
    }

    impl Render for GhostUser {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            signals!(signal_name);

            html! {
                p !bind((@&signal_name)) !on:click({ "console.log(" (@&signal_name) ")" }) {}
            }
            .render_to(buffer);
        }
    }

    let result = GhostUser {
        id: "user.123",
        name: "Ole".to_owned(),
    }
    .render()
    .into_inner();

    assert_eq!(
        result,
        r#"<p data-bind="ghost_user['user.123']['name']" data-on:click="console.log($ghost_user['user.123']['name'])"></p>"#
    )
}

#[test]
fn signal_deserialized_with_id_scope() {
    #[derive(Cheers)]
    #[signal(task_status: String)]
    #[expect(dead_code)]
    struct Project {
        #[id]
        project_id: i32,
        #[signal]
        name: String,
    }

    let got: ProjectSignalsJson = serde_json::from_str(
        r#"{ "project": { "1": { "name": "Website Redesign", "task_status": "in_progress" } } }"#,
    )
    .expect("signals JSON should deserialize");

    let project = got.project.get(&1).expect("project with id 1 should exist");
    assert_eq!(project.name, "Website Redesign");
    assert_eq!(project.task_status, "in_progress");
}

#[test]
fn signal_deserialized_nested_scope() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Child {
        #[signal]
        value: i32,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Parent {
        #[signal(nested)]
        child: Child,
    }

    let got: ParentSignalsJson =
        serde_json::from_str(r#"{ "parent": { "child": { "value": 1 } } }"#)
            .expect("nested signals JSON should deserialize");

    assert_eq!(got.parent.child.value, 1);
}

#[test]
fn signal_patch_with_id_scope() {
    #[derive(Cheers)]
    #[expect(dead_code)]
    struct Project {
        #[id]
        id: i32,
        #[signal]
        name: String,
    }

    let name = Project::signal_name(1);
    let result = html! {
        div !signals(name: "Website Redesign".to_owned()) {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div data-signals="{project:{1:{name:'Website Redesign'}}}"></div>"#
    );
}

#[test]
fn signal_without_id() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Flare {
        #[signal]
        num: i32,
    }

    impl Flare {
        fn assert_signals(&self) {
            signals!(signal_num);

            assert_eq!(signal_num.render().into_inner(), "$flare['num']");
        }
    }

    Flare { num: 5 }.assert_signals();
}

type Ctx = ();
cheers::app!(Ctx);

#[test]
fn action_with_plain_path() {
    #[action(POST)]
    #[expect(unused_variables)]
    async fn do_stuff(Path(name): Path<String>) {}

    let result = DoStuffAction {
        name: "Bob".to_owned(),
    }
    .render();
    assert_eq!(result.as_inner(), "@post('/cheers/actions/do_stuff/Bob')");
}

#[test]
fn action_path_segments_are_escaped_for_js_attributes() {
    #[action(POST)]
    #[expect(unused_variables)]
    async fn do_stuff(Path(name): Path<String>) {}

    let result = DoStuffAction {
        name: "O'Reilly & <friends> \"x\"".to_owned(),
    }
    .render();

    assert_eq!(
        result.as_inner(),
        "@post('/cheers/actions/do_stuff/O\\'Reilly &amp; &lt;friends&gt; &quot;x&quot;')"
    );
}

#[test]
fn action_with_tuple_path() {
    #[action(POST)]
    #[expect(unused_variables)]
    async fn do_stuff(Path((name, age)): Path<(String, i32)>) {}

    let result = DoStuffAction {
        name: "Bob".to_owned(),
        age: 42,
    }
    .render();
    assert_eq!(
        result.as_inner(),
        "@post('/cheers/actions/do_stuff/Bob/42')"
    );
}

#[test]
fn action_explicit_path() {
    struct NotPath;

    impl<S: Sync> FromRequestParts<S> for NotPath {
        type Rejection = ();

        async fn from_request_parts(
            _: &mut axum::http::request::Parts,
            _: &S,
        ) -> Result<Self, Self::Rejection> {
            Ok(NotPath)
        }
    }

    #[action(PUT)]
    async fn my_handler(#[path] _: NotPath) {}

    let result = MyHandlerAction {}.render();
    assert_eq!(result.as_inner(), "@put('/cheers/actions/my_handler')");
}

#[test]
#[allow(dead_code)]
fn action_form_generics() {
    #[derive(Cheers)]
    struct Stuff<'a, S> {
        #[form]
        whatever: &'a S,
    }

    impl<'a, S: Render> Render for Stuff<'a, S> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            form_names!(form_whatever);

            html! {
                form {
                    input name=(form_whatever);
                    p { (self.whatever) }
                }
            }
            .render_to(buffer);
        }
    }

    #[action(PUT)]
    async fn my_handler(_: Form<StuffForm<String>>) {}

    let result = MyHandlerAction {}.render();
    assert_eq!(
        result.as_inner(),
        "@put('/cheers/actions/my_handler',{contentType:'form'})"
    );

    let stuff = Stuff {
        whatever: &"hello".to_owned(),
    };
    let result = stuff.render().into_inner();
    assert_eq!(
        result,
        r#"<form><input name="whatever"><p>hello</p></form>"#
    );
}

#[test]
fn action_explicit_form() {
    struct NotForm;

    impl<S: Sync> FromRequest<S> for NotForm {
        type Rejection = ();

        async fn from_request(_: axum::extract::Request, _: &S) -> Result<Self, Self::Rejection> {
            Ok(NotForm)
        }
    }

    #[action(POST)]
    async fn my_handler(#[form] _: NotForm) {}

    let result = MyHandlerAction {}.render();
    assert_eq!(
        result.as_inner(),
        "@post('/cheers/actions/my_handler',{contentType:'form'})"
    );
}

#[test]
fn action_form_serde() {
    fn default_whatever() -> String {
        String::from("lol")
    }

    #[expect(dead_code)]
    #[derive(Cheers, PartialEq)]
    struct Stuff {
        #[signal]
        #[form(serde(default = "default_whatever"))]
        whatever: String,
    }

    let result: StuffForm = serde_json::from_str("{}").expect("form JSON should deserialize");
    assert_eq!(result.whatever, String::from("lol"));
}

#[test]
fn form_without_field() {
    #[derive(Cheers)]
    #[form(keepsake: String, serde(default))]
    struct Ghost<'a> {
        name: &'a str,
    }

    impl<'a> Render for Ghost<'a> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            form_names!(form_keepsake);

            html! {
                form {
                    input name=(form_keepsake);
                    p { (self.name) }
                }
            }
            .render_to(buffer);
        }
    }

    impl<'a> Ghost<'a> {
        fn assert_form_names(&self) {
            form_names!(form_keepsake);
            assert_eq!(form_keepsake.render().into_inner(), "keepsake");
        }
    }

    Ghost { name: "whatever" }.assert_form_names();

    let result: GhostForm = serde_json::from_str("{}").expect("form JSON should deserialize");
    assert_eq!(result.keepsake, String::from(""));

    let result = Ghost { name: "and" }.render();
    assert_eq!(
        result.as_inner(),
        r#"<form><input name="keepsake"><p>and</p></form>"#
    );
}

#[test]
fn action_def_path_and_method() {
    #[action(POST)]
    #[expect(unused_variables)]
    async fn do_stuff(Path(name): Path<String>) {}

    assert_eq!(DoStuffAction::PATH, "/cheers/actions/do_stuff/{name}");
    assert_eq!(DoStuffAction::METHOD, axum::http::Method::POST);
}

#[test]
fn action_def_no_path() {
    #[action(DELETE)]
    async fn remove_thing() {}

    assert_eq!(RemoveThingAction::PATH, "/cheers/actions/remove_thing");
    assert_eq!(RemoveThingAction::METHOD, axum::http::Method::DELETE);
}

#[tokio::test]
async fn action_mock_routes_to_correct_path() {
    use axum::{Router, extract::State, http::Request};

    #[action(POST)]
    #[expect(unused_variables)]
    async fn greet(Path(name): Path<String>, State(()): State<()>) -> impl IntoResponse {
        "real"
    }

    let app = Router::new().mock_action(GreetAction::mock(
        |Path(_name): Path<String>, State(()): State<()>| async { "mocked" },
    ));

    let req = Request::builder()
        .method("POST")
        .uri("/cheers/actions/greet/world")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(read_axum_body(resp).await, "mocked");
}

#[test]
fn form_with_derive() {
    #[expect(dead_code)]
    #[derive(Cheers)]
    #[form_derive(Debug, Default, PartialEq)]
    struct Simple {
        #[form]
        name: String,
    }

    assert_eq!(
        SimpleForm::default(),
        SimpleForm {
            name: String::new()
        }
    );
}
