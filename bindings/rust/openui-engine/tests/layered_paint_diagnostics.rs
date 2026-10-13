//! Native owned source/order/cache guards; all layered/dynamic qualification
//! remains explicit until these run and the full pixel/hardware gates pass.
use openui_compositor::{SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_paint::{LayeredPaintQualification, PaintLayerNodeIdentity, PaintLayoutScopeRole};
use openui_style::{
    Color, CornerRadii, Display, Edges, LengthValue, Overflow, Position, StyleProperty, StyleValue,
    TransformList, TransformOperation,
};
use std::sync::Arc;

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: impl Into<StyleValue>) {
    e.set_property(n, p, v.into()).unwrap();
}
fn box_(e: &mut Engine, p: NodeHandle, tag: ElementTag, w: f32, h: f32) -> NodeHandle {
    let n = e.create_native_element(tag).unwrap();
    e.append_child(p, n).unwrap();
    if tag == ElementTag::Div {
        set(e, n, StyleProperty::Display, Display::Block);
    }
    set(e, n, StyleProperty::Width, LengthValue::px(w));
    set(e, n, StyleProperty::Height, LengthValue::px(h));
    n
}
fn identity(n: NodeHandle) -> PaintLayerNodeIdentity {
    PaintLayerNodeIdentity {
        document: n.document_id(),
        index: n.index(),
        generation: n.generation(),
    }
}
fn source_id(scene: &SceneSnapshot, n: NodeHandle) -> u64 {
    scene
        .layered_paint_capture()
        .unwrap()
        .content()
        .sources()
        .iter()
        .find(|s| s.binding().owner == Some(identity(n)))
        .unwrap()
        .binding()
        .id
        .0
}
fn equal_replay(scene: &SceneSnapshot) -> openui_compositor::Frame {
    let mut c = SoftwareCompositor::default();
    let canonical = c.render(scene).unwrap();
    let layered = c.render_layered_diagnostic(scene).unwrap();
    assert_eq!(layered, canonical);
    let stats = c.stats();
    assert_eq!(c.render_layered_diagnostic(scene).unwrap(), canonical);
    assert_eq!(c.stats().rasterized, stats.rasterized);
    assert_eq!(c.stats().reused, stats.reused + 1);
    canonical
}

#[test]
fn default_canonical_opt_in_and_unchanged_capture_preserve_zero_work() {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(160.0, 96.0, 1.25).unwrap()).unwrap();
    let root = e.root();
    let n = box_(&mut e, root, ElementTag::Div, 60.0, 40.0);
    set(&mut e, n, StyleProperty::BackgroundColor, Color::GREEN);
    let default = e.scene().unwrap();
    assert!(default.layered_paint_capture().is_none());
    assert!(SoftwareCompositor::default()
        .render_layered_diagnostic(&default)
        .is_err());
    let expected = SoftwareCompositor::default()
        .render(&default)
        .unwrap()
        .pixels;
    let before = e.stats();
    e.set_layered_paint_capture_enabled(true);
    let first = e.scene().unwrap();
    assert_eq!(e.stats().layouts, before.layouts);
    assert_eq!(equal_replay(&first).pixels, expected);
    let capture = first.layered_paint_capture().unwrap();
    let stats = e.stats();
    e.set_layered_paint_capture_enabled(true);
    let second = e.scene().unwrap();
    assert_eq!(e.stats(), stats);
    assert!(Arc::ptr_eq(
        capture,
        second.layered_paint_capture().unwrap()
    ));
    assert_eq!(
        capture.qualification(),
        LayeredPaintQualification::UnqualifiedStaticReplay
    );
    e.set_layered_paint_capture_enabled(false);
    let off = e.scene().unwrap();
    assert!(off.layered_paint_capture().is_none());
    assert_eq!(
        SoftwareCompositor::default().render(&off).unwrap().pixels,
        expected
    );
}

