use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, AnimationTimelineSource, Engine, NodeHandle};
use openui_geometry::ViewportMetrics;
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, AnimationPhase, Display, FillMode, Keyframes,
    LengthValue, NamedTimelineRanges, Overflow, Position, PropertyKeyframes, StyleProperty,
    StyleValue, TimelineAxis, TimelineNamedRange, TimelineRange, TimelineRangeBounds,
    TimelineRangeOffset,
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
fn fixture(
    scale: f64,
    subject_size: f32,
) -> (Engine, NodeHandle, NodeHandle, NodeHandle, NodeHandle) {
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
    let content = box_(&mut engine, source, 100.0, 400.0);
    box_(&mut engine, content, 100.0, 120.0);
    let subject = box_(&mut engine, content, 100.0, subject_size);
    let target = box_(&mut engine, root, 10.0, 10.0);
    (engine, source, content, subject, target)
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
    source: AnimationTimelineSource,
    range: AnimationAttachmentRange,
    options: AnimationOptions,
) -> AnimationId {
    engine
        .animate_with_attachment_range(
            target,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
            )
            .unwrap(),
            options,
            source,
            range,
        )
        .unwrap()
}
fn endpoint(name: TimelineNamedRange, length: LengthValue) -> TimelineRangeOffset {
    TimelineRangeOffset::new(name, length).unwrap()
}
fn bounds(start: f64, end: f64) -> TimelineRangeBounds {
    TimelineRangeBounds { start, end }
}
fn resolved(
    engine: &mut Engine,
    id: AnimationId,
) -> openui_engine::AnimationResolvedTimelineRanges {
    engine
        .animation_attachment_snapshot(id)
        .unwrap()
        .unwrap()
        .resolved
        .unwrap()
}

#[test]
fn named_view_ranges_measure_small_equal_and_covering_subjects_at_five_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for size in [20.0_f32, 80.0, 120.0] {
            let (mut engine, source, _, subject, target) = fixture(scale, size);
            let cover = bounds(40.0, 120.0 + size as f64);
            let entry_end = 40.0 + size as f64;
            let contain = bounds(120.0_f64.min(entry_end), 120.0_f64.max(entry_end));
            let expected = NamedTimelineRanges {
                cover,
                contain,
                entry: bounds(40.0, contain.start),
                entry_crossing: bounds(40.0, entry_end),
                exit: bounds(contain.end, cover.end),
                exit_crossing: bounds(120.0, cover.end),
                scroll: bounds(0.0, 320.0),
            };
            for name in [
                TimelineNamedRange::Cover,
                TimelineNamedRange::Contain,
                TimelineNamedRange::Entry,
                TimelineNamedRange::EntryCrossing,
                TimelineNamedRange::Exit,
                TimelineNamedRange::ExitCrossing,
                TimelineNamedRange::Scroll,
            ] {
                let id = animate(
                    &mut engine,
                    target,
                    AnimationTimelineSource::View {
                        subject,
                        axis: TimelineAxis::Block,
                    },
                    AnimationAttachmentRange::named(name),
                    options(),
                );
                let owned = resolved(&mut engine, id);
                assert_eq!(owned.named, expected);
                assert_eq!(owned.attachment, expected.get(name));
                assert_eq!(owned.entry_crossing_distance, size as f64);
                assert_eq!(owned.exit_crossing_distance, size as f64);
                engine
                    .scroll_to(
                        source,
                        0.0,
                        (owned.attachment.start + owned.attachment.end) * 0.5,
                    )
                    .unwrap();
                assert_eq!(
                    engine.computed_style(target).unwrap().opacity,
                    if owned.attachment.start == owned.attachment.end {
                        1.0
                    } else {
                        0.5
                    }
                );
                assert_eq!(engine.animation_time(), 0.0);
                engine.cancel_animation(id).unwrap();
            }
        }
    }
}

