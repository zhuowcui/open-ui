//! Actual painter ownership/geometry and opt-in static renderer. No Viz claim.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_paint::{
    PaintLayerNodeIdentity, SourceLayerAdmission, SourceLayerQualification, SourceLayerStrategy,
    SourcePositioningReference,
};
use openui_style::{
    parse_literal, BorderStyle, Color, Display, LengthValue, Overflow, Position,
    RendererInternalStyleValue, RendererStyleValue, StyleProperty, StyleValue, TransformList,
    TransformOperation,
};
use std::sync::Arc;
const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: impl Into<StyleValue>) {
    e.set_property(n, p, v.into()).unwrap();
}
fn literal(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: &str) {
    e.set_property(n, p, parse_literal(p, v).unwrap()).unwrap();
}
fn box_(e: &mut Engine, parent: NodeHandle, w: f32, h: f32) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(parent, n).unwrap();
    set(e, n, StyleProperty::Display, Display::Block);
    set(e, n, StyleProperty::Width, LengthValue::px(w));
    set(e, n, StyleProperty::Height, LengthValue::px(h));
    n
}
fn fixture(scale: f64, width: f64, height: f64) -> (Engine, NodeHandle) {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(width, height, scale).unwrap()).unwrap();
    e.set_layered_paint_capture_enabled(true);
    let root = e.root();
    let n = box_(&mut e, root, width as f32, 50.0);
    set(&mut e, n, StyleProperty::Position, Position::Fixed);
    set(&mut e, n, StyleProperty::Top, LengthValue::px(0.0));
    set(&mut e, n, StyleProperty::Left, LengthValue::px(0.0));
    set(&mut e, n, StyleProperty::BackgroundColor, Color::BLUE);
    (e, n)
}
fn identity(n: NodeHandle) -> PaintLayerNodeIdentity {
    PaintLayerNodeIdentity {
        document: n.document_id(),
        index: n.index(),
        generation: n.generation(),
    }
}
fn context(scene: &SceneSnapshot, n: NodeHandle) -> &openui_paint::SourcePaintContextProjection {
    scene
        .source_layer_paint_capture()
        .unwrap()
        .recording_programs()
        .flat_map(|p| p.contexts())
        .find(|p| p.binding.owner == Some(identity(n)) && p.owns_element_box)
        .unwrap()
}
fn pixel(f: &Frame, x: u32, y: u32) -> [u8; 4] {
    let i = y as usize * f.stride + x as usize * 4;
    f.pixels[i..i + 4].try_into().unwrap()
}
fn frame(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default()
        .render_source_layer_diagnostic(scene)
        .unwrap()
}

#[derive(Clone, Copy, Debug)]
enum SourceChromiumFrame {
    Initial,
    Alpha,
    Fieldset,
}

