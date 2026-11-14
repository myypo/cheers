use std::{collections::BTreeMap, sync::Mutex, time::Duration};

use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use cheers::{
    components::{CssBundle, Doctype, Scripts, SvgSpritePreload, SvgSymbol},
    prelude::*,
};
use rand::Rng;
use tokio::sync::broadcast::error::RecvError;

#[derive(Clone)]
struct Ctx {
    stocks: &'static Mutex<BTreeMap<String, (String, u32)>>,
    stocks_tx: tokio::sync::broadcast::Sender<(String, String, u32)>,
}

type StockRows = Vec<(String, String, u32)>;

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
                    SvgSpritePreload;
                    (MAIN_CSS)
                }
                body {
                    main { (self.children) }
                    Scripts;
                }
            }
        }
        .render_to(buffer);
    }
}

#[derive(Cheers)]
struct Stock<'a> {
    #[id]
    id: &'a str,
    name: &'a str,
    #[signal]
    price_cents: u32,
}

impl<'a> Render for Stock<'a> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let StockSignals { signal_price_cents } = self.signals();
        let StockIds { id } = self.ids();

        let increment_action = IncrementStockAction {
            stock_id: self.id.to_owned(),
        };
        let dollar_price = format!("${:.2}", self.price_cents as f64 / 100.0);
        html! {
            section id=id {
                h3 { (SvgSymbol("icon-stock")) " " (self.name) }
                button
                    type="button"
                    aria:label={ "Increase " (self.name) " stock price by $1.00" }
                    !on:click(increment_action)
                {
                    "Price: "
                    output
                        aria:label={ (self.name) " current price" }
                        !signals(signal_price_cents: self.price_cents)
                    { (dollar_price) }
                    " (+$1.00)"
                }
            }
        }
        .render_to(buffer);
    }
}

async fn home_page(ctx: State<Ctx>) -> AsyncLazy<impl AsyncRender> {
    let get_stocks = async move || {
        tokio::time::sleep(Duration::from_millis(500)).await;
        ctx.stocks
            .lock()
            .expect("lock")
            .iter()
            .map(|(id, (name, price_cents))| (id.clone(), name.clone(), *price_cents))
            .collect::<StockRows>()
    };

    html! {
        Base {
            article !init(CreateSubscriptionAction) {
                @async {
                    @let stocks = get_stocks().await;
                    h1 { "Dwarven Stock Exchange" }
                    @for (id, name, price_cents) in stocks.iter() {
                        Stock id=(id.as_str()) name=(name.as_str()) price_cents=(*price_cents);
                    }
                    h2 { "Total Value" }
                    p   !text({
                            "'$' + ("
                            0
                            @for (id, _, _) in stocks.iter() {
                                "+"
                                (Stock::signal_price_cents(id.as_str()))
                            }
                            ") / 100"
                        }) {}
                } @else {
                    p { "Loading stocks..." }
                }
            }
        }
    }
}

#[action(PATCH)]
async fn increment_stock(Path(stock_id): Path<String>, ctx: State<Ctx>) -> PatchElements {
    let mut stocks = ctx.stocks.lock().expect("lock");
    let (name, price_cents) = stocks.get_mut(&stock_id).expect("stock exists");
    *price_cents += 100;
    if let Err(e) = ctx
        .stocks_tx
        .send((stock_id.clone(), name.clone(), *price_cents))
    {
        eprintln!("error sending stock update: {e}");
    };

    let stock = Stock {
        id: &stock_id,
        name,
        price_cents: *price_cents,
    };
    PatchElements::new().element(stock)
}

#[action(POST)]
async fn create_subscription(ctx: State<Ctx>) -> EventStream {
    println!("creating new subscription");
    EventStream::spawn(|tx| async move {
        let mut stocks_rx = ctx.stocks_tx.subscribe();
        loop {
            let (id, name, price_cents) = match stocks_rx.recv().await {
                Ok(update) => update,
                Err(RecvError::Lagged(skipped)) => {
                    eprintln!("subscription skipped {skipped} stock updates");
                    continue;
                }
                Err(RecvError::Closed) => return Ok::<_, SendError>(()),
            };
            let stock = Stock {
                id: &id,
                name: &name,
                price_cents,
            };
            tx.send(PatchElements::new().id(Stock::id(&id)).element(stock))?;
            println!("sent stock update for {id}: {name}");
        }
    })
}

