use cheers::prelude::*;

fn main() {
    let on = false;
    let _ = datastar_object! { "a": @if on { "1" } };
    let _ = html! {
        div !attr("b": {}) {}
        div !style("c": @if on { "1" } @else if !on { "2" }) {}
        div !attr("d": @match on { true => "1", false => {} }) {}
        div !attr("e": "") {}
    };
}
