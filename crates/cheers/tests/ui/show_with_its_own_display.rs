use cheers::prelude::*;

#[derive(Cheers)]
struct Panel {
    #[signal]
    open: bool,
}

impl Render for Panel {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let PanelSignals { signal_open } = self.signals();

        html! {
            div style="display:flex" !show(signal_open, initially: false) { "Panel" }
            div style="DISPLAY:flex" !show(signal_open, initially: false) { "Shouty" }
        }
        .render_to(buffer);
    }
}

fn main() {}
