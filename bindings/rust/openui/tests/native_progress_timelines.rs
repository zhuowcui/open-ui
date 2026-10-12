use openui::prelude::*;
use openui::typed_style::{Display, Overflow};
use openui::{AnimationId, AnimationTimelineSnapshot};
use std::{cell::RefCell, rc::Rc};

fn fixture(scale: f64) -> (Document, Element, Element, AnimationId) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(200.0, 160.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    document.body().set_background_color(Color::WHITE).unwrap();
    let source = Element::create(&document, "div").unwrap();
    source.set_display(Display::Block).unwrap();
    source.set_width(LengthValue::px(100.0)).unwrap();
    source.set_height(LengthValue::px(80.0)).unwrap();
    source.set_overflow(Overflow::Hidden).unwrap();
    let content = Element::create(&document, "div").unwrap();
    content.set_height(LengthValue::px(180.0)).unwrap();
    content.set_width(LengthValue::px(100.0)).unwrap();
    source.append_child(&content).unwrap();
    document.body().append_child(&source).unwrap();
    let target = Element::create(&document, "div").unwrap();
    target.set_width(LengthValue::px(40.0)).unwrap();
    target.set_height(LengthValue::px(30.0)).unwrap();
    target.set_background_color(Color::BLUE).unwrap();
    document.body().append_child(&target).unwrap();
    let id = target
        .animate_on_scroll(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
            &source,
            TimelineAxis::Block,
            TimelineRange::new(0.0, 100.0).unwrap(),
        )
        .unwrap();
    (document, source, target, id)
}
fn snapshot(document: &Document, id: AnimationId) -> AnimationTimelineSnapshot {
    document.animation_timeline_snapshot(id).unwrap().unwrap()
}

#[test]
fn native_scroll_changes_picture_and_delivers_lifecycle_without_document_clock_ticks() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, source, target, id) = fixture(scale);
        let start = document.render_to_png_buffer().unwrap();
        source.scroll_to(0.0, 50.0).unwrap();
        assert_eq!(target.computed_style().unwrap().opacity, 0.5);
        assert_eq!(document.get_time().unwrap(), 0.0);
        assert!(!document.is_animating().unwrap());
        assert_ne!(document.render_to_png_buffer().unwrap(), start);
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let native = document.clone();
        target
            .on("animationend", move |_| {
                let state = snapshot(&native, id);
                observed
                    .borrow_mut()
                    .push((native.get_time().unwrap(), state.current_time_percentage));
            })
            .unwrap();
        source.scroll_to(0.0, 100.0).unwrap();
        // An auto-aligned physical endpoint remains active. Explicit native
        // seeking disables that context and reaches the exclusive effect end.
        assert!(events.borrow().is_empty());
        assert_eq!(
            document.animation_state(id).unwrap().phase,
            AnimationPhase::Active
        );
        document.seek_animation_progress(id, 100.0).unwrap();
        assert_eq!(&*events.borrow(), &[(0.0, Some(100.0))]);
        source.scroll_to(0.0, 100.0).unwrap();
        document.update_all().unwrap();
        assert_eq!(events.borrow().len(), 1);
        target.remove_event("animationend").unwrap();
        let weak = target.downgrade();
        drop(target);
        drop(source);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn native_percentage_controls_and_owned_snapshots_expose_progress_time_explicitly() {
    let (document, source, target, id) = fixture(1.0);
    source.scroll_to(0.0, 25.0).unwrap();
    let owned = snapshot(&document, id);
    assert_eq!(owned.current_time_ms, Some(0.4));
    assert_eq!(owned.current_time_percentage, Some(25.0));
    document.set_animation_playback_rate(id, 2.0).unwrap();
    assert_eq!(target.computed_style().unwrap().opacity, 0.5);
    document.seek_animation_progress(id, 80.0).unwrap();
    assert_eq!(target.computed_style().unwrap().opacity, 0.8);
    document.pause_animation(id).unwrap();
    let picture = document.render_to_png_buffer().unwrap();
    source.scroll_to(0.0, 75.0).unwrap();
    assert_eq!(document.render_to_png_buffer().unwrap(), picture);
    assert_eq!(owned.current_time_percentage, Some(25.0));
    for invalid in [f64::NAN, f64::INFINITY] {
        assert!(document.seek_animation_progress(id, invalid).is_err());
        assert_eq!(document.render_to_png_buffer().unwrap(), picture);
    }
    document.play_animation(id).unwrap();
    assert_eq!(target.computed_style().unwrap().opacity, 1.0);
}

