//! Holding action requests and capturing tracked items in `cheers::test`.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use cheers::{
    components::{Doctype, Scripts},
    prelude::*,
    track::{Item, TrackAction, TrackConfig},
};
use thirtyfour::prelude::*;

type Saves = Arc<AtomicUsize>;

#[derive(Cheers)]
#[id]
struct Status<'a> {
    text: &'a str,
}

impl Render for Status<'_> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let StatusIds { id } = self.ids();
        html! {
            p id=id { (self.text) }
        }
        .render_to(buffer);
    }
}

#[derive(Cheers)]
struct SaveButton;

impl Render for SaveButton {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        scoped_signal!(signal_saving: bool);

        html! {
            button
                type="button"
                !on:click(SaveAction)
                !indicator(signal_saving)
                !attr("disabled": signal_saving)
            { "Save" }
        }
        .render_to(buffer);
    }
}

#[action(POST)]
async fn save(State(saves): State<Saves>) -> PatchElements {
    saves.fetch_add(1, Ordering::SeqCst);
    PatchElements::new().element(Status { text: "saved" })
}

fn save_app(saves: Saves) -> Router {
    cheers::router::new(
        Router::new()
            .route(
                "/",
                get(|| async {
                    html! {
                        Doctype;
                        html lang="en" {
                            head { title { "Save" } }
                            body {
                                Status text="unsaved";
                                SaveButton;
                                Scripts;
                            }
                        }
                    }
                }),
            )
            .action::<SaveAction>(),
        cheers::router::Config::default(),
    )
    .expect("create app")
    .with_state(saves)
}

const BUTTON: &str = "//button[text()='Save']";

async fn status(app: &WebDriver) -> WebDriverResult<String> {
    app.find(By::Id(Status::id().to_string()))
        .await?
        .text()
        .await
}

async fn wait_for_saved(app: &WebDriver) -> WebDriverResult<()> {
    app.query(By::Id(Status::id().to_string()))
        .with_text("saved")
        .first()
        .await?;
    Ok(())
}

#[tokio::test]
async fn a_held_response_keeps_the_request_pending_after_the_handler_ran() {
    let saves = Saves::default();
    cheers::test::App::new(save_app(saves.clone()))
        .await
        .expect("start the app")
        .run(|app| async move {
            app.goto(app.url("/")).await?;
            let held = app.hold_action::<SaveAction>();
            app.query(By::XPath(BUTTON))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            held.reached().await?;
            assert_eq!(saves.load(Ordering::SeqCst), 1);
            assert_eq!(status(&app).await?, "unsaved");
            assert!(!app.find(By::XPath(BUTTON)).await?.is_enabled().await?);

            held.release();
            wait_for_saved(&app).await?;
            app.query(By::XPath(BUTTON)).and_enabled().first().await?;
            Ok(())
        })
        .await
        .expect("hold the response");
}

#[tokio::test]
async fn a_held_request_reaches_the_handler_once_released() {
    let saves = Saves::default();
    cheers::test::App::new(save_app(saves.clone()))
        .await
        .expect("start the app")
        .run(|app| async move {
            app.goto(app.url("/")).await?;
            let held = app.hold_action_request::<SaveAction>();
            app.query(By::XPath(BUTTON))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            held.reached().await?;
            assert_eq!(saves.load(Ordering::SeqCst), 0);

            held.release();
            wait_for_saved(&app).await?;
            assert_eq!(saves.load(Ordering::SeqCst), 1);
            Ok(())
        })
        .await
        .expect("hold the request");
}

#[tokio::test]
async fn an_unreached_hold_dropped_holds_nothing() {
    let saves = Saves::default();
    cheers::test::App::new(save_app(saves.clone()))
        .await
        .expect("start the app")
        .run(|app| async move {
            app.goto(app.url("/")).await?;
            drop(app.hold_action_request::<SaveAction>());
            app.query(By::XPath(BUTTON))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;
            wait_for_saved(&app).await?;
            Ok(())
        })
        .await
        .expect("save without a hold");
}

#[tokio::test]
async fn tracked_items_are_captured_as_the_ingest_route_answers_them() {
    let ingested = Arc::new(AtomicUsize::new(0));
    let counted = ingested.clone();
    let router = cheers::router::new(
        Router::new()
            .route(
                "/_harness_track",
                post(move || async move {
                    counted.fetch_add(1, Ordering::SeqCst);
                    StatusCode::ACCEPTED
                }),
            )
            .route(
                "/",
                get(|| async {
                    html! {
                        Doctype;
                        html lang="en" {
                            head { title { "Vault" } }
                            body {
                                button
                                    type="button"
                                    !on:click((TrackAction(serde_json::json!({ "vault_id": 69 }))))
                                { "Open vault" }
                                Scripts;
                            }
                        }
                    }
                }),
            ),
        cheers::router::Config::default().track(TrackConfig::new("/_harness_track")),
    )
    .expect("create app");

    cheers::test::App::new(router)
        .await
        .expect("start the app")
        .run(|app| async move {
            app.goto(app.url("/")).await?;
            let viewed = app
                .wait_for_tracked(|items| {
                    items.iter().find_map(|item| match item {
                        Item::PageView(view) => Some(view.context.pathname.clone()),
                        _ => None,
                    })
                })
                .await?;
            assert_eq!(viewed, "/");

            app.query(By::XPath("//button[text()='Open vault']"))
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;
            let props = app
                .wait_for_tracked(|items| {
                    items.iter().find_map(|item| match item {
                        Item::Analytics(event) => Some(event.props.clone()),
                        _ => None,
                    })
                })
                .await?;
            assert_eq!(props, serde_json::json!({ "vault_id": 69 }));
            let tracked = app.tracked();
            let count = |kind: fn(&Item<serde_json::Value>) -> bool| {
                tracked.iter().filter(|item| kind(item)).count()
            };
            assert_eq!(count(|item| matches!(item, Item::PageView(_))), 1);
            assert_eq!(count(|item| matches!(item, Item::Analytics(_))), 1);
            assert!(ingested.load(Ordering::SeqCst) >= 1);
            Ok(())
        })
        .await
        .expect("capture tracked items");
}

#[tokio::test]
async fn a_hold_outliving_the_test_does_not_keep_the_app_from_shutting_down() {
    use tokio::io::AsyncWriteExt;

    let app = cheers::test::App::new(save_app(Saves::default()))
        .await
        .expect("start the app");
    let run = app.run(|app| async move {
        let held = app.hold_action_request::<SaveAction>();
        // A connection of its own, which closing the browser would not end.
        let address = app.url("").trim_start_matches("http://").to_owned();
        let mut connection = tokio::net::TcpStream::connect(address.trim_end_matches('/')).await?;
        connection
            .write_all(
                b"POST /cheers/actions/save HTTP/1.1\r\nHost: test\r\nContent-Length: 0\r\n\r\n",
            )
            .await?;
        held.reached().await?;
        Ok((held, connection))
    });
    let (held, _connection) = tokio::time::timeout(std::time::Duration::from_secs(10), run)
        .await
        .expect("shut down while a request is held")
        .expect("run the app");
    held.release();
}
