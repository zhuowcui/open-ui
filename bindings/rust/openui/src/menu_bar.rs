//! A window's menu: what its items do and, where the platform has no native
//! menu bar, the bar drawn at the top of the window.
//!
//! The drawn bar is ordinary document content: a fixed strip with a spacer
//! under it, so the page lays out below it and scrolls under it. It opens
//! with a click, F10 or Alt with a menu's mnemonic, and is driven with the
//! arrow keys, Enter, Space, Escape and item mnemonics. While a menu is open
//! it takes every key, as native menus do. Choosing an item is recorded and
//! run by the window after the event that chose it, so its action runs with
//! the window current and the document free.

use crate::document::Document;
use crate::element::Element;
use crate::menu::{Action, Resolved};
use crate::view_node::ViewNode;
use crate::AccessibilityRole;
use openui_platform::{KeyPhase, KeyboardInput, MenuEntry};
use openui_style::{
    Border, BorderStyle, BoxShadow, Color, Display, Edges, FlexDirection, FontFamily,
    FontFamilyList, GenericFontFamily, Length, LengthValue, Position, Style, TextDecorationLine,
    TextWrapMode,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The bar's height in CSS pixels; the spacer under it matches.
const BAR_HEIGHT: f32 = 24.0;

const F10: i32 = 121;
const ENTER: i32 = 13;
const ESCAPE: i32 = 27;
const SPACE: i32 = 32;
const LEFT: i32 = 37;
const UP: i32 = 38;
const RIGHT: i32 = 39;
const DOWN: i32 = 40;

/// Which menus are open: empty when the bar is closed, else the open
/// top-level entry followed by the highlighted entry at each open depth.
/// A submenu entry at `path[k]` is expanded when `path` goes deeper.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct State {
    path: Vec<usize>,
    /// Underline mnemonics, as after F10 or Alt.
    mnemonics: bool,
}

/// A window's menu: the resolved entries and, when drawn, the bar.
#[derive(Default)]
pub(crate) struct WindowMenu {
    resolved: RefCell<Resolved>,
    state: RefCell<State>,
    chosen: Cell<Option<u32>>,
    drawn: RefCell<Option<Drawn>>,
    /// Elements of earlier drawings, detached but kept alive: one may be
    /// the target of the event still being dispatched.
    detached: RefCell<Vec<Element>>,
    /// A bar listener is running inside an event dispatch.
    dispatching: Cell<bool>,
}

struct Drawn {
    document: Document,
    root: Element,
}

