//! App menus: a typed menu bar shared by every window.
//!
//! Declare it with [`AppSpec::menu`](crate::AppSpec::menu). Items run their
//! action on the UI thread with [`window::current`](crate::window::current)
//! naming the window whose menu was used. An item's accelerator works
//! whether or not the menu is open, and is matched before the focused
//! document sees the key.
//!
//! On Windows the menu bar is native. Elsewhere it is not shown yet, but
//! accelerators and roles still work.
//!
//! ```no_run
//! use openui::menu::{Accelerator, Key, Menu, MenuItem, Modifiers, Role};
//!
//! fn app_menu() -> Menu {
//!     Menu::new()
//!         .submenu("&File", |menu| {
//!             menu.item(
//!                 MenuItem::new("&New Window")
//!                     .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::N))
//!                     .on_select(|| println!("new window")),
//!             )
//!             .separator()
//!             .role(Role::Quit)
//!         })
//!         .submenu("&Edit", |menu| {
//!             menu.role(Role::Undo)
//!                 .role(Role::Redo)
//!                 .separator()
//!                 .role(Role::Cut)
//!                 .role(Role::Copy)
//!                 .role(Role::Paste)
//!                 .role(Role::SelectAll)
//!         })
//! }
//! ```

use openui_platform::MenuEntry;
use std::fmt;
use std::rc::Rc;

/// A menu bar, or the contents of one menu.
#[derive(Clone, Default)]
#[must_use]
pub struct Menu {
    entries: Vec<Entry>,
}

#[derive(Clone)]
enum Entry {
    Submenu { label: String, menu: Menu },
    Item(MenuItem),
    Role(Role),
    Separator,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a submenu built by `build`. `&` before a letter marks its
    /// mnemonic, as in `"&File"`; `&&` is a literal ampersand.
    pub fn submenu(mut self, label: impl Into<String>, build: impl FnOnce(Menu) -> Menu) -> Self {
        self.entries.push(Entry::Submenu {
            label: label.into(),
            menu: build(Menu::new()),
        });
        self
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// Add a standard item with the platform's label, shortcut and action.
    pub fn role(mut self, role: Role) -> Self {
        self.entries.push(Entry::Role(role));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }
}

impl fmt::Debug for Menu {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Menu")
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// A menu item that runs an action.
#[derive(Clone)]
#[must_use]
pub struct MenuItem {
    label: String,
    accelerator: Option<Accelerator>,
    enabled: Flag,
    checked: Option<Flag>,
    on_select: Option<Rc<dyn Fn()>>,
}

impl MenuItem {
    /// An item labelled `label`. `&` marks the mnemonic letter.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            accelerator: None,
            enabled: Flag::Value(true),
            checked: None,
            on_select: None,
        }
    }

    pub fn accelerator(mut self, accelerator: Accelerator) -> Self {
        self.accelerator = Some(accelerator);
        self
    }

    /// Whether the item can be chosen: a value, or a closure read
    /// reactively so the item follows the signals it reads.
    pub fn enabled(mut self, enabled: impl IntoFlag) -> Self {
        self.enabled = enabled.into_flag();
        self
    }

    /// Make this a check item, checked as `checked` says: a value, or a
    /// closure read reactively.
    pub fn checked(mut self, checked: impl IntoFlag) -> Self {
        self.checked = Some(checked.into_flag());
        self
    }

    pub fn on_select(mut self, action: impl Fn() + 'static) -> Self {
        self.on_select = Some(Rc::new(action));
        self
    }
}

/// A boolean that is either fixed or read from a closure.
#[derive(Clone)]
pub enum Flag {
    Value(bool),
    Read(Rc<dyn Fn() -> bool>),
}

impl Flag {
    fn get(&self) -> bool {
        match self {
            Self::Value(value) => *value,
            Self::Read(read) => read(),
        }
    }
}

/// A value or closure that [`MenuItem::enabled`] and
/// [`MenuItem::checked`] accept.
pub trait IntoFlag {
    fn into_flag(self) -> Flag;
}

impl IntoFlag for bool {
    fn into_flag(self) -> Flag {
        Flag::Value(self)
    }
}

impl<F: Fn() -> bool + 'static> IntoFlag for F {
    fn into_flag(self) -> Flag {
        Flag::Read(Rc::new(self))
    }
}

/// A standard menu item. Each has the platform's usual label and shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    /// End the app.
    Quit,
    /// Close the window, asking its close handler first.
    CloseWindow,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    /// Open a small window naming the app.
    About,
}

