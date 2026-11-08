use cheers::prelude::*;

fn main() {
    let base = datastar_object! { "color": "'red'" };
    let _ = datastar_object! { ...base };
    let _ = html! { pre !style(...base) {} };
}
