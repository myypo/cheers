use std::{collections::BTreeMap, sync::Mutex, time::Duration};

use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use cheers::{
    components::{CssStylesheet, Doctype, Scripts, SvgSymbol},
    prelude::*,
};
use rand::Rng;

#[derive(Clone)]
struct Ctx {
    stocks: &'static Mutex<BTreeMap<String, (String, u32)>>,
    stocks_tx: tokio::sync::broadcast::Sender<(String, String, u32)>,
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
                    CssStylesheet;
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
        signals!(signal_price_cents);
        ids!(id);

        let increment_action = IncrementStockAction {
            stock_id: self.id.to_owned(),
        };
        let dollar_price = format!("${:.2}", self.price_cents as f64 / 100.0);
        html! {
            section id=id {
                h3 { (SvgSymbol("icon-stock")) " " (self.name) }
                button type="button" aria:label={ "Increase " (self.name) " stock price by $1.00" } !on:click(increment_action) {
                    "Price: "
                    output aria:label={ (self.name) " current price" } !signals(signal_price_cents: self.price_cents) { (dollar_price) }
                    " (+$1.00)"
                }
            }
        }
        .render_to(buffer);
    }
}

async fn home_page(ctx: State<Ctx>) -> AsyncLazy<impl Render> {
    let get_stocks = async move || {
        tokio::time::sleep(Duration::from_millis(500)).await;
        ctx.stocks.lock().expect("lock")
    };

    html! {
        Base {
            article !init(CreateSubscriptionAction) {
                @async {
                    @let stocks = get_stocks().await;
                    h1 { "Dwarven Stock Exchange" }
                    @for (id, (name, price_cents)) in stocks.iter() {
                        Stock id name price_cents=(*price_cents);
                    }
                    h2 { "Total Value" }
                    p   !text({
                            "'$' + ("
                            0
                            @for (id, _) in stocks.iter() { "+" (Stock::signal_price_cents(id)) }
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
async fn create_subscription(ctx: State<Ctx>) -> EventReceiver {
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

cheers::app!(Ctx);

include_css!("./main.css");
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

        let app = app(
            Router::new().route("/", get(home_page)),
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
    use super::*;
    use thirtyfour::{prelude::*, stringmatch::StringMatchable};

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

        let app = app(
            Router::new().route(
                "/",
                get(move || async move {
                    html! {
                        Base {
                            Stock id name="yep" price_cents;
                        }
                    }
                }),
            ),
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
}
