//! Feature-gated native platform runtime.

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
mod desktop;

#[cfg(all(feature = "windows", target_os = "windows"))]
mod windows_clipboard;

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
pub use desktop::run;

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
    pub width: f64,
    pub height: f64,
    pub backend: BackendPreference,
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
