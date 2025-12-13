use std::{collections::BTreeMap, sync::Mutex, time::Duration};

use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use cheers::{
    Buffer,
    components::{Css, Doctype, Scripts},
    prelude::*,
    router::CheersRouterExt,
};

#[derive(Clone)]
struct Ctx {
    stocks: &'static Mutex<BTreeMap<String, String>>,
    stocks_tx: tokio::sync::broadcast::Sender<(String, String)>,
}

struct Base<T> {
    children: T,
}

impl<T: Render> Render for Base<T> {
    fn render_to(&self, buffer: &mut Buffer<cheers::context::Node>) {
        html! {
            Doctype;
            html {
                head {
                    Css;
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

#[derive(Component)]
#[id(id)]
struct Stock<'a> {
    id: &'a str,
    name: &'a str,
}

impl<'a> Render for Stock<'a> {
    fn render_to(&self, buffer: &mut Buffer<cheers::context::Node>) {
        html! {
            section id=(Self::id(self.id)) {
                h3 { "And the name is " (self.name) }
            }
        }
        .render_to(buffer);
    }
}

async fn home_page(ctx: State<Ctx>) -> AsyncLazy<Lazy<impl Fn(&mut Buffer)>> {
    html! {
        Base {
            article !init="@post('/subscriptions')" {
                @async {
                    @let resp = async { "Hey" };
                    button
                        !on:click="@post('/')"
                        !indicator="fetching"
                        !style="{display: $fetching && 'none'}"
                    { (resp.await) }
                    @for (id, name) in &ctx.stocks.lock().expect("lock").clone() {
                        Stock id name;
                    }
                } @else {
                    p { "Wait..." }
                }
            }
        }
    }
}

async fn update_stock(ctx: State<Ctx>) -> PatchElements {
    tokio::time::sleep(Duration::from_millis(500)).await;

    let id = "Wow".to_owned();
    let name = "Major".to_owned();
    let mut stocks = ctx.stocks.lock().expect("lock");
    stocks.insert(id.clone(), name.clone());
    if let Err(e) = ctx.stocks_tx.send((id.clone(), name.clone())) {
        eprintln!("error sending stock update: {e}");
    };

    PatchElements::new().component(Stock {
        id: &id,
        name: &name,
    })
}

async fn create_subscription(ctx: State<Ctx>) -> impl IntoResponse {
    println!("creating new subscription");
    let (tx, rx) = events();
    tokio::spawn(async move {
        let mut stocks_rx = ctx.stocks_tx.subscribe();
        while let Ok((id, name)) = stocks_rx.recv().await {
            if let Err(e) = tx.send(PatchElements::new().id(Stock::id(&id)).component(Stock {
                id: &id,
                name: &name,
            })) {
                eprintln!("error forwarding update to subscription: {e}");
                break;
            } else {
                println!("sent stock update for {id}: {name}");
            };
        }
    });

    rx
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        include_css!("./main.css");

        let router = Router::new()
            .route("/", get(home_page))
            .route("/", post(update_stock))
            .route("/subscriptions", post(create_subscription))
            .with_state(Ctx {
                stocks_tx: tokio::sync::broadcast::channel(16).0,
                stocks: Box::leak(Box::new(Mutex::new(BTreeMap::from([(
                    "Wow".to_owned(),
                    "Hotsteel".to_owned(),
                )])))),
            })
            .serve_cheers_application()
            .unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
            .await
            .unwrap();
        axum::serve(listener, router).await.unwrap();
    })
    .await
    .unwrap();
}
