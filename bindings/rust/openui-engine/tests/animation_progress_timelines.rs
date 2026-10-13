use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, AnimationTimelineSource, Engine, NodeHandle};
use openui_geometry::ViewportMetrics;
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, AnimationPhase, Color, Direction, Display,
    FillMode, FlexDirection, IterationCount, Keyframes, LengthValue, Overflow, Position,
    PropertyKeyframes, StyleProperty, StyleValue, TimelineAxis, TimelineRange, TransformList,
    TransformOperation, ViewTimelineInset, WritingMode,
};

fn set(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: impl Into<StyleValue>,
) {
    engine.set_property(node, property, value.into()).unwrap();
}
fn box_(engine: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    set(engine, node, StyleProperty::Display, Display::Block);
    set(engine, node, StyleProperty::Width, LengthValue::px(width));
    set(engine, node, StyleProperty::Height, LengthValue::px(height));
    engine.append_child(parent, node).unwrap();
    node
}
fn fixture(scale: f64) -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(500.0, 400.0, scale).unwrap()).unwrap();
    let root = engine.root();
    let source = box_(&mut engine, root, 100.0, 80.0);
    set(
        &mut engine,
        source,
        StyleProperty::Overflow,
        Overflow::Hidden,
    );
    let content = box_(&mut engine, source, 100.0, 180.0);
    let target = box_(&mut engine, root, 10.0, 10.0);
    (engine, source, content, target)
}
fn options() -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    }
}
fn animate(
    engine: &mut Engine,
    target: NodeHandle,
    timeline: AnimationTimeline,
    options: AnimationOptions,
) -> AnimationId {
    engine
        .animate(
            target,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
            )
            .unwrap(),
            options,
            timeline,
        )
        .unwrap()
}
fn scroll(source: NodeHandle, axis: TimelineAxis, start: f64, end: f64) -> AnimationTimeline {
    AnimationTimeline::Scroll {
        source,
        axis,
        range: TimelineRange::new(start, end).unwrap(),
    }
}
fn opacity(engine: &Engine, target: NodeHandle) -> f32 {
    engine.computed_style(target).unwrap().opacity
}

#[test]
fn logical_axes_and_negative_origins_share_retained_scroll_geometry_at_five_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for direction in [Direction::Ltr, Direction::Rtl] {
                for axis in [TimelineAxis::Inline, TimelineAxis::Block] {
                    let (mut engine, source, content, target) = fixture(scale);
                    set(
                        &mut engine,
                        content,
                        StyleProperty::Width,
                        LengthValue::px(400.0),
                    );
                    set(
                        &mut engine,
                        content,
                        StyleProperty::Height,
                        LengthValue::px(300.0),
                    );
                    set(&mut engine, source, StyleProperty::WritingMode, mode);
                    set(&mut engine, source, StyleProperty::Direction, direction);
                    let id = animate(
                        &mut engine,
                        target,
                        scroll(source, axis, 0.0, 100.0),
                        options(),
                    );
                    let x = if (mode == WritingMode::HorizontalTb && direction == Direction::Rtl)
                        || mode == WritingMode::VerticalRl
                    {
                        -50.0
                    } else {
                        50.0
                    };
                    let y = if mode != WritingMode::HorizontalTb && direction == Direction::Rtl {
                        -50.0
                    } else {
                        50.0
                    };
                    engine.scroll_to(source, x, y).unwrap();
                    assert_eq!(opacity(&engine, target), 0.5);
                    let owned = engine.animation_timeline_snapshot(id).unwrap().unwrap();
                    let horizontal = (axis == TimelineAxis::Inline) == mode.is_horizontal();
                    assert_eq!(
                        owned.physical_axis,
                        Some(if horizontal {
                            TimelineAxis::X
                        } else {
                            TimelineAxis::Y
                        })
                    );
                    assert_eq!(owned.scroll_offset, Some(50.0));
                    assert!(!engine.is_animating());
                    let generation = engine.scene().unwrap().generation();
                    engine.set_animation_time(1000.0).unwrap();
                    assert_eq!(opacity(&engine, target), 0.5);
                    assert_eq!(engine.scene().unwrap().generation(), generation);
                }
            }
        }
    }
}

