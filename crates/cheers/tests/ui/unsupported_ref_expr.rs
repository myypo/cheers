use cheers::prelude::*;

fn main() {
    let name = String::from("Ferris");

    let _ = html! {
        p { (@&format!("Hello, {name}!")) }
    };
}
