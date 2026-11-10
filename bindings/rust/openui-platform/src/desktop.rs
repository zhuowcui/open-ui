use crate::{
    BackendPreference, BackendStatus, Clipboard, ClipboardBackend, CursorIcon, KeyPhase, LoopWaker,
    Modifiers, PlatformApplication, PlatformError, PlatformEvent, PointerButton, PointerPhase,
    SoftwareFrame, WindowCommand, WindowController, WindowOptions,
};
use accesskit_winit::{Adapter as AccessibilityAdapter, WindowEvent as AccessibilityEvent};
use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, GlContext, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::{GlSurface, NotCurrentGlContext, PossiblyCurrentGlContext};
use glutin::surface::{Surface as GlutinSurface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
#[cfg(target_os = "linux")]
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use softbuffer::{Context as SoftContext, Surface as SoftSurface};
use std::ffi::CString;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::time::Duration;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{
    ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy, OwnedDisplayHandle,
};
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorIcon as WinitCursorIcon, Window, WindowAttributes, WindowId};

#[derive(Debug)]
enum UserEvent {
    Accessibility(accesskit_winit::Event),
    /// The `openui dev` parent closed the app's stdin: exit as if the window
    /// were closed, so the app shuts down through its normal path.
    DevExit,
    /// A [`LoopWaker`] was woken: run the controller's queued UI tasks.
    Wake,
}

/// Set by `openui dev` for the apps it runs.
const DEV_ENV: &str = "OPENUI_DEV";

impl From<accesskit_winit::Event> for UserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::Accessibility(event)
    }
}

enum NativeClipboard {
    #[cfg(target_os = "linux")]
    Wayland(Box<smithay_clipboard::Clipboard>),
    #[cfg(target_os = "linux")]
    X11(Box<x11_clipboard::Clipboard>),
    #[cfg(target_os = "windows")]
    Windows(crate::windows_clipboard::WindowsClipboard),
}

enum Presenter {
    Software {
        _context: SoftContext<OwnedDisplayHandle>,
        surface: SoftSurface<OwnedDisplayHandle, Arc<Window>>,
    },
    OpenGl(OpenGlPresenter),
}

struct OpenGlPresenter {
    surface: GlutinSurface<WindowSurface>,
    context: PossiblyCurrentContext,
    program: u32,
    texture: u32,
    texture_size: Option<(u32, u32)>,
    vertex_array: u32,
    vertex_buffer: u32,
    window: Arc<Window>,
}

impl Drop for OpenGlPresenter {
    fn drop(&mut self) {
        // Another window's context may be current; these names belong to
        // this one. If it cannot be made current, leak the names rather than
        // delete another context's objects.
        if self.context.make_current(&self.surface).is_err() {
            return;
        }
        // SAFETY: this presenter owns these names and its context is current.
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteProgram(self.program);
        }
    }
}

/// The GL entry points the OpenGL presenter calls that failed to load.
fn missing_gl_functions() -> Vec<&'static str> {
    macro_rules! missing {
        ($($name:ident),* $(,)?) => {{
            let mut missing = Vec::new();
            $(if !gl::$name::is_loaded() {
                missing.push(stringify!($name));
            })*
            missing
        }};
    }
    missing!(
        ActiveTexture,
        AttachShader,
        BindBuffer,
        BindTexture,
        BindVertexArray,
        BufferData,
        Clear,
        ClearColor,
        CompileShader,
        CreateProgram,
        CreateShader,
        DeleteBuffers,
        DeleteProgram,
        DeleteShader,
        DeleteTextures,
        DeleteVertexArrays,
        DrawArrays,
        EnableVertexAttribArray,
        GenBuffers,
        GenTextures,
        GenVertexArrays,
        GetProgramInfoLog,
        GetProgramiv,
        GetShaderInfoLog,
        GetShaderiv,
        GetUniformLocation,
        LinkProgram,
        PixelStorei,
        ShaderSource,
        TexImage2D,
        TexParameteri,
        TexSubImage2D,
        Uniform1i,
        UseProgram,
        VertexAttribPointer,
        Viewport,
    )
}

