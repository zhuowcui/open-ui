use openui_dom::ElementTag;
use openui_engine::{AnimationEventKind, AnimationId, AnimationTimeline, Engine, NodeHandle};
use openui_geometry::ViewportMetrics;
use openui_style::{
    AnimationOptions, FillMode, IterationCount, Keyframes, PlayState, PlaybackDirection,
    PropertyKeyframes, StyleProperty,
};

fn options() -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
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
fn fixture(options: AnimationOptions) -> (Engine, NodeHandle, AnimationId) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(100.0, 80.0, 1.0).unwrap()).unwrap();
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    let id = engine
        .animate(node, frames(), options, AnimationTimeline::Document)
        .unwrap();
    (engine, node, id)
}
fn value(engine: &Engine, node: NodeHandle) -> f32 {
    engine.computed_style(node).unwrap().opacity
}

// Exact .50 Blink animation.cc:644-666,1024-1102,1253-1261,2288-2311.
#[test]
fn rate_changes_and_non_unit_seeks_preserve_animation_local_time() {
    let (mut engine, node, id) = fixture(options());
    engine.set_animation_time(25.0).unwrap();
    let generation = engine.scene().unwrap().generation();
    engine.set_animation_playback_rate(id, 2.0).unwrap();
    assert_eq!(value(&engine, node), 0.25);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 25.0);
    assert_eq!(engine.scene().unwrap().generation(), generation);
    engine.set_animation_time(40.0).unwrap();
    assert_eq!(value(&engine, node), 0.55);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 55.0);
    engine.seek_animation(id, 80.0).unwrap();
    assert_eq!(value(&engine, node), 0.8);
    engine.set_animation_time(45.0).unwrap();
    assert_eq!(value(&engine, node), 0.9);
    engine.pause_animation(id).unwrap();
    engine.set_animation_time(100.0).unwrap();
    engine.seek_animation(id, 80.0).unwrap();
    let generation = engine.scene().unwrap().generation();
    engine.set_animation_playback_rate(id, -2.0).unwrap();
    assert_eq!(value(&engine, node), 0.8);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 80.0);
    assert_eq!(engine.scene().unwrap().generation(), generation);
    assert!(!engine.is_animating());
    engine.play_animation(id).unwrap();
    assert_eq!(value(&engine, node), 0.8);
    engine.set_animation_time(110.0).unwrap();
    assert_eq!(value(&engine, node), 0.6);
    // .50: start = .1 - .08 / -2; current = (.11 - start) * -2.
    // The old additive-offset route differs by two ULPs here.
    assert_eq!(
        engine
            .animation_state(id)
            .unwrap()
            .current_time_ms
            .to_bits(),
        0x404e_0000_0000_0004
    );
}

#[test]
fn zero_rate_holds_the_picture_and_resumes_without_time_or_work_jumps() {
    let (mut engine, node, id) = fixture(options());
    engine.set_animation_time(40.0).unwrap();
    engine.drain_animation_events();
    engine.set_animation_playback_rate(id, 0.0).unwrap();
    let generation = engine.scene().unwrap().generation();
    assert!(!engine.is_animating());
    engine.set_animation_time(1000.0).unwrap();
    assert_eq!(value(&engine, node), 0.4);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 40.0);
    assert_eq!(engine.scene().unwrap().generation(), generation);
    assert!(engine.drain_animation_events().is_empty());
    assert!(engine.finish_animation(id).is_err());
    assert_eq!(value(&engine, node), 0.4);
    engine.seek_animation(id, 70.0).unwrap();
    assert_eq!(value(&engine, node), 0.7);
    engine.set_animation_playback_rate(id, 2.0).unwrap();
    assert_eq!(value(&engine, node), 0.7);
    engine.set_animation_time(1010.0).unwrap();
    assert_eq!(value(&engine, node), 0.9);
    let (mut zero, node, id) = fixture(AnimationOptions {
        playback_rate: 0.0,
        ..options()
    });
    zero.set_animation_time(1000.0).unwrap();
    assert_eq!(value(&zero, node), 0.0);
    assert_eq!(zero.animation_state(id).unwrap().current_time_ms, 0.0);
    assert!(!zero.is_animating());
}

