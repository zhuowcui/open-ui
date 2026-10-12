//! Native author, computed renderer and endpoint forms use the same effect fold.
use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    AnimationOptions, Color, CompositeOperation, FillMode, FontStretch, Keyframes, Length,
    LengthValue, PropertyKeyframes, RendererStyleValue, StyleColor, StyleProperty, StyleValue,
};

fn fixture(scale: f64) -> (Engine, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(160.0, 120.0, scale).unwrap()).unwrap();
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    engine
        .set_property(node, StyleProperty::Width, LengthValue::px(10.0).into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Height, LengthValue::px(20.0).into())
        .unwrap();
    (engine, node)
}
fn options(composite: CompositeOperation) -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        composite,
        ..AnimationOptions::default()
    }
}
fn animate(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    from: StyleValue,
    to: StyleValue,
    composite: CompositeOperation,
) -> AnimationId {
    engine
        .animate(
            node,
            PropertyKeyframes::typed(property, Keyframes::from_values(from, to)).unwrap(),
            options(composite),
            AnimationTimeline::Document,
        )
        .unwrap()
}

#[test]
fn opacity_add_folds_computed_underlying_and_current_native_author_mutation() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, node) = fixture(scale);
        engine
            .set_property(node, StyleProperty::Opacity, 0.25_f32.into())
            .unwrap();
        let value = StyleValue::Renderer(RendererStyleValue::Opacity(0.375));
        let id = animate(
            &mut engine,
            node,
            StyleProperty::Opacity,
            value.clone(),
            value,
            CompositeOperation::Add,
        );
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.625);
        engine
            .set_property(node, StyleProperty::Opacity, 0.5_f32.into())
            .unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.875);
        engine.cancel_animation(id).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.5);
    }
}

#[test]
fn width_add_and_accumulate_mix_native_and_renderer_endpoints_without_losing_base() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, node) = fixture(scale);
        let renderer = StyleValue::Renderer(RendererStyleValue::Width(Length::fixed(5.0)));
        let lower = animate(
            &mut engine,
            node,
            StyleProperty::Width,
            renderer.clone(),
            renderer,
            CompositeOperation::Add,
        );
        let native = StyleValue::from(LengthValue::px(7.0));
        let upper = animate(
            &mut engine,
            node,
            StyleProperty::Width,
            native.clone(),
            native,
            CompositeOperation::Accumulate,
        );
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 22.0);
        assert_eq!(engine.bounds(node).unwrap().unwrap().width, 22.0);
        engine.cancel_animation(lower).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 17.0);
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(20.0).into())
            .unwrap();
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 27.0);
        engine.cancel_animation(upper).unwrap();
        assert_eq!(engine.bounds(node).unwrap().unwrap().width, 20.0);
    }
}

#[test]
fn native_renderer_number_endpoints_share_interpolation_and_endpoint_composition() {
    for operation in [
        CompositeOperation::Replace,
        CompositeOperation::Add,
        CompositeOperation::Accumulate,
    ] {
        let (mut engine, node) = fixture(1.25);
        engine
            .set_property(node, StyleProperty::Opacity, 0.25_f32.into())
            .unwrap();
        let id = animate(
            &mut engine,
            node,
            StyleProperty::Opacity,
            0.25_f32.into(),
            StyleValue::Renderer(RendererStyleValue::Opacity(0.75)),
            operation,
        );
        engine.seek_animation(id, 25.0).unwrap();
        let base = if operation == CompositeOperation::Replace {
            0.0
        } else {
            0.25
        };
        assert_eq!(engine.computed_style(node).unwrap().opacity, base + 0.375);
        engine.seek_animation(id, 0.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, base + 0.25);
        engine.seek_animation(id, 100.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, base + 0.75);
    }
}

#[test]
fn resolved_renderer_colors_compose_against_current_author_and_restore_on_cancel() {
    let (mut engine, node) = fixture(1.0);
    let red = Color::from_rgba_f32(0.25, 0.0, 0.0, 0.5);
    let blue = Color::from_rgba_f32(0.0, 0.0, 0.25, 0.5);
    engine
        .set_property(node, StyleProperty::BackgroundColor, red.into())
        .unwrap();
    let value = StyleValue::Renderer(RendererStyleValue::BackgroundColor(blue));
    let id = animate(
        &mut engine,
        node,
        StyleProperty::BackgroundColor,
        value.clone(),
        value,
        CompositeOperation::Add,
    );
    assert_eq!(
        engine.computed_style(node).unwrap().background_color,
        Color::from_rgba_f32(0.25, 0.0, 0.25, 0.5)
    );
    let green = Color::from_rgba_f32(0.0, 0.25, 0.0, 0.5);
    engine
        .set_renderer_style(node, RendererStyleValue::BackgroundColor(green))
        .unwrap();
    assert_eq!(
        engine.computed_style(node).unwrap().background_color,
        Color::from_rgba_f32(0.0, 0.25, 0.25, 0.5)
    );
    engine.cancel_animation(id).unwrap();
    assert_eq!(engine.computed_style(node).unwrap().background_color, green);
}

