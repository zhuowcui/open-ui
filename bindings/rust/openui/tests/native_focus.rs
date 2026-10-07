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

#[test]
fn programmatic_negative_tabindex_focus_and_sequential_anchor_match_chromium() {
    for positive_order in [false, true] {
        let document = Document::new(320, 200).unwrap();
        let x = Element::create(&document, "input").unwrap();
        let a = Element::create(&document, "div").unwrap();
        let b = Element::create(&document, "input").unwrap();
        let c = Element::create(&document, "button").unwrap();
        for element in [&x, &a, &b, &c] {
            document.body().append_child(element).unwrap();
        }
        for element in [&a, &b] {
            element.set_attribute("tabindex", "-1").unwrap();
            element.focus().unwrap();
            assert!(element.has_focus().unwrap());
        }
        if positive_order {
            x.set_attribute("tabindex", "2").unwrap();
            c.set_attribute("tabindex", "1").unwrap();
        }
        document.advance_focus(1).unwrap();
        assert!(c.has_focus().unwrap());
        b.focus().unwrap();
        document.advance_focus(-1).unwrap();
        assert!(x.has_focus().unwrap());
        document.advance_focus(1).unwrap();
        assert!(c.has_focus().unwrap());
    }
}

#[test]
fn focus_and_blur_capture_then_target_without_bubbling_or_cancellation() {
    let document = Document::new(320, 200).unwrap();
    let parent = Element::create(&document, "div").unwrap();
    let input = Element::create(&document, "input").unwrap();
    document.body().append_child(&parent).unwrap();
    parent.append_child(&input).unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    for (element, label) in [(&parent, "parent"), (&input, "target")] {
        for event in ["focus", "blur"] {
            for capture in [true, false] {
                let log = log.clone();
                let callback = move |e: &openui::Event| {
                    e.prevent_default();
                    log.borrow_mut().push((
                        label,
                        event,
                        capture,
                        e.phase(),
                        e.default_prevented(),
                    ));
                };
                if capture {
                    element.on_capture(event, callback).unwrap();
                } else {
                    element.on(event, callback).unwrap();
                }
            }
        }
    }
    input.focus().unwrap();
    input.blur().unwrap();
    let mut expected = Vec::new();
    for event in ["focus", "blur"] {
        expected.extend([
            (
                "parent",
                event,
                true,
                Some(openui::EventPhase::Capture),
                false,
            ),
            (
                "target",
                event,
                true,
                Some(openui::EventPhase::Target),
                false,
            ),
            (
                "target",
                event,
                false,
                Some(openui::EventPhase::Target),
                false,
            ),
        ]);
    }
    assert_eq!(*log.borrow(), expected);
    for element in [&parent, &input] {
        element.remove_event("focus").unwrap();
        element.remove_event("blur").unwrap();
    }
}

#[test]
fn saved_focus_events_keep_related_nodes_weak_and_generation_checked() {
    let document = Document::new(320, 200).unwrap();
    let a = Element::create(&document, "input").unwrap();
    let b = Element::create(&document, "input").unwrap();
    b.set_id("original").unwrap();
    document.body().append_child(&a).unwrap();
    document.body().append_child(&b).unwrap();
    let saved = Rc::new(RefCell::new(Vec::new()));
    for event_type in ["blur", "focusout"] {
        let saved = saved.clone();
        a.on(event_type, move |event| {
            saved.borrow_mut().push(event.clone())
        })
        .unwrap();
    }
    a.focus().unwrap();
    b.focus().unwrap();
    assert_eq!(saved.borrow().len(), 2);
    for event in saved.borrow().iter() {
        assert_eq!(
            event
                .related_target()
                .unwrap()
                .get_attribute("id")
                .unwrap()
                .as_deref(),
            Some("original")
        );
        assert!(!event.cancelable());
        assert!(event.current_target().is_none());
        assert!(event.phase().is_none());
    }
    b.detach().unwrap();
    assert!(saved
        .borrow()
        .iter()
        .all(|event| event.related_target().is_some()));
    b.remove().unwrap();
    let replacement = Element::create(&document, "input").unwrap();
    replacement.set_id("replacement").unwrap();
    document.body().append_child(&replacement).unwrap();
    assert!(saved
        .borrow()
        .iter()
        .all(|event| event.related_target().is_none()));
    for event_type in ["blur", "focusout"] {
        a.remove_event(event_type).unwrap();
    }
    drop(replacement);
    drop(b);
    drop(a);
    drop(document);
    assert!(saved.borrow().iter().all(|event| event.target().is_none()));
}

