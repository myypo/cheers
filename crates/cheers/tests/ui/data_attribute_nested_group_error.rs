use cheers::prelude::*;

fn main() {
    let x = 1;
    let _ = html! {
        div !attr("k": { "a" ...x }) {}
        button !on:click({ "a" @@ }) {}
        p { (x) }
    };
}