#[test]
fn resolved_stylecolor_underlying_and_native_color_endpoint_have_one_color_stack() {
    let (mut engine, node) = fixture(1.0);
    let red = Color::from_rgba_f32(0.25, 0.0, 0.0, 0.5);
    let blue = Color::from_rgba_f32(0.0, 0.0, 0.25, 0.5);
    engine
        .set_renderer_style(
            node,
            RendererStyleValue::BorderTopColor(StyleColor::Resolved(red)),
        )
        .unwrap();
    let id = animate(
        &mut engine,
        node,
        StyleProperty::BorderTopColor,
        blue.into(),
        blue.into(),
        CompositeOperation::Accumulate,
    );
    assert_eq!(
        engine
            .computed_style(node)
            .unwrap()
            .border_top_color
            .resolve(&Color::BLACK),
        Color::from_rgba_f32(0.25, 0.0, 0.25, 0.5)
    );
    engine.cancel_animation(id).unwrap();
    assert_eq!(
        engine.computed_style(node).unwrap().border_top_color,
        StyleColor::Resolved(red)
    );
}

#[test]
fn typography_renderer_form_reaches_existing_native_typed_sampler() {
    let (mut engine, node) = fixture(1.0);
    let id = animate(
        &mut engine,
        node,
        StyleProperty::FontStretch,
        FontStretch(100.0).into(),
        StyleValue::Renderer(RendererStyleValue::FontStretch(FontStretch(150.0))),
        CompositeOperation::Replace,
    );
    engine.seek_animation(id, 50.0).unwrap();
    assert_eq!(
        engine.computed_style(node).unwrap().font_stretch,
        FontStretch(125.0)
    );
}

#[test]
fn optional_auto_underlying_remains_owned_and_count_publication_uses_pinned_bounds() {
    let (mut engine, node) = fixture(1.0);
    assert_eq!(engine.computed_style(node).unwrap().z_index, None);
    let id = animate(
        &mut engine,
        node,
        StyleProperty::ZIndex,
        3_i32.into(),
        StyleValue::Renderer(RendererStyleValue::ZIndex(Some(3))),
        CompositeOperation::Add,
    );
    assert_eq!(engine.computed_style(node).unwrap().z_index, Some(3));
    engine.cancel_animation(id).unwrap();
    assert_eq!(engine.computed_style(node).unwrap().z_index, None);
    for (property, value, expected) in [
        (
            StyleProperty::Orphans,
            RendererStyleValue::Orphans(40_000),
            32_767,
        ),
        (
            StyleProperty::Widows,
            RendererStyleValue::Widows(40_000),
            32_767,
        ),
        (
            StyleProperty::ColumnCount,
            RendererStyleValue::ColumnCount(Some(70_000)),
            65_535,
        ),
    ] {
        let frames = PropertyKeyframes::typed(
            property,
            Keyframes::from_values(StyleValue::Integer(1), StyleValue::Renderer(value)),
        )
        .unwrap();
        assert_eq!(
            frames.sample(1.0, &openui_style::Easing::Linear),
            StyleValue::Integer(expected)
        );
    }
}

#[test]
fn settled_mixed_form_stack_preserves_unchanged_frame_and_cancel_survivor_zero_work() {
    let (mut engine, node) = fixture(1.0);
    let lower = animate(
        &mut engine,
        node,
        StyleProperty::Opacity,
        0.25_f32.into(),
        0.25_f32.into(),
        CompositeOperation::Replace,
    );
    let upper = animate(
        &mut engine,
        node,
        StyleProperty::Opacity,
        StyleValue::Renderer(RendererStyleValue::Opacity(0.5)),
        0.5_f32.into(),
        CompositeOperation::Replace,
    );
    let scene = engine.scene().unwrap();
    let stats = engine.stats();
    let generations = engine.dirty_generations();
    engine.scene().unwrap();
    assert_eq!(engine.stats(), stats);
    engine.cancel_animation(lower).unwrap();
    assert_eq!(engine.scene().unwrap().generation(), scene.generation());
    assert_eq!(engine.stats(), stats);
    assert_eq!(engine.dirty_generations(), generations);
    engine.cancel_animation(upper).unwrap();
    assert_eq!(engine.computed_style(node).unwrap().opacity, 1.0);
}
