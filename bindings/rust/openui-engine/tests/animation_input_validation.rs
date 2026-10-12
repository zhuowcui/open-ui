use openui_dom::ElementTag;
use openui_engine::{AnimationEventKind, AnimationTimeline, Engine};
use openui_geometry::ViewportMetrics;
use openui_style::{
    AnimationOptions, Display, Easing, FillMode, Keyframe, Keyframes, LengthValue, LinearStop,
    Overflow, PropertyKeyframes, StepPosition, StyleProperty, TimelineAxis, TimelineRange,
};

fn malformed_curves() -> Vec<Easing> {
    vec![
        Easing::LinearStops(Vec::new()),
        Easing::LinearStops(vec![LinearStop {
            input: 0.0,
            output: 0.0,
        }]),
        Easing::LinearStops(vec![
            LinearStop {
                input: 0.0,
                output: 0.0,
            },
            LinearStop {
                input: 1.0,
                output: f64::NAN,
            },
        ]),
        Easing::Steps {
            count: 0,
            position: StepPosition::JumpEnd,
        },
        Easing::Steps {
            count: 1,
            position: StepPosition::JumpNone,
        },
        Easing::CubicBezier {
            x1: -0.1,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        },
        Easing::CubicBezier {
            x1: 0.0,
            y1: f64::INFINITY,
            x2: 1.0,
            y2: 1.0,
        },
    ]
}

fn scrolling_engine() -> (Engine, openui_engine::NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
    let scroller = engine.create_element(ElementTag::Div).unwrap();
    let content = engine.create_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), scroller).unwrap();
    engine.append_child(scroller, content).unwrap();
    for (node, property, value) in [
        (scroller, StyleProperty::Display, Display::Block.into()),
        (
            scroller,
            StyleProperty::Width,
            LengthValue::px(100.0).into(),
        ),
        (
            scroller,
            StyleProperty::Height,
            LengthValue::px(80.0).into(),
        ),
        (scroller, StyleProperty::Overflow, Overflow::Hidden.into()),
        (content, StyleProperty::Display, Display::Block.into()),
        (content, StyleProperty::Width, LengthValue::px(300.0).into()),
        (
            content,
            StyleProperty::Height,
            LengthValue::px(240.0).into(),
        ),
    ] {
        engine.set_property(node, property, value).unwrap();
    }
    engine
        .smooth_scroll_to(scroller, 100.0, 40.0, 100.0, Easing::Linear)
        .unwrap();
    (engine, scroller)
}

#[test]
fn invalid_scroll_curves_preserve_the_running_animation_and_unchanged_scene() {
    let (mut engine, scroller) = scrolling_engine();
    engine.set_animation_time(25.0).unwrap();
    let generation = engine.scene().unwrap().generation();
    assert_eq!(engine.scroll_offset(scroller).unwrap(), (25.0, 10.0));
    for easing in malformed_curves() {
        assert!(engine
            .smooth_scroll_to(scroller, 150.0, 60.0, 100.0, easing)
            .is_err());
        assert_eq!(engine.scroll_offset(scroller).unwrap(), (25.0, 10.0));
        assert_eq!(engine.scene().unwrap().generation(), generation);
        assert!(engine.is_animating());
        assert!(engine.drain_animation_events().is_empty());
    }
    engine.set_animation_time(50.0).unwrap();
    assert_eq!(engine.scroll_offset(scroller).unwrap(), (50.0, 20.0));
    engine.set_animation_time(100.0).unwrap();
    assert_eq!(engine.scroll_offset(scroller).unwrap(), (100.0, 40.0));
    assert!(!engine.is_animating());
}

#[test]
fn snap_noop_validates_timing_and_keeps_the_existing_scroll() {
    let (mut engine, scroller) = scrolling_engine();
    engine.set_animation_time(25.0).unwrap();
    let generation = engine.scene().unwrap().generation();
    for easing in malformed_curves() {
        assert!(engine
            .settle_scroll_snap(scroller, &[25.0], &[10.0], 100.0, easing)
            .is_err());
    }
    for duration in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(engine
            .settle_scroll_snap(scroller, &[25.0], &[10.0], duration, Easing::Linear)
            .is_err());
        assert!(engine
            .smooth_scroll_to(scroller, 150.0, 60.0, duration, Easing::Linear)
            .is_err());
    }
    assert_eq!(
        engine
            .settle_scroll_snap(scroller, &[25.0], &[10.0], 0.0, Easing::Linear)
            .unwrap(),
        None
    );
    assert_eq!(engine.scene().unwrap().generation(), generation);
    assert!(engine.is_animating());
    assert!(engine.drain_animation_events().is_empty());
    engine.set_animation_time(50.0).unwrap();
    assert_eq!(engine.scroll_offset(scroller).unwrap(), (50.0, 20.0));
}

