use openui::prelude::*;
use openui_engine::AccessibilityAction;
use openui_style::{Easing, Overflow, RendererStyleValue, ScrollbarWidth};
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};

fn scroller(document: &Document) -> (Element, Element) {
    let port = Element::create(document, "div").unwrap();
    port.set_width(LengthValue::px(100.0)).unwrap();
    port.set_height(LengthValue::px(100.0)).unwrap();
    port.set_property(StyleProperty::Overflow, Overflow::Auto.into())
        .unwrap();
    port.set_property(
        StyleProperty::ScrollbarWidth,
        StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
    )
    .unwrap();
    document.body().append_child(&port).unwrap();
    let content = Element::create(document, "div").unwrap();
    content.set_width(LengthValue::px(300.0)).unwrap();
    content.set_height(LengthValue::px(300.0)).unwrap();
    port.append_child(&content).unwrap();
    (port, content)
}

fn fixture(scale: f64) -> (Document, Element, Element) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    let (port, content) = scroller(&document);
    document.update_all().unwrap();
    assert!(!document.has_pending_events().unwrap());
    (document, port, content)
}

#[test]
fn scroll_notifications_coalesce_reenter_and_keep_native_propagation_and_idle_pixels() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, port, _content) = fixture(scale);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let observed = changes.clone();
        let weak = port.downgrade();
        port.on("scroll", move |event| {
            assert!(!event.bubbles());
            assert!(!event.cancelable());
            event.prevent_default();
            assert!(!event.default_prevented());
            let port = weak.upgrade().unwrap();
            observed
                .borrow_mut()
                .push((port.scroll_left().unwrap(), port.scroll_top().unwrap()));
        })
        .unwrap();
        let captures = Rc::new(Cell::new(0));
        let captured = captures.clone();
        document
            .body()
            .on_capture("scroll", move |_| captured.set(captured.get() + 1))
            .unwrap();
        let bubbles = Rc::new(Cell::new(0));
        let bubbled = bubbles.clone();
        document
            .body()
            .on("scroll", move |_| bubbled.set(bubbled.get() + 1))
            .unwrap();
        port.scroll_to(0.0, 40.0).unwrap();
        port.scroll_by(20.0, 10.0).unwrap();
        port.scroll_to(40.0, 60.0).unwrap();
        assert!(changes.borrow().is_empty());
        assert!(document.has_pending_events().unwrap());
        let pixels = document.render_to_png_buffer().unwrap();
        document.dispatch_pending_events().unwrap();
        assert_eq!(&*changes.borrow(), &[(40.0, 60.0)]);
        assert_eq!(captures.get(), 1);
        assert_eq!(bubbles.get(), 0);
        assert!(!document.has_pending_events().unwrap());
        document.dispatch_pending_events().unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        assert!(!Event::native("scroll", 0).bubbles());
        assert!(!Event::native("scroll", 0).cancelable());
        port.remove().unwrap();
    }
}

#[test]
fn callback_scrolling_waits_for_a_later_bounded_native_turn() {
    let (document, port, _content) = fixture(1.0);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let weak = port.downgrade();
    let native = document.clone();
    port.on("scroll", move |_| {
        observed.set(observed.get() + 1);
        if observed.get() < 3 {
            let port = weak.upgrade().unwrap();
            port.scroll_by(0.0, 40.0).unwrap();
            native.dispatch_pending_events().unwrap();
            assert!(native.has_pending_events().unwrap());
        }
    })
    .unwrap();
    port.scroll_to(0.0, 40.0).unwrap();
    for expected in 1..=3 {
        document.dispatch_pending_events().unwrap();
        assert_eq!(calls.get(), expected);
        assert_eq!(
            port.scroll_top().unwrap(),
            (expected + 1).min(3) as f64 * 40.0
        );
    }
    assert!(!document.has_pending_events().unwrap());
    port.remove().unwrap();
}