impl OpenGlPresenter {
    fn new(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Result<(Arc<Window>, Self), PlatformError> {
        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(attributes));
        let (window, config) = display_builder
            .build(event_loop, template, |configs| {
                configs
                    .reduce(|selected, candidate| {
                        if candidate.num_samples() < selected.num_samples() {
                            candidate
                        } else {
                            selected
                        }
                    })
                    .expect("glutin supplied no configuration to its mandatory picker")
            })
            .map_err(|error| PlatformError::Initialization(error.to_string()))?;
        let window = Arc::new(window.ok_or_else(|| {
            PlatformError::Window("glutin did not create the requested window".into())
        })?);
        let raw_window = window
            .window_handle()
            .map_err(|error| PlatformError::Initialization(error.to_string()))?
            .as_raw();
        let attributes = ContextAttributesBuilder::new().build(Some(raw_window));
        let fallback_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(raw_window));
        // SAFETY: `raw_window` comes from `window`, which is retained by this
        // presenter until every GL object has been destroyed.
        let not_current = unsafe {
            config
                .display()
                .create_context(&config, &attributes)
                .or_else(|_| {
                    config
                        .display()
                        .create_context(&config, &fallback_attributes)
                })
        }
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
        let surface_attributes = window
            .build_surface_attributes(Default::default())
            .map_err(|error| PlatformError::Initialization(error.to_string()))?;
        // SAFETY: the surface attributes were built from the retained winit
        // window and the surface is dropped before that window.
        let surface = unsafe {
            config
                .display()
                .create_window_surface(&config, &surface_attributes)
        }
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
        let context = not_current
            .make_current(&surface)
            .map_err(|error| PlatformError::Initialization(error.to_string()))?;
        let display = config.display();
        gl::load_with(|name| {
            CString::new(name)
                .ok()
                .map_or(std::ptr::null(), |name| display.get_proc_address(&name))
        });
        // A context can be created without the entry points this presenter
        // uses, for example Windows' GDI OpenGL 1.1 renderer. Report it as an
        // initialization failure so `Auto` falls back to software presentation.
        let missing = missing_gl_functions();
        if !missing.is_empty() {
            return Err(PlatformError::Initialization(format!(
                "OpenGL context lacks {}",
                missing.join(", ")
            )));
        }
        let gles = matches!(context.context_api(), ContextApi::Gles(_));
        let (program, texture, vertex_array, vertex_buffer) = initialize_gl_objects(gles)?;
        let _ = surface.set_swap_interval(
            &context,
            SwapInterval::Wait(NonZeroU32::new(1).expect("one is nonzero")),
        );
        Ok((
            window.clone(),
            Self {
                surface,
                context,
                program,
                texture,
                texture_size: None,
                vertex_array,
                vertex_buffer,
                window,
            },
        ))
    }

    fn present(&mut self, frame: &SoftwareFrame) -> Result<(), PlatformError> {
        let physical = self.window.inner_size();
        if physical.width == 0 || physical.height == 0 {
            return Ok(());
        }
        // Each window has its own context; make this one current before
        // touching GL state.
        self.context
            .make_current(&self.surface)
            .map_err(|error| PlatformError::Presentation(error.to_string()))?;
        self.window.resize_surface(&self.surface, &self.context);
        validate_frame(frame)?;
        if (frame.width, frame.height) != (physical.width, physical.height) {
            return Err(PlatformError::Presentation(format!(
                "frame/surface size mismatch: frame={}x{}, surface={}x{}",
                frame.width, frame.height, physical.width, physical.height
            )));
        }
        let width = i32::try_from(frame.width)
            .map_err(|_| PlatformError::Presentation("frame width exceeds i32".into()))?;
        let height = i32::try_from(frame.height)
            .map_err(|_| PlatformError::Presentation("frame height exceeds i32".into()))?;
        let viewport_width = i32::try_from(physical.width)
            .map_err(|_| PlatformError::Presentation("surface width exceeds i32".into()))?;
        let viewport_height = i32::try_from(physical.height)
            .map_err(|_| PlatformError::Presentation("surface height exceeds i32".into()))?;
        // SAFETY: all names were created by this current context; the frame
        // slice was validated for a tightly packed RGBA upload below.
        unsafe {
            gl::Viewport(0, 0, viewport_width, viewport_height);
            gl::ClearColor(1.0, 1.0, 1.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::PixelStorei(gl::UNPACK_ROW_LENGTH, (frame.stride / 4) as i32);
            if self.texture_size != Some((frame.width, frame.height)) {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    width,
                    height,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                self.texture_size = Some((frame.width, frame.height));
            }
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                width,
                height,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                frame.pixels.as_ptr().cast(),
            );
            gl::BindVertexArray(self.vertex_array);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::BindVertexArray(0);
            gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
        }
        self.surface
            .swap_buffers(&self.context)
            .map_err(|error| PlatformError::Presentation(error.to_string()))
    }
}

fn initialize_gl_objects(gles: bool) -> Result<(u32, u32, u32, u32), PlatformError> {
    let vertex_source = if gles {
        "#version 300 es\nlayout(location=0) in vec2 position; layout(location=1) in vec2 uv; out vec2 texture_uv; void main(){ texture_uv=uv; gl_Position=vec4(position,0.0,1.0); }"
    } else {
        "#version 330 core\nlayout(location=0) in vec2 position; layout(location=1) in vec2 uv; out vec2 texture_uv; void main(){ texture_uv=uv; gl_Position=vec4(position,0.0,1.0); }"
    };
    let fragment_source = if gles {
        "#version 300 es\nprecision mediump float; in vec2 texture_uv; uniform sampler2D frame_texture; out vec4 color; void main(){ color=texture(frame_texture,texture_uv); }"
    } else {
        "#version 330 core\nin vec2 texture_uv; uniform sampler2D frame_texture; out vec4 color; void main(){ color=texture(frame_texture,texture_uv); }"
    };
    let vertex_shader = compile_shader(gl::VERTEX_SHADER, vertex_source)?;
    let fragment_shader = match compile_shader(gl::FRAGMENT_SHADER, fragment_source) {
        Ok(shader) => shader,
        Err(error) => {
            // SAFETY: the vertex shader belongs to the current context and is
            // no longer needed after the paired shader failed.
            unsafe { gl::DeleteShader(vertex_shader) };
            return Err(error);
        }
    };
    // SAFETY: the GL context is current, shader identifiers are valid, and all
    // buffer source pointers remain valid for the duration of their calls.
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex_shader);
        gl::AttachShader(program, fragment_shader);
        gl::LinkProgram(program);
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);
        check_program(program)?;

        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0,
        ];
        let mut vertex_array = 0;
        let mut vertex_buffer = 0;
        gl::GenVertexArrays(1, &mut vertex_array);
        gl::GenBuffers(1, &mut vertex_buffer);
        gl::BindVertexArray(vertex_array);
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            isize::try_from(std::mem::size_of_val(&vertices)).expect("vertex data fits isize"),
            vertices.as_ptr().cast(),
            gl::STATIC_DRAW,
        );
        let stride = i32::try_from(4 * std::mem::size_of::<f32>()).expect("vertex stride fits i32");
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (2 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::BindVertexArray(0);

        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        // Presentation is an exact 1:1 transfer of an already rasterized
        // physical frame. Nearest sampling prevents a driver from blending
        // adjacent pixels even if its fullscreen-quad convention differs.
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::UseProgram(program);
        let sampler = CString::new("frame_texture").expect("static string has no nul");
        gl::Uniform1i(gl::GetUniformLocation(program, sampler.as_ptr()), 0);
        Ok((program, texture, vertex_array, vertex_buffer))
    }
}

fn compile_shader(kind: u32, source: &str) -> Result<u32, PlatformError> {
    let source = CString::new(source).expect("shader source has no nul");
    // SAFETY: the context is current, `source` is NUL terminated, and GL owns
    // the returned shader until it is explicitly deleted.
    unsafe {
        let shader = gl::CreateShader(kind);
        gl::ShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);
        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let message = shader_log(shader);
            gl::DeleteShader(shader);
            return Err(PlatformError::Initialization(format!(
                "OpenGL shader compilation failed: {message}"
            )));
        }
        Ok(shader)
    }
}

unsafe fn shader_log(shader: u32) -> String {
    let mut length = 0;
    // SAFETY: `shader` is a live shader owned by the caller.
    unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut length) };
    let mut bytes = vec![0_u8; length.max(1) as usize];
    // SAFETY: `bytes` is writable for the reported log length.
    unsafe {
        gl::GetShaderInfoLog(
            shader,
            length,
            std::ptr::null_mut(),
            bytes.as_mut_ptr().cast(),
        )
    };
    String::from_utf8_lossy(&bytes)
        .trim_end_matches(char::from(0))
        .to_owned()
}

