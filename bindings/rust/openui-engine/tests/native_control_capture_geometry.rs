//! Capture-only theme operands use the ordinary native intrinsic kernel.
use openui_dom::{ElementTag, FormControlRole, NodeId, ReplacedContent, ReplacedResourceKind};
use openui_engine::{
    AnimationTimeline, Engine, EngineError, NodeHandle, RendererNodeState, ViewportMetrics,
};
use openui_style::*;

fn engine(scale: f64) -> Engine {
    Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(640.0, 480.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
    )
    .unwrap()
}
fn authored(e: &mut Engine, n: NodeHandle, value: RendererStyleValue) {
    e.set_property(n, value.property(), StyleValue::Renderer(value))
        .unwrap();
}
fn control(e: &mut Engine, parent: NodeHandle, tag: ElementTag) -> NodeHandle {
    let n = e.create_element(tag).unwrap();
    e.set_attribute(n, "id", format!("control-{}", n.index()))
        .unwrap();
    authored(e, n, RendererStyleValue::Display(Display::InlineBlock));
    authored(
        e,
        n,
        RendererStyleValue::FontFamily(FontFamilyList::single("Ahem")),
    );
    authored(e, n, RendererStyleValue::FontSize(16.0));
    e.append_child(parent, n).unwrap();
    n
}
fn metadata(e: &mut Engine, n: NodeHandle) -> Option<ReplacedContent> {
    let id = e.attribute(n, "id").unwrap().unwrap().to_owned();
    let _scene = e.scene().unwrap();
    fn find(d: &openui_dom::Document, node: NodeId, id: &str) -> Option<NodeId> {
        if d.node(node).attributes.get("id").map(String::as_str) == Some(id) {
            return Some(node);
        }
        d.children(node).find_map(|child| find(d, child, id))
    }
    let d = e.native_document();
    d.node(find(d, d.root(), &id).unwrap()).replaced
}
fn natural(e: &mut Engine, n: NodeHandle) -> (Option<f32>, Option<f32>) {
    let content = metadata(e, n).unwrap();
    (content.intrinsic_width, content.intrinsic_height)
}
fn content_fields(
    content: Option<ReplacedContent>,
) -> Option<(
    ReplacedResourceKind,
    Option<f32>,
    Option<f32>,
    Option<(f32, f32)>,
)> {
    content.map(|content| {
        (
            content.resource,
            content.intrinsic_width,
            content.intrinsic_height,
            content.intrinsic_ratio,
        )
    })
}
fn capture(e: &mut Engine, n: NodeHandle, role: FormControlRole, width: ScrollbarWidth) {
    e.set_renderer_style(n, RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None))
        .unwrap();
    e.request_native_control_geometry(n, role, width).unwrap();
}

#[test]
fn captured_theme_width_uses_current_native_font_attributes_axes_and_layout_contexts() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for writing in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for (display, columns) in [
                (Display::Block, false),
                (Display::Flex, false),
                (Display::Grid, false),
                (Display::Block, true),
            ] {
                let mut e = engine(scale);
                let parent = e.create_element(ElementTag::Div).unwrap();
                authored(&mut e, parent, RendererStyleValue::Display(display));
                authored(&mut e, parent, RendererStyleValue::Width(Length::px(500.0)));
                if columns {
                    authored(&mut e, parent, RendererStyleValue::ColumnCount(Some(2)));
                    authored(
                        &mut e,
                        parent,
                        RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
                    );
                    authored(
                        &mut e,
                        parent,
                        RendererStyleValue::Height(Length::px(200.0)),
                    );
                }
                let root = e.root();
                e.append_child(root, parent).unwrap();
                let ordinary = control(&mut e, parent, ElementTag::TextArea);
                let captured = control(&mut e, parent, ElementTag::TextArea);
                for n in [ordinary, captured] {
                    authored(&mut e, n, RendererStyleValue::WritingMode(writing));
                }
                for width in [
                    ScrollbarWidth::Auto,
                    ScrollbarWidth::Thin,
                    ScrollbarWidth::None,
                ] {
                    authored(&mut e, ordinary, RendererStyleValue::ScrollbarWidth(width));
                    capture(&mut e, captured, FormControlRole::TextArea, width);
                    assert_eq!(
                        e.computed_style(captured).unwrap().scrollbar_width,
                        ScrollbarWidth::None
                    );
                    assert_eq!(natural(&mut e, captured), natural(&mut e, ordinary));
                    for n in [ordinary, captured] {
                        e.set_attribute(n, "cols", "7").unwrap();
                        e.set_attribute(n, "rows", "3").unwrap();
                        authored(
                            &mut e,
                            n,
                            RendererStyleValue::FontFamily(FontFamilyList::single("DejaVu Sans")),
                        );
                        authored(&mut e, n, RendererStyleValue::FontSize(20.0));
                        authored(
                            &mut e,
                            n,
                            RendererStyleValue::LineHeight(LineHeight::Length(24.0)),
                        );
                    }
                    assert_eq!(natural(&mut e, captured), natural(&mut e, ordinary));
                }
            }
        }
    }
}

