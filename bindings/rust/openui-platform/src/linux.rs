use crate::{
    BackendPreference, BackendStatus, CursorIcon, KeyPhase, Modifiers, PlatformApplication,
    PlatformError, PlatformEvent, PointerButton, PointerPhase, SoftwareFrame, WindowOptions,
};
use accesskit_winit::{Adapter as AccessibilityAdapter, WindowEvent as AccessibilityEvent};
use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, GlContext, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::{GlSurface, NotCurrentGlContext};
use glutin::surface::{Surface as GlutinSurface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use softbuffer::{Context as SoftContext, Surface as SoftSurface};
use std::ffi::CString;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
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
}

impl From<accesskit_winit::Event> for UserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::Accessibility(event)
    }
}

enum NativeClipboard {
    Wayland(Box<smithay_clipboard::Clipboard>),
    X11(Box<x11_clipboard::Clipboard>),
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
        // SAFETY: this presenter owns these names and its glutin context is
        // current on the event-loop thread for its entire lifetime.
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteProgram(self.program);
        }
    }
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
        .with_visible(false);
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

impl NativeClipboard {
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
            Self::Wayland(clipboard) => clipboard
                .load()
                .map_err(|error| PlatformError::Application(error.to_string())),
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
            Self::Wayland(clipboard) => clipboard.store(text),
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

struct Runtime<A: PlatformApplication> {
    application: A,
    options: WindowOptions,
    display: OwnedDisplayHandle,
    clipboard: NativeClipboard,
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    presenter: Option<Presenter>,
    accessibility: Option<AccessibilityAdapter>,
    cursor: LogicalPosition<f64>,
    scale_factor: f64,
    modifiers: Modifiers,
    ime: ImeState,
    started: Instant,
    error: Option<PlatformError>,
    presented_frames: u64,
}

impl<A: PlatformApplication> Runtime<A> {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
        event_loop.exit();
    }

    fn send(&mut self, event_loop: &ActiveEventLoop, event: PlatformEvent) -> bool {
        match self.application.event(event) {
            Ok(()) => true,
            Err(error) => {
                self.fail(event_loop, PlatformError::Application(error));
                false
            }
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn update_accessibility(&mut self, event_loop: &ActiveEventLoop) {
        let Some(adapter) = self.accessibility.as_mut() else {
            return;
        };
        let mut error = None;
        adapter.update_if_active(|| match self.application.accessibility_update() {
            Ok(update) => update,
            Err(message) => {
                error = Some(PlatformError::Application(message));
                empty_tree_update()
            }
        });
        if let Some(error) = error {
            self.fail(event_loop, error);
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let physical = window.inner_size();
        if physical.width == 0 || physical.height == 0 {
            return;
        }
        let elapsed_ms = self.started.elapsed().as_secs_f64() * 1000.0;
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
                match software_presenter(&self.display, window) {
                    Ok(presenter) => {
                        self.presenter = Some(presenter);
                        let status = BackendStatus {
                            active: BackendPreference::Software,
                            fallback_reason: Some(error.to_string()),
                        };
                        if self.send(event_loop, PlatformEvent::BackendChanged(status)) {
                            self.request_redraw();
                        }
                    }
                    Err(fallback_error) => self.fail(event_loop, fallback_error),
                }
                return;
            }
            self.fail(event_loop, error);
            return;
        }
        self.presented_frames = self.presented_frames.saturating_add(1);
        if !self.send(
            event_loop,
            PlatformEvent::Presented {
                frame_number: self.presented_frames,
                time_ms: elapsed_ms,
            },
        ) {
            return;
        }
        self.update_accessibility(event_loop);
        if self.application.is_animating() || self.application.needs_redraw() {
            self.request_redraw();
        }
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, event: winit::event::KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let key_text = logical_key_text(&event.logical_key);
        let shortcut = key_text.as_deref().map(str::to_ascii_lowercase);
        let command_modifier = self.modifiers.control || self.modifiers.meta;
        if pressed && command_modifier && shortcut.as_deref() == Some("v") {
            if let Ok(text) = self.clipboard.load() {
                if let Err(error) = self.application.set_clipboard_text(text) {
                    self.fail(event_loop, PlatformError::Application(error));
                    return;
                }
            }
        }
        if let Err(error) = self.application.key_input(crate::KeyboardInput {
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
        }) {
            self.fail(event_loop, PlatformError::Application(error));
            return;
        }
        if pressed && command_modifier && matches!(shortcut.as_deref(), Some("c") | Some("x")) {
            if let Ok(text) = self.application.clipboard_text() {
                self.clipboard.store(text);
            }
        }
        self.request_redraw();
    }

    fn handle_accessibility_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: accesskit_winit::Event,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(event.window_id) {
            return;
        }
        match event.window_event {
            AccessibilityEvent::InitialTreeRequested => self.update_accessibility(event_loop),
            AccessibilityEvent::ActionRequested(request) => {
                if let Err(error) = self.application.accessibility_action(request) {
                    self.fail(event_loop, PlatformError::Application(error));
                    return;
                }
                self.request_redraw();
            }
            AccessibilityEvent::AccessibilityDeactivated => {}
        }
    }
}

