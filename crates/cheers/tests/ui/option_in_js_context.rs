use cheers::prelude::*;

fn main() {
    let count: Option<u32> = None;
    let _ = html! {
        div !text(count) {}
    };
}
