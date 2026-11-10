//! Feature-gated native platform runtime.

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
mod desktop;

#[cfg(all(feature = "windows", target_os = "windows"))]
mod windows_clipboard;

#[cfg(all(feature = "dialog", any(target_os = "linux", target_os = "windows")))]
pub mod dialog;

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
pub use desktop::{run, run_windows};

#[cfg(any(feature = "linux", feature = "windows"))]
pub use accesskit::{ActionRequest, TreeUpdate};

use openui_geometry::ViewportMetrics;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendPreference {
    #[default]
    Auto,
    OpenGl,
    Software,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendStatus {
    pub active: BackendPreference,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowOptions {
    pub title: String,
    /// Initial inner size in logical pixels.
    pub width: f64,
    pub height: f64,
    pub backend: BackendPreference,
    /// Smallest and largest inner size in logical pixels.
    pub min_size: Option<(f64, f64)>,
    pub max_size: Option<(f64, f64)>,
    pub resizable: bool,
    pub decorations: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: String::new(),
            width: 800.0,
            height: 600.0,
            backend: BackendPreference::Auto,
            min_size: None,
            max_size: None,
            resizable: true,
            decorations: true,
        }
    }
}

/// Names a window for the life of the process. The controller chooses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

/// A change to the set of open windows, queued by a [`WindowController`].
pub enum WindowCommand {
    /// Open a window drawn by `application`. If `id` is already open, focus
    /// that window and drop `application`.
    Open {
        id: WindowId,
        options: WindowOptions,
        application: Box<dyn PlatformApplication>,
    },
    /// Close a window without asking its controller.
    Close(WindowId),
    Focus(WindowId),
    SetTitle(WindowId, String),
}

impl std::fmt::Debug for WindowCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { id, options, .. } => formatter
                .debug_struct("Open")
                .field("id", id)
                .field("options", options)
                .finish_non_exhaustive(),
            Self::Close(id) => formatter.debug_tuple("Close").field(id).finish(),
            Self::Focus(id) => formatter.debug_tuple("Focus").field(id).finish(),
            Self::SetTitle(id, title) => formatter
                .debug_tuple("SetTitle")
                .field(id)
                .field(title)
                .finish(),
        }
    }
}

/// Wakes the event loop from any thread so it runs queued UI tasks
/// ([`WindowController::run_tasks`]). Wakes coalesce until the loop handles
/// one.
#[derive(Clone)]
pub struct LoopWaker(std::sync::Arc<dyn Fn() + Send + Sync>);

impl LoopWaker {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self(std::sync::Arc::new(wake))
    }

    pub fn wake(&self) {
        (self.0)();
    }
}

impl std::fmt::Debug for LoopWaker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LoopWaker")
    }
}

/// A native window's handle, which dialogs use as their owner. It can be
/// sent to other threads; it does not keep the window open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeWindow(NativeHandle);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeHandle {
    Win32(isize),
    X11(u64),
    Other,
}

