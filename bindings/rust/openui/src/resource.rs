//! [`create_resource`]: async data that follows a reactive source.

use crate::task::{spawn_local_unscoped, Task};
use crate::{create_effect, create_signal, Signal};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/// The latest result of a [`create_resource`] fetch, read reactively.
pub struct Resource<T: 'static, E: 'static> {
    value: Signal<Option<Result<T, E>>>,
    loading: Signal<bool>,
    version: Signal<u64>,
}

impl<T: 'static, E: 'static> Copy for Resource<T, E> {}

impl<T: 'static, E: 'static> Clone for Resource<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, E: 'static> std::fmt::Debug for Resource<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource")
            .field("value", &self.value)
            .field("loading", &self.loading)
            .finish()
    }
}

impl<T: Clone + 'static, E: Clone + 'static> Resource<T, E> {
    /// The latest completed fetch, or `None` before the first completes. A
    /// refetch keeps the previous result until the new one arrives.
    pub fn get(&self) -> Option<Result<T, E>> {
        self.value.get()
    }
}

impl<T: 'static, E: 'static> Resource<T, E> {
    /// Whether a fetch is running.
    pub fn loading(&self) -> bool {
        self.loading.get()
    }

    /// Fetch again with the current source.
    pub fn refetch(&self) {
        self.version.update(|version| *version += 1);
    }
}

/// Run `fetch(source())` on the UI thread now and whenever `source` changes
/// or [`Resource::refetch`] is called. Starting a fetch cancels the previous
/// one, and disposing the current scope cancels the running one, so a stale
/// or late result is never stored.
///
/// ```no_run
/// use openui::{create_resource, create_signal};
/// use openui::task::{spawn_blocking, time};
/// use std::time::Duration;
///
/// let path = create_signal("notes.txt".to_owned());
/// let text = create_resource(move || path.get(), |path| async move {
///     time::sleep(Duration::from_millis(10)).await;
///     spawn_blocking(move || std::fs::read_to_string(path).map_err(|error| error.to_string()))
///         .await
/// });
/// ```
pub fn create_resource<S, T, E, Fut>(
    source: impl Fn() -> S + 'static,
    fetch: impl Fn(S) -> Fut + 'static,
) -> Resource<T, E>
where
    S: 'static,
    T: 'static,
    E: 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let resource = Resource {
        value: create_signal(None),
        loading: create_signal(false),
        version: create_signal(0_u64),
    };
    // The effect owns the running fetch: replacing it, or disposing the
    // effect with its scope, drops the task and so cancels it.
    let running: Rc<RefCell<Option<Task<()>>>> = Rc::default();
    create_effect(move || {
        let input = source();
        let _ = resource.version.get();
        let future = fetch(input);
        let task = spawn_local_unscoped(async move {
            let result = future.await;
            resource.value.set(Some(result));
            resource.loading.set(false);
        });
        drop(running.replace(Some(task)));
        resource.loading.set(true);
    });
    resource
}
