//! Native effect-stack ownership and authored-style mutation guards.
use openui_dom::ElementTag;
use openui_engine::{
    AnimationEventKind, AnimationId, AnimationTimeline, Engine, NodeHandle, ViewportMetrics,
};
use openui_style::{
    AnimationOptions, CompositeOperation, FillMode, IterationCount, Keyframes, LengthValue,
    PropertyKeyframes, RendererStyleValue, StyleProperty, StyleValue,
};

fn engine() -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(160.0, 120.0, 1.0).unwrap()).unwrap()
}
fn node(engine: &mut Engine) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    node
}
fn options(operation: CompositeOperation) -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        composite: operation,
        ..AnimationOptions::default()
    }
}
fn constant(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: StyleValue,
    operation: CompositeOperation,
) -> AnimationId {
    engine
        .animate(
            node,
            PropertyKeyframes::typed(property, Keyframes::from_values(value.clone(), value))
                .unwrap(),
            options(operation),
            AnimationTimeline::Document,
        )
        .unwrap()
}
fn width(engine: &Engine, node: NodeHandle) -> f32 {
    engine.computed_style(node).unwrap().width.value()
}
fn set_width(engine: &mut Engine, node: NodeHandle, value: f32) {
    engine
        .set_property(node, StyleProperty::Width, LengthValue::px(value).into())
        .unwrap();
}

#[test]
fn cancel_top_replace_reveals_unchanged_survivor_and_current_authored_value() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_property(n, StyleProperty::Opacity, 0.25_f32.into())
        .unwrap();
    let lower = constant(
        &mut e,
        n,
        StyleProperty::Opacity,
        0.5_f32.into(),
        CompositeOperation::Replace,
    );
    let upper = constant(
        &mut e,
        n,
        StyleProperty::Opacity,
        0.75_f32.into(),
        CompositeOperation::Replace,
    );
    e.set_property(n, StyleProperty::Opacity, 0.125_f32.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.75);
    e.cancel_animation(upper).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.5);
    e.cancel_animation(lower).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.125);
}

#[test]
fn cancel_lower_replace_keeps_upper_and_does_no_frame_work() {
    let mut e = engine();
    let n = node(&mut e);
    let lower = constant(
        &mut e,
        n,
        StyleProperty::Opacity,
        0.25_f32.into(),
        CompositeOperation::Replace,
    );
    let upper = constant(
        &mut e,
        n,
        StyleProperty::Opacity,
        0.5_f32.into(),
        CompositeOperation::Replace,
    );
    let before = e.scene().unwrap();
    let stats = e.stats();
    let generations = e.dirty_generations();
    e.drain_animation_events();
    e.cancel_animation(lower).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.5);
    assert_eq!(e.scene().unwrap().generation(), before.generation());
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), generations);
    let events = e.drain_animation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].animation, lower);
    assert_eq!(events[0].kind, AnimationEventKind::Cancel);
    e.cancel_animation(upper).unwrap();
}

#[test]
fn additive_effects_fold_over_all_lower_effects_and_latest_authored_base() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 10.0);
    let a = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(5.0).into(),
        CompositeOperation::Add,
    );
    let b = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(7.0).into(),
        CompositeOperation::Add,
    );
    assert_eq!(width(&e, n), 22.0);
    set_width(&mut e, n, 20.0);
    assert_eq!(width(&e, n), 32.0);
    e.cancel_animation(a).unwrap();
    assert_eq!(width(&e, n), 27.0);
    e.cancel_animation(b).unwrap();
    assert_eq!(width(&e, n), 20.0);
}

#[test]
fn replacement_resets_lower_stack_then_addition_uses_that_result() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 10.0);
    let hidden = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(100.0).into(),
        CompositeOperation::Add,
    );
    let replacement = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(20.0).into(),
        CompositeOperation::Replace,
    );
    let add = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(5.0).into(),
        CompositeOperation::Add,
    );
    assert_eq!(width(&e, n), 25.0);
    e.cancel_animation(replacement).unwrap();
    assert_eq!(width(&e, n), 115.0);
    e.cancel_animation(hidden).unwrap();
    assert_eq!(width(&e, n), 15.0);
    e.cancel_animation(add).unwrap();
    assert_eq!(width(&e, n), 10.0);
}

