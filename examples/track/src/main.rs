use axum::{
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use cheers::{
    components::{Doctype, Scripts},
    prelude::*,
    track::{TrackConfig, TrackRequest},
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Ctx;

async fn guild_hall() -> impl IntoResponse {
    html! {
        Doctype;
        html {
            head {
                title { "Dwarven Ledger Example" }
            }
            body {
                main style="padding: 2rem; display: flex; flex-direction: column; gap: 1rem" {
                    h1 { "Guild Ledger Tracking" }
                    p {
                        "Open the browser console and inspect the network tab to watch the guild ledger record page visits and rune-marked events."
                    }
                    button
                        !on:click((
                            TrackAction(GuildLedgerPayload::VaultOpened {
                                vault_id: 69,
                            })
                        ))
                    { "Open vault" }
                    button !on:click((TrackAction(GuildLedgerPayload::AleMenuOpened))) {
                        "Open ale menu"
                    }
                }
                Scripts;
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GuildLedgerPayload {
    AleMenuOpened,
    VaultOpened { vault_id: u64 },
}

async fn record_in_ledger(track: TrackRequest<GuildLedgerPayload>) -> StatusCode {
    println!("--- etching into the ledger ---");
    println!("{:?}", track);
    println!("--- etched into the ledger ---");
    StatusCode::ACCEPTED
}

cheers::app!(Ctx);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let track_config = TrackConfig::new("/_track")
        .service("guild-ledger")
        .release("0.1.0");

    let app = app(
        Router::new()
            .route("/", get(guild_hall))
            .route("/_track", post(record_in_ledger)),
        cheers::router::Config::default().track(track_config),
    )?
    .with_state(Ctx);

    println!("Listening on http://127.0.0.1:8080");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
