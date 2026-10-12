use openui::prelude::*;
use openui::AnimationId;
use std::{cell::RefCell, rc::Rc};

fn fixture(scale: f64, playback_rate: f64) -> (Document, Element, AnimationId) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(100.0, 80.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    document.body().set_background_color(Color::WHITE).unwrap();
    let node = Element::create(&document, "div").unwrap();
    node.set_width(LengthValue::px(40.0)).unwrap();
    node.set_height(LengthValue::px(30.0)).unwrap();
    node.set_background_color(Color::BLUE).unwrap();
    document.body().append_child(&node).unwrap();
    let id = node
        .animate(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            AnimationOptions {
                duration_ms: 100.0,
                playback_rate,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
        )
        .unwrap();
    (document, node, id)
}

// Pixel continuity is a public native application invariant, independently of
// Chromium qualification. No HTML/JS, test IDs, post-raster edits or fixture offsets.
#[test]
fn native_rate_changes_keep_running_and_paused_pixels_until_the_clock_advances() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, node, id) = fixture(scale, 1.0);
        document.set_animation_time(40.0).unwrap();
        let pixels = document.render_to_png_buffer().unwrap();
        document.set_animation_playback_rate(id, 2.0).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        assert_eq!(document.animation_state(id).unwrap().current_time_ms, 40.0);
        document.set_animation_time(50.0).unwrap();
        assert_eq!(node.computed_style().unwrap().opacity, 0.6);
        document.pause_animation(id).unwrap();
        document.seek_animation(id, 80.0).unwrap();
        let pixels = document.render_to_png_buffer().unwrap();
        document.set_animation_playback_rate(id, -1.0).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        document.play_animation(id).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
        document.set_animation_time(60.0).unwrap();
        assert_eq!(node.computed_style().unwrap().opacity, 0.7);
        assert_ne!(document.render_to_png_buffer().unwrap(), pixels);
    }
}

#[test]
fn native_zero_rate_and_invalid_controls_preserve_owned_state_and_picture() {
    let (document, node, id) = fixture(1.0, 1.0);
    document.seek_animation(id, 35.0).unwrap();
    document.set_animation_playback_rate(id, 0.0).unwrap();
    let owned_state = document.animation_state(id).unwrap();
    let pixels = document.render_to_png_buffer().unwrap();
    document.set_animation_time(1000.0).unwrap();
    assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
    assert_eq!(document.animation_state(id).unwrap().current_time_ms, 35.0);
    assert!(!document.is_animating().unwrap());
    assert!(document.finish_animation(id).is_err());
    for rate in [f64::NAN, f64::INFINITY] {
        assert!(document.set_animation_playback_rate(id, rate).is_err());
    }
    assert_eq!(document.render_to_png_buffer().unwrap(), pixels);
    document.set_animation_playback_rate(id, -2.0).unwrap();
    document.set_animation_time(1010.0).unwrap();
    assert_eq!(node.computed_style().unwrap().opacity, 0.15);
    assert_eq!(owned_state.current_time_ms, 35.0);
    document.cancel_animation(id).unwrap();
    assert!(document.animation_state(id).is_err());
}

#[test]
fn native_reverse_lifecycle_callbacks_can_read_the_same_document_after_controls() {
    let (document, node, id) = fixture(1.0, -1.0);
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let native = document.clone();
    node.on("animationend", move |_| {
        let state = native.animation_state(id).unwrap();
        observed
            .borrow_mut()
            .push((state.current_time_ms, state.finished));
    })
    .unwrap();
    document.set_animation_time(25.0).unwrap();
    assert_eq!(node.computed_style().unwrap().opacity, 0.75);
    document.pause_animation(id).unwrap();
    document.finish_animation(id).unwrap();
    assert_eq!(&*events.borrow(), &[(0.0, true)]);
    assert!(!document.is_animating().unwrap());
    document.set_animation_time(1000.0).unwrap();
    assert_eq!(events.borrow().len(), 1);
    document.play_animation(id).unwrap();
    assert_eq!(document.animation_state(id).unwrap().current_time_ms, 100.0);
    document.finish_animation(id).unwrap();
    assert_eq!(&*events.borrow(), &[(0.0, true), (0.0, true)]);
    // The callback intentionally owns a Document; removing it releases that
    // application-owned reference before document teardown.
    node.remove_event("animationend").unwrap();
    let weak = node.downgrade();
    drop(node);
    drop(document);
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_delta_clock_delivers_lifecycle_callbacks_once_after_engine_borrows() {
    let (document, node, id) = fixture(1.0, 1.0);
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let native = document.clone();
    node.on("animationend", move |_| {
        let state = native.animation_state(id).unwrap();
        observed.borrow_mut().push((
            native.get_time().unwrap(),
            state.current_time_ms,
            state.finished,
        ));
    })
    .unwrap();
    document.advance_time_by(100.0).unwrap();
    assert_eq!(&*events.borrow(), &[(100.0, 100.0, true)]);
    document.advance_time_by(100.0).unwrap();
    document.advance_time(300.0).unwrap();
    assert_eq!(events.borrow().len(), 1);
    node.remove_event("animationend").unwrap();
    let weak = node.downgrade();
    drop(node);
    drop(document);
    assert!(weak.upgrade().is_none());
}