#[cfg_attr(not(feature = "dialog"), allow(dead_code))]
impl NativeWindow {
    #[cfg(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    ))]
    pub(crate) fn from_raw(handle: raw_window_handle::RawWindowHandle) -> Self {
        use raw_window_handle::RawWindowHandle;
        Self(match handle {
            RawWindowHandle::Win32(handle) => NativeHandle::Win32(handle.hwnd.get()),
            RawWindowHandle::Xlib(handle) => NativeHandle::X11(handle.window as u64),
            RawWindowHandle::Xcb(handle) => NativeHandle::X11(u64::from(handle.window.get())),
            _ => NativeHandle::Other,
        })
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn win32(self) -> Option<isize> {
        match self.0 {
            NativeHandle::Win32(hwnd) => Some(hwnd),
            _ => None,
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn x11(self) -> Option<u64> {
        match self.0 {
            NativeHandle::X11(window) => Some(window),
            _ => None,
        }
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn x11_for_test(window: u64) -> Self {
        Self(NativeHandle::X11(window))
    }
}

/// The system clipboard's text while a native runtime runs. Use it on the
/// event-loop thread; it is not `Send`.
#[derive(Clone)]
pub struct Clipboard(std::rc::Weak<dyn ClipboardBackend>);

pub(crate) trait ClipboardBackend {
    fn load(&self) -> Result<String, PlatformError>;
    fn store(&self, text: String);
}

impl Clipboard {
    #[cfg_attr(
        not(any(
            all(feature = "linux", target_os = "linux"),
            all(feature = "windows", target_os = "windows")
        )),
        allow(dead_code)
    )]
    pub(crate) fn new(backend: std::rc::Weak<dyn ClipboardBackend>) -> Self {
        Self(backend)
    }

    /// The clipboard's text. Fails if it holds no text, another program is
    /// using it, or the runtime has ended.
    pub fn read_text(&self) -> Result<String, PlatformError> {
        self.0.upgrade().ok_or(PlatformError::Unavailable)?.load()
    }

    /// Replace the clipboard's contents with `text`. Does nothing once the
    /// runtime has ended.
    pub fn write_text(&self, text: impl Into<String>) {
        if let Some(backend) = self.0.upgrade() {
            backend.store(text.into());
        }
    }
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Clipboard")
    }
}

/// Owns the set of windows in [`run_windows`]. Every method runs on the
/// event-loop thread between window events, never inside one.
pub trait WindowController: 'static {
    /// Commands queued since the last call, applied in order. Called when
    /// the loop starts, after every event and before the loop waits, and
    /// again until it returns none, so return each command only once.
    fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String>;

    /// Called once, before any window opens, with a waker for this loop.
    fn started(&mut self, _waker: LoopWaker) {}

    /// Called once, before any window opens, with the system clipboard.
    fn clipboard(&mut self, _clipboard: Clipboard) {}

    /// `id` opened; `native` is its handle for parenting native dialogs.
    fn window_opened(&mut self, _id: WindowId, _native: NativeWindow) {}

    /// Run queued UI tasks for one bounded turn and return whether more are
    /// queued. Called after a [`LoopWaker::wake`] and before the loop waits.
    fn run_tasks(&mut self) -> Result<bool, String> {
        Ok(false)
    }

    /// The user asked to close `id`; its application has already received
    /// [`PlatformEvent::CloseRequested`]. Return false to keep it open.
    fn close_requested(&mut self, _id: WindowId) -> Result<bool, String> {
        Ok(true)
    }

    /// `id` was destroyed and its application dropped.
    fn window_closed(&mut self, _id: WindowId) -> Result<(), String> {
        Ok(())
    }