unsafe fn program_log(program: u32) -> String {
    let mut length = 0;
    // SAFETY: `program` is a live program owned by the caller.
    unsafe { gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut length) };
    let mut bytes = vec![0_u8; length.max(1) as usize];
    // SAFETY: `bytes` is writable for the reported log length.
    unsafe {
        gl::GetProgramInfoLog(
            program,
            length,
            std::ptr::null_mut(),
            bytes.as_mut_ptr().cast(),
        )
    };
    String::from_utf8_lossy(&bytes)
        .trim_end_matches(char::from(0))
        .to_owned()
}

fn check_program(program: u32) -> Result<(), PlatformError> {
    let mut success = 0;
    // SAFETY: `program` was returned by the current GL context.
    unsafe { gl::GetProgramiv(program, gl::LINK_STATUS, &mut success) };
    if success == 0 {
        // SAFETY: the program is still live and can be queried before deletion.
        let message = unsafe { program_log(program) };
        // SAFETY: the program belongs to the current context.
        unsafe { gl::DeleteProgram(program) };
        Err(PlatformError::Initialization(format!(
            "OpenGL program link failed: {message}"
        )))
    } else {
        Ok(())
    }
}

fn validate_frame(frame: &SoftwareFrame) -> Result<(), PlatformError> {
    if frame.viewport.physical_size() != (frame.width, frame.height)
        || frame.width == 0
        || frame.height == 0
        || frame.stride < frame.width as usize * 4
        || frame.stride % 4 != 0
        || frame.pixels.len() < frame.stride * frame.height as usize
        || frame.stride / 4 > i32::MAX as usize
    {
        Err(PlatformError::Presentation(
            "application returned an invalid software frame".into(),
        ))
    } else {
        Ok(())
    }
}

fn software_presenter(
    display: &OwnedDisplayHandle,
    window: Arc<Window>,
) -> Result<Presenter, PlatformError> {
    let context = SoftContext::new(display.clone())
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    let surface = SoftSurface::new(&context, window)
        .map_err(|error| PlatformError::Presentation(error.to_string()))?;
    Ok(Presenter::Software {
        _context: context,
        surface,
    })
}

fn initialize_presenter(
    event_loop: &ActiveEventLoop,
    display: &OwnedDisplayHandle,
    options: &WindowOptions,
) -> Result<(Arc<Window>, Presenter, BackendStatus), PlatformError> {
    let attributes = WindowAttributes::default()
        .with_title(options.title.clone())
        .with_inner_size(LogicalSize::new(options.width, options.height))
        .with_resizable(options.resizable)
        .with_decorations(options.decorations)
        .with_visible(false);
    let attributes = match options.min_size {
        Some((width, height)) => attributes.with_min_inner_size(LogicalSize::new(width, height)),
        None => attributes,
    };
    let attributes = match options.max_size {
        Some((width, height)) => attributes.with_max_inner_size(LogicalSize::new(width, height)),
        None => attributes,
    };
    if options.backend != BackendPreference::Software {
        match OpenGlPresenter::new(event_loop, attributes.clone()) {
            Ok((window, presenter)) => {
                return Ok((
                    window,
                    Presenter::OpenGl(presenter),
                    BackendStatus {
                        active: BackendPreference::OpenGl,
                        fallback_reason: None,
                    },
                ));
            }
            Err(error) if options.backend == BackendPreference::OpenGl => return Err(error),
            Err(error) => {
                let window = Arc::new(
                    event_loop
                        .create_window(attributes)
                        .map_err(|error| PlatformError::Window(error.to_string()))?,
                );
                let presenter = software_presenter(display, window.clone())?;
                return Ok((
                    window,
                    presenter,
                    BackendStatus {
                        active: BackendPreference::Software,
                        fallback_reason: Some(error.to_string()),
                    },
                ));
            }
        }
    }
    let window = Arc::new(
        event_loop
            .create_window(attributes)
            .map_err(|error| PlatformError::Window(error.to_string()))?,
    );
    let presenter = software_presenter(display, window.clone())?;
    Ok((
        window,
        presenter,
        BackendStatus {
            active: BackendPreference::Software,
            fallback_reason: None,
        },
    ))
}

impl ClipboardBackend for NativeClipboard {
    fn load(&self) -> Result<String, PlatformError> {
        NativeClipboard::load(self)
    }

    fn store(&self, text: String) {
        NativeClipboard::store(self, text);
    }
}

impl NativeClipboard {
    #[cfg(target_os = "windows")]
    fn new(_display: &OwnedDisplayHandle) -> Result<Self, PlatformError> {
        Ok(Self::Windows(crate::windows_clipboard::WindowsClipboard))
    }

    #[cfg(target_os = "linux")]
    fn new(display: &OwnedDisplayHandle) -> Result<Self, PlatformError> {
        let handle = display
            .display_handle()
            .map_err(|error| PlatformError::Initialization(error.to_string()))?;
        match handle.as_raw() {
            RawDisplayHandle::Wayland(handle) => {
                // SAFETY: the winit owned display handle is retained by the runtime's
                // softbuffer context and event loop, both of which outlive this clipboard.
                Ok(Self::Wayland(Box::new(unsafe {
                    smithay_clipboard::Clipboard::new(handle.display.as_ptr())
                })))
            }
            RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_) => x11_clipboard::Clipboard::new()
                .map(Box::new)
                .map(Self::X11)
                .map_err(|error| PlatformError::Initialization(error.to_string())),
            other => Err(PlatformError::Initialization(format!(
                "unsupported Linux clipboard display {other:?}"
            ))),
        }
    }

    fn load(&self) -> Result<String, PlatformError> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(clipboard) => clipboard.load().map_err(PlatformError::Application),
            #[cfg(target_os = "linux")]
            Self::Wayland(clipboard) => clipboard
                .load()
                .map_err(|error| PlatformError::Application(error.to_string())),
            #[cfg(target_os = "linux")]
            Self::X11(clipboard) => {
                let atoms = &clipboard.getter.atoms;
                clipboard
                    .load(
                        atoms.clipboard,
                        atoms.utf8_string,
                        atoms.property,
                        Duration::from_millis(100),
                    )
                    .map_err(|error| PlatformError::Application(error.to_string()))
                    .and_then(|bytes| {
                        String::from_utf8(bytes)
                            .map_err(|error| PlatformError::Application(error.to_string()))
                    })
            }
        }
    }

    fn store(&self, text: String) {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(clipboard) => clipboard.store(text),
            #[cfg(target_os = "linux")]
            Self::Wayland(clipboard) => clipboard.store(text),
            #[cfg(target_os = "linux")]
            Self::X11(clipboard) => {
                let atoms = &clipboard.setter.atoms;
                let _ = clipboard.store(atoms.clipboard, atoms.utf8_string, text.into_bytes());
            }
        }
    }
}

