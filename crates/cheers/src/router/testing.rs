use axum::{Router, routing::MethodRouter};

pub struct MockedAction<S> {
    pub(crate) path: &'static str,
    pub(crate) route: MethodRouter<S>,
}

impl<S> MockedAction<S> {
    pub fn new(path: &'static str, route: MethodRouter<S>) -> Self {
        Self { path, route }
    }
}

pub trait RouterExt<S>: Sized {
    fn mock_action(self, mocked: MockedAction<S>) -> Self;
}

impl<S: Clone + Send + Sync + 'static> RouterExt<S> for Router<S> {
    fn mock_action(self, mocked: MockedAction<S>) -> Self {
        self.route(mocked.path, mocked.route)
    }
}
