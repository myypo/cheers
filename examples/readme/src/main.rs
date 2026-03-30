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
    let app = app(Router::new().route("/", get(hall_of_ancestors)))?.with_state(Ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