#[test]
fn finite_and_infinite_effect_timing_attach_to_ranges_without_document_time() {
    let (mut engine, source, _, target) = fixture(1.0);
    let id = animate(
        &mut engine,
        target,
        scroll(source, TimelineAxis::Y, 0.0, 100.0),
        AnimationOptions {
            delay_ms: -25.0,
            iterations: IterationCount::Number(2.5),
            ..options()
        },
    );
    engine.scroll_to(source, 0.0, 25.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.8125);
    let owned = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(owned.duration_ms, Some(1.6));
    assert_eq!(owned.current_time_ms, Some(0.4));
    assert_eq!(owned.current_time_percentage, Some(25.0));
    let root = engine.root();
    let infinite = box_(&mut engine, root, 10.0, 10.0);
    let id = animate(
        &mut engine,
        infinite,
        scroll(source, TimelineAxis::Y, 0.0, 100.0),
        AnimationOptions {
            iterations: IterationCount::Infinite,
            ..options()
        },
    );
    assert_eq!(opacity(&engine, infinite), 1.0);
    engine.finish_animation(id).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
}

#[test]
fn progress_controls_follow_nonmonotonic_alignment_and_can_reenter_after_finish() {
    let (mut engine, source, _, target) = fixture(1.0);
    let id = animate(
        &mut engine,
        target,
        scroll(source, TimelineAxis::Y, 0.0, 100.0),
        options(),
    );
    engine.scroll_to(source, 0.0, 25.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.25);
    engine.set_animation_playback_rate(id, 2.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.5);
    engine.set_animation_playback_rate(id, -1.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.75);
    engine.scroll_to(source, 0.0, 75.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.25);
    engine.pause_animation(id).unwrap();
    engine.scroll_to(source, 0.0, 50.0).unwrap();
    engine.set_animation_playback_rate(id, 1.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.25);
    engine.play_animation(id).unwrap();
    assert_eq!(opacity(&engine, target), 0.5);
    engine.seek_animation_progress(id, 80.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.8);
    engine.scroll_to(source, 0.0, 60.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.9);
    engine.set_animation_playback_rate(id, -1.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.7);
    engine.finish_animation(id).unwrap();
    assert_eq!(opacity(&engine, target), 0.0);
    assert!(engine.animation_state(id).unwrap().finished);
    engine.scroll_to(source, 0.0, 50.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.1);
    assert!(!engine.animation_state(id).unwrap().finished);
    engine.set_animation_playback_rate(id, 0.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.0);
    engine.scroll_to(source, 0.0, 75.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.0);
    assert!(engine.finish_animation(id).is_err());
}

#[test]
fn paused_percentage_tracks_snapshot_duration_changes_and_owned_data_stays_copied() {
    let (mut engine, source, content, target) = fixture(1.0);
    let id = animate(
        &mut engine,
        target,
        scroll(source, TimelineAxis::Y, 0.0, 100.0),
        options(),
    );
    engine.scroll_to(source, 0.0, 25.0).unwrap();
    engine.pause_animation(id).unwrap();
    let previous = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    set(
        &mut engine,
        content,
        StyleProperty::Height,
        LengthValue::px(280.0),
    );
    let current = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(current.duration_ms, Some(3.2));
    assert_eq!(current.current_time_ms, Some(0.8));
    assert_eq!(current.current_time_percentage, Some(25.0));
    assert_eq!(opacity(&engine, target), 0.5);
    assert_eq!(previous.duration_ms, Some(1.6));
    assert_eq!(previous.current_time_ms, Some(0.4));
}

#[test]
fn inactive_hidden_detached_and_deleted_timeline_dependencies_do_not_poison_the_engine() {
    let (mut engine, source, _, target) = fixture(1.0);
    set(&mut engine, source, StyleProperty::Display, Display::None);
    let id = animate(
        &mut engine,
        target,
        scroll(source, TimelineAxis::Y, 0.0, 100.0),
        options(),
    );
    let inactive = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert!(!inactive.active);
    assert_eq!(inactive.current_time_ms, None);
    engine.seek_animation_progress(id, 35.0).unwrap();
    set(&mut engine, source, StyleProperty::Display, Display::Block);
    let active = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert!(active.active);
    assert_eq!(active.current_time_percentage, Some(35.0));
    assert_eq!(opacity(&engine, target), 0.35);
    engine.detach(source).unwrap();
    assert!(
        !engine
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .active
    );
    assert_eq!(opacity(&engine, target), 1.0);
    engine.append_child(engine.root(), source).unwrap();
    assert!(
        engine
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .active
    );
    engine.remove(source).unwrap();
    assert!(
        !engine
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .active
    );
    engine.set_animation_time(100.0).unwrap();
    engine.scene().unwrap();
    engine.cancel_animation(id).unwrap();
    assert!(!inactive.active);
}

