use crate::types::{OuiEventCallback, OuiStatus, OuiUtf8};
use openui_engine::{
    AccessibilityNode, AccessibilityNodeId, AnimationEvent, Engine, EngineError,
    FontCollectionError, FontFaceHandle, NodeHandle,
};
use openui_style::{ImageResourceId, StyleProperty, StyleValue};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandleKind {
    App,
    Document,
    Element,
    Compound,
    Resource,
    FontFace,
    Listener,
    Buffer,
    AccessibilitySnapshot,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub document: Rc<DocumentState>,
    #[cfg(all(feature = "linux", target_os = "linux"))]
    pub title: String,
    #[cfg(all(feature = "linux", target_os = "linux"))]
    pub backend: u32,
    pub running: Cell<bool>,
    pub has_run: Cell<bool>,
    pub exit_requested: Cell<bool>,
    pub exit_handle: RefCell<Option<openui::AppExitHandle>>,
}

pub(crate) struct DocumentState {
    pub engine: Rc<RefCell<Engine>>,
    pub native: openui::Document,
    pub update_depth: Cell<u32>,
    pub listeners: RefCell<Vec<ListenerRecord>>,
    pub next_listener: Cell<u64>,
    pub element_handles: RefCell<HashMap<NodeHandle, Vec<usize>>>,
    pub animation_events: RefCell<Vec<AnimationEvent>>,
}

#[derive(Clone)]
pub(crate) struct ElementRef {
    pub document: Weak<DocumentState>,
    pub node: NodeHandle,
}

#[derive(Clone)]
pub(crate) struct ResourceRef {
    pub document: Weak<DocumentState>,
    pub resource: ImageResourceId,
}

#[derive(Clone)]
pub(crate) struct FontFaceRef {
    pub document: Weak<DocumentState>,
    pub face: FontFaceHandle,
}

#[derive(Clone)]
pub(crate) struct ListenerRef {
    pub document: Weak<DocumentState>,
    pub id: u64,
}

#[derive(Clone)]
pub(crate) struct ListenerRecord {
    pub id: u64,
    pub node: NodeHandle,
    pub event_type: u32,
    pub capture: bool,
    pub callback: OuiEventCallback,
    pub user_data: *mut c_void,
    pub element_address: usize,
}

#[derive(Clone)]
pub(crate) struct AccessibilitySnapshotState {
    pub document: Weak<DocumentState>,
    pub generation: u64,
    pub focus_id: u64,
    pub reduced_motion: bool,
    pub nodes: Vec<(AccessibilityNodeId, AccessibilityNode)>,
    pub changed: Vec<u64>,
    pub removed: Vec<u64>,
    pub full_tree: bool,
}

#[derive(Clone)]
pub(crate) enum LocalHandle {
    App(Rc<AppState>),
    Document(Rc<DocumentState>),
    Element(ElementRef),
    Compound(StyleValue),
    PropertyCompound(StyleProperty, StyleValue),
    Resource(ResourceRef),
    FontFace(FontFaceRef),
    Listener(ListenerRef),
    Buffer(Rc<Vec<u8>>),
    AccessibilitySnapshot(Rc<AccessibilitySnapshotState>),
}

impl LocalHandle {
    fn kind(&self) -> HandleKind {
        match self {
            Self::App(_) => HandleKind::App,
            Self::Document(_) => HandleKind::Document,
            Self::Element(_) => HandleKind::Element,
            Self::Compound(_) | Self::PropertyCompound(..) => HandleKind::Compound,
            Self::Resource(_) => HandleKind::Resource,
            Self::FontFace(_) => HandleKind::FontFace,
            Self::Listener(_) => HandleKind::Listener,
            Self::Buffer(_) => HandleKind::Buffer,
            Self::AccessibilitySnapshot(_) => HandleKind::AccessibilitySnapshot,
        }
    }
}

#[derive(Clone)]
struct GlobalHandle {
    thread: ThreadId,
    kind: HandleKind,
}

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(0x1_0000);
static GLOBAL_HANDLES: OnceLock<Mutex<HashMap<usize, GlobalHandle>>> = OnceLock::new();

thread_local! {
    static LOCAL_HANDLES: RefCell<HashMap<usize, LocalHandle>> = RefCell::new(HashMap::new());
    static LAST_ERROR: RefCell<LastError> = RefCell::new(LastError::default());
}

fn global_handles() -> &'static Mutex<HashMap<usize, GlobalHandle>> {
    GLOBAL_HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone)]
pub(crate) struct ApiError {
    pub status: OuiStatus,
    pub detail: u32,
    pub message: String,
}

