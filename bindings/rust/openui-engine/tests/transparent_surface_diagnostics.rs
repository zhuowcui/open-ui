//! Actual native painter identity/interval sources. Diagnostic-only; Chromium,
//! cull/query/independent surface and presentation track qualification are open.
use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_paint::{
    PaintLayoutScopeRole, PaintPropertyDecision, PaintPropertyRole, TransparentSurfaceQualification,
};
use openui_style::{
    Color, Display, LengthValue, Overflow, RendererInternalStyleValue, StyleProperty, StyleValue,
    TransformList, TransformOperation,
};
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
#[test]
fn actual_owner_decisions_sources_and_same_value_frame_remain_owned_and_unqualified() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let viewport = ViewportMetrics::from_logical_size(200.0, 160.0, scale).unwrap();
        let mut e = Engine::new(viewport).unwrap();
        e.set_layered_paint_capture_enabled(true);
        let root = e.root();
        let n = box_(&mut e, root, ElementTag::Div, 80.0, 64.0);
        set(&mut e, n, StyleProperty::Opacity, StyleValue::Number(0.2));
        set(&mut e, n, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            n,
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Translate(
                LengthValue::px(0.25),
                LengthValue::px(0.75),
            )]),
        );
        let child = box_(&mut e, n, ElementTag::Div, 60.0, 48.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        let before = e.scene().unwrap();
        let capture = before.layered_paint_capture().unwrap();
        let properties = capture
            .recording_programs()
            .flat_map(|p| p.properties())
            .filter(|p| {
                p.binding.and_then(|b| b.owner).is_some_and(|id| {
                    id.document == n.document_id()
                        && id.index == n.index()
                        && id.generation == n.generation()
                })
            })
            .collect::<Vec<_>>();
        for role in [
            PaintPropertyRole::Opacity,
            PaintPropertyRole::Transform,
            PaintPropertyRole::OverflowClip,
        ] {
            assert!(
                properties
                    .iter()
                    .any(|p| p.identity.is_some_and(|id| id.role == role)),
                "missing actual {role:?}"
            );
        }
        let mut cpu = SoftwareCompositor::default();
        let canonical = cpu.render(&before).unwrap();
        assert_eq!(cpu.render_layered_diagnostic(&before).unwrap(), canonical);
        // This diagnostic selects an existing engine-owned hint. There is no
        // public authored will-change property or Chromium admission claim here.
        e.set_internal_style(n, RendererInternalStyleValue::WillChangeTransform(true))
            .unwrap();
        let scene = e.scene().unwrap();
        let capture = scene.layered_paint_capture().unwrap();
        let program = capture
            .recording_programs()
            .find(|p| {
                p.properties().iter().any(|p| {
                    matches!(
                        p.decision,
                        PaintPropertyDecision::Opacity {
                            will_change_transform: true,
                            ..
                        }
                    ) && p
                        .binding
                        .and_then(|b| b.owner)
                        .is_some_and(|id| id.index == n.index() && id.generation == n.generation())
                })
            })
            .unwrap();
        let plan = program
            .transparent_surface_program_for_viewport(viewport)
            .unwrap();
        assert!(plan.sources().iter().any(|s| s
            .interval()
            .property
            .binding
            .unwrap()
            .owner
            .unwrap()
            .index
            == n.index()));
        assert_eq!(
            plan.qualification(),
            TransparentSurfaceQualification::UnqualifiedStaticDiagnostic
        );
        assert!(!plan.dynamic_tracks_are_admitted());
        let stats = e.stats();
        // The internal hint publisher intentionally invalidates Subtree. Keep
        // the strict zero-work check on the real authored same-value operation.
        set(&mut e, n, StyleProperty::Opacity, StyleValue::Number(0.2));
        assert_eq!(e.scene().unwrap().generation(), scene.generation());
        assert_eq!(e.stats(), stats);
        drop(e);
        drop(before);
        drop(scene);
        let plan = std::thread::spawn(move || plan).join().unwrap();
        let surface = plan.rasterize_diagnostic(viewport).unwrap();
        assert_eq!(surface.width() as u32, viewport.physical_width());
        assert_eq!(surface.height() as u32, viewport.physical_height());
    }
}
#[test]
fn delegated_content_clip_keeps_wrapper_identity_without_duplicate_opacity() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut e =
            Engine::new(ViewportMetrics::from_logical_size(200.0, 160.0, scale).unwrap()).unwrap();
        e.set_layered_paint_capture_enabled(true);
        let root = e.root();
        let f = box_(&mut e, root, ElementTag::Fieldset, 100.0, 64.0);
        set(&mut e, f, StyleProperty::Opacity, StyleValue::Number(0.5));
        set(&mut e, f, StyleProperty::Overflow, Overflow::Hidden);
        let child = box_(&mut e, f, ElementTag::Div, 160.0, 80.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::BLUE);
        let scene = e.scene().unwrap();
        let capture = scene.layered_paint_capture().unwrap();
        let properties = capture
            .recording_programs()
            .flat_map(|p| p.properties())
            .filter(|p| {
                p.binding
                    .and_then(|b| b.owner)
                    .is_some_and(|id| id.index == f.index() && id.generation == f.generation())
            })
            .collect::<Vec<_>>();
        assert!(properties
            .iter()
            .any(|p| p.layout_role == PaintLayoutScopeRole::FieldsetContent
                && p.identity
                    .is_some_and(|id| id.role == PaintPropertyRole::OverflowClip)));
        assert!(properties
            .iter()
            .any(|p| p.layout_role == PaintLayoutScopeRole::AuthoredBox
                && matches!(p.decision, PaintPropertyDecision::Opacity { .. })));
        assert!(!properties
            .iter()
            .any(|p| p.layout_role == PaintLayoutScopeRole::FieldsetContent
                && matches!(p.decision, PaintPropertyDecision::Opacity { .. })));
        let mut cpu = SoftwareCompositor::default();
        let canonical = cpu.render(&scene).unwrap();
        assert_eq!(cpu.render_layered_diagnostic(&scene).unwrap(), canonical);
    }
}