// Actual pinned Chromium frames, including every opaque-white exterior pixel.
// These fixtures have no text or external resources. The stored ink crops are
// lossless and do not relax the complete physical frame/dimension assertion.
fn assert_chromium_frame(frame: &Frame, kind: SourceChromiumFrame, scale: f64) {
    let index = SCALES.iter().position(|value| *value == scale).unwrap();
    let (width, height, ink_height, expected): (u32, u32, u32, &[u8]) = match kind {
        SourceChromiumFrame::Initial => [
            (
                800,
                600,
                50,
                include_bytes!("source_layer_chromium_oracles/initial-scale-1.rgba").as_slice(),
            ),
            (
                1000,
                750,
                63,
                include_bytes!("source_layer_chromium_oracles/initial-scale-1_25.rgba").as_slice(),
            ),
            (
                1200,
                900,
                75,
                include_bytes!("source_layer_chromium_oracles/initial-scale-1_5.rgba").as_slice(),
            ),
            (
                1400,
                1050,
                88,
                include_bytes!("source_layer_chromium_oracles/initial-scale-1_75.rgba").as_slice(),
            ),
            (
                1600,
                1200,
                100,
                include_bytes!("source_layer_chromium_oracles/initial-scale-2.rgba").as_slice(),
            ),
        ][index],
        SourceChromiumFrame::Alpha => [
            (
                160,
                96,
                50,
                include_bytes!("source_layer_chromium_oracles/alpha-scale-1.rgba").as_slice(),
            ),
            (
                200,
                120,
                63,
                include_bytes!("source_layer_chromium_oracles/alpha-scale-1_25.rgba").as_slice(),
            ),
            (
                240,
                144,
                75,
                include_bytes!("source_layer_chromium_oracles/alpha-scale-1_5.rgba").as_slice(),
            ),
            (
                280,
                168,
                88,
                include_bytes!("source_layer_chromium_oracles/alpha-scale-1_75.rgba").as_slice(),
            ),
            (
                320,
                192,
                100,
                include_bytes!("source_layer_chromium_oracles/alpha-scale-2.rgba").as_slice(),
            ),
        ][index],
        SourceChromiumFrame::Fieldset => [
            (
                160,
                96,
                50,
                include_bytes!("source_layer_chromium_oracles/fieldset-scale-1.rgba").as_slice(),
            ),
            (
                200,
                120,
                63,
                include_bytes!("source_layer_chromium_oracles/fieldset-scale-1_25.rgba").as_slice(),
            ),
            (
                240,
                144,
                75,
                include_bytes!("source_layer_chromium_oracles/fieldset-scale-1_5.rgba").as_slice(),
            ),
            (
                280,
                168,
                88,
                include_bytes!("source_layer_chromium_oracles/fieldset-scale-1_75.rgba").as_slice(),
            ),
            (
                320,
                192,
                100,
                include_bytes!("source_layer_chromium_oracles/fieldset-scale-2.rgba").as_slice(),
            ),
        ][index],
    };
    assert_eq!(frame.width, width, "{kind:?} scale {scale}: physical width");
    assert_eq!(
        frame.height, height,
        "{kind:?} scale {scale}: physical height"
    );
    assert_eq!(
        frame.stride,
        width as usize * 4,
        "{kind:?} scale {scale}: packed RGBA stride"
    );
    assert_eq!(
        frame.pixels.len(),
        frame.stride * height as usize,
        "{kind:?} scale {scale}: full RGBA length"
    );
    assert_eq!(expected.len(), width as usize * ink_height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let actual = pixel(frame, x, y);
            let wanted: [u8; 4] = if y < ink_height {
                let offset = (y as usize * width as usize + x as usize) * 4;
                expected[offset..offset + 4].try_into().unwrap()
            } else {
                [255, 255, 255, 255]
            };
            assert_eq!(
                actual, wanted,
                "{kind:?} scale {scale}: Chromium RGBA at ({x}, {y})"
            );
        }
    }
}