#[test]
fn clamping_noops_invalid_requests_and_removed_generations_preserve_scroll_notifications() {
    let (document, port, content) = fixture(1.0);
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let observed = offsets.clone();
    let weak = port.downgrade();
    port.on("scroll", move |_| {
        observed
            .borrow_mut()
            .push(weak.upgrade().unwrap().scroll_top().unwrap())
    })
    .unwrap();
    port.scroll_to(0.0, 300.0).unwrap();
    assert!(port.scroll_to(0.0, f64::NAN).is_err());
    document.dispatch_pending_events().unwrap();
    assert_eq!(&*offsets.borrow(), &[200.0]);
    port.scroll_to(0.0, 300.0).unwrap();
    document.dispatch_pending_events().unwrap();
    assert_eq!(&*offsets.borrow(), &[200.0]);
    content.set_height(LengthValue::px(50.0)).unwrap();
    document.update_all().unwrap();
    assert_eq!(port.scroll_top().unwrap(), 0.0);
    assert_eq!(&*offsets.borrow(), &[200.0]);
    document.dispatch_pending_events().unwrap();
    assert_eq!(&*offsets.borrow(), &[200.0, 0.0]);
    content.set_height(LengthValue::px(300.0)).unwrap();
    port.scroll_to(0.0, 40.0).unwrap();
    port.remove().unwrap();
    let (replacement, _content) = scroller(&document);
    let unexpected = Rc::new(Cell::new(0));
    let observed = unexpected.clone();
    replacement
        .on("scroll", move |_| observed.set(observed.get() + 1))
        .unwrap();
    document.dispatch_pending_events().unwrap();
    assert_eq!(unexpected.get(), 0);
    assert!(!document.has_pending_events().unwrap());
    replacement.remove().unwrap();
}

#[test]
fn accessibility_wheel_and_timed_scroll_share_native_task_delivery() {
    let (document, port, _content) = fixture(1.0);
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let observed = offsets.clone();
    let weak = port.downgrade();
    port.on("scroll", move |_| {
        observed
            .borrow_mut()
            .push(weak.upgrade().unwrap().scroll_top().unwrap())
    })
    .unwrap();
    port.perform_accessibility_action(AccessibilityAction::ScrollBy {
        delta_x: 0.0,
        delta_y: 40.0,
    })
    .unwrap();
    assert!(offsets.borrow().is_empty());
    document.dispatch_pending_events().unwrap();
    assert_eq!(&*offsets.borrow(), &[40.0]);
    document
        .dispatch_wheel_event(10.0, 10.0, 0.0, 10.0, Modifiers::NONE)
        .unwrap();
    document.dispatch_pending_events().unwrap();
    assert_eq!(&*offsets.borrow(), &[40.0, 50.0]);
    port.smooth_scroll_to(0.0, 150.0, 100.0, Easing::Linear)
        .unwrap();
    document.advance_time(50.0).unwrap();
    assert_eq!(port.scroll_top().unwrap(), 100.0);
    assert_eq!(offsets.borrow().len(), 2);
    document.dispatch_pending_events().unwrap();
    document.advance_time(100.0).unwrap();
    document.dispatch_pending_events().unwrap();
    assert_eq!(&*offsets.borrow(), &[40.0, 50.0, 100.0, 150.0]);
    port.remove().unwrap();
}

#[test]
fn callback_unwind_does_not_lose_or_repeat_scroll_tasks() {
    let (document, first, _content) = fixture(1.0);
    let (second, _content) = scroller(&document);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    first
        .on("scroll", move |_| {
            observed.set(observed.get() + 1);
            panic!("one delivered notification");
        })
        .unwrap();
    let second_calls = Rc::new(Cell::new(0));
    let observed = second_calls.clone();
    second
        .on("scroll", move |_| observed.set(observed.get() + 1))
        .unwrap();
    first.scroll_to(0.0, 40.0).unwrap();
    second.scroll_to(0.0, 40.0).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| document
        .dispatch_pending_events()
        .unwrap()))
    .is_err());
    assert_eq!(calls.get(), 1);
    assert_eq!(second_calls.get(), 0);
    assert!(document.has_pending_events().unwrap());
    document.dispatch_pending_events().unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(second_calls.get(), 1);
    assert!(!document.has_pending_events().unwrap());
    first.remove().unwrap();
    second.remove().unwrap();
}

