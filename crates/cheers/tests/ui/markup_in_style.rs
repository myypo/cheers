use cheers::prelude::*;

#[derive(Cheers)]
struct Greeting;

impl Render for Greeting {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! { p { "hi" } }.render_to(buffer);
    }
}

fn main() {
    let _ = html! {
        style {
            "a { color: red; }"
            b { "bold" }
            Greeting;
            @async {
                p { "later" }
            } @else {
                p { "soon" }
            }
        }
    };
}
