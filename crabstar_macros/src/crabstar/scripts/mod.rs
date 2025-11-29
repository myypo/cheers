use crate::crabstar::CrabstarArgs;

const STREAMING_SSR_SCRIPT: &str = include_str!("./streaming-ssr-script.html");
const LIVE_RELOAD_SCRIPT: &str = include_str!("./live-reload-script.html");

// FIXME: super scuffed, for some reason the askama CACHE inits my paths twice
fn inject_script(source: &mut String, script: &str) {
    let Some(pos) = source.rfind("<!-- crabstar: inject_scripts() -->") else {
        return;
    };

    source.insert_str(pos, script);
}

pub fn inject_scripts(CrabstarArgs { suspense, page }: &CrabstarArgs, source: &mut String) {
    if page.is_none() {
        return;
    }

    if !suspense.is_empty() {
        // TODO: inject it into datastar bundle?
        inject_script(source, STREAMING_SSR_SCRIPT);
    }
    if cfg!(debug_assertions) {
        inject_script(source, LIVE_RELOAD_SCRIPT);
    }

    *source = source.replace("<!-- crabstar: inject_scripts() -->", "");
}
