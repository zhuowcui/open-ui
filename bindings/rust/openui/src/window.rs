//! Windows: declaring them, and opening and closing them while the app runs.
//!
//! Every window has its own [`Document`](crate::Document) and root reactive
//! scope, disposed when the window closes. All windows share the UI thread's
//! reactive runtime, so one signal can drive views in several windows.
//!
//! Declare the first windows with [`AppSpec::window`](crate::AppSpec::window)
//! and open more from any event handler:
//!
//! ```no_run
//! use openui::window::{self, WindowKey, WindowSpec};
//!
//! let note = window::open(WindowSpec::new(WindowKey::unique()).title("Note"), || "Text");
//! note.set_title("Renamed");
//! ```
// Without a desktop runtime nothing applies queued requests.
#![cfg_attr(
    not(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    )),
    allow(dead_code)
)]

use crate::app::LogicalSize;
use crate::view_node::{IntoView, ViewNode};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Names a window. Opening a key that is already open focuses that window
/// instead, which suits singleton windows such as settings;
/// [`WindowKey::unique`] makes a fresh key for document windows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowKey(KeyName);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum KeyName {
    Named(String),
    Unique(u64),
}

impl WindowKey {
    pub fn new(name: impl Into<String>) -> Self {
        Self(KeyName::Named(name.into()))
    }

    /// A key no other window has.
    pub fn unique() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(KeyName::Unique(NEXT.fetch_add(1, Ordering::Relaxed)))
    }

    /// The name given to [`WindowKey::new`], or `None` for a unique key.
    pub fn name(&self) -> Option<&str> {
        match &self.0 {
            KeyName::Named(name) => Some(name),
            KeyName::Unique(_) => None,
        }
    }
}

impl From<&str> for WindowKey {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for WindowKey {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl fmt::Display for WindowKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            KeyName::Named(name) => formatter.write_str(name),
            KeyName::Unique(number) => write!(formatter, "#{number}"),
        }
    }
}

/// The answer to a request to close a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Close {
    Allow,
    /// Keep the window open, for example to ask about unsaved changes. Close
    /// it later with [`WindowHandle::close`].
    Prevent,
}

/// A window's key and native attributes.
#[derive(Clone)]
pub struct WindowSpec {
    pub(crate) key: WindowKey,
    pub(crate) title: Option<String>,
    pub(crate) size: LogicalSize,
    pub(crate) min_size: Option<LogicalSize>,
    pub(crate) max_size: Option<LogicalSize>,
    pub(crate) resizable: bool,
    pub(crate) decorations: bool,
    pub(crate) on_close_requested: Option<Rc<dyn Fn() -> Close>>,
}

impl WindowSpec {
    /// A resizable, decorated 800 × 600 window titled with the app's name.
    pub fn new(key: impl Into<WindowKey>) -> Self {
        Self {
            key: key.into(),
            title: None,
            size: LogicalSize::new(800.0, 600.0),
            min_size: None,
            max_size: None,
            resizable: true,
            decorations: true,
            on_close_requested: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Initial inner size in logical pixels.
    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.size = LogicalSize::new(width, height);
        self
    }

    pub fn min_size(mut self, width: f64, height: f64) -> Self {
        self.min_size = Some(LogicalSize::new(width, height));
        self
    }

    pub fn max_size(mut self, width: f64, height: f64) -> Self {
        self.max_size = Some(LogicalSize::new(width, height));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Whether the window has the system title bar and borders.
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Decide whether a close the user asked for goes ahead. Runs on the UI
    /// thread with [`current`] returning this window.
    pub fn on_close_requested(mut self, handler: impl Fn() -> Close + 'static) -> Self {
        self.on_close_requested = Some(Rc::new(handler));
        self
    }

    pub fn key(&self) -> &WindowKey {
        &self.key
    }
}

impl fmt::Debug for WindowSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowSpec")
            .field("key", &self.key)
            .field("title", &self.title)
            .field("size", &self.size)
            .field("min_size", &self.min_size)
            .field("max_size", &self.max_size)
            .field("resizable", &self.resizable)
            .field("decorations", &self.decorations)
            .field("on_close_requested", &self.on_close_requested.is_some())
            .finish()
    }
}

/// A window opened by [`open`] or declared with
/// [`AppSpec::window`](crate::AppSpec::window). Handles are cheap to clone
/// and stay valid after the window closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowHandle {
    key: WindowKey,
}

impl WindowHandle {
    pub fn key(&self) -> &WindowKey {
        &self.key
    }

    /// True from [`open`] until the window closes.
    pub fn is_open(&self) -> bool {
        with_registry(|registry| registry.windows.contains_key(&self.key))
    }

