//! Guards that use the consuming application's public raw event API.

use super::*;

fn mounted(document: &Document, name: &str) -> Element {
    let element = Element::create(document, name).unwrap();
    document.body().append_child(&element).unwrap();
    element
}

#[test]
fn native_boundary_policy_and_explicit_application_options_are_distinct() {
    for (name, bubbles) in [
        ("pointerenter", false),
        ("pointerleave", false),
        ("pointercancel", true),
        ("mouseenter", false),
        ("mouseleave", false),
        ("mousecancel", true),
    ] {
        let document = Document::new(100, 80).unwrap();
        let node = mounted(&document, "div");
        let log = Rc::new(RefCell::new(Vec::new()));
        let seen = log.clone();
        document
            .body()
            .on_capture(name, move |event| {
                seen.borrow_mut().push((1, event.phase()));
                event.prevent_default();
            })
            .unwrap();
        let seen = log.clone();
        node.on(name, move |event| {
            seen.borrow_mut().push((2, event.phase()));
            event.prevent_default();
        })
        .unwrap();
        let seen = log.clone();
        document
            .body()
            .on(name, move |event| {
                seen.borrow_mut().push((3, event.phase()))
            })
            .unwrap();
        let event = Event::native(name, 417);
        assert_eq!(event.bubbles(), bubbles);
        assert!(!event.cancelable());
        assert!(document.dispatch_event(&node, &event).unwrap());
        assert_eq!(
            *log.borrow(),
            if bubbles {
                vec![
                    (1, Some(EventPhase::Capture)),
                    (2, Some(EventPhase::Target)),
                    (3, Some(EventPhase::Bubble)),
                ]
            } else {
                vec![
                    (1, Some(EventPhase::Capture)),
                    (2, Some(EventPhase::Target)),
                ]
            }
        );
        assert!(!event.default_prevented());
        assert_eq!(event.timestamp_ns(), 417);
        assert_eq!(event.phase(), None);
        assert!(event.current_target().is_none());
        assert_eq!(event.target().unwrap().handle, node.handle);

        let explicit = Event::new(name, true, true);
        assert!(!document.dispatch_event(&node, &explicit).unwrap());
        assert!(explicit.default_prevented());
        document.body().remove_event(name).unwrap();
        node.remove_event(name).unwrap();
    }
}

#[test]
fn active_event_cannot_redispatch_but_distinct_nested_events_preserve_outer_metadata() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document, "div");
    let target = node.clone();
    let dispatcher = document.clone();
    let log = Rc::new(RefCell::new(Vec::new()));
    let seen = log.clone();
    node.on("wheel", move |event| {
        seen.borrow_mut().push(event.timestamp_ns());
        assert_eq!(event.phase(), Some(EventPhase::Target));
        assert!(matches!(
            dispatcher.dispatch_event(&target, event),
            Err(Error::ReentrantMutation)
        ));
        if event.timestamp_ns() == 1 {
            dispatcher
                .dispatch_event(&target, &Event::native("wheel", 2))
                .unwrap();
            assert_eq!(event.phase(), Some(EventPhase::Target));
            assert_eq!(event.current_target().unwrap().handle, target.handle);
        }
    })
    .unwrap();
    let event = Event::native("wheel", 1);
    document.dispatch_event(&node, &event).unwrap();
    assert_eq!(*log.borrow(), vec![1, 2]);
    assert_eq!(event.phase(), None);
    assert!(event.current_target().is_none());
    node.remove_event("wheel").unwrap();
}

#[test]
fn public_raw_dispatch_rejects_foreign_targets_and_invalid_payloads_before_callbacks() {
    let document = Document::new(100, 80).unwrap();
    let other = Document::new(100, 80).unwrap();
    let node = mounted(&document, "div");
    let foreign = mounted(&other, "div");
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    node.on("wheel", move |_| seen.set(seen.get() + 1)).unwrap();
    let mut event = Event::native("wheel", 7);
    assert!(matches!(
        document.dispatch_event(&foreign, &event),
        Err(Error::Engine(openui_engine::EngineError::WrongDocument))
    ));
    event.delta_x = f32::NAN;
    assert!(matches!(
        document.dispatch_event(&node, &event),
        Err(Error::InvalidArgument(_))
    ));
    assert_eq!(calls.get(), 0);
    assert!(event.target().is_none());
    node.remove_event("wheel").unwrap();
}

#[test]
fn redispatching_the_same_owned_event_clears_stop_flags_and_retains_cancellation() {
    for immediate in [false, true] {
        let document = Document::new(100, 80).unwrap();
        let node = mounted(&document, "div");
        let stopped = Rc::new(Cell::new(false));
        let first = stopped.clone();
        let log = Rc::new(RefCell::new(Vec::new()));
        let seen = log.clone();
        node.on("wheel", move |event| {
            seen.borrow_mut().push(1);
            if !first.replace(true) {
                event.prevent_default();
                if immediate {
                    event.stop_immediate_propagation();
                } else {
                    event.stop_propagation();
                }
            }
        })
        .unwrap();
        let seen = log.clone();
        node.on("wheel", move |_| seen.borrow_mut().push(2))
            .unwrap();
        let seen = log.clone();
        document
            .body()
            .on("wheel", move |_| seen.borrow_mut().push(3))
            .unwrap();
        let event = Event::native("wheel", 1);
        assert!(!document.dispatch_event(&node, &event).unwrap());
        assert_eq!(*log.borrow(), if immediate { vec![1] } else { vec![1, 2] });
        assert!(!event.propagation_stopped());
        assert!(!event.immediate_propagation_stopped());
        assert!(event.default_prevented());
        log.borrow_mut().clear();
        assert!(!document.dispatch_event(&node, &event).unwrap());
        assert_eq!(*log.borrow(), vec![1, 2, 3]);
        assert!(!event.propagation_stopped());
        assert!(!event.immediate_propagation_stopped());
        assert!(event.default_prevented());
        node.remove_event("wheel").unwrap();
        document.body().remove_event("wheel").unwrap();
    }
}

#[test]
fn public_raw_click_keeps_native_checkable_activation_and_cancellation() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document, "input");
    node.set_attribute("type", "checkbox").unwrap();
    assert!(document
        .dispatch_event(&node, &Event::new("click", true, true))
        .unwrap());
    assert!(node.is_checked().unwrap());
    node.on("click", |event| event.prevent_default()).unwrap();
    assert!(!document
        .dispatch_event(&node, &Event::new("click", true, true))
        .unwrap());
    assert!(node.is_checked().unwrap());
    // An explicitly noncancelable event cannot be canceled by a listener.
    assert!(document
        .dispatch_event(&node, &Event::new("click", true, false))
        .unwrap());
    assert!(!node.is_checked().unwrap());
    node.remove_event("click").unwrap();
}

#[test]
fn a_pre_stopped_application_event_resolves_target_skips_callbacks_and_can_be_reused() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document, "div");
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    node.on("wheel", move |_| seen.set(seen.get() + 1)).unwrap();
    let event = Event::new("wheel", true, true);
    event.stop_immediate_propagation();
    assert!(document.dispatch_event(&node, &event).unwrap());
    assert_eq!(calls.get(), 0);
    assert_eq!(event.target().unwrap().handle, node.handle);
    assert!(!event.propagation_stopped());
    assert!(!event.immediate_propagation_stopped());
    assert_eq!(event.phase(), None);
    assert!(document.dispatch_event(&node, &event).unwrap());
    assert_eq!(calls.get(), 1);
    node.remove_event("wheel").unwrap();
}
