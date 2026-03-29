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

This example lives in [`examples/readme/src/main.rs`](examples/readme/src/main.rs)
and is synced into this README from that source file.

- `@async { ... } @else { ... }` suspense while the initial records load
- `scoped_signal!` for a component-local in-flight indicator
- `#[form(name: String)]` for a generated `DwarfListForm` type without storing form state
- `#[action(POST)]` plus `PatchElementsMode::Append` to stream a new record into the list

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

        let signal_forging: Signal<bool> = scoped_signal!("forging");

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
    let app = app(Router::new().route("/", get(hall_of_ancestors)))?.with_state(Ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```
<!-- readme-app:end -->

## Acknowledgements

- Most of the current templating code was taken from [vidhanio/hypertext](https://github.com/vidhanio/hypertext).
- `cargo cheers fmt` formatter was based on [Jeosas/maudfmt](https://github.com/Jeosas/maudfmt).
