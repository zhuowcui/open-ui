//! Native application guards for text sizing across layout entry points.
//! These use owned Engine nodes and typed styles, without markup or a browser
//! runtime. The independent Chromium/pixel gates qualify the final result.

use openui_dom::ElementTag;
use openui_engine::{Engine, FontCollection, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    Direction, Display, FontFamily, FontFamilyList, FontVariantLigatures, LengthValue,
    RendererStyleValue, StyleProperty, StyleValue, TextTransform, TextWrapMode, UnicodeBidi,
    WhiteSpaceCollapse, WhiteSpaceShorthand,
};

struct TextContainers {
    boxes: [NodeHandle; 3],
    texts: [NodeHandle; 3],
}

fn engine_with_families(families: &[&str]) -> Engine {
    let mut engine = Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(1024.0, 768.0, 1.0).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::FontFamily,
            FontFamilyList {
                families: families
                    .iter()
                    .map(|family| FontFamily::Named((*family).into()))
                    .collect(),
            }
            .into(),
        )
        .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::FontSize,
            LengthValue::px(16.0).into(),
        )
        .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::WhiteSpace,
            WhiteSpaceShorthand {
                collapse: WhiteSpaceCollapse::Collapse,
                wrap: TextWrapMode::Nowrap,
            }
            .into(),
        )
        .unwrap();
    engine
}

fn text_containers(engine: &mut Engine, text: &str) -> TextContainers {
    let inline = engine.create_native_element(ElementTag::Span).unwrap();
    engine
        .set_property(inline, StyleProperty::Display, Display::InlineBlock.into())
        .unwrap();
    let flex = engine.create_native_element(ElementTag::Div).unwrap();
    engine
        .set_property(flex, StyleProperty::Display, Display::InlineFlex.into())
        .unwrap();
    let table = engine.create_native_element(ElementTag::Table).unwrap();
    engine
        .set_property(table, StyleProperty::Display, Display::InlineTable.into())
        .unwrap();
    engine
        .set_property(
            table,
            StyleProperty::BorderSpacing,
            StyleValue::Renderer(RendererStyleValue::BorderSpacing((
                Length::px(0.0),
                Length::px(0.0),
            ))),
        )
        .unwrap();
    let boxes = [inline, flex, table];
    for node in boxes {
        engine
            .set_property(
                node,
                StyleProperty::Width,
                LengthValue::Computed(Length::max_content()).into(),
            )
            .unwrap();
        engine.append_child(engine.root(), node).unwrap();
    }
    let row = engine.create_native_element(ElementTag::TableRow).unwrap();
    let cell = engine.create_native_element(ElementTag::TableCell).unwrap();
    engine.append_child(table, row).unwrap();
    engine.append_child(row, cell).unwrap();
    let mut texts = Vec::new();
    for parent in [inline, flex, cell] {
        let node = engine.create_text(text).unwrap();
        engine.append_child(parent, node).unwrap();
        texts.push(node);
    }
    TextContainers {
        boxes,
        texts: texts.try_into().unwrap(),
    }
}

fn shared_width(engine: &mut Engine, nodes: &TextContainers) -> f32 {
    let widths = nodes
        .boxes
        .map(|node| engine.bounds(node).unwrap().unwrap().width);
    assert_eq!(
        widths[0], widths[1],
        "ordinary inline and anonymous flex text differ"
    );
    assert_eq!(
        widths[0], widths[2],
        "ordinary inline and table text differ"
    );
    widths[0]
}

#[test]
fn ahem_anonymous_flex_inline_and_table_size_from_shaped_advances() {
    for size in [10.0, 12.0, 16.0, 23.0] {
        let mut engine = engine_with_families(&["Ahem"]);
        engine
            .set_property(
                engine.root(),
                StyleProperty::FontSize,
                LengthValue::px(size).into(),
            )
            .unwrap();
        let nodes = text_containers(&mut engine, "XX");
        assert_eq!(shared_width(&mut engine, &nodes), size * 2.0);
        let stats = engine.stats();
        assert_eq!(shared_width(&mut engine, &nodes), size * 2.0);
        assert_eq!(
            engine.stats(),
            stats,
            "unchanged geometry reads must reuse layout"
        );
        for node in nodes.texts {
            assert_eq!(engine.text_data(node).unwrap(), "XX");
        }
    }
}