#[test]
fn pixel_percent_and_calc_offsets_use_used_layout_units_and_keep_owned_snapshots() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, _, subject, target) = fixture(scale, 20.0);
        for (offset, expected) in [
            (LengthValue::px(-33.3), 6.703125),
            (LengthValue::percent(33.3), 73.296875),
            (LengthValue::calc_percent_px(25.0, -0.009), 64.984375),
        ] {
            let range = AnimationAttachmentRange::new(
                endpoint(TimelineNamedRange::Cover, offset),
                endpoint(TimelineNamedRange::Cover, LengthValue::percent(100.0)),
            )
            .unwrap();
            let id = animate(
                &mut engine,
                target,
                AnimationTimelineSource::View {
                    subject,
                    axis: TimelineAxis::Y,
                },
                range,
                options(),
            );
            let owned = engine.animation_attachment_snapshot(id).unwrap().unwrap();
            assert_eq!(owned.range, range);
            assert_eq!(owned.resolved.unwrap().attachment, bounds(expected, 140.0));
            engine.scroll_to(source, 0.0, 70.0).unwrap();
            assert_eq!(owned.range, range);
            assert_eq!(owned.resolved.unwrap().attachment, bounds(expected, 140.0));
            engine.cancel_animation(id).unwrap();
        }
    }
}

#[test]
fn attachment_offsets_re_resolve_target_font_root_font_and_viewport_lengths() {
    let (mut engine, _, _, subject, target) = fixture(1.0, 20.0);
    let root = engine.root();
    set(
        &mut engine,
        root,
        StyleProperty::FontSize,
        LengthValue::px(16.0),
    );
    set(
        &mut engine,
        target,
        StyleProperty::FontSize,
        LengthValue::px(12.0),
    );
    for (offset, expected) in [
        (LengthValue::Em(1.0), 52.0),
        (LengthValue::Rem(1.0), 56.0),
        (LengthValue::ViewportWidth(1.0), 45.0),
        (LengthValue::ViewportHeight(1.0), 44.0),
        (LengthValue::ViewportMin(1.0), 44.0),
        (LengthValue::ViewportMax(1.0), 45.0),
    ] {
        let range = AnimationAttachmentRange {
            start: Some(endpoint(TimelineNamedRange::Cover, offset)),
            end: None,
        };
        let id = animate(
            &mut engine,
            target,
            AnimationTimelineSource::View {
                subject,
                axis: TimelineAxis::Y,
            },
            range,
            options(),
        );
        assert_eq!(resolved(&mut engine, id).attachment.start, expected);
        if matches!(offset, LengthValue::Em(_)) {
            set(
                &mut engine,
                target,
                StyleProperty::FontSize,
                LengthValue::px(20.0),
            );
            assert_eq!(resolved(&mut engine, id).attachment.start, 60.0);
            set(
                &mut engine,
                target,
                StyleProperty::FontSize,
                LengthValue::px(12.0),
            );
        }
        engine.cancel_animation(id).unwrap();
    }
}

#[test]
fn browser_reversed_and_equal_attachments_keep_zero_intrinsic_time_in_both_directions() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for rate in [1.0, -1.0] {
            for (start, end) in [(75.0, 25.0), (50.0, 50.0)] {
                let (mut engine, source, _, subject, target) = fixture(scale, 20.0);
                set(
                    &mut engine,
                    target,
                    StyleProperty::Opacity,
                    StyleValue::Number(0.25),
                );
                let range = AnimationAttachmentRange::new(
                    endpoint(TimelineNamedRange::Cover, LengthValue::percent(start)),
                    endpoint(TimelineNamedRange::Cover, LengthValue::percent(end)),
                )
                .unwrap();
                let id = animate(
                    &mut engine,
                    target,
                    AnimationTimelineSource::View {
                        subject,
                        axis: TimelineAxis::Y,
                    },
                    range,
                    AnimationOptions {
                        playback_rate: rate,
                        fill: FillMode::None,
                        ..options()
                    },
                );
                assert_eq!(
                    resolved(&mut engine, id).attachment,
                    bounds(40.0 + start as f64, 40.0 + end as f64)
                );
                for position in [0.0, 65.0, 90.0, 115.0, 200.0, 320.0] {
                    engine.scroll_to(source, 0.0, position).unwrap();
                    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.25);
                }
                engine.finish_animation(id).unwrap();
                assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
            }
        }
    }
}

