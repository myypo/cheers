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
#[id]
#[form(name: String)]
struct DwarfList {
    dwarfs: Vec<Dwarf>,
}

impl Render for DwarfList {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let DwarfListIds { id } = self.ids();
        let DwarfListFormNames { form_name } = self.form_names();

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = cheers::router::new(
        Router::new()
            .route("/", get(hall_of_ancestors))
            .action::<ForgeRecordAction>(),
        cheers::router::Config::default(),
    )?
    .with_state(Ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use thirtyfour::{prelude::*, stringmatch::StringMatchable};

    use super::*;

    #[tokio::test]
    async fn forge_record_action_updates_the_page() {
        let app = cheers::router::new(
            Router::new()
                .route("/", get(hall_of_ancestors))
                .action::<ForgeRecordAction>(),
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
                .send_keys("Balin Stonehelm")
                .await?;

            app.query(By::Css("button"))
                .with_text("Engrave".match_full())
                .and_clickable()
                .first()
                .await?
                .click()
                .await?;

            let new_record = app
                .query(By::Css("li"))
                .with_text("Engraved name: Balin Stonehelm".match_full())
                .first()
                .await?;

            assert_eq!(new_record.text().await?, "Engraved name: Balin Stonehelm");

            Ok(())
        })
        .await
        .expect("action should update the page");
    }
}