#[test]
fn source_background_single_record_and_child_picture_are_real_strategies() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 800.0, 600.0);
        let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        let ctx = context(&scene, n);
        assert!(matches!(
            ctx.positioning,
            SourcePositioningReference::Viewport { .. }
        ));
        let capture = scene.source_layer_paint_capture().unwrap();
        let source = capture
            .recording_programs()
            .flat_map(|p| p.sources())
            .find(|s| s.binding().owner == Some(identity(n)))
            .unwrap();
        // Actual background item records one draw plus two Save/Restore pairs.
        // The pure captured(0) paint-unit neighbor is the true one-op solid source.
        assert_eq!(source.strategy(), SourceLayerStrategy::OpaquePicture);
        let backgrounds: Vec<_> = capture
            .recording_programs()
            .flat_map(|p| p.display_items())
            .filter(|item| item.binding.owner == Some(identity(n)) && item.draw_count > 0)
            .collect();
        assert_eq!(backgrounds.len(), 1);
        assert_eq!(
            backgrounds[0].role,
            openui_paint::PaintDisplayItemRole::BoxDecorationBackground
        );
        assert_eq!(backgrounds[0].draw_count, 1);
        assert_eq!(backgrounds[0].record_operation_count, 5);
        assert_eq!(source.rect_known_to_be_opaque(), source.bounds());
        assert_eq!(
            source.qualification(),
            SourceLayerQualification::UnqualifiedStaticDiagnostic
        );
        assert!(!source.dynamic_tracks_are_admitted());
        assert_chromium_frame(&frame(&scene), SourceChromiumFrame::Initial, scale);
        let child = box_(&mut e, n, 40.0, 20.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::BLACK);
        let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        let source = scene
            .source_layer_paint_capture()
            .unwrap()
            .recording_programs()
            .flat_map(|p| p.sources())
            .find(|s| s.binding().owner == Some(identity(n)))
            .unwrap();
        assert_eq!(source.strategy(), SourceLayerStrategy::OpaquePicture);
        assert_eq!(source.rect_known_to_be_opaque(), source.bounds());
        let f = frame(&scene);
        assert_eq!(pixel(&f, 0, 0), [0, 0, 0, 255]);
        let row = (50.0 * scale).ceil() as u32 - 1;
        assert_eq!(
            pixel(&f, f.width / 2, row),
            [0, 0, 255, 255],
            "child strategy {scale}"
        );
    }
}
#[test]
fn original_edge_and_all_eight_controls_keep_real_property_boundaries() {
    for scale in SCALES {
        for mode in 0..9 {
            let width = if mode == 8 { 1280.0 } else { 800.0 };
            let height = if mode == 8 { 720.0 } else { 600.0 };
            let (mut e, n) = fixture(scale, width, height);
            let root = e.root();
            if mode == 1 || mode == 3 || mode == 8 {
                box_(&mut e, root, 10.0, 1000.0);
            }
            if mode == 2 || mode == 3 {
                let cb = box_(&mut e, root, width as f32, 600.0);
                set(
                    &mut e,
                    cb,
                    StyleProperty::Transform,
                    TransformList(vec![TransformOperation::Scale(1.0, 1.0)]),
                );
                e.append_or_move_child(cb, n).unwrap();
            }
            if mode == 4 {
                set(&mut e, n, StyleProperty::Top, LengthValue::px(0.25));
            }
            if mode == 5 {
                literal(&mut e, n, StyleProperty::BorderRadius, "8px");
            }
            if mode == 6 {
                set(
                    &mut e,
                    n,
                    StyleProperty::OutlineWidth,
                    StyleValue::Renderer(RendererStyleValue::OutlineWidth(1)),
                );
                set(
                    &mut e,
                    n,
                    StyleProperty::OutlineStyle,
                    StyleValue::Renderer(RendererStyleValue::OutlineStyle(BorderStyle::Solid)),
                );
                set(&mut e, n, StyleProperty::OutlineColor, Color::BLACK);
            }
            if mode == 7 {
                let child = box_(&mut e, n, 40.0, 20.0);
                set(&mut e, child, StyleProperty::BackgroundColor, Color::BLACK);
            }
            let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
            let ctx = context(&scene, n);
            let selected = scene
                .source_layer_paint_capture()
                .unwrap()
                .recording_programs()
                .flat_map(|p| p.candidates())
                .find(|p| p.context == ctx.identity)
                .unwrap();
            if mode == 2 || mode == 3 {
                assert!(matches!(
                    ctx.positioning,
                    SourcePositioningReference::ContainingBox { .. }
                ));
                assert_eq!(selected.admission, None);
                assert_eq!(
                    frame(&scene).pixels,
                    SoftwareCompositor::default().render(&scene).unwrap().pixels
                );
            } else {
                assert_eq!(
                    selected.admission,
                    Some(SourceLayerAdmission::ViewportContainingBlock)
                );
            }
            let f = frame(&scene);
            if scale == 1.25 && [0, 1, 4, 7, 8].contains(&mode) {
                for x in 0..f.width {
                    assert_eq!(pixel(&f, x, 62), [0, 0, 255, 255], "state {mode} x{x}");
                }
                assert_eq!(pixel(&f, f.width / 2, 63), [255, 255, 255, 255]);
            }
        }
    }
}

