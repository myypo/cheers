mod root;
use root::{RootTemplate, Template};

use syn::{Error, LitStr};

pub struct Templates<'a> {
    pub root: RootTemplate<'a>,
    pub children: Vec<Template>,
}

pub fn process_templates<'a>(suspense: bool, path: &'a LitStr) -> Result<Templates<'a>, Error> {
    let root = RootTemplate::new(suspense, path)?;
    let children = root.all_content()?;

    Ok(Templates { root, children })
}