impl WindowMenu {
    pub(crate) fn resolved(&self) -> std::cell::Ref<'_, Resolved> {
        self.resolved.borrow()
    }

    /// Replace the resolved menu and redraw the bar, if there is one.
    pub(crate) fn set_resolved(self: &Rc<Self>, resolved: Resolved) {
        *self.resolved.borrow_mut() = resolved;
        let mut state = self.state.borrow().clone();
        state.path = self.valid_path(&state.path);
        *self.state.borrow_mut() = state;
        self.redraw();
    }

    /// The drawn bar and the spacer that keeps the page below it.
    pub(crate) fn view(self: &Rc<Self>) -> ViewNode {
        let document = crate::current_document();
        let root = element(&document, "div", bar_style());
        let _ = root.set_accessibility_role(AccessibilityRole::MenuBar);
        let spacer = element(
            &document,
            "div",
            Style::default().height(LengthValue::px(BAR_HEIGHT)),
        );
        let _ = spacer.set_accessibility_hidden(true);
        *self.drawn.borrow_mut() = Some(Drawn {
            document,
            root: root.clone(),
        });
        self.redraw();
        ViewNode::Fragment(vec![ViewNode::Element(root), ViewNode::Element(spacer)])
    }

    /// The action of the item chosen since the last call, if any. Call it
    /// after each event's dispatch: it also frees earlier drawings.
    pub(crate) fn take_chosen(&self) -> Option<Action> {
        self.settle();
        let id = self.chosen.take()?;
        self.resolved.borrow().action(id)
    }

    /// Destroy the elements of earlier drawings. Only call it outside the
    /// bar's listeners.
    fn settle(&self) {
        for element in self.detached.take() {
            let _ = element.remove();
        }
    }

    /// Handle a key for the drawn bar. Returns whether the bar used it.
    pub(crate) fn key_input(self: &Rc<Self>, input: &KeyboardInput) -> bool {
        if self.drawn.borrow().is_none() {
            return false;
        }
        let open = !self.state.borrow().path.is_empty();
        if input.phase != KeyPhase::Down {
            return open;
        }
        let modifiers = input.modifiers;
        let plain = !modifiers.control && !modifiers.alt && !modifiers.meta;
        if !open {
            if input.key_code == F10 && plain && !modifiers.shift {
                self.open_top(0, true);
                return true;
            }
            if modifiers.alt && !modifiers.control && !modifiers.meta {
                let top = self.resolved.borrow().entries.clone();
                if let Some(index) = mnemonic_index(&top, input) {
                    self.open_top(index, true);
                    return true;
                }
            }
            return false;
        }
        match input.key_code {
            ESCAPE => self.back(),
            F10 => self.close(),
            LEFT => self.left(),
            RIGHT => self.right(),
            UP => self.step(false),
            DOWN => self.step(true),
            ENTER | SPACE => self.activate_highlight(),
            _ => self.choose_mnemonic(input),
        }
        true
    }

    fn set_state(self: &Rc<Self>, state: State) {
        if *self.state.borrow() == state {
            return;
        }
        *self.state.borrow_mut() = state;
        self.redraw();
    }

    fn close(self: &Rc<Self>) {
        self.set_state(State::default());
    }

    /// Open top-level entry `index`, highlighting its first item when the
    /// keyboard opened it.
    fn open_top(self: &Rc<Self>, index: usize, keyboard: bool) {
        let top = self.resolved.borrow().entries.clone();
        let mut path = vec![index];
        if keyboard {
            if let Some(MenuEntry::Submenu { entries, .. }) = top.get(index) {
                path.extend(next_selectable(entries, None, true));
            }
        }
        self.set_state(State {
            path,
            mnemonics: keyboard,
        });
    }

    fn back(self: &Rc<Self>) {
        let mut state = self.state.borrow().clone();
        if state.path.len() > 2 {
            state.path.pop();
            self.set_state(state);
        } else {
            self.close();
        }
    }

    fn left(self: &Rc<Self>) {
        let state = self.state.borrow().clone();
        if state.path.len() > 2 {
            self.back();
        } else {
            self.switch_top(false);
        }
    }

    fn right(self: &Rc<Self>) {
        let state = self.state.borrow().clone();
        let entries = self.resolved.borrow().entries.clone();
        if let Some(MenuEntry::Submenu {
            entries: children, ..
        }) = entry_at(&entries, &state.path)
        {
            if state.path.len() > 1 {
                let mut state = state.clone();
                state.path.extend(next_selectable(children, None, true));
                self.set_state(state);
                return;
            }
        }
        self.switch_top(true);
    }

    fn switch_top(self: &Rc<Self>, forward: bool) {
        let count = self.resolved.borrow().entries.len();
        let Some(&current) = self.state.borrow().path.first() else {
            return;
        };
        if count == 0 {
            return;
        }
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        self.open_top(next, true);
    }

    /// Move the highlight within the deepest open menu.
    fn step(self: &Rc<Self>, forward: bool) {
        let mut state = self.state.borrow().clone();
        let entries = self.resolved.borrow().entries.clone();
        let (menu_path, current) = if state.path.len() == 1 {
            (state.path.clone(), None)
        } else {
            let (last, parent) = state.path.split_last().expect("the bar is open");
            (parent.to_vec(), Some(*last))
        };
        let Some(MenuEntry::Submenu { entries: menu, .. }) = entry_at(&entries, &menu_path) else {
            return;
        };
        if let Some(next) = next_selectable(menu, current, forward) {
            state.path = menu_path;
            state.path.push(next);
            self.set_state(state);
        }
    }

    fn activate_highlight(self: &Rc<Self>) {
        let path = self.state.borrow().path.clone();
        self.activate(&path);
    }

    /// Choose the item at `path`, or expand the submenu there.
    fn activate(self: &Rc<Self>, path: &[usize]) {
        let entries = self.resolved.borrow().entries.clone();
        match entry_at(&entries, path) {
            Some(MenuEntry::Item {
                id, enabled: true, ..
            }) => {
                self.chosen.set(Some(*id));
                self.close();
            }
            Some(MenuEntry::Submenu {
                entries: children,
                enabled: true,
                ..
            }) => {
                let mut state = self.state.borrow().clone();
                state.path = path.to_vec();
                state.path.extend(next_selectable(children, None, true));
                self.set_state(state);
            }
            _ => {}
        }
    }

    fn choose_mnemonic(self: &Rc<Self>, input: &KeyboardInput) {
        let path = self.state.borrow().path.clone();
        let entries = self.resolved.borrow().entries.clone();
        // The deepest open menu: the top menu alone, else the highlighted
        // entry's menu.
        let menu_path = if path.len() > 1 {
            &path[..path.len() - 1]
        } else {
            &path[..]
        };
        if let Some(MenuEntry::Submenu { entries: menu, .. }) = entry_at(&entries, menu_path) {
            if let Some(index) = mnemonic_index(menu, input) {
                let mut target = menu_path.to_vec();
                target.push(index);
                self.activate(&target);
            }
        }
    }

    /// Keep only the part of `path` that still names entries.
    fn valid_path(&self, path: &[usize]) -> Vec<usize> {
        let entries = self.resolved.borrow().entries.clone();
        let mut valid = Vec::new();
        for &index in path {
            valid.push(index);
            if entry_at(&entries, &valid).is_none() {
                valid.pop();
                break;
            }
        }
        valid
    }

    fn redraw(self: &Rc<Self>) {
        let drawn = self.drawn.borrow();
        let Some(Drawn { document, root }) = drawn.as_ref() else {
            return;
        };
        let entries = self.resolved.borrow().entries.clone();
        let state = self.state.borrow().clone();
        if !self.dispatching.get() {
            self.settle();
        }
        let mut child = root.first_child().ok().flatten();
        while let Some(current) = child {
            child = current.next_sibling().ok().flatten();
            let _ = current.detach();
            self.detached.borrow_mut().push(current);
        }
        let weak = Rc::downgrade(self);
        if !state.path.is_empty() {
            // Catches clicks outside the open menus and closes them.
            let overlay = element(document, "div", overlay_style());
            let _ = overlay.set_accessibility_hidden(true);
            listen(&overlay, "click", &weak, |menu| menu.close());
            let _ = root.append_child(&overlay);
        }
        for (index, entry) in entries.iter().enumerate() {
            let open = state.path.first() == Some(&index);
            let (label, enabled) = match entry {
                MenuEntry::Submenu { label, enabled, .. } => (label, *enabled),
                MenuEntry::Item { label, enabled, .. } => (label, *enabled),
                MenuEntry::Separator => continue,
            };
            let item = element(document, "div", top_item_style(open, enabled));
            let _ = item.set_accessibility_role(AccessibilityRole::MenuItem);
            let _ = item.set_accessibility_label(&plain_label(label));
            append_label(document, &item, label, state.mnemonics);
            if enabled {
                listen(&item, "click", &weak, move |menu| {
                    let entries = menu.resolved.borrow().entries.clone();
                    match entries.get(index) {
                        Some(MenuEntry::Item { .. }) => menu.activate(&[index]),
                        _ if menu.state.borrow().path.first() == Some(&index) => menu.close(),
                        _ => menu.open_top(index, false),
                    }
                });
                listen(&item, "mouseenter", &weak, move |menu| {
                    let path = menu.state.borrow().path.clone();
                    if !path.is_empty() && path[0] != index {
                        let mnemonics = menu.state.borrow().mnemonics;
                        menu.set_state(State {
                            path: vec![index],
                            mnemonics,
                        });
                    }
                });
            }
            if let (true, MenuEntry::Submenu { entries, .. }) = (open, entry) {
                let menu = self.draw_menu(document, entries, &state, &[index]);
                let _ = menu.apply_style(
                    &Style::default()
                        .top(Length::percent(100.0))
                        .left(Length::zero()),
                );
                let _ = item.append_child(&menu);
            }
            let _ = root.append_child(&item);
        }
    }

    /// Draw the open menu whose entries are `entries`, found at `at`.
    fn draw_menu(
        self: &Rc<Self>,
        document: &Document,
        entries: &[MenuEntry],
        state: &State,
        at: &[usize],
    ) -> Element {
        let depth = at.len();
        let weak = Rc::downgrade(self);
        let menu = element(document, "div", menu_style());
        let _ = menu.set_accessibility_role(AccessibilityRole::Menu);
        for (index, entry) in entries.iter().enumerate() {
            let mut path = at.to_vec();
            path.push(index);
            let highlighted = state.path.get(depth) == Some(&index);
            let (label, accelerator, enabled, checked, children) = match entry {
                MenuEntry::Separator => {
                    let line = element(document, "div", separator_style());
                    let _ = line.set_accessibility_role(AccessibilityRole::Splitter);
                    let _ = menu.append_child(&line);
                    continue;
                }
                MenuEntry::Item {
                    label,
                    accelerator,
                    enabled,
                    checked,
                    ..
                } => (label, accelerator.as_deref(), *enabled, *checked, None),
                MenuEntry::Submenu {
                    label,
                    enabled,
                    entries,
                } => (label, None, *enabled, None, Some(entries)),
            };
            let row = element(document, "div", row_style(highlighted && enabled, enabled));
            let role = if checked.is_some() {
                AccessibilityRole::MenuItemCheckBox
            } else {
                AccessibilityRole::MenuItem
            };
            let _ = row.set_accessibility_role(role);
            let _ = row.set_accessibility_label(&plain_label(label));
            if checked == Some(true) {
                let _ = row.set_accessibility_description("checked");
            }
            if !enabled {
                let _ = row.set_accessibility_description("unavailable");
            }
            let check = element(document, "span", gutter_style());
            if checked == Some(true) {
                let _ = check.append_text_node("\u{2713}");
            }
            let _ = row.append_child(&check);
            let text = element(document, "span", Style::default().flex_grow(1.0));
            append_label(document, &text, label, state.mnemonics);
            let _ = row.append_child(&text);
            let hint = accelerator
                .map(str::to_owned)
                .or(children.map(|_| "\u{25b8}".into()));
            if let Some(hint) = hint {
                let shortcut = element(document, "span", shortcut_style(enabled));
                let _ = shortcut.append_text_node(&hint);
                let _ = row.append_child(&shortcut);
            }
            if enabled {
                let hover = path.clone();
                listen(&row, "mouseenter", &weak, move |menu| {
                    let mut state = menu.state.borrow().clone();
                    state.path = hover.clone();
                    let entries = menu.resolved.borrow().entries.clone();
                    if let Some(MenuEntry::Submenu { entries, .. }) = entry_at(&entries, &hover) {
                        state.path.extend(next_selectable(entries, None, true));
                    }
                    menu.set_state(state);
                });
                let chosen = path.clone();
                listen(&row, "click", &weak, move |menu| menu.activate(&chosen));
            }
            if let Some(children) = children {
                if highlighted && state.path.len() > depth + 1 {
                    let flyout = self.draw_menu(document, children, state, &path);
                    let _ = flyout.apply_style(
                        &Style::default()
                            .top(Length::px(-4.0))
                            .left(Length::percent(100.0)),
                    );
                    let _ = row.append_child(&flyout);
                }
            }
            let _ = menu.append_child(&row);
        }
        menu
    }
}