#[test]
fn view_cover_uses_nearest_scrollport_padding_and_ignores_authored_transforms() {
    let (mut engine, source, content, target) = fixture(1.0);
    set(
        &mut engine,
        content,
        StyleProperty::Height,
        LengthValue::px(300.0),
    );
    let subject = box_(&mut engine, content, 30.0, 20.0);
    set(
        &mut engine,
        subject,
        StyleProperty::MarginTop,
        LengthValue::px(120.0),
    );
    set(
        &mut engine,
        subject,
        StyleProperty::Transform,
        TransformList(vec![TransformOperation::Translate(
            LengthValue::px(200.0),
            LengthValue::px(200.0),
        )]),
    );
    set(
        &mut engine,
        source,
        StyleProperty::ScrollPaddingTop,
        LengthValue::px(10.0),
    );
    set(
        &mut engine,
        source,
        StyleProperty::ScrollPaddingBottom,
        LengthValue::px(20.0),
    );
    let id = animate(
        &mut engine,
        target,
        AnimationTimeline::View {
            subject,
            axis: TimelineAxis::Block,
            range: TimelineRange::new(0.0, 70.0).unwrap(),
        },
        options(),
    );
    let initial = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(initial.source, Some(source));
    assert_eq!(initial.cover_start, Some(60.0));
    assert_eq!(initial.cover_end, Some(130.0));
    engine.scroll_to(source, 0.0, 70.0).unwrap();
    assert_eq!(opacity(&engine, target), 1.0_f32 / 7.0);
    assert_eq!(engine.animation_time(), 0.0);
}

#[test]
fn sticky_view_range_retains_source_constraints_across_scroll_and_repeated_barriers() {
    let (mut engine, source, content, target) = fixture(1.0);
    set(
        &mut engine,
        content,
        StyleProperty::Height,
        LengthValue::px(300.0),
    );
    let spacer = box_(&mut engine, content, 30.0, 120.0);
    let subject = box_(&mut engine, content, 30.0, 20.0);
    set(
        &mut engine,
        subject,
        StyleProperty::Position,
        Position::Sticky,
    );
    set(
        &mut engine,
        subject,
        StyleProperty::Top,
        LengthValue::px(0.0),
    );
    let id = animate(
        &mut engine,
        target,
        AnimationTimeline::View {
            subject,
            axis: TimelineAxis::Y,
            range: TimelineRange::new(0.0, 260.0).unwrap(),
        },
        options(),
    );
    let initial = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(initial.cover_start, Some(40.0));
    assert_eq!(initial.cover_end, Some(300.0));
    engine.scroll_to(source, 0.0, 150.0).unwrap();
    let current = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(current.cover_start, initial.cover_start);
    assert_eq!(current.cover_end, initial.cover_end);
    let generation = engine.scene().unwrap().generation();
    for _ in 0..3 {
        assert_eq!(
            engine.animation_timeline_snapshot(id).unwrap().unwrap(),
            current
        );
        assert_eq!(engine.scene().unwrap().generation(), generation);
    }
    let _ = spacer;
}

#[test]
fn reversed_native_ranges_remain_oriented_and_overflowing_spans_reject_before_retention() {
    let (mut engine, source, _, target) = fixture(1.0);
    let id = animate(
        &mut engine,
        target,
        scroll(source, TimelineAxis::Y, 100.0, 0.0),
        options(),
    );
    engine.scroll_to(source, 0.0, 25.0).unwrap();
    assert_eq!(opacity(&engine, target), 0.75);
    let owned = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(owned.range_start, 100.0);
    assert_eq!(owned.range_end, 0.0);
    let count = engine.active_animation_count();
    let generation = engine.scene().unwrap().generation();
    assert!(engine
        .animate(
            target,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32)
            )
            .unwrap(),
            options(),
            AnimationTimeline::Scroll {
                source,
                axis: TimelineAxis::Y,
                range: TimelineRange {
                    start: -f64::MAX,
                    end: f64::MAX
                }
            }
        )
        .is_err());
    assert_eq!(engine.active_animation_count(), count);
    assert_eq!(engine.scene().unwrap().generation(), generation);
}

