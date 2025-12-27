use axum::{Router, http::Method};

pub trait Action<S> {
    const PATH: &str;
    const METHOD: Method;

    fn router(&self) -> Router<S>;
}