#[test]
fn canonical_and_layered_replay_share_effect_clip_scroll_tiles_and_helper_roles() {
    for scale in SCALES {
        for width in [160.0_f64, 720.0] {
            let mut e =
                Engine::new(ViewportMetrics::from_logical_size(width, 240.0, scale).unwrap())
                    .unwrap();
            e.set_layered_paint_capture_enabled(true);
            let root = e.root();
            let outer = box_(&mut e, root, ElementTag::Div, width as f32, 64.0);
            set(&mut e, outer, StyleProperty::Overflow, Overflow::Hidden);
            set(
                &mut e,
                outer,
                StyleProperty::Opacity,
                StyleValue::Number(0.5),
            );
            set(
                &mut e,
                outer,
                StyleProperty::BorderRadius,
                CornerRadii(Edges::all(LengthValue::percent(25.0))),
            );
            set(
                &mut e,
                outer,
                StyleProperty::Transform,
                TransformList(vec![TransformOperation::Translate(
                    LengthValue::px(0.25),
                    LengthValue::px(0.75),
                )]),
            );
            let child = box_(&mut e, outer, ElementTag::Div, 1000.0, 90.0);
            set(&mut e, child, StyleProperty::BackgroundColor, Color::BLUE);
            set(
                &mut e,
                child,
                StyleProperty::Opacity,
                StyleValue::Number(0.5),
            );
            let oof = box_(&mut e, outer, ElementTag::Div, 16.0, 12.0);
            set(&mut e, oof, StyleProperty::Position, Position::Absolute);
            set(&mut e, oof, StyleProperty::BackgroundColor, Color::RED);
            let table = box_(&mut e, root, ElementTag::Table, 80.0, 30.0);
            let caption = box_(&mut e, table, ElementTag::TableCaption, 80.0, 12.0);
            set(
                &mut e,
                caption,
                StyleProperty::BackgroundColor,
                Color::GREEN,
            );
            let row = e.create_native_element(ElementTag::TableRow).unwrap();
            e.append_child(table, row).unwrap();
            let cell = box_(&mut e, row, ElementTag::TableCell, 80.0, 18.0);
            set(&mut e, cell, StyleProperty::BackgroundColor, Color::BLUE);
            let fieldset = box_(&mut e, root, ElementTag::Fieldset, 100.0, 40.0);
            set(&mut e, fieldset, StyleProperty::Overflow, Overflow::Hidden);
            let legend = box_(&mut e, fieldset, ElementTag::Legend, 30.0, 10.0);
            set(&mut e, legend, StyleProperty::BackgroundColor, Color::RED);
            let content = box_(&mut e, fieldset, ElementTag::Div, 180.0, 80.0);
            set(
                &mut e,
                content,
                StyleProperty::BackgroundColor,
                Color::GREEN,
            );
            e.scroll_to(fieldset, 12.0, 8.0).unwrap();
            let scene = e.scene().unwrap();
            equal_replay(&scene);
            let program = scene.layered_paint_capture().unwrap().content();
            assert!(program
                .scopes()
                .iter()
                .any(|scope| scope.layout_role == PaintLayoutScopeRole::FieldsetContent));
            assert!(program
                .scopes()
                .iter()
                .any(|scope| scope.layout_role == PaintLayoutScopeRole::TableHelper));
            assert_eq!(
                source_id(&scene, fieldset),
                scene
                    .retained_scene()
                    .unwrap()
                    .layers()
                    .iter()
                    .find(|layer| layer.owner.document == fieldset.document_id()
                        && layer.owner.index == fieldset.index()
                        && layer.owner.generation == fieldset.generation())
                    .unwrap()
                    .id
                    .0
            );
            assert!(!program.dynamic_tracks_are_admitted());
        }
    }
}

