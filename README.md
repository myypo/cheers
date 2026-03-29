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

- `@async { ... } @else { ... }` suspense profile loading
- `#[signal]` save profile in-flight indicator
- `#[form]` for generated form field names and the generated `ProfileForm` type
- `#[action(POST)]` backend action that patches the updated component back into the page

<!-- readme-app:start -->
```rust no_run
use axum::{
    Router,
    extract::{Form, Path, State},
    routing::get,
};
use cheers::{
    components::{Doctype, Scripts},
    prelude::*,
};

#[derive(Clone)]
struct Ctx;

#[derive(Cheers)]
struct Profile {
    #[id]
    id: u32,
    #[signal]
    saving: bool,
    #[form]
    name: String,
}

impl Render for Profile {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id);
        signals!(signal_saving);
        form_names!(form_name);

        let save_action = SaveProfileAction {
            profile_id: self.id,
        };

        html! {
            section id=id !signals(signal_saving: self.saving) {
                h2 { "Edit profile" }
                form !on:submit(save_action) {
                    label {
                        "Name"
                        input name=(form_name) value=(self.name);
                    }
                    button type="submit" !indicator(signal_saving) {
                        "Save"
                    }
                }
                p !show(signal_saving) { "Saving..." }
                p {
                    "Saved name: "
                    strong { (self.name) }
                }
            }
        }
        .render_to(buffer);
    }
}

async fn home_page(_: State<Ctx>) -> AsyncLazy<impl Render> {
    let profile = async {
        Profile {
            id: 1,
            saving: false,
            name: String::from("Ferris"),
        }
    };

    html! {
        Doctype;
        html {
            body {
                main {
                    h1 { "Cheers" }
                    @async {
                        @let profile = profile.await;
                        Profile id=(profile.id) saving=(profile.saving) name=(profile.name);
                    } @else {
                        p { "Loading profile..." }
                    }
                }
                Scripts;
            }
        }
    }
}

#[action(POST)]
async fn save_profile(
    Path(profile_id): Path<u32>,
    _: State<Ctx>,
    Form(form): Form<ProfileForm>,
) -> PatchElements {
    let updated = Profile {
        id: profile_id,
        saving: false,
        name: form.name,
    };

    PatchElements::new()
        .id(Profile::id(profile_id))
        .mode(PatchElementsMode::Outer)
        .element(updated)
}

cheers::app!(Ctx);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(Router::new().route("/", get(home_page)))?.with_state(Ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```
<!-- readme-app:end -->

## Acknowledgements

- Most of the current templating code was taken from [vidhanio/hypertext](https://github.com/vidhanio/hypertext).
- `cargo cheers fmt` formatter was based on [Jeosas/maudfmt](https://github.com/Jeosas/maudfmt).