#[test]
fn font_svg_resource_dependencies_stay_owned_without_claiming_child_extraction() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 160.0, 96.0);
        let root = e.root();
        e.register_font_face(
            Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice()),
            openui_engine::FontFaceDescriptor::new("Source plan Ahem"),
        )
        .unwrap();
        set(
            &mut e,
            root,
            StyleProperty::FontFamily,
            openui_style::FontFamilyList {
                families: vec![openui_style::FontFamily::Named("Source plan Ahem".into())],
            },
        );
        let text = e.create_text("Owned").unwrap();
        e.append_child(root, text).unwrap();
        let image = e.create_native_element(ElementTag::Image).unwrap();
        e.append_child(root, image).unwrap();
        set(&mut e, image, StyleProperty::Display, Display::Block);
        set(&mut e, image, StyleProperty::Width, LengthValue::px(20.0));
        set(&mut e, image, StyleProperty::Height, LengthValue::px(20.0));
        let resource=e.register_image_resource("memory:source-layer-svg","image/svg+xml",
            "809bab3d90547e75eda8d473aa72a4f4f20b9e23ea7885f2e900eb0cf61aa6de",
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="#008080"/></svg>"##.to_vec());
        e.set_image_resource(image, resource, Some((1.0, 1.0)))
            .unwrap();
        // Keep the fixed box below this visible resource while retaining its source identity.
        set(&mut e, n, StyleProperty::Top, LengthValue::px(60.0));
        let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        let capture = scene.source_layer_paint_capture().unwrap();
        // set_image_resource publishes Image even for SVG MIME; decoding
        // rasterizes this resource before ordinary tracked image painting.
        assert!(capture.unavailable_recordings().is_empty());
        let layered = scene.layered_paint_capture().unwrap();
        assert!(layered.recording_programs().all(|program| {
            program.capture_status() == &openui_paint::PaintCaptureStatus::Complete
        }));
        let image_program = layered
            .recording_programs()
            .find(|program| {
                program
                    .sources()
                    .iter()
                    .any(|source| source.binding().owner == Some(identity(image)))
            })
            .unwrap();
        let image_source = image_program
            .sources()
            .iter()
            .find(|source| source.binding().owner == Some(identity(image)))
            .unwrap();
        assert_eq!(image_source.chunks().len(), 1);
        assert!(image_program
            .draw_references()
            .iter()
            .any(|draw| draw.source == image_source.binding().id));
        if scale == 1.0 {
            assert!(image_program.coverage().unclassified_image_draws > 0);
            assert_eq!(
                image_source.chunks()[0].shader_dependency(),
                openui_paint::PaintShaderDependency::None
            );
        } else {
            assert!(image_program.coverage().unclassified_image_shader_draws > 0);
            assert_eq!(
                image_source.chunks()[0].shader_dependency(),
                openui_paint::PaintShaderDependency::Image
            );
        }
        assert!(capture
            .recording_programs()
            .flat_map(|p| p.sources())
            .all(|source| source.binding().owner != Some(identity(image))));
        let expected = frame(&scene);
        assert!(expected
            .pixels
            .chunks_exact(4)
            .any(|p| p == [0, 128, 128, 255]));
        assert_eq!(
            expected.pixels,
            SoftwareCompositor::default().render(&scene).unwrap().pixels
        );
        drop(e);
        assert_eq!(
            std::thread::spawn(move || frame(&scene)).join().unwrap(),
            expected
        );
    }
}
#[test]
fn ordinary_absolute_and_hint_states_do_not_select_by_position_tag() {
    for scale in SCALES {
        for position in [Position::Static, Position::Absolute] {
            let (mut e, n) = fixture(scale, 100.0, 96.0);
            set(&mut e, n, StyleProperty::Position, position);
            let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
            assert!(!scene
                .source_layer_paint_capture()
                .unwrap()
                .recording_programs()
                .flat_map(|p| p.sources())
                .any(|s| s.binding().owner == Some(identity(n))));
            assert_eq!(
                frame(&scene).pixels,
                SoftwareCompositor::default().render(&scene).unwrap().pixels
            );
            e.set_internal_style(n, RendererInternalStyleValue::WillChangeTransform(true))
                .unwrap();
            let hinted = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
            assert!(hinted
                .source_layer_paint_capture()
                .unwrap()
                .recording_programs()
                .flat_map(|p| p.candidates())
                .any(|c| c.context == context(&hinted, n).identity
                    && c.admission == Some(SourceLayerAdmission::TransformHint)));
        }
    }
}
#[test]
fn immutable_publication_source_generations_and_distinct_cache_survive_engine_drop() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 160.0, 96.0);
        let original = e.scene().unwrap();
        assert!(original.source_layer_paint_capture().is_none());
        let scene = original.clone().with_source_layer_diagnostic().unwrap();
        let binding = context(&scene, n).binding;
        assert_eq!(
            scene.source_layer_paint_capture().unwrap().qualification(),
            SourceLayerQualification::UnqualifiedStaticDiagnostic
        );
        assert!(scene
            .retained_scene()
            .unwrap()
            .source_layer_paint_capture()
            .is_some());
        let scene2 = scene.clone().with_source_layer_diagnostic().unwrap();
        assert!(Arc::ptr_eq(
            scene.source_layer_paint_capture().unwrap(),
            scene2.source_layer_paint_capture().unwrap()
        ));
        let stats = e.stats();
        assert_eq!(e.scene().unwrap().generation(), original.generation());
        assert_eq!(e.stats(), stats);
        let mut c = SoftwareCompositor::default();
        let expected = c.render_source_layer_diagnostic(&scene).unwrap();
        let stats = c.stats();
        assert_eq!(c.render_source_layer_diagnostic(&scene).unwrap(), expected);
        assert_eq!(c.stats().rasterized, stats.rasterized);
        assert_eq!(c.stats().reused, stats.reused + 1);
        set(&mut e, n, StyleProperty::Display, Display::None);
        e.scene().unwrap();
        set(&mut e, n, StyleProperty::Display, Display::Block);
        let restored = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        assert_eq!(context(&restored, n).binding, binding);
        e.remove(n).unwrap();
        let root = e.root();
        let newer = box_(&mut e, root, 160.0, 50.0);
        set(&mut e, newer, StyleProperty::Position, Position::Fixed);
        set(&mut e, newer, StyleProperty::BackgroundColor, Color::BLUE);
        let replaced = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        assert_ne!(context(&replaced, newer).binding, binding);
        drop(e);
        let replay = std::thread::spawn(move || frame(&scene)).join().unwrap();
        assert_eq!(replay, expected);
    }
}