#[test]
fn native_spacing_and_text_mutations_recompute_each_text_entry_point() {
    let mut engine = engine_with_families(&["Ahem"]);
    engine
        .set_property(
            engine.root(),
            StyleProperty::LetterSpacing,
            StyleValue::Number(2.0),
        )
        .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::WordSpacing,
            StyleValue::Number(7.0),
        )
        .unwrap();
    let nodes = text_containers(&mut engine, "X X");
    assert_eq!(shared_width(&mut engine, &nodes), 61.0);
    for node in nodes.texts {
        engine.set_text(node, "XX").unwrap();
    }
    assert_eq!(shared_width(&mut engine, &nodes), 36.0);
    for node in nodes.texts {
        engine.set_text(node, "X X").unwrap();
    }
    assert_eq!(shared_width(&mut engine, &nodes), 61.0);
}

#[test]
fn native_ligature_controls_change_all_intrinsic_paths_together() {
    let mut engine = engine_with_families(&["DejaVu Sans"]);
    engine
        .set_property(
            engine.root(),
            StyleProperty::FontSize,
            LengthValue::px(48.0).into(),
        )
        .unwrap();
    let nodes = text_containers(&mut engine, "office");
    let with_ligatures = shared_width(&mut engine, &nodes);
    engine
        .set_property(
            engine.root(),
            StyleProperty::FontVariantLigatures,
            FontVariantLigatures::none().into(),
        )
        .unwrap();
    let without_ligatures = shared_width(&mut engine, &nodes);
    assert_ne!(
        with_ligatures, without_ligatures,
        "the guard must exercise a real ligature"
    );
    engine
        .set_property(
            engine.root(),
            StyleProperty::FontVariantLigatures,
            FontVariantLigatures::NORMAL.into(),
        )
        .unwrap();
    assert_eq!(shared_width(&mut engine, &nodes), with_ligatures);
}

#[test]
fn native_bidi_and_script_fallback_keep_anonymous_flex_in_sync() {
    for (direction, unicode_bidi) in [
        (Direction::Ltr, UnicodeBidi::Normal),
        (Direction::Rtl, UnicodeBidi::Normal),
        (Direction::Ltr, UnicodeBidi::Plaintext),
    ] {
        let mut engine = engine_with_families(&[
            "Ahem",
            "Droid Sans Fallback",
            "Noto Sans Devanagari",
            "Noto Color Emoji",
            "DejaVu Sans",
        ]);
        engine
            .set_property(engine.root(), StyleProperty::Direction, direction.into())
            .unwrap();
        engine
            .set_property(
                engine.root(),
                StyleProperty::UnicodeBidi,
                unicode_bidi.into(),
            )
            .unwrap();
        let nodes = text_containers(&mut engine, "Aकि丁😀אב");
        assert!(shared_width(&mut engine, &nodes) > 0.0);
    }
}

#[test]
fn native_min_content_uses_shaped_soft_wrap_segments() {
    let mut engine = engine_with_families(&["Ahem"]);
    engine
        .set_property(
            engine.root(),
            StyleProperty::WhiteSpace,
            WhiteSpaceShorthand::default().into(),
        )
        .unwrap();
    let nodes = text_containers(&mut engine, "XX XXX");
    assert_eq!(shared_width(&mut engine, &nodes), 96.0);
    for node in nodes.boxes {
        engine
            .set_property(
                node,
                StyleProperty::Width,
                LengthValue::Computed(Length::min_content()).into(),
            )
            .unwrap();
    }
    assert_eq!(shared_width(&mut engine, &nodes), 48.0);
}

#[test]
fn native_transform_and_forced_lines_leave_owned_text_unchanged() {
    let mut engine = engine_with_families(&["Ahem"]);
    engine
        .set_property(
            engine.root(),
            StyleProperty::WhiteSpace,
            WhiteSpaceShorthand {
                collapse: WhiteSpaceCollapse::Preserve,
                wrap: TextWrapMode::Nowrap,
            }
            .into(),
        )
        .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::TextTransform,
            TextTransform::Uppercase.into(),
        )
        .unwrap();
    let nodes = text_containers(&mut engine, "x\nxx");
    for _ in 0..2 {
        assert_eq!(shared_width(&mut engine, &nodes), 32.0);
        for node in nodes.texts {
            assert_eq!(engine.text_data(node).unwrap(), "x\nxx");
        }
    }
}