#[test]
fn native_inactive_source_reports_unresolved_time_and_preserves_the_live_application() {
    let (document, source, target, id) = fixture(1.0);
    source.set_display(Display::None).unwrap();
    document.update_all().unwrap();
    let owned = snapshot(&document, id);
    assert!(!owned.active);
    assert_eq!(owned.current_time_ms, None);
    document.seek_animation_progress(id, 35.0).unwrap();
    source.set_display(Display::Block).unwrap();
    document.update_all().unwrap();
    assert_eq!(snapshot(&document, id).current_time_percentage, Some(35.0));
    assert_eq!(
        snapshot(&document, id).current_time_ms,
        Some(((35.0_f64 / 100.0) * ((100.0 * 16.0 / 1000.0) / 1000.0)) * 1000.0)
    );
    assert_eq!(target.computed_style().unwrap().opacity, 0.35);
    assert!(!owned.active);
}

#[test]
fn native_fill_none_keeps_physical_end_value_and_explicit_seek_restores_underlying() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, source, target, previous) = fixture(scale);
        document.cancel_animation(previous).unwrap();
        target.set_opacity(0.25).unwrap();
        let underlying = document.render_to_png_buffer().unwrap();
        let id = target
            .animate_on_scroll(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::None,
                    ..AnimationOptions::default()
                },
                &source,
                TimelineAxis::Block,
                TimelineRange::new(0.0, 100.0).unwrap(),
            )
            .unwrap();
        source.scroll_to(0.0, 100.0).unwrap();
        assert_eq!(
            document.animation_state(id).unwrap().phase,
            AnimationPhase::Active
        );
        assert_eq!(target.computed_style().unwrap().opacity, 1.0);
        let endpoint = document.render_to_png_buffer().unwrap();
        assert_ne!(endpoint, underlying);
        document.seek_animation_progress(id, 100.0).unwrap();
        assert_eq!(
            document.animation_state(id).unwrap().phase,
            AnimationPhase::After
        );
        assert_eq!(target.computed_style().unwrap().opacity, 0.25);
        assert_eq!(document.render_to_png_buffer().unwrap(), underlying);
        document.cancel_animation(id).unwrap();
        // An explicitly zero-duration effect has no active interval at the
        // physical endpoint and keeps the unfilled underlying publication.
        let zero = target
            .animate_on_scroll(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                AnimationOptions {
                    duration_ms: 0.0,
                    fill: FillMode::None,
                    ..AnimationOptions::default()
                },
                &source,
                TimelineAxis::Block,
                TimelineRange::new(0.0, 100.0).unwrap(),
            )
            .unwrap();
        assert_eq!(
            document.animation_state(zero).unwrap().phase,
            AnimationPhase::After
        );
        assert_eq!(target.computed_style().unwrap().opacity, 0.25);
        assert_eq!(document.render_to_png_buffer().unwrap(), underlying);
    }
}