#[test]
fn progress_timeline_idle_transitions_cancel_and_reactivation_starts_again() {
    let (mut engine, source, _, target) = fixture(1.0);
    let id = animate(
        &mut engine,
        target,
        scroll(source, TimelineAxis::Y, 0.0, 100.0),
        options(),
    );
    engine.scroll_to(source, 0.0, 25.0).unwrap();
    engine.drain_animation_events();
    set(&mut engine, source, StyleProperty::Display, Display::None);
    engine.animation_timeline_snapshot(id).unwrap();
    let canceled = engine.drain_animation_events();
    assert_eq!(canceled.len(), 1);
    assert_eq!(canceled[0].kind, openui_engine::AnimationEventKind::Cancel);
    assert_eq!(canceled[0].elapsed_time_ms, 0.4);
    engine.scene().unwrap();
    assert!(engine.drain_animation_events().is_empty());
    set(&mut engine, source, StyleProperty::Display, Display::Block);
    engine.animation_timeline_snapshot(id).unwrap();
    let restarted = engine.drain_animation_events();
    assert_eq!(restarted.len(), 1);
    assert_eq!(restarted[0].kind, openui_engine::AnimationEventKind::Start);
    assert_eq!(restarted[0].elapsed_time_ms, 0.0);
    assert_eq!(opacity(&engine, target), 0.25);
}

#[test]
fn integer_scroll_api_and_fractional_attachment_alignment_keep_distinct_clock_policies() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, _, target) = fixture(scale);
        let id = animate(
            &mut engine,
            target,
            scroll(source, TimelineAxis::Y, 0.0, 100.0),
            options(),
        );
        engine.scroll_to(source, 0.0, 1.0 / 32.0).unwrap();
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .current_time_ms,
            Some(0.0)
        );
        assert_eq!(opacity(&engine, target), 0.0);
        engine.scroll_to(source, 0.0, 3.0 / 32.0).unwrap();
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .current_time_ms,
            Some(0.0)
        );
        // Default .50 FractionalScrollOffsets is disabled: the Engine's
        // ordinary scroll setter rounds this 3/32px request to zero. The pure
        // cover-clock guard separately tests integer-us truncation.
        assert_eq!(engine.scroll_offset(source).unwrap().1, 0.0);
        assert_eq!(opacity(&engine, target), 0.0);
        engine.cancel_animation(id).unwrap();
        engine.scroll_to(source, 0.0, 0.0).unwrap();
        let shifted = animate(
            &mut engine,
            target,
            scroll(source, TimelineAxis::Y, 1.0 / 32.0, 100.0),
            options(),
        );
        // The attachment start is not quantized together with the timeline.
        assert_eq!(
            engine
                .animation_timeline_snapshot(shifted)
                .unwrap()
                .unwrap()
                .current_time_ms,
            Some(-((1.0_f64 / 32.0 / 100.0) * ((100.0 * 16.0 / 1000.0) / 1000.0)) * 1000.0)
        );
    }
}

