use openui::prelude::*;
use openui::AnimationTimelineBinding;
use std::{cell::RefCell, rc::Rc};
fn box_(d: &Document, p: &Element, w: f32, h: f32) -> Element {
    let n = Element::create(d, "div").unwrap();
    n.set_width(LengthValue::px(w)).unwrap();
    n.set_height(LengthValue::px(h)).unwrap();
    p.append_child(&n).unwrap();
    n
}
fn fixture(scale: f64) -> (Document, Element, Element, Element) {
    let d = Document::with_font_collection(
        ViewportMetrics::from_logical_size(400.0, 400.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    let body = d.body();
    body.set_margin(Edges::all(LengthValue::px(0.0))).unwrap();
    body.set_background_color(Color::WHITE).unwrap();
    let a = box_(&d, &body, 100.0, 100.0);
    a.set_overflow(Overflow::Hidden).unwrap();
    box_(&d, &a, 100.0, 200.0);
    let b = box_(&d, &body, 100.0, 100.0);
    b.set_overflow(Overflow::Hidden).unwrap();
    box_(&d, &b, 100.0, 200.0);
    let t = box_(&d, &body, 20.0, 20.0);
    t.set_background_color(Color::BLUE).unwrap();
    a.scroll_to(0.0, 25.0).unwrap();
    b.scroll_to(0.0, 75.0).unwrap();
    (d, a, b, t)
}
fn options() -> AnimationOptions {
    AnimationOptions {
        duration_ms: 1000.0,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    }
}
#[test]
fn borrowed_native_sources_rebind_existing_effect_and_repeat_exact_pixels() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (d, a, b, t) = fixture(scale);
        let id = t
            .animate_on_scroll_range(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                options(),
                &a,
                TimelineAxis::Block,
                AnimationAttachmentRange::default(),
            )
            .unwrap();
        d.seek_animation_progress(id, 50.0).unwrap();
        let before = d.render_to_png_buffer().unwrap();
        d.set_animation_timeline_source(
            id,
            AnimationTimelineBinding::Scroll {
                source: &a,
                axis: TimelineAxis::Block,
            },
        )
        .unwrap();
        assert_eq!(d.render_to_png_buffer().unwrap(), before);
        d.set_animation_timeline_source(
            id,
            AnimationTimelineBinding::Scroll {
                source: &b,
                axis: TimelineAxis::Block,
            },
        )
        .unwrap();
        assert_eq!(
            d.animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .current_time_percentage,
            Some((1200.0 / 1_000_000.0) / ((100.0 * 16.0 / 1000.0) / 1000.0) * 100.0)
        );
        assert_eq!(t.computed_style().unwrap().opacity, 0.75);
        let after = d.render_to_png_buffer().unwrap();
        assert_ne!(after, before);
        assert_eq!(d.render_to_png_buffer().unwrap(), after);
        assert_eq!(d.animation_state(id).unwrap().id, id);
        d.pause_animation(id).unwrap();
        d.set_animation_timeline_source(id, AnimationTimelineBinding::Document)
            .unwrap();
        assert!(d.animation_timeline_snapshot(id).unwrap().is_none());
        assert!(d
            .animation_attachment_snapshot(id)
            .unwrap()
            .unwrap()
            .resolved
            .is_none());
        assert_eq!(
            d.animation_state(id).unwrap().current_time_ms,
            ((1200.0 / 1_000_000.0) / ((100.0 * 16.0 / 1000.0) / 1000.0)) * 1000.0
        );
        assert_eq!(t.computed_style().unwrap().opacity, 0.75);
    }
}
#[test]
fn callback_observes_published_source_and_can_reenter_without_stale_outer_write() {
    let (d, a, b, t) = fixture(1.0);
    let range = AnimationAttachmentRange {
        start: Some(
            TimelineRangeOffset::new(TimelineNamedRange::Scroll, LengthValue::px(50.0)).unwrap(),
        ),
        end: None,
    };
    let id = t
        .animate_on_scroll_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &a,
            TimelineAxis::Block,
            range,
        )
        .unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    let native = d.clone();
    let target = t.clone();
    t.on("animationstart", move |_| {
        observed
            .borrow_mut()
            .push(target.computed_style().unwrap().opacity);
        native.seek_animation_progress(id, 12.5).unwrap();
        native.pause_animation(id).unwrap();
    })
    .unwrap();
    d.set_animation_timeline_source(
        id,
        AnimationTimelineBinding::Scroll {
            source: &b,
            axis: TimelineAxis::Block,
        },
    )
    .unwrap();
    assert_eq!(&*calls.borrow(), &[0.5]);
    assert_eq!(
        d.animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .current_time_percentage,
        Some(
            ((1200.0 / 1_000_000.0)
                - ((1200.0 / 1_000_000.0) - (0.125 * ((100.0 * 16.0 / 1000.0) / 1000.0))))
                / ((100.0 * 16.0 / 1000.0) / 1000.0)
                * 100.0
        )
    );
    assert_eq!(t.computed_style().unwrap().opacity, 0.25);
    assert_eq!(d.animation_state(id).unwrap().play_state, PlayState::Paused);
    t.remove_event("animationstart").unwrap();
}
#[test]
fn borrowed_foreign_and_removed_sources_do_not_mutate_valid_binding() {
    let (d, a, b, t) = fixture(1.0);
    let (other, foreign, _, _) = fixture(1.0);
    let id = t
        .animate_on_scroll_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &a,
            TimelineAxis::Block,
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    let state = d.animation_state(id).unwrap();
    let pixels = d.render_to_png_buffer().unwrap();
    assert!(d
        .set_animation_timeline_source(
            id,
            AnimationTimelineBinding::Scroll {
                source: &foreign,
                axis: TimelineAxis::Block
            }
        )
        .is_err());
    assert_eq!(d.animation_state(id).unwrap(), state);
    assert_eq!(d.render_to_png_buffer().unwrap(), pixels);
    b.remove().unwrap();
    assert!(d
        .set_animation_timeline_source(
            id,
            AnimationTimelineBinding::Scroll {
                source: &b,
                axis: TimelineAxis::Block
            }
        )
        .is_err());
    assert_eq!(d.animation_state(id).unwrap(), state);
    drop(other);
}
