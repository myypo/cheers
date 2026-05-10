use cheers::prelude::*;

fn main() {
    let _ = html! {
        div !on_interval[prevent]("count++") {}
    };
}
