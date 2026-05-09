use cheers::prelude::*;

fn main() {
    let _ = html! {
        div !on:not_registered("console.log('nope')") {}
    };
}
