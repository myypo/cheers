use cheers::prelude::*;

fn main() {
    let label = "ab";
    let _ = html! { button !attr(..label) {} };
}
