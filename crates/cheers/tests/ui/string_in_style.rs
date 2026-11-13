use cheers::prelude::*;

fn main() {
    let color = "red";
    let _ = html! {
        style { "a { color: " (color) "; }" }
    };
}