#[test]
fn native_static_sticky_consumers_use_one_stage_for_block_flex_and_grid() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for display in [Display::Block, Display::Flex, Display::Grid] {
            let document = Document::with_font_collection(
                ViewportMetrics::from_logical_size(200.0, 200.0, scale).unwrap(),
                FontCollection::deterministic_test(),
            )
            .unwrap();
            document.body().set_background_color(Color::WHITE).unwrap();
            let source = Element::create(&document, "div").unwrap();
            source.set_display(display).unwrap();
            source.set_flex_direction(FlexDirection::Column).unwrap();
            source.set_width(LengthValue::px(100.0)).unwrap();
            source.set_height(LengthValue::px(80.0)).unwrap();
            source.set_overflow(Overflow::Hidden).unwrap();
            document.body().append_child(&source).unwrap();
            let content = Element::create(&document, "div").unwrap();
            content.set_width(LengthValue::px(100.0)).unwrap();
            content.set_height(LengthValue::px(300.0)).unwrap();
            content.set_flex_shrink(0.0).unwrap();
            source.append_child(&content).unwrap();
            let spacer = Element::create(&document, "div").unwrap();
            spacer.set_height(LengthValue::px(120.0)).unwrap();
            content.append_child(&spacer).unwrap();
            let subject = Element::create(&document, "div").unwrap();
            subject.set_width(LengthValue::px(30.0)).unwrap();
            subject.set_height(LengthValue::px(20.0)).unwrap();
            subject.set_position(Position::Sticky).unwrap();
            subject.set_top(Length::px(0.0)).unwrap();
            subject.set_background_color(Color::BLUE).unwrap();
            subject.set_attribute("id", "sticky-native").unwrap();
            content.append_child(&subject).unwrap();
            let normal = document.render_to_png_buffer().unwrap();
            source.scroll_to(0.0, 150.0).unwrap();
            assert_eq!(subject.bounding_rect().unwrap().unwrap().y, 0.0);
            assert_eq!(
                document
                    .hit_test(5.0, 5.0)
                    .unwrap()
                    .unwrap()
                    .get_attribute("id")
                    .unwrap()
                    .as_deref(),
                Some("sticky-native")
            );
            let stuck = document.render_to_png_buffer().unwrap();
            assert_ne!(stuck, normal);
            for _ in 0..3 {
                document.update_all().unwrap();
                assert_eq!(document.render_to_png_buffer().unwrap(), stuck);
                assert_eq!(subject.bounding_rect().unwrap().unwrap().y, 0.0);
            }
            source.scroll_to(0.0, 0.0).unwrap();
            assert_eq!(subject.bounding_rect().unwrap().unwrap().y, 120.0);
            assert_eq!(document.render_to_png_buffer().unwrap(), normal);
            assert!(!document.is_animating().unwrap());
        }
    }
}

#[test]
fn native_fractional_document_boundaries_deliver_events_after_snapped_picture_updates() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, _, target, previous) = fixture(scale);
        document.cancel_animation(previous).unwrap();
        target.set_opacity(0.25).unwrap();
        let underlying = document.render_to_png_buffer().unwrap();
        let id = target
            .animate(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                AnimationOptions {
                    delay_ms: 1.0,
                    duration_ms: 2.0,
                    fill: FillMode::None,
                    play_state: PlayState::Paused,
                    ..AnimationOptions::default()
                },
            )
            .unwrap();
        let observed = Rc::new(RefCell::new(Vec::new()));
        for name in ["animationstart", "animationend"] {
            let native = document.clone();
            let retained = target.clone();
            let observed = observed.clone();
            target
                .on(name, move |_| {
                    observed.borrow_mut().push((
                        name,
                        native.animation_state(id).unwrap().phase,
                        retained.computed_style().unwrap().opacity,
                    ));
                })
                .unwrap();
        }
        document.seek_animation(id, 0.998999).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), underlying);
        assert!(observed.borrow().is_empty());
        document.seek_animation(id, 0.999001).unwrap();
        assert_eq!(target.computed_style().unwrap().opacity, 0.0);
        assert_ne!(document.render_to_png_buffer().unwrap(), underlying);
        assert_eq!(
            &*observed.borrow(),
            &[("animationstart", AnimationPhase::Active, 0.0)]
        );
        document.seek_animation(id, 2.999001).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), underlying);
        assert_eq!(
            &*observed.borrow(),
            &[
                ("animationstart", AnimationPhase::Active, 0.0),
                ("animationend", AnimationPhase::After, 0.25)
            ]
        );
        document.seek_animation(id, 3.000999).unwrap();
        assert_eq!(observed.borrow().len(), 2);
        target.remove_event("animationstart").unwrap();
        target.remove_event("animationend").unwrap();
    }
}
