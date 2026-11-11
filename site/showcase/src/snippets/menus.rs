use openui::menu::{Accelerator, Key, Menu, MenuItem, Modifiers, Role};
use openui::prelude::*;

fn main() -> Result<(), Error> {
    let large = create_signal(false);
    App::new("com.example.notes")
        .name("Notes")
        // Built for every window, and rebuilt when the signals it reads change.
        .menu(move || app_menu(large))
        .window(WindowSpec::new("main").title("Notes"), move || notes(large))
        .run()
}

fn app_menu(large: Signal<bool>) -> Menu {
    Menu::new()
        .submenu("&File", |menu| {
            menu.item(
                MenuItem::new("&New Note")
                    .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::N))
                    .on_select(|| println!("new note")),
            )
            .separator()
            .role(Role::CloseWindow)
            .role(Role::Quit)
        })
        .submenu("&Edit", |menu| {
            menu.role(Role::Undo)
                .role(Role::Redo)
                .separator()
                .role(Role::Cut)
                .role(Role::Copy)
                .role(Role::Paste)
        })
        .submenu("&View", |menu| {
            menu.item(
                MenuItem::new("&Large Text")
                    .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::L))
                    .checked(move || large.get())
                    .on_select(move || large.update(|on| *on = !*on)),
            )
            .item(
                MenuItem::new("&Reset Text Size")
                    .enabled(move || large.get())
                    .on_select(move || large.set(false)),
            )
        })
}

fn notes(large: Signal<bool>) -> ViewNode {
    let size = move || LengthValue::px(if large.get() { 22.0 } else { 15.0 });
    view! { <p style:padding="16px" style:font-size={size()}>"Notes go here."</p> }
}