/// Run `action` on the menu for each `event` on `target`, while the menu
/// lives. The listener holds the menu weakly: the document must not keep
/// its own window's menu alive.
fn listen(
    target: &Element,
    event: &str,
    menu: &Weak<WindowMenu>,
    action: impl Fn(&Rc<WindowMenu>) + 'static,
) {
    let menu = menu.clone();
    let _ = target.on(event, move |event| {
        event.stop_propagation();
        if let Some(menu) = menu.upgrade() {
            menu.dispatching.set(true);
            action(&menu);
            menu.dispatching.set(false);
        }
    });
}

/// The entry `path` names, starting at the top-level `entries`.
fn entry_at<'a>(entries: &'a [MenuEntry], path: &[usize]) -> Option<&'a MenuEntry> {
    let (first, rest) = path.split_first()?;
    let mut entry = entries.get(*first)?;
    for &index in rest {
        let MenuEntry::Submenu { entries, .. } = entry else {
            return None;
        };
        entry = entries.get(index)?;
    }
    Some(entry)
}

/// The next entry that can be highlighted after `current`, wrapping round.
fn next_selectable(entries: &[MenuEntry], current: Option<usize>, forward: bool) -> Option<usize> {
    let count = entries.len();
    let selectable = |index: usize| match &entries[index] {
        MenuEntry::Item { enabled, .. } | MenuEntry::Submenu { enabled, .. } => *enabled,
        MenuEntry::Separator => false,
    };
    (1..=count)
        .map(|offset| match (current, forward) {
            (None, true) => offset - 1,
            (None, false) => count - offset,
            (Some(current), true) => (current + offset) % count,
            (Some(current), false) => (current + count - offset % count) % count,
        })
        .find(|&index| selectable(index))
}