const MAIN_CSS: CssBundle = include_css!("./main.css");
include_svg_sprite! {
    svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" {
        symbol id="icon-stock" viewBox="0 0 16 16" {
            path d="M2 13h12v1H2z";
            path d="M3 11V6h2v5z";
            path d="M7 11V3h2v8z";
            path d="M11 11V8h2v3z";
        }
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        let stocks_tx = tokio::sync::broadcast::channel(16).0;
        let stocks = Box::leak(Box::new(Mutex::new(BTreeMap::from([
            (
                "IRONFIST".to_owned(),
                ("Ironfist Mining Co.".to_owned(), 15000),
            ),
            (
                "STONEBREW".to_owned(),
                ("Stonebrew Ale & Spirits".to_owned(), 28000),
            ),
            (
                "DEEPFORGE".to_owned(),
                ("Deepforge Steel Works".to_owned(), 37500),
            ),
            (
                "GEMBEARD".to_owned(),
                ("Gembeard Jewelers".to_owned(), 25000),
            ),
            (
                "MOUNTAINHEART".to_owned(),
                ("Mountainheart Excavations".to_owned(), 17500),
            ),
        ]))));

        let ctx = Ctx {
            stocks_tx: stocks_tx.clone(),
            stocks,
        };

        let update_ctx = ctx.clone();
        tokio::spawn(async move {
            use rand::{SeedableRng, rngs::StdRng};

            let stock_ids = [
                "IRONFIST",
                "STONEBREW",
                "DEEPFORGE",
                "GEMBEARD",
                "MOUNTAINHEART",
            ];
            let mut rng = StdRng::from_entropy();

            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;

                let stock_id = stock_ids[rng.gen_range(0..stock_ids.len())];
                let change: i64 = loop {
                    let c = rng.gen_range(-10..=10);
                    if c != 0 {
                        break c;
                    }
                };

                let mut stocks = update_ctx.stocks.lock().expect("lock");
                if let Some((name, price)) = stocks.get_mut(stock_id) {
                    let new_price = (*price as i64 + change).max(1) as u32;
                    *price = new_price;

                    if let Err(e) =
                        update_ctx
                            .stocks_tx
                            .send((stock_id.to_owned(), name.clone(), new_price))
                    {
                        eprintln!("error sending stock update: {e}");
                    }
                }
            }
        });

        let app = cheers::router::new(
            Router::new()
                .route("/", get(home_page))
                .action::<IncrementStockAction>()
                .action::<CreateSubscriptionAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    })
    .await
    .unwrap();
}

#[cfg(test)]
mod tests {
    use thirtyfour::{prelude::*, stringmatch::StringMatchable};

    use super::*;

