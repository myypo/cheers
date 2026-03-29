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
