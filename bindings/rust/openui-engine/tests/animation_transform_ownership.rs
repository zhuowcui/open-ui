//! Shared transform source policy, retained positioned owners and frame work.
//! These source guards depend on the retained transform kernel successor.
use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    AnimationOptions, CompositeOperation, Display, FillMode, Keyframes, LengthValue, Position,
    PropertyKeyframes, RendererInternalStyleValue, RendererStyleValue, StyleProperty, Transform2D,
    TransformList, TransformOperation,
};

fn engine() -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(160.0, 120.0, 1.0).unwrap()).unwrap()
}
fn box_node(e: &mut Engine, parent: NodeHandle, tag: ElementTag) -> NodeHandle {
    let n = e.create_native_element(tag).unwrap();
    e.set_property(n, StyleProperty::Width, LengthValue::px(80.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Height, LengthValue::px(60.0).into())
        .unwrap();
    e.append_child(parent, n).unwrap();
    n
}
fn positioned(
    e: &mut Engine,
    parent: NodeHandle,
    position: Position,
    tag: ElementTag,
) -> NodeHandle {
    let n = box_node(e, parent, tag);
    e.set_property(n, StyleProperty::Width, LengthValue::px(8.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Height, LengthValue::px(8.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Position, position.into())
        .unwrap();
    e.set_property(n, StyleProperty::Left, LengthValue::px(0.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Top, LengthValue::px(0.0).into())
        .unwrap();
    n
}
fn owner(e: &mut Engine) -> NodeHandle {
    let root = e.root();
    let n = box_node(e, root, ElementTag::Div);
    e.set_property(n, StyleProperty::MarginLeft, LengthValue::px(20.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::MarginTop, LengthValue::px(30.0).into())
        .unwrap();
    n
}
fn identity() -> TransformList {
    TransformList(vec![TransformOperation::Scale(1.0, 1.0)])
}
fn set_transform(e: &mut Engine, owner: NodeHandle, value: TransformList) {
    e.set_property(owner, StyleProperty::Transform, value.into())
        .unwrap();
}
fn constant(
    e: &mut Engine,
    owner: NodeHandle,
    value: TransformList,
    fill: FillMode,
) -> AnimationId {
    e.animate(
        owner,
        PropertyKeyframes::typed(
            StyleProperty::Transform,
            Keyframes::from_values(value.clone(), value),
        )
        .unwrap(),
        AnimationOptions {
            duration_ms: 100.0,
            fill,
            composite: CompositeOperation::Replace,
            ..AnimationOptions::default()
        },
        AnimationTimeline::Document,
    )
    .unwrap()
}
fn assert_origin(e: &mut Engine, child: NodeHandle, owner: NodeHandle) {
    let child = e.bounds(child).unwrap().unwrap();
    let owner = e.bounds(owner).unwrap().unwrap();
    assert_eq!((child.x, child.y), (owner.x, owner.y));
}

#[test]
fn specified_identity_and_removal_relayout_absolute_child_without_matrix_change() {
    let mut e = engine();
    let owner = owner(&mut e);
    let child = positioned(&mut e, owner, Position::Absolute, ElementTag::Div);
    let before = e.bounds(child).unwrap().unwrap();
    let owner_box = e.bounds(owner).unwrap().unwrap();
    assert_ne!((before.x, before.y), (owner_box.x, owner_box.y));
    let layouts = e.stats().layouts;
    set_transform(&mut e, owner, identity());
    assert_origin(&mut e, child, owner);
    assert_eq!(e.stats().layouts, layouts + 1);
    let layouts = e.stats().layouts;
    set_transform(&mut e, owner, TransformList::default());
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    assert_eq!(e.stats().layouts, layouts + 1);
    let stats = e.stats();
    let scene = e.scene().unwrap();
    assert_eq!(e.scene().unwrap().generation(), scene.generation());
    assert_eq!(e.stats(), stats);
}

#[test]
fn fixed_native_control_uses_pending_percentage_and_font_relative_owned_sources() {
    for source in [
        TransformList(vec![TransformOperation::Translate(
            LengthValue::percent(0.0),
            LengthValue::px(0.0),
        )]),
        TransformList(vec![TransformOperation::Translate(
            LengthValue::Em(0.0),
            LengthValue::px(0.0),
        )]),
    ] {
        let mut e = engine();
        let owner = owner(&mut e);
        let child = positioned(&mut e, owner, Position::Fixed, ElementTag::Button);
        let before = e.bounds(child).unwrap().unwrap();
        let layouts = e.stats().layouts;
        set_transform(&mut e, owner, source);
        assert_origin(&mut e, child, owner);
        assert_eq!(e.stats().layouts, layouts + 1);
        set_transform(&mut e, owner, TransformList::default());
        assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    }
}

#[test]
fn matrix_changes_and_unpositioned_policy_changes_keep_layout_idle() {
    let mut e = engine();
    let owner = owner(&mut e);
    e.scene().unwrap();
    let layouts = e.stats().layouts;
    set_transform(&mut e, owner, identity());
    e.scene().unwrap();
    assert_eq!(e.stats().layouts, layouts);
    let child = positioned(&mut e, owner, Position::Absolute, ElementTag::Div);
    e.scene().unwrap();
    let layouts = e.stats().layouts;
    set_transform(
        &mut e,
        owner,
        TransformList(vec![TransformOperation::Translate(
            LengthValue::px(5.0),
            LengthValue::px(0.0),
        )]),
    );
    let before = e.bounds(child).unwrap().unwrap();
    assert_eq!(e.stats().layouts, layouts);
    let stats = e.stats();
    let dirty = e.dirty_generations();
    set_transform(
        &mut e,
        owner,
        TransformList(vec![TransformOperation::Translate(
            LengthValue::px(5.0),
            LengthValue::px(0.0),
        )]),
    );
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
}

#[test]
fn relative_owner_keeps_absolute_geometry_but_new_transform_captures_fixed_child() {
    let mut e = engine();
    let owner = owner(&mut e);
    e.set_property(owner, StyleProperty::Position, Position::Relative.into())
        .unwrap();
    let absolute = positioned(&mut e, owner, Position::Absolute, ElementTag::Div);
    let before = e.bounds(absolute).unwrap().unwrap();
    let layouts = e.stats().layouts;
    set_transform(&mut e, owner, identity());
    assert_eq!(e.bounds(absolute).unwrap().unwrap(), before);
    assert_eq!(e.stats().layouts, layouts);
    set_transform(&mut e, owner, TransformList::default());
    e.scene().unwrap();
    let fixed = positioned(&mut e, owner, Position::Fixed, ElementTag::Div);
    e.scene().unwrap();
    let layouts = e.stats().layouts;
    set_transform(&mut e, owner, identity());
    assert_origin(&mut e, fixed, owner);
    assert_eq!(e.stats().layouts, layouts + 1);
}

#[test]
fn intervening_authored_owner_keeps_descendant_out_of_outer_policy_invalidation() {
    for position in [Position::Absolute, Position::Fixed] {
        let mut e = engine();
        let outer = owner(&mut e);
        let inner = box_node(&mut e, outer, ElementTag::Div);
        set_transform(&mut e, inner, identity());
        let child = positioned(&mut e, inner, position, ElementTag::Div);
        let before = e.bounds(child).unwrap().unwrap();
        let layouts = e.stats().layouts;
        set_transform(&mut e, outer, identity());
        assert_eq!(e.bounds(child).unwrap().unwrap(), before);
        assert_eq!(e.stats().layouts, layouts);
        set_transform(&mut e, outer, TransformList::default());
        assert_eq!(e.bounds(child).unwrap().unwrap(), before);
        assert_eq!(e.stats().layouts, layouts);
    }
}

#[test]
fn renderer_affine_source_enters_and_leaves_same_owned_policy_and_repeated_setter_is_idle() {
    let mut e = engine();
    let owner = owner(&mut e);
    let child = positioned(&mut e, owner, Position::Fixed, ElementTag::Div);
    let before = e.bounds(child).unwrap().unwrap();
    let layouts = e.stats().layouts;
    let matrix = Transform2D {
        e: 5.0,
        ..Transform2D::IDENTITY
    };
    e.set_renderer_style(owner, RendererStyleValue::Transform(matrix))
        .unwrap();
    e.bounds(child).unwrap();
    assert_eq!(e.stats().layouts, layouts + 1);
    let stats = e.stats();
    let dirty = e.dirty_generations();
    e.set_renderer_style(owner, RendererStyleValue::Transform(matrix))
        .unwrap();
    e.scene().unwrap();
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
    e.set_renderer_style(owner, RendererStyleValue::Transform(Transform2D::IDENTITY))
        .unwrap();
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    assert!(!e.computed_style(owner).unwrap().has_transform());
}

#[test]
fn removing_none_replace_reveals_identity_survivor_then_current_authored_none() {
    let mut e = engine();
    let owner = owner(&mut e);
    let child = positioned(&mut e, owner, Position::Absolute, ElementTag::Div);
    let before = e.bounds(child).unwrap().unwrap();
    let lower = constant(&mut e, owner, identity(), FillMode::Both);
    e.set_animation_time(0.0).unwrap();
    assert_origin(&mut e, child, owner);
    let upper = constant(&mut e, owner, TransformList::default(), FillMode::Both);
    e.set_animation_time(0.0).unwrap();
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    let layouts = e.stats().layouts;
    e.cancel_animation(upper).unwrap();
    assert_origin(&mut e, child, owner);
    assert_eq!(e.stats().layouts, layouts + 1);
    let layouts = e.stats().layouts;
    e.cancel_animation(lower).unwrap();
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    assert_eq!(e.stats().layouts, layouts + 1);
}

#[test]
fn masked_authored_transform_mutation_publishes_once_and_cancellation_reveals_policy() {
    let mut e = engine();
    let owner = owner(&mut e);
    let child = positioned(&mut e, owner, Position::Fixed, ElementTag::Div);
    let mask = constant(&mut e, owner, identity(), FillMode::Both);
    e.set_animation_time(0.0).unwrap();
    assert_origin(&mut e, child, owner);
    let stats = e.stats();
    let dirty = e.dirty_generations();
    set_transform(
        &mut e,
        owner,
        TransformList(vec![TransformOperation::Translate(
            LengthValue::percent(25.0),
            LengthValue::px(0.0),
        )]),
    );
    e.scene().unwrap();
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
    set_transform(&mut e, owner, TransformList::default());
    e.scene().unwrap();
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
    e.cancel_animation(mask).unwrap();
    let after = e.bounds(child).unwrap().unwrap();
    let owner_box = e.bounds(owner).unwrap().unwrap();
    assert_ne!((after.x, after.y), (owner_box.x, owner_box.y));
    assert_eq!(e.stats().layouts, stats.layouts + 1);
}

#[test]
fn no_fill_effect_expires_and_restores_viewport_fixed_owner() {
    let mut e = engine();
    let owner = owner(&mut e);
    let child = positioned(&mut e, owner, Position::Fixed, ElementTag::Div);
    let before = e.bounds(child).unwrap().unwrap();
    constant(&mut e, owner, identity(), FillMode::None);
    e.set_animation_time(0.0).unwrap();
    assert_origin(&mut e, child, owner);
    let layouts = e.stats().layouts;
    e.set_animation_time(100.0).unwrap();
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    assert_eq!(e.stats().layouts, layouts + 1);
}

#[test]
fn independent_will_change_policy_survives_native_transform_removal() {
    let mut e = engine();
    let owner = owner(&mut e);
    let child = positioned(&mut e, owner, Position::Fixed, ElementTag::Div);
    e.set_internal_style(owner, RendererInternalStyleValue::WillChangeTransform(true))
        .unwrap();
    e.set_internal_style(
        owner,
        RendererInternalStyleValue::EstablishesTransformContainingBlock(true),
    )
    .unwrap();
    assert_origin(&mut e, child, owner);
    let layouts = e.stats().layouts;
    set_transform(&mut e, owner, identity());
    e.scene().unwrap();
    set_transform(&mut e, owner, TransformList::default());
    assert_origin(&mut e, child, owner);
    let style = e.computed_style(owner).unwrap();
    assert!(style.will_change_transform);
    assert!(style.establishes_transform_containing_block);
    assert_eq!(e.stats().layouts, layouts);
}

#[test]
fn captioned_table_decoration_helpers_do_not_mask_a_real_positioned_descendant() {
    let mut e = engine();
    let root = e.root();
    let table = box_node(&mut e, root, ElementTag::Table);
    e.set_property(table, StyleProperty::Display, Display::Table.into())
        .unwrap();
    e.set_property(
        table,
        StyleProperty::MarginLeft,
        LengthValue::px(20.0).into(),
    )
    .unwrap();
    e.set_property(
        table,
        StyleProperty::MarginTop,
        LengthValue::px(30.0).into(),
    )
    .unwrap();
    let caption = box_node(&mut e, table, ElementTag::TableCaption);
    e.set_property(
        caption,
        StyleProperty::Display,
        Display::TableCaption.into(),
    )
    .unwrap();
    e.set_property(caption, StyleProperty::Height, LengthValue::px(12.0).into())
        .unwrap();
    let row = box_node(&mut e, table, ElementTag::TableRow);
    e.set_property(row, StyleProperty::Display, Display::TableRow.into())
        .unwrap();
    let cell = box_node(&mut e, row, ElementTag::TableCell);
    e.set_property(cell, StyleProperty::Display, Display::TableCell.into())
        .unwrap();
    let child = positioned(&mut e, cell, Position::Fixed, ElementTag::Div);
    let before = e.bounds(child).unwrap().unwrap();
    let layouts = e.stats().layouts;
    set_transform(&mut e, table, identity());
    e.scene().unwrap();
    assert_eq!(e.stats().layouts, layouts + 1);
    set_transform(&mut e, table, TransformList::default());
    assert_eq!(e.bounds(child).unwrap().unwrap(), before);
    assert_eq!(e.stats().layouts, layouts + 2);
}