    #[tokio::test]
    async fn stock_increments() {
        let id = "whatever";
        let price_cents = 69;

        let stocks = Box::leak(Box::new(Mutex::new(BTreeMap::from([(
            id.to_owned(),
            ("yep".to_owned(), price_cents),
        )]))));
        let ctx = Ctx {
            stocks,
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };

        let app = cheers::router::new(
            Router::new()
                .route(
                    "/",
                    get(move || async move {
                        html! {
                            Base {
                                Stock id name="yep" price_cents;
                            }
                        }
                    }),
                )
                .action::<IncrementStockAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx);

        let app = cheers::test::App::new(app).await.unwrap();

        let button_selector = "//button[@aria-label='Increase yep stock price by $1.00']";
        let price_selector = "//*[@aria-label='yep current price']";
        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            app.query(By::XPath(price_selector))
                .with_text("$0.69".match_full())
                .first()
                .await?;

            app.query(By::XPath(button_selector))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            let price = app
                .query(By::XPath(price_selector))
                .with_text("$1.69".match_full())
                .first()
                .await?;
            assert_eq!(price.text().await?, "$1.69");

            Ok(())
        })
        .await
        .expect("increment stock in browser");
    }

    #[derive(Cheers)]
    struct Note<'a> {
        #[id]
        id: u32,
        text: &'a str,
    }

    impl Render for Note<'_> {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            let NoteIds { id } = self.ids();
            html! {
                p id=id { (self.text) }
            }
            .render_to(buffer);
        }
    }

    #[action(POST)]
    async fn update_notes() -> EventBatch {
        EventBatch::new()
            .with(PatchElements::new().element(Note {
                id: 1,
                text: "first updated",
            }))
            .with(PatchElements::new().element(Note {
                id: 2,
                text: "second updated",
            }))
    }

    #[tokio::test]
    async fn event_batch_applies_every_event() {
        let ctx = Ctx {
            stocks: Box::leak(Box::new(Mutex::new(BTreeMap::new()))),
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };

        let app = cheers::router::new(
            Router::new()
                .route(
                    "/",
                    get(|| async {
                        html! {
                            Base {
                                Note id=1 text="first";
                                Note id=2 text="second";
                                button type="button" !on:click(UpdateNotesAction) { "Update" }
                            }
                        }
                    }),
                )
                .action::<UpdateNotesAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx);

        let app = cheers::test::App::new(app).await.unwrap();

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            app.query(By::XPath("//button[text()='Update']"))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            for (id, text) in [(1, "first updated"), (2, "second updated")] {
                app.query(By::Id(Note::id(id).to_string()))
                    .with_text(text.match_full())
                    .first()
                    .await?;
            }

            Ok(())
        })
        .await
        .expect("apply every event in a batch");
    }

    #[derive(Cheers)]
    struct RunButton;

    impl Render for RunButton {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            scoped_signal!(signal_running: bool);

            html! {
                button
                    type="button"
                    !on:click(MarkScriptRanAction)
                    !indicator(signal_running)
                    !attr("disabled": signal_running)
                { "Run" }
            }
            .render_to(buffer);
        }
    }

    #[action(POST)]
    async fn mark_script_ran() -> ExecuteScript {
        ExecuteScript::new(js_script! { "window.scriptRan = true" })
    }

    #[tokio::test]
    async fn a_script_response_finishes_the_request() {
        let ctx = Ctx {
            stocks: Box::leak(Box::new(Mutex::new(BTreeMap::new()))),
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };

        let app = cheers::router::new(
            Router::new()
                .route(
                    "/",
                    get(|| async {
                        html! {
                            Base {
                                RunButton;
                            }
                        }
                    }),
                )
                .action::<MarkScriptRanAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx);

        let app = cheers::test::App::new(app).await.unwrap();

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            let button = By::XPath("//button[text()='Run']");
            app.query(button.clone())
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            let mut ran = false;
            for _ in 0..50 {
                ran = app
                    .execute("return window.scriptRan === true;", vec![])
                    .await?
                    .convert::<bool>()?;
                if ran {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            assert!(ran, "the script response should run");

            app.query(button)
                .and_enabled()
                .wait(Duration::from_secs(2), Duration::from_millis(100))
                .first()
                .await?;

            Ok(())
        })
        .await
        .expect("clear the indicator after a script response");
    }

    #[action(POST)]
    async fn sign_in() -> axum::response::Redirect {
        axum::response::Redirect::to("/hall")
    }

    #[action(POST)]
    async fn open_script_url() -> axum::response::Redirect {
        axum::response::Redirect::to("javascript:window.scriptUrlRan = true")
    }

    #[action(POST)]
    async fn show_notes() -> axum::response::Redirect {
        axum::response::Redirect::to("/#notes")
    }

    fn navigation_app() -> Router {
        let ctx = Ctx {
            stocks: Box::leak(Box::new(Mutex::new(BTreeMap::new()))),
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };

        cheers::router::new(
            Router::new()
                .route(
                    "/",
                    get(|| async {
                        html! {
                            Base {
                                Note id=1 text="first page";
                                button type="button" !on:click(SignInAction) { "Sign in" }
                                button type="button" !on:click(OpenScriptUrlAction) { "Open" }
                                button type="button" !on:click(ShowNotesAction) { "Notes" }
                            }
                        }
                    }),
                )
                .route(
                    "/hall",
                    get(|| async {
                        html! {
                            Base {
                                h1 { "Hall" }
                            }
                        }
                    }),
                )
                .action::<SignInAction>()
                .action::<OpenScriptUrlAction>()
                .action::<ShowNotesAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx)
    }

    #[tokio::test]
    async fn a_redirect_from_an_action_navigates_the_browser() {
        let app = cheers::test::App::new(navigation_app()).await.unwrap();

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            app.query(By::XPath("//button[text()='Sign in']"))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            app.query(By::XPath("//h1[text()='Hall']")).first().await?;
            assert_eq!(app.current_url().await?.path(), "/hall");
            assert!(
                app.find_all(By::Id(Note::id(1).to_string()))
                    .await?
                    .is_empty()
            );

            Ok(())
        })
        .await
        .expect("navigate after a redirect");
    }

    #[tokio::test]
    async fn a_redirect_to_a_script_url_does_not_run_it() {
        let app = cheers::test::App::new(navigation_app()).await.unwrap();

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            app.query(By::XPath("//button[text()='Open']"))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;
            tokio::time::sleep(Duration::from_millis(500)).await;

            let ran = app
                .execute("return window.scriptUrlRan === true;", vec![])
                .await?
                .convert::<bool>()?;
            assert!(!ran, "a redirect must not run a javascript: URL");
            assert_eq!(app.current_url().await?.path(), "/");

            Ok(())
        })
        .await
        .expect("ignore a redirect to a script URL");
    }

    #[tokio::test]
    async fn a_redirect_to_the_same_page_with_a_fragment_reloads_it() {
        let app = cheers::test::App::new(navigation_app()).await.unwrap();

        app.run(|app| async move {
            app.goto(app.url("/")).await?;
            app.execute(
                "document.body.insertAdjacentHTML('beforeend', '<p id=\"stale\"></p>');",
                vec![],
            )
            .await?;

            app.query(By::XPath("//button[text()='Notes']"))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            assert!(app.query(By::Id("stale")).not_exists().await?);
            let url = app.current_url().await?;
            assert_eq!((url.path(), url.fragment()), ("/", Some("notes")));

            Ok(())
        })
        .await
        .expect("reload the page on a redirect to its own URL");
    }

    /// Datastar binds the page from an undelayed `setTimeout`; holding it past the load event makes
    /// a navigation that does not wait for Datastar observably early.
    const HOLD_DATASTAR_SCRIPT: &str = r#"<script>