#[test]
fn capture_source_survives_canonical_width_removal_effects_and_true_equal_requests() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut e = engine(scale);
        let root = e.root();
        let ordinary = control(&mut e, root, ElementTag::TextArea);
        let captured = control(&mut e, root, ElementTag::TextArea);
        capture(
            &mut e,
            captured,
            FormControlRole::TextArea,
            ScrollbarWidth::Auto,
        );
        let baseline = natural(&mut e, ordinary);
        assert_eq!(natural(&mut e, captured), baseline);
        authored(
            &mut e,
            captured,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Thin),
        );
        authored(
            &mut e,
            ordinary,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Thin),
        );
        assert_eq!(natural(&mut e, captured), natural(&mut e, ordinary));
        assert!(e
            .remove_property(captured, StyleProperty::ScrollbarWidth)
            .unwrap());
        assert_eq!(natural(&mut e, captured), baseline);
        let endpoint =
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Thin));
        let id = e
            .animate(
                captured,
                PropertyKeyframes::typed(
                    StyleProperty::ScrollbarWidth,
                    Keyframes::from_values(endpoint.clone(), endpoint),
                )
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
                AnimationTimeline::Document,
            )
            .unwrap();
        assert_eq!(natural(&mut e, captured), natural(&mut e, ordinary));
        e.cancel_animation(id).unwrap();
        assert_eq!(natural(&mut e, captured), baseline);
        let s = e.scene().unwrap();
        let generation = s.generation();
        let stats = e.stats();
        let dirty = e.dirty_generations();
        e.request_native_control_geometry(
            captured,
            FormControlRole::TextArea,
            ScrollbarWidth::Auto,
        )
        .unwrap();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
        assert_eq!(e.dirty_generations(), dirty);
        // Presentation was already None; a real snapshot source write must
        // still replace its separately owned Auto metric operand.
        e.set_renderer_style(
            captured,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        )
        .unwrap();
        authored(
            &mut e,
            ordinary,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        );
        assert_eq!(natural(&mut e, captured), natural(&mut e, ordinary));
    }
}

#[test]
fn native_default_context_inherits_fonts_and_clone_owns_its_source() {
    let mut e = engine(1.0);
    let parent = e.create_element(ElementTag::Div).unwrap();
    let root = e.root();
    e.append_child(root, parent).unwrap();
    authored(
        &mut e,
        parent,
        RendererStyleValue::FontFamily(FontFamilyList::single("Ahem")),
    );
    let original = e.create_element(ElementTag::Input).unwrap();
    e.set_attribute(original, "id", "original").unwrap();
    e.append_child(parent, original).unwrap();
    e.request_native_control_geometry(original, FormControlRole::TextInput, ScrollbarWidth::Auto)
        .unwrap();
    let clone = e.clone_subtree(original).unwrap();
    e.set_attribute(clone, "id", "clone").unwrap();
    e.append_child(parent, clone).unwrap();
    let before = natural(&mut e, original);
    authored(&mut e, parent, RendererStyleValue::FontSize(20.0));
    assert_ne!(natural(&mut e, original), before);
    assert_eq!(natural(&mut e, original), natural(&mut e, clone));
    e.set_attribute(clone, "size", "3").unwrap();
    assert_ne!(natural(&mut e, original), natural(&mut e, clone));
    e.remove(original).unwrap();
    assert_eq!(
        e.request_native_control_geometry(
            original,
            FormControlRole::TextInput,
            ScrollbarWidth::Auto
        ),
        Err(EngineError::StaleHandle)
    );
    let fresh = control(&mut e, parent, ElementTag::TextArea);
    e.set_renderer_style(
        fresh,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    )
    .unwrap();
    assert_eq!(fresh.index(), original.index());
    assert_ne!(fresh, original);
    let natural_none = natural(&mut e, fresh);
    e.request_native_control_geometry(fresh, FormControlRole::TextArea, ScrollbarWidth::Auto)
        .unwrap();
    let natural_auto = natural(&mut e, fresh);
    assert_eq!(natural_auto.0, natural_none.0.map(|width| width + 15.0));
    assert_eq!(natural_auto.1, natural_none.1);
}

