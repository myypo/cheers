use cheers::prelude::*;

fn main() {
    let busy = datastar_object! { "disabled": "true" };
    let show = true;
    let _ = html! { button !attr(..@if show { (busy) }) {} };
    let _ = html! { button !attr(..@&busy) {} };
}