#[derive(Default)]
struct ImeState {
    enabled: bool,
    focused: bool,
    composing: bool,
}

impl ImeState {
    fn focus_changed(&mut self, focused: bool) -> Option<PlatformEvent> {
        self.focused = focused;
        if !focused && std::mem::take(&mut self.composing) {
            Some(PlatformEvent::CompositionEnd(String::new()))
        } else {
            None
        }
    }

    fn events(&mut self, ime: Ime) -> Vec<PlatformEvent> {
        match ime {
            Ime::Enabled => {
                self.enabled = true;
                Vec::new()
            }
            Ime::Disabled => {
                self.enabled = false;
                if std::mem::take(&mut self.composing) {
                    vec![PlatformEvent::CompositionEnd(String::new())]
                } else {
                    Vec::new()
                }
            }
            Ime::Preedit(text, _) if self.enabled && self.focused => {
                let mut events = Vec::new();
                if !self.composing && !text.is_empty() {
                    self.composing = true;
                    events.push(PlatformEvent::CompositionStart);
                }
                if self.composing {
                    events.push(PlatformEvent::CompositionUpdate(text));
                }
                events
            }
            Ime::Commit(text) if self.enabled && self.focused => {
                vec![if std::mem::take(&mut self.composing) {
                    PlatformEvent::CompositionEnd(text)
                } else {
                    PlatformEvent::TextInput(text)
                }]
            }
            Ime::Preedit(_, _) | Ime::Commit(_) => Vec::new(),
        }
    }
}

/// One native window and the application object that draws it.
///
/// Fields drop in declaration order: the application first, then the window
/// and its presenter, then the accessibility adapter, as the single-window
/// runtime did.
struct WindowState {
    id: crate::WindowId,
    application: Box<dyn PlatformApplication>,
    options: WindowOptions,
    window: Arc<Window>,
    presenter: Option<Presenter>,
    accessibility: AccessibilityAdapter,
    cursor: LogicalPosition<f64>,
    scale_factor: f64,
    modifiers: Modifiers,
    ime: ImeState,
    presented_frames: u64,
    /// The last frame round this window drew in.
    drawn_round: u64,
}

impl WindowState {
    fn open(
        event_loop: &ActiveEventLoop,
        display: &OwnedDisplayHandle,
        proxy: &EventLoopProxy<UserEvent>,
        id: crate::WindowId,
        options: WindowOptions,
        application: Box<dyn PlatformApplication>,
    ) -> Result<(Self, BackendStatus), PlatformError> {
        let (window, presenter, status) = initialize_presenter(event_loop, display, &options)?;
        // AccessKit requires its adapter before the window is first shown.
        let accessibility =
            AccessibilityAdapter::with_event_loop_proxy(event_loop, &window, proxy.clone());
        window.set_ime_allowed(true);
        window.set_visible(true);
        Ok((
            Self {
                id,
                application,
                options,
                scale_factor: window.scale_factor(),
                window,
                presenter: Some(presenter),
                accessibility,
                cursor: LogicalPosition::new(0.0, 0.0),
                modifiers: Modifiers::default(),
                ime: ImeState::default(),
                presented_frames: 0,
                drawn_round: 0,
            },
            status,
        ))
    }

    /// Tell a newly opened window's application its backend and size.
    fn start(&mut self, status: BackendStatus) -> Result<(), PlatformError> {
        self.send(PlatformEvent::BackendChanged(status))?;
        let physical = self.window.inner_size();
        self.send(self.resized(physical.width, physical.height))?;
        self.request_redraw();
        Ok(())
    }

    fn send(&mut self, event: PlatformEvent) -> Result<(), PlatformError> {
        self.application
            .event(event)
            .map_err(PlatformError::Application)
    }