    /// Close the window without asking its close handler.
    pub fn close(&self) {
        with_registry(|registry| {
            if let Some(entry) = registry.windows.remove(&self.key) {
                registry.queue.push(Request::Close(entry.id));
            }
        });
    }

    pub fn focus(&self) {
        with_registry(|registry| {
            if let Some(entry) = registry.windows.get(&self.key) {
                registry.queue.push(Request::Focus(entry.id));
            }
        });
    }

    pub fn set_title(&self, title: impl Into<String>) {
        let title = title.into();
        with_registry(|registry| {
            if let Some(entry) = registry.windows.get(&self.key) {
                registry.queue.push(Request::SetTitle(entry.id, title));
            }
        });
    }
}

/// Open a window showing `view`, or focus the window already open with the
/// spec's key. The window appears when the UI thread next returns to the
/// event loop; before [`AppSpec::run`](crate::AppSpec::run) it opens when
/// the app starts.
pub fn open<V: IntoView>(spec: WindowSpec, view: impl FnOnce() -> V + 'static) -> WindowHandle {
    let key = spec.key.clone();
    with_registry(|registry| {
        if let Some(entry) = registry.windows.get(&key) {
            registry.queue.push(Request::Focus(entry.id));
            return;
        }
        let id = registry.next_id;
        registry.next_id += 1;
        registry.windows.insert(
            key.clone(),
            Entry {
                id,
                on_close_requested: spec.on_close_requested.clone(),
            },
        );
        registry.queue.push(Request::Open {
            id,
            spec,
            view: Box::new(move || view().into_view()),
        });
    });
    WindowHandle { key }
}

/// Register an existing app's window under a fresh key.
pub(crate) fn adopt(app: crate::App) -> WindowHandle {
    let key = WindowKey::unique();
    with_registry(|registry| {
        let id = registry.next_id;
        registry.next_id += 1;
        registry.windows.insert(
            key.clone(),
            Entry {
                id,
                on_close_requested: None,
            },
        );
        registry.queue.push(Request::Adopt {
            id,
            app: Box::new(app),
        });
    });
    WindowHandle { key }
}

/// The window whose event, view or close handler is running, if any.
pub fn current() -> Option<WindowHandle> {
    with_registry(|registry| registry.current.clone()).map(|key| WindowHandle { key })
}

/// Every open window, in the order they were opened.
pub fn all() -> Vec<WindowHandle> {
    with_registry(|registry| {
        let mut windows: Vec<_> = registry
            .windows
            .iter()
            .map(|(key, entry)| (entry.id, key.clone()))
            .collect();
        windows.sort_unstable_by_key(|(id, _)| *id);
        windows
            .into_iter()
            .map(|(_, key)| WindowHandle { key })
            .collect()
    })
}

pub(crate) type ViewFn = Box<dyn FnOnce() -> ViewNode>;

pub(crate) enum Request {
    Open {
        id: u64,
        spec: WindowSpec,
        view: ViewFn,
    },
    /// A window for an [`App`](crate::App) built by the caller, as
    /// [`App::run`](crate::App::run) does.
    Adopt {
        id: u64,
        app: Box<crate::App>,
    },
    Close(u64),
    Focus(u64),
    SetTitle(u64, String),
    #[cfg(feature = "menu")]
    SetMenu(u64, Vec<openui_platform::MenuEntry>),
}

struct Entry {
    id: u64,
    on_close_requested: Option<Rc<dyn Fn() -> Close>>,
}

/// The UI thread's windows: open or waiting to open, keyed by their key.
/// Requests wait here until the event loop applies them.
struct Registry {
    next_id: u64,
    windows: BTreeMap<WindowKey, Entry>,
    queue: Vec<Request>,
    current: Option<WindowKey>,
    exit: bool,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            next_id: 1,
            windows: BTreeMap::new(),
            queue: Vec::new(),
            current: None,
            exit: false,
        }
    }
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Borrow the registry briefly. Never call app code inside `operation`: it
/// may open or close windows itself.
fn with_registry<R>(operation: impl FnOnce(&mut Registry) -> R) -> R {
    REGISTRY.with(|registry| operation(&mut registry.borrow_mut()))
}

pub(crate) fn take_requests() -> Vec<Request> {
    with_registry(|registry| std::mem::take(&mut registry.queue))
}

/// The runtime id of the window with `key`, if it is open or opening.
#[cfg_attr(not(feature = "dialog"), allow(dead_code))]
pub(crate) fn id_of(key: &WindowKey) -> Option<u64> {
    with_registry(|registry| registry.windows.get(key).map(|entry| entry.id))
}