#[test]
fn auto_aligned_physical_endpoints_are_active_but_interior_and_explicit_boundaries_end() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for rate in [1.0, -1.0] {
            let (mut engine, source, _, target) = fixture(scale);
            set(
                &mut engine,
                target,
                StyleProperty::Opacity,
                StyleValue::Number(0.25),
            );
            let id = animate(
                &mut engine,
                target,
                scroll(source, TimelineAxis::Y, 0.0, 100.0),
                AnimationOptions {
                    playback_rate: rate,
                    fill: FillMode::None,
                    ..options()
                },
            );
            assert_eq!(
                engine.animation_state(id).unwrap().phase,
                AnimationPhase::Active
            );
            assert_eq!(opacity(&engine, target), if rate > 0.0 { 0.0 } else { 1.0 });
            engine.drain_animation_events();
            engine.scroll_to(source, 0.0, 50.0).unwrap();
            engine.drain_animation_events();
            engine.scroll_to(source, 0.0, 100.0).unwrap();
            assert_eq!(
                engine.animation_state(id).unwrap().phase,
                AnimationPhase::Active
            );
            assert_eq!(opacity(&engine, target), if rate > 0.0 { 1.0 } else { 0.0 });
            assert!(engine
                .drain_animation_events()
                .iter()
                .all(|event| event.kind != openui_engine::AnimationEventKind::End));
            engine
                .seek_animation_progress(id, if rate > 0.0 { 100.0 } else { 0.0 })
                .unwrap();
            assert_eq!(
                engine.animation_state(id).unwrap().phase,
                if rate > 0.0 {
                    AnimationPhase::After
                } else {
                    AnimationPhase::Before
                }
            );
            assert_eq!(opacity(&engine, target), 0.25);
            let ends = engine
                .drain_animation_events()
                .into_iter()
                .filter(|event| event.kind == openui_engine::AnimationEventKind::End)
                .collect::<Vec<_>>();
            assert_eq!(ends.len(), 1);
            engine.cancel_animation(id).unwrap();
            engine.scroll_to(source, 0.0, 0.0).unwrap();
            let interior = animate(
                &mut engine,
                target,
                scroll(source, TimelineAxis::Y, 0.0, 50.0),
                AnimationOptions {
                    fill: FillMode::None,
                    ..options()
                },
            );
            engine.drain_animation_events();
            engine.scroll_to(source, 0.0, 50.0).unwrap();
            assert_eq!(
                engine.animation_state(interior).unwrap().phase,
                AnimationPhase::After
            );
            assert_eq!(opacity(&engine, target), 0.25);
            assert_eq!(
                engine
                    .drain_animation_events()
                    .iter()
                    .filter(|event| event.kind == openui_engine::AnimationEventKind::End)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn tiny_ordinary_overflow_has_no_active_scroll_timeline_at_five_scales() {
    // Stock pinned Chromium: these retained sizes both reflect scrollHeight80,
    // legal offsets0..0 and an unresolved ScrollTimeline.currentTime.
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for excess in [1.0 / 64.0, 9.0 / 64.0] {
            let (mut engine, source, content, target) = fixture(scale);
            set(
                &mut engine,
                content,
                StyleProperty::Height,
                LengthValue::px(80.0 + excess),
            );
            let id = animate(
                &mut engine,
                target,
                scroll(source, TimelineAxis::Y, 0.0, excess as f64),
                options(),
            );
            engine.scroll_to(source, 0.0, 100.0).unwrap();
            assert_eq!(engine.scroll_offset(source).unwrap(), (0.0, 0.0));
            let state = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(state.duration_ms, None);
            assert_eq!(state.current_time_ms, None);
            assert_eq!(engine.scroll_height(source).unwrap(), 80);
            set(
                &mut engine,
                content,
                StyleProperty::Height,
                LengthValue::px(81.0),
            );
            let state = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(state.duration_ms, Some(0.016));
            assert!(state.current_time_ms.is_some());
            assert_eq!(engine.scroll_height(source).unwrap(), 81);
        }
    }
}

#[test]
fn paused_timeline_resize_uses_resolved_percentage_before_new_duration() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, subject, target) = fixture(scale);
        set(
            &mut engine,
            subject,
            StyleProperty::Height,
            LengthValue::px(1.0 / 64.0),
        );
        // The source has a real legal scroll range. An 80px start inset makes
        // the ViewTimeline cover range exactly the tiny subject size.
        box_(&mut engine, source, 100.0, 200.0);
        let id = engine
            .animate_with_attachment_range(
                target,
                PropertyKeyframes::typed(
                    StyleProperty::Opacity,
                    Keyframes::from_values(0.0_f32, 1.0_f32),
                )
                .unwrap(),
                options(),
                AnimationTimelineSource::ViewWithInset {
                    subject,
                    axis: TimelineAxis::Y,
                    inset: ViewTimelineInset::new(
                        Some(LengthValue::px(80.0)),
                        Some(LengthValue::px(0.0)),
                    )
                    .unwrap(),
                },
                AnimationAttachmentRange::default(),
            )
            .unwrap();
        engine.seek_animation(id, 0.001).unwrap();
        engine.pause_animation(id).unwrap();
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .duration_ms,
            Some(0.00025)
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Height,
            LengthValue::px(9.0 / 64.0),
        );
        let state = engine.animation_timeline_snapshot(id).unwrap().unwrap();
        assert_eq!(state.duration_ms, Some(0.00225));
        assert_eq!(state.current_time_ms, Some(0.009));
        // Browser400% remains400% after this resize; preserve the exact native
        // milliseconds arithmetic assertion independently of phase snapping.
        assert_eq!(state.current_time_percentage, Some(400.0));
    }
}