    fn resized(&self, width: u32, height: u32) -> PlatformEvent {
        PlatformEvent::Resized(
            openui_geometry::ViewportMetrics::from_physical_size(width, height, self.scale_factor)
                .expect("winit returned validated window metrics"),
        )
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn wants_frame(&self) -> bool {
        self.application.is_animating() || self.application.needs_redraw()
    }

    /// Whether this window takes part in frame rounds. A minimized window
    /// may never be asked to paint, so it would stall the others.
    fn in_frame_round(&self) -> bool {
        self.wants_frame() && self.window.is_minimized() != Some(true)
    }

    fn update_accessibility(&mut self) -> Result<(), PlatformError> {
        let mut error = None;
        let application = &mut self.application;
        self.accessibility
            .update_if_active(|| match application.accessibility_update() {
                Ok(update) => update,
                Err(message) => {
                    error = Some(PlatformError::Application(message));
                    empty_tree_update()
                }
            });
        error.map_or(Ok(()), Err)
    }

    fn redraw(
        &mut self,
        display: &OwnedDisplayHandle,
        started: Instant,
    ) -> Result<(), PlatformError> {
        let physical = self.window.inner_size();
        if physical.width == 0 || physical.height == 0 {
            return Ok(());
        }
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let result = match self.presenter.as_mut() {
            Some(Presenter::OpenGl(presenter)) => self
                .application
                .render(elapsed_ms)
                .map_err(PlatformError::Application)
                .and_then(|frame| presenter.present(&frame)),
            Some(Presenter::Software { surface, .. }) => self
                .application
                .render(elapsed_ms)
                .map_err(PlatformError::Application)
                .and_then(|frame| {
                    surface
                        .resize(
                            NonZeroU32::new(physical.width).expect("nonzero width checked"),
                            NonZeroU32::new(physical.height).expect("nonzero height checked"),
                        )
                        .map_err(|error| PlatformError::Presentation(error.to_string()))?;
                    let mut buffer = surface
                        .buffer_mut()
                        .map_err(|error| PlatformError::Presentation(error.to_string()))?;
                    copy_frame(&frame, physical.width, physical.height, &mut buffer)?;
                    buffer
                        .present()
                        .map_err(|error| PlatformError::Presentation(error.to_string()))
                }),
            None => Err(PlatformError::Presentation(
                "presentation backend is unavailable".into(),
            )),
        };
        if let Err(error) = result {
            if self.options.backend == BackendPreference::Auto
                && matches!(self.presenter, Some(Presenter::OpenGl(_)))
            {
                self.presenter.take();
                self.presenter = Some(software_presenter(display, self.window.clone())?);
                let status = BackendStatus {
                    active: BackendPreference::Software,
                    fallback_reason: Some(error.to_string()),
                };
                self.send(PlatformEvent::BackendChanged(status))?;
                self.request_redraw();
                return Ok(());
            }
            return Err(error);
        }
        self.presented_frames = self.presented_frames.saturating_add(1);
        self.send(PlatformEvent::Presented {
            frame_number: self.presented_frames,
            time_ms: elapsed_ms,
        })?;
        // The runtime requests the next frame, fairly across windows.
        self.update_accessibility()
    }

    fn handle_key(
        &mut self,
        clipboard: &NativeClipboard,
        event: winit::event::KeyEvent,
    ) -> Result<(), PlatformError> {
        let pressed = event.state == ElementState::Pressed;
        let key_text = logical_key_text(&event.logical_key);
        let shortcut = key_text.as_deref().map(str::to_ascii_lowercase);
        let command_modifier = self.modifiers.control || self.modifiers.meta;
        if pressed && command_modifier && shortcut.as_deref() == Some("v") {
            if let Ok(text) = clipboard.load() {
                self.application
                    .set_clipboard_text(text)
                    .map_err(PlatformError::Application)?;
            }
        }
        self.application
            .key_input(crate::KeyboardInput {
                phase: if pressed {
                    KeyPhase::Down
                } else {
                    KeyPhase::Up
                },
                key_code: key_code(&event.logical_key),
                key_text,
                text: event.text.as_ref().map(ToString::to_string),
                modifiers: self.modifiers,
                repeat: event.repeat,
            })
            .map_err(PlatformError::Application)?;
        if pressed && command_modifier && matches!(shortcut.as_deref(), Some("c") | Some("x")) {
            if let Ok(text) = self.application.clipboard_text() {
                clipboard.store(text);
            }
        }
        self.request_redraw();
        Ok(())
    }

    fn handle_accessibility_event(
        &mut self,
        event: AccessibilityEvent,
    ) -> Result<(), PlatformError> {
        match event {
            AccessibilityEvent::InitialTreeRequested => self.update_accessibility(),
            AccessibilityEvent::ActionRequested(request) => {
                self.application
                    .accessibility_action(request)
                    .map_err(PlatformError::Application)?;
                self.request_redraw();
                Ok(())
            }
            AccessibilityEvent::AccessibilityDeactivated => Ok(()),
        }
    }

    fn pointer(&mut self, phase: PointerPhase, button: PointerButton) -> Result<(), PlatformError> {
        self.send(PlatformEvent::Pointer {
            pointer_id: 0,
            phase,
            x: self.cursor.x as f32,
            y: self.cursor.y as f32,
            button,
            modifiers: self.modifiers,
        })
    }

    /// Every window event except closing, which the runtime handles.
    fn handle_event(
        &mut self,
        event: WindowEvent,
        clipboard: &NativeClipboard,
        display: &OwnedDisplayHandle,
        started: Instant,
    ) -> Result<(), PlatformError> {
        match event {
            WindowEvent::Resized(physical) => {
                if physical.width > 0 && physical.height > 0 {
                    self.send(self.resized(physical.width, physical.height))?;
                    self.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                let physical = self.window.inner_size();
                self.send(self.resized(physical.width, physical.height))?;
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = position.to_logical(self.scale_factor);
                self.pointer(PointerPhase::Move, PointerButton::Other(0))?;
                let icon = self
                    .application
                    .cursor_icon(self.cursor.x as f32, self.cursor.y as f32)
                    .map_err(PlatformError::Application)?;
                self.window.set_cursor(winit_cursor(icon));
                self.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer(PointerPhase::Leave, PointerButton::Other(0))?;
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let phase = if state == ElementState::Pressed {
                    PointerPhase::Down
                } else {
                    PointerPhase::Up
                };
                self.pointer(phase, pointer_button(button))?;
                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
                    MouseScrollDelta::PixelDelta(position) => (
                        (position.x / self.scale_factor) as f32,
                        (position.y / self.scale_factor) as f32,
                    ),
                };
                self.send(PlatformEvent::Wheel {
                    x: self.cursor.x as f32,
                    y: self.cursor.y as f32,
                    delta_x,
                    delta_y,
                    modifiers: self.modifiers,
                })?;
                self.request_redraw();
            }
            WindowEvent::Touch(touch) => {
                let logical = touch.location.to_logical::<f64>(self.scale_factor);
                let phase = match touch.phase {
                    TouchPhase::Started => PointerPhase::Down,
                    TouchPhase::Moved => PointerPhase::Move,
                    TouchPhase::Ended => PointerPhase::Up,
                    TouchPhase::Cancelled => PointerPhase::Cancel,
                };
                self.send(PlatformEvent::Pointer {
                    pointer_id: touch.id.saturating_add(1),
                    phase,
                    x: logical.x as f32,
                    y: logical.y as f32,
                    button: PointerButton::Left,
                    modifiers: self.modifiers,
                })?;
                self.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(clipboard, event)?,
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.modifiers = Modifiers {
                    shift: state.shift_key(),
                    control: state.control_key(),
                    alt: state.alt_key(),
                    meta: state.super_key(),
                };
            }
            WindowEvent::Ime(ime) => {
                for event in self.ime.events(ime) {
                    self.send(event)?;
                    self.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(event) = self.ime.focus_changed(focused) {
                    self.send(event)?;
                }
                self.send(PlatformEvent::Focused(focused))?;
                self.request_redraw();
            }
            WindowEvent::DroppedFile(path) => {
                // Like the other file notifications, a failing handler does
                // not end the app.
                let _ = self.send(PlatformEvent::DroppedFile(path));
            }
            WindowEvent::HoveredFile(path) => {
                let _ = self.send(PlatformEvent::HoveredFile(path));
            }
            WindowEvent::HoveredFileCancelled => {
                let _ = self.send(PlatformEvent::HoveredFileCancelled);
            }
            WindowEvent::RedrawRequested => self.redraw(display, started)?,
            _ => {}
        }
        Ok(())
    }
}

/// The event loop and every open window. Windows drop before the clipboard,
/// and the clipboard before the display handle it was created from.
struct Runtime<C: WindowController> {
    windows: Vec<WindowState>,
    controller: C,
    clipboard: Rc<NativeClipboard>,
    display: OwnedDisplayHandle,
    proxy: EventLoopProxy<UserEvent>,
    started: Instant,
    resumed: bool,
    error: Option<PlatformError>,
    /// Windows that want frames draw once per round. Windows delivers paint
    /// requests to one window first, so re-requesting a frame as soon as a
    /// window draws would starve the others.
    frame_round: u64,
    frame_round_started: Instant,
    /// Set by [`LoopWaker::wake`] until the loop handles the wake event.
    wake_pending: Arc<AtomicBool>,
}

/// A round ends early if a window has not drawn by then, so one window that
/// is never asked to paint cannot stop the others.
const FRAME_ROUND_LIMIT: std::time::Duration = std::time::Duration::from_millis(250);

impl<C: WindowController> Runtime<C> {
    /// End the loop. Post a wake as well: with no windows open, Windows
    /// otherwise waits for a message before it sees the request.
    fn exit(&self, event_loop: &ActiveEventLoop) {
        if !event_loop.exiting() {
            event_loop.exit();
            let _ = self.proxy.send_event(UserEvent::Wake);
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
        self.exit(event_loop);
    }

    fn check(&mut self, event_loop: &ActiveEventLoop, result: Result<(), PlatformError>) -> bool {
        match result {
            Ok(()) => true,
            Err(error) => {
                self.fail(event_loop, error);
                false
            }
        }
    }

    fn index_of(&self, window: WindowId) -> Option<usize> {
        self.windows
            .iter()
            .position(|state| state.window.id() == window)
    }

    fn index_of_id(&self, id: crate::WindowId) -> Option<usize> {
        self.windows.iter().position(|state| state.id == id)
    }

    /// Apply the controller's queued window commands until it has none. A
    /// new window's first events may queue more.
    fn apply_commands(&mut self, event_loop: &ActiveEventLoop) {
        while self.resumed && !event_loop.exiting() {
            let commands = match self.controller.take_commands() {
                Ok(commands) => commands,
                Err(error) => return self.fail(event_loop, PlatformError::Application(error)),
            };
            if commands.is_empty() {
                return;
            }
            for command in commands {
                self.apply(event_loop, command);
                if event_loop.exiting() {
                    return;
                }
            }
        }
    }

    fn apply(&mut self, event_loop: &ActiveEventLoop, command: WindowCommand) {
        match command {
            WindowCommand::Open {
                id,
                options,
                application,
            } => {
                if let Some(index) = self.index_of_id(id) {
                    self.windows[index].window.focus_window();
                    return;
                }
                match WindowState::open(
                    event_loop,
                    &self.display,
                    &self.proxy,
                    id,
                    options,
                    application,
                ) {
                    Ok((state, status)) => {
                        self.windows.push(state);
                        let result = self.windows.last_mut().expect("just pushed").start(status);
                        self.check(event_loop, result);
                    }
                    Err(error) => self.fail(event_loop, error),
                }
            }
            WindowCommand::Close(id) => {
                if let Some(index) = self.index_of_id(id) {
                    self.close(event_loop, index);
                }
            }
            WindowCommand::Focus(id) => {
                if let Some(index) = self.index_of_id(id) {
                    self.windows[index].window.focus_window();
                }
            }
            WindowCommand::SetTitle(id, title) => {
                if let Some(index) = self.index_of_id(id) {
                    self.windows[index].window.set_title(&title);
                    self.windows[index].options.title = title;
                }
            }
        }
    }

    /// Destroy a window, then tell the controller.
    fn close(&mut self, event_loop: &ActiveEventLoop, index: usize) {
        let id = self.windows[index].id;
        drop(self.windows.remove(index));
        let result = self
            .controller
            .window_closed(id)
            .map_err(PlatformError::Application);
        self.check(event_loop, result);
    }

    /// Request a frame from each window that wants one and has not drawn in
    /// this round, starting a new round once all have.
    fn request_frames(&mut self) {
        let now = Instant::now();
        let round = self.frame_round;
        let waiting = self
            .windows
            .iter()
            .any(|state| state.in_frame_round() && state.drawn_round < round);
        if !waiting || now.duration_since(self.frame_round_started) > FRAME_ROUND_LIMIT {
            self.frame_round += 1;
            self.frame_round_started = now;
        }
        for state in &self.windows {
            if state.in_frame_round() && state.drawn_round < self.frame_round {
                state.request_redraw();
            }
        }
    }

    /// Run one turn of queued UI tasks; true if more are queued.
    fn run_tasks(&mut self, event_loop: &ActiveEventLoop) -> bool {
        match self.controller.run_tasks() {
            Ok(more) => more,
            Err(error) => {
                self.fail(event_loop, PlatformError::Application(error));
                false
            }
        }
    }

    fn exit_if_requested(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let requested = self.resumed
            && (self.controller.exit_requested(self.windows.len())
                || self
                    .windows
                    .iter()
                    .any(|state| state.application.exit_requested()));
        if requested {
            self.exit(event_loop);
        }
        requested || event_loop.exiting()
    }
}

impl<C: WindowController> ApplicationHandler<UserEvent> for Runtime<C> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.resumed {
            return;
        }
        self.resumed = true;
        self.apply_commands(event_loop);
        self.exit_if_requested(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Accessibility(event) => {
                let Some(index) = self.index_of(event.window_id) else {
                    return;
                };
                let result = self.windows[index].handle_accessibility_event(event.window_event);
                if self.check(event_loop, result) {
                    self.apply_commands(event_loop);
                    self.exit_if_requested(event_loop);
                }
            }
            UserEvent::Wake if event_loop.exiting() => {}
            UserEvent::Wake => {
                // Clear first, so a wake from a task that runs now posts
                // another event.
                self.wake_pending.store(false, Ordering::Release);
                if self.run_tasks(event_loop) {
                    event_loop.set_control_flow(ControlFlow::Poll);
                }
                self.apply_commands(event_loop);
                self.exit_if_requested(event_loop);
            }
            UserEvent::DevExit => {
                for state in &mut self.windows {
                    let _ = state.send(PlatformEvent::CloseRequested);
                }
                self.exit(event_loop);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(index) = self.index_of(window_id) else {
            return;
        };
        {
            let state = &mut self.windows[index];
            state.accessibility.process_event(&state.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                let id = self.windows[index].id;
                // As before, a failing close handler does not keep the window.
                let _ = self.windows[index].send(PlatformEvent::CloseRequested);
                match self.controller.close_requested(id) {
                    Ok(true) => self.close(event_loop, index),
                    Ok(false) => {}
                    Err(error) => return self.fail(event_loop, PlatformError::Application(error)),
                }
            }
            WindowEvent::Destroyed => {
                let _ = self.windows[index].send(PlatformEvent::CloseRequested);
                self.close(event_loop, index);
            }
            event => {
                let redraw = matches!(event, WindowEvent::RedrawRequested);
                let state = &mut self.windows[index];
                let result =
                    state.handle_event(event, &self.clipboard, &self.display, self.started);
                if redraw {
                    state.drawn_round = self.frame_round;
                }
                if !self.check(event_loop, result) {
                    return;
                }
                if redraw {
                    // Request the next frame now, as the single-window
                    // runtime did, so animation continues where the loop
                    // does not reach `about_to_wait`, such as a modal resize.
                    self.request_frames();
                }
            }
        }
        self.apply_commands(event_loop);
        self.exit_if_requested(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.apply_commands(event_loop);
        if self.exit_if_requested(event_loop) {
            return;
        }
        // The preceding platform/default-action borrows have ended. Rust and
        // C applications use this same adapter and retained Document queue.
        let mut pending = self.run_tasks(event_loop);
        if event_loop.exiting() {
            return;
        }
        for index in 0..self.windows.len() {
            match self.windows[index].application.dispatch_pending_events() {
                Ok(more) => pending |= more,
                Err(error) => return self.fail(event_loop, PlatformError::Application(error)),
            }
        }
        self.apply_commands(event_loop);
        if self.exit_if_requested(event_loop) {
            return;
        }
        self.request_frames();
        event_loop.set_control_flow(if pending {
            ControlFlow::Poll
        } else {
            ControlFlow::Wait
        });
    }
}

/// Opens one window for `application` and runs until it closes.
struct SingleWindow(Option<WindowCommand>);

impl WindowController for SingleWindow {
    fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String> {
        Ok(self.0.take().into_iter().collect())
    }
}

/// Run one window until it closes or the application requests exit.
pub fn run<A: PlatformApplication>(
    application: A,
    options: WindowOptions,
) -> Result<(), PlatformError> {
    run_windows(SingleWindow(Some(WindowCommand::Open {
        id: crate::WindowId(0),
        options,
        application: Box::new(application),
    })))
}

/// Run the native event loop with the windows `controller` opens, until it
/// asks to exit, by default after its last window closes.
pub fn run_windows<C: WindowController>(mut controller: C) -> Result<(), PlatformError> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let display = event_loop.owned_display_handle();
    let clipboard = Rc::new(NativeClipboard::new(&display)?);
    let proxy = event_loop.create_proxy();
    if std::env::var_os(DEV_ENV).is_some() {
        watch_dev_stdin(proxy.clone())?;
    }
    let wake_pending = Arc::new(AtomicBool::new(false));
    controller.started({
        let proxy = proxy.clone();
        let pending = wake_pending.clone();
        LoopWaker::new(move || {
            if !pending.swap(true, Ordering::AcqRel) {
                // Fails only once the loop has exited, when there is
                // nothing left to wake.
                let _ = proxy.send_event(UserEvent::Wake);
            }
        })
    });
    // Controllers get a weak handle: the clipboard must drop with the runtime,
    // before the display connection it was created from closes.
    let weak: std::rc::Weak<NativeClipboard> = Rc::downgrade(&clipboard);
    controller.clipboard(Clipboard::new(weak));
    let mut runtime = Runtime {
        windows: Vec::new(),
        controller,
        clipboard,
        display,
        proxy,
        started: Instant::now(),
        resumed: false,
        error: None,
        frame_round: 1,
        frame_round_started: Instant::now(),
        wake_pending,
    };
    event_loop
        .run_app(&mut runtime)
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    runtime.error.map_or(Ok(()), Err)
}

/// Under `openui dev`, exit when the parent closes stdin. Reading blocks a
/// helper thread only; it ends with the process.
fn watch_dev_stdin(proxy: EventLoopProxy<UserEvent>) -> Result<(), PlatformError> {
    std::thread::Builder::new()
        .name("openui-dev-stdin".into())
        .spawn(move || {
            let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
            let _ = proxy.send_event(UserEvent::DevExit);
        })
        .map(drop)
        .map_err(|error| PlatformError::Initialization(error.to_string()))
}

fn pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Left,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Right => PointerButton::Right,
        MouseButton::Back => PointerButton::Other(4),
        MouseButton::Forward => PointerButton::Other(5),
        MouseButton::Other(value) => PointerButton::Other(value),
    }
}

fn winit_cursor(cursor: CursorIcon) -> WinitCursorIcon {
    match cursor {
        CursorIcon::Default => WinitCursorIcon::Default,
        CursorIcon::Pointer => WinitCursorIcon::Pointer,
        CursorIcon::Text => WinitCursorIcon::Text,
        CursorIcon::Move => WinitCursorIcon::Move,
        CursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
    }
}

fn logical_key_text(key: &Key) -> Option<String> {
    match key {
        Key::Character(value) => Some(value.to_string()),
        Key::Named(value) => Some(named_key_name(*value).to_owned()),
        Key::Dead(value) => value.map(|value| value.to_string()),
        Key::Unidentified(_) => None,
    }
}

fn named_key_name(key: NamedKey) -> &'static str {
    match key {
        NamedKey::Tab => "Tab",
        NamedKey::Enter => "Enter",
        NamedKey::Space => " ",
        NamedKey::ArrowLeft => "ArrowLeft",
        NamedKey::ArrowRight => "ArrowRight",
        NamedKey::ArrowUp => "ArrowUp",
        NamedKey::ArrowDown => "ArrowDown",
        NamedKey::Home => "Home",
        NamedKey::End => "End",
        NamedKey::PageUp => "PageUp",
        NamedKey::PageDown => "PageDown",
        NamedKey::Backspace => "Backspace",
        NamedKey::Delete => "Delete",
        NamedKey::Escape => "Escape",
        _ => "",
    }
}

