use syn::{Error, LitStr};

use crate::askama_config::{ASKAMA_CONFIG, ReadTemplate};

const STREAMING_SSR_SCRIPT: &str = include_str!("./streaming-ssr-script.html");
const LIVE_RELOAD_SCRIPT: &str = include_str!("./live-reload-script.html");

fn inject_script(path: &LitStr, content: &mut String, script: &str) -> Result<(), Error> {
    let pos = content.rfind("<!-- inject-crabstar -->")
    .or_else(|| content.rfind("</body>"))
    .ok_or_else(|| Error::new_spanned(
        path,
        "Page template must either contain a visible closing </body> tag or explicitly state where to inject scripts with '<!-- inject-crabstar -->' comment",
    ))?;

    content.insert_str(pos, script);
    Ok(())
}

pub fn template_with_scripts(suspense: bool, path: &LitStr) -> Result<String, Error> {
    let path_str = path.value();

    let ReadTemplate { mut content, .. } = ASKAMA_CONFIG.read_template(path, &path_str)?;

    if suspense {
        // TODO: move this to assets router
        inject_script(path, &mut content, STREAMING_SSR_SCRIPT)?;
    }
    if cfg!(debug_assertions) {
        inject_script(path, &mut content, LIVE_RELOAD_SCRIPT)?;
    }

    ASKAMA_CONFIG.write_template(&path_str, content.clone());

    Ok(content)
}
