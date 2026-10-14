//! Shared native clock/source controls; exact expectations use pinned .50 order.
use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, AnimationTimelineSource, Engine, NodeHandle};
use openui_geometry::ViewportMetrics;
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, FillMode, IterationCount, Keyframes, LengthValue,
    Overflow, PlayState, PropertyKeyframes, StyleProperty, StyleValue, TimelineAxis,
    TimelineNamedRange, TimelineRange, TimelineRangeOffset, ViewTimelineInset,
};

fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: impl Into<StyleValue>) {
    e.set_property(n, p, v.into()).unwrap();
}
fn node(e: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    set(e, n, StyleProperty::Width, LengthValue::px(width));
    set(e, n, StyleProperty::Height, LengthValue::px(height));
    e.append_child(parent, n).unwrap();
    n
}
fn fixture(scale: f64) -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(400.0, 400.0, scale).unwrap()).unwrap();
    let root = e.root();
    let a = node(&mut e, root, 100.0, 100.0);
    set(&mut e, a, StyleProperty::Overflow, Overflow::Hidden);
    node(&mut e, a, 100.0, 200.0);
    let b = node(&mut e, root, 100.0, 100.0);
    set(&mut e, b, StyleProperty::Overflow, Overflow::Hidden);
    node(&mut e, b, 100.0, 200.0);
    let target = node(&mut e, root, 10.0, 10.0);
    e.update().unwrap();
    e.scroll_to(a, 0.0, 25.0).unwrap();
    e.scroll_to(b, 0.0, 75.0).unwrap();
    (e, a, b, target)
}
fn source(n: NodeHandle) -> AnimationTimelineSource {
    AnimationTimelineSource::Scroll {
        source: n,
        axis: TimelineAxis::Block,
    }
}
fn options() -> AnimationOptions {
    AnimationOptions {
        duration_ms: 1000.0,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    }
}
fn frames() -> PropertyKeyframes {
    PropertyKeyframes::typed(
        StyleProperty::Opacity,
        Keyframes::from_values(0.0_f32, 1.0_f32),
    )
    .unwrap()
}
fn progress(e: &mut Engine, id: AnimationId) -> f64 {
    e.animation_timeline_snapshot(id)
        .unwrap()
        .unwrap()
        .current_time_percentage
        .unwrap()
}
// Pinned cover time truncates to integer microseconds before double seconds.
// Duration is calculated through the separate millisecond division boundary.
fn fixture_duration() -> f64 {
    (100.0 * 16.0 / 1000.0) / 1000.0
}
fn fixture_clock(offset: f64) -> f64 {
    ((offset * 16.0) as i64) as f64 / 1_000_000.0
}
fn forward_percentage() -> f64 {
    fixture_clock(75.0) / fixture_duration() * 100.0
}
fn reverse_percentage() -> f64 {
    (fixture_clock(75.0) - fixture_duration()) * -1.0 / fixture_duration() * 100.0
}
fn opacity(e: &Engine, n: NodeHandle) -> f32 {
    e.computed_style(n).unwrap().opacity
}

#[test]
fn equal_binding_preserves_manual_seek_while_a_new_source_autoaligns_at_five_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut e, a, b, target) = fixture(scale);
        let id = e
            .animate_with_attachment_range(
                target,
                frames(),
                options(),
                source(a),
                AnimationAttachmentRange::default(),
            )
            .unwrap();
        e.seek_animation_progress(id, 50.0).unwrap();
        e.update().unwrap();
        e.drain_animation_events();
        let state = e.animation_state(id).unwrap();
        let generations = e.dirty_generations();
        let stats = e.stats();
        e.set_animation_timeline_source(id, source(a)).unwrap();
        assert_eq!(e.animation_state(id).unwrap(), state);
        assert_eq!(e.dirty_generations(), generations);
        assert_eq!(e.stats(), stats);
        assert!(e.drain_animation_events().is_empty());
        e.set_animation_timeline_source(id, source(b)).unwrap();
        assert_eq!(progress(&mut e, id), forward_percentage());
        assert_eq!(opacity(&e, target), 0.75);
        assert_eq!(
            e.animation_timeline_snapshot(id).unwrap().unwrap().source,
            Some(b)
        );
        assert_eq!(e.animation_state(id).unwrap().id, id);
        assert_eq!(e.animation_state(id).unwrap().target, target);
    }
}

