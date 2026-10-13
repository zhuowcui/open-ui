//! Internal borrow and work counters complement the public native API guards.
use super::Document;
use crate::prelude::*;
use openui_style::{Overflow, RendererStyleValue, ScrollbarWidth};
use std::{cell::Cell, rc::Rc};

fn fixture(scale: f64) -> (Document, Element) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    let port = Element::create(&document, "div").unwrap();
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
    let content = Element::create(&document, "div").unwrap();
    content.set_width(LengthValue::px(300.0)).unwrap();
    content.set_height(LengthValue::px(300.0)).unwrap();
    port.append_child(&content).unwrap();
    document.update_all().unwrap();
    assert!(!document.has_pending_events().unwrap());
    (document, port)
}

#[test]
fn scroll_callback_reads_and_unchanged_pumping_do_no_layout_or_paint_work() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, port) = fixture(scale);
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let weak = port.downgrade();
        port.on("scroll", move |_| {
            let port = weak.upgrade().unwrap();
            assert_eq!(
                (port.scroll_left().unwrap(), port.scroll_top().unwrap()),
                (40.0, 60.0)
            );
            observed.set(observed.get() + 1);
        })
        .unwrap();
        port.scroll_to(40.0, 60.0).unwrap();
        let pixels = document.render_to_png_buffer().unwrap();
        let stats = document.inner.engine.borrow().stats();
        document.dispatch_pending_events().unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(document.inner.engine.borrow().stats(), stats);
        document.dispatch_pending_events().unwrap();
        assert_eq!(document.inner.engine.borrow().stats(), stats);
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        assert!(!document.has_pending_events().unwrap());
        port.remove().unwrap();
    }
}

#[test]
fn outstanding_engine_borrow_rejects_pumping_before_consuming_scroll_tasks() {
    let (document, port) = fixture(1.0);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    port.on("scroll", move |_| observed.set(observed.get() + 1))
        .unwrap();
    port.scroll_to(0.0, 40.0).unwrap();
    let borrowed = document.inner.engine.borrow_mut();
    assert!(document.dispatch_pending_events().is_err());
    assert_eq!(calls.get(), 0);
    assert!(borrowed.has_pending_scroll_events());
    drop(borrowed);
    document.dispatch_pending_events().unwrap();
    assert_eq!(calls.get(), 1);
    assert!(!document.has_pending_events().unwrap());
    port.remove().unwrap();
}
