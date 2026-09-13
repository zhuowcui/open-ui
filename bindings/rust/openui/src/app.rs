//! Fallible application and deterministic headless lifecycle APIs.

use crate::context::with_document;
use crate::scope::{create_scope, dispose_scope};
use crate::style::{Bitmap, Error};
use crate::view_node::{mount_view, IntoView};
use crate::{Document, ScopeId};
use openui_engine::Viewport;
use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

impl LogicalSize {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendPreference {
    #[default]
    Auto,
    OpenGl,
    Software,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowOptions {
    pub title: String,
    pub size: LogicalSize,
    pub backend: BackendPreference,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "Open UI".to_owned(),
            size: LogicalSize::new(800.0, 600.0),
            backend: BackendPreference::Auto,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppBuilder {
    options: WindowOptions,
}

impl AppBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = title.into();
        self
    }

    pub fn size(mut self, size: LogicalSize) -> Self {
        self.options.size = size;
        self
    }

    pub fn backend(mut self, backend: BackendPreference) -> Self {
        self.options.backend = backend;
        self
    }

    pub fn build(self) -> Result<App, Error> {
        let width = dimension(self.options.size.width)?;
        let height = dimension(self.options.size.height)?;
        Ok(App {
            document: Document::with_viewport(Viewport::new(width, height)?)?,
            options: self.options,
            root_scope: None,
            exit_requested: Cell::new(false),
        })
    }
}

fn dimension(value: f64) -> Result<u32, Error> {
    if !value.is_finite() || value <= 0.0 || value > u32::MAX as f64 {
        return Err(Error::InvalidArgument(
            "window dimensions must be finite and positive",
        ));
    }
    Ok(value.round() as u32)
}

pub struct App {
    document: Document,
    options: WindowOptions,
    root_scope: Option<ScopeId>,
    exit_requested: Cell<bool>,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn options(&self) -> &WindowOptions {
        &self.options
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn mount<V: IntoView>(&mut self, view: impl FnOnce() -> V) -> Result<(), Error> {
        if let Some(scope) = self.root_scope.take() {
            dispose_scope(scope);
        }
        self.document.body().remove_all_children()?;
        let document = self.document.clone();
        let scope = create_scope(|| {
            let view = with_document(&document, || view().into_view());
            mount_view(&document.body(), view);
        });
        self.root_scope = Some(scope);
        self.document.update_all()
    }

    /// Run the owned native event loop.
    pub fn run<V: IntoView>(mut self, view: impl FnOnce() -> V) -> Result<(), Error> {
        self.mount(view)?;
        #[cfg(all(feature = "linux", target_os = "linux"))]
        {
            let options = openui_platform::WindowOptions {
                title: self.options.title.clone(),
                width: self.options.size.width,
                height: self.options.size.height,
                backend: match self.options.backend {
                    BackendPreference::Auto => openui_platform::BackendPreference::Auto,
                    BackendPreference::OpenGl => openui_platform::BackendPreference::OpenGl,
                    BackendPreference::Software => openui_platform::BackendPreference::Software,
                },
            };
            openui_platform::run(self, options).map_err(|error| Error::Platform(error.to_string()))
        }
        #[cfg(not(all(feature = "linux", target_os = "linux")))]
        Err(Error::PlatformUnavailable)
    }

    pub fn request_exit(&self) {
        self.exit_requested.set(true);
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested.get()
    }
}

#[cfg(all(feature = "linux", target_os = "linux"))]
impl openui_platform::PlatformApplication for App {
    fn event(&mut self, event: openui_platform::PlatformEvent) -> Result<(), String> {
        use openui_engine::PointerEventKind;
        use openui_platform::{KeyPhase, PlatformEvent, PointerButton, PointerPhase};

        let modifiers = |value: openui_platform::Modifiers| {
            let mut result = crate::Modifiers::NONE;
            if value.shift {
                result |= crate::Modifiers::SHIFT;
            }
            if value.control {
                result |= crate::Modifiers::CTRL;
            }
            if value.alt {
                result |= crate::Modifiers::ALT;
            }
            if value.meta {
                result |= crate::Modifiers::META;
            }
            result
        };
        let button = |value: PointerButton| match value {
            PointerButton::Left => crate::MouseButton::Left,
            PointerButton::Middle => crate::MouseButton::Middle,
            PointerButton::Right | PointerButton::Other(_) => crate::MouseButton::Right,
        };
        let result = match event {
            PlatformEvent::Resized {
                logical_width,
                logical_height,
                scale_factor,
            } => self
                .document
                .set_viewport_with_scale(logical_width, logical_height, scale_factor),
            PlatformEvent::Pointer {
                pointer_id,
                phase,
                x,
                y,
                button: pointer,
                modifiers: keys,
            } => self.document.dispatch_pointer_event(
                pointer_id,
                match phase {
                    PointerPhase::Move => PointerEventKind::Move,
                    PointerPhase::Down => PointerEventKind::Down,
                    PointerPhase::Up => PointerEventKind::Up,
                    PointerPhase::Cancel | PointerPhase::Leave => PointerEventKind::Cancel,
                },
                x,
                y,
                button(pointer),
                modifiers(keys),
                pointer_id != 0,
            ),
            PlatformEvent::Wheel {
                x,
                y,
                delta_x,
                delta_y,
                modifiers: keys,
            } => self
                .document
                .dispatch_wheel_event(x, y, delta_x, delta_y, modifiers(keys)),
            PlatformEvent::Key {
                phase,
                key_code,
                text,
                modifiers: keys,
                ..
            } => {
                let keys = modifiers(keys);
                let dispatched = self.document.dispatch_key_event(
                    match phase {
                        KeyPhase::Down => crate::KeyEventType::Down,
                        KeyPhase::Up => crate::KeyEventType::Up,
                    },
                    key_code,
                    text.as_deref(),
                    keys,
                );
                if dispatched.is_ok()
                    && phase == KeyPhase::Down
                    && !keys.contains(crate::Modifiers::CTRL)
                    && !keys.contains(crate::Modifiers::META)
                    && text.as_deref().is_some_and(is_text_input)
                {
                    self.document
                        .dispatch_text_input(text.as_deref().expect("text was checked"))
                } else {
                    dispatched
                }
            }
            PlatformEvent::TextInput(text) => self.document.dispatch_text_input(&text),
            PlatformEvent::CompositionStart => self.document.dispatch_composition_start(),
            PlatformEvent::CompositionUpdate(text) => {
                self.document.dispatch_composition_update(&text)
            }
            PlatformEvent::CompositionEnd(text) => self.document.dispatch_composition_end(&text),
            PlatformEvent::Focused(_)
            | PlatformEvent::DroppedFile(_)
            | PlatformEvent::HoveredFile(_)
            | PlatformEvent::HoveredFileCancelled => Ok(()),
        };
        result.map_err(|error| error.to_string())
    }