#[test]
fn legacy_orientation_remains_distinct_and_browser_range_mutation_preserves_identity() {
    let (mut engine, source, _, subject, target) = fixture(1.0, 20.0);
    let id = engine
        .animate(
            target,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
            )
            .unwrap(),
            options(),
            AnimationTimeline::View {
                subject,
                axis: TimelineAxis::Y,
                range: TimelineRange::new(100.0, 0.0).unwrap(),
            },
        )
        .unwrap();
    assert!(engine.animation_attachment_snapshot(id).unwrap().is_none());
    engine.scroll_to(source, 0.0, 90.0).unwrap();
    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
    engine
        .set_animation_attachment_range(
            id,
            AnimationAttachmentRange::named(TimelineNamedRange::Entry),
        )
        .unwrap();
    assert_eq!(engine.animation_state(id).unwrap().id, id);
    assert_eq!(resolved(&mut engine, id).attachment, bounds(40.0, 60.0));
    engine.scroll_to(source, 0.0, 50.0).unwrap();
    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
    engine.pause_animation(id).unwrap();
    let held = engine.animation_state(id).unwrap().current_time_ms;
    engine
        .set_animation_attachment_range(
            id,
            AnimationAttachmentRange::named(TimelineNamedRange::Exit),
        )
        .unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, held);
    engine.play_animation(id).unwrap();
    assert_eq!(
        engine.animation_state(id).unwrap().phase,
        AnimationPhase::Before
    );
}

#[test]
fn zero_size_subject_uses_cover_ranges_and_normal_offsets_ignore_named_conversion() {
    let (mut engine, _, _, subject, target) = fixture(1.0, 0.0);
    for name in [
        TimelineNamedRange::Cover,
        TimelineNamedRange::Contain,
        TimelineNamedRange::Entry,
        TimelineNamedRange::EntryCrossing,
        TimelineNamedRange::Exit,
        TimelineNamedRange::ExitCrossing,
        TimelineNamedRange::Scroll,
    ] {
        let id = animate(
            &mut engine,
            target,
            AnimationTimelineSource::View {
                subject,
                axis: TimelineAxis::Y,
            },
            AnimationAttachmentRange::named(name),
            options(),
        );
        assert_eq!(resolved(&mut engine, id).attachment, bounds(40.0, 120.0));
        engine.cancel_animation(id).unwrap();
    }
}

#[test]
fn document_sources_retain_validated_descriptors_without_changing_document_time() {
    let (mut engine, _, _, _, target) = fixture(1.0, 20.0);
    let range = AnimationAttachmentRange::named(TimelineNamedRange::Exit);
    let id = animate(
        &mut engine,
        target,
        AnimationTimelineSource::Document,
        range,
        options(),
    );
    engine.set_animation_time(25.0).unwrap();
    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.25);
    let copy = engine.animation_attachment_snapshot(id).unwrap().unwrap();
    assert_eq!(copy.range, range);
    assert_eq!(copy.resolved, None);
    assert_eq!(engine.animation_timeline_snapshot(id).unwrap(), None);
    engine
        .set_animation_attachment_range(
            id,
            AnimationAttachmentRange::named(TimelineNamedRange::Entry),
        )
        .unwrap();
    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.25);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 25.0);
    assert_eq!(copy.range, range);
}