impl Role {
    fn label(self, app_name: &str) -> String {
        match self {
            Self::Quit if cfg!(target_os = "windows") => "E&xit".into(),
            Self::Quit => "&Quit".into(),
            Self::CloseWindow => "&Close Window".into(),
            Self::Undo => "&Undo".into(),
            Self::Redo => "&Redo".into(),
            Self::Cut => "Cu&t".into(),
            Self::Copy => "&Copy".into(),
            Self::Paste => "&Paste".into(),
            Self::SelectAll => "Select &All".into(),
            Self::About => format!("&About {}", escape_mnemonics(app_name)),
        }
    }

    fn accelerator(self) -> Option<Accelerator> {
        let primary = |key| Some(Accelerator::new(Modifiers::PRIMARY, key));
        match self {
            // Windows apps exit with Alt+F4, which closes the window.
            Self::Quit if cfg!(target_os = "windows") => None,
            Self::Quit => primary(Key::Q),
            Self::CloseWindow => primary(Key::W),
            Self::Undo => primary(Key::Z),
            Self::Redo if cfg!(target_os = "windows") => primary(Key::Y),
            Self::Redo => Some(Accelerator::new(
                Modifiers::PRIMARY | Modifiers::SHIFT,
                Key::Z,
            )),
            Self::Cut => primary(Key::X),
            Self::Copy => primary(Key::C),
            Self::Paste => primary(Key::V),
            Self::SelectAll => primary(Key::A),
            Self::About => None,
        }
    }

    /// Whether the focused document already handles this role's shortcut,
    /// so the menu leaves the key to it.
    fn document_handles_shortcut(self) -> bool {
        matches!(
            self,
            Self::Undo | Self::Redo | Self::Cut | Self::Copy | Self::Paste | Self::SelectAll
        )
    }
}

fn escape_mnemonics(text: &str) -> String {
    text.replace('&', "&&")
}

/// Keyboard modifiers of an [`Accelerator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(1);
    pub const SHIFT: Self = Self(2);
    pub const ALT: Self = Self(4);
    /// The Windows or Super key.
    pub const META: Self = Self(8);
    /// The platform's command modifier: Ctrl on Windows and Linux.
    pub const PRIMARY: Self = Self::CTRL;

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    fn from_platform(modifiers: openui_platform::Modifiers) -> Self {
        let mut result = Self::NONE;
        for (held, modifier) in [
            (modifiers.control, Self::CTRL),
            (modifiers.shift, Self::SHIFT),
            (modifiers.alt, Self::ALT),
            (modifiers.meta, Self::META),
        ] {
            if held {
                result = result | modifier;
            }
        }
        result
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// A key of an [`Accelerator`]. Letters are `Key::A` to `Key::Z`, or
/// `Key::Char` for any character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    /// A character key; letters match either case.
    Char(char),
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    /// A function key, `Function(1)` to `Function(24)`.
    Function(u8),
}

macro_rules! letter_keys {
    ($($letter:ident)*) => {
        impl Key {
            $(
                #[doc = concat!("The ", stringify!($letter), " key.")]
                pub const $letter: Key = Key::Char(stringify!($letter).as_bytes()[0] as char);
            )*
        }
    };
}

letter_keys!(A B C D E F G H I J K L M N O P Q R S T U V W X Y Z);

impl Key {
    /// The platform key code this key arrives with, if it has one.
    fn code(self) -> Option<i32> {
        Some(match self {
            Self::Char(character) if character.is_ascii_alphanumeric() => {
                character.to_ascii_uppercase() as i32
            }
            Self::Char(_) => return None,
            Self::Enter => 13,
            Self::Escape => 27,
            Self::Tab => 9,
            Self::Space => 32,
            Self::Backspace => 8,
            Self::Delete => 46,
            Self::Home => 36,
            Self::End => 35,
            Self::PageUp => 33,
            Self::PageDown => 34,
            Self::ArrowLeft => 37,
            Self::ArrowUp => 38,
            Self::ArrowRight => 39,
            Self::ArrowDown => 40,
            Self::Function(number @ 1..=24) => 111 + i32::from(number),
            Self::Function(_) => return None,
        })
    }

    fn matches(self, key_code: i32, text: Option<&str>) -> bool {
        match (self.code(), self) {
            (Some(code), _) => code == key_code,
            (None, Self::Char(character)) => {
                text.is_some_and(|text| text.chars().eq(std::iter::once(character)))
            }
            (None, _) => false,
        }
    }