fn key_code(key: &Key) -> i32 {
    match key {
        Key::Named(NamedKey::Tab) => 9,
        Key::Named(NamedKey::Enter) => 13,
        Key::Named(NamedKey::Space) => 32,
        Key::Named(NamedKey::PageUp) => 33,
        Key::Named(NamedKey::PageDown) => 34,
        Key::Named(NamedKey::End) => 35,
        Key::Named(NamedKey::Home) => 36,
        Key::Named(NamedKey::ArrowLeft) => 37,
        Key::Named(NamedKey::ArrowUp) => 38,
        Key::Named(NamedKey::ArrowRight) => 39,
        Key::Named(NamedKey::ArrowDown) => 40,
        Key::Named(NamedKey::Delete) => 46,
        Key::Named(NamedKey::Backspace) => 8,
        Key::Named(NamedKey::Escape) => 27,
        // Character code points collide with the navigation/control codes.
        // Letters and digits keep their conventional codes; other characters
        // carry their identity in key_text and committed text.
        Key::Character(value) => {
            let mut characters = value.chars();
            characters.next().map_or(0, |character| {
                if character.is_ascii_alphanumeric() && characters.next().is_none() {
                    character.to_ascii_uppercase() as i32
                } else {
                    0
                }
            })
        }
        _ => 0,
    }
}

