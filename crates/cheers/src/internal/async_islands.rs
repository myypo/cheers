use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

type Renderer = Box<dyn FnMut() -> String + Send>;

static ASYNC_ISLANDS: OnceLock<Mutex<HashMap<String, Renderer>>> = OnceLock::new();

fn async_islands() -> &'static Mutex<HashMap<String, Renderer>> {
    ASYNC_ISLANDS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[inline]
pub const fn enabled() -> bool {
    crate::subsecond::enabled()
}

pub fn register(key: impl Into<String>, renderer: impl FnMut() -> String + Send + 'static) {
    if !enabled() {
        return;
    }

    let mut islands = async_islands().lock().expect("async island cache poisoned");
    islands.insert(key.into(), Box::new(renderer));
}

pub fn render(keys: &[String]) -> Vec<(String, String)> {
    if !enabled() {
        return Vec::new();
    }

    let mut islands = async_islands().lock().expect("async island cache poisoned");
    keys.iter()
        .filter_map(|key| {
            islands
                .get_mut(key)
                .map(|renderer| (key.clone(), renderer()))
        })
        .collect()
}
