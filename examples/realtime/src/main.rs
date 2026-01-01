use std::{collections::BTreeMap, sync::Mutex, time::Duration};

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use cheers::{
    components::{Css, Doctype, Scripts},
    prelude::*,
    router::{App, CheersRouterExt},
};

#[derive(Clone)]
struct Ctx {
    stocks: &'static Mutex<BTreeMap<String, (String, u64)>>,
    stocks_tx: tokio::sync::broadcast::Sender<(String, String, u64)>,
}

struct Base<T> {
    children: T,
}

impl<T: Render> Render for Base<T> {
    fn render_to(&self, buffer: &mut cheers::Buffer<cheers::context::Element>) {
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
    #[signal(id)]
    id: &'a str,
    name: &'a str,
    #[signal]
    price_cents: u64,
}

impl<'a> Render for Stock<'a> {
    fn render_to(&self, buffer: &mut cheers::Buffer<cheers::context::Element>) {
        let price_cents_signal = Stock::price_cents_signal(self.id);
        html! {
            section id=(Self::id(self.id)) {
                h3 { "Name " (self.name) }
                input
                    value=(self.price_cents)
                    type="number"
                    !signals(price_cents_signal: self.price_cents)
                    !bind(price_cents_signal);
                p !text(price_cents_signal) { (self.price_cents) }
            }
        }
        .render_to(buffer);
    }
}

async fn home_page(ctx: State<Ctx>) -> AsyncLazy<impl Render> {
    let fetching = Signal::<bool>::scoped("fetching");
    let get_stocks = async move || {
        tokio::time::sleep(Duration::from_millis(500)).await;
        ctx.stocks.lock().expect("lock")
    };

    html! {
        Base {
            article !init(CreateSubscriptionAction) {
                @async {
                    @let stocks = get_stocks().await;
                    button
                        !on:click(CreateSubscriptionAction)
                        !indicator(fetching)
                        !style("display": { (fetching) " && 'none'" })
                    { "Do stuff" }
                    h1 { "Sum" }
                    @for (id, (name, price_cents)) in stocks.iter() {
                        Stock id name price_cents=(*price_cents);
                    }
                    p   !text({
                            0
                            @for (id, _) in stocks.iter() { "+" (Stock::price_cents_signal(id)) }
                        }) {}
                } @else {
                    p { "Wait..." }
                }
            }
        }
    }
}

#[action(PATCH)]
async fn update_stock(ctx: State<Ctx>) -> PatchElements {
    tokio::time::sleep(Duration::from_millis(500)).await;

    let id = "Wow";
    let mut stocks = ctx.stocks.lock().expect("lock");
    let (name, price_cents) = stocks.get_mut(id).expect(
        "hardcoded Wow
stock",
    );
    *price_cents += 1;
    if let Err(e) = ctx
        .stocks_tx
        .send((id.to_owned(), name.clone(), *price_cents))
    {
        eprintln!("error sending stock update: {e}");
    };

    let stock = Stock {
        id,
        name,
        price_cents: *price_cents,
    };
    PatchElements::new().element(stock)
}

#[action(POST)]
async fn create_subscription(ctx: State<Ctx>) -> impl IntoResponse {
    println!("creating new subscription");
    let (tx, rx) = events();
    tokio::spawn(async move {
        let mut stocks_rx = ctx.stocks_tx.subscribe();
        while let Ok((id, name, price_cents)) = stocks_rx.recv().await {
            let stock = Stock {
                id: &id,
                name: &name,
                price_cents,
            };
            if let Err(e) = tx.send(PatchElements::new().id(Stock::id(&id)).element(stock)) {
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

        let app = App::new()
            .unwrap()
            .with_action::<CreateSubscriptionAction>()
            .with_action::<UpdateStockAction>();

        let router = Router::new()
            .route("/", get(home_page))
            .serve_cheers_application(app)
            .with_state(Ctx {
                stocks_tx: tokio::sync::broadcast::channel(16).0,
                stocks: Box::leak(Box::new(Mutex::new(BTreeMap::from([(
                    "Wow".to_owned(),
                    ("Hotsteel".to_owned(), 42),
                )])))),
            });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
            .await
            .unwrap();
        axum::serve(listener, router).await.unwrap();
    })
    .await
    .unwrap();
}