#[test]
fn sticky_stage_for_block_flex_grid_preserves_static_ink_bounds_hits_and_normal_ranges() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for display in [Display::Block, Display::Flex, Display::Grid] {
            let (mut engine, source, content, target) = fixture(scale);
            let root = engine.root();
            set(
                &mut engine,
                root,
                StyleProperty::BackgroundColor,
                Color::WHITE,
            );
            set(&mut engine, source, StyleProperty::Display, display);
            set(
                &mut engine,
                source,
                StyleProperty::FlexDirection,
                FlexDirection::Column,
            );
            set(
                &mut engine,
                content,
                StyleProperty::FlexShrink,
                StyleValue::Number(0.0),
            );
            set(
                &mut engine,
                content,
                StyleProperty::Height,
                LengthValue::px(300.0),
            );
            box_(&mut engine, content, 30.0, 120.0);
            let subject = box_(&mut engine, content, 30.0, 20.0);
            set(
                &mut engine,
                subject,
                StyleProperty::Position,
                Position::Sticky,
            );
            set(
                &mut engine,
                subject,
                StyleProperty::Top,
                LengthValue::px(0.0),
            );
            set(
                &mut engine,
                subject,
                StyleProperty::BackgroundColor,
                Color::BLUE,
            );
            let id = animate(
                &mut engine,
                target,
                AnimationTimeline::View {
                    subject,
                    axis: TimelineAxis::Y,
                    range: TimelineRange::new(0.0, 260.0).unwrap(),
                },
                options(),
            );
            let initial = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(
                (initial.cover_start, initial.cover_end),
                (Some(40.0), Some(300.0))
            );
            engine.scroll_to(source, 0.0, 150.0).unwrap();
            assert_eq!(engine.bounds(subject).unwrap().unwrap().y, 0.0);
            assert_eq!(engine.hit_test(5.0, 5.0).unwrap(), Some(subject));
            let current = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(
                (current.cover_start, current.cover_end),
                (initial.cover_start, initial.cover_end)
            );
            let scene = engine.scene().unwrap();
            let frame = openui_compositor::SoftwareCompositor::default()
                .render(&scene)
                .unwrap();
            for _ in 0..3 {
                assert_eq!(
                    openui_compositor::SoftwareCompositor::default()
                        .render(&engine.scene().unwrap())
                        .unwrap(),
                    frame
                );
                assert_eq!(engine.hit_test(5.0, 5.0).unwrap(), Some(subject));
            }
        }
    }
}

#[test]
fn normalized_table_helpers_do_not_own_sticky_boundaries_but_hidden_table_does() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, content, target) = fixture(scale);
        engine.detach(content).unwrap();
        let table = box_(&mut engine, source, 100.0, 300.0);
        set(&mut engine, table, StyleProperty::Display, Display::Table);
        let caption = box_(&mut engine, table, 100.0, 20.0);
        set(
            &mut engine,
            caption,
            StyleProperty::Display,
            Display::TableCaption,
        );
        // Anonymous row and group helpers share authored styles but are not
        // an additional table/scrollport owner.
        let cell = box_(&mut engine, table, 100.0, 280.0);
        set(
            &mut engine,
            cell,
            StyleProperty::Display,
            Display::TableCell,
        );
        box_(&mut engine, cell, 30.0, 120.0);
        let subject = box_(&mut engine, cell, 30.0, 20.0);
        set(
            &mut engine,
            subject,
            StyleProperty::Position,
            Position::Sticky,
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Top,
            LengthValue::px(0.0),
        );
        let id = animate(
            &mut engine,
            target,
            AnimationTimeline::View {
                subject,
                axis: TimelineAxis::Y,
                range: TimelineRange::new(0.0, 240.0).unwrap(),
            },
            options(),
        );
        for overflow in [
            Overflow::Auto,
            Overflow::Scroll,
            Overflow::Clip,
            Overflow::Auto,
        ] {
            set(&mut engine, table, StyleProperty::Overflow, overflow);
            engine.scroll_to(source, 0.0, 150.0).unwrap();
            let snapshot = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(snapshot.source, Some(source));
            assert_eq!(engine.bounds(subject).unwrap().unwrap().y, 0.0);
            engine.scroll_to(source, 0.0, 0.0).unwrap();
        }
        set(
            &mut engine,
            table,
            StyleProperty::Overflow,
            Overflow::Hidden,
        );
        let snapshot = engine.animation_timeline_snapshot(id).unwrap().unwrap();
        assert_eq!(snapshot.source, Some(table));
        engine.scroll_to(source, 0.0, 150.0).unwrap();
        assert_eq!(engine.bounds(subject).unwrap().unwrap().y, -10.0);
        set(&mut engine, table, StyleProperty::Overflow, Overflow::Auto);
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .source,
            Some(source)
        );
        assert_eq!(engine.bounds(subject).unwrap().unwrap().y, 0.0);
    }
}