#[test]
fn paused_proportional_hold_survives_document_progress_and_document_roundtrip() {
    let (mut e, a, b, target) = fixture(1.0);
    let mut o = options();
    o.play_state = PlayState::Paused;
    let id = e
        .animate(target, frames(), o, AnimationTimeline::Document)
        .unwrap();
    e.seek_animation(id, 250.0).unwrap();
    e.set_animation_timeline_source(id, source(a)).unwrap();
    assert_eq!(progress(&mut e, id), 25.0);
    assert_eq!(e.animation_state(id).unwrap().play_state, PlayState::Paused);
    assert_eq!(opacity(&e, target), 0.25);
    e.set_animation_timeline_source(id, source(b)).unwrap();
    assert_eq!(progress(&mut e, id), 25.0);
    e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
        .unwrap();
    assert_eq!(e.animation_state(id).unwrap().current_time_ms, 250.0);
    assert_eq!(e.animation_state(id).unwrap().play_state, PlayState::Paused);
    assert_eq!(opacity(&e, target), 0.25);
    assert!(e.animation_timeline_snapshot(id).unwrap().is_none());
}

#[test]
fn finished_start_and_hold_are_distinct_from_a_paused_hold_on_document_reassignment() {
    let (mut e, a, _, target) = fixture(1.0);
    e.set_animation_time(250.0).unwrap();
    e.scroll_to(a, 0.0, 100.0).unwrap();
    let id = e
        .animate_with_attachment_range(
            target,
            frames(),
            options(),
            source(a),
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    assert!(e.animation_state(id).unwrap().finished);
    e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
        .unwrap();
    // setTimeline scales the finished hold, then clears it because its original
    // progress start0 is resolved. The document now is0.25s, not the scaled1s.
    assert_eq!(e.animation_state(id).unwrap().current_time_ms, 250.0);
    assert!(!e.animation_state(id).unwrap().finished);
    assert_eq!(opacity(&e, target), 0.25);
    e.pause_animation(id).unwrap();
    e.seek_animation(id, 1000.0).unwrap();
    e.set_animation_timeline_source(id, source(a)).unwrap();
    e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
        .unwrap();
    assert_eq!(e.animation_state(id).unwrap().current_time_ms, 1000.0);
    assert_eq!(e.animation_state(id).unwrap().play_state, PlayState::Paused);
    assert_eq!(opacity(&e, target), 1.0);
}

#[test]
fn raw_attachment_and_reversed_native_ranges_survive_clock_type_changes() {
    let (mut e, a, b, target) = fixture(1.0);
    let range = AnimationAttachmentRange {
        start: Some(
            TimelineRangeOffset::new(TimelineNamedRange::Scroll, LengthValue::Em(1.0)).unwrap(),
        ),
        end: None,
    };
    let id = e
        .animate_with_attachment_range(target, frames(), options(), source(a), range)
        .unwrap();
    e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
        .unwrap();
    assert_eq!(
        e.animation_attachment_snapshot(id).unwrap().unwrap().range,
        range
    );
    assert!(e
        .animation_attachment_snapshot(id)
        .unwrap()
        .unwrap()
        .resolved
        .is_none());
    e.set_animation_timeline_source(id, source(b)).unwrap();
    assert_eq!(
        e.animation_attachment_snapshot(id).unwrap().unwrap().range,
        range
    );
    e.cancel_animation(id).unwrap();
    let legacy = e
        .animate(
            target,
            frames(),
            options(),
            AnimationTimeline::Scroll {
                source: a,
                axis: TimelineAxis::Block,
                range: TimelineRange::new(100.0, 0.0).unwrap(),
            },
        )
        .unwrap();
    e.set_animation_timeline_source(legacy, AnimationTimelineSource::Document)
        .unwrap();
    e.set_animation_timeline_source(legacy, source(b)).unwrap();
    let snapshot = e.animation_timeline_snapshot(legacy).unwrap().unwrap();
    assert_eq!((snapshot.range_start, snapshot.range_end), (100.0, 0.0));
    assert_eq!(snapshot.current_time_percentage, Some(reverse_percentage()));
    assert!(e.animation_attachment_snapshot(legacy).unwrap().is_none());
}

#[test]
fn inactive_browser_hold_uses_fallback_duration_then_rescales_on_activation() {
    let (mut e, a, b, target) = fixture(1.0);
    set(
        &mut e,
        b,
        StyleProperty::Display,
        openui_style::Display::None,
    );
    let mut o = options();
    o.play_state = PlayState::Paused;
    let id = e
        .animate(target, frames(), o, AnimationTimeline::Document)
        .unwrap();
    e.seek_animation(id, 250.0).unwrap();
    e.set_animation_timeline_source(id, source(b)).unwrap();
    assert!(!e.animation_timeline_snapshot(id).unwrap().unwrap().active);
    assert_eq!(
        e.animation_state(id).unwrap().current_time_ms,
        (0.25 * 100.0) * 1000.0
    );
    set(
        &mut e,
        b,
        StyleProperty::Display,
        openui_style::Display::Block,
    );
    e.update().unwrap();
    assert_eq!(progress(&mut e, id), 25.0);
    assert_eq!(opacity(&e, target), 0.25);
    e.set_animation_timeline_source(id, source(a)).unwrap();
    assert_eq!(progress(&mut e, id), 25.0);
}

#[test]
fn deferred_percentage_equal_binding_and_changed_binding_preserve_optional_control_state() {
    let (mut e, a, b, target) = fixture(1.0);
    set(
        &mut e,
        a,
        StyleProperty::Display,
        openui_style::Display::None,
    );
    let id = e
        .animate_with_attachment_range(
            target,
            frames(),
            options(),
            source(a),
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    e.seek_animation_progress(id, 50.0).unwrap();
    e.set_animation_timeline_source(id, source(a)).unwrap();
    e.set_animation_timeline_source(id, source(b)).unwrap();
    assert_eq!(progress(&mut e, id), 50.0);
    assert_eq!(e.animation_state(id).unwrap().play_state, PlayState::Paused);
    assert_eq!(opacity(&e, target), 0.5);
    e.play_animation(id).unwrap();
    assert_eq!(progress(&mut e, id), forward_percentage());
    e.seek_animation_progress(id, 400.0).unwrap();
    e.pause_animation(id).unwrap();
    e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
        .unwrap();
    assert_eq!(e.animation_state(id).unwrap().current_time_ms, 4000.0);
}

#[test]
fn invalid_foreign_stale_and_removed_source_recovery_are_atomic() {
    let (mut e, a, b, target) = fixture(1.0);
    let id = e
        .animate_with_attachment_range(
            target,
            frames(),
            options(),
            source(a),
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    e.update().unwrap();
    e.drain_animation_events();
    let state = e.animation_state(id).unwrap();
    let old_opacity = opacity(&e, target);
    let generations = e.dirty_generations();
    let (mut other, foreign, _, _) = fixture(1.0);
    other.update().unwrap();
    assert!(e
        .set_animation_timeline_source(id, source(foreign))
        .is_err());
    assert!(e
        .set_animation_timeline_source(
            id,
            AnimationTimelineSource::ViewWithInset {
                subject: b,
                axis: TimelineAxis::Block,
                inset: ViewTimelineInset {
                    start: Some(LengthValue::auto()),
                    end: None
                }
            }
        )
        .is_err());
    assert_eq!(e.animation_state(id).unwrap(), state);
    assert_eq!(opacity(&e, target), old_opacity);
    assert_eq!(e.dirty_generations(), generations);
    assert!(e.drain_animation_events().is_empty());
    e.remove(a).unwrap();
    assert!(e.set_animation_timeline_source(id, source(a)).is_err());
    e.set_animation_timeline_source(id, source(b)).unwrap();
    assert_eq!(progress(&mut e, id), forward_percentage());
    e.cancel_animation(id).unwrap();
    assert!(e.set_animation_timeline_source(id, source(b)).is_err());
}

#[test]
fn empty_view_inset_is_the_same_binding_and_axis_changes_are_real() {
    let (mut e, a, _, target) = fixture(1.0);
    let subject = node(&mut e, a, 20.0, 20.0);
    let view = AnimationTimelineSource::View {
        subject,
        axis: TimelineAxis::Block,
    };
    let id = e
        .animate_with_attachment_range(
            target,
            frames(),
            options(),
            view,
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    e.seek_animation_progress(id, 25.0).unwrap();
    e.update().unwrap();
    let state = e.animation_state(id).unwrap();
    let generations = e.dirty_generations();
    e.set_animation_timeline_source(
        id,
        AnimationTimelineSource::ViewWithInset {
            subject,
            axis: TimelineAxis::Block,
            inset: ViewTimelineInset::default(),
        },
    )
    .unwrap();
    assert_eq!(e.animation_state(id).unwrap(), state);
    assert_eq!(e.dirty_generations(), generations);
    set(
        &mut e,
        subject,
        StyleProperty::Width,
        LengthValue::px(200.0),
    );
    e.set_animation_timeline_source(
        id,
        AnimationTimelineSource::Scroll {
            source: a,
            axis: TimelineAxis::X,
        },
    )
    .unwrap();
    assert_eq!(
        e.animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .physical_axis,
        Some(TimelineAxis::X)
    );
}

#[test]
fn reverse_and_zero_end_keep_their_shared_control_and_normalization_rules() {
    let (mut e, a, b, target) = fixture(1.0);
    let mut o = options();
    o.playback_rate = -1.0;
    let id = e
        .animate_with_attachment_range(
            target,
            frames(),
            o,
            source(a),
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    e.set_animation_timeline_source(id, source(b)).unwrap();
    assert_eq!(progress(&mut e, id), reverse_percentage());
    assert_eq!(opacity(&e, target), 0.25);
    e.pause_animation(id).unwrap();
    e.seek_animation_progress(id, -100.0).unwrap();
    e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
        .unwrap();
    assert_eq!(e.animation_state(id).unwrap().current_time_ms, -1000.0);
    assert_eq!(e.animation_state(id).unwrap().play_state, PlayState::Paused);
    e.cancel_animation(id).unwrap();
    let mut o = options();
    o.duration_ms = 0.0;
    o.play_state = PlayState::Paused;
    let zero = e
        .animate(target, frames(), o, AnimationTimeline::Document)
        .unwrap();
    e.set_animation_timeline_source(zero, source(a)).unwrap();
    assert_eq!(e.animation_state(zero).unwrap().current_time_ms, 0.0);
    e.set_animation_timeline_source(zero, AnimationTimelineSource::Document)
        .unwrap();
    assert_eq!(e.animation_state(zero).unwrap().current_time_ms, 0.0);
    assert_eq!(
        e.animation_state(zero).unwrap().play_state,
        PlayState::Paused
    );
}

#[test]
fn zero_playback_rate_distinguishes_play_alignment_from_a_paused_hold() {
    for play_state in [PlayState::Running, PlayState::Paused] {
        let (mut e, a, b, target) = fixture(1.0);
        let mut o = options();
        o.playback_rate = 0.0;
        o.play_state = play_state;
        let id = e
            .animate(target, frames(), o, AnimationTimeline::Document)
            .unwrap();
        e.seek_animation(id, 250.0).unwrap();
        e.set_animation_timeline_source(id, source(a)).unwrap();
        // Running setTimeline invokes PlayInternal then OnValidateSnapshot,
        // which clears its temporary hold when the aligned start changes.
        // A paused transfer keeps its proportional hold without playing.
        let expected = if play_state == PlayState::Paused {
            25.0
        } else {
            0.0
        };
        assert_eq!(progress(&mut e, id), expected);
        assert_eq!(e.animation_state(id).unwrap().play_state, play_state);
        assert!(e.animation_state(id).unwrap().current_time_ms.is_finite());
        e.set_animation_timeline_source(id, source(b)).unwrap();
        assert_eq!(progress(&mut e, id), expected);
        e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
            .unwrap();
        assert_eq!(
            e.animation_state(id).unwrap().current_time_ms,
            if play_state == PlayState::Paused {
                250.0
            } else {
                0.0
            }
        );
        assert_eq!(e.animation_state(id).unwrap().play_state, play_state);
        assert!(opacity(&e, target).is_finite());
    }
}

#[test]
fn infinite_effect_end_does_not_invent_a_transfer_ratio() {
    for play_state in [PlayState::Running, PlayState::Paused] {
        let (mut e, a, b, target) = fixture(1.0);
        let mut o = options();
        o.iterations = IterationCount::Infinite;
        o.play_state = play_state;
        assert!(o.effect_end_ms().is_infinite());
        let id = e
            .animate(target, frames(), o, AnimationTimeline::Document)
            .unwrap();
        e.seek_animation(id, 250.0).unwrap();
        e.set_animation_timeline_source(id, source(a)).unwrap();
        // setTimeline excludes an infinite old EffectEnd from progress
        // conversion. A paused hold is kept as its absolute time. The running
        // infinite-iteration normalization has zero intrinsic duration; it
        // reaches the normalized end0 after its automatic start is resolved.
        let expected = if play_state == PlayState::Paused {
            250.0
        } else {
            0.0
        };
        assert_eq!(e.animation_state(id).unwrap().current_time_ms, expected);
        assert!(opacity(&e, target).is_finite());
        e.set_animation_timeline_source(id, source(b)).unwrap();
        assert_eq!(e.animation_state(id).unwrap().current_time_ms, expected);
        e.set_animation_timeline_source(id, AnimationTimelineSource::Document)
            .unwrap();
        assert_eq!(e.animation_state(id).unwrap().play_state, play_state);
        assert!(e.animation_state(id).unwrap().current_time_ms.is_finite());
        assert!(opacity(&e, target).is_finite());
    }
}