#[test]
fn overlapping_alpha_and_fieldset_helper_keep_one_effect_and_structural_owner() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 160.0, 96.0);
        set(&mut e, n, StyleProperty::Opacity, 0.5_f32);
        let a = box_(&mut e, n, 60.0, 20.0);
        set(&mut e, a, StyleProperty::BackgroundColor, Color::RED);
        let b = box_(&mut e, n, 60.0, 20.0);
        set(&mut e, b, StyleProperty::Position, Position::Absolute);
        set(&mut e, b, StyleProperty::Left, LengthValue::px(20.0));
        set(&mut e, b, StyleProperty::Top, LengthValue::px(10.0));
        set(&mut e, b, StyleProperty::BackgroundColor, Color::BLACK);
        let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        let source = scene
            .source_layer_paint_capture()
            .unwrap()
            .recording_programs()
            .flat_map(|p| p.sources())
            .find(|s| s.binding().owner == Some(identity(n)))
            .unwrap();
        assert_eq!(source.strategy(), SourceLayerStrategy::TransparentPicture);
        assert_chromium_frame(&frame(&scene), SourceChromiumFrame::Alpha, scale);
        let root = e.root();
        let field = e.create_native_element(ElementTag::Fieldset).unwrap();
        e.append_child(root, field).unwrap();
        set(&mut e, field, StyleProperty::Width, LengthValue::px(100.0));
        set(&mut e, field, StyleProperty::Height, LengthValue::px(30.0));
        set(&mut e, field, StyleProperty::Overflow, Overflow::Hidden);
        let legend = e.create_native_element(ElementTag::Legend).unwrap();
        e.append_child(field, legend).unwrap();
        set(&mut e, legend, StyleProperty::Width, LengthValue::px(20.0));
        set(&mut e, legend, StyleProperty::Height, LengthValue::px(10.0));
        let child = box_(&mut e, field, 100.0, 80.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        e.scroll_to(field, 0.0, 15.0).unwrap();
        let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        let helper = scene
            .source_layer_paint_capture()
            .unwrap()
            .recording_programs()
            .flat_map(|p| p.contexts())
            .find(|c| {
                c.binding.owner == Some(identity(field))
                    && c.identity.layout_role == openui_paint::PaintLayoutScopeRole::FieldsetContent
            })
            .unwrap();
        assert!(!helper.owns_element_box);
        assert_eq!(helper.scroll_owner.unwrap().owner, Some(identity(field)));
        assert!(scene
            .source_layer_paint_capture()
            .unwrap()
            .recording_programs()
            .flat_map(|p| p.candidates())
            .any(|c| c.context == helper.identity
                && c.unavailable == Some(openui_paint::SourceLayerUnavailable::BorrowedLayoutBox)));
        assert_chromium_frame(&frame(&scene), SourceChromiumFrame::Fieldset, scale);
    }
}

