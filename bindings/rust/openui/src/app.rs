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
    ///
    /// The Linux platform feature supplies the event loop in W7. Headless-only
    /// builds return an explicit error after validating and mounting the view.
    pub fn run<V: IntoView>(mut self, view: impl FnOnce() -> V) -> Result<(), Error> {
        self.mount(view)?;
        Err(Error::PlatformUnavailable)
    }

    pub fn request_exit(&self) {
        self.exit_requested.set(true);
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested.get()
    }
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
}
