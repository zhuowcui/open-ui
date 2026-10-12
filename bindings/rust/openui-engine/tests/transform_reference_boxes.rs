use openui_dom::ElementTag;
use openui_engine::{Engine, ViewportMetrics};
use openui_style::{Display, LengthValue, StyleProperty, TransformList, TransformOperation};

#[test]
fn transform_only_updates_refresh_actual_box_geometry_without_relayout_or_idle_work() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(400.0, 200.0, scale).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(100.0).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::px(40.0).into())
            .unwrap();
        engine.scene().unwrap();
        let before = engine.stats();
        engine
            .set_property(
                node,
                StyleProperty::Transform,
                TransformList(vec![TransformOperation::Translate(
                    LengthValue::percent(50.0),
                    LengthValue::px(0.0),
                )])
                .into(),
            )
            .unwrap();
        engine.scene().unwrap();
        let after = engine.stats();
        assert_eq!(after.layouts, before.layouts);
        let rect = engine.bounds(node).unwrap().unwrap();
        assert_eq!(rect.x, 50.0);
        assert_eq!(rect.width, 100.0);
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(200.0).into())
            .unwrap();
        engine.scene().unwrap();
        let rect = engine.bounds(node).unwrap().unwrap();
        assert_eq!(rect.x, 100.0);
        assert_eq!(rect.width, 200.0);
        let settled = engine.stats();
        engine.scene().unwrap();
        assert_eq!(engine.stats().layouts, settled.layouts);
        assert_eq!(engine.stats().paints, settled.paints);
    }
}