/// The entry whose mnemonic is the pressed key's character.
fn mnemonic_index(entries: &[MenuEntry], input: &KeyboardInput) -> Option<usize> {
    let pressed = input
        .key_text
        .as_deref()
        .and_then(|text| text.chars().next())
        .or_else(|| char::from_u32(input.key_code as u32).filter(char::is_ascii_alphanumeric))?
        .to_lowercase()
        .next()?;
    entries.iter().position(|entry| match entry {
        MenuEntry::Item {
            label,
            enabled: true,
            ..
        }
        | MenuEntry::Submenu {
            label,
            enabled: true,
            ..
        } => {
            split_mnemonic(label)
                .1
                .and_then(|mnemonic| mnemonic.to_lowercase().next())
                == Some(pressed)
        }
        _ => false,
    })
}

/// Split a label into the text before its mnemonic, the mnemonic, and the
/// text after it. `&&` is a literal ampersand.
fn split_mnemonic(label: &str) -> (String, Option<char>, String) {
    let mut before = String::new();
    let mut characters = label.chars();
    while let Some(character) = characters.next() {
        if character != '&' {
            before.push(character);
            continue;
        }
        match characters.next() {
            Some('&') => before.push('&'),
            Some(mnemonic) => {
                let after = characters.as_str().replace("&&", "&");
                return (before, Some(mnemonic), after);
            }
            None => {}
        }
    }
    (before, None, String::new())
}