#[test]
fn explicit_static_svg_raw_helper_retains_canonical_pixels_without_source_extraction() {
    for scale in SCALES {
        let mut e =
            Engine::new(ViewportMetrics::from_logical_size(80.0, 64.0, scale).unwrap()).unwrap();
        e.set_layered_paint_capture_enabled(true);
        let image = e.create_native_element(ElementTag::Image).unwrap();
        let root = e.root();
        e.append_child(root, image).unwrap();
        set(&mut e, image, StyleProperty::Display, Display::Block);
        set(&mut e, image, StyleProperty::Width, LengthValue::px(20.0));
        set(&mut e, image, StyleProperty::Height, LengthValue::px(20.0));
        let resource=e.register_image_resource("memory:source-layer-static-svg","image/svg+xml",
            "809bab3d90547e75eda8d473aa72a4f4f20b9e23ea7885f2e900eb0cf61aa6de",
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="#008080"/></svg>"##.to_vec());
        e.set_renderer_node_state(
            image,
            openui_engine::RendererNodeState::Replaced(Some(openui_dom::ReplacedContent {
                resource: openui_dom::ReplacedResourceKind::StaticSvg(resource),
                intrinsic_width: Some(1.0),
                intrinsic_height: Some(1.0),
                intrinsic_ratio: Some((1.0, 1.0)),
            })),
        )
        .unwrap();
        let scene = e.scene().unwrap().with_source_layer_diagnostic().unwrap();
        let capture = scene.source_layer_paint_capture().unwrap();
        assert!(capture.unavailable_recordings().iter().any(|recording| matches!(
            &recording.capture_status,
            openui_paint::PaintCaptureStatus::Incomplete(reasons)
                if reasons.contains(&openui_paint::PaintCaptureIncompleteReason::RawCanvasAccess)
        )));
        assert!(capture
            .recording_programs()
            .flat_map(|p| p.sources())
            .all(|source| source.binding().owner != Some(identity(image))));
        let expected = SoftwareCompositor::default().render(&scene).unwrap();
        assert!(expected
            .pixels
            .chunks_exact(4)
            .any(|p| p == [0, 128, 128, 255]));
        assert_eq!(frame(&scene), expected);
        drop(e);
        assert_eq!(
            std::thread::spawn(move || frame(&scene)).join().unwrap(),
            expected
        );
    }
}