#[test]
fn invalid_descriptors_and_foreign_sources_preserve_live_animation_and_style() {
    let (mut engine, source, _, subject, target) = fixture(1.0, 20.0);
    let range = AnimationAttachmentRange::named(TimelineNamedRange::Cover);
    let id = animate(
        &mut engine,
        target,
        AnimationTimelineSource::View {
            subject,
            axis: TimelineAxis::Y,
        },
        range,
        options(),
    );
    engine.scroll_to(source, 0.0, 90.0).unwrap();
    let owned = engine.animation_attachment_snapshot(id).unwrap().unwrap();
    let state = engine.animation_state(id).unwrap();
    for offset in [
        LengthValue::auto(),
        LengthValue::none(),
        LengthValue::px(f32::NAN),
        LengthValue::percent(f32::INFINITY),
        LengthValue::calc_percent_px(0.0, f32::INFINITY),
        LengthValue::Em(f32::NAN),
    ] {
        let invalid = AnimationAttachmentRange {
            start: Some(TimelineRangeOffset {
                name: TimelineNamedRange::Cover,
                offset,
            }),
            end: None,
        };
        assert!(engine.set_animation_attachment_range(id, invalid).is_err());
        assert_eq!(
            engine.animation_attachment_snapshot(id).unwrap().unwrap(),
            owned
        );
        assert_eq!(engine.animation_state(id).unwrap(), state);
        assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
    }
    let (foreign, foreign_source, _, _, _) = fixture(1.0, 20.0);
    assert!(engine
        .animate_with_attachment_range(
            target,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32)
            )
            .unwrap(),
            options(),
            AnimationTimelineSource::Scroll {
                source: foreign_source,
                axis: TimelineAxis::Y
            },
            range
        )
        .is_err());
    assert_eq!(
        engine.animation_attachment_snapshot(id).unwrap().unwrap(),
        owned
    );
    drop(foreign);
}

#[test]
fn inactive_sources_keep_descriptors_pending_percentage_and_owned_resolved_data() {
    let (mut engine, source, _, subject, target) = fixture(1.0, 20.0);
    let id = animate(
        &mut engine,
        target,
        AnimationTimelineSource::View {
            subject,
            axis: TimelineAxis::Y,
        },
        AnimationAttachmentRange::named(TimelineNamedRange::Entry),
        options(),
    );
    let owned = engine.animation_attachment_snapshot(id).unwrap().unwrap();
    set(&mut engine, source, StyleProperty::Display, Display::None);
    let inactive = engine.animation_attachment_snapshot(id).unwrap().unwrap();
    assert_eq!(inactive.range, owned.range);
    assert_eq!(inactive.resolved, None);
    engine.seek_animation_progress(id, 10.0).unwrap();
    set(&mut engine, source, StyleProperty::Display, Display::Block);
    assert!(engine
        .animation_attachment_snapshot(id)
        .unwrap()
        .unwrap()
        .resolved
        .is_some());
    assert_eq!(
        engine
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .current_time_percentage,
        Some(10.0)
    );
    let duration_seconds = (100.0_f64 * 16.0 / 1000.0) / 1000.0;
    let raw_cover_seconds = (-40.0_f64 * 16.0) / 1_000_000.0;
    let requested_local_seconds = (10.0 / 100.0) * duration_seconds;
    let start_seconds = raw_cover_seconds - requested_local_seconds;
    assert_eq!(
        engine
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .current_time_ms,
        Some((raw_cover_seconds - start_seconds) * 1000.0)
    );
    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
    assert_eq!(owned.resolved.unwrap().attachment, bounds(40.0, 60.0));
}

