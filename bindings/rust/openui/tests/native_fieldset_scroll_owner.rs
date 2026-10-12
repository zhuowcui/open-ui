//! Native application default input over the same retained anonymous scroll box.
use openui::prelude::*;
use std::{cell::Cell, rc::Rc};

fn box_(document: &Document, parent: &Element, tag: &str, width: f32, height: f32) -> Element {
    let node = Element::create(document, tag).unwrap();
    node.set_display(Display::Block).unwrap();
    node.set_width(LengthValue::px(width)).unwrap();
    node.set_height(LengthValue::px(height)).unwrap();
    node.set_margin(Edges::all(LengthValue::px(0.0))).unwrap();
    node.set_padding(Edges::all(LengthValue::px(0.0))).unwrap();
    node.set_border(Border {
        width: 0.0,
        style: BorderStyle::None,
        color: Color::BLACK,
    })
    .unwrap();
    node.set_min_width(LengthValue::px(0.0)).unwrap();
    node.set_min_height(LengthValue::px(0.0)).unwrap();
    parent.append_child(&node).unwrap();
    node
}

#[test]
fn wheel_default_action_excludes_legend_and_runs_after_native_callbacks_release_borrows() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let document = Document::with_viewport_metrics(
            ViewportMetrics::from_logical_size(500.0, 400.0, scale).unwrap(),
        )
        .unwrap();
        document
            .body()
            .set_margin(Edges::all(LengthValue::px(0.0)))
            .unwrap();
        let fieldset = box_(&document, &document.body(), "fieldset", 100.0, 80.0);
        fieldset
            .set_border(Border {
                width: 2.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            })
            .unwrap();
        fieldset.set_overflow_x(Overflow::Hidden).unwrap();
        fieldset.set_overflow_y(Overflow::Auto).unwrap();
        let legend = box_(&document, &fieldset, "legend", 30.0, 10.0);
        let content = box_(&document, &fieldset, "div", 80.0, 200.0);
        let title = legend.bounding_rect().unwrap().unwrap();
        document
            .dispatch_wheel_event(title.x + 5.0, title.y + 5.0, 0.0, 20.0, Modifiers::NONE)
            .unwrap();
        assert_eq!(fieldset.scroll_top().unwrap(), 0.0);
        let before = content.bounding_rect().unwrap().unwrap();
        let observed = Rc::new(Cell::new(0));
        let callbacks = observed.clone();
        let weak = fieldset.downgrade();
        content
            .on("wheel", move |event| {
                let owner = weak.upgrade().unwrap();
                owner.set_width(LengthValue::px(100.0)).unwrap();
                assert_eq!(owner.client_width().unwrap(), 100);
                callbacks.set(callbacks.get() + 1);
                event.prevent_default();
            })
            .unwrap();
        document
            .dispatch_wheel_event(before.x + 5.0, before.y + 5.0, 0.0, 20.0, Modifiers::NONE)
            .unwrap();
        assert_eq!(observed.get(), 1);
        assert_eq!(fieldset.scroll_top().unwrap(), 0.0);
        content.remove_event("wheel").unwrap();
        document
            .dispatch_wheel_event(before.x + 5.0, before.y + 5.0, 0.0, 20.0, Modifiers::NONE)
            .unwrap();
        assert_eq!(fieldset.scroll_top().unwrap(), 20.0);
        assert_eq!(legend.bounding_rect().unwrap().unwrap(), title);
        assert_eq!(content.bounding_rect().unwrap().unwrap().y, before.y - 20.0);
        let weak = fieldset.downgrade();
        drop(content);
        drop(legend);
        drop(fieldset);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
}
