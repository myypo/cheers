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
        script {
            "start();"
            b { "bold" }
            Greeting;
            @async {
                p { "later" }
            } @else {
                p { "soon" }
            }
        }
    };
    let _ = js_script! {
        @async { "later" } @else { div {} }
    };
    let _ = datastar_source! {
        @async { script { "a" } } @else { div {} }
    };
}