#[test]
fn sticky_entry_and_exit_crossing_distances_survive_scroll_and_layout_barriers() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, content, subject, target) = fixture(scale, 20.0);
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
            LengthValue::px(70.0),
        );
        let id = animate(
            &mut engine,
            target,
            AnimationTimelineSource::View {
                subject,
                axis: TimelineAxis::Y,
            },
            AnimationAttachmentRange::named(TimelineNamedRange::EntryCrossing),
            options(),
        );
        let owned = resolved(&mut engine, id);
        // At top70 the 20px subject sticks while crossing the80px entry edge.
        // It can move260px inside its400px containing block.
        assert_eq!(owned.entry_crossing_distance, 280.0);
        assert_eq!(owned.exit_crossing_distance, 20.0);
        assert_eq!(owned.named.cover, bounds(40.0, 400.0));
        assert_eq!(owned.named.entry_crossing, bounds(40.0, 320.0));
        for scroll in [0.0, 50.0, 150.0, 300.0, 50.0, 0.0] {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            assert_eq!(resolved(&mut engine, id), owned);
        }
        set(
            &mut engine,
            subject,
            StyleProperty::Top,
            LengthValue::auto(),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Bottom,
            LengthValue::px(70.0),
        );
        let current = resolved(&mut engine, id);
        // Bottom70 places the subject at-10 while stuck: during exit.
        assert_eq!(current.entry_crossing_distance, 20.0);
        assert_eq!(current.exit_crossing_distance, 140.0);
        assert_eq!(current.named.cover, bounds(-80.0, 140.0));
        assert_eq!(current.named.exit_crossing, bounds(0.0, 140.0));
        // Changing the containing block size is live retained geometry.
        set(
            &mut engine,
            content,
            StyleProperty::Height,
            LengthValue::px(420.0),
        );
        assert_eq!(resolved(&mut engine, id).exit_crossing_distance, 140.0);
    }
}

#[test]
fn browser_scroll_attachments_resolve_logical_axes_and_signed_scroll_origins() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [
            openui_style::WritingMode::HorizontalTb,
            openui_style::WritingMode::VerticalRl,
            openui_style::WritingMode::VerticalLr,
        ] {
            for direction in [openui_style::Direction::Ltr, openui_style::Direction::Rtl] {
                for axis in [TimelineAxis::Inline, TimelineAxis::Block] {
                    let (mut engine, source, content, _, target) = fixture(scale, 20.0);
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
                        AnimationTimelineSource::Scroll { source, axis },
                        AnimationAttachmentRange::default(),
                        options(),
                    );
                    let physical_x = if axis == TimelineAxis::Inline {
                        mode == openui_style::WritingMode::HorizontalTb
                    } else {
                        mode != openui_style::WritingMode::HorizontalTb
                    };
                    let maximum = if physical_x { 300.0 } else { 220.0 };
                    let negative = if physical_x {
                        mode == openui_style::WritingMode::VerticalRl
                            || (mode == openui_style::WritingMode::HorizontalTb
                                && direction == openui_style::Direction::Rtl)
                    } else {
                        mode != openui_style::WritingMode::HorizontalTb
                            && direction == openui_style::Direction::Rtl
                    };
                    let offset = maximum * 0.5 * if negative { -1.0 } else { 1.0 };
                    engine
                        .scroll_to(
                            source,
                            if physical_x { offset } else { 0.0 },
                            if physical_x { 0.0 } else { offset },
                        )
                        .unwrap();
                    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
                    let snapshot = engine.animation_timeline_snapshot(id).unwrap().unwrap();
                    assert_eq!(
                        snapshot.physical_axis,
                        Some(if physical_x {
                            TimelineAxis::X
                        } else {
                            TimelineAxis::Y
                        })
                    );
                    assert_eq!(snapshot.scroll_offset, Some(maximum * 0.5));
                    let ranges = resolved(&mut engine, id);
                    assert_eq!(ranges.source, source);
                    assert_eq!(ranges.attachment, bounds(0.0, maximum));
                    assert_eq!(ranges.named.contain, bounds(0.0, maximum));
                }
            }
        }
    }
}