#[test]
fn explicit_renderer_and_resource_geometry_never_becomes_native_from_matching_bytes() {
    let mut e = engine(1.0);
    let root = e.root();
    for explicit in [
        None,
        Some(ReplacedContent {
            resource: ReplacedResourceKind::TransparentCanvas,
            intrinsic_width: Some(7.0),
            intrinsic_height: Some(11.0),
            intrinsic_ratio: None,
        }),
    ] {
        let n = control(&mut e, root, ElementTag::TextArea);
        capture(&mut e, n, FormControlRole::TextArea, ScrollbarWidth::Auto);
        e.set_renderer_node_state(n, RendererNodeState::Replaced(explicit))
            .unwrap();
        let before = metadata(&mut e, n);
        let dirty = e.dirty_generations();
        assert!(e
            .request_native_control_geometry(n, FormControlRole::TextArea, ScrollbarWidth::Auto)
            .is_err());
        assert_eq!(content_fields(metadata(&mut e, n)), content_fields(before));
        assert_eq!(e.dirty_generations(), dirty);
    }
    let n = control(&mut e, root, ElementTag::TextArea);
    capture(&mut e, n, FormControlRole::TextArea, ScrollbarWidth::Auto);
    let before = e.control_state(n).unwrap().unwrap().clone();
    assert_eq!(
        e.set_image_resource(n, ImageResourceId::new(u32::MAX), None),
        Err(EngineError::UnknownResource)
    );
    assert_eq!(e.control_state(n).unwrap().unwrap(), &before);
    const SVG: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><rect width='1' height='1' fill='green'/></svg>";
    let image = e.register_image_resource(
        "owned-test.svg",
        "image/svg+xml",
        "5feefdeb641a28d864e88a8b6007ed13a2dc6f145b813caf08a2b595db180e8a",
        SVG.to_vec(),
    );
    e.set_image_resource(n, image, Some((37.0, 19.0))).unwrap();
    assert!(e
        .request_native_control_geometry(n, FormControlRole::TextArea, ScrollbarWidth::Auto)
        .is_err());
    assert_eq!(natural(&mut e, n), (Some(37.0), Some(19.0)));
}

#[test]
fn requests_validate_document_live_role_and_preserve_explicit_dimensions() {
    let mut e = engine(1.0);
    let mut other = engine(1.0);
    let root = e.root();
    let n = control(&mut e, root, ElementTag::Input);
    let foreign = other.create_element(ElementTag::Input).unwrap();
    assert_eq!(
        e.request_native_control_geometry(
            foreign,
            FormControlRole::TextInput,
            ScrollbarWidth::Auto
        ),
        Err(EngineError::WrongDocument)
    );
    let before = e.control_state(n).unwrap().unwrap().clone();
    assert!(e
        .request_native_control_geometry(n, FormControlRole::TextArea, ScrollbarWidth::Auto)
        .is_err());
    assert_eq!(e.control_state(n).unwrap().unwrap(), &before);
    e.request_native_control_geometry(n, FormControlRole::TextInput, ScrollbarWidth::Auto)
        .unwrap();
    for (p, v) in [
        (StyleProperty::Width, "80px"),
        (StyleProperty::Height, "30px"),
    ] {
        e.set_property(n, p, parse_literal(p, v).unwrap()).unwrap();
    }
    e.set_attribute(n, "size", "7").unwrap();
    authored(&mut e, n, RendererStyleValue::FontSize(20.0));
    let bounds = e.bounds(n).unwrap().unwrap();
    assert_eq!((bounds.width, bounds.height), (80.0, 30.0));
    e.set_attribute(n, "type", "range").unwrap();
    assert!(e
        .request_native_control_geometry(n, FormControlRole::TextInput, ScrollbarWidth::Auto)
        .is_err());
    e.remove_attribute(n, "type").unwrap();
    e.request_native_control_geometry(n, FormControlRole::TextInput, ScrollbarWidth::None)
        .unwrap();
    let bounds = e.bounds(n).unwrap().unwrap();
    assert_eq!((bounds.width, bounds.height), (80.0, 30.0));
}
