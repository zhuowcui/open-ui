use openui_dom::ElementTag;
use openui_engine::{AnimationTimeline, Engine, EngineError, NodeHandle, ViewportMetrics};
use openui_style::{
    AnimationError, AnimationOptions, Display, Easing, FillMode, IterationCount, Keyframe,
    Keyframes, LengthValue, PercentageLinearStop, PlaybackDirection, PropertyKeyframes,
    StepPosition, StyleProperty,
};

fn native_engine() -> (Engine, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap()).unwrap();
    let node = engine.create_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    engine
        .set_property(node, StyleProperty::Display, Display::Block.into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Width, LengthValue::px(60.0).into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Height, LengthValue::px(20.0).into())
        .unwrap();
    (engine, node)
}

fn frames(values: &[(f64, f32)]) -> PropertyKeyframes {
    PropertyKeyframes::typed(
        StyleProperty::Width,
        Keyframes::new(
            values
                .iter()
                .map(|(offset, value)| Keyframe::new(*offset, LengthValue::px(*value)))
                .collect(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn curve(mid_output: f64) -> Easing {
    Easing::linear_stops_percent(vec![
        PercentageLinearStop {
            input_percent: 0.0,
            output: 0.0,
        },
        PercentageLinearStop {
            input_percent: 50.0,
            output: mid_output,
        },
        PercentageLinearStop {
            input_percent: 100.0,
            output: 1.0,
        },
    ])
    .unwrap()
}

fn options(easing: Easing) -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        easing,
        ..AnimationOptions::default()
    }
}

fn width(engine: &Engine, node: NodeHandle) -> f32 {
    engine.computed_style(node).unwrap().width.value()
}

#[test]
fn animation_allocation_failure_preserves_its_type_through_the_engine() {
    assert_eq!(
        EngineError::from(AnimationError::AllocationFailure),
        EngineError::AllocationFailure,
    );
}

#[test]
fn effect_easing_transforms_the_whole_multikeyframe_effect() {
    let (mut engine, node) = native_engine();
    engine
        .animate(
            node,
            frames(&[(0.0, 0.0), (0.5, 20.0), (1.0, 100.0)]),
            options(curve(0.25)),
            AnimationTimeline::Document,
        )
        .unwrap();
    for (time, expected) in [
        (0.0, 0.0),
        (25.0, 5.0),
        (50.0, 10.0),
        (75.0, 40.0),
        (100.0, 100.0),
    ] {
        engine.set_animation_time(time).unwrap();
        assert_eq!(width(&engine, node), expected);
    }
}

#[test]
fn effect_and_explicit_keyframe_easing_compose_in_order() {
    let (mut engine, node) = native_engine();
    let keyframes = PropertyKeyframes::typed(
        StyleProperty::Width,
        Keyframes::new(vec![
            Keyframe::new(0.0, LengthValue::px(0.0)).easing(curve(0.75)),
            Keyframe::new(0.5, LengthValue::px(20.0)),
            Keyframe::new(1.0, LengthValue::px(100.0)),
        ])
        .unwrap(),
    )
    .unwrap();
    engine
        .animate(
            node,
            keyframes,
            options(curve(0.25)),
            AnimationTimeline::Document,
        )
        .unwrap();
    engine.set_animation_time(50.0).unwrap();
    assert_eq!(width(&engine, node), 15.0);

    let (mut linear, neighbor) = native_engine();
    linear
        .animate(
            neighbor,
            frames(&[(0.0, 0.0), (0.5, 20.0), (1.0, 100.0)]),
            options(Easing::Linear),
            AnimationTimeline::Document,
        )
        .unwrap();
    linear.set_animation_time(50.0).unwrap();
    assert_eq!(width(&linear, neighbor), 20.0);
}

#[test]
fn effect_and_keyframe_step_limits_follow_the_distinct_pinned_phase_rules() {
    for per_keyframe in [false, true] {
        for reversed in [false, true] {
            let (mut engine, node) = native_engine();
            let steps = Easing::steps(4, StepPosition::JumpStart).unwrap();
            let first = Keyframe::new(0.0, LengthValue::px(0.0));
            let first = if per_keyframe {
                first.easing(steps.clone())
            } else {
                first
            };
            let keyframes = PropertyKeyframes::typed(
                StyleProperty::Width,
                Keyframes::new(vec![first, Keyframe::new(1.0, LengthValue::px(100.0))]).unwrap(),
            )
            .unwrap();
            let mut timing = options(if per_keyframe { Easing::Linear } else { steps });
            timing.delay_ms = 10.0;
            timing.direction = if reversed {
                PlaybackDirection::Reverse
            } else {
                PlaybackDirection::Normal
            };
            let id = engine
                .animate(node, keyframes, timing, AnimationTimeline::Document)
                .unwrap();
            assert_eq!(width(&engine, node), if reversed { 100.0 } else { 0.0 });
            engine.set_animation_time(10.0).unwrap();
            assert_eq!(width(&engine, node), if reversed { 100.0 } else { 25.0 });
            engine.set_animation_time(110.0).unwrap();
            let endpoint = if !reversed {
                100.0
            } else if per_keyframe {
                25.0
            } else {
                0.0
            };
            assert_eq!(width(&engine, node), endpoint);
            assert!(engine.animation_state(id).unwrap().finished);
        }
    }
}

#[test]
fn native_percent_curves_preserve_the_29_percent_discontinuity() {
    let (mut engine, node) = native_engine();
    let easing = Easing::linear_stops_percent(vec![
        PercentageLinearStop {
            input_percent: 0.0,
            output: 0.0,
        },
        PercentageLinearStop {
            input_percent: 29.0,
            output: 0.25,
        },
        PercentageLinearStop {
            input_percent: 29.0,
            output: 0.75,
        },
        PercentageLinearStop {
            input_percent: 100.0,
            output: 1.0,
        },
    ])
    .unwrap();
    let id = engine
        .animate(
            node,
            frames(&[(0.0, 60.0), (1.0, 100.0)]),
            options(easing),
            AnimationTimeline::Document,
        )
        .unwrap();
    engine.seek_animation(id, 29.0).unwrap();
    assert_eq!(width(&engine, node), 70.0);
    engine
        .seek_animation(id, f64::from_bits(29.0_f64.to_bits() + 2))
        .unwrap();
    assert_eq!(width(&engine, node), 90.0);
}

#[test]
fn effect_overshoot_continues_through_the_outer_keyframe_intervals() {
    for (output, expected) in [(-0.25, 50.0), (1.25, 110.0)] {
        let (mut engine, node) = native_engine();
        engine
            .animate(
                node,
                frames(&[(0.0, 60.0), (1.0, 100.0)]),
                options(curve(output)),
                AnimationTimeline::Document,
            )
            .unwrap();
        engine.set_animation_time(50.0).unwrap();
        assert_eq!(width(&engine, node), expected);
    }
}

#[test]
fn effect_step_limits_use_actual_direction_after_the_iteration_counter_saturates() {
    for (direction, position, expected) in [
        (PlaybackDirection::Alternate, StepPosition::JumpEnd, 100.0),
        (
            PlaybackDirection::AlternateReverse,
            StepPosition::JumpStart,
            0.0,
        ),
    ] {
        let (mut engine, node) = native_engine();
        let mut timing = options(Easing::steps(4, position).unwrap());
        timing.duration_ms = 0.0;
        timing.iterations = IterationCount::Infinite;
        timing.direction = direction;
        let id = engine
            .animate(
                node,
                frames(&[(0.0, 0.0), (1.0, 100.0)]),
                timing,
                AnimationTimeline::Document,
            )
            .unwrap();
        assert_eq!(engine.animation_state(id).unwrap().iteration, u64::MAX);
        assert_eq!(width(&engine, node), expected);
        engine.set_animation_time(100.0).unwrap();
        assert_eq!(width(&engine, node), expected);
    }
}

#[test]
fn native_duplicate_keyframes_keep_the_outgoing_value_and_owned_underlying_style() {
    let (mut engine, node) = native_engine();
    let id = engine
        .animate(
            node,
            frames(&[(0.0, 10.0), (0.5, 20.0), (0.5, 80.0), (1.0, 100.0)]),
            options(Easing::Linear),
            AnimationTimeline::Document,
        )
        .unwrap();
    engine.set_animation_time(50.0).unwrap();
    assert_eq!(width(&engine, node), 80.0);
    engine.cancel_animation(id).unwrap();
    assert_eq!(width(&engine, node), 60.0);
    assert_eq!(engine.active_animation_count(), 0);
}