#[test]
fn explicit_and_auto_view_insets_share_layout_unit_resolution_and_subject_style() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, _, subject, target) = fixture(scale, 20.0);
        set(
            &mut engine,
            source,
            StyleProperty::ScrollPaddingTop,
            LengthValue::px(5.009),
        );
        set(
            &mut engine,
            source,
            StyleProperty::ScrollPaddingBottom,
            LengthValue::percent(12.3),
        );
        let auto = animate(
            &mut engine,
            target,
            AnimationTimelineSource::View {
                subject,
                axis: TimelineAxis::Y,
            },
            AnimationAttachmentRange::default(),
            options(),
        );
        assert_eq!(
            resolved(&mut engine, auto).named.cover,
            bounds(49.828125, 135.0)
        );
        engine.cancel_animation(auto).unwrap();
        set(
            &mut engine,
            subject,
            StyleProperty::FontSize,
            LengthValue::px(12.0),
        );
        set(
            &mut engine,
            target,
            StyleProperty::FontSize,
            LengthValue::px(20.0),
        );
        let inset = openui_style::ViewTimelineInset::new(Some(LengthValue::Em(1.0)), None).unwrap();
        let id = animate(
            &mut engine,
            target,
            AnimationTimelineSource::ViewWithInset {
                subject,
                axis: TimelineAxis::Y,
                inset,
            },
            AnimationAttachmentRange::default(),
            options(),
        );
        assert_eq!(
            resolved(&mut engine, id).named.cover,
            bounds(49.828125, 128.0)
        );
        set(
            &mut engine,
            subject,
            StyleProperty::FontSize,
            LengthValue::px(16.0),
        );
        assert_eq!(
            resolved(&mut engine, id).named.cover,
            bounds(49.828125, 124.0)
        );
        assert_eq!(resolved(&mut engine, id).entry_crossing_distance, 20.0);
        engine.cancel_animation(id).unwrap();
        let invalid = openui_style::ViewTimelineInset {
            start: Some(LengthValue::auto()),
            end: None,
        };
        assert!(engine
            .animate_with_attachment_range(
                target,
                PropertyKeyframes::typed(
                    StyleProperty::Opacity,
                    Keyframes::from_values(0.0_f32, 1.0_f32)
                )
                .unwrap(),
                options(),
                AnimationTimelineSource::ViewWithInset {
                    subject,
                    axis: TimelineAxis::Y,
                    inset: invalid
                },
                AnimationAttachmentRange::default()
            )
            .is_err());
    }
}

fn scroll_control_fixture(
    scale: f64,
    legacy: bool,
) -> (Engine, NodeHandle, NodeHandle, AnimationId) {
    let (mut engine, source, _, _, target) = fixture(scale, 20.0);
    let id = if legacy {
        engine
            .animate(
                target,
                PropertyKeyframes::typed(
                    StyleProperty::Opacity,
                    Keyframes::from_values(0.0_f32, 1.0_f32),
                )
                .unwrap(),
                options(),
                AnimationTimeline::Scroll {
                    source,
                    axis: TimelineAxis::Y,
                    range: TimelineRange::new(0.0, 320.0).unwrap(),
                },
            )
            .unwrap()
    } else {
        animate(
            &mut engine,
            target,
            AnimationTimelineSource::Scroll {
                source,
                axis: TimelineAxis::Y,
            },
            AnimationAttachmentRange::default(),
            options(),
        )
    };
    engine.animation_timeline_snapshot(id).unwrap();
    (engine, source, target, id)
}

