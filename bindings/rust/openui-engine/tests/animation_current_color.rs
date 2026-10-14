//! Shared native color capture, dependency publication and surviving-stack guards.
//! Source-only guards; exact Chromium/runtime qualification is a separate gate.
use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    AnimationOptions, Color, CompositeOperation, FillMode, Keyframes, PropertyKeyframes,
    RendererStyleValue, StyleColor, StyleProperty,
};

fn engine() -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(160.0, 120.0, 1.0).unwrap()).unwrap()
}
fn node(e: &mut Engine) -> NodeHandle {
    let node = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(e.root(), node).unwrap();
    node
}
fn constant(
    e: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    color: StyleColor,
    composite: CompositeOperation,
) -> AnimationId {
    e.animate(
        node,
        PropertyKeyframes::typed(property, Keyframes::from_values(color, color)).unwrap(),
        AnimationOptions {
            duration_ms: 100.0,
            fill: FillMode::Both,
            composite,
            ..AnimationOptions::default()
        },
        AnimationTimeline::Document,
    )
    .unwrap()
}

#[test]
fn own_currentcolor_inherits_parent_while_background_keeps_own_color_dependency() {
    let mut e = engine();
    let n = node(&mut e);
    let root = e.root();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    e.set_property(n, StyleProperty::Color, StyleColor::CurrentColor.into())
        .unwrap();
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::RED);
    assert_eq!(e.computed_style(n).unwrap().background_color, Color::RED);
    e.set_property(root, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::BLUE);
    assert_eq!(e.computed_style(n).unwrap().background_color, Color::BLUE);
    e.set_property(n, StyleProperty::Color, Color::RED.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().background_color, Color::RED);
}

#[test]
fn own_keyword_animation_captures_mutated_parent_and_cancel_restores_authored_keyword() {
    let mut e = engine();
    let n = node(&mut e);
    let root = e.root();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    e.set_property(n, StyleProperty::Color, StyleColor::CurrentColor.into())
        .unwrap();
    let lower = constant(
        &mut e,
        n,
        StyleProperty::Color,
        StyleColor::CurrentColor,
        CompositeOperation::Replace,
    );
    let upper = constant(
        &mut e,
        n,
        StyleProperty::Color,
        StyleColor::Resolved(Color::WHITE),
        CompositeOperation::Replace,
    );
    e.set_property(root, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::WHITE);
    e.cancel_animation(upper).unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::BLUE);
    e.cancel_animation(lower).unwrap();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::RED);
}

#[test]
fn final_own_color_resolves_dependent_effects_independently_of_creation_order() {
    for dependent_first in [false, true] {
        let mut e = engine();
        let n = node(&mut e);
        e.set_property(n, StyleProperty::Color, Color::BLUE.into())
            .unwrap();
        if dependent_first {
            constant(
                &mut e,
                n,
                StyleProperty::BackgroundColor,
                StyleColor::CurrentColor,
                CompositeOperation::Replace,
            );
        }
        constant(
            &mut e,
            n,
            StyleProperty::Color,
            StyleColor::Resolved(Color::RED),
            CompositeOperation::Replace,
        );
        if !dependent_first {
            constant(
                &mut e,
                n,
                StyleProperty::BackgroundColor,
                StyleColor::CurrentColor,
                CompositeOperation::Replace,
            );
        }
        assert_eq!(e.computed_style(n).unwrap().background_color, Color::RED);
        constant(
            &mut e,
            n,
            StyleProperty::OutlineColor,
            StyleColor::CurrentColor,
            CompositeOperation::Replace,
        );
        assert_eq!(
            e.computed_style(n)
                .unwrap()
                .outline_color
                .resolve(&Color::BLACK),
            Color::RED
        );
    }
}