fn copy_frame(
    frame: &SoftwareFrame,
    destination_width: u32,
    destination_height: u32,
    destination: &mut [u32],
) -> Result<(), PlatformError> {
    if frame.width != destination_width
        || frame.height != destination_height
        || frame.width == 0
        || frame.height == 0
        || frame.stride < frame.width as usize * 4
        || frame.pixels.len() < frame.stride * frame.height as usize
        || destination.len() < destination_width as usize * destination_height as usize
    {
        return Err(PlatformError::Presentation(
            "application returned an invalid software frame".into(),
        ));
    }
    for y in 0..destination_height {
        for x in 0..destination_width {
            let source = y as usize * frame.stride + x as usize * 4;
            let red = u32::from(frame.pixels[source]);
            let green = u32::from(frame.pixels[source + 1]);
            let blue = u32::from(frame.pixels[source + 2]);
            destination[y as usize * destination_width as usize + x as usize] =
                blue | (green << 8) | (red << 16);
        }
    }
    Ok(())
}

fn empty_tree_update() -> accesskit::TreeUpdate {
    accesskit::TreeUpdate {
        nodes: Vec::new(),
        tree: None,
        tree_id: accesskit::TreeId::ROOT,
        focus: accesskit::NodeId(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ime_normalization_preserves_commit_after_the_synthetic_empty_preview() {
        let mut state = ImeState::default();
        state.focus_changed(true);
        assert!(state.events(Ime::Enabled).is_empty());
        assert!(matches!(
            state.events(Ime::Preedit("e".into(), Some((1, 1)))).as_slice(),
            [PlatformEvent::CompositionStart, PlatformEvent::CompositionUpdate(text)] if text == "e"
        ));
        assert!(matches!(
            state.events(Ime::Preedit(String::new(), None)).as_slice(),
            [PlatformEvent::CompositionUpdate(text)] if text.is_empty()
        ));
        assert!(matches!(
            state.events(Ime::Commit("é".into())).as_slice(),
            [PlatformEvent::CompositionEnd(text)] if text == "é"
        ));
        assert!(matches!(
            state.events(Ime::Commit("direct".into())).as_slice(),
            [PlatformEvent::TextInput(text)] if text == "direct"
        ));
    }

    #[test]
    fn ime_focus_loss_and_disable_cancel_without_inserting_a_late_commit() {
        let mut state = ImeState::default();
        state.focus_changed(true);
        state.events(Ime::Enabled);
        state.events(Ime::Preedit("preview".into(), Some((0, 0))));
        assert!(matches!(
            state.focus_changed(false),
            Some(PlatformEvent::CompositionEnd(text)) if text.is_empty()
        ));
        assert!(state.events(Ime::Commit("late".into())).is_empty());
        state.focus_changed(true);
        assert!(matches!(
            state.events(Ime::Preedit("new".into(), Some((0, 0)))).as_slice(),
            [PlatformEvent::CompositionStart, PlatformEvent::CompositionUpdate(text)] if text == "new"
        ));
        assert!(matches!(
            state.events(Ime::Disabled).as_slice(),
            [PlatformEvent::CompositionEnd(text)] if text.is_empty()
        ));
        assert!(state.events(Ime::Commit("late".into())).is_empty());
        assert!(state
            .events(Ime::Preedit("late".into(), Some((0, 0))))
            .is_empty());
    }

    #[test]
    fn character_keys_do_not_alias_navigation_and_editing_keys() {
        for text in ["!", "\"", "#", "$", "%", "&", "'", "(", ".", "é", "é"] {
            assert_eq!(key_code(&Key::Character(text.into())), 0, "{text}");
        }
        assert_eq!(key_code(&Key::Character("a".into())), 65);
        assert_eq!(key_code(&Key::Character("1".into())), 49);
        assert_eq!(key_code(&Key::Named(NamedKey::Delete)), 46);
        assert_eq!(key_code(&Key::Named(NamedKey::ArrowLeft)), 37);
        assert_eq!(
            logical_key_text(&Key::Named(NamedKey::Escape)).as_deref(),
            Some("Escape")
        );
    }

    #[test]
    fn software_frame_is_copied_without_resampling() {
        let viewport = openui_geometry::ViewportMetrics::from_physical_size(2, 1, 1.0).unwrap();
        let frame = SoftwareFrame {
            width: 2,
            height: 1,
            viewport,
            stride: 8,
            pixels: vec![255, 0, 0, 255, 0, 0, 255, 255],
        };
        let mut output = [0; 2];
        copy_frame(&frame, 2, 1, &mut output).unwrap();
        assert_eq!(output, [0xff0000, 0x0000ff]);
        assert!(copy_frame(&frame, 4, 2, &mut [0; 8]).is_err());
    }

    #[test]
    fn invalid_frame_layout_is_rejected_before_native_upload() {
        let viewport = openui_geometry::ViewportMetrics::from_physical_size(2, 2, 1.0).unwrap();
        let short = SoftwareFrame {
            width: 2,
            height: 2,
            viewport,
            stride: 8,
            pixels: vec![0; 15],
        };
        assert!(matches!(
            validate_frame(&short),
            Err(PlatformError::Presentation(_))
        ));

        let padded = SoftwareFrame {
            width: 2,
            height: 2,
            viewport,
            stride: 12,
            pixels: vec![0; 24],
        };
        validate_frame(&padded).unwrap();
    }
}
