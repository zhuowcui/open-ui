//! Declaring an app with several windows, and the controller that opens them.

use crate::app::{App, BackendPreference, RENDER_PNG_ENV};
use crate::style::Error;
use crate::view_node::IntoView;
use crate::window::{self, ViewFn, WindowKey, WindowSpec};
use openui_engine::EngineOptions;

/// An app declaration, made by [`App::new`]: its id, name and first windows.
///
/// ```no_run
/// use openui::prelude::*;
/// use openui::window::WindowSpec;
///
/// fn main() -> Result<(), openui::Error> {
///     App::new("dev.example.notes")
///         .name("Notes")
///         .window(WindowSpec::new("main").size(960.0, 640.0), || "Notes")
///         .run()
/// }
/// ```
pub struct AppSpec {
    id: String,
    name: Option<String>,
    windows: Vec<(WindowSpec, ViewFn)>,
    exit_on_last_window_closed: bool,
    backend: BackendPreference,
    engine_options: EngineOptions,
}

impl App {
    /// Declare an app. `id` is a reverse-DNS name such as
    /// `dev.example.notes`; the runtime will use it for per-app state.
    #[allow(clippy::new_ret_no_self)] // `App::new(id)...run()` reads as one declaration.
    pub fn new(id: impl Into<String>) -> AppSpec {
        AppSpec {
            id: id.into(),
            name: None,
            windows: Vec::new(),
            exit_on_last_window_closed: true,
            backend: BackendPreference::Auto,
            engine_options: EngineOptions::default(),
        }
    }
}

impl AppSpec {
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The app's display name, and the title of windows that set none.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// A window to open when the app starts.
    pub fn window<V: IntoView>(
        mut self,
        spec: WindowSpec,
        view: impl FnOnce() -> V + 'static,
    ) -> Self {
        self.windows
            .push((spec, Box::new(move || view().into_view())));
        self
    }

    /// Whether closing the last window exits the app (the default).
    pub fn exit_on_last_window_closed(mut self, exit: bool) -> Self {
        self.exit_on_last_window_closed = exit;
        self
    }

    pub fn backend(mut self, backend: BackendPreference) -> Self {
        self.backend = backend;
        self
    }

    /// Engine options for every window's document.
    pub fn engine_options(mut self, options: EngineOptions) -> Self {
        self.engine_options = options;
        self
    }

    /// Open the declared windows and block until the app exits.
    ///
    /// When `OPENUI_RENDER_PNG` names a file, render the first declared
    /// window's first frame there instead, as [`App::run`] does.
    pub fn run(self) -> Result<(), Error> {
        let defaults = Defaults {
            title: self.name.clone().unwrap_or_else(|| self.id.clone()),
            backend: self.backend,
            engine_options: self.engine_options,
        };
        if let Some(path) = std::env::var_os(RENDER_PNG_ENV) {
            let (spec, view) = self
                .windows
                .into_iter()
                .next()
                .ok_or(Error::InvalidArgument(
                    "the app declares no window to render",
                ))?;
            let key = spec.key.clone();
            let app = defaults.build(&key, &spec, view)?;
            app.document().update_all()?;
            return app.render_first_frame_to(path);
        }
        for (spec, view) in self.windows {
            window::open(spec, view);
        }
        run_windows(defaults, self.exit_on_last_window_closed)
    }
}

/// What a window takes from its app when its spec does not say.
pub(crate) struct Defaults {
    pub(crate) title: String,
    pub(crate) backend: BackendPreference,
    pub(crate) engine_options: EngineOptions,
}

impl Defaults {
    /// A window's document with its view mounted.
    fn build(&self, key: &WindowKey, spec: &WindowSpec, view: ViewFn) -> Result<App, Error> {
        let mut app = App::builder()
            .title(spec.title.clone().unwrap_or_else(|| self.title.clone()))
            .size(spec.size)
            .backend(self.backend)
            .engine_options(self.engine_options)
            .build()?;
        window::with_current(key, || app.mount(view))?;
        Ok(app)
    }
}

/// Run the event loop over the window registry, then clear the registry.
pub(crate) fn run_windows(
    defaults: Defaults,
    exit_on_last_window_closed: bool,
) -> Result<(), Error> {
    #[cfg(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    ))]
    let result = {
        // Follow the operating system's form-control accent, as Chromium's
        // native theme does.
        let accent = openui_platform::system_accent_color()
            .map(|[r, g, b]| openui_style::Color::from_rgba8(r, g, b, 255));
        openui_platform::run_windows(desktop::Controller {
            defaults,
            exit_on_last_window_closed,
            accent,
        })
        .map_err(|error| Error::Platform(error.to_string()))
    };
    #[cfg(not(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    )))]
    let result = {
        let _ = (defaults, exit_on_last_window_closed);
        Err(Error::PlatformUnavailable)
    };
    #[cfg(feature = "async")]
    crate::task::set_loop_waker(None);
    #[cfg(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    ))]
    crate::clipboard::set_native(None);
    window::reset();
    result
}

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
mod desktop {
    use super::Defaults;
    use crate::app::App;
    use crate::window::{self, Request, WindowKey, WindowSpec};
    use openui_platform::{
        ActionRequest, CursorIcon, KeyboardInput, PlatformApplication, PlatformEvent,
        SoftwareFrame, TreeUpdate, WindowCommand, WindowController, WindowId,
    };

