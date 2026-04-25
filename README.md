# Cheers

Cheers is an experimental **alpha-quality** fullstack hypermedia framework for Rust
inspired by [Datastar](https://data-star.dev).

It is designed from the ground up to solve issues I encountered when trying to
model highly dynamic and hierarchical data with
[Leptos](https://github.com/leptos-rs/leptos) and [Dioxus](https://github.com/dioxuslabs/dioxus),
then I stumbled on Datastar and after a quick evaluation, to my surprise
it fit my use-case so well that I begun building abstractions on top of it
and (back then) Jinja-like templates.

Cheers relates to Datastar the way Next.js relates to React - sorry for the jumpscare.
However, it is not guaranteed that it is going to stay this way as Cheers may eventually switch
to a completely custom JS solution or even WASM (very unlikely considering all of its downsides).

It uses [Maud](https://github.com/lambda-fairy/maud)-like macros for HTML templating,
[Datastar](https://github.com/starfederation/datastar) for client-side reactivity and
[Axum](https://github.com/tokio-rs/axum) as the HTTP server library

The repository currently contains:

- `cheers` - public library API
- `cargo-cheers` - Cargo subcommand, currently only includes `cargo cheers fmt` for formatting macros
- Other crates in the repo are for internal-use only

## Minimal app

- `@async { ... } @else { ... }` suspense while the initial records load
- `scoped_signal!` for a component-local in-flight indicator
- `#[form(name: String)]` for a generated `DwarfListForm` type without storing form state
- `#[action(POST)]` plus `PatchElementsMode::Append` to add a new record into the list

<!-- readme-app:start -->
```rust no_run
use std::time::Duration;

use axum::{
    Router,
    extract::{Form, State},
    routing::get,
};
use cheers::{
    components::{Doctype, Scripts},
    prelude::*,
};

#[derive(Clone)]
struct Ctx;

#[derive(Cheers)]
struct Dwarf {
    name: String,
}

impl Render for Dwarf {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! {
            p {
                "Engraved name: "
                strong { (self.name) }
            }
        }
        .render_to(buffer);
    }
}

#[derive(Cheers)]
#[form(name: String)]
struct DwarfList {
    dwarfs: Vec<Dwarf>,
}

impl Render for DwarfList {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id);
        form_names!(form_name);

        scoped_signal!(signal_forging: bool);

        html! {
            section {
                h2 { "Add Dwarf Record" }
                form {
                    label {
                        "Dwarf Name"
                        input name=form_name;
                    }
                    button
                        type="submit"
                        !on:click((ForgeRecordAction {}))
                        !indicator(signal_forging)
                        !attr("disabled": signal_forging)
                    { "Engrave" }
                }
                h2 { "Dwarf List" }
                ul id=id {
                    @for d in &self.dwarfs {
                        li { (d) }
                    }
                }
            }
        }
        .render_to(buffer);
    }
}

async fn hall_of_ancestors(_: State<Ctx>) -> AsyncLazy<impl Render> {
    let thorin = async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        Dwarf {
            name: String::from("Thorin Ironmantle"),
        }
    };

    html! {
        Doctype;
        html {
            body {
                main {
                    h1 { "⛏ The Deep Halls" }
                    @async {
                        @let dwarfs = vec![thorin.await];
                        DwarfList dwarfs;
                    } @else {
                        p { "Consulting the elder runes..." }
                    }
                }
                Scripts;
            }
        }
    }
}

#[action(POST)]
async fn forge_record(_: State<Ctx>, Form(form): Form<DwarfListForm>) -> PatchElements {
    tokio::time::sleep(Duration::from_millis(500)).await;

    let new_record = Dwarf { name: form.name };

    PatchElements::new()
        .id(DwarfList::id())
        .mode(PatchElementsMode::Append)
        .element(html! {
            li { (new_record) }
        })
}

cheers::app!(Ctx);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(
        Router::new().route("/", get(hall_of_ancestors)),
        cheers::router::Config::default(),
    )?
    .with_state(Ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::extract::State;
    use cheers::RouterExt as _;
    use thirtyfour::{prelude::*, stringmatch::StringMatchable};

    use super::*;

    #[tokio::test]
    async fn forge_record_action_mock_updates_the_page() {
        let app = cheers::router::new(
            Router::new()
                .route("/", get(hall_of_ancestors))
                .mock_action(ForgeRecordAction::mock(|State(_ctx): State<Ctx>| async {
                    PatchElements::new()
                        .id(DwarfList::id())
                        .mode(PatchElementsMode::Append)
                        .element(html! {
                            li {
                                (
                                    Dwarf {
                                        name: String::from("Mocked Silvervein"),
                                    }
                                )
                            }
                        })
                })),
            cheers::router::Config::default(),
        )
        .expect("create test app")
        .with_state(Ctx);

        let app = cheers::test::App::new(app)
            .await
            .expect("start browser app");

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            app.query(By::Css("li"))
                .with_text("Engraved name: Thorin Ironmantle".match_full())
                .first()
                .await?;

            app.find(By::Css("input[name='name']"))
                .await?
                .send_keys("Real input ignored by the mock")
                .await?;

            app.query(By::Css("button"))
                .with_text("Engrave".match_full())
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            let mocked_record = app
                .query(By::Css("li"))
                .with_text("Engraved name: Mocked Silvervein".match_full())
                .first()
                .await?;

            assert_eq!(
                mocked_record.text().await?,
                "Engraved name: Mocked Silvervein"
            );

            Ok(())
        })
        .await
        .expect("mocked action should update the page");
    }
}
```
<!-- readme-app:end -->

## Acknowledgements

- Most of the current templating code was taken from [vidhanio/hypertext](https://github.com/vidhanio/hypertext).
- `cargo cheers fmt` formatter was based on [Jeosas/maudfmt](https://github.com/Jeosas/maudfmt).