impl ApiError {
    pub fn new(status: OuiStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            detail: 0,
            message: message.into(),
        }
    }

    pub fn detail(mut self, detail: u32) -> Self {
        self.detail = detail;
        self
    }
}

impl From<EngineError> for ApiError {
    fn from(value: EngineError) -> Self {
        let status = match value {
            EngineError::WrongDocument => OuiStatus::WrongDocument,
            EngineError::StaleHandle => OuiStatus::StaleHandle,
            EngineError::PropertyType { .. } => OuiStatus::WrongValueType,
            EngineError::InvalidViewport => OuiStatus::InvalidArgument,
            EngineError::Font(FontCollectionError::AllocationFailed) => OuiStatus::OutOfMemory,
            EngineError::Font(FontCollectionError::WrongCollection) => OuiStatus::WrongDocument,
            EngineError::Font(FontCollectionError::UnknownFace) => OuiStatus::StaleHandle,
            EngineError::Font(_) => OuiStatus::InvalidArgument,
            _ => OuiStatus::InvalidState,
        };
        Self::new(status, value.to_string())
    }
}

impl From<openui_compositor::CompositorError> for ApiError {
    fn from(value: openui_compositor::CompositorError) -> Self {
        Self::new(OuiStatus::Internal, value.to_string())
    }
}

#[derive(Default, Clone)]
pub(crate) struct LastError {
    pub status: i32,
    pub detail: u32,
    pub message: Vec<u8>,
}

pub(crate) fn last_error() -> LastError {
    LAST_ERROR.with(|slot| slot.borrow().clone())
}

fn set_error(error: &ApiError) {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = LastError {
            status: error.status as i32,
            detail: error.detail,
            message: error.message.as_bytes().to_vec(),
        };
    });
}

pub(crate) fn ffi_preserve(operation: impl FnOnce() -> Result<(), ApiError>) -> OuiStatus {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => OuiStatus::Ok,
        Ok(Err(error)) => {
            set_error(&error);
            error.status
        }
        Err(_) => {
            let error = ApiError::new(OuiStatus::Internal, "panic contained at C ABI boundary");
            set_error(&error);
            error.status
        }
    }
}

fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = LastError::default());
}

pub(crate) fn ffi(operation: impl FnOnce() -> Result<(), ApiError>) -> OuiStatus {
    clear_error();
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => OuiStatus::Ok,
        Ok(Err(error)) => {
            set_error(&error);
            error.status
        }
        Err(_) => {
            let error = ApiError::new(OuiStatus::Internal, "panic contained at C ABI boundary");
            set_error(&error);
            error.status
        }
    }
}

pub(crate) fn ffi_value<T: Copy>(
    fallback: T,
    operation: impl FnOnce() -> Result<T, ApiError>,
) -> T {
    clear_error();
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            set_error(&error);
            fallback
        }
        Err(_) => {
            set_error(&ApiError::new(
                OuiStatus::Internal,
                "panic contained at C ABI boundary",
            ));
            fallback
        }
    }
}

pub(crate) fn register(handle: LocalHandle) -> Result<usize, ApiError> {
    let token = NEXT_TOKEN.fetch_add(16, Ordering::Relaxed);
    if token > isize::MAX as usize - 16 {
        return Err(ApiError::new(
            OuiStatus::OutOfMemory,
            "opaque handle space exhausted",
        ));
    }
    let kind = handle.kind();
    LOCAL_HANDLES.with(|handles| {
        handles.borrow_mut().insert(token, handle);
    });
    global_handles()
        .lock()
        .map_err(|_| ApiError::new(OuiStatus::Internal, "handle registry is poisoned"))?
        .insert(
            token,
            GlobalHandle {
                thread: std::thread::current().id(),
                kind,
            },
        );
    Ok(token)
}

fn validate(address: usize, expected: HandleKind) -> Result<(), ApiError> {
    if address == 0 {
        return Err(ApiError::new(OuiStatus::InvalidArgument, "null handle"));
    }
    let handles = global_handles()
        .lock()
        .map_err(|_| ApiError::new(OuiStatus::Internal, "handle registry is poisoned"))?;
    let metadata = handles
        .get(&address)
        .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "unknown or destroyed handle"))?;
    if metadata.thread != std::thread::current().id() {
        return Err(ApiError::new(
            OuiStatus::WrongThread,
            "handle used from a thread other than its owner",
        ));
    }
    if metadata.kind != expected {
        return Err(ApiError::new(
            OuiStatus::InvalidHandle,
            "opaque handle has the wrong type",
        ));
    }
    Ok(())
}

