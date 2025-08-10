use std::time::Duration;

use axum::Router;
use memory_serve::MemoryServe;
use tower_livereload::LiveReloadLayer;

pub trait CrabstarRouterExt {
    fn serve_crabstar_application(self) -> Self;
}

fn memory_router() -> Router {
    let memory_router = MemoryServe::from_env();
    #[cfg(debug_assertions)]
    let memory_router = memory_router.cache_control(memory_serve::CacheControl::NoCache);
    memory_router.into_router()
}

fn livereload_layer() -> tower_livereload::LiveReloadLayer {
    LiveReloadLayer::new().reload_interval(Duration::from_millis(50))
}

impl<S> CrabstarRouterExt for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn serve_crabstar_application(self) -> Self {
        let router = self.nest_service("/static", memory_router());

        #[cfg(debug_assertions)]
        let router = router.layer(livereload_layer());

        router
    }
}