#[test]
fn keyword_add_and_accumulate_keep_fraction_and_alpha_until_the_complete_stack() {
    for operation in [CompositeOperation::Add, CompositeOperation::Accumulate] {
        let mut e = engine();
        let n = node(&mut e);
        e.set_property(
            n,
            StyleProperty::Color,
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.25).into(),
        )
        .unwrap();
        e.set_property(
            n,
            StyleProperty::BackgroundColor,
            StyleColor::CurrentColor.into(),
        )
        .unwrap();
        let lower = constant(
            &mut e,
            n,
            StyleProperty::BackgroundColor,
            StyleColor::CurrentColor,
            operation,
        );
        assert_eq!(
            e.computed_style(n).unwrap().background_color,
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
        );
        let upper = constant(
            &mut e,
            n,
            StyleProperty::BackgroundColor,
            StyleColor::CurrentColor,
            operation,
        );
        assert_eq!(
            e.computed_style(n).unwrap().background_color,
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.75)
        );
        e.cancel_animation(lower).unwrap();
        assert_eq!(
            e.computed_style(n).unwrap().background_color,
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
        );
        e.cancel_animation(upper).unwrap();
        assert_eq!(
            e.computed_style(n).unwrap().background_color,
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.25)
        );
    }
}

#[test]
fn text_decoration_keyword_uses_final_fill_while_emphasis_uses_final_own_color() {
    let mut e = engine();
    let n = node(&mut e);
    constant(
        &mut e,
        n,
        StyleProperty::TextDecorationColor,
        StyleColor::CurrentColor,
        CompositeOperation::Replace,
    );
    constant(
        &mut e,
        n,
        StyleProperty::TextEmphasisColor,
        StyleColor::CurrentColor,
        CompositeOperation::Replace,
    );
    constant(
        &mut e,
        n,
        StyleProperty::WebkitTextFillColor,
        StyleColor::Resolved(Color::BLUE),
        CompositeOperation::Replace,
    );
    constant(
        &mut e,
        n,
        StyleProperty::Color,
        StyleColor::Resolved(Color::RED),
        CompositeOperation::Replace,
    );
    let style = e.computed_style(n).unwrap();
    assert_eq!(style.effective_text_fill_color(), Color::BLUE);
    assert_eq!(
        style.text_decoration_color.resolve(&Color::BLACK),
        Color::BLUE
    );
    assert_eq!(style.text_emphasis_color.resolve(&Color::BLACK), Color::RED);
}

#[test]
fn inherited_fill_keyword_remains_a_dependency_on_the_childs_own_color() {
    let mut e = engine();
    let n = node(&mut e);
    let root = e.root();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    e.set_property(
        root,
        StyleProperty::WebkitTextFillColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    e.set_property(n, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::BLUE
    );
    e.set_property(
        root,
        StyleProperty::WebkitTextFillColor,
        StyleColor::Resolved(Color::RED).into(),
    )
    .unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::RED
    );
    e.set_property(
        root,
        StyleProperty::WebkitTextFillColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::BLUE
    );
}

#[test]
fn numeric_background_setters_clear_the_authored_keyword_dependency() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    e.set_property(n, StyleProperty::BackgroundColor, Color::BLUE.into())
        .unwrap();
    e.set_property(n, StyleProperty::Color, Color::RED.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().background_color, Color::BLUE);
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    e.set_renderer_style(n, RendererStyleValue::BackgroundColor(Color::WHITE))
        .unwrap();
    e.set_property(n, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().background_color, Color::WHITE);
}

#[test]
fn unchanged_keyword_effect_frames_do_no_scene_work() {
    let mut e = engine();
    let n = node(&mut e);
    constant(
        &mut e,
        n,
        StyleProperty::BackgroundColor,
        StyleColor::CurrentColor,
        CompositeOperation::Replace,
    );
    let first = e.scene().unwrap();
    let stats = e.stats();
    let dirty = e.dirty_generations();
    e.set_animation_time(10.0).unwrap();
    assert_eq!(e.scene().unwrap().generation(), first.generation());
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
}

