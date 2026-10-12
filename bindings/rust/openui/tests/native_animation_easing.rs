use openui::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn native_document(scale: f64) -> (Document, Element) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(200.0, 100.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    document.body().set_background_color(Color::WHITE).unwrap();
    let node = Element::create(&document, "div").unwrap();
    node.set_width(LengthValue::px(60.0)).unwrap();
    node.set_height(LengthValue::px(20.0)).unwrap();
    node.set_background_color(Color::BLUE).unwrap();
    document.body().append_child(&node).unwrap();
    (document, node)
}

// Native control continuity is separate from Chromium pixel qualification.
#[test]
fn native_multikeyframe_easing_preserves_pixels_during_paused_controls() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, node) = native_document(scale);
        let easing = Easing::linear_stops_percent(vec![
            PercentageLinearStop {
                input_percent: 0.0,
                output: 0.0,
            },
            PercentageLinearStop {
                input_percent: 50.0,
                output: 0.25,
            },
            PercentageLinearStop {
                input_percent: 100.0,
                output: 1.0,
            },
        ])
        .unwrap();
        let id = node
            .animate(
                StyleProperty::Width,
                Keyframes::new(vec![
                    Keyframe::new(0.0, LengthValue::px(0.0)),
                    Keyframe::new(0.5, LengthValue::px(20.0)),
                    Keyframe::new(1.0, LengthValue::px(100.0)),
                ])
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    easing,
                    ..AnimationOptions::default()
                },
            )
            .unwrap();
        document.advance_time(50.0).unwrap();
        assert_eq!(node.computed_style().unwrap().width.value(), 10.0);
        let pixels = document.render_to_png_buffer().unwrap();
        document.pause_animation(id).unwrap();
        document.set_animation_playback_rate(id, -2.0).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        document.seek_animation(id, 50.0).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        document.cancel_animation(id).unwrap();
        assert_eq!(node.computed_style().unwrap().width.value(), 60.0);
    }
}

#[test]
fn native_reverse_step_endpoint_is_visible_to_reentrant_lifecycle_callbacks() {
    let (document, node) = native_document(1.0);
    let id = node
        .animate(
            StyleProperty::Width,
            Keyframes::from_values(LengthValue::px(0.0), LengthValue::px(100.0)),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                direction: PlaybackDirection::Reverse,
                easing: Easing::steps(4, StepPosition::JumpStart).unwrap(),
                ..AnimationOptions::default()
            },
        )
        .unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let native = document.clone();
    let target = node.clone();
    node.on("animationend", move |_| {
        observed.borrow_mut().push((
            target.computed_style().unwrap().width.value(),
            native.animation_state(id).unwrap().finished,
        ));
    })
    .unwrap();
    document.advance_time_by(100.0).unwrap();
    assert_eq!(&*events.borrow(), &[(0.0, true)]);
    document.advance_time_by(100.0).unwrap();
    assert_eq!(events.borrow().len(), 1);
    node.remove_event("animationend").unwrap();
    let weak = node.downgrade();
    drop(node);
    drop(document);
    assert!(weak.upgrade().is_none());
}
