//! A pointer that leaves the window leaves its whole hover path, as Chromium
//! does when the mouse leaves the page (RFC 0001, state sources).

use openui::prelude::*;
use openui::{MouseEventType, PointerEventKind};
use std::cell::RefCell;
use std::rc::Rc;

fn block(document: &Document, parent: &Element) -> Element {
    let element = Element::create(document, "div").unwrap();
    element
        .set_property(StyleProperty::Height, LengthValue::px(40.0).into())
        .unwrap();
    parent.append_child(&element).unwrap();
    element
}

fn record(log: &Rc<RefCell<Vec<String>>>, element: &Element, name: &'static str) {
    for event in [
        "mouseleave",
        "mouseenter",
        "pointerleave",
        "pointerenter",
        "pointercancel",
        "mousecancel",
        "click",
    ] {
        let log = log.clone();
        element
            .on(event, move |_| {
                log.borrow_mut().push(format!("{event} {name}"))
            })
            .unwrap();
    }
}

#[test]
fn leaving_the_window_leaves_the_hover_path_innermost_first() {
    let document = Document::new(100, 80).unwrap();
    let outer = block(&document, &document.body());
    let inner = block(&document, &outer);
    document.update_all().unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    record(&log, &outer, "outer");
    record(&log, &inner, "inner");

    document
        .dispatch_mouse_event(
            MouseEventType::Move,
            10.0,
            10.0,
            MouseButton::Left,
            Modifiers::NONE,
        )
        .unwrap();
    assert_eq!(*log.borrow(), ["mouseenter outer", "mouseenter inner"]);
    log.borrow_mut().clear();

    document
        .dispatch_pointer_leave(0, 10.0, 10.0, MouseButton::Left, Modifiers::NONE, false)
        .unwrap();
    assert_eq!(*log.borrow(), ["mouseleave inner", "mouseleave outer"]);
    assert!(!inner.is_hovered().unwrap());
    assert!(!outer.is_hovered().unwrap());
    log.borrow_mut().clear();

    // Nothing is hovered, so a second leave dispatches nothing.
    document
        .dispatch_pointer_leave(0, 10.0, 10.0, MouseButton::Left, Modifiers::NONE, false)
        .unwrap();
    assert!(log.borrow().is_empty());
}

#[test]
fn a_press_survives_leaving_and_coming_back() {
    let document = Document::new(100, 80).unwrap();
    let outer = block(&document, &document.body());
    let inner = block(&document, &outer);
    document.update_all().unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    record(&log, &inner, "inner");

    let pointer = |kind| {
        document
            .dispatch_pointer_event(
                1,
                kind,
                10.0,
                10.0,
                MouseButton::Left,
                Modifiers::NONE,
                true,
            )
            .unwrap()
    };
    pointer(PointerEventKind::Down);
    assert!(inner.is_active().unwrap());
    document
        .dispatch_pointer_leave(1, 10.0, 10.0, MouseButton::Left, Modifiers::NONE, true)
        .unwrap();
    assert!(inner.is_active().unwrap());
    pointer(PointerEventKind::Move);
    pointer(PointerEventKind::Up);
    assert_eq!(
        *log.borrow(),
        [
            "pointerenter inner",
            "pointerleave inner",
            "pointerenter inner",
            "click inner"
        ]
    );
}