#[test]
fn live_ids_survive_mutation_hidden_reattach_and_retire_generations() {
    let mut e = Engine::new(ViewportMetrics::from_logical_size(160.0, 96.0, 1.0).unwrap()).unwrap();
    e.set_layered_paint_capture_enabled(true);
    let root = e.root();
    let n = box_(&mut e, root, ElementTag::Div, 60.0, 40.0);
    set(&mut e, n, StyleProperty::BackgroundColor, Color::GREEN);
    let saved = e.scene().unwrap();
    let id = source_id(&saved, n);
    set(&mut e, n, StyleProperty::Width, LengthValue::px(70.0));
    assert_eq!(source_id(&e.scene().unwrap(), n), id);
    set(&mut e, n, StyleProperty::Display, Display::None);
    let hidden = e.scene().unwrap();
    equal_replay(&hidden);
    assert!(!hidden
        .layered_paint_capture()
        .unwrap()
        .content()
        .sources()
        .iter()
        .any(|s| s.binding().owner == Some(identity(n))));
    set(&mut e, n, StyleProperty::Display, Display::Block);
    assert_eq!(source_id(&e.scene().unwrap(), n), id);
    e.detach(n).unwrap();
    e.scene().unwrap();
    e.append_child(root, n).unwrap();
    assert_eq!(source_id(&e.scene().unwrap(), n), id);
    e.remove(n).unwrap();
    let replacement = box_(&mut e, root, ElementTag::Div, 60.0, 40.0);
    set(
        &mut e,
        replacement,
        StyleProperty::BackgroundColor,
        Color::GREEN,
    );
    assert_ne!(source_id(&e.scene().unwrap(), replacement), id);
    let mut other =
        Engine::new(ViewportMetrics::from_logical_size(160.0, 96.0, 1.0).unwrap()).unwrap();
    other.set_layered_paint_capture_enabled(true);
    let root = other.root();
    let n = box_(&mut other, root, ElementTag::Div, 60.0, 40.0);
    set(&mut other, n, StyleProperty::BackgroundColor, Color::GREEN);
    assert_ne!(source_id(&other.scene().unwrap(), n), id);
    equal_replay(&saved);
}

#[test]
fn immutable_sources_survive_font_image_engine_drop_and_cross_thread_replay() {
    for scale in SCALES {
        let mut e =
            Engine::new(ViewportMetrics::from_logical_size(160.0, 96.0, scale).unwrap()).unwrap();
        e.set_layered_paint_capture_enabled(true);
        let root = e.root();
        e.register_font_face(
            Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice()),
            openui_engine::FontFaceDescriptor::new("Layered source Ahem"),
        )
        .unwrap();
        set(
            &mut e,
            root,
            StyleProperty::FontFamily,
            openui_style::FontFamilyList {
                families: vec![openui_style::FontFamily::Named(
                    "Layered source Ahem".into(),
                )],
            },
        );
        let text = e.create_text("Owned").unwrap();
        e.append_child(root, text).unwrap();
        let image = box_(&mut e, root, ElementTag::Image, 60.0, 40.0);
        let resource=e.register_image_resource("memory:layered-source","image/svg+xml",
            "809bab3d90547e75eda8d473aa72a4f4f20b9e23ea7885f2e900eb0cf61aa6de",
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="#008080"/></svg>"##.to_vec());
        e.set_image_resource(image, resource, Some((1.0, 1.0)))
            .unwrap();
        let scene = e.scene().unwrap();
        let expected = equal_replay(&scene);
        assert!(expected
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel == [0, 128, 128, 255]));
        assert!(scene
            .layered_paint_capture()
            .unwrap()
            .content()
            .sources()
            .iter()
            .any(|source| source.retained_font_face_count() > 0));
        drop(e);
        let actual = std::thread::spawn(move || equal_replay(&scene))
            .join()
            .unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn opaque_viewport_sources_and_query_operands_do_not_claim_layer_admission() {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(160.0, 96.0, 1.25).unwrap()).unwrap();
    e.set_layered_paint_capture_enabled(true);
    let root = e.root();
    set(&mut e, root, StyleProperty::Overflow, Overflow::Scroll);
    let child = box_(&mut e, root, ElementTag::Div, 300.0, 300.0);
    set(&mut e, child, StyleProperty::BackgroundColor, Color::BLUE);
    let scene = e.scene().unwrap();
    equal_replay(&scene);
    let capture = scene.layered_paint_capture().unwrap();
    assert!(capture.has_opaque_viewport_content_source());
    assert!(capture.viewport_overlay().is_some());
    assert!(capture.content().coverage().opaque_picture_draws > 0);
    assert_eq!(
        capture.qualification(),
        LayeredPaintQualification::UnqualifiedStaticReplay
    );
    assert!(!capture.content().dynamic_tracks_are_admitted());
    assert!(!scene
        .retained_scene()
        .unwrap()
        .authored_layers_are_partitioned());
}