#[test]
fn propagated_body_sticky_geometry_and_invalidation_belong_to_viewport() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(500.0, 400.0, scale).unwrap()).unwrap();
        let root = engine.root();
        let html = engine.create_native_element(ElementTag::Html).unwrap();
        let body = engine.create_native_element(ElementTag::Body).unwrap();
        for node in [html, body] {
            set(&mut engine, node, StyleProperty::Display, Display::Block);
            set(
                &mut engine,
                node,
                StyleProperty::Width,
                LengthValue::px(100.0),
            );
            set(
                &mut engine,
                node,
                StyleProperty::Height,
                LengthValue::px(800.0),
            );
        }
        engine.append_child(root, html).unwrap();
        engine.append_child(html, body).unwrap();
        set(&mut engine, body, StyleProperty::Overflow, Overflow::Hidden);
        box_(&mut engine, body, 30.0, 250.0);
        let subject = box_(&mut engine, body, 30.0, 20.0);
        set(
            &mut engine,
            subject,
            StyleProperty::Position,
            Position::Sticky,
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Top,
            LengthValue::px(0.0),
        );
        let target = box_(&mut engine, root, 10.0, 10.0);
        let id = animate(
            &mut engine,
            target,
            AnimationTimeline::View {
                subject,
                axis: TimelineAxis::Y,
                range: TimelineRange::new(0.0, 950.0).unwrap(),
            },
            options(),
        );
        engine.scroll_to(body, 0.0, 300.0).unwrap();
        assert_eq!(engine.scroll_offset(body).unwrap(), (0.0, 0.0));
        engine.scroll_to(root, 0.0, 300.0).unwrap();
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .source,
            Some(root)
        );
        assert_eq!(engine.bounds(subject).unwrap().unwrap().y, 0.0);
        set(&mut engine, html, StyleProperty::Overflow, Overflow::Hidden);
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .source,
            Some(body)
        );
        assert_eq!(engine.bounds(subject).unwrap().unwrap().y, -50.0);
        set(
            &mut engine,
            html,
            StyleProperty::Overflow,
            Overflow::Visible,
        );
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .source,
            Some(root)
        );
        assert_eq!(engine.bounds(subject).unwrap().unwrap().y, 0.0);
    }
}

#[test]
fn resource_backed_replaced_sticky_subject_keeps_outer_scroll_ownership() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, content, target) = fixture(scale);
        set(
            &mut engine,
            content,
            StyleProperty::Height,
            LengthValue::px(300.0),
        );
        box_(&mut engine, content, 30.0, 120.0);
        let subject = engine.create_native_element(ElementTag::Image).unwrap();
        let resource = engine.register_image_resource(
            "memory:native-sticky-replaced", "image/svg+xml", "ad10da54d813bd1a7e9a74a8d7ecc0563a64d9c844e9810021dc83f014fb9948",
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="20"><rect width="30" height="20" fill="blue"/></svg>"#.to_vec(),
        );
        engine
            .set_image_resource(subject, resource, Some((30.0, 20.0)))
            .unwrap();
        set(&mut engine, subject, StyleProperty::Display, Display::Block);
        set(
            &mut engine,
            subject,
            StyleProperty::Width,
            LengthValue::px(30.0),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Height,
            LengthValue::px(20.0),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Position,
            Position::Sticky,
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Top,
            LengthValue::px(0.0),
        );
        engine.append_child(content, subject).unwrap();
        let id = animate(
            &mut engine,
            target,
            AnimationTimeline::View {
                subject,
                axis: TimelineAxis::Block,
                range: TimelineRange::new(0.0, 260.0).unwrap(),
            },
            options(),
        );
        for overflow in [
            Overflow::Hidden,
            Overflow::Scroll,
            Overflow::Clip,
            Overflow::Visible,
        ] {
            set(&mut engine, subject, StyleProperty::Overflow, overflow);
            engine.scroll_to(source, 0.0, 150.0).unwrap();
            let owned = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(owned.source, Some(source));
            assert_eq!(
                (owned.cover_start, owned.cover_end),
                (Some(40.0), Some(300.0))
            );
            assert_eq!(engine.bounds(subject).unwrap().unwrap().y, 0.0);
            assert_eq!(engine.scroll_offset(subject).unwrap(), (0.0, 0.0));
            engine.scroll_to(source, 0.0, 0.0).unwrap();
        }
    }
}