#[test]
fn effect_accumulate_does_not_repeat_with_iteration_index() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 10.0);
    let mut opts = options(CompositeOperation::Accumulate);
    opts.iterations = IterationCount::Number(3.0);
    let id = e
        .animate(
            n,
            PropertyKeyframes::typed(
                StyleProperty::Width,
                Keyframes::from_values(LengthValue::px(5.0), LengthValue::px(5.0)),
            )
            .unwrap(),
            opts,
            AnimationTimeline::Document,
        )
        .unwrap();
    e.set_animation_time(150.0).unwrap();
    assert_eq!(e.animation_state(id).unwrap().iteration, 1);
    assert_eq!(width(&e, n), 15.0);
    e.set_animation_time(250.0).unwrap();
    assert_eq!(width(&e, n), 15.0);
}

#[test]
fn numeric_stack_is_clamped_once_after_all_effects() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_property(n, StyleProperty::Opacity, 1.0_f32.into())
        .unwrap();
    constant(
        &mut e,
        n,
        StyleProperty::Opacity,
        1.0_f32.into(),
        CompositeOperation::Add,
    );
    constant(
        &mut e,
        n,
        StyleProperty::Opacity,
        (-1.0_f32).into(),
        CompositeOperation::Add,
    );
    assert_eq!(e.computed_style(n).unwrap().opacity, 1.0);
}

#[test]
fn a_no_fill_endpoint_reveals_a_constant_lower_effect() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 10.0);
    let lower = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(20.0).into(),
        CompositeOperation::Replace,
    );
    let mut opts = options(CompositeOperation::Replace);
    opts.duration_ms = 50.0;
    opts.fill = FillMode::None;
    let upper = e
        .animate(
            n,
            PropertyKeyframes::typed(
                StyleProperty::Width,
                Keyframes::from_values(LengthValue::px(30.0), LengthValue::px(40.0)),
            )
            .unwrap(),
            opts,
            AnimationTimeline::Document,
        )
        .unwrap();
    e.set_animation_time(50.0).unwrap();
    assert_eq!(width(&e, n), 20.0);
    e.seek_animation(upper, 25.0).unwrap();
    assert_eq!(width(&e, n), 35.0);
    e.finish_animation(upper).unwrap();
    assert_eq!(width(&e, n), 20.0);
    e.cancel_animation(lower).unwrap();
    assert_eq!(width(&e, n), 10.0);
}

#[test]
fn seek_pause_rate_and_finish_recompose_without_duplicate_events() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 10.0);
    constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(5.0).into(),
        CompositeOperation::Add,
    );
    let upper = e
        .animate(
            n,
            PropertyKeyframes::typed(
                StyleProperty::Width,
                Keyframes::from_values(LengthValue::px(0.0), LengthValue::px(10.0)),
            )
            .unwrap(),
            options(CompositeOperation::Add),
            AnimationTimeline::Document,
        )
        .unwrap();
    e.pause_animation(upper).unwrap();
    e.seek_animation(upper, 50.0).unwrap();
    assert_eq!(width(&e, n), 20.0);
    e.set_animation_playback_rate(upper, -1.0).unwrap();
    assert_eq!(width(&e, n), 20.0);
    e.drain_animation_events();
    let before = e.scene().unwrap();
    let stats = e.stats();
    e.set_animation_time(0.0).unwrap();
    assert_eq!(e.scene().unwrap().generation(), before.generation());
    assert_eq!(e.stats(), stats);
    assert!(e.drain_animation_events().is_empty());
    e.finish_animation(upper).unwrap();
    assert_eq!(width(&e, n), 15.0);
}

#[test]
fn relative_authored_base_recomputes_after_parent_font_changes() {
    let mut e = engine();
    let n = node(&mut e);
    let root = e.root();
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(10.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Width, LengthValue::Em(2.0).into())
        .unwrap();
    let add = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(2.0).into(),
        CompositeOperation::Add,
    );
    assert_eq!(width(&e, n), 22.0);
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
        .unwrap();
    assert_eq!(width(&e, n), 42.0);
    e.cancel_animation(add).unwrap();
    assert_eq!(width(&e, n), 40.0);
}

#[test]
fn own_font_effect_recomputes_authored_relative_base_and_other_effect_endpoints() {
    let mut e = engine();
    let n = node(&mut e);
    let root = e.root();
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(10.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Width, LengthValue::Em(2.0).into())
        .unwrap();
    let font = constant(
        &mut e,
        n,
        StyleProperty::FontSize,
        LengthValue::px(20.0).into(),
        CompositeOperation::Replace,
    );
    let add = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::Em(1.0).into(),
        CompositeOperation::Add,
    );
    assert_eq!(width(&e, n), 60.0);
    e.cancel_animation(font).unwrap();
    assert_eq!(width(&e, n), 30.0);
    e.cancel_animation(add).unwrap();
    assert_eq!(width(&e, n), 20.0);
}

