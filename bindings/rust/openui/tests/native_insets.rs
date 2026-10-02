use openui::prelude::*;
use openui::typed_style::parse_literal;
use std::{cell::Cell, rc::Rc};

#[test]
fn public_native_insets_survive_callbacks_resize_and_document_teardown() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = Document::with_viewport_metrics(
            ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        )
        .unwrap();
        let root = document.body();
        root.set_overflow(Overflow::Hidden).unwrap();
        let child = Element::create(&document, "div").unwrap();
        child.set_position(Position::Absolute).unwrap();
        child.set_display(Display::Block).unwrap();
        child.set_width(LengthValue::px(80.0)).unwrap();
        child.set_height(LengthValue::px(40.0)).unwrap();
        child.set_font_size(LengthValue::px(20.0)).unwrap();
        root.append_child(&child).unwrap();
        for (property, literal) in [(StyleProperty::Left, "25%"), (StyleProperty::Top, "10%")] {
            child
                .set_property(property, parse_literal(property, literal).unwrap())
                .unwrap();
        }
        let original = child.bounding_rect().unwrap().unwrap();
        assert_eq!((original.x, original.y), (80.0, 24.0));
        assert_eq!((original.width, original.height), (80.0, 40.0));

        let calls = Rc::new(Cell::new(0));
        let callback_calls = calls.clone();
        let target = child.downgrade();
        child
            .on("click", move |_| {
                let child = target.upgrade().unwrap();
                for (property, literal) in
                    [(StyleProperty::Left, "2em"), (StyleProperty::Top, "1.5rem")]
                {
                    child
                        .set_property(property, parse_literal(property, literal).unwrap())
                        .unwrap();
                }
                callback_calls.set(callback_calls.get() + 1);
            })
            .unwrap();
        child.click().unwrap();
        assert_eq!(calls.get(), 1);
        let after_callback = child.bounding_rect().unwrap().unwrap();
        assert_eq!((after_callback.x, after_callback.y), (40.0, 24.0));

        for (property, literal) in [
            (StyleProperty::Left, "auto"),
            (StyleProperty::Top, "auto"),
            (StyleProperty::Right, "20%"),
            (StyleProperty::Bottom, "25%"),
        ] {
            child
                .set_property(property, parse_literal(property, literal).unwrap())
                .unwrap();
        }
        let before_resize = child.bounding_rect().unwrap().unwrap();
        assert_eq!((before_resize.x, before_resize.y), (176.0, 140.0));
        document
            .set_viewport(ViewportMetrics::from_logical_size(400.0, 320.0, scale).unwrap())
            .unwrap();
        let after_resize = child.bounding_rect().unwrap().unwrap();
        assert_eq!((after_resize.x, after_resize.y), (240.0, 200.0));
        assert_eq!((original.x, original.y), (80.0, 24.0));
        let weak = child.downgrade();
        drop(child);
        drop(root);
        drop(document);
        assert!(weak.upgrade().is_none());
        assert_eq!((original.width, original.height), (80.0, 40.0));
    }
}
