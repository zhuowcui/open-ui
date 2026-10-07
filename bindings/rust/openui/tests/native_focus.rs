use openui::prelude::*;
use openui::{AccessibilityAction, Error};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<(String, String, String)>>>;

fn fixture() -> (Document, [Element; 3], Log) {
    let document = Document::new(320, 200).unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    let elements = ["a", "b", "c"].map(|id| {
        let element = Element::create(&document, "input").unwrap();
        element.set_id(id).unwrap();
        document.body().append_child(&element).unwrap();
        for event_type in ["focus", "blur"] {
            let retained = document.clone();
            let log = log.clone();
            element
                .on(event_type, move |_| {
                    let focused = retained
                        .focused_element()
                        .unwrap()
                        .map(|element| element.get_attribute("id").unwrap().unwrap())
                        .unwrap_or_else(|| "body".into());
                    log.borrow_mut()
                        .push((id.into(), event_type.into(), focused));
                })
                .unwrap();
        }
        element
    });
    (document, elements, log)
}

fn rows(expected: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
    expected
        .iter()
        .map(|(id, event, focused)| ((*id).into(), (*event).into(), (*focused).into()))
        .collect()
}

#[test]
fn native_focus_callback_order_matches_the_pinned_chromium_observations() {
    for scenario in 0..6 {
        let (document, [a, b, c], log) = fixture();
        match scenario {
            1 => {
                let next = c.clone();
                a.on("blur", move |_| next.focus().unwrap()).unwrap();
            }
            2 => {
                let next = b.clone();
                a.on("focus", move |_| next.focus().unwrap()).unwrap();
            }
            3 => {
                let next = b.clone();
                a.on("blur", move |_| next.remove().unwrap()).unwrap();
            }
            4 => {
                let next = b.clone();
                a.on("blur", move |_| next.set_attribute("disabled", "").unwrap())
                    .unwrap();
            }
            5 => {
                let next = c.clone();
                a.on("blur", move |_| {
                    next.focus().unwrap();
                    next.blur().unwrap();
                })
                .unwrap();
            }
            _ => {}
        }
        a.focus().unwrap();
        if scenario != 2 {
            b.focus().unwrap();
        }
        if scenario == 0 {
            b.focus().unwrap();
            a.blur().unwrap(); // Blurring an unfocused element is a no-op.
            b.blur().unwrap();
            b.blur().unwrap();
        }
        let mut expected = vec![("a", "focus", "a"), ("a", "blur", "body")];
        match scenario {
            0 => expected.extend([("b", "focus", "b"), ("b", "blur", "body")]),
            1 => expected.push(("c", "focus", "c")),
            2 => expected.push(("b", "focus", "b")),
            5 => expected.extend([
                ("c", "focus", "c"),
                ("c", "blur", "body"),
                ("b", "focus", "b"),
            ]),
            _ => {}
        }
        assert_eq!(*log.borrow(), rows(&expected), "scenario {scenario}");
        let focused = document
            .focused_element()
            .unwrap()
            .map(|element| element.get_attribute("id").unwrap().unwrap());
        assert_eq!(
            focused.as_deref(),
            match scenario {
                1 => Some("c"),
                2 | 5 => Some("b"),
                _ => None,
            }
        );
        for element in [a, b, c] {
            element.remove_event("focus").unwrap();
            element.remove_event("blur").unwrap();
        }
    }
}

#[test]
fn native_focus_transfer_cancels_composition_before_blur_and_allows_callback_mutation() {
    let (document, [a, b, c], log) = fixture();
    a.set_control_value("kept").unwrap();
    a.set_selection(4, 4).unwrap();
    a.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let callback_log = observed.clone();
    let old = a.clone();
    let sibling = c.clone();
    a.on("compositionend", move |event| {
        callback_log.borrow_mut().push((
            event.key_text.clone(),
            old.has_focus().unwrap(),
            old.control_value().unwrap().unwrap(),
        ));
        sibling.set_attribute("data-cancel", "seen").unwrap();
    })
    .unwrap();
    b.focus().unwrap();
    assert_eq!(*observed.borrow(), [("".into(), true, "kept".into())]);
    assert_eq!(
        c.get_attribute("data-cancel").unwrap().as_deref(),
        Some("seen")
    );
    assert_eq!(
        *log.borrow(),
        rows(&[
            ("a", "focus", "a"),
            ("a", "blur", "body"),
            ("b", "focus", "b")
        ])
    );
    document.dispatch_composition_end("late commit").unwrap();
    assert_eq!(a.control_value().unwrap().as_deref(), Some("kept"));
    for element in [a, b, c] {
        for event in ["focus", "blur", "compositionend"] {
            element.remove_event(event).unwrap();
        }
    }
}

#[test]
fn composition_callback_focus_redirect_aborts_the_pending_request() {
    let (document, [a, b, c], log) = fixture();
    a.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    let next = c.clone();
    a.on("compositionend", move |_| next.focus().unwrap())
        .unwrap();
    b.focus().unwrap();
    assert!(c.has_focus().unwrap());
    assert_eq!(
        *log.borrow(),
        rows(&[
            ("a", "focus", "a"),
            ("a", "blur", "body"),
            ("c", "focus", "c")
        ])
    );
    for element in [a, b, c] {
        for event in ["focus", "blur", "compositionend"] {
            element.remove_event(event).unwrap();
        }
    }
}

#[test]
fn disabled_focus_request_preserves_focus_and_an_active_composition() {
    let (document, [a, b, c], log) = fixture();
    a.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    b.set_attribute("disabled", "").unwrap();
    assert!(matches!(
        b.focus(),
        Err(Error::Engine(openui_engine::EngineError::NotFocusable))
    ));
    assert!(a.has_focus().unwrap());
    assert_eq!(*log.borrow(), rows(&[("a", "focus", "a")]));
    document.dispatch_composition_end("committed").unwrap();
    assert_eq!(a.control_value().unwrap().as_deref(), Some("committed"));
    for element in [a, b, c] {
        element.remove_event("focus").unwrap();
        element.remove_event("blur").unwrap();
    }
}

#[test]
fn keyboard_and_accessibility_focus_use_the_same_callback_transition() {
    let (document, [a, b, c], log) = fixture();
    document.advance_focus(1).unwrap();
    document.advance_focus(1).unwrap();
    c.perform_accessibility_action(AccessibilityAction::Focus)
        .unwrap();
    c.perform_accessibility_action(AccessibilityAction::Blur)
        .unwrap();
    assert_eq!(
        *log.borrow(),
        rows(&[
            ("a", "focus", "a"),
            ("a", "blur", "body"),
            ("b", "focus", "b"),
            ("b", "blur", "body"),
            ("c", "focus", "c"),
            ("c", "blur", "body"),
        ])
    );
    for element in [a, b, c] {
        element.remove_event("focus").unwrap();
        element.remove_event("blur").unwrap();
    }
}
