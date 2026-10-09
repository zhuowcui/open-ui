use openui::prelude::*;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn input(document: &Document, parent: &Element) -> Element {
    let element = Element::create(document, "input").unwrap();
    element.set_attribute("type", "checkbox").unwrap();
    element
        .set_property(StyleProperty::Display, Display::Block.into())
        .unwrap();
    element
        .set_property(StyleProperty::Width, LengthValue::px(30.0).into())
        .unwrap();
    element
        .set_property(StyleProperty::Height, LengthValue::px(30.0).into())
        .unwrap();
    parent.append_child(&element).unwrap();
    element
}

#[test]
fn click_reentry_is_suppressed_and_panic_releases_the_dispatch_guard() {
    let document = Document::new(100, 100).unwrap();
    let element = input(&document, &document.body());
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    let weak = element.downgrade();
    element
        .on("click", move |_| {
            observed.set(observed.get() + 1);
            weak.upgrade().unwrap().click().unwrap();
        })
        .unwrap();
    element.click().unwrap();
    assert_eq!(count.get(), 1);
    element.remove_event("click").unwrap();
    element
        .on("click", |_| panic!("native callback panic"))
        .unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| element.click())).is_err());
    element.remove_event("click").unwrap();
    element.click().unwrap();
    assert!(element.is_checked().unwrap());
}

#[test]
fn destroyed_click_target_cannot_activate_a_reused_slot() {
    let document = Rc::new(Document::new(100, 100).unwrap());
    let callback_document = Rc::downgrade(&document);
    let element = input(&document, &document.body());
    let replacement: Rc<RefCell<Option<Element>>> = Rc::new(RefCell::new(None));
    let retained = replacement.clone();
    let weak = element.downgrade();
    // Retain only the weak document through the weak element until the callback.
    element
        .on("click", move |_| {
            let original = weak.upgrade().unwrap();
            let parent = original.parent().unwrap().unwrap();
            let document = callback_document.upgrade().unwrap();
            original.remove().unwrap();
            *retained.borrow_mut() = Some(input(&document, &parent));
        })
        .unwrap();
    element.click().unwrap();
    assert!(!replacement.borrow().as_ref().unwrap().is_checked().unwrap());
}

#[test]
fn parent_capture_pointer_and_keyboard_clicks_share_preactivation_and_cancellation() {
    let document = Document::new(100, 100).unwrap();
    let parent = Element::create(&document, "div").unwrap();
    document.body().append_child(&parent).unwrap();
    let element = input(&document, &parent);
    let canceled = Rc::new(Cell::new(true));
    let policy = canceled.clone();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let states = seen.clone();
    let weak = element.downgrade();
    parent
        .on_capture("click", move |event| {
            let control = weak.upgrade().unwrap();
            states.borrow_mut().push((
                control.is_checked().unwrap(),
                control.is_indeterminate().unwrap(),
            ));
            if policy.get() {
                event.prevent_default();
            }
        })
        .unwrap();
    element.set_indeterminate(true).unwrap();
    let box_ = element.bounding_rect().unwrap().unwrap();
    let x = box_.x + box_.width / 2.0;
    let y = box_.y + box_.height / 2.0;
    document
        .dispatch_mouse_event(
            MouseEventType::Down,
            x,
            y,
            MouseButton::Left,
            Modifiers::NONE,
        )
        .unwrap();
    document
        .dispatch_mouse_event(MouseEventType::Up, x, y, MouseButton::Left, Modifiers::NONE)
        .unwrap();
    assert_eq!(&*seen.borrow(), &[(true, false)]);
    assert!(!element.is_checked().unwrap());
    assert!(element.is_indeterminate().unwrap());
    canceled.set(false);
    element.focus().unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 32, Some(" "), Modifiers::NONE)
        .unwrap();
    assert_eq!(&*seen.borrow(), &[(true, false), (true, false)]);
    assert!(element.is_checked().unwrap());
    assert!(!element.is_indeterminate().unwrap());
    assert_eq!(element.get_attribute("checked").unwrap(), None);
}

#[test]
fn raw_click_dispatch_keeps_disabled_controls_dispatchable_and_reports_cancellation() {
    let document = Document::new(100, 100).unwrap();
    let element = input(&document, &document.body());
    element.set_attribute("disabled", "").unwrap();
    assert!(element.dispatch_click_event().unwrap());
    assert!(element.is_checked().unwrap());
    let weak = element.downgrade();
    element
        .on("click", move |event| {
            assert!(!weak.upgrade().unwrap().is_checked().unwrap());
            event.prevent_default();
        })
        .unwrap();
    assert!(!element.dispatch_click_event().unwrap());
    assert!(element.is_checked().unwrap());
}
