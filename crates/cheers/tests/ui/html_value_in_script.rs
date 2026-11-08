use cheers::prelude::*;

fn main() {
    let markup = html! { b { "bold" } };
    let _ = html! {
        script { (markup) }
    };
}
