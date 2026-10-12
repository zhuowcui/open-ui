//! Removed transform owners retain distinct normal-flow continuation windows.

use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_layout::Fragment;
use openui_style::{
    ColumnFill, Display, LengthValue, Position, RendererStyleValue, StyleProperty, TransformList,
    TransformOperation,
};

fn box_(engine: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    for (property, value) in [
        (StyleProperty::Width, width),
        (StyleProperty::Height, height),
    ] {
        engine
            .set_property(node, property, LengthValue::px(value).into())
            .unwrap();
    }
    engine
        .set_property(node, StyleProperty::Display, Display::Block.into())
        .unwrap();
    engine.append_child(parent, node).unwrap();
    node
}

#[test]
fn nested_normal_sources_do_not_reuse_the_nearest_owners_relative_window() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(400.0, 240.0, scale).unwrap()).unwrap();
        let root = engine.root();
        let columns = box_(&mut engine, root, 200.0, 50.0);
        engine
            .set_renderer_style(columns, RendererStyleValue::ColumnCount(Some(2)))
            .unwrap();
        engine
            .set_renderer_style(columns, RendererStyleValue::ColumnFill(ColumnFill::Auto))
            .unwrap();
        engine
            .set_renderer_style(
                columns,
                RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
            )
            .unwrap();
        let outer = box_(&mut engine, columns, 100.0, 68.0);
        let inner = box_(&mut engine, outer, 80.0, 50.0);
        for (owner, top, scale) in [(outer, 7.0, 0.5), (inner, 11.0, 0.75)] {
            engine
                .set_property(owner, StyleProperty::Position, Position::Relative.into())
                .unwrap();
            engine
                .set_property(owner, StyleProperty::Top, LengthValue::px(top).into())
                .unwrap();
            engine
                .set_property(
                    owner,
                    StyleProperty::Transform,
                    TransformList(vec![TransformOperation::Scale(scale, scale)]).into(),
                )
                .unwrap();
        }
        engine
            .set_property(
                outer,
                StyleProperty::PaddingTop,
                LengthValue::px(12.0).into(),
            )
            .unwrap();
        let child = box_(&mut engine, inner, 8.0, 120.0);
        engine
            .set_property(child, StyleProperty::Position, Position::Fixed.into())
            .unwrap();
        for property in [StyleProperty::Top, StyleProperty::Left] {
            engine
                .set_property(child, property, LengthValue::px(0.0).into())
                .unwrap();
        }
        let scene = engine.scene().unwrap();
        fn collect(
            fragment: &Fragment,
            out: &mut Vec<(u32, Vec<openui_layout::PromotedTransformAncestor>)>,
        ) {
            if fragment.promoted_transform_ancestors.len() == 2 {
                if let Some(index) = fragment
                    .positioned_fragmentation
                    .and_then(|positioned| positioned.fragmentainer_index)
                {
                    out.push((index, fragment.promoted_transform_ancestors.clone()));
                }
            }
            for child in &fragment.children {
                collect(child, out);
            }
        }
        let mut sources = Vec::new();
        collect(scene.fragments(), &mut sources);
        let first = &sources
            .iter()
            .find(|(index, _)| *index == 0)
            .expect("first actual promoted continuation")
            .1;
        let second = &sources
            .iter()
            .find(|(index, _)| *index == 1)
            .expect("second actual promoted continuation")
            .1;
        assert_eq!(first[0].source_reference_box.height().to_f32(), 80.0);
        assert_eq!(first[1].source_reference_box.height().to_f32(), 50.0);
        assert_eq!(first[0].source_offset.top.to_f32(), 0.0);
        assert_eq!(first[1].source_offset.top.to_f32(), 12.0);
        assert_eq!(first[0].visual_offset.top.to_f32(), 7.0);
        assert_eq!(first[1].visual_offset.top.to_f32(), 18.0);
        assert_eq!(first[0].fragment_size.height.to_f32(), 50.0);
        assert_eq!(first[1].fragment_size.height.to_f32(), 38.0);
        assert_eq!(first[0].fragment_offset.top.to_f32(), 7.0);
        assert_eq!(first[1].fragment_offset.top.to_f32(), 30.0);
        assert_eq!(second[0].fragment_size.height.to_f32(), 30.0);
        assert_eq!(second[1].fragment_size.height.to_f32(), 12.0);
        assert_eq!(second[0].fragment_offset.top.to_f32(), 7.0);
        assert_eq!(second[1].fragment_offset.top.to_f32(), 18.0);
        // Source metadata is immutable after publication and retained scene
        // reuse must not rebuild or apply these offsets a second time.
        let before = engine.stats();
        assert_eq!(engine.scene().unwrap().generation(), scene.generation());
        assert_eq!(engine.stats(), before);
        drop(engine);
        let mut retained = Vec::new();
        collect(scene.fragments(), &mut retained);
        assert_eq!(retained, sources);
    }
}
