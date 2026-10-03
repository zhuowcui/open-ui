//! Framework rendering values and errors.

pub use openui_compositor::SceneRect as Rect;

/// Owned premultiplied RGBA8888 bitmap produced by the software compositor.
#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: usize,
    pub(crate) viewport: openui_engine::ViewportMetrics,
}

impl Bitmap {
    /// Top-to-bottom RGBA bytes. Color channels are premultiplied by alpha.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    pub fn viewport(&self) -> openui_engine::ViewportMetrics {
        self.viewport
    }
}

#[derive(Debug)]
pub enum Error {
    Engine(openui_engine::EngineError),
    Compositor(openui_compositor::CompositorError),
    Animation(openui_style::AnimationError),
    UnknownTag(String),
    ReentrantMutation,
    InvalidArgument(&'static str),
    Io(std::io::Error),
    PlatformUnavailable,
    Platform(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(error) => error.fmt(f),
            Self::Compositor(error) => error.fmt(f),
            Self::Animation(error) => error.fmt(f),
            Self::UnknownTag(tag) => write!(f, "unknown element tag `{tag}`"),
            Self::ReentrantMutation => {
                f.write_str("a callback attempted a reentrant document mutation")
            }
            Self::InvalidArgument(message) => f.write_str(message),
            Self::Io(error) => error.fmt(f),
            Self::PlatformUnavailable => {
                f.write_str("the native platform runtime is not enabled in this build")
            }
            Self::Platform(message) => write!(f, "platform runtime failed: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<openui_engine::EngineError> for Error {
    fn from(value: openui_engine::EngineError) -> Self {
        Self::Engine(value)
    }
}

impl From<openui_engine::ViewportMetricsError> for Error {
    fn from(value: openui_engine::ViewportMetricsError) -> Self {
        Self::Engine(value.into())
    }
}

impl From<openui_compositor::CompositorError> for Error {
    fn from(value: openui_compositor::CompositorError) -> Self {
        Self::Compositor(value)
    }
}

impl From<openui_style::AnimationError> for Error {
    fn from(value: openui_style::AnimationError) -> Self {
        Self::Animation(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