#[test]
fn changed_controls_replace_inactive_seek_but_equal_attachment_endpoints_preserve_it() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for legacy in [false, true] {
            for operation in ["seek", "play", "finish", "range-change", "same-range"] {
                let (mut engine, source, target, id) = scroll_control_fixture(scale, legacy);
                let root = engine.root();
                engine.detach(source).unwrap();
                assert!(
                    !engine
                        .animation_timeline_snapshot(id)
                        .unwrap()
                        .unwrap()
                        .active
                );
                engine.seek_animation_progress(id, 50.0).unwrap();
                match operation {
                    "seek" => engine.seek_animation(id, 1.28).unwrap(),
                    "play" => engine.play_animation(id).unwrap(),
                    "finish" => engine.finish_animation(id).unwrap(),
                    "range-change" => engine
                        .set_animation_attachment_range(
                            id,
                            AnimationAttachmentRange::new(
                                endpoint(TimelineNamedRange::Cover, LengthValue::percent(25.0)),
                                endpoint(TimelineNamedRange::Cover, LengthValue::percent(75.0)),
                            )
                            .unwrap(),
                        )
                        .unwrap(),
                    "same-range" => engine
                        .set_animation_attachment_range(id, AnimationAttachmentRange::default())
                        .unwrap(),
                    _ => unreachable!(),
                }
                engine.append_child(root, source).unwrap();
                let active = engine.animation_timeline_snapshot(id).unwrap().unwrap();
                assert!(active.active);
                let (percentage, opacity) = match operation {
                    "seek" => (25.0, 0.25),
                    "play" => (0.0, 0.0),
                    // Equal browser attachment endpoints do not call
                    // OnRangeUpdate, so the deferred held percentage survives.
                    // The legacy numeric timeline has no attachment descriptor:
                    // converting it to AttachmentDefault is a real change.
                    "same-range" if !legacy => (50.0, 0.5),
                    "same-range" => (0.0, 0.0),
                    "finish" => (100.0, 1.0),
                    "range-change" => (-25.0, 0.0),
                    _ => unreachable!(),
                };
                assert_eq!(
                    active.current_time_percentage,
                    Some(percentage),
                    "{operation}, legacy={legacy}"
                );
                assert_eq!(
                    engine.computed_style(target).unwrap().opacity,
                    opacity,
                    "{operation}, legacy={legacy}"
                );
                let stable = engine.animation_timeline_snapshot(id).unwrap().unwrap();
                assert_eq!(
                    stable.current_time_percentage,
                    active.current_time_percentage
                );
            }
        }
    }
}

#[test]
fn inactive_percentage_hold_survives_pause_rate_and_rejected_new_time_requests() {
    for legacy in [false, true] {
        for operation in ["pause-rate", "invalid-seek", "invalid-rate"] {
            let (mut engine, source, target, id) = scroll_control_fixture(1.0, legacy);
            let root = engine.root();
            engine.detach(source).unwrap();
            engine.animation_timeline_snapshot(id).unwrap();
            engine.seek_animation_progress(id, 50.0).unwrap();
            match operation {
                "pause-rate" => {
                    engine.pause_animation(id).unwrap();
                    engine.set_animation_playback_rate(id, -2.0).unwrap();
                }
                "invalid-seek" => assert!(engine.seek_animation(id, f64::NAN).is_err()),
                "invalid-rate" => assert!(engine
                    .set_animation_playback_rate(id, f64::INFINITY)
                    .is_err()),
                _ => unreachable!(),
            }
            engine.append_child(root, source).unwrap();
            let resolved = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(resolved.current_time_percentage, Some(50.0), "{operation}");
            assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
            engine.animation_timeline_snapshot(id).unwrap();
            assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 2.56);
        }
    }
}

#[test]
fn paused_inactive_percentage_request_survives_attachment_range_mutation() {
    let (mut engine, source, target, id) = scroll_control_fixture(1.0, false);
    let root = engine.root();
    engine.detach(source).unwrap();
    engine.seek_animation_progress(id, 25.0).unwrap();
    engine.pause_animation(id).unwrap();
    engine
        .set_animation_attachment_range(
            id,
            AnimationAttachmentRange::new(
                endpoint(TimelineNamedRange::Cover, LengthValue::percent(25.0)),
                endpoint(TimelineNamedRange::Cover, LengthValue::percent(75.0)),
            )
            .unwrap(),
        )
        .unwrap();
    engine.append_child(root, source).unwrap();
    let resolved = engine.animation_timeline_snapshot(id).unwrap().unwrap();
    assert_eq!(resolved.current_time_percentage, Some(25.0));
    assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
}

