use cheers::prelude::*;

#[derive(Cheers)]
struct BadSignal {
    #[signal]
    __proto__: String,
}

fn main() {}