fn plain_label(label: &str) -> String {
    let (before, mnemonic, after) = split_mnemonic(label);
    let mut text = before;
    text.extend(mnemonic);
    text.push_str(&after);
    text
}

/// Show `label` in `parent`, underlining its mnemonic when `underline`.
fn append_label(document: &Document, parent: &Element, label: &str, underline: bool) {
    let (before, mnemonic, after) = split_mnemonic(label);
    match (mnemonic, underline) {
        (Some(mnemonic), true) => {
            if !before.is_empty() {
                let _ = parent.append_text_node(&before);
            }
            let marked = element(
                document,
                "span",
                Style::default().text_decoration_line(TextDecorationLine::UNDERLINE),
            );
            let _ = marked.append_text_node(&mnemonic.to_string());
            let _ = parent.append_child(&marked);
            if !after.is_empty() {
                let _ = parent.append_text_node(&after);
            }
        }
        _ => {
            let _ = parent.append_text_node(&plain_label(label));
        }
    }
}

fn element(document: &Document, tag: &str, style: Style) -> Element {
    let element = Element::create(document, tag).expect("menu bar elements are valid tags");
    let _ = element.apply_style(&style);
    element
}

fn px(value: f32) -> LengthValue {
    LengthValue::px(value)
}

fn edges(vertical: f32, horizontal: f32) -> Edges<LengthValue> {
    Edges {
        top: px(vertical),
        right: px(horizontal),
        bottom: px(vertical),
        left: px(horizontal),
    }
}

fn gray(value: u32) -> Color {
    Color::from_hex(value * 0x010101, false)
}

fn bar_style() -> Style {
    let font = FontFamilyList::new([FontFamily::Generic(GenericFontFamily::SansSerif)])
        .expect("one family");
    Style::default()
        .display(Display::Flex)
        .position(Position::Fixed)
        .top(Length::zero())
        .left(Length::zero())
        .right(Length::zero())
        .height(px(BAR_HEIGHT))
        .z_index(1000)
        .background_color(gray(0xf3))
        .border_bottom(Border {
            width: 1.0,
            style: BorderStyle::Solid,
            color: gray(0xd9),
        })
        .font_family(font)
        .font_size(px(13.0))
        .color(gray(0x1f))
        .text_wrap_mode(TextWrapMode::Nowrap)
}

fn overlay_style() -> Style {
    Style::default()
        .position(Position::Fixed)
        .top(Length::zero())
        .left(Length::zero())
        .width(LengthValue::ViewportWidth(100.0))
        .height(LengthValue::ViewportHeight(100.0))
}

fn top_item_style(open: bool, enabled: bool) -> Style {
    let style = Style::default()
        .position(Position::Relative)
        .z_index(1)
        .height(px(BAR_HEIGHT - 8.0))
        .padding(edges(4.0, 8.0))
        .color(gray(if enabled { 0x1f } else { 0xa0 }));
    if open {
        style.background_color(gray(0xdc))
    } else {
        style
    }
}