#[test]
fn running_manual_view_seek_rescales_using_new_cover_clock_and_old_duration() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for new_height in [100.0_f32, 180.0] {
            let (mut engine, source, _, subject, target) = fixture(scale, 20.0);
            let id = animate(
                &mut engine,
                target,
                AnimationTimelineSource::View {
                    subject,
                    axis: TimelineAxis::Y,
                },
                AnimationAttachmentRange::named(TimelineNamedRange::Cover),
                options(),
            );
            engine.seek_animation_progress(id, 40.0).unwrap();
            let old = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(old.cover_start, Some(40.0));
            assert_eq!(old.cover_end, Some(140.0));
            assert_eq!(old.duration_ms, Some(1.6));
            assert_eq!(old.current_time_percentage, Some(40.0));
            let old_duration_seconds = (100.0_f64 * 16.0 / 1000.0) / 1000.0;
            let old_clock_seconds = (-40.0_f64 * 16.0) / 1_000_000.0;
            let old_anchor_seconds = old_clock_seconds - (40.0 / 100.0) * old_duration_seconds;
            set(
                &mut engine,
                source,
                StyleProperty::Height,
                LengthValue::px(new_height),
            );
            let next = engine.animation_timeline_snapshot(id).unwrap().unwrap();
            assert_eq!(next.cover_start, Some(120.0 - new_height as f64));
            assert_eq!(next.cover_end, Some(140.0));
            let new_cover = new_height as f64 + 20.0;
            let new_duration_seconds = (new_cover * 16.0 / 1000.0) / 1000.0;
            assert_eq!(next.duration_ms, Some(new_duration_seconds * 1000.0));
            let new_clock_seconds = ((new_height as f64 - 120.0) * 16.0) / 1_000_000.0;
            // OnValidateSnapshot uses new raw time with the old anchor; only its
            // resize ratio reflects both operands into milliseconds.
            let unlimited_seconds = new_clock_seconds - old_anchor_seconds;
            let resized_seconds = ((unlimited_seconds * 1000.0) / (old_duration_seconds * 1000.0))
                * new_duration_seconds;
            let new_anchor_seconds = new_clock_seconds - resized_seconds;
            let current_seconds = new_clock_seconds - new_anchor_seconds;
            // The old guard exposed the intermediate UnlimitedCurrentTime as
            // public current time. A subsequent continuous finished update holds
            // EffectEnd when the previous 0.64ms was inside the new effect.
            // Stock .50 repeats confirm 100%/finished at height180 and the exact
            // in-range percentage below at height100, at all five scales.
            let exposed_seconds = if new_height == 180.0 {
                assert_eq!(current_seconds * 1000.0, 4.48);
                assert!(current_seconds > new_duration_seconds);
                new_duration_seconds
            } else {
                assert!(current_seconds < new_duration_seconds);
                current_seconds
            };
            assert_eq!(next.current_time_ms, Some(exposed_seconds * 1000.0));
            assert_eq!(
                next.current_time_percentage,
                Some(exposed_seconds / new_duration_seconds * 100.0)
            );
            assert_eq!(
                engine.animation_state(id).unwrap().finished,
                new_height == 180.0
            );
            let stale_clock_rescale = ((old_clock_seconds - old_anchor_seconds) * 1000.0)
                / (old_duration_seconds * 1000.0)
                * new_duration_seconds;
            assert_ne!(next.current_time_ms, Some(stale_clock_rescale * 1000.0));
            assert_eq!(
                engine.computed_style(target).unwrap().opacity,
                if new_height == 180.0 { 1.0 } else { 0.6 }
            );
            assert_eq!(
                engine.animation_state(id).unwrap().phase,
                if new_height == 180.0 {
                    AnimationPhase::After
                } else {
                    AnimationPhase::Active
                }
            );
            if new_height == 180.0 {
                assert_eq!(next.current_time_ms, Some(3.2));
                assert_eq!(next.current_time_percentage, Some(100.0));
            } else {
                assert_eq!(next.current_time_percentage, Some(60.00000000000001));
            }
            assert_eq!(old.current_time_percentage, Some(40.0));
            engine.cancel_animation(id).unwrap();
        }
    }
}
