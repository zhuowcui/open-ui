//! A native consuming application owns independent ellipse axes and carriers.
use openui::prelude::*;
use openui_style::{CompositeOperation, Display, Overflow};
use std::{cell::RefCell, rc::Rc};
#[test]
fn public_axis_setters_reflect_resolved_sources_and_keep_scene_ownership() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let document = Document::with_font_collection(
            ViewportMetrics::from_logical_size(160.0, 140.0, scale).unwrap(),
            FontCollection::deterministic_test(),
        )
        .unwrap();
        document.body().set_background_color(Color::WHITE).unwrap();
        let node = Element::create(&document, "div").unwrap();
        node.set_property(StyleProperty::Display, Display::Block.into())
            .unwrap();
        node.set_width(LengthValue::px(100.0)).unwrap();
        node.set_height(LengthValue::px(80.0)).unwrap();
        node.set_property(StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        node.set_border_top_left_radius_x(LengthValue::calc_percent_px(25.0, 5.0))
            .unwrap();
        node.set_border_top_left_radius_y(LengthValue::percent(25.0))
            .unwrap();
        node.set_border_top_right_radius_x(LengthValue::px(10.0))
            .unwrap();
        node.set_border_top_right_radius_y(LengthValue::Em(1.0))
            .unwrap();
        node.set_border_bottom_right_radius_x(LengthValue::px(12.0))
            .unwrap();
        node.set_border_bottom_right_radius_y(LengthValue::px(8.0))
            .unwrap();
        node.set_border_bottom_left_radius_x(LengthValue::percent(10.0))
            .unwrap();
        node.set_border_bottom_left_radius_y(LengthValue::px(6.0))
            .unwrap();
        document.body().append_child(&node).unwrap();
        let child = Element::create(&document, "div").unwrap();
        child.set_width(LengthValue::px(100.0)).unwrap();
        child.set_height(LengthValue::px(80.0)).unwrap();
        child.set_background_color(Color::GREEN).unwrap();
        node.append_child(&child).unwrap();
        assert_eq!(
            node.radius_axis_length(StyleProperty::BorderTopLeftRadiusX)
                .unwrap(),
            LengthValue::calc_percent_px(25.0, 5.0)
        );
        assert_eq!(
            node.radius_axis_length(StyleProperty::BorderTopRightRadiusY)
                .unwrap(),
            LengthValue::px(16.0)
        );
        let snapshot = node.computed_style().unwrap();
        let first = document.render_to_bitmap().unwrap();
        assert!(document
            .hit_test(1.0, 1.0)
            .unwrap()
            .as_ref()
            .is_none_or(|hit| !hit.is_same_node(&child).unwrap()));
        assert!(document
            .hit_test(50.0, 40.0)
            .unwrap()
            .unwrap()
            .is_same_node(&child)
            .unwrap());
        node.set_border_top_left_radius_x(LengthValue::px(30.0))
            .unwrap();
        node.set_border_top_left_radius_y(LengthValue::px(20.0))
            .unwrap();
        node.set_border_top_right_radius_y(LengthValue::px(16.0))
            .unwrap();
        node.set_border_bottom_left_radius_x(LengthValue::px(10.0))
            .unwrap();
        assert_eq!(
            first.pixels(),
            document.render_to_bitmap().unwrap().pixels(),
            "scale={scale}"
        );
        assert_eq!(
            snapshot.border_radius_lengths[0].unwrap().0,
            openui::typed_style::Length::calc_percent_px(25.0, 5.0)
        );
        node.set_border_top_right_radius_y(LengthValue::Em(1.0))
            .unwrap();
        document
            .body()
            .set_font_size(LengthValue::px(20.0))
            .unwrap();
        assert_eq!(
            node.radius_axis_length(StyleProperty::BorderTopRightRadiusY)
                .unwrap(),
            LengthValue::px(20.0)
        );
        assert!(node
            .remove_property(StyleProperty::BorderTopLeftRadiusX)
            .unwrap());
        assert_eq!(
            node.radius_axis_length(StyleProperty::BorderTopLeftRadiusX)
                .unwrap(),
            LengthValue::px(0.0)
        );
        assert_eq!(
            node.radius_axis_length(StyleProperty::BorderTopLeftRadiusY)
                .unwrap(),
            LengthValue::px(20.0)
        );
        assert!(!node
            .remove_property(StyleProperty::BorderTopLeftRadiusX)
            .unwrap());
        assert!(node.radius_axis_length(StyleProperty::Width).is_err());
        child.remove().unwrap();
        node.remove().unwrap();
        let weak = node.downgrade();
        drop(child);
        drop(node);
        drop(document);
        assert!(weak.upgrade().is_none());
        assert!(!first.pixels().is_empty());
    }
}
#[test]
fn public_axis_effect_cancel_callbacks_reenter_after_borrows_and_restore_sources() {
    let document = Document::new(160, 120).unwrap();
    let node = Element::create(&document, "div").unwrap();
    document.body().append_child(&node).unwrap();
    node.set_border_top_left_radius_x(LengthValue::percent(25.0))
        .unwrap();
    node.set_border_top_left_radius_y(LengthValue::px(8.0))
        .unwrap();
    let id = node
        .animate(
            StyleProperty::BorderTopLeftRadiusX,
            Keyframes::from_values(LengthValue::px(5.0), LengthValue::px(5.0)),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                composite: CompositeOperation::Add,
                ..AnimationOptions::default()
            },
        )
        .unwrap();
    assert_eq!(
        node.radius_axis_length(StyleProperty::BorderTopLeftRadiusX)
            .unwrap(),
        LengthValue::calc_percent_px(25.0, 5.0)
    );
    node.set_border_top_left_radius_x(LengthValue::percent(40.0))
        .unwrap();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let events = observed.clone();
    let native = node.clone();
    node.on("animationcancel", move |_| {
        events.borrow_mut().push(
            native
                .radius_axis_length(StyleProperty::BorderTopLeftRadiusX)
                .unwrap(),
        );
    })
    .unwrap();
    document.cancel_animation(id).unwrap();
    assert_eq!(&*observed.borrow(), &[LengthValue::percent(40.0)]);
    assert_eq!(
        node.radius_axis_length(StyleProperty::BorderTopLeftRadiusY)
            .unwrap(),
        LengthValue::px(8.0)
    );
    node.remove_event("animationcancel").unwrap();
    let weak = node.downgrade();
    node.remove().unwrap();
    drop(node);
    drop(document);
    assert!(weak.upgrade().is_none());
}
