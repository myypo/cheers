#![deny(ambiguous_glob_imports)]

use cheers::prelude::*;

fn main() {
    let _ = html! {
        form {
            input;
            button !on:click("console.log('ok')") { "Save" }
        }
    };
}
