#![expect(missing_docs, reason = "Test binary")]

use std::{
    fmt::{self, Debug, Display, Formatter},
    marker::Sync,
    sync::Arc,
    time::Duration,
};

use axum::{
    Form,
    extract::{FromRequest, FromRequestParts, Path},
    response::IntoResponse,
};
use cheers::{
    components::{Debugged, Displayed, Doctype, Scripts},
    macros::{html_borrow, svg_borrow, svg_static},
    prelude::*,
};
use futures::StreamExt;
use tokio::sync::{Barrier, Mutex};

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
    let result = html_borrow! {
        span { (s) }
    };
    // still able to use `s` after the borrow, as we use `html_borrow!`
    let expected = format!("<span>{s}</span>");

    assert_eq!(result.render().into_inner(), expected);
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

    for result in [result] {
        assert_eq!(
            result.as_inner(),
            "<div>Hello, World! &lt;script&gt;</div><div>Greeting(\"World\")</div><div>0xDEADBEEF</div>"
        );
    }
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
fn svg_namespace_attributes() {
    let result = html! {
        svg viewBox="0 0 10 10" {
            g xml:lang="en" xmlns:sprite="urn:cheers:test" {
                circle cx="5" cy="5" r="4";
            }
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<svg viewBox="0 0 10 10"><g xml:lang="en" xmlns:sprite="urn:cheers:test"><circle cx="5" cy="5" r="4"/></g></svg>"#
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

async fn read_axum_body(resp: impl axum::response::IntoResponse) -> String {
    use futures::StreamExt;

    let resp = resp.into_response();
    resp.into_body()
        .into_data_stream()
        .fold(String::new(), async |mut acc, ch| {
            acc.push_str(&String::from_utf8(ch.unwrap().to_vec()).unwrap());
            acc
        })
        .await
}

async fn next_axum_chunk(body: &mut axum::body::BodyDataStream) -> String {
    use futures::StreamExt;

    let ch = body.next().await.unwrap().unwrap();
    String::from_utf8(ch.to_vec()).unwrap()
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

#[test]
fn scoped_signal_hash() {
    let toggle: Signal<bool> = scoped_signal!("toggle");
    let nested: Signal<&'static str> = scoped_signal!("nested", "go42", "bye");
    let result = html! {
        div !on:interval("@get('/')") {}
        p !signals(toggle: true, nested: "'impressive'") {}
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r#"<div data-on:interval="@get('/')"></div><p data-signals="{toggle3463295118:true,nested:{go42:{bye1528366059:'impressive'}}}"></p>"#
    );
}

#[test]
fn svg_macro_sprite_bundle() {
    let result = svg! {
        svg xmlns:sprite="urn:cheers:test" xml:lang="en" viewBox="0 0 16 16" {
            defs {
                symbol id="icon-check" viewBox="0 0 16 16" {
                    path d="M6.5 11.2 3.3 8l-1.1 1.1 4.3 4.3L14 5.9l-1.1-1.1z";
                }
            }
            use href="#icon-check" x="0" y="0" width="16" height="16";
        }
    }
    .render();

    assert_eq!(
        result.as_inner(),
        r##"<svg xmlns:sprite="urn:cheers:test" xml:lang="en" viewBox="0 0 16 16"><defs><symbol id="icon-check" viewBox="0 0 16 16"><path d="M6.5 11.2 3.3 8l-1.1 1.1 4.3 4.3L14 5.9l-1.1-1.1z"/></symbol></defs><use href="#icon-check" x="0" y="0" width="16" height="16"/></svg>"##
    );
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
fn svg_borrow_captures_by_reference() {
    let label = String::from("Icon");

    let result = svg_borrow! {
        svg viewBox="0 0 16 16" {
            title { (label) }
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
fn svg_static_supports_fragments() {
    let result = svg_static! {
        circle cx="50" cy="50" r="40";
    };

    assert_eq!(*result.as_inner(), r#"<circle cx="50" cy="50" r="40"/>"#);
}

#[tokio::test]
async fn async_can_render_concurrently_in_order() {
    async fn test_page(
        user: String,
        title: String,
        content: String,
        outages_today: i32,
        barrier: Arc<Barrier>,
        mutex_a: Arc<Mutex<()>>,
        mutex_b: Arc<Mutex<()>>,
        mutex_c: Arc<Mutex<()>>,
    ) -> AsyncLazy<Lazy<impl Fn(&mut Buffer)>> {
        let post_html = {
            let barrier = barrier.clone();
            let mutex_a = mutex_a.clone();
            let mutex_b = mutex_b.clone();
            async move {
                let _guard_a = mutex_a.lock().await;
                barrier.wait().await;
                let _guard_b = mutex_b.lock().await;
                format!("Title: {} Content: {}", title, content)
            }
        };
        let status_data = {
            let barrier = barrier.clone();
            let mutex_a = mutex_a.clone();
            let mutex_c = mutex_c.clone();
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
        barrier,
        mutex_a,
        mutex_b,
        mutex_c,
    )
    .await;

    let h = h.into_response();
    let mut h = h.into_body().into_data_stream();
    tokio::time::timeout(Duration::from_secs(1), async {
        h.next().await.unwrap().unwrap();
        h.next().await.unwrap().unwrap();
        h.next().await.unwrap().unwrap();
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
        Feedback text=("Great") ..;
    }
    .render();

    assert_eq!(result.as_inner(), r#"<p>Great</p>"#);
}

#[test]
fn ids_with_id() {
    #[expect(dead_code)]
    #[derive(Refs)]
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
    #[derive(Refs)]
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
    #[derive(Refs)]
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
        r#"<button data-indicator="something.fetching" data-json-signals></button><div data-show="!$something.fetching || true">Loaded!</div>"#
    );
}

#[test]
fn data_signals() {
    #[expect(dead_code)]
    #[derive(Refs)]
    struct Counter {
        #[signal]
        count: i32,
    }

    #[expect(dead_code)]
    #[derive(Refs)]
    struct Other {
        #[signal]
        value: i32,
    }

    let count = Counter::signal_count();
    let value = Other::signal_value();

    let multiple_nested = html! {
        div !signals(count: 5, value: 100) {}
    }
    .render();

    assert_eq!(
        multiple_nested.as_inner(),
        r#"<div data-signals="{counter:{count:5},other:{value:100}}"></div>"#
    );
}

#[test]
fn data_style() {
    #[expect(dead_code)]
    #[derive(Refs)]
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
        r#"<pre data-style="{display:$options.hiding ? 'none' : 'flex',color:'red'}"></pre>"#
    )
}

#[allow(dead_code)]
#[test]
fn signal_computed() {
    #[derive(Refs)]
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
                p   !computed(signal_c: { (signal_a) "+" (signal_b) }, signal_d: {
                            (signal_c)
                            "- 1"
                        }) {}
            }
            .render_to(buffer);
        }
    }

    #[derive(Refs)]
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
        r#"<div><p data-computed="{input:{c:()=>$input.a+$input.b}}" data-computed="{input:{d:()=>$input.c- 1}}"></p></div>"#
    )
}

#[test]
fn signal_outer_without_id() {
    #[derive(Refs)]
    #[signal(keepsake: String)]
    struct Ghost {
        name: String,
    }

    impl Render for Ghost {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            signals!(signal_keepsake);

            html! {
                p !bind(&signal_keepsake) !on:close({ (signal_keepsake) " + 'noooo'" }) {
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
        r#"<p data-bind="ghost.keepsake" data-on:close="$ghost.keepsake + 'noooo'">El</p>"#
    )
}

#[test]
fn signal_outer_with_id() {
    #[expect(dead_code)]
    #[derive(Refs)]
    #[signal(outside: String)]
    struct Outer {
        #[id]
        id: i32,
        name: String,
    }

    impl Outer {
        fn assert_signals(&self) {
            signals!(signal_outside);
            assert_eq!(signal_outside.render().into_inner(), "$outer.42.outside");
        }
    }

    let outer = Outer {
        id: 42,
        name: "skipped".to_owned(),
    };
    outer.assert_signals();
    assert_eq!(
        Outer::signal_outside(42).render().into_inner(),
        "$outer.42.outside"
    );
}

#[test]
fn signal_id() {
    #[derive(Refs)]
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
                p !bind(signal_name) !on:click({ "console.log(" signal_name ")" }) {}
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
        r#"<p data-bind="ghost.69.name" data-on:click="console.log($ghost.69.name)"></p>"#
    )
}

#[test]
fn signal_deserialized_with_id_scope() {
    #[derive(Refs)]
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
    .unwrap();

    let project = got.project.get(&1).unwrap();
    assert_eq!(project.name, "Website Redesign");
    assert_eq!(project.task_status, "in_progress");
}

#[test]
fn signal_deserialized_nested_scope() {
    #[expect(dead_code)]
    #[derive(Refs)]
    struct Child {
        #[signal]
        value: i32,
    }

    #[expect(dead_code)]
    #[derive(Refs)]
    struct Parent {
        #[signal(nested)]
        child: Child,
    }

    let got: ParentSignalsJson =
        serde_json::from_str(r#"{ "parent": { "child": { "value": 1 } } }"#).unwrap();

    assert_eq!(got.parent.child.value, 1);
}

#[test]
fn signal_patch_with_id_scope() {
    #[derive(Refs)]
    #[expect(dead_code)]
    struct Project {
        #[id]
        id: i32,
        #[signal]
        name: String,
    }

    let name = Project::signal_name(1);
    let result = html! {
        div !signals(name: "'Website Redesign'".to_owned()) {}
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
    #[derive(Refs)]
    struct Flare {
        #[signal]
        num: i32,
    }

    impl Flare {
        fn assert_signals(&self) {
            signals!(signal_num);

            assert_eq!(signal_num.render().into_inner(), "$flare.num");
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
    #[derive(Refs)]
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
    #[derive(Refs, PartialEq)]
    struct Stuff {
        #[signal]
        #[form(serde(default = "default_whatever"))]
        whatever: String,
    }

    let result: StuffForm = serde_json::from_str("{}").unwrap();
    assert_eq!(result.whatever, String::from("lol"));
}

#[test]
fn form_without_field() {
    #[derive(Refs)]
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

    let result: GhostForm = serde_json::from_str("{}").unwrap();
    assert_eq!(result.keepsake, String::from(""));

    let result = Ghost { name: "and" }.render();
    assert_eq!(
        result.as_inner(),
        r#"<form><input name="keepsake"><p>and</p></form>"#
    );
}

#[test]
fn form_with_derive() {
    #[expect(dead_code)]
    #[derive(Refs)]
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
