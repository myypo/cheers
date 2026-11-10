use cheers::prelude::*;

fn main() {
    let _ = datastar_object! { "k": @async { "1" } @else { span {} } };
    let on = true;
    let _ = html! {
        div !attr("k": { @if on { @async { "1" } @else { span {} } } @else { "2" } }) {}
    };
}