    pub(crate) struct Controller {
        pub(crate) defaults: Defaults,
        pub(crate) exit_on_last_window_closed: bool,
        pub(crate) accent: Option<openui_style::Color>,
    }

    impl Controller {
        fn open(
            &self,
            id: u64,
            key: WindowKey,
            spec: Option<&WindowSpec>,
            app: App,
        ) -> Result<WindowCommand, String> {
            let accent = self.accent;
            app.document()
                .with_engine_mut(|engine| {
                    engine.set_platform_accent_color(accent);
                    Ok(())
                })
                .map_err(|error| error.to_string())?;
            let options = app.options();
            let mut native = openui_platform::WindowOptions {
                title: options.title.clone(),
                width: options.size.width,
                height: options.size.height,
                backend: crate::app::platform_backend(options.backend),
                ..openui_platform::WindowOptions::default()
            };
            if let Some(spec) = spec {
                native.min_size = spec.min_size.map(|size| (size.width, size.height));
                native.max_size = spec.max_size.map(|size| (size.width, size.height));
                native.resizable = spec.resizable;
                native.decorations = spec.decorations;
            }
            Ok(WindowCommand::Open {
                id: WindowId(id),
                options: native,
                application: Box::new(WindowApp { key, app }),
            })
        }
    }

    impl WindowController for Controller {
        fn clipboard(&mut self, clipboard: openui_platform::Clipboard) {
            crate::clipboard::set_native(Some(clipboard));
        }

        #[cfg(feature = "async")]
        fn started(&mut self, waker: openui_platform::LoopWaker) {
            crate::task::set_loop_waker(Some(std::sync::Arc::new(move || waker.wake())));
        }

        #[cfg(feature = "async")]
        fn run_tasks(&mut self) -> Result<bool, String> {
            Ok(crate::task::run_turn())
        }

        fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String> {
            let mut commands = Vec::new();
            for request in window::take_requests() {
                commands.push(match request {
                    Request::Open { id, spec, view } => {
                        // Closed again before the loop got to it.
                        let Some(key) = window::key_of(id) else {
                            continue;
                        };
                        let app = self
                            .defaults
                            .build(&key, &spec, view)
                            .map_err(|error| error.to_string())?;
                        self.open(id, key, Some(&spec), app)?
                    }
                    Request::Adopt { id, app } => {
                        let Some(key) = window::key_of(id) else {
                            continue;
                        };
                        self.open(id, key, None, *app)?
                    }
                    Request::Close(id) => WindowCommand::Close(WindowId(id)),
                    Request::Focus(id) => WindowCommand::Focus(WindowId(id)),
                    Request::SetTitle(id, title) => WindowCommand::SetTitle(WindowId(id), title),
                });
            }
            Ok(commands)
        }

        fn close_requested(&mut self, id: WindowId) -> Result<bool, String> {
            Ok(window::close_requested(id.0))
        }

        fn window_closed(&mut self, id: WindowId) -> Result<(), String> {
            window::forget(id.0);
            Ok(())
        }

        fn exit_requested(&self, open_windows: usize) -> bool {
            window::exit_requested()
                || (self.exit_on_last_window_closed && open_windows == 0 && !window::has_windows())
        }
    }

    /// A window's [`App`], with [`window::current`] naming it while its
    /// callbacks run.
    struct WindowApp {
        key: WindowKey,
        app: App,
    }

    impl WindowApp {
        fn with<R>(&mut self, operation: impl FnOnce(&mut App) -> R) -> R {
            let app = &mut self.app;
            window::with_current(&self.key, || operation(app))
        }
    }

    impl PlatformApplication for WindowApp {
        fn event(&mut self, event: PlatformEvent) -> Result<(), String> {
            self.with(|app| app.event(event))
        }

        fn render(&mut self, time_ms: f64) -> Result<SoftwareFrame, String> {
            self.with(|app| app.render(time_ms))
        }

        fn exit_requested(&self) -> bool {
            PlatformApplication::exit_requested(&self.app)
        }

        fn key_input(&mut self, input: KeyboardInput) -> Result<(), String> {
            self.with(|app| app.key_input(input))
        }

        fn dispatch_pending_events(&mut self) -> Result<bool, String> {
            self.with(|app| app.dispatch_pending_events())
        }

        fn clipboard_text(&self) -> Result<String, String> {
            self.app.clipboard_text()
        }

        fn set_clipboard_text(&mut self, text: String) -> Result<(), String> {
            self.with(|app| app.set_clipboard_text(text))
        }

        fn is_animating(&self) -> bool {
            self.app.is_animating()
        }

        fn needs_redraw(&self) -> bool {
            self.app.needs_redraw()
        }

        fn cursor_icon(&mut self, x: f32, y: f32) -> Result<CursorIcon, String> {
            self.with(|app| app.cursor_icon(x, y))
        }

        fn accessibility_update(&mut self) -> Result<TreeUpdate, String> {
            self.with(|app| app.accessibility_update())
        }

        fn accessibility_action(&mut self, request: ActionRequest) -> Result<(), String> {
            self.with(|app| app.accessibility_action(request))
        }
    }
}
