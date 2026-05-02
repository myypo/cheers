use std::{
    collections::BTreeMap,
    ptr,
    sync::{LazyLock, Mutex, MutexGuard},
};

use crate::{
    context::Context,
    render::{Buffer, Rendered},
};

#[doc(hidden)]
/// Stream type used by generated async render code.
///
/// This is an implementation detail of the `html!` macro and is not part of
/// Cheers' stable public API.
pub type AsyncStream = ::std::pin::Pin<Box<dyn futures::Stream<Item = Rendered<String>> + Send>>;

type AsyncStreamCollections = BTreeMap<usize, Vec<Vec<AsyncStream>>>;

static ASYNC_STREAM_COLLECTIONS: LazyLock<Mutex<AsyncStreamCollections>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

fn async_stream_collections() -> MutexGuard<'static, AsyncStreamCollections> {
    match ASYNC_STREAM_COLLECTIONS.lock() {
        Ok(collections) => collections,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn async_stream_collection_key<C: Context>(buffer: &mut Buffer<C>) -> usize {
    ptr::from_mut(buffer).cast::<()>() as usize
}

fn pop_async_stream_collection(key: usize) -> Vec<AsyncStream> {
    let mut collections = async_stream_collections();
    let Some(stack) = collections.get_mut(&key) else {
        return Vec::new();
    };

    let streams = stack.pop().unwrap_or_else(Vec::new);
    if stack.is_empty() {
        collections.remove(&key);
    }

    streams
}

#[doc(hidden)]
/// Guard for a generated nested-async stream collection scope.
///
/// Dropping the guard without finishing discards the collected streams and
/// cleans up the internal sidecar registry.
#[must_use]
pub struct AsyncStreamCollectionGuard {
    key: usize,
    active: bool,
}

impl AsyncStreamCollectionGuard {
    #[doc(hidden)]
    #[inline]
    pub fn finish(mut self) -> Vec<AsyncStream> {
        self.active = false;
        pop_async_stream_collection(self.key)
    }
}

impl Drop for AsyncStreamCollectionGuard {
    #[inline]
    fn drop(&mut self) {
        if self.active {
            let _streams = pop_async_stream_collection(self.key);
        }
    }
}

#[doc(hidden)]
/// Starts collecting nested async streams rendered into `buffer`.
///
/// This is an implementation detail of the `html!` macro and is not part of
/// Cheers' stable public API.
#[inline]
pub fn enter<C: Context>(buffer: &mut Buffer<C>) -> AsyncStreamCollectionGuard {
    let key = async_stream_collection_key(buffer);
    async_stream_collections()
        .entry(key)
        .or_default()
        .push(Vec::new());

    AsyncStreamCollectionGuard { key, active: true }
}

#[doc(hidden)]
/// Pushes a nested async stream into the collection scope for `buffer`.
///
/// This is an implementation detail of the `html!` macro and is not part of
/// Cheers' stable public API.
#[inline]
pub fn push<C: Context>(buffer: &mut Buffer<C>, stream: AsyncStream) {
    let key = async_stream_collection_key(buffer);
    let mut collections = async_stream_collections();
    let Some(streams) = collections.get_mut(&key).and_then(|stack| stack.last_mut()) else {
        panic!("nested async stream pushed without active buffer async collector");
    };

    streams.push(stream);
}