#[test]
fn callback_changes_to_an_undelivered_target_keep_a_separate_later_scroll_batch() {
    for unwind in [false, true] {
        let (document, first, _content) = fixture(1.0);
        let (second, _content) = scroller(&document);
        let second_weak = second.downgrade();
        let first_calls = Rc::new(Cell::new(0));
        let observed = first_calls.clone();
        first
            .on("scroll", move |_| {
                observed.set(observed.get() + 1);
                second_weak.upgrade().unwrap().scroll_to(0.0, 80.0).unwrap();
                if unwind {
                    panic!("the captured second target must survive");
                }
            })
            .unwrap();
        let second_offsets = Rc::new(RefCell::new(Vec::new()));
        let observed = second_offsets.clone();
        let weak = second.downgrade();
        second
            .on("scroll", move |_| {
                observed
                    .borrow_mut()
                    .push(weak.upgrade().unwrap().scroll_top().unwrap())
            })
            .unwrap();
        first.scroll_to(0.0, 40.0).unwrap();
        second.scroll_to(0.0, 40.0).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            document.dispatch_pending_events().unwrap()
        }));
        assert_eq!(result.is_err(), unwind);
        assert_eq!(first_calls.get(), 1);
        if unwind {
            assert!(second_offsets.borrow().is_empty());
            document.dispatch_pending_events().unwrap();
        }
        assert_eq!(&*second_offsets.borrow(), &[80.0]);
        assert!(document.has_pending_events().unwrap());
        document.dispatch_pending_events().unwrap();
        assert_eq!(&*second_offsets.borrow(), &[80.0, 80.0]);
        assert_eq!(first_calls.get(), 1);
        assert!(!document.has_pending_events().unwrap());
        first.remove().unwrap();
        second.remove().unwrap();
    }
}

#[test]
fn target_aware_accessibility_pages_use_the_native_queue_and_release_callback_borrows() {
    use openui::AccessibilityScrollDirection;
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, port, _content) = fixture(scale);
        let offsets = Rc::new(RefCell::new(Vec::new()));
        let observed = offsets.clone();
        let weak = port.downgrade();
        port.on("scroll", move |event| {
            assert!(!event.bubbles());
            let port = weak.upgrade().unwrap();
            let offset = (port.scroll_left().unwrap(), port.scroll_top().unwrap());
            observed.borrow_mut().push(offset);
            // Geometry reads and a second AX action borrow the same retained
            // Engine after callback dispatch has released its original borrows.
            assert_eq!(port.client_width().unwrap(), 100);
            if offset == (0.0, 80.0) {
                port.perform_accessibility_action(AccessibilityAction::ScrollPage {
                    direction: AccessibilityScrollDirection::Right,
                })
                .unwrap();
            }
        })
        .unwrap();
        port.perform_accessibility_action(AccessibilityAction::ScrollPage {
            direction: AccessibilityScrollDirection::Down,
        })
        .unwrap();
        assert_eq!(
            (port.scroll_left().unwrap(), port.scroll_top().unwrap()),
            (0.0, 80.0)
        );
        assert!(offsets.borrow().is_empty());
        document.dispatch_pending_events().unwrap();
        assert_eq!(&*offsets.borrow(), &[(0.0, 80.0)]);
        assert!(document.has_pending_events().unwrap());
        document.dispatch_pending_events().unwrap();
        assert_eq!(&*offsets.borrow(), &[(0.0, 80.0), (80.0, 80.0)]);
        port.scroll_to(200.0, 200.0).unwrap();
        document.dispatch_pending_events().unwrap();
        let before = document.render_to_png_buffer().unwrap();
        let calls = offsets.borrow().len();
        for direction in [
            AccessibilityScrollDirection::Down,
            AccessibilityScrollDirection::Right,
        ] {
            port.perform_accessibility_action(AccessibilityAction::ScrollPage { direction })
                .unwrap();
        }
        assert!(!document.has_pending_events().unwrap());
        document.dispatch_pending_events().unwrap();
        assert_eq!(offsets.borrow().len(), calls);
        assert_eq!(document.render_to_png_buffer().unwrap(), before);
        port.remove().unwrap();
    }
}
