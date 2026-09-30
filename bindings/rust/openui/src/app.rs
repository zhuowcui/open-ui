//! Fallible application and deterministic headless lifecycle APIs.

use crate::context::with_document;
use crate::scope::{create_scope, dispose_scope};
use crate::style::{Bitmap, Error};
use crate::view_node::{mount_view, IntoView};
use crate::{Document, ScopeId};
use openui_engine::ViewportMetrics;
use std::cell::Cell;
#[cfg(all(feature = "linux", target_os = "linux"))]
use std::cell::RefCell;
use std::rc::Rc;

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

/// Cloneable shutdown request for the application's owning UI thread.
#[derive(Clone, Default)]
pub struct AppExitHandle(Rc<Cell<bool>>);

impl AppExitHandle {
    pub fn request_exit(&self) {
        self.0.set(true);
    }
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
        let viewport = ViewportMetrics::from_logical_size(
            self.options.size.width,
            self.options.size.height,
            1.0,
        )?;
        Ok(App {
            document: Document::with_viewport_metrics(viewport)?,
            options: self.options,
            root_scope: None,
            exit_requested: AppExitHandle::default(),
            #[cfg(all(feature = "linux", target_os = "linux"))]
            software_compositor: RefCell::new(openui_compositor::SoftwareCompositor::default()),
            #[cfg(all(feature = "linux", target_os = "linux"))]
            platform_event_handler: None,
            #[cfg(all(feature = "linux", target_os = "linux"))]
            rendered_generations: Cell::new(None),
        })
    }
}

pub struct App {
    document: Document,
    options: WindowOptions,
    root_scope: Option<ScopeId>,
    exit_requested: AppExitHandle,
    #[cfg(all(feature = "linux", target_os = "linux"))]
    software_compositor: RefCell<openui_compositor::SoftwareCompositor>,
    #[cfg(all(feature = "linux", target_os = "linux"))]
    platform_event_handler: Option<Box<dyn FnMut(&openui_platform::PlatformEvent)>>,
    #[cfg(all(feature = "linux", target_os = "linux"))]
    rendered_generations: Cell<Option<[u64; 5]>>,
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

    /// Present an existing retained document, preserving its nodes and listeners.
    pub fn from_document(document: Document, options: WindowOptions) -> Result<Self, Error> {
        ViewportMetrics::from_logical_size(options.size.width, options.size.height, 1.0)?;
        Ok(Self {
            document,
            options,
            root_scope: None,
            exit_requested: AppExitHandle::default(),
            #[cfg(all(feature = "linux", target_os = "linux"))]
            software_compositor: RefCell::new(openui_compositor::SoftwareCompositor::default()),
            #[cfg(all(feature = "linux", target_os = "linux"))]
            platform_event_handler: None,
            #[cfg(all(feature = "linux", target_os = "linux"))]
            rendered_generations: Cell::new(None),
        })
    }

    pub fn exit_handle(&self) -> AppExitHandle {
        self.exit_requested.clone()
    }

    /// Observe platform events after document processing and all engine borrows.
    /// The callback runs on the UI thread, may use retained handles, and ends
    /// when the blocking run returns. It must not start another native run.
    #[cfg(all(feature = "linux", target_os = "linux"))]
    pub fn on_platform_event(
        &mut self,
        handler: impl FnMut(&openui_platform::PlatformEvent) + 'static,
    ) {
        self.platform_event_handler = Some(Box::new(handler));
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
        self.run_document()
    }

    /// Block in the native event loop using the document already attached.
    pub fn run_document(self) -> Result<(), Error> {
        self.document.update_all()?;
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
        self.exit_requested.request_exit();
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested.0.get()
    }
}

