use cheers::prelude::*;

fn main() {
    let _ = html! {
        @async {
            p { "Loaded" }
        } @else {
            p { "Loading" }
            "Still loading"
        }
    };
}