    /// Whether to end the loop with `open_windows` windows open. Any
    /// window's [`PlatformApplication::exit_requested`] also ends it.
    fn exit_requested(&self, open_windows: usize) -> bool {
        open_windows == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPhase {
    Move,
    Down,
    Up,
    Cancel,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorIcon {
    #[default]
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPhase {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    BackendChanged(BackendStatus),
    Resized(ViewportMetrics),
    Pointer {
        pointer_id: u64,
        phase: PointerPhase,
        x: f32,
        y: f32,
        button: PointerButton,
        modifiers: Modifiers,
    },
    Wheel {
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        modifiers: Modifiers,
    },
    Key {
        phase: KeyPhase,
        key_code: i32,
        text: Option<String>,
        modifiers: Modifiers,
        repeat: bool,
    },
    TextInput(String),
    CompositionStart,
    CompositionUpdate(String),
    CompositionEnd(String),
    Focused(bool),
    DroppedFile(PathBuf),
    HoveredFile(PathBuf),
    HoveredFileCancelled,
    Presented {
        frame_number: u64,
        time_ms: f64,
    },
    CloseRequested,
}

/// A native key notification and its separately supplied committed text.
/// Logical key names such as `Escape` must never become text input.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardInput {
    pub phase: KeyPhase,
    pub key_code: i32,
    pub key_text: Option<String>,
    pub text: Option<String>,
    pub modifiers: Modifiers,
    pub repeat: bool,
}

impl KeyboardInput {
    pub fn key_event(&self) -> PlatformEvent {
        PlatformEvent::Key {
            phase: self.phase,
            key_code: self.key_code,
            text: self.key_text.clone(),
            modifiers: self.modifiers,
            repeat: self.repeat,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoftwareFrame {
    pub width: u32,
    pub height: u32,
    pub viewport: ViewportMetrics,
    pub stride: usize,
    pub pixels: Vec<u8>,
}

/// Application-side contract used by the owned native event loop.
pub trait PlatformApplication: 'static {
    fn event(&mut self, event: PlatformEvent) -> Result<(), String>;
    fn render(&mut self, time_ms: f64) -> Result<SoftwareFrame, String>;
    fn exit_requested(&self) -> bool;

    /// The default preserves the legacy key notification. Implementations with
    /// cancellable keyboard defaults can process logical keys and committed
    /// text separately; Open UI's Rust App implements that shared path.
    fn key_input(&mut self, input: KeyboardInput) -> Result<(), String> {
        self.event(PlatformEvent::Key {
            phase: input.phase,
            key_code: input.key_code,
            text: input.text.or(input.key_text),
            modifiers: input.modifiers,
            repeat: input.repeat,
        })
    }

    /// Run one bounded owning-thread native task turn. Return true when
    /// another turn is pending. The default preserves existing applications.
    fn dispatch_pending_events(&mut self) -> Result<bool, String> {
        Ok(false)
    }

    fn clipboard_text(&self) -> Result<String, String> {
        Ok(String::new())
    }

    fn set_clipboard_text(&mut self, _text: String) -> Result<(), String> {
        Ok(())
    }

    fn is_animating(&self) -> bool {
        false
    }

    fn needs_redraw(&self) -> bool {
        false
    }

    fn cursor_icon(&mut self, _x: f32, _y: f32) -> Result<CursorIcon, String> {
        Ok(CursorIcon::Default)
    }

    #[cfg(any(feature = "linux", feature = "windows"))]
    fn accessibility_update(&mut self) -> Result<TreeUpdate, String>;

    #[cfg(any(feature = "linux", feature = "windows"))]
    fn accessibility_action(&mut self, request: ActionRequest) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    Unavailable,
    Initialization(String),
    Window(String),
    Presentation(String),
    Application(String),
    Dialog(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("the native platform runtime is unavailable"),
            Self::Initialization(message) => {
                write!(formatter, "platform initialization failed: {message}")
            }
            Self::Window(message) => write!(formatter, "window operation failed: {message}"),
            Self::Presentation(message) => {
                write!(formatter, "frame presentation failed: {message}")
            }
            Self::Application(message) => {
                write!(formatter, "application callback failed: {message}")
            }
            Self::Dialog(message) => write!(formatter, "file dialog failed: {message}"),
        }
    }
}

impl std::error::Error for PlatformError {}

/// Decode `HKCU\Software\Microsoft\Windows\DWM\AccentColor` (0xAABBGGRR)
/// into RGB.
pub fn accent_from_dwm_value(value: u32) -> [u8; 3] {
    [value as u8, (value >> 8) as u8, (value >> 16) as u8]
}

/// The operating system's accent color, which Chromium's native theme uses for
/// form controls. Windows reports the DWM accent; other platforms report none,
/// so documents keep Chromium's default accent.
pub fn system_accent_color() -> Option<[u8; 3]> {
    #[cfg(all(feature = "windows", target_os = "windows"))]
    {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
        };
        let wide = |text: &str| {
            text.encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
        };
        let key = wide(r"Software\Microsoft\Windows\DWM");
        let name = wide("AccentColor");
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: the key and value names are NUL-terminated UTF-16 buffers
        // and the output buffer is a u32 whose size is passed in `size`.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        if status == 0 {
            return Some(accent_from_dwm_value(value));
        }
    }
    None
}

#[cfg(test)]
mod accent_tests {
    #[test]
    fn dwm_accent_value_is_abgr() {
        assert_eq!(
            super::accent_from_dwm_value(0xFFD4_7800),
            [0x00, 0x78, 0xd4]
        );
    }
}

#[cfg(all(test, not(any(feature = "linux", feature = "windows"))))]
mod tests {
    use super::*;

    #[test]
    fn headless_build_has_no_native_default_feature() {
        assert!(!cfg!(feature = "linux") && !cfg!(feature = "windows"));
        assert_eq!(BackendPreference::default(), BackendPreference::Auto);
    }
}