#[test]
fn inherited_fill_mutations_cross_transparent_descendants_and_invalidate_decoration_paint() {
    let mut e = engine();
    let root = e.root();
    let middle = node(&mut e);
    let child = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(middle, child).unwrap();
    e.set_property(middle, StyleProperty::Color, Color::TRANSPARENT.into())
        .unwrap();
    e.set_property(child, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    e.set_property(
        root,
        StyleProperty::WebkitTextFillColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    assert_eq!(
        e.computed_style(child).unwrap().effective_text_fill_color(),
        Color::BLUE
    );
    let first = e.scene().unwrap();
    let dirty = e.dirty_generations();
    e.set_property(
        root,
        StyleProperty::WebkitTextFillColor,
        StyleColor::Resolved(Color::RED).into(),
    )
    .unwrap();
    assert_eq!(e.computed_style(middle).unwrap().color, Color::TRANSPARENT);
    assert_eq!(
        e.computed_style(child).unwrap().effective_text_fill_color(),
        Color::RED
    );
    assert!(e.dirty_generations().paint > dirty.paint);
    assert_ne!(e.scene().unwrap().generation(), first.generation());
    let fill = constant(
        &mut e,
        child,
        StyleProperty::WebkitTextFillColor,
        StyleColor::Resolved(Color::WHITE),
        CompositeOperation::Replace,
    );
    e.set_property(
        root,
        StyleProperty::WebkitTextFillColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    assert_eq!(
        e.computed_style(child).unwrap().effective_text_fill_color(),
        Color::WHITE
    );
    e.cancel_animation(fill).unwrap();
    assert_eq!(
        e.computed_style(child).unwrap().text_fill_color,
        StyleColor::CurrentColor
    );
    assert_eq!(
        e.computed_style(child).unwrap().effective_text_fill_color(),
        Color::BLUE
    );
}

#[test]
fn text_fill_cancel_and_remove_restore_authored_currentcolor_without_duplicate_events() {
    let mut e = engine();
    let n = node(&mut e);
    e.set_property(n, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    e.set_property(
        n,
        StyleProperty::WebkitTextFillColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    let fill = constant(
        &mut e,
        n,
        StyleProperty::WebkitTextFillColor,
        StyleColor::Resolved(Color::RED),
        CompositeOperation::Replace,
    );
    e.drain_animation_events();
    e.cancel_animation(fill).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().text_fill_color,
        StyleColor::CurrentColor
    );
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::BLUE
    );
    assert_eq!(e.drain_animation_events().len(), 1);
    constant(
        &mut e,
        n,
        StyleProperty::WebkitTextFillColor,
        StyleColor::Resolved(Color::WHITE),
        CompositeOperation::Replace,
    );
    e.detach(n).unwrap();
    assert_eq!(e.object_counts().animations, 0);
    assert_eq!(
        e.computed_style(n).unwrap().text_fill_color,
        StyleColor::CurrentColor
    );
    e.set_property(n, StyleProperty::Color, Color::RED.into())
        .unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::RED
    );
}

#[test]
fn scalar_color_transport_and_literal_currentcolor_use_the_same_property_path() {
    let mut e = engine();
    let n = node(&mut e);
    let root = e.root();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    e.set_property(
        n,
        StyleProperty::Color,
        openui_style::parse_literal(StyleProperty::Color, "currentcolor").unwrap(),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        openui_style::parse_literal(StyleProperty::BackgroundColor, "currentcolor").unwrap(),
    )
    .unwrap();
    e.set_property(n, StyleProperty::WebkitTextFillColor, Color::BLUE.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::RED);
    assert_eq!(e.computed_style(n).unwrap().background_color, Color::RED);
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::BLUE
    );
    e.set_property(
        n,
        StyleProperty::WebkitTextFillColor,
        openui_style::parse_literal(StyleProperty::WebkitTextFillColor, "currentcolor").unwrap(),
    )
    .unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().effective_text_fill_color(),
        Color::RED
    );
}