    fn name(self) -> String {
        match self {
            Self::Char(' ') | Self::Space => "Space".into(),
            Self::Char(character) => character.to_uppercase().collect(),
            Self::Enter => "Enter".into(),
            Self::Escape => "Esc".into(),
            Self::Tab => "Tab".into(),
            Self::Backspace => "Backspace".into(),
            Self::Delete => "Del".into(),
            Self::Home => "Home".into(),
            Self::End => "End".into(),
            Self::PageUp => "PgUp".into(),
            Self::PageDown => "PgDn".into(),
            Self::ArrowUp => "Up".into(),
            Self::ArrowDown => "Down".into(),
            Self::ArrowLeft => "Left".into(),
            Self::ArrowRight => "Right".into(),
            Self::Function(number) => format!("F{number}"),
        }
    }
}

/// A shortcut that chooses a menu item from the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Accelerator {
    modifiers: Modifiers,
    key: Key,
}

impl Accelerator {
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    /// How menus show it, such as `Ctrl+Shift+N`.
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        for (modifier, name) in [
            (Modifiers::CTRL, "Ctrl"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::META, "Super"),
        ] {
            if self.modifiers.contains(modifier) {
                parts.push(name.to_owned());
            }
        }
        parts.push(self.key.name());
        parts.join("+")
    }

    fn matches(&self, modifiers: Modifiers, key_code: i32, text: Option<&str>) -> bool {
        modifiers == self.modifiers && self.key.matches(key_code, text)
    }
}

/// What choosing a resolved item does.
#[derive(Clone)]
pub(crate) enum Action {
    Run(Rc<dyn Fn()>),
    Role(Role),
    Nothing,
}

/// A chosen item: its action, shortcut and state when the menu was last
/// resolved.
#[derive(Clone)]
pub(crate) struct ResolvedItem {
    pub(crate) action: Action,
    accelerator: Option<Accelerator>,
    enabled: bool,
    /// The focused document handles the shortcut itself.
    document_shortcut: bool,
}

/// A window's resolved menu: what to show and what each item id does.
#[derive(Clone, Default)]
pub(crate) struct Resolved {
    pub(crate) entries: Vec<MenuEntry>,
    pub(crate) items: Vec<ResolvedItem>,
}

impl Resolved {
    /// Read every flag, so a reactive effect that calls this re-runs when
    /// any of them changes.
    pub(crate) fn of(menu: &Menu, app_name: &str) -> Self {
        let mut resolved = Self::default();
        resolved.entries = resolved.entries_of(menu, app_name);
        resolved
    }

    fn entries_of(&mut self, menu: &Menu, app_name: &str) -> Vec<MenuEntry> {
        menu.entries
            .iter()
            .map(|entry| match entry {
                Entry::Submenu { label, menu } => MenuEntry::Submenu {
                    label: label.clone(),
                    enabled: true,
                    entries: self.entries_of(menu, app_name),
                },
                Entry::Separator => MenuEntry::Separator,
                Entry::Item(item) => {
                    let enabled = item.enabled.get();
                    let checked = item.checked.as_ref().map(Flag::get);
                    self.push(
                        item.label.clone(),
                        ResolvedItem {
                            action: item.on_select.clone().map_or(Action::Nothing, Action::Run),
                            accelerator: item.accelerator,
                            enabled,
                            document_shortcut: false,
                        },
                        checked,
                    )
                }
                Entry::Role(role) => self.push(
                    role.label(app_name),
                    ResolvedItem {
                        action: Action::Role(*role),
                        accelerator: role.accelerator(),
                        enabled: true,
                        document_shortcut: role.document_handles_shortcut(),
                    },
                    None,
                ),
            })
            .collect()
    }

    fn push(&mut self, label: String, item: ResolvedItem, checked: Option<bool>) -> MenuEntry {
        let entry = MenuEntry::Item {
            id: self.items.len() as u32,
            label,
            accelerator: item.accelerator.map(|accelerator| accelerator.text()),
            enabled: item.enabled,
            checked,
        };
        self.items.push(item);
        entry
    }

    /// The action of item `id`, if it exists and is enabled.
    pub(crate) fn action(&self, id: u32) -> Option<Action> {
        let item = self.items.get(id as usize)?;
        item.enabled.then(|| item.action.clone())
    }