#[test]
fn document_boundary_snapping_changes_fill_and_active_progress_at_five_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for rate in [1.0, -1.0] {
            let (mut engine, _, _, target) = fixture(scale);
            set(
                &mut engine,
                target,
                StyleProperty::Opacity,
                StyleValue::Number(0.25),
            );
            let id = animate(
                &mut engine,
                target,
                AnimationTimeline::Document,
                AnimationOptions {
                    delay_ms: 1.0,
                    duration_ms: 2.0,
                    playback_rate: rate,
                    play_state: openui_style::PlayState::Paused,
                    fill: FillMode::None,
                    ..AnimationOptions::default()
                },
            );
            for offset in [-0.001001, -0.000999, 0.0, 0.000999, 0.001001] {
                engine.seek_animation(id, 1.0 + offset).unwrap();
                let before = offset < -0.001 || (rate < 0.0 && offset <= 0.001);
                assert_eq!(
                    engine.animation_state(id).unwrap().phase,
                    if before {
                        AnimationPhase::Before
                    } else {
                        AnimationPhase::Active
                    }
                );
                let expected = if before {
                    0.25
                } else if offset.abs() <= 0.001 {
                    0.0
                } else {
                    (((1.0 + offset) / 1000.0 - 0.001) / 0.002) as f32
                };
                assert_eq!(opacity(&engine, target), expected);
                engine.seek_animation(id, 3.0 + offset).unwrap();
                let after = offset > 0.001 || (rate > 0.0 && offset >= -0.001);
                assert_eq!(
                    engine.animation_state(id).unwrap().phase,
                    if after {
                        AnimationPhase::After
                    } else {
                        AnimationPhase::Active
                    }
                );
                let expected = if after {
                    0.25
                } else if offset.abs() <= 0.001 {
                    1.0
                } else {
                    (((3.0 + offset) / 1000.0 - 0.001) / 0.002) as f32
                };
                assert_eq!(opacity(&engine, target), expected);
            }
        }
    }
}

#[test]
fn tiny_effects_snap_before_then_after_and_keep_boundary_alignment_context() {
    let forward = AnimationOptions {
        delay_ms: 1.0,
        duration_ms: 0.0005,
        fill: FillMode::None,
        ..AnimationOptions::default()
    };
    let (sample, _) = forward.sample_local_time_with_direction_and_boundaries(1.0, false, false);
    assert_eq!(sample.phase, AnimationPhase::After);
    assert_eq!(sample.progress, None);
    let (sample, _) = forward.sample_local_time_with_direction_and_boundaries(1.0, false, true);
    assert_eq!(sample.phase, AnimationPhase::Active);
    assert_eq!(sample.progress, Some(1.0));
    let backward = AnimationOptions {
        playback_rate: -1.0,
        ..forward.clone()
    };
    let (sample, _) = backward.sample_local_time_with_direction_and_boundaries(1.0, false, false);
    assert_eq!(sample.phase, AnimationPhase::Before);
    assert_eq!(sample.progress, None);
    let (sample, _) = backward.sample_local_time_with_direction_and_boundaries(1.0, true, false);
    assert_eq!(sample.phase, AnimationPhase::Active);
    assert_eq!(sample.progress, Some(1.0));
    // More than one microsecond away never snaps, including the inactive side.
    assert_eq!(
        forward.sample_local_time(0.998999).phase,
        AnimationPhase::Before
    );
    assert_eq!(forward.sample_local_time(0.998999).progress, None);
    assert_eq!(
        backward.sample_local_time(1.001501).phase,
        AnimationPhase::After
    );
    assert_eq!(backward.sample_local_time(1.001501).progress, None);
}

#[test]
fn fractional_attachment_boundary_uses_snapped_active_time_without_resolving_inactive_sources() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, _, target) = fixture(scale);
        set(
            &mut engine,
            target,
            StyleProperty::Opacity,
            StyleValue::Number(0.25),
        );
        let id = animate(
            &mut engine,
            target,
            scroll(source, TimelineAxis::Y, 1.0 / 32.0, 100.0),
            AnimationOptions {
                fill: FillMode::None,
                ..options()
            },
        );
        assert_eq!(
            engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .current_time_ms,
            Some(-((1.0_f64 / 32.0 / 100.0) * ((100.0 * 16.0 / 1000.0) / 1000.0)) * 1000.0)
        );
        assert_eq!(
            engine.animation_state(id).unwrap().phase,
            AnimationPhase::Active
        );
        assert_eq!(opacity(&engine, target), 0.0);
        set(&mut engine, source, StyleProperty::Display, Display::None);
        let inactive = engine.animation_timeline_snapshot(id).unwrap().unwrap();
        assert!(!inactive.active);
        assert_eq!(inactive.current_time_ms, None);
        assert_eq!(opacity(&engine, target), 0.25);
        set(&mut engine, source, StyleProperty::Display, Display::Block);
        engine.animation_timeline_snapshot(id).unwrap();
        assert_eq!(
            engine.animation_state(id).unwrap().phase,
            AnimationPhase::Active
        );
        assert_eq!(opacity(&engine, target), 0.0);
    }
}
