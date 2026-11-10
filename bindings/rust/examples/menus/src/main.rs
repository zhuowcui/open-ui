//! An app menu shared by every window.
//!
//! File opens and closes windows, Edit uses the standard roles on the text
//! field, and View has a check item and an item enabled only while it is
//! checked. Shortcuts work whether or not a menu is open. On Windows the
//! menu bar is native; on Linux the framework draws it in the window.
//!
//! Set `OUI_SMOKE_EXIT=1` to open, choose "New Window" through its shortcut
//! path, print the result and exit, as CI does.

use openui::prelude::*;

#[cfg(any(feature = "linux", feature = "windows"))]
mod app {
    use openui::menu::{Accelerator, Key, Menu, MenuItem, Modifiers, Role};
    use openui::prelude::*;
    use openui::window::{self, WindowKey, WindowSpec};

    thread_local! {
        static LARGE: Signal<bool> = create_signal(false);
        static OPENED: Signal<u32> = create_signal(0);
    }

    fn large() -> Signal<bool> {
        LARGE.with(|large| *large)
    }

    fn opened() -> Signal<u32> {
        OPENED.with(|opened| *opened)
    }

    pub fn new_window() {
        opened().update(|count| *count += 1);
        let number = opened().get();
        window::open(
            WindowSpec::new(WindowKey::unique())
                .title(format!("Note {number}"))
                .size(420.0, 220.0),
            note_view,
        );
    }

    pub fn app_menu() -> Menu {
        Menu::new()
            .submenu("&File", |menu| {
                menu.item(
                    MenuItem::new("&New Window")
                        .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::N))
                        .on_select(new_window),
                )
                .role(Role::CloseWindow)
                .separator()
                .role(Role::Quit)
            })
            .submenu("&Edit", |menu| {
                menu.role(Role::Undo)
                    .role(Role::Redo)
                    .separator()
                    .role(Role::Cut)
                    .role(Role::Copy)
                    .role(Role::Paste)
                    .role(Role::SelectAll)
            })
            .submenu("&View", |menu| {
                menu.item(
                    MenuItem::new("&Large Text")
                        .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::L))
                        .checked(|| large().get())
                        .on_select(|| large().update(|large| *large = !*large)),
                )
                .item(
                    MenuItem::new("&Reset Text Size")
                        .enabled(|| large().get())
                        .on_select(|| large().set(false)),
                )
            })
            .submenu("&Help", |menu| menu.role(Role::About))
    }

    pub fn note_view() -> ViewNode {
        view! {
            <div style:font-family="sans-serif" style:padding="16px">
                <p style:font-size={LengthValue::px(if large().get() { 22.0 } else { 14.0 })}>
                    "Type, then use the Edit menu or its shortcuts."
                </p>
                <input value="Hello from Open UI" style:width="300px" />
                <p>{format!("Windows opened from the menu: {}", opened().get())}</p>
            </div>
        }
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    use openui::window::WindowSpec;
    let smoke = std::env::var_os("OUI_SMOKE_EXIT").is_some();
    App::new("dev.openui.examples.menus")
        .name("Menus")
        .menu(app::app_menu)
        .window(WindowSpec::new("main").size(420.0, 220.0), move || {
            if smoke {
                // Two windows open, then the app exits.
                app::new_window();
                println!("menus: opened a window from the menu action");
                openui::app::exit();
            }
            app::note_view()
        })
        .run()
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() {
    println!("menus: build with --features linux or --features windows");
}