#[test]
fn authored_transition_destination_survives_cancellation_beneath_replacement() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 10.0);
    let lower = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(30.0).into(),
        CompositeOperation::Replace,
    );
    let transition = e
        .transition(
            n,
            StyleProperty::Width,
            LengthValue::px(60.0),
            options(CompositeOperation::Replace),
        )
        .unwrap();
    assert_eq!(width(&e, n), 30.0);
    e.cancel_animation(transition).unwrap();
    assert_eq!(width(&e, n), 30.0);
    e.cancel_animation(lower).unwrap();
    assert_eq!(width(&e, n), 60.0);
}

#[test]
fn resolved_snapshot_clone_and_detach_restore_the_unanimated_source() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_renderer_style(
        n,
        RendererStyleValue::Width(openui_geometry::Length::px(20.0)),
    )
    .unwrap();
    constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(50.0).into(),
        CompositeOperation::Replace,
    );
    let clone = e.clone_subtree(n).unwrap();
    assert_eq!(width(&e, clone), 20.0);
    assert_eq!(width(&e, n), 50.0);
    e.detach(n).unwrap();
    assert_eq!(width(&e, n), 20.0);
    assert_eq!(e.object_counts().animations, 0);
}

#[test]
fn renderer_snapshot_mutation_updates_underlying_without_erasing_active_effects() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_renderer_style(
        n,
        RendererStyleValue::Width(openui_geometry::Length::px(20.0)),
    )
    .unwrap();
    let add = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(5.0).into(),
        CompositeOperation::Add,
    );
    e.set_renderer_style(
        n,
        RendererStyleValue::Width(openui_geometry::Length::px(40.0)),
    )
    .unwrap();
    assert_eq!(width(&e, n), 45.0);
    e.cancel_animation(add).unwrap();
    assert_eq!(width(&e, n), 40.0);
}

#[test]
fn removed_animation_owners_do_not_leak_into_reused_handles() {
    let mut e = engine();
    let n = node(&mut e);
    set_width(&mut e, n, 20.0);
    let id = constant(
        &mut e,
        n,
        StyleProperty::Width,
        LengthValue::px(5.0).into(),
        CompositeOperation::Add,
    );
    e.remove(n).unwrap();
    assert_eq!(e.object_counts().animations, 0);
    assert!(e.animation_state(id).is_err());
    let fresh = node(&mut e);
    assert!(e.computed_style(fresh).unwrap().width.is_auto());
    set_width(&mut e, fresh, 30.0);
    assert_eq!(width(&e, fresh), 30.0);
}

#[test]
fn color_effect_stack_keeps_premultiplied_values_until_final_resolution() {
    use openui_style::Color;
    let mut e = engine();
    let n = node(&mut e);
    let authored = Color::from_rgba_f32(1.0, 0.0, 0.0, 0.75);
    e.set_property(n, StyleProperty::BackgroundColor, authored.into())
        .unwrap();
    let blue = constant(
        &mut e,
        n,
        StyleProperty::BackgroundColor,
        Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5).into(),
        CompositeOperation::Add,
    );
    let green = constant(
        &mut e,
        n,
        StyleProperty::BackgroundColor,
        Color::from_rgba_f32(0.0, 1.0, 0.0, 0.25).into(),
        CompositeOperation::Accumulate,
    );
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(0.75, 0.25, 0.5, 1.0)
    );
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5).into(),
    )
    .unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(2.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 0.75)
    );
    e.cancel_animation(blue).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(2.0 / 3.0, 1.0 / 3.0, 0.0, 0.75)
    );
    e.cancel_animation(green).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
    );
}

#[test]
fn equal_translucent_color_alpha_uses_the_pinned_composite_branch() {
    use openui_style::Color;
    let mut e = engine();
    let n = node(&mut e);
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5).into(),
    )
    .unwrap();
    let blue = constant(
        &mut e,
        n,
        StyleProperty::BackgroundColor,
        Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5).into(),
        CompositeOperation::Add,
    );
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(1.0, 0.0, 1.0, 0.5)
    );
    e.cancel_animation(blue).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
    );
}

#[test]
fn color_interpolation_is_premultiplied_before_upper_addition() {
    use openui_style::Color;
    let mut e = engine();
    let n = node(&mut e);
    let lower = e
        .animate(
            n,
            PropertyKeyframes::typed(
                StyleProperty::BackgroundColor,
                Keyframes::from_values(Color::RED, Color::from_rgba_f32(0.0, 0.0, 1.0, 0.0)),
            )
            .unwrap(),
            options(CompositeOperation::Replace),
            AnimationTimeline::Document,
        )
        .unwrap();
    e.set_animation_time(50.0).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
    );
    let upper = constant(
        &mut e,
        n,
        StyleProperty::BackgroundColor,
        Color::from_rgba_f32(0.0, 1.0, 0.0, 0.25).into(),
        CompositeOperation::Add,
    );
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(2.0 / 3.0, 1.0 / 3.0, 0.0, 0.75)
    );
    e.cancel_animation(upper).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().background_color,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
    );
    e.cancel_animation(lower).unwrap();
}