pub(crate) fn get(address: usize, expected: HandleKind) -> Result<LocalHandle, ApiError> {
    validate(address, expected)?;
    LOCAL_HANDLES.with(|handles| {
        handles
            .try_borrow()
            .map_err(|_| ApiError::new(OuiStatus::Reentrant, "handle registry is borrowed"))?
            .get(&address)
            .cloned()
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "handle is unavailable"))
    })
}

pub(crate) fn destroy(address: usize, expected: HandleKind) -> Result<LocalHandle, ApiError> {
    validate(address, expected)?;
    let removed = LOCAL_HANDLES.with(|handles| {
        handles
            .try_borrow_mut()
            .map_err(|_| ApiError::new(OuiStatus::Reentrant, "handle registry is borrowed"))?
            .remove(&address)
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "handle is unavailable"))
    })?;
    global_handles()
        .lock()
        .map_err(|_| ApiError::new(OuiStatus::Internal, "handle registry is poisoned"))?
        .remove(&address);
    Ok(removed)
}

#[cfg(test)]
mod miri_handle_tests {
    use super::*;

    #[test]
    fn opaque_handles_reject_wrong_kind_thread_and_reuse() {
        let first = register(LocalHandle::Buffer(Rc::new(vec![1, 2, 3]))).unwrap();
        assert_eq!(
            get(first, HandleKind::Document).err().unwrap().status,
            OuiStatus::InvalidHandle
        );
        assert_eq!(
            std::thread::spawn(move || get(first, HandleKind::Buffer).err().unwrap().status)
                .join()
                .unwrap(),
            OuiStatus::WrongThread
        );
        assert!(matches!(
            get(first, HandleKind::Buffer),
            Ok(LocalHandle::Buffer(_))
        ));
        destroy(first, HandleKind::Buffer).unwrap_or_else(|error| panic!("{}", error.message));
        assert_eq!(
            get(first, HandleKind::Buffer).err().unwrap().status,
            OuiStatus::InvalidHandle
        );
        let second = register(LocalHandle::Buffer(Rc::new(vec![4]))).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            get(first, HandleKind::Buffer).err().unwrap().status,
            OuiStatus::InvalidHandle
        );
        destroy(second, HandleKind::Buffer).unwrap_or_else(|error| panic!("{}", error.message));
    }
}

pub(crate) fn document(address: usize) -> Result<Rc<DocumentState>, ApiError> {
    match get(address, HandleKind::Document)? {
        LocalHandle::Document(value) => Ok(value),
        _ => unreachable!("kind checked by registry"),
    }
}

pub(crate) fn element(address: usize) -> Result<ElementRef, ApiError> {
    match get(address, HandleKind::Element)? {
        LocalHandle::Element(value) => Ok(value),
        _ => unreachable!("kind checked by registry"),
    }
}

pub(crate) fn element_document(value: &ElementRef) -> Result<Rc<DocumentState>, ApiError> {
    value
        .document
        .upgrade()
        .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "owning document was destroyed"))
}

pub(crate) fn utf8(value: OuiUtf8, label: &'static str) -> Result<String, ApiError> {
    if value.length == 0 {
        return Ok(String::new());
    }
    if value.data.is_null() || value.length > isize::MAX as usize {
        return Err(ApiError::new(
            OuiStatus::InvalidArgument,
            format!("invalid {label} UTF-8 slice"),
        ));
    }
    // SAFETY: the C contract requires `data` to reference `length` readable
    // bytes for the duration of this call; null and excessive lengths were
    // rejected above. The bytes are copied before returning.
    let bytes = unsafe { std::slice::from_raw_parts(value.data, value.length) };
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ApiError::new(
            OuiStatus::InvalidArgument,
            format!("{label} is not valid UTF-8"),
        )
    })?;
    Ok(text.to_owned())
}

pub(crate) fn bytes(data: *const u8, length: usize) -> Result<Vec<u8>, ApiError> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if data.is_null() || length > isize::MAX as usize {
        return Err(ApiError::new(
            OuiStatus::InvalidArgument,
            "invalid byte slice",
        ));
    }
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(length)
        .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "byte allocation failed"))?;
    // SAFETY: the C contract requires `data` to reference `length` readable
    // bytes for the duration of this call. The range is copied immediately.
    owned.extend_from_slice(unsafe { std::slice::from_raw_parts(data, length) });
    Ok(owned)
}

pub(crate) fn borrow_engine(state: &DocumentState) -> Result<std::cell::Ref<'_, Engine>, ApiError> {
    state
        .engine
        .try_borrow()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "document is mutably borrowed"))
}

pub(crate) fn borrow_engine_mut(
    state: &DocumentState,
) -> Result<std::cell::RefMut<'_, Engine>, ApiError> {
    state
        .engine
        .try_borrow_mut()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "document is already borrowed"))
}