#[test]
fn target_capture_stop_preserves_same_invocation_until_immediate_stop() {
    for immediate in [false, true] {
        let document = Document::new(320, 200).unwrap();
        let input = Element::create(&document, "input").unwrap();
        document.body().append_child(&input).unwrap();
        let log = Rc::new(RefCell::new(Vec::new()));
        let observed = log.clone();
        input
            .on("focus", move |_| observed.borrow_mut().push("noncapture"))
            .unwrap();
        let observed = log.clone();
        input
            .on_capture("focus", move |event| {
                observed.borrow_mut().push("capture-stop");
                if immediate {
                    event.stop_immediate_propagation();
                } else {
                    event.stop_propagation();
                }
            })
            .unwrap();
        let observed = log.clone();
        input
            .on_capture("focus", move |_| {
                observed.borrow_mut().push("later-capture")
            })
            .unwrap();
        input.focus().unwrap();
        assert_eq!(
            log.borrow().as_slice(),
            if immediate {
                &["capture-stop"][..]
            } else {
                &["capture-stop", "later-capture"][..]
            }
        );
        assert!(input.has_focus().unwrap());
        input.remove_event("focus").unwrap();
    }
}

#[test]
fn relabeling_a_saved_event_does_not_make_native_focus_cancelable() {
    let document = Document::new(320, 200).unwrap();
    let input = Element::create(&document, "input").unwrap();
    document.body().append_child(&input).unwrap();
    input
        .on("focus", |event| {
            let mut saved = event.clone();
            saved.event_type = "click".into();
            saved.prevent_default();
            assert!(!saved.cancelable());
            assert!(!saved.bubbles());
            assert!(!event.default_prevented());
        })
        .unwrap();
    input.focus().unwrap();
    assert!(input.has_focus().unwrap());
    input.remove_event("focus").unwrap();
}

#[test]
fn modal_focus_delivers_callbacks_cancels_composition_and_restores_focus() {
    let (document, [outside, inside, sibling], log) = fixture();
    let modal = Element::create(&document, "div").unwrap();
    document.body().append_child(&modal).unwrap();
    inside.detach().unwrap();
    modal.append_child(&inside).unwrap();
    outside.set_control_value("kept").unwrap();
    outside.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    document.set_modal_root(Some(&modal)).unwrap();
    assert!(inside.has_focus().unwrap());
    assert_eq!(outside.control_value().unwrap().as_deref(), Some("kept"));
    document.set_modal_root(None).unwrap();
    assert!(outside.has_focus().unwrap());
    document.set_modal_root(None).unwrap();
    assert!(outside.has_focus().unwrap());
    assert_eq!(
        *log.borrow(),
        rows(&[
            ("a", "focus", "a"),
            ("a", "blur", "body"),
            ("b", "focus", "b"),
            ("b", "blur", "body"),
            ("a", "focus", "a"),
        ])
    );
    for element in [outside, inside, sibling] {
        element.remove_event("focus").unwrap();
        element.remove_event("blur").unwrap();
    }
}

#[test]
fn modal_change_from_blur_callback_aborts_the_old_focus_request() {
    let (document, [outside, inside, sibling], log) = fixture();
    let modal = Element::create(&document, "div").unwrap();
    document.body().append_child(&modal).unwrap();
    inside.detach().unwrap();
    modal.append_child(&inside).unwrap();
    outside.focus().unwrap();
    let retained = document.clone();
    outside
        .on("blur", move |_| retained.set_modal_root(None).unwrap())
        .unwrap();
    document.set_modal_root(Some(&modal)).unwrap();
    assert!(outside.has_focus().unwrap());
    assert_eq!(
        *log.borrow(),
        rows(&[
            ("a", "focus", "a"),
            ("a", "blur", "body"),
            ("a", "focus", "a")
        ])
    );
    assert!(sibling.focus().is_ok()); // The callback also removed modal containment.
    for element in [outside, inside, sibling] {
        element.remove_event("focus").unwrap();
        element.remove_event("blur").unwrap();
    }
}
