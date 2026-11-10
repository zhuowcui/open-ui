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
    #[cfg(feature = "menu")]
    menu: Option<std::rc::Rc<dyn Fn() -> crate::menu::Menu>>,
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
            #[cfg(feature = "menu")]
            menu: None,
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
    /// Give every window the menu bar `menu` builds. It is built once per
    /// window and rebuilt when the signals it reads change; items' enabled
    /// and checked closures are read reactively too.
    #[cfg(feature = "menu")]
    pub fn menu(mut self, menu: impl Fn() -> crate::menu::Menu + 'static) -> Self {
        self.menu = Some(std::rc::Rc::new(menu));
        self
    }

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
            #[cfg(feature = "menu")]
            menu: self.menu,
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
    #[cfg(feature = "menu")]
    pub(crate) menu: Option<std::rc::Rc<dyn Fn() -> crate::menu::Menu>>,
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
        #[cfg(feature = "menu")]
        desktop::set_app_name(defaults.title.clone());
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
    #[cfg(all(feature = "dialog", any(target_os = "linux", target_os = "windows")))]
    crate::dialog::reset();
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
    #[cfg(feature = "menu")]
    use std::rc::Rc;

    pub(crate) struct Controller {
        pub(crate) defaults: Defaults,
        pub(crate) exit_on_last_window_closed: bool,
        pub(crate) accent: Option<openui_style::Color>,
    }

    impl Controller {
        /// Wrap `view` so the window's root scope keeps its menu resolved,
        /// and return the shared resolved menu.
        #[cfg(feature = "menu")]
        fn with_menu(&self, id: u64, view: window::ViewFn) -> (window::ViewFn, Option<SharedMenu>) {
            let Some(build) = self.defaults.menu.clone() else {
                return (view, None);
            };
            let shared: SharedMenu = Rc::default();
            let name = self.defaults.title.clone();
            let resolved = shared.clone();
            let view: window::ViewFn = Box::new(move || {
                crate::create_effect(move || {
                    let menu = crate::menu::Resolved::of(&build(), &name);
                    if openui_platform::NATIVE_MENU_BAR {
                        window::request_menu(id, menu.entries.clone());
                    }
                    *resolved.borrow_mut() = menu;
                });
                view()
            });
            (view, Some(shared))
        }

        fn open(
            &self,
            id: u64,
            spec: Option<&WindowSpec>,
            app: WindowApp,
        ) -> Result<WindowCommand, String> {
            let accent = self.accent;
            let options = app.app.options().clone();
            app.app
                .document()
                .with_engine_mut(|engine| {
                    engine.set_platform_accent_color(accent);
                    Ok(())
                })
                .map_err(|error| error.to_string())?;
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
                application: Box::new(app),
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
                        #[cfg(feature = "menu")]
                        let (view, menu) = self.with_menu(id, view);
                        let app = self
                            .defaults
                            .build(&key, &spec, view)
                            .map_err(|error| error.to_string())?;
                        #[cfg(feature = "menu")]
                        let app = WindowApp { key, app, menu };
                        #[cfg(not(feature = "menu"))]
                        let app = WindowApp { key, app };
                        self.open(id, Some(&spec), app)?
                    }
                    Request::Adopt { id, app } => {
                        let Some(key) = window::key_of(id) else {
                            continue;
                        };
                        let app = WindowApp {
                            key,
                            app: *app,
                            #[cfg(feature = "menu")]
                            menu: None,
                        };
                        self.open(id, None, app)?
                    }
                    Request::Close(id) => WindowCommand::Close(WindowId(id)),
                    Request::Focus(id) => WindowCommand::Focus(WindowId(id)),
                    Request::SetTitle(id, title) => WindowCommand::SetTitle(WindowId(id), title),
                    #[cfg(feature = "menu")]
                    Request::SetMenu(id, entries) => WindowCommand::SetMenu(WindowId(id), entries),
                });
            }
            Ok(commands)
        }

        fn close_requested(&mut self, id: WindowId) -> Result<bool, String> {
            Ok(window::close_requested(id.0))
        }

        #[cfg(all(feature = "dialog", any(target_os = "linux", target_os = "windows")))]
        fn window_opened(&mut self, id: WindowId, native: openui_platform::NativeWindow) {
            crate::dialog::window_opened(id.0, native);
        }

        fn window_closed(&mut self, id: WindowId) -> Result<(), String> {
            #[cfg(all(feature = "dialog", any(target_os = "linux", target_os = "windows")))]
            crate::dialog::window_closed(id.0);
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
    #[cfg(feature = "menu")]
    type SharedMenu = Rc<std::cell::RefCell<crate::menu::Resolved>>;

    struct WindowApp {
        key: WindowKey,
        app: App,
        #[cfg(feature = "menu")]
        menu: Option<SharedMenu>,
    }

    impl WindowApp {
        fn with<R>(&mut self, operation: impl FnOnce(&mut App) -> R) -> R {
            let app = &mut self.app;
            window::with_current(&self.key, || operation(app))
        }

        /// Run a menu item's action as if this window's event caused it.
        #[cfg(feature = "menu")]
        fn run_menu_action(&mut self, action: crate::menu::Action) -> Result<(), String> {
            use crate::menu::{Action, Role};
            use crate::EditCommand;
            let key = self.key.clone();
            self.with(|app| {
                let document = app.document();
                let result = match action {
                    Action::Nothing => Ok(()),
                    Action::Run(run) => {
                        run();
                        Ok(())
                    }
                    Action::Role(Role::Quit) => {
                        crate::app::exit();
                        Ok(())
                    }
                    Action::Role(Role::CloseWindow) => {
                        window::request_close(&key);
                        Ok(())
                    }
                    Action::Role(Role::About) => {
                        about_window();
                        Ok(())
                    }
                    Action::Role(Role::Undo) => document.edit_focused(EditCommand::Undo),
                    Action::Role(Role::Redo) => document.edit_focused(EditCommand::Redo),
                    Action::Role(Role::SelectAll) => document.edit_focused(EditCommand::SelectAll),
                    Action::Role(Role::Cut) => document.copy_selection(true),
                    Action::Role(Role::Copy) => document.copy_selection(false),
                    Action::Role(Role::Paste) => document.paste_clipboard(),
                };
                result.map_err(|error| error.to_string())
            })
        }
    }

    /// The window [`Role::About`](crate::menu::Role::About) opens: the app's
    /// name, once.
    #[cfg(feature = "menu")]
    fn about_window() {
        let name = ABOUT_NAME
            .with(|name| name.borrow().clone())
            .unwrap_or_default();
        window::open(
            WindowSpec::new("openui.about")
                .title(format!("About {name}"))
                .size(320.0, 120.0)
                .resizable(false),
            move || name,
        );
    }

    #[cfg(feature = "menu")]
    pub(super) fn set_app_name(name: String) {
        ABOUT_NAME.with(|about| *about.borrow_mut() = Some(name));
    }

    #[cfg(feature = "menu")]
    thread_local! {
        /// The running app's name, for its About window.
        static ABOUT_NAME: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
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

        #[cfg(feature = "menu")]
        fn menu_selected(&mut self, id: u32) -> Result<(), String> {
            let action = self.menu.as_ref().and_then(|menu| menu.borrow().action(id));
            match action {
                Some(action) => self.run_menu_action(action),
                None => Ok(()),
            }
        }

        fn key_input(&mut self, input: KeyboardInput) -> Result<(), String> {
            #[cfg(feature = "menu")]
            if input.phase == openui_platform::KeyPhase::Down {
                let action = self.menu.as_ref().and_then(|menu| {
                    menu.borrow().accelerator_action(
                        input.modifiers,
                        input.key_code,
                        input.key_text.as_deref(),
                    )
                });
                if let Some(action) = action {
                    return self.run_menu_action(action);
                }
            }
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
