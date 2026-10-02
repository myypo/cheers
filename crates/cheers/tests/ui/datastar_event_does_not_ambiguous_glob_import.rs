#![deny(ambiguous_glob_imports)]

use cheers::prelude::*;

cheers::define_events! { saved }

fn main() {
    let _ = html! {
        form !on:saved("console.log('saved')") {
            input !on:input("console.log('input')");
            button !on:click("console.log('ok')") { "Save" }
        }
    };
}
