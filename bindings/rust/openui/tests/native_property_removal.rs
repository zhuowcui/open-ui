//! Public native Rust mutation/removal uses the retained shared Engine source.
use openui::prelude::*;
use std::{cell::Cell, rc::Rc};

fn document() -> Document {
    Document::with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap()
}

#[test]
fn native_alias_defaults_and_callback_source_removal_survive_parent_mutation() {
    let document = document();
    let parent = document.body();
    parent.set_font_size(LengthValue::px(10.0)).unwrap();
    let item = Element::create(&document, "li").unwrap();
    parent.append_child(&item).unwrap();
    item.set_display(Display::None).unwrap();
    assert!(item.remove_property(StyleProperty::Display).unwrap());
    assert_eq!(item.computed_style().unwrap().display, Display::ListItem);
    let child = Element::create(&document, "div").unwrap();
    parent.append_child(&child).unwrap();
    child
        .set_property(
            StyleProperty::Padding,
            Edges::all(LengthValue::Em(2.0)).into(),
        )
        .unwrap();
    child
        .set_property(StyleProperty::PaddingLeft, LengthValue::px(3.0).into())
        .unwrap();
    let before = child.computed_style().unwrap();
    let weak = child.downgrade();
    let target = weak.clone();
    let parent_target = parent.downgrade();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    child
        .on("click", move |_| {
            let child = target.upgrade().unwrap();
            assert!(child.remove_property(StyleProperty::PaddingLeft).unwrap());
            assert!(!child.remove_property(StyleProperty::PaddingLeft).unwrap());
            parent_target
                .upgrade()
                .unwrap()
                .set_font_size(LengthValue::px(20.0))
                .unwrap();
            let style = child.computed_style().unwrap();
            assert_eq!(
                (style.padding_top.value(), style.padding_left.value()),
                (40.0, 0.0)
            );
            called.set(called.get() + 1);
        })
        .unwrap();
    child.click().unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        (before.padding_top.value(), before.padding_left.value()),
        (20.0, 3.0)
    );
    drop(child);
    drop(item);
    drop(parent);
    drop(document);
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_effect_removal_restores_constructor_source_and_owned_style_snapshots() {
    let document = document();
    let node = Element::create(&document, "div").unwrap();
    document.body().append_child(&node).unwrap();
    node.set_opacity(0.4).unwrap();
    let before = node.computed_style().unwrap();
    let id = node
        .animate(
            StyleProperty::Opacity,
            Keyframes::from_values(0.8_f32, 0.8_f32),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
        )
        .unwrap();
    assert!(node.remove_property(StyleProperty::Opacity).unwrap());
    assert_eq!(node.computed_style().unwrap().opacity, 0.8);
    document.cancel_animation(id).unwrap();
    assert_eq!(node.computed_style().unwrap().opacity, 1.0);
    assert_eq!(before.opacity, 0.4);
    assert!(!node.remove_property(StyleProperty::Opacity).unwrap());
    let clone = node.clone_subtree().unwrap();
    assert_eq!(clone.computed_style().unwrap().opacity, 1.0);
    node.remove().unwrap();
    assert!(node.remove_property(StyleProperty::Opacity).is_err());
    assert_eq!(clone.computed_style().unwrap().opacity, 1.0);
}