fn menu_style() -> Style {
    Style::default()
        .display(Display::Flex)
        .flex_direction(FlexDirection::Column)
        .position(Position::Absolute)
        .min_width(px(200.0))
        .padding(edges(4.0, 0.0))
        .background_color(Color::WHITE)
        .border(Border {
            width: 1.0,
            style: BorderStyle::Solid,
            color: gray(0xc4),
        })
        .box_shadow(vec![BoxShadow {
            offset_x: 0.0,
            offset_y: 2.0,
            blur_radius: 6.0,
            spread_radius: 0.0,
            color: Color::from_hex(0x00000033, true),
            inset: false,
        }])
}

fn row_style(highlighted: bool, enabled: bool) -> Style {
    let style = Style::default()
        .display(Display::Flex)
        .position(Position::Relative)
        .padding(edges(4.0, 12.0))
        .color(gray(if enabled { 0x1f } else { 0xa0 }));
    if highlighted {
        style.background_color(gray(0xe5))
    } else {
        style
    }
}

fn gutter_style() -> Style {
    Style::default().width(px(20.0))
}

fn shortcut_style(enabled: bool) -> Style {
    Style::default()
        .margin_left(px(32.0))
        .color(gray(if enabled { 0x6b } else { 0xb4 }))
}

fn separator_style() -> Style {
    Style::default()
        .height(px(1.0))
        .margin(Edges {
            top: px(4.0),
            right: px(0.0),
            bottom: px(4.0),
            left: px(0.0),
        })
        .background_color(gray(0xdc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{Menu, MenuItem, Role};
    use crate::view_node::IntoView;
    use crate::{HeadlessApp, MouseButton, MouseEventType, ViewportMetrics};

    fn key(code: i32, text: Option<&str>, alt: bool) -> KeyboardInput {
        KeyboardInput {
            phase: KeyPhase::Down,
            key_code: code,
            key_text: text.map(Into::into),
            text: text.map(Into::into),
            modifiers: openui_platform::Modifiers {
                alt,
                ..Default::default()
            },
            repeat: false,
        }
    }

    fn click(document: &Document, x: f32, y: f32) {
        document
            .dispatch_mouse_event(
                MouseEventType::Up,
                x,
                y,
                MouseButton::Left,
                crate::Modifiers::NONE,
            )
            .unwrap();
    }

    fn click_on(document: &Document, element: &Element) -> crate::Rect {
        let rect = element.bounding_rect().unwrap().expect("laid out");
        click(
            document,
            rect.x + rect.width / 2.0,
            rect.y + rect.height / 2.0,
        );
        rect
    }

    fn path(menu: &WindowMenu) -> Vec<usize> {
        menu.state.borrow().path.clone()
    }

    /// The top-level item elements, after the overlay when a menu is open.
    fn top_items(root: &Element) -> Vec<Element> {
        let mut items = Vec::new();
        let mut next = root.first_child().unwrap();
        while let Some(item) = next {
            next = item.next_sibling().unwrap();
            items.push(item);
        }
        items
    }

    #[test]
    fn the_drawn_bar_opens_highlights_and_chooses() {
        let ran = Rc::new(Cell::new(0));
        let counter = ran.clone();
        let app_menu = Menu::new()
            .submenu("&File", |menu| {
                menu.item(MenuItem::new("&New").on_select(move || counter.set(counter.get() + 1)))
                    .separator()
                    .item(MenuItem::new("&Wrap").checked(true))
                    .item(MenuItem::new("&Print").enabled(false))
            })
            .submenu("&Edit", |menu| menu.role(Role::Copy));
        let menu = Rc::new(WindowMenu::default());
        menu.set_resolved(Resolved::of(&app_menu, "Test"));
        let mut app =
            HeadlessApp::new(ViewportMetrics::from_logical_size(320.0, 200.0, 1.0).unwrap())
                .unwrap();
        let shown = menu.clone();
        app.mount(move || ViewNode::Fragment(vec![shown.view(), "content".into_view()]))
            .unwrap();
        let document = app.document().clone();
        let root = menu.drawn.borrow().as_ref().unwrap().root.clone();
        let closed = app.render_at(0.0).unwrap();
        assert_eq!(root.text_content().unwrap(), "FileEdit");

        // A click on a top item opens its menu below the bar; a click on an
        // item chooses it and closes the menu.
        let bar = click_on(&document, &top_items(&root)[0]);
        assert_eq!(path(&menu), [0]);
        assert_ne!(app.render_at(0.0).unwrap().pixels(), closed.pixels());
        assert!(root.text_content().unwrap().contains("\u{2713}Wrap"));
        let items = top_items(&root);
        assert_eq!(items.len(), 3, "the overlay and two menus");
        let dropdown = items[1].first_child().unwrap().unwrap();
        let new = dropdown.first_child().unwrap().unwrap();
        let row = click_on(&document, &new);
        assert!(row.y >= bar.y + bar.height, "the menu opens below the bar");
        assert!(path(&menu).is_empty(), "choosing closes the menu");
        let Some(Action::Run(run)) = menu.take_chosen() else {
            panic!("New was chosen");
        };
        run();
        assert_eq!(ran.get(), 1);
        assert_eq!(app.render_at(0.0).unwrap().pixels(), closed.pixels());

        // A click outside an open menu closes it without choosing.
        click_on(&document, &top_items(&root)[0]);
        assert_eq!(path(&menu), [0]);
        click(&document, 300.0, 180.0);
        assert!(path(&menu).is_empty());
        assert!(menu.take_chosen().is_none());

        // F10 opens the first menu; arrows skip separators and disabled
        // items and move between menus; Escape closes.
        assert!(menu.key_input(&key(F10, None, false)));
        assert_eq!(path(&menu), [0, 0]);
        assert!(menu.state.borrow().mnemonics);
        menu.key_input(&key(DOWN, None, false));
        assert_eq!(path(&menu), [0, 2]);
        menu.key_input(&key(DOWN, None, false));
        assert_eq!(path(&menu), [0, 0], "Print is disabled");
        menu.key_input(&key(UP, None, false));
        assert_eq!(path(&menu), [0, 2]);
        menu.key_input(&key(RIGHT, None, false));
        assert_eq!(path(&menu), [1, 0]);
        assert!(
            menu.key_input(&key('Q' as i32, Some("q"), false)),
            "an open menu takes keys"
        );
        menu.key_input(&key(ESCAPE, None, false));
        assert!(path(&menu).is_empty());
        assert!(!menu.key_input(&key('Q' as i32, Some("q"), false)));

        // Alt with a mnemonic opens that menu; an item's mnemonic chooses it.
        assert!(menu.key_input(&key('F' as i32, Some("f"), true)));
        assert_eq!(path(&menu), [0, 0]);
        menu.key_input(&key('P' as i32, Some("p"), false));
        assert_eq!(path(&menu), [0, 0], "disabled items cannot be chosen");
        menu.key_input(&key('W' as i32, Some("w"), false));
        assert!(path(&menu).is_empty());
        assert!(matches!(menu.take_chosen(), Some(Action::Nothing)));
        assert!(!menu.key_input(&key('X' as i32, Some("x"), true)));
    }

    #[test]
    fn mnemonics_split_and_escape() {
        assert_eq!(
            split_mnemonic("&File"),
            (String::new(), Some('F'), "ile".into())
        );
        assert_eq!(
            split_mnemonic("Save &As"),
            ("Save ".into(), Some('A'), "s".into())
        );
        assert_eq!(
            split_mnemonic("A&&B &Now && then"),
            ("A&B ".into(), Some('N'), "ow & then".into())
        );
        assert_eq!(plain_label("Cu&t"), "Cut");
        assert_eq!(plain_label("Plain"), "Plain");
    }

    #[test]
    fn highlight_skips_separators_and_disabled_items() {
        let item = |id, enabled| MenuEntry::Item {
            id,
            label: format!("&{id}"),
            accelerator: None,
            enabled,
            checked: None,
        };
        let entries = vec![
            item(0, false),
            item(1, true),
            MenuEntry::Separator,
            item(2, true),
        ];
        assert_eq!(next_selectable(&entries, None, true), Some(1));
        assert_eq!(next_selectable(&entries, Some(1), true), Some(3));
        assert_eq!(next_selectable(&entries, Some(3), true), Some(1));
        assert_eq!(next_selectable(&entries, Some(1), false), Some(3));
        assert_eq!(next_selectable(&entries, None, false), Some(3));
        assert_eq!(next_selectable(&[MenuEntry::Separator], None, true), None);
    }
}