    fn render(&mut self, time_ms: f64) -> Result<openui_platform::SoftwareFrame, String> {
        self.document
            .begin_frame(time_ms)
            .and_then(|_| self.document.render_to_bitmap())
            .map(|bitmap| openui_platform::SoftwareFrame {
                width: bitmap.width,
                height: bitmap.height,
                stride: bitmap.stride,
                pixels: bitmap.pixels,
            })
            .map_err(|error| error.to_string())
    }

    fn exit_requested(&self) -> bool {
        self.exit_requested()
    }

    fn clipboard_text(&self) -> Result<String, String> {
        self.document
            .clipboard_text()
            .map_err(|error| error.to_string())
    }

    fn set_clipboard_text(&mut self, text: String) -> Result<(), String> {
        self.document
            .set_clipboard_text(text)
            .map_err(|error| error.to_string())
    }

    fn cursor_icon(&mut self, x: f32, y: f32) -> Result<openui_platform::CursorIcon, String> {
        self.document
            .with_engine_mut(|engine| {
                let Some(target) = engine.hit_test(x, y)? else {
                    return Ok(openui_platform::CursorIcon::Default);
                };
                Ok(match engine.cursor(target)? {
                    openui_style::Cursor::Auto | openui_style::Cursor::Default => {
                        openui_platform::CursorIcon::Default
                    }
                    openui_style::Cursor::Pointer => openui_platform::CursorIcon::Pointer,
                    openui_style::Cursor::Text => openui_platform::CursorIcon::Text,
                    openui_style::Cursor::Move => openui_platform::CursorIcon::Move,
                    openui_style::Cursor::NotAllowed => openui_platform::CursorIcon::NotAllowed,
                })
            })
            .map_err(|error| error.to_string())
    }

    fn accessibility_update(&mut self) -> Result<openui_platform::TreeUpdate, String> {
        self.document
            .accessibility_update()
            .map_err(|error| error.to_string())
    }

    fn accessibility_action(
        &mut self,
        request: openui_platform::ActionRequest,
    ) -> Result<(), String> {
        let (target, action) = self
            .document
            .with_engine(|engine| engine.accessibility_action_from_request(&request))
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        self.document
            .perform_accessibility_action(target, action)
            .map_err(|error| error.to_string())
    }
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn is_text_input(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|character| !character.is_control())
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(scope) = self.root_scope.take() {
            dispose_scope(scope);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub time_ms: f64,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { time_ms: 0.0 }
    }
}

pub struct HeadlessApp {
    app: App,
}

impl HeadlessApp {
    pub fn new(viewport: Viewport) -> Result<Self, Error> {
        Ok(Self {
            app: App {
                document: Document::with_viewport(viewport)?,
                options: WindowOptions {
                    size: LogicalSize::new(viewport.width as f64, viewport.height as f64),
                    ..WindowOptions::default()
                },
                root_scope: None,
                exit_requested: Cell::new(false),
            },
        })
    }