    /// The action whose accelerator a key press matches, unless the focused
    /// document handles that shortcut itself.
    pub(crate) fn accelerator_action(
        &self,
        modifiers: openui_platform::Modifiers,
        key_code: i32,
        text: Option<&str>,
    ) -> Option<Action> {
        let modifiers = Modifiers::from_platform(modifiers);
        self.items
            .iter()
            .find(|item| {
                item.accelerator
                    .is_some_and(|accelerator| accelerator.matches(modifiers, key_code, text))
            })
            .filter(|item| item.enabled && !item.document_shortcut)
            .map(|item| item.action.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn platform(control: bool, shift: bool) -> openui_platform::Modifiers {
        openui_platform::Modifiers {
            control,
            shift,
            ..Default::default()
        }
    }

    #[test]
    fn accelerators_read_as_menus_show_them() {
        let accelerator = Accelerator::new(Modifiers::PRIMARY | Modifiers::SHIFT, Key::N);
        assert_eq!(accelerator.text(), "Ctrl+Shift+N");
        assert_eq!(
            Accelerator::new(Modifiers::NONE, Key::Function(5)).text(),
            "F5"
        );
        assert_eq!(
            Accelerator::new(Modifiers::ALT, Key::Char(',')).text(),
            "Alt+,"
        );
        assert_eq!(Key::F, Key::Char('F'));
    }

    #[test]
    fn items_resolve_to_entries_with_ids_and_state() {
        let flag = crate::create_signal(false);
        let menu = Menu::new().submenu("&File", |menu| {
            menu.item(
                MenuItem::new("&Save")
                    .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::S))
                    .enabled(move || flag.get()),
            )
            .separator()
            .item(MenuItem::new("&Wrap").checked(true))
            .role(Role::Quit)
        });
        let resolved = Resolved::of(&menu, "Notes");
        let MenuEntry::Submenu { label, entries, .. } = &resolved.entries[0] else {
            panic!("expected a submenu");
        };
        assert_eq!(label, "&File");
        assert_eq!(
            entries[0],
            MenuEntry::Item {
                id: 0,
                label: "&Save".into(),
                accelerator: Some("Ctrl+S".into()),
                enabled: false,
                checked: None,
            }
        );
        assert_eq!(entries[1], MenuEntry::Separator);
        assert!(matches!(
            entries[2],
            MenuEntry::Item {
                id: 1,
                checked: Some(true),
                ..
            }
        ));
        assert!(resolved.action(0).is_none(), "disabled items do nothing");
        flag.set(true);
        assert!(Resolved::of(&menu, "Notes").action(0).is_some());
    }

    #[test]
    fn accelerators_pick_enabled_items_and_leave_edit_keys_to_the_document() {
        let ran = Rc::new(Cell::new(0));
        let menu = Menu::new().submenu("Edit", |menu| {
            let ran = ran.clone();
            menu.item(
                MenuItem::new("Find")
                    .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::F))
                    .on_select(move || ran.set(ran.get() + 1)),
            )
            .role(Role::Copy)
            .role(Role::CloseWindow)
        });
        let resolved = Resolved::of(&menu, "Notes");
        let Some(Action::Run(find)) =
            resolved.accelerator_action(platform(true, false), 'F' as i32, Some("f"))
        else {
            panic!("Ctrl+F chooses Find");
        };
        find();
        assert_eq!(ran.get(), 1);
        assert!(resolved
            .accelerator_action(platform(true, true), 'F' as i32, Some("F"))
            .is_none());
        assert!(
            resolved
                .accelerator_action(platform(true, false), 'C' as i32, Some("c"))
                .is_none(),
            "the document copies"
        );
        assert!(matches!(
            resolved.accelerator_action(platform(true, false), 'W' as i32, Some("w")),
            Some(Action::Role(Role::CloseWindow))
        ));
    }

    #[test]
    fn roles_use_the_platform_conventions() {
        let resolved = Resolved::of(
            &Menu::new()
                .role(Role::Quit)
                .role(Role::Redo)
                .role(Role::About),
            "A&B",
        );
        let labels: Vec<_> = resolved
            .entries
            .iter()
            .map(|entry| match entry {
                MenuEntry::Item {
                    label, accelerator, ..
                } => (label.clone(), accelerator.clone()),
                _ => unreachable!(),
            })
            .collect();
        if cfg!(target_os = "windows") {
            assert_eq!(labels[0], ("E&xit".into(), None));
            assert_eq!(labels[1], ("&Redo".into(), Some("Ctrl+Y".into())));
        } else {
            assert_eq!(labels[0], ("&Quit".into(), Some("Ctrl+Q".into())));
            assert_eq!(labels[1], ("&Redo".into(), Some("Ctrl+Shift+Z".into())));
        }
        assert_eq!(labels[2], ("&About A&&B".into(), None));
    }
}