#[test]
fn integer_animation_rounds_after_fractional_lower_effects_are_composed() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_property(n, StyleProperty::ZIndex, 0_i32.into())
        .unwrap();
    let frames =
        PropertyKeyframes::typed(StyleProperty::ZIndex, Keyframes::from_values(0_i32, 1_i32))
            .unwrap();
    let a = e
        .animate(
            n,
            frames.clone(),
            options(CompositeOperation::Add),
            AnimationTimeline::Document,
        )
        .unwrap();
    let b = e
        .animate(
            n,
            frames,
            options(CompositeOperation::Add),
            AnimationTimeline::Document,
        )
        .unwrap();
    e.set_animation_time(50.0).unwrap();
    assert_eq!(e.computed_style(n).unwrap().z_index, Some(1));
    e.set_animation_time(25.0).unwrap();
    assert_eq!(e.computed_style(n).unwrap().z_index, Some(1));
    e.cancel_animation(b).unwrap();
    assert_eq!(e.computed_style(n).unwrap().z_index, Some(0));
    e.cancel_animation(a).unwrap();
    assert_eq!(e.computed_style(n).unwrap().z_index, Some(0));
}

#[test]
fn shorthand_and_longhand_effects_share_the_same_lower_component() {
    use openui_style::{Edges, Gap};
    let mut e = engine();
    let n = node(&mut e);
    let margin = constant(
        &mut e,
        n,
        StyleProperty::Margin,
        StyleValue::Edges(Edges::all(LengthValue::px(10.0))),
        CompositeOperation::Add,
    );
    let left = constant(
        &mut e,
        n,
        StyleProperty::MarginLeft,
        LengthValue::px(5.0).into(),
        CompositeOperation::Add,
    );
    assert_eq!(e.computed_style(n).unwrap().margin_left.value(), 15.0);
    e.set_property(n, StyleProperty::MarginLeft, LengthValue::px(2.0).into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().margin_left.value(), 17.0);
    assert_eq!(e.computed_style(n).unwrap().margin_top.value(), 10.0);
    e.cancel_animation(margin).unwrap();
    assert_eq!(e.computed_style(n).unwrap().margin_left.value(), 7.0);
    assert_eq!(e.computed_style(n).unwrap().margin_top.value(), 0.0);
    e.cancel_animation(left).unwrap();
    assert_eq!(e.computed_style(n).unwrap().margin_left.value(), 2.0);
    let gap = constant(
        &mut e,
        n,
        StyleProperty::Gap,
        StyleValue::Gap(Gap {
            row: LengthValue::px(3.0),
            column: LengthValue::px(4.0),
        }),
        CompositeOperation::Replace,
    );
    let row = constant(
        &mut e,
        n,
        StyleProperty::RowGap,
        LengthValue::px(2.0).into(),
        CompositeOperation::Add,
    );
    assert_eq!(e.computed_style(n).unwrap().row_gap.unwrap().value(), 5.0);
    assert_eq!(
        e.computed_style(n).unwrap().column_gap.unwrap().value(),
        4.0
    );
    e.cancel_animation(gap).unwrap();
    assert_eq!(e.computed_style(n).unwrap().row_gap.unwrap().value(), 2.0);
    e.cancel_animation(row).unwrap();
}

#[test]
fn constant_decimal_color_keeps_native_float_bits_while_effect_owned() {
    use openui_style::Color;
    let mut e = engine();
    let n = node(&mut e);
    let color = Color::from_rgba_f32(0.1, 0.2, 0.3, 0.1);
    e.set_property(n, StyleProperty::BackgroundColor, color.into())
        .unwrap();
    let effect = constant(
        &mut e,
        n,
        StyleProperty::BackgroundColor,
        color.into(),
        CompositeOperation::Replace,
    );
    for time in [0.0, 25.0, 50.0, 100.0] {
        e.set_animation_time(time).unwrap();
        assert_eq!(e.computed_style(n).unwrap().background_color, color);
    }
    let current = Color::from_rgba_f32(0.3, 0.2, 0.1, 0.25);
    e.set_property(n, StyleProperty::BackgroundColor, current.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().background_color, color);
    e.cancel_animation(effect).unwrap();
    assert_eq!(e.computed_style(n).unwrap().background_color, current);
}
