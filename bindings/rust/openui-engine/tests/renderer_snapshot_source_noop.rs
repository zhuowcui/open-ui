//! Derived-equal renderer inputs must become permanent snapshot sources.
//! Native geometry/state guards; external Chromium pixels remain separate.
use openui_dom::{ElementTag, FormControlRole, ReplacedContent, ReplacedResourceKind};
use openui_engine::{Engine, NodeHandle, RendererNodeState, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    BorderStyle, BoxSizing, Display, Overflow, RendererStyleValue, Resize, ScrollbarWidth,
    StyleProperty, StyleValue,
};
const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn engine(scale: f64) -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap()).unwrap()
}
fn set(e: &mut Engine, n: NodeHandle, value: RendererStyleValue) {
    e.set_renderer_style(n, value).unwrap();
}
fn area(e: &mut Engine) -> NodeHandle {
    let n = e.create_element(ElementTag::TextArea).unwrap();
    set(
        e,
        n,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    );
    set(e, n, RendererStyleValue::Display(Display::InlineBlock));
    assert_eq!(e.computed_style(n).unwrap().overflow_x, Overflow::Auto);
    assert_eq!(e.computed_style(n).unwrap().overflow_y, Overflow::Auto);
    n
}
fn metadata(e: &mut Engine, n: NodeHandle, width: f32, height: f32) {
    e.set_renderer_node_state(
        n,
        RendererNodeState::Replaced(Some(ReplacedContent {
            resource: ReplacedResourceKind::TransparentCanvas,
            intrinsic_width: Some(width),
            intrinsic_height: Some(height),
            intrinsic_ratio: None,
        })),
    )
    .unwrap();
}
fn border(e: &mut Engine, n: NodeHandle, width: i32) {
    set(e, n, RendererStyleValue::BorderTopWidth(width));
    set(e, n, RendererStyleValue::BorderRightWidth(width));
    set(e, n, RendererStyleValue::BorderBottomWidth(width));
    set(e, n, RendererStyleValue::BorderLeftWidth(width));
    set(e, n, RendererStyleValue::BorderTopStyle(BorderStyle::Inset));
    set(
        e,
        n,
        RendererStyleValue::BorderRightStyle(BorderStyle::Inset),
    );
    set(
        e,
        n,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Inset),
    );
    set(
        e,
        n,
        RendererStyleValue::BorderLeftStyle(BorderStyle::Inset),
    );
}
fn idle(e: &mut Engine, n: NodeHandle, value: RendererStyleValue) {
    let generation = e.scene().unwrap().generation();
    let before = e.stats();
    set(e, n, value);
    assert_eq!(e.scene().unwrap().generation(), generation);
    assert_eq!(e.stats(), before);
}
#[test]
fn derived_equal_auto_survives_metadata_replacement_and_flexes_the_owned_box() {
    for scale in SCALES {
        let mut e = engine(scale);
        let columns = e.create_native_element(ElementTag::Div).unwrap();
        set(
            &mut e,
            columns,
            RendererStyleValue::Width(Length::px(760.0)),
        );
        set(&mut e, columns, RendererStyleValue::ColumnCount(Some(2)));
        set(
            &mut e,
            columns,
            RendererStyleValue::ColumnGap(Some(Length::px(16.0))),
        );
        let root = e.root();
        e.append_child(root, columns).unwrap();
        let flex = e.create_native_element(ElementTag::Div).unwrap();
        set(&mut e, flex, RendererStyleValue::Display(Display::Flex));
        e.append_child(columns, flex).unwrap();
        let textarea = area(&mut e);
        set(
            &mut e,
            textarea,
            RendererStyleValue::BoxSizing(BoxSizing::BorderBox),
        );
        set(&mut e, textarea, RendererStyleValue::Resize(Resize::Both));
        // Presentation already equals Auto; these are nevertheless explicit inputs.
        set(
            &mut e,
            textarea,
            RendererStyleValue::OverflowX(Overflow::Auto),
        );
        set(
            &mut e,
            textarea,
            RendererStyleValue::OverflowY(Overflow::Auto),
        );
        border(&mut e, textarea, 1);
        metadata(&mut e, textarea, 176.0, 28.0);
        set(
            &mut e,
            textarea,
            RendererStyleValue::BoxSizing(BoxSizing::ContentBox),
        );
        assert_eq!(
            e.computed_style(textarea).unwrap().overflow_x,
            Overflow::Auto
        );
        assert_eq!(
            e.computed_style(textarea).unwrap().overflow_y,
            Overflow::Auto
        );
        e.append_child(flex, textarea).unwrap();
        let input = e.create_native_element(ElementTag::Input).unwrap();
        e.set_renderer_node_state(
            input,
            RendererNodeState::FormControl(Some(FormControlRole::TextInput)),
        )
        .unwrap();
        metadata(&mut e, input, 262.0, 14.0);
        set(
            &mut e,
            input,
            RendererStyleValue::BoxSizing(BoxSizing::ContentBox),
        );
        border(&mut e, input, 2);
        e.append_child(flex, input).unwrap();
        assert_eq!(e.bounds(flex).unwrap().unwrap().width, 372.0);
        assert_eq!(e.bounds(textarea).unwrap().unwrap().width, 106.0);
        assert_eq!(e.bounds(input).unwrap().unwrap().width, 266.0);
        idle(
            &mut e,
            textarea,
            RendererStyleValue::OverflowX(Overflow::Auto),
        );
        idle(
            &mut e,
            textarea,
            RendererStyleValue::OverflowY(Overflow::Auto),
        );
        // A real source change still restores its ordinary content-based minimum.
        set(
            &mut e,
            textarea,
            RendererStyleValue::OverflowX(Overflow::Visible),
        );
        set(
            &mut e,
            textarea,
            RendererStyleValue::OverflowY(Overflow::Visible),
        );
        assert_eq!(e.bounds(textarea).unwrap().unwrap().width, 178.0);
    }
}
#[test]
fn derived_equal_none_is_distinct_from_unset_contents_source() {
    for scale in SCALES {
        let mut e = engine(scale);
        for explicit_none in [false, true] {
            let n = area(&mut e);
            set(&mut e, n, RendererStyleValue::Display(Display::Contents));
            assert_eq!(e.computed_style(n).unwrap().display, Display::None);
            if explicit_none {
                set(&mut e, n, RendererStyleValue::Display(Display::None));
            }
            metadata(&mut e, n, 40.0, 20.0);
            set(&mut e, n, RendererStyleValue::Width(Length::px(47.0)));
            assert_eq!(
                e.computed_style(n).unwrap().display,
                if explicit_none {
                    Display::None
                } else {
                    Display::Contents
                }
            );
            let root = e.root();
            e.append_child(root, n).unwrap();
            if explicit_none {
                assert!(e.bounds(n).unwrap().is_none());
                idle(&mut e, n, RendererStyleValue::Display(Display::None));
            }
        }
    }
}
#[test]
fn authored_inputs_and_reused_generational_slots_keep_their_owners() {
    let mut e = engine(1.0);
    let n = area(&mut e);
    e.set_property(
        n,
        StyleProperty::Overflow,
        StyleValue::Overflow(Overflow::Auto),
    )
    .unwrap();
    // Snapshot input freezes the surviving declarations before clearing their owners.
    set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Auto));
    assert!(!e.remove_property(n, StyleProperty::Overflow).unwrap());
    metadata(&mut e, n, 40.0, 20.0);
    set(&mut e, n, RendererStyleValue::Width(Length::px(47.0)));
    assert_eq!(e.computed_style(n).unwrap().overflow_x, Overflow::Auto);
    assert_eq!(e.computed_style(n).unwrap().overflow_y, Overflow::Auto);
    e.remove(n).unwrap();
    let fresh = e.create_native_element(ElementTag::Div).unwrap();
    let generation = e.scene().unwrap().generation();
    let before = e.stats();
    assert!(e
        .set_renderer_style(n, RendererStyleValue::Display(Display::None))
        .is_err());
    assert_eq!(e.computed_style(fresh).unwrap().display, Display::Block);
    assert_eq!(e.scene().unwrap().generation(), generation);
    assert_eq!(e.stats(), before);
}