pub(crate) fn key_of(id: u64) -> Option<WindowKey> {
    with_registry(|registry| {
        registry
            .windows
            .iter()
            .find(|(_, entry)| entry.id == id)
            .map(|(key, _)| key.clone())
    })
}

/// Ask a window's close handler, then forget the window if it may close.
pub(crate) fn close_requested(id: u64) -> bool {
    let Some(key) = key_of(id) else {
        return true;
    };
    let handler = with_registry(|registry| {
        registry
            .windows
            .get(&key)
            .and_then(|entry| entry.on_close_requested.clone())
    });
    let close = match handler {
        Some(handler) => with_current(&key, || handler()),
        None => Close::Allow,
    };
    if close == Close::Allow {
        forget(id);
    }
    close == Close::Allow
}

/// Show `entries` as window `id`'s native menu bar.
#[cfg(feature = "menu")]
pub(crate) fn request_menu(id: u64, entries: Vec<openui_platform::MenuEntry>) {
    with_registry(|registry| registry.queue.push(Request::SetMenu(id, entries)));
}

/// Close the window with `key` as its close button would: its close handler
/// may keep it open.
#[cfg(feature = "menu")]
pub(crate) fn request_close(key: &WindowKey) {
    let Some(id) = id_of(key) else {
        return;
    };
    if close_requested(id) {
        with_registry(|registry| registry.queue.push(Request::Close(id)));
    }
}

/// Forget a window the event loop closed.
pub(crate) fn forget(id: u64) {
    with_registry(|registry| registry.windows.retain(|_, entry| entry.id != id));
}

pub(crate) fn has_windows() -> bool {
    with_registry(|registry| !registry.windows.is_empty())
}

pub(crate) fn request_exit() {
    with_registry(|registry| registry.exit = true);
}

pub(crate) fn exit_requested() -> bool {
    with_registry(|registry| registry.exit)
}

/// Clear every window and request after the event loop ends, so a later
/// run on this thread starts empty.
pub(crate) fn reset() {
    let old = with_registry(std::mem::take);
    drop(old);
}

/// Run `operation` with [`current`] returning `key`.
pub(crate) fn with_current<R>(key: &WindowKey, operation: impl FnOnce() -> R) -> R {
    struct Restore(Option<WindowKey>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            with_registry(|registry| registry.current = previous);
        }
    }
    let previous = with_registry(|registry| registry.current.replace(key.clone()));
    let _restore = Restore(previous);
    operation()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(requests: &[Request]) -> Vec<u64> {
        requests
            .iter()
            .filter_map(|request| match request {
                Request::Open { id, .. } => Some(*id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn opening_an_open_key_focuses_it_and_unique_keys_differ() {
        reset();
        let first = open(WindowSpec::new("settings"), || "a");
        let again = open(WindowSpec::new("settings"), || "b");
        assert_eq!(first, again);
        assert_ne!(WindowKey::unique(), WindowKey::unique());
        let other = open(WindowSpec::new(WindowKey::unique()), || "c");
        let requests = take_requests();
        assert_eq!(opened(&requests).len(), 2);
        assert!(
            matches!(requests[1], Request::Focus(id) if Some(id) == opened(&requests).first().copied())
        );
        assert_eq!(all(), [first, other]);
        reset();
    }

    #[test]
    fn closing_forgets_the_window_and_queues_one_close() {
        reset();
        let handle = open(WindowSpec::new("main"), || "a");
        assert!(handle.is_open() && has_windows());
        handle.close();
        handle.close();
        handle.set_title("ignored");
        assert!(!handle.is_open() && !has_windows());
        let requests = take_requests();
        assert_eq!(requests.len(), 2);
        assert!(matches!(requests[1], Request::Close(_)));
        reset();
    }

    #[test]
    fn close_handlers_can_prevent_a_close_and_see_their_window() {
        reset();
        let seen = Rc::new(RefCell::new(None));
        let record = seen.clone();
        let handle = open(
            WindowSpec::new("doc").on_close_requested(move || {
                *record.borrow_mut() = current().map(|window| window.key().clone());
                Close::Prevent
            }),
            || "a",
        );
        let id = opened(&take_requests())[0];
        assert!(!close_requested(id));
        assert!(handle.is_open());
        assert_eq!(*seen.borrow(), Some(WindowKey::new("doc")));
        assert_eq!(current(), None);
        reset();
    }

    #[test]
    fn exit_requests_are_cleared_by_reset() {
        reset();
        request_exit();
        assert!(exit_requested());
        reset();
        assert!(!exit_requested());
    }
}
