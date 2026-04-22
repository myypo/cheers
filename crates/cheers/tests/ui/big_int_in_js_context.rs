use cheers::prelude::*;

fn main() {
    let big: u64 = 9_000_000_000;
    let _ = html! {
        div !text(big) {}
    };
}
