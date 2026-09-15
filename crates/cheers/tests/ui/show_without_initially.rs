use cheers::prelude::*;

#[derive(Cheers)]
struct Details {
    #[signal]
    open: bool,
}

impl Render for Details {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let DetailsSignals { signal_open } = self.signals();

        html! {
            div !show(signal_open) { "Details" }
        }
        .render_to(buffer);
    }
}

fn main() {}
