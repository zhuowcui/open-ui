//! Feature-gated native platform runtime.

#[cfg(all(feature = "linux", target_os = "linux"))]
mod linux;

#[cfg(all(feature = "linux", target_os = "linux"))]
pub use linux::run;

#[cfg(feature = "linux")]
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

    #[cfg(feature = "linux")]
    fn accessibility_update(&mut self) -> Result<TreeUpdate, String>;

    #[cfg(feature = "linux")]
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

#[cfg(all(test, not(feature = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn headless_build_has_no_native_default_feature() {
        assert!(!cfg!(feature = "linux"));
        assert_eq!(BackendPreference::default(), BackendPreference::Auto);
    }
}