const nativeSetTimeout = window.setTimeout;
window.setTimeout = (callback, delay, ...args) =>
  nativeSetTimeout(callback, delay === undefined ? 500 : delay, ...args);
</script>"#;

    #[tokio::test]
    async fn goto_waits_for_datastar_without_live_reload() {
        let ctx = Ctx {
            stocks: Box::leak(Box::new(Mutex::new(BTreeMap::new()))),
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };

        let app = cheers::router::new(
            Router::new().route(
                "/",
                get(|| async {
                    // XSS SAFETY: static test script
                    let hold_datastar = cheers::Raw::dangerously_create(HOLD_DATASTAR_SCRIPT);
                    html! {
                        Doctype;
                        html {
                            body {
                                (hold_datastar)
                                Scripts;
                            }
                        }
                    }
                }),
            ),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx);

        let app = cheers::test::App::new(app).await.unwrap();

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            let ready = app
                .execute("return window.__cheersTestDatastarReady === true;", vec![])
                .await?
                .convert::<bool>()?;
            assert!(
                ready,
                "goto should return after Datastar has bound the page"
            );

            let live_reload_scripts = app
                .find_all(By::Css("script[data-cheers-runtime='live-reload']"))
                .await?;
            assert!(live_reload_scripts.is_empty());

            Ok(())
        })
        .await
        .expect("wait for datastar in browser");
    }

    #[action(POST)]
    async fn tag_request(headers: axum::http::HeaderMap) -> PatchElements {
        let text = match headers.get("x-trigger").map(|value| value.to_str()) {
            Some(Ok("save")) => "tagged save",
            _ => "untagged",
        };
        PatchElements::new().element(Note { id: 3, text })
    }

    #[tokio::test]
    async fn action_options_reach_the_request() {
        let ctx = Ctx {
            stocks: Box::leak(Box::new(Mutex::new(BTreeMap::new()))),
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };

        let app = cheers::router::new(
            Router::new()
                .route(
                    "/",
                    get(|| async {
                        let tag = TagRequestAction.options(
                            ActionOptions::new()
                                .header("X-Trigger", "save")
                                .open_when_hidden(true)
                                .request_cancellation(ActionCancellation::Cleanup),
                        );
                        html! {
                            Base {
                                Note id=3 text="waiting";
                                button type="button" !on:click(tag) { "Tag" }
                            }
                        }
                    }),
                )
                .action::<TagRequestAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx);

        cheers::test::App::new(app)
            .await
            .unwrap()
            .run(|app| async move {
                app.goto(app.url("/")).await?;
                app.query(By::XPath("//button[text()='Tag']"))
                    .and_clickable()
                    .first()
                    .await?
                    .click()
                    .await?;
                app.query(By::Id(Note::id(3).to_string()))
                    .with_text("tagged save".match_full())
                    .first()
                    .await?;
                Ok(())
            })
            .await
            .expect("send the action's header");
    }

    #[action(POST)]
    async fn search(axum::Form(_): axum::Form<BTreeMap<String, String>>) -> PatchElements {
        PatchElements::new().element(Note {
            id: 4,
            text: "searched",
        })
    }

    /// Registered after the runtime's listeners, so each entry says whether the runtime
    /// prevented the submission.
    const RECORD_SUBMISSIONS: &str = "window.__submits = []; window.__warnings = []; \
        console.warn = (...args) => window.__warnings.push(String(args[0])); \
        window.addEventListener('submit', (evt) => window.__submits.push(evt.defaultPrevented));";

    async fn submit_with_enter(app: &WebDriver) -> WebDriverResult<()> {
        app.find(By::Css("input[name='q']"))
            .await?
            .send_keys("ore" + Key::Enter)
            .await
    }

    async fn recorded_submissions(app: &WebDriver) -> WebDriverResult<(Vec<bool>, Vec<String>)> {
        for _ in 0..50 {
            let submits: Vec<bool> = app
                .execute("return window.__submits;", vec![])
                .await?
                .convert()?;
            if !submits.is_empty() {
                let warnings = app
                    .execute("return window.__warnings;", vec![])
                    .await?
                    .convert()?;
                return Ok((submits, warnings));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the form was not submitted");
    }

    fn search_page(form: impl Render) -> axum::response::Html<String> {
        axum::response::Html(
            html! {
                Base {
                    Note id=4 text="idle";
                    (form)
                }
            }
            .render()
            .into_inner(),
        )
    }

    fn search_app() -> Router {
        let ctx = Ctx {
            stocks: Box::leak(Box::new(Mutex::new(BTreeMap::new()))),
            stocks_tx: tokio::sync::broadcast::channel(1).0,
        };
        cheers::router::new(
            Router::new()
                .route(
                    "/button",
                    get(|| async {
                        search_page(html! {
                            form {
                                input name="q";
                                button type="button" !on:click(SearchAction) { "Search" }
                            }
                        })
                    }),
                )
                .route(
                    "/outside",
                    get(|| async {
                        let search =
                            SearchAction.options(ActionOptions::new().form_id("search-form"));
                        search_page(html! {
                            form id="search-form" {
                                input name="q";
                            }
                            button type="button" !on:click(search) { "Search" }
                        })
                    }),
                )
                .route(
                    "/submit",
                    get(|| async {
                        search_page(html! {
                            form !on:submit(SearchAction) {
                                input name="q";
                                button type="submit" { "Search" }
                            }
                        })
                    }),
                )
                .route(
                    "/native",
                    get(|| async {
                        search_page(html! {
                            form action="/native" {
                                input name="q";
                                button type="button" !on:click(SearchAction) { "Search" }
                            }
                        })
                    }),
                )
                .action::<SearchAction>(),
            cheers::router::Config::default(),
        )
        .expect("create app")
        .with_state(ctx)
    }

    #[tokio::test]
    async fn enter_does_not_reload_a_form_whose_action_is_on_a_button() {
        cheers::test::App::new(search_app())
            .await
            .unwrap()
            .run(|app| async move {
                for path in ["/button", "/outside"] {
                    app.goto(app.url(path)).await?;
                    app.execute(RECORD_SUBMISSIONS, vec![]).await?;
                    submit_with_enter(&app).await?;

                    let (submits, warnings) = recorded_submissions(&app).await?;
                    assert_eq!(submits, [true], "{path}");
                    assert_eq!(
                        warnings
                            .iter()
                            .any(|warning| warning.contains("blocked the native submission")),
                        cfg!(debug_assertions),
                        "{path}: {warnings:?}"
                    );
                    assert_eq!(app.current_url().await?.query(), None, "{path}");
                    app.query(By::Id(Note::id(4).to_string()))
                        .with_text("idle".match_full())
                        .first()
                        .await?;
                }
                Ok(())
            })
            .await
            .expect("block the native submission");
    }

    #[tokio::test]
    async fn a_form_submitting_through_its_action_does_not_warn() {
        cheers::test::App::new(search_app())
            .await
            .unwrap()
            .run(|app| async move {
                app.goto(app.url("/submit")).await?;
                app.execute(RECORD_SUBMISSIONS, vec![]).await?;
                submit_with_enter(&app).await?;

                app.query(By::Id(Note::id(4).to_string()))
                    .with_text("searched".match_full())
                    .first()
                    .await?;
                let (submits, warnings) = recorded_submissions(&app).await?;
                assert_eq!(submits, [true]);
                assert!(warnings.is_empty(), "{warnings:?}");
                assert_eq!(app.current_url().await?.query(), None);
                Ok(())
            })
            .await
            .expect("submit through the action without a warning");
    }

    #[tokio::test]
    async fn a_form_with_an_action_attribute_submits_natively() {
        cheers::test::App::new(search_app())
            .await
            .unwrap()
            .run(|app| async move {
                app.goto(app.url("/native")).await?;
                submit_with_enter(&app).await?;

                for _ in 0..50 {
                    if app.current_url().await?.query() == Some("q=ore") {
                        return Ok(());
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                panic!("the form was not submitted natively");
            })
            .await
            .expect("submit a form with an action attribute natively");
    }
}
