use crate::{
    BackendPreference, CursorIcon, KeyPhase, Modifiers, PlatformApplication, PlatformError,
    PlatformEvent, PointerButton, PointerPhase, SoftwareFrame, WindowOptions,
};
use accesskit_winit::{Adapter as AccessibilityAdapter, WindowEvent as AccessibilityEvent};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use softbuffer::{Context, Surface};
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

struct Runtime<A: PlatformApplication> {
    application: A,
    options: WindowOptions,
    context: Context<OwnedDisplayHandle>,
    clipboard: NativeClipboard,
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    surface: Option<Surface<OwnedDisplayHandle, Arc<Window>>>,
    accessibility: Option<AccessibilityAdapter>,
    cursor: LogicalPosition<f64>,
    scale_factor: f64,
    modifiers: Modifiers,
    composing: bool,
    started: Instant,
    error: Option<PlatformError>,
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
        let frame = match self.application.render(elapsed_ms) {
            Ok(frame) => frame,
            Err(error) => {
                self.fail(event_loop, PlatformError::Application(error));
                return;
            }
        };
        let result = (|| {
            let surface = self.surface.as_mut().ok_or_else(|| {
                PlatformError::Presentation("software surface is unavailable".into())
            })?;
            surface
                .resize(
                    NonZeroU32::new(physical.width).expect("nonzero width checked"),
                    NonZeroU32::new(physical.height).expect("nonzero height checked"),
                )
                .map_err(|error| PlatformError::Presentation(error.to_string()))?;
            let mut buffer = surface
                .buffer_mut()
                .map_err(|error| PlatformError::Presentation(error.to_string()))?;
            copy_scaled_frame(&frame, physical.width, physical.height, &mut buffer)?;
            buffer
                .present()
                .map_err(|error| PlatformError::Presentation(error.to_string()))
        })();
        if let Err(error) = result {
            self.fail(event_loop, error);
            return;
        }
        self.update_accessibility(event_loop);
        if self.application.is_animating() {
            self.request_redraw();
        }
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, event: winit::event::KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let text = event
            .text
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| logical_key_text(&event.logical_key));
        let shortcut = text.as_deref().map(str::to_ascii_lowercase);
        let command_modifier = self.modifiers.control || self.modifiers.meta;
        if pressed && command_modifier && shortcut.as_deref() == Some("v") {
            if let Ok(text) = self.clipboard.load() {
                if let Err(error) = self.application.set_clipboard_text(text) {
                    self.fail(event_loop, PlatformError::Application(error));
                    return;
                }
            }
        }
        if !self.send(
            event_loop,
            PlatformEvent::Key {
                phase: if pressed {
                    KeyPhase::Down
                } else {
                    KeyPhase::Up
                },
                key_code: key_code(&event.logical_key),
                text: text.clone(),
                modifiers: self.modifiers,
                repeat: event.repeat,
            },
        ) {
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
        let attributes = WindowAttributes::default()
            .with_title(self.options.title.clone())
            .with_inner_size(LogicalSize::new(self.options.width, self.options.height))
            .with_visible(false);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, PlatformError::Window(error.to_string()));
                return;
            }
        };
        let surface = match Surface::new(&self.context, window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                self.fail(event_loop, PlatformError::Presentation(error.to_string()));
                return;
            }
        };
        self.scale_factor = window.scale_factor();
        let size = window.inner_size().to_logical::<f64>(self.scale_factor);
        let accessibility =
            AccessibilityAdapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
        window.set_ime_allowed(true);
        window.set_visible(true);
        self.surface = Some(surface);
        self.accessibility = Some(accessibility);
        self.window = Some(window);
        if self.send(
            event_loop,
            PlatformEvent::Resized {
                logical_width: logical_dimension(size.width),
                logical_height: logical_dimension(size.height),
                scale_factor: self.scale_factor,
            },
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
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Resized(physical) => {
                let logical = physical.to_logical::<f64>(self.scale_factor);
                if physical.width > 0
                    && physical.height > 0
                    && self.send(
                        event_loop,
                        PlatformEvent::Resized {
                            logical_width: logical_dimension(logical.width),
                            logical_height: logical_dimension(logical.height),
                            scale_factor: self.scale_factor,
                        },
                    )
                {
                    self.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                let physical = window.inner_size();
                let logical = physical.to_logical::<f64>(scale_factor);
                if self.send(
                    event_loop,
                    PlatformEvent::Resized {
                        logical_width: logical_dimension(logical.width),
                        logical_height: logical_dimension(logical.height),
                        scale_factor,
                    },
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
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => {}
                Ime::Preedit(text, _) => {
                    if !self.composing && !text.is_empty() {
                        self.composing = self.send(event_loop, PlatformEvent::CompositionStart);
                    }
                    if self.composing
                        && self.send(event_loop, PlatformEvent::CompositionUpdate(text))
                    {
                        self.request_redraw();
                    }
                }
                Ime::Commit(text) => {
                    let event = if self.composing {
                        self.composing = false;
                        PlatformEvent::CompositionEnd(text)
                    } else {
                        PlatformEvent::TextInput(text)
                    };
                    if self.send(event_loop, event) {
                        self.request_redraw();
                    }
                }
                Ime::Disabled => {
                    if self.composing {
                        self.composing = false;
                        let _ = self.send(event_loop, PlatformEvent::CompositionEnd(String::new()));
                    }
                }
            },
            WindowEvent::Focused(focused) => {
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
        } else if self.application.is_animating() {
            self.request_redraw();
        }
    }
}

pub fn run<A: PlatformApplication>(
    application: A,
    options: WindowOptions,
) -> Result<(), PlatformError> {
    if options.backend == BackendPreference::OpenGl {
        return Err(PlatformError::Initialization(
            "the OpenGL backend is introduced by compositor wave W8".into(),
        ));
    }
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let display = event_loop.owned_display_handle();
    let context = Context::new(display.clone())
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    let clipboard = NativeClipboard::new(&display)?;
    let proxy = event_loop.create_proxy();
    let mut runtime = Runtime {
        application,
        options,
        context,
        clipboard,
        proxy,
        window: None,
        surface: None,
        accessibility: None,
        cursor: LogicalPosition::new(0.0, 0.0),
        scale_factor: 1.0,
        modifiers: Modifiers::default(),
        composing: false,
        started: Instant::now(),
        error: None,
    };
    event_loop
        .run_app(&mut runtime)
        .map_err(|error| PlatformError::Initialization(error.to_string()))?;
    runtime.error.map_or(Ok(()), Err)
}

fn logical_dimension(value: f64) -> u32 {
    value.round().clamp(1.0, u32::MAX as f64) as u32
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
        Key::Character(value) => value.chars().next().map_or(0, |value| value as i32),
        _ => 0,
    }
}

fn copy_scaled_frame(
    frame: &SoftwareFrame,
    destination_width: u32,
    destination_height: u32,
    destination: &mut [u32],
) -> Result<(), PlatformError> {
    if frame.width == 0
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
        let source_y =
            (u64::from(y) * u64::from(frame.height) / u64::from(destination_height)) as usize;
        for x in 0..destination_width {
            let source_x =
                (u64::from(x) * u64::from(frame.width) / u64::from(destination_width)) as usize;
            let source = source_y * frame.stride + source_x * 4;
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
    fn software_frame_is_scaled_and_converted_for_softbuffer() {
        let frame = SoftwareFrame {
            width: 2,
            height: 1,
            stride: 8,
            pixels: vec![255, 0, 0, 255, 0, 0, 255, 255],
        };
        let mut output = [0; 8];
        copy_scaled_frame(&frame, 4, 2, &mut output).unwrap();
        assert_eq!(
            output,
            [0xff0000, 0xff0000, 0x0000ff, 0x0000ff, 0xff0000, 0xff0000, 0x0000ff, 0x0000ff]
        );
    }
}