#[cfg(all(feature = "linux", target_os = "linux"))]
impl openui_platform::PlatformApplication for App {
    fn event(&mut self, event: openui_platform::PlatformEvent) -> Result<(), String> {
        use openui_engine::PointerEventKind;
        use openui_platform::{PlatformEvent, PointerButton, PointerPhase};

        let button = |value: PointerButton| match value {
            PointerButton::Left => crate::MouseButton::Left,
            PointerButton::Middle => crate::MouseButton::Middle,
            PointerButton::Right | PointerButton::Other(_) => crate::MouseButton::Right,
        };
        let result = match &event {
            PlatformEvent::BackendChanged(_) => Ok(()),
            PlatformEvent::Resized(viewport) => self.document.set_viewport(*viewport),
            PlatformEvent::Pointer {
                pointer_id,
                phase,
                x,
                y,
                button: pointer,
                modifiers: keys,
            } => self.document.dispatch_pointer_event(
                *pointer_id,
                match *phase {
                    PointerPhase::Move => PointerEventKind::Move,
                    PointerPhase::Down => PointerEventKind::Down,
                    PointerPhase::Up => PointerEventKind::Up,
                    PointerPhase::Cancel | PointerPhase::Leave => PointerEventKind::Cancel,
                },
                *x,
                *y,
                button(*pointer),
                platform_modifiers(*keys),
                *pointer_id != 0,
            ),
            PlatformEvent::Wheel {
                x,
                y,
                delta_x,
                delta_y,
                modifiers: keys,
            } => self.document.dispatch_wheel_event(
                *x,
                *y,
                *delta_x,
                *delta_y,
                platform_modifiers(*keys),
            ),
            PlatformEvent::Key {
                phase,
                key_code,
                text,
                modifiers: keys,
                repeat,
            } => {
                // Compatibility injection: navigation keys have no committed
                // text. The native adapter supplies both fields explicitly.
                return self.key_input(openui_platform::KeyboardInput {
                    phase: *phase,
                    key_code: *key_code,
                    key_text: text.clone(),
                    text: text
                        .clone()
                        .filter(|_| !matches!(*key_code, 8 | 9 | 13 | 27 | 33..=40 | 46)),
                    modifiers: *keys,
                    repeat: *repeat,
                });
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
            | PlatformEvent::HoveredFileCancelled
            | PlatformEvent::Presented { .. }
            | PlatformEvent::CloseRequested => Ok(()),
        };
        result.map_err(|error| error.to_string())?;
        if let Some(handler) = self.platform_event_handler.as_mut() {
            handler(&event);
        }
        Ok(())
    }

    fn key_input(&mut self, input: openui_platform::KeyboardInput) -> Result<(), String> {
        use openui_platform::KeyPhase;
        self.document
            .dispatch_key_input(
                match input.phase {
                    KeyPhase::Down => crate::KeyEventType::Down,
                    KeyPhase::Up => crate::KeyEventType::Up,
                },
                input.key_code,
                input.key_text.as_deref(),
                input.text.as_deref(),
                platform_modifiers(input.modifiers),
            )
            .map_err(|error| error.to_string())?;
        if let Some(handler) = self.platform_event_handler.as_mut() {
            handler(&input.key_event());
        }
        Ok(())
    }

    fn render(&mut self, time_ms: f64) -> Result<openui_platform::SoftwareFrame, String> {
        self.document
            .begin_frame(time_ms)
            .map_err(|error| error.to_string())?;
        let scene = self
            .document
            .with_engine_mut(|engine| engine.scene())
            .map_err(|error| error.to_string())?;
        let frame = self
            .software_compositor
            .borrow_mut()
            .render(&scene)
            .map_err(|error| error.to_string())?;
        self.rendered_generations.set(Some(
            self.document
                .with_engine(visual_generations)
                .map_err(|error| error.to_string())?,
        ));
        Ok(openui_platform::SoftwareFrame {
            width: frame.width,
            height: frame.height,
            viewport: frame.viewport,
            stride: frame.stride,
            pixels: frame.pixels,
        })
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

    fn is_animating(&self) -> bool {
        self.document.is_animating().unwrap_or(false)
    }

    fn needs_redraw(&self) -> bool {
        self.document
            .with_engine(visual_generations)
            .is_ok_and(|generations| self.rendered_generations.get() != Some(generations))
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
fn platform_modifiers(value: openui_platform::Modifiers) -> crate::Modifiers {
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
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn visual_generations(engine: &openui_engine::Engine) -> [u64; 5] {
    let generations = engine.dirty_generations();
    [
        generations.tree,
        generations.intrinsic,
        generations.layout,
        generations.paint,
        generations.compositing,
    ]
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
    pub fn new(viewport: ViewportMetrics) -> Result<Self, Error> {
        Ok(Self {
            app: App {
                document: Document::with_viewport_metrics(viewport)?,
                options: WindowOptions {
                    size: LogicalSize::new(viewport.logical_width(), viewport.logical_height()),
                    ..WindowOptions::default()
                },
                root_scope: None,
                exit_requested: AppExitHandle::default(),
                #[cfg(all(feature = "linux", target_os = "linux"))]
                software_compositor: RefCell::new(openui_compositor::SoftwareCompositor::default()),
                #[cfg(all(feature = "linux", target_os = "linux"))]
                platform_event_handler: None,
                #[cfg(all(feature = "linux", target_os = "linux"))]
                rendered_generations: Cell::new(None),
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
        let mut app =
            HeadlessApp::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
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
    fn app_from_document_retains_existing_nodes_and_exit_handle() {
        let document = Document::new(80, 40).unwrap();
        let element = crate::Element::create(&document, "button").unwrap();
        document.body().append_child(&element).unwrap();
        element.set_attribute("id", "retained").unwrap();
        let app = App::from_document(document, WindowOptions::default()).unwrap();
        assert!(app.document().element_by_id("retained").unwrap().is_some());
        let exit = app.exit_handle();
        assert!(!app.exit_requested());
        exit.request_exit();
        assert!(app.exit_requested());
    }

    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn platform_observer_can_mutate_and_request_exit_after_render_borrows() {
        use openui_platform::{PlatformApplication, PlatformEvent};
        let document = Document::new(80, 40).unwrap();
        let element = crate::Element::create(&document, "div").unwrap();
        document.body().append_child(&element).unwrap();
        let mut app = App::from_document(document, WindowOptions::default()).unwrap();
        let exit = app.exit_handle();
        app.on_platform_event(move |event| {
            if matches!(event, PlatformEvent::Presented { .. }) {
                element.set_text("next frame").unwrap();
                exit.request_exit();
            }
        });
        assert!(app.needs_redraw());
        let first = app.render(0.0).unwrap();
        assert!(!app.needs_redraw());
        app.event(PlatformEvent::Presented {
            frame_number: 1,
            time_ms: 0.0,
        })
        .unwrap();
        assert!(app.exit_requested());
        assert!(app.needs_redraw());
        let second = app.render(0.0).unwrap();
        assert_ne!(first.pixels, second.pixels);
        assert!(!app.needs_redraw());
        let unchanged = app.render(0.0).unwrap();
        assert_eq!(second.pixels, unchanged.pixels);
        assert!(!app.needs_redraw());
    }

    #[test]
    fn event_driven_signal_updates_retarget_the_native_text_node() {
        let count = create_signal(0_i32);
        let mut app =
            HeadlessApp::new(ViewportMetrics::from_logical_size(100.0, 50.0, 1.0).unwrap())
                .unwrap();
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
        let mut app =
            HeadlessApp::new(ViewportMetrics::from_logical_size(120.0, 80.0, 1.0).unwrap())
                .unwrap();
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
            PlatformEvent::Resized(ViewportMetrics::from_physical_size(480, 240, 2.0).unwrap()),
        )
        .unwrap();
        let frame = PlatformApplication::render(&mut app, 0.0).unwrap();
        assert_eq!((frame.width, frame.height), (480, 240));
    }

    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn cancelled_native_keydown_does_not_insert_committed_text() {
        use openui_platform::{KeyPhase, KeyboardInput, PlatformApplication};
        let mut app = App::builder().build().unwrap();
        let input = crate::Element::create(app.document(), "input").unwrap();
        app.document().body().append_child(&input).unwrap();
        input.focus().unwrap();
        let callback_input = input.clone();
        input
            .on("keydown", move |event| {
                callback_input.set_control_value("callback").unwrap();
                event.prevent_default();
            })
            .unwrap();
        app.key_input(KeyboardInput {
            phase: KeyPhase::Down,
            key_code: 65,
            key_text: Some("a".into()),
            text: Some("a".into()),
            modifiers: openui_platform::Modifiers::default(),
            repeat: false,
        })
        .unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("callback"));
    }

    #[cfg(all(feature = "linux", target_os = "linux"))]
    #[test]
    fn native_named_keys_and_noneditable_controls_do_not_receive_text() {
        use openui_platform::{KeyPhase, KeyboardInput, PlatformApplication};
        let mut app = App::builder().build().unwrap();
        let input = crate::Element::create(app.document(), "input").unwrap();
        app.document().body().append_child(&input).unwrap();
        input.focus().unwrap();
        for (key_code, key_text, text) in [
            (27, "Escape", None),
            (38, "ArrowUp", None),
            (0, ".", Some(".")),
            (0, "#", Some("#")),
            (0, "é", Some("é")),
        ] {
            app.key_input(KeyboardInput {
                phase: KeyPhase::Down,
                key_code,
                key_text: Some(key_text.into()),
                text: text.map(str::to_owned),
                modifiers: openui_platform::Modifiers::default(),
                repeat: false,
            })
            .unwrap();
        }
        assert_eq!(input.control_value().unwrap().as_deref(), Some(".#é"));
        let checkbox = crate::Element::create(app.document(), "input").unwrap();
        checkbox.set_attribute("type", "checkbox").unwrap();
        app.document().body().append_child(&checkbox).unwrap();
        checkbox.focus().unwrap();
        app.key_input(KeyboardInput {
            phase: KeyPhase::Down,
            key_code: 32,
            key_text: Some(" ".into()),
            text: Some(" ".into()),
            modifiers: openui_platform::Modifiers::default(),
            repeat: false,
        })
        .unwrap();
        assert!(app
            .document()
            .with_engine(|engine| {
                engine
                    .control_state(checkbox.handle)
                    .unwrap()
                    .unwrap()
                    .checked
            })
            .unwrap());
        app.event(openui_platform::PlatformEvent::TextInput("ignored".into()))
            .unwrap();
        let button = crate::Element::create(app.document(), "button").unwrap();
        app.document().body().append_child(&button).unwrap();
        button.focus().unwrap();
        app.event(openui_platform::PlatformEvent::TextInput("ignored".into()))
            .unwrap();
    }
}