// Blink finish1889-1911 and finished holding1954-2011 use local, directional endpoints.
#[test]
fn natural_and_explicit_finish_hold_directional_endpoints_and_play_rewinds() {
    let (mut engine, node, id) = fixture(AnimationOptions {
        playback_rate: -2.0,
        ..options()
    });
    assert_eq!(value(&engine, node), 1.0);
    assert!(!engine.animation_state(id).unwrap().finished);
    engine.drain_animation_events();
    engine.set_animation_time(20.0).unwrap();
    assert_eq!(value(&engine, node), 0.6);
    engine.set_animation_time(80.0).unwrap();
    assert_eq!(value(&engine, node), 0.0);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
    assert!(engine.animation_state(id).unwrap().finished);
    assert!(!engine.is_animating());
    let events = engine.drain_animation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, AnimationEventKind::End);
    assert_eq!(events[0].elapsed_time_ms, 0.0);
    engine.set_animation_time(200.0).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
    assert!(engine.drain_animation_events().is_empty());
    engine.play_animation(id).unwrap();
    // The synchronous native play has resolved its ready task. Pinned .50
    // CommitPendingPlay clears the reverse end hold, and CalculateCurrentTime
    // evaluates (0.2 - (0.2 - 0.1 / -2.0)) * -2.0 before reflecting ms.
    // Pending browser play holds 100 exactly; resolved play remains raw.
    let rewound = engine.animation_state(id).unwrap();
    assert_eq!(rewound.current_time_ms.to_bits(), 0x4058_ffff_ffff_fffe);
    assert_eq!(rewound.play_state, PlayState::Running);
    assert!(!rewound.finished);
    assert!(engine.is_animating());
    engine.set_animation_time(210.0).unwrap();
    assert_eq!(value(&engine, node), 0.8);
    engine.pause_animation(id).unwrap();
    engine.finish_animation(id).unwrap();
    assert_eq!(value(&engine, node), 0.0);
    assert!(engine.animation_state(id).unwrap().finished);
    assert_eq!(
        engine.animation_state(id).unwrap().play_state,
        PlayState::Running
    );
    engine.set_animation_playback_rate(id, 1.0).unwrap();
    engine.seek_animation(id, 150.0).unwrap();
    engine.set_animation_time(1000.0).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 150.0);
    engine.play_animation(id).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
    engine.set_animation_time(1200.0).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 100.0);
}

#[test]
fn infinite_reverse_finish_and_invalid_controls_have_transactional_limits() {
    let (mut engine, node, id) = fixture(AnimationOptions {
        iterations: IterationCount::Infinite,
        ..options()
    });
    engine.set_animation_time(225.0).unwrap();
    engine.drain_animation_events();
    let generation = engine.scene().unwrap().generation();
    assert!(engine.finish_animation(id).is_err());
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(engine.set_animation_playback_rate(id, invalid).is_err());
        assert!(engine.seek_animation(id, invalid).is_err());
    }
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 225.0);
    assert_eq!(engine.scene().unwrap().generation(), generation);
    assert!(engine.drain_animation_events().is_empty());
    engine.set_animation_playback_rate(id, -1.0).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 225.0);
    assert_eq!(value(&engine, node), 0.25);
    engine.finish_animation(id).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
    assert_eq!(value(&engine, node), 0.0);
    assert!(engine.animation_state(id).unwrap().finished);
    let count = engine.active_animation_count();
    assert!(engine
        .animate(
            node,
            frames(),
            AnimationOptions {
                iterations: IterationCount::Infinite,
                playback_rate: -1.0,
                ..options()
            },
            AnimationTimeline::Document
        )
        .is_err());
    assert_eq!(engine.active_animation_count(), count);
    assert!(engine.play_animation(id).is_err());
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 0.0);
}

#[test]
fn lifecycle_uses_effect_intervals_and_one_event_for_skipped_iterations_in_both_directions() {
    let (mut engine, node, id) = fixture(AnimationOptions {
        delay_ms: -20.0,
        iterations: IterationCount::Number(10.0),
        ..options()
    });
    let events = engine.drain_animation_events();
    assert_eq!(events[0].kind, AnimationEventKind::Start);
    assert_eq!(events[0].elapsed_time_ms, 20.0);
    engine.seek_animation(id, 330.0).unwrap();
    let events = engine.drain_animation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, AnimationEventKind::Iteration);
    // .50 normalized iteration duration is .1 seconds, multiplied by
    // boundary 3 before the native millisecond reflection.
    assert_eq!(events[0].elapsed_time_ms.to_bits(), 0x4072_c000_0000_0001);
    engine.set_animation_playback_rate(id, -1.0).unwrap();
    engine.seek_animation(id, 130.0).unwrap();
    let events = engine.drain_animation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, AnimationEventKind::Iteration);
    assert_eq!(events[0].elapsed_time_ms, 200.0);
    engine
        .set_property(node, StyleProperty::Opacity, 0.75_f32.into())
        .unwrap();
    engine.cancel_animation(id).unwrap();
    let events = engine.drain_animation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, AnimationEventKind::Cancel);
    assert_eq!(events[0].elapsed_time_ms, 150.0);
    assert_eq!(engine.active_animation_count(), 0);
}