#[test]
fn invalid_keyframe_easing_is_rejected_before_a_native_animation_can_start() {
    let (mut engine, _) = scrolling_engine();
    let generation = engine.scene().unwrap().generation();
    for easing in malformed_curves() {
        let keyframes = Keyframes::new(vec![
            Keyframe::new(0.0, 0.0_f32).easing(easing.clone()),
            Keyframe::new(1.0, 1.0_f32),
        ]);
        assert!(keyframes.is_err());
        let options = AnimationOptions {
            easing,
            duration_ms: 100.0,
            ..AnimationOptions::default()
        };
        let keyframes = PropertyKeyframes::typed(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
        )
        .unwrap();
        assert!(engine
            .animate(
                engine.root(),
                keyframes,
                options,
                AnimationTimeline::Document
            )
            .is_err());
        assert_eq!(engine.active_animation_count(), 0);
        assert_eq!(engine.scene().unwrap().generation(), generation);
        assert!(engine.drain_animation_events().is_empty());
    }
}

#[test]
fn valid_interval_override_remains_distinct_from_the_animation_default() {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
    let node = engine.create_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    let easing = Easing::linear_stops(vec![
        LinearStop {
            input: 0.0,
            output: 0.0,
        },
        LinearStop {
            input: 0.5,
            output: 0.25,
        },
        LinearStop {
            input: 1.0,
            output: 1.0,
        },
    ])
    .unwrap();
    let keyframes = PropertyKeyframes::typed(
        StyleProperty::Opacity,
        Keyframes::new(vec![
            Keyframe::new(0.0, 0.0_f32).easing(easing),
            Keyframe::new(1.0, 1.0_f32),
        ])
        .unwrap(),
    )
    .unwrap();
    let id = engine
        .animate(
            node,
            keyframes,
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                easing: Easing::Linear,
                ..AnimationOptions::default()
            },
            AnimationTimeline::Document,
        )
        .unwrap();
    engine.set_animation_time(50.0).unwrap();
    assert_eq!(engine.computed_style(node).unwrap().opacity, 0.25);
    engine.set_animation_time(100.0).unwrap();
    assert_eq!(engine.computed_style(node).unwrap().opacity, 1.0);
    assert!(engine.animation_state(id).unwrap().finished);
    let events = engine.drain_animation_events();
    assert_eq!(events.first().unwrap().kind, AnimationEventKind::Start);
    assert_eq!(events.last().unwrap().kind, AnimationEventKind::End);
}

#[test]
fn malformed_timeline_ranges_do_not_start_an_animation_and_reversed_ranges_work() {
    let (mut engine, scroller) = scrolling_engine();
    engine.set_animation_time(25.0).unwrap();
    let generation = engine.scene().unwrap().generation();
    for range in [
        TimelineRange {
            start: 0.0,
            end: 0.0,
        },
        TimelineRange {
            start: f64::NAN,
            end: 100.0,
        },
        TimelineRange {
            start: 0.0,
            end: f64::INFINITY,
        },
    ] {
        for timeline in [
            AnimationTimeline::Scroll {
                source: scroller,
                axis: TimelineAxis::X,
                range,
            },
            AnimationTimeline::View {
                subject: scroller,
                axis: TimelineAxis::X,
                range,
            },
        ] {
            let frames = PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
            )
            .unwrap();
            assert!(engine
                .animate(
                    engine.root(),
                    frames,
                    AnimationOptions {
                        duration_ms: 100.0,
                        fill: FillMode::Both,
                        ..AnimationOptions::default()
                    },
                    timeline
                )
                .is_err());
            assert_eq!(engine.active_animation_count(), 0);
            assert_eq!(engine.scene().unwrap().generation(), generation);
            assert!(engine.drain_animation_events().is_empty());
        }
    }
    let frames = PropertyKeyframes::typed(
        StyleProperty::Opacity,
        Keyframes::from_values(0.0_f32, 1.0_f32),
    )
    .unwrap();
    let id = engine
        .animate(
            engine.root(),
            frames,
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
            AnimationTimeline::Scroll {
                source: scroller,
                axis: TimelineAxis::X,
                range: TimelineRange {
                    start: 100.0,
                    end: 0.0,
                },
            },
        )
        .unwrap();
    assert_eq!(engine.computed_style(engine.root()).unwrap().opacity, 0.75);
    assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 75.0);
    engine.cancel_animation(id).unwrap();
    engine.set_animation_time(50.0).unwrap();
    assert_eq!(engine.scroll_offset(scroller).unwrap(), (50.0, 20.0));
}
