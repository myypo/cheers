use cheers::prelude::*;

#[derive(Cheers)]
struct Counter {
    #[signal]
    count: i32,
}

impl Render for Counter {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let CounterSignals { signal_count } = self.signals();

        html! {
            div id=(signal_count) {}
        }
        .render_to(buffer);
    }
}

fn main() {}