    pub fn mount<V: IntoView>(&mut self, view: impl FnOnce() -> V) -> Result<(), Error> {
        self.app.mount(view)
    }

    pub fn document(&self) -> &Document {
        self.app.document()
    }

    pub fn render(&self, options: RenderOptions) -> Result<Bitmap, Error> {
        self.app.document.begin_frame(options.time_ms)?;
        self.app.document.render_to_bitmap()
    }

    pub fn render_at(&self, time_ms: f64) -> Result<Bitmap, Error> {
        self.render(RenderOptions { time_ms })
    }

    pub fn render_png_at(&self, time_ms: f64) -> Result<Vec<u8>, Error> {
        self.app.document.begin_frame(time_ms)?;
        self.app.document.render_to_png_buffer()
    }

    pub fn render_png_to(
        &self,
        time_ms: f64,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), Error> {
        self.app.document.begin_frame(time_ms)?;
        self.app.document.render_to_png(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_signal, view, For, Modifiers, MouseButton, MouseEventType, Show};

    #[test]
    fn invalid_builder_size_is_fallible() {
        assert!(App::builder()
            .size(LogicalSize::new(0.0, 10.0))
            .build()
            .is_err());
    }

    #[test]
    fn headless_app_mounts_and_renders_the_typed_view_path() {
        let mut app = HeadlessApp::new(Viewport::new(64, 64).unwrap()).unwrap();
        app.mount(|| {
            view! {
                <div style:width="32px" style:height="32px" style:background-color="red">
                    "native"
                </div>
            }
        })
        .unwrap();
        let first = app.render_at(0.0).unwrap();
        let second = app.render_at(0.0).unwrap();
        assert_eq!((first.width(), first.height()), (64, 64));
        assert_eq!(first.pixels(), second.pixels());
        assert!(first.pixels().iter().any(|channel| *channel != 255));
    }

    #[test]
    fn event_driven_signal_updates_retarget_the_native_text_node() {
        let count = create_signal(0_i32);
        let mut app = HeadlessApp::new(Viewport::new(100, 50).unwrap()).unwrap();
        app.mount(move || {
            view! {
                <button style:width="100px" style:height="50px"
                    on:click={move |_| count.update(|value| *value += 1)}>
                    {count.get()}
                </button>
            }
        })
        .unwrap();
        let before = app.render_at(0.0).unwrap();
        app.document()
            .dispatch_mouse_event(
                MouseEventType::Up,
                5.0,
                5.0,
                MouseButton::Left,
                Modifiers::NONE,
            )
            .unwrap();
        let after = app.render_at(0.0).unwrap();
        assert_eq!(count.get(), 1);
        assert_ne!(before.pixels(), after.pixels());
    }

    #[test]
    fn show_and_keyed_for_mutate_retained_nodes() {
        let visible = create_signal(true);
        let items = create_signal(vec![1_i32, 2, 3]);
        let mut app = HeadlessApp::new(Viewport::new(120, 80).unwrap()).unwrap();
        app.mount(move || {
            vec![
                Show(
                    move || visible.get(),
                    || view! { <div style:color="red">"hidden"</div> },
                    || view! { <div style:color="blue">"visible"</div> },
                ),
                For(
                    move || items.get(),
                    |item| *item,
                    |item| view! { <span>{item}</span> },
                ),
            ]
        })
        .unwrap();
        let before = app.render_at(0.0).unwrap();
        visible.set(false);
        items.set(vec![3, 1, 4]);
        let after = app.render_at(0.0).unwrap();
        assert_ne!(before.pixels(), after.pixels());
    }

    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn normalized_platform_events_drive_the_headless_reference_path() {
        use openui_platform::{
            KeyPhase, Modifiers as PlatformModifiers, PlatformApplication, PlatformEvent,
        };

        let mut app = App::builder()
            .size(LogicalSize::new(100.0, 50.0))
            .build()
            .unwrap();
        let input = crate::Element::create(app.document(), "input").unwrap();
        app.document().body().append_child(&input).unwrap();
        input.focus().unwrap();
        PlatformApplication::event(
            &mut app,
            PlatformEvent::Key {
                phase: KeyPhase::Down,
                key_code: 'a' as i32,
                text: Some("a".into()),
                modifiers: PlatformModifiers::default(),
                repeat: false,
            },
        )
        .unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("a"));
        PlatformApplication::event(
            &mut app,
            PlatformEvent::Resized {
                logical_width: 240,
                logical_height: 120,
                scale_factor: 2.0,
            },
        )
        .unwrap();
        let frame = PlatformApplication::render(&mut app, 0.0).unwrap();
        assert_eq!((frame.width, frame.height), (240, 120));
    }
}
