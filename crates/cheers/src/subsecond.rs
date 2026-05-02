//! Subsecond integration helpers.

#[cfg(feature = "subsecond")]
mod imp {
    pub use dioxus_devtools::subsecond::{HotFn, HotFunction, call, register_handler};

    /// Returns whether the Subsecond integration should run.
    #[inline]
    pub const fn enabled() -> bool {
        cfg!(debug_assertions)
    }

    /// Runs a generated static render continuation through Subsecond.
    #[inline]
    pub fn hot_call<A, M, F>(f: F, args: A) -> F::Return
    where
        F: HotFunction<A, M>,
    {
        let mut hot = HotFn::current(f);
        hot.call(args)
    }

    /// Runs a generated one-argument render continuation through Subsecond.
    ///
    /// The explicit `FnMut(A)` bound keeps closure-argument types inferable for generated
    /// tuple-argument render bodies.
    #[inline]
    pub fn hot_call_with_arg<A, M, F>(f: F, arg: A) -> F::Return
    where
        F: FnMut(A) + HotFunction<(A,), M>,
    {
        hot_call(f, (arg,))
    }

    /// Connect this process to a Dioxus/Subsecond devserver when one is available.
    ///
    /// `cargo cheers subsecond` runs the app under `dx serve --hot-patch`, which sets
    /// the `DIOXUS_DEVSERVER_*` environment variables consumed by this function.
    /// Calling it when no devserver is present is harmless.
    pub fn connect() {
        dioxus_devtools::connect_subsecond();
    }
}

#[cfg(not(feature = "subsecond"))]
mod imp {
    /// Returns whether the Subsecond integration should run.
    #[inline]
    pub const fn enabled() -> bool {
        false
    }

    /// Runs a generated render body directly when Subsecond is not compiled in.
    #[inline]
    pub fn call<O>(mut f: impl FnMut() -> O) -> O {
        f()
    }

    /// Calls a generated static render continuation directly when Subsecond is not compiled in.
    #[inline]
    pub fn hot_call<A, O>(f: impl FnOnce(A) -> O, args: (A,)) -> O {
        f(args.0)
    }

    /// Calls a generated one-argument render continuation directly when Subsecond is not compiled in.
    #[inline]
    pub fn hot_call_with_arg<A, O>(mut f: impl FnMut(A) -> O, arg: A) -> O {
        f(arg)
    }

    /// Registers a callback that runs after a Subsecond patch is applied.
    #[inline]
    pub fn register_handler(_handler: std::sync::Arc<dyn Fn() + Send + Sync + 'static>) {}

    /// Does nothing when Subsecond is not compiled in.
    #[inline]
    pub fn connect() {}
}

pub use imp::*;