impl<A: PlatformApplication> ApplicationHandler<UserEvent> for Runtime<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (window, presenter, status) =
            match initialize_presenter(event_loop, &self.display, &self.options) {
                Ok(initialized) => initialized,
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            };
        self.scale_factor = window.scale_factor();
        let physical = window.inner_size();
        let accessibility =
            AccessibilityAdapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
        window.set_ime_allowed(true);
        window.set_visible(true);
        self.presenter = Some(presenter);
        self.accessibility = Some(accessibility);
        self.window = Some(window);
        if !self.send(event_loop, PlatformEvent::BackendChanged(status)) {
            return;
        }
        if self.send(
            event_loop,
            PlatformEvent::Resized(
                openui_geometry::ViewportMetrics::from_physical_size(
                    physical.width,
                    physical.height,
                    self.scale_factor,
                )
                .expect("winit returned validated window metrics"),
            ),
        ) {
            self.request_redraw();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Accessibility(event) => self.handle_accessibility_event(event_loop, event),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if window.id() != window_id {
            return;
        }
        if let Some(adapter) = self.accessibility.as_mut() {
            adapter.process_event(&window, &event);
        }
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                let _ = self.send(event_loop, PlatformEvent::CloseRequested);
                event_loop.exit();
            }
            WindowEvent::Resized(physical) => {
                if physical.width > 0
                    && physical.height > 0
                    && self.send(
                        event_loop,
                        PlatformEvent::Resized(
                            openui_geometry::ViewportMetrics::from_physical_size(
                                physical.width,
                                physical.height,
                                self.scale_factor,
                            )
                            .expect("winit returned validated window metrics"),
                        ),
                    )
                {
                    self.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                let physical = window.inner_size();
                if self.send(
                    event_loop,
                    PlatformEvent::Resized(
                        openui_geometry::ViewportMetrics::from_physical_size(
                            physical.width,
                            physical.height,
                            scale_factor,
                        )
                        .expect("winit returned validated window metrics"),
                    ),
                ) {
                    self.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = position.to_logical(self.scale_factor);
                let cursor = self.cursor;
                let modifiers = self.modifiers;
                if self.send(
                    event_loop,
                    PlatformEvent::Pointer {
                        pointer_id: 0,
                        phase: PointerPhase::Move,
                        x: cursor.x as f32,
                        y: cursor.y as f32,
                        button: PointerButton::Other(0),
                        modifiers,
                    },
                ) {
                    match self
                        .application
                        .cursor_icon(cursor.x as f32, cursor.y as f32)
                    {
                        Ok(icon) => window.set_cursor(winit_cursor(icon)),
                        Err(error) => {
                            self.fail(event_loop, PlatformError::Application(error));
                            return;
                        }
                    }
                    self.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                let cursor = self.cursor;
                let modifiers = self.modifiers;
                if self.send(
                    event_loop,
                    PlatformEvent::Pointer {
                        pointer_id: 0,
                        phase: PointerPhase::Leave,
                        x: cursor.x as f32,
                        y: cursor.y as f32,
                        button: PointerButton::Other(0),
                        modifiers,
                    },
                ) {
                    self.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let cursor = self.cursor;
                let modifiers = self.modifiers;
                if self.send(
                    event_loop,
                    PlatformEvent::Pointer {
                        pointer_id: 0,
                        phase: if state == ElementState::Pressed {
                            PointerPhase::Down
                        } else {
                            PointerPhase::Up
                        },
                        x: cursor.x as f32,
                        y: cursor.y as f32,
                        button: pointer_button(button),
                        modifiers,
                    },
                ) {
                    self.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
                    MouseScrollDelta::PixelDelta(position) => (
                        (position.x / self.scale_factor) as f32,
                        (position.y / self.scale_factor) as f32,
                    ),
                };
                let cursor = self.cursor;
                let modifiers = self.modifiers;
                if self.send(
                    event_loop,
                    PlatformEvent::Wheel {
                        x: cursor.x as f32,
                        y: cursor.y as f32,
                        delta_x,
                        delta_y,
                        modifiers,
                    },
                ) {
                    self.request_redraw();
                }
            }
            WindowEvent::Touch(touch) => {
                let logical = touch.location.to_logical::<f64>(self.scale_factor);
                let phase = match touch.phase {
                    TouchPhase::Started => PointerPhase::Down,
                    TouchPhase::Moved => PointerPhase::Move,
                    TouchPhase::Ended => PointerPhase::Up,
                    TouchPhase::Cancelled => PointerPhase::Cancel,
                };
                let modifiers = self.modifiers;
                if self.send(
                    event_loop,
                    PlatformEvent::Pointer {
                        pointer_id: touch.id.saturating_add(1),
                        phase,
                        x: logical.x as f32,
                        y: logical.y as f32,
                        button: PointerButton::Left,
                        modifiers,
                    },
                ) {
                    self.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event_loop, event),
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
                    if !self.send(event_loop, event) {
                        break;
                    }
                    self.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(event) = self.ime.focus_changed(focused) {
                    if !self.send(event_loop, event) {
                        return;
                    }
                }
                if self.send(event_loop, PlatformEvent::Focused(focused)) {
                    self.request_redraw();
                }
            }
            WindowEvent::DroppedFile(path) => {
                let _ = self.send(event_loop, PlatformEvent::DroppedFile(path));
            }
            WindowEvent::HoveredFile(path) => {
                let _ = self.send(event_loop, PlatformEvent::HoveredFile(path));
            }
            WindowEvent::HoveredFileCancelled => {
                let _ = self.send(event_loop, PlatformEvent::HoveredFileCancelled);
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
        if self.application.exit_requested() {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.application.exit_requested() {
            event_loop.exit();
        } else if self.application.is_animating() || self.application.needs_redraw() {
            self.request_redraw();
        }
    }
}

pub fn run<A: PlatformApplication>(
    application: A,
    options: WindowOptions,
) -> Result<(), PlatformError> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let display = event_loop.owned_display_handle();
    let clipboard = NativeClipboard::new(&display)?;
    let proxy = event_loop.create_proxy();
    let mut runtime = Runtime {
        application,
        options,
        display,
        clipboard,
        proxy,
        window: None,
        presenter: None,
        accessibility: None,
        cursor: LogicalPosition::new(0.0, 0.0),
        scale_factor: 1.0,
        modifiers: Modifiers::default(),
        ime: ImeState::default(),
        started: Instant::now(),
        error: None,
        presented_frames: 0,
    };
    event_loop
        .run_app(&mut runtime)
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    runtime.error.map_or(Ok(()), Err)
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