#[test]
fn negative_delays_fractional_iterations_and_zero_duration_use_local_effect_end() {
    let (mut engine, node, id) = fixture(AnimationOptions {
        delay_ms: -25.0,
        iterations: IterationCount::Number(2.5),
        playback_rate: -1.0,
        ..options()
    });
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 225.0);
    assert_eq!(value(&engine, node), 0.5);
    engine.finish_animation(id).unwrap();
    assert_eq!(value(&engine, node), 0.25);
    let (zero, _, id) = fixture(AnimationOptions {
        duration_ms: 0.0,
        iterations: IterationCount::Infinite,
        ..options()
    });
    assert!(zero.animation_state(id).unwrap().finished);
    assert_eq!(zero.animation_state(id).unwrap().current_time_ms, 0.0);
    assert!(!zero.is_animating());
}

// Exact .50 timing_calculations.cc:277-308 keeps current iteration as a
// double and treats infinity as even, independently of its public reflection.
#[test]
fn alternating_direction_uses_unsaturated_finite_and_infinite_iteration_parity() {
    for (direction, terminal, even_start, odd_quarter) in [
        (PlaybackDirection::Alternate, 1.0, 0.0, 0.75),
        (PlaybackDirection::AlternateReverse, 0.0, 1.0, 0.25),
    ] {
        let (zero, node, id) = fixture(AnimationOptions {
            duration_ms: 0.0,
            iterations: IterationCount::Infinite,
            direction,
            ..options()
        });
        assert_eq!(value(&zero, node), terminal);
        assert_eq!(zero.animation_state(id).unwrap().iteration, u64::MAX);
        let (mut finite, node, id) = fixture(AnimationOptions {
            duration_ms: 1.0,
            iterations: IterationCount::Number(2_f64.powi(66)),
            direction,
            ..options()
        });
        finite.seek_animation(id, 2_f64.powi(65)).unwrap();
        assert_eq!(finite.animation_state(id).unwrap().iteration, u64::MAX);
        assert_eq!(value(&finite, node), even_start);
        finite.seek_animation(id, 1.25).unwrap();
        assert_eq!(value(&finite, node), odd_quarter);
        finite.finish_animation(id).unwrap();
        assert_eq!(value(&finite, node), terminal);
    }
}

#[test]
fn no_fill_sampling_retains_unresolved_current_direction_context() {
    for (direction, reversed) in [
        (PlaybackDirection::Alternate, true),
        (PlaybackDirection::AlternateReverse, false),
    ] {
        let timing = AnimationOptions {
            delay_ms: 10.0,
            direction,
            fill: FillMode::None,
            ..options()
        };
        for local_time in [0.0, 110.0] {
            let (sample, current_direction_is_reversed) =
                timing.sample_local_time_with_direction(local_time);
            assert_eq!(sample.progress, None);
            assert_eq!(current_direction_is_reversed, reversed);
            assert_eq!(timing.sample_local_time(local_time), sample);
        }
    }
}

// Exact .50 CommitPendingPlay and CalculateStartTime establish the anchor
// before CalculateCurrentTime multiplies the timeline difference by rate.
#[test]
fn fractional_seek_reverse_start_and_running_rate_use_authoritative_start_time() {
    let (mut engine, _node, id) = fixture(AnimationOptions {
        play_state: PlayState::Paused,
        playback_rate: 1.5,
        ..options()
    });
    engine.set_animation_time(37.5).unwrap();
    engine.seek_animation(id, 23.75).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 23.75);
    engine.play_animation(id).unwrap();
    engine.set_animation_time(52.5).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 46.25);
    engine.pause_animation(id).unwrap();
    engine.set_animation_time(500.0).unwrap();
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 46.25);

    let (mut reverse, _node, id) = fixture(AnimationOptions {
        duration_ms: 97.5,
        playback_rate: -1.5,
        ..options()
    });
    assert_eq!(reverse.animation_state(id).unwrap().current_time_ms, 97.5);
    reverse.set_animation_time(7.5).unwrap();
    // start = 0 - .0975 / -1.5; current = (.0075 - start) * -1.5.
    assert_eq!(
        reverse
            .animation_state(id)
            .unwrap()
            .current_time_ms
            .to_bits(),
        0x4055_9000_0000_0001
    );

    let (mut running, _node, id) = fixture(options());
    running.set_animation_time(47.5).unwrap();
    running.set_animation_playback_rate(id, 1.75).unwrap();
    assert_eq!(running.animation_state(id).unwrap().current_time_ms, 47.5);
    running.set_animation_time(52.5).unwrap();
    // .50 preserves currentTime's millisecond reflection before setting the
    // new start = .0475 - (.0475 * 1000 / 1000) / 1.75.
    assert_eq!(
        running
            .animation_state(id)
            .unwrap()
            .current_time_ms
            .to_bits(),
        0x404c_1fff_ffff_ffff
    );
}
