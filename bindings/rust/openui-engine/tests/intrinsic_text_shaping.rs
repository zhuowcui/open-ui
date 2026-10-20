//! Native application guards for text sizing across layout entry points.
//! These use owned Engine nodes and typed styles, without markup or a browser
//! runtime. The independent Chromium/pixel gates qualify the final result.

use openui_dom::ElementTag;
use openui_engine::{Engine, FontCollection, NodeHandle, ViewportMetrics};
use openui_geometry::{LayoutUnit, Length};
use openui_style::{
    Direction, Display, FontFamily, FontFamilyList, FontVariantLigatures, LengthValue, LineBreak,
    OverflowWrap, RendererStyleValue, StyleProperty, StyleValue, TextTransform, TextWrapMode,
    UnicodeBidi, WhiteSpaceCollapse, WhiteSpaceShorthand, WordBreak,
};
use openui_text::{Font, FontDescription, TextDirection, TextShaper};

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

// Independent whole-run shaping supplies the source SnappedWidth contract.
// These are strict native data-flow guards, not new Chromium observations;
// the repeated .50 atomic-word geometry is retained in native_font_units.
fn shaped_snapped_width(engine: &Engine, text: &str) -> f32 {
    let description =
        FontDescription::from_computed_style(engine.computed_style(engine.root()).unwrap());
    let font = Font::new_in_collection(description, engine.font_collection().clone());
    let shape = TextShaper::new().shape(text, &font, TextDirection::Ltr);
    LayoutUnit::from_f32_ceil(shape.width).to_f32()
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
        let expected = shaped_snapped_width(&engine, "XX");
        assert_eq!(shared_width(&mut engine, &nodes), expected);
        let stats = engine.stats();
        assert_eq!(shared_width(&mut engine, &nodes), expected);
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
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "linux-oracle: FreeType fractional advance of loaded Ahem"
)]
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
    let spaced = shaped_snapped_width(&engine, "X X");
    assert!(
        spaced > 61.0,
        "the guard must retain a fractional shaped advance"
    );
    assert_eq!(shared_width(&mut engine, &nodes), spaced);
    for node in nodes.texts {
        engine.set_text(node, "XX").unwrap();
    }
    let unspaced = shaped_snapped_width(&engine, "XX");
    assert_eq!(shared_width(&mut engine, &nodes), unspaced);
    for node in nodes.texts {
        engine.set_text(node, "X X").unwrap();
    }
    assert_eq!(shared_width(&mut engine, &nodes), spaced);
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
    let maximum = shaped_snapped_width(&engine, "XX XXX");
    assert_eq!(shared_width(&mut engine, &nodes), maximum);
    for node in nodes.boxes {
        engine
            .set_property(
                node,
                StyleProperty::Width,
                LengthValue::Computed(Length::min_content()).into(),
            )
            .unwrap();
    }
    let minimum = shaped_snapped_width(&engine, "XXX");
    assert_eq!(shared_width(&mut engine, &nodes), minimum);
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
        let expected = shaped_snapped_width(&engine, "XX");
        assert_eq!(shared_width(&mut engine, &nodes), expected);
        for node in nodes.texts {
            assert_eq!(engine.text_data(node).unwrap(), "x\nxx");
        }
    }
}

fn set_intrinsic_width(engine: &mut Engine, nodes: &TextContainers, min_content: bool) -> f32 {
    let width = if min_content {
        Length::min_content()
    } else {
        Length::max_content()
    };
    for node in nodes.boxes {
        engine
            .set_property(
                node,
                StyleProperty::Width,
                LengthValue::Computed(width).into(),
            )
            .unwrap();
    }
    shared_width(engine, nodes)
}

fn permit_native_wrap(engine: &mut Engine) {
    engine
        .set_property(
            engine.root(),
            StyleProperty::WhiteSpace,
            WhiteSpaceShorthand::default().into(),
        )
        .unwrap();
}

#[test]
fn native_cjk_normal_keep_all_and_anywhere_share_min_content_with_stable_max_content() {
    let mut engine = engine_with_families(&["Ahem", "Droid Sans Fallback"]);
    permit_native_wrap(&mut engine);
    let nodes = text_containers(&mut engine, "丁丁");
    let max = shared_width(&mut engine, &nodes);
    let normal = set_intrinsic_width(&mut engine, &nodes, true);
    assert!(
        normal > 0.0 && normal < max,
        "normal CJK must retain a legal soft wrap"
    );
    engine
        .set_property(
            engine.root(),
            StyleProperty::WordBreak,
            WordBreak::KeepAll.into(),
        )
        .unwrap();
    assert_eq!(shared_width(&mut engine, &nodes), max);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), max);
    engine
        .set_property(
            engine.root(),
            StyleProperty::LineBreak,
            LineBreak::Anywhere.into(),
        )
        .unwrap();
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, true), normal);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), max);
    for text in nodes.texts {
        assert_eq!(engine.text_data(text).unwrap(), "丁丁");
    }
}

#[test]
fn native_strict_small_kana_and_anywhere_use_the_same_contextual_shaped_ranges() {
    let mut engine = engine_with_families(&["Droid Sans Fallback"]);
    permit_native_wrap(&mut engine);
    engine
        .set_property(
            engine.root(),
            StyleProperty::WordBreak,
            WordBreak::BreakAll.into(),
        )
        .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::LineBreak,
            LineBreak::Normal.into(),
        )
        .unwrap();
    let nodes = text_containers(&mut engine, "丁ぁ");
    let max = shared_width(&mut engine, &nodes);
    let normal = set_intrinsic_width(&mut engine, &nodes, true);
    assert!(normal > 0.0 && normal < max);
    engine
        .set_property(
            engine.root(),
            StyleProperty::LineBreak,
            LineBreak::Strict.into(),
        )
        .unwrap();
    assert_eq!(
        shared_width(&mut engine, &nodes),
        max,
        "strict breaking keeps the small kana with its preceding character"
    );
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), max);
    engine
        .set_property(
            engine.root(),
            StyleProperty::LineBreak,
            LineBreak::Anywhere.into(),
        )
        .unwrap();
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, true), normal);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), max);
}

#[test]
fn native_hyphen_nbsp_and_emergency_wrap_neighbors_preserve_css_min_content_rules() {
    let mut engine = engine_with_families(&["Ahem"]);
    permit_native_wrap(&mut engine);
    let nodes = text_containers(&mut engine, "XX-XXX");
    let hyphen_maximum = shaped_snapped_width(&engine, "XX-XXX");
    let hyphen_minimum = shaped_snapped_width(&engine, "XXX");
    assert_eq!(shared_width(&mut engine, &nodes), hyphen_maximum);
    assert_eq!(
        set_intrinsic_width(&mut engine, &nodes, true),
        hyphen_minimum
    );
    assert_eq!(
        set_intrinsic_width(&mut engine, &nodes, false),
        hyphen_maximum
    );
    for text in nodes.texts {
        engine.set_text(text, "X\u{00a0}X").unwrap();
    }
    let nbsp = shaped_snapped_width(&engine, "X\u{00a0}X");
    assert_eq!(shared_width(&mut engine, &nodes), nbsp);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, true), nbsp);
    for text in nodes.texts {
        engine.set_text(text, "XXXX").unwrap();
    }
    engine
        .set_property(
            engine.root(),
            StyleProperty::OverflowWrap,
            OverflowWrap::BreakWord.into(),
        )
        .unwrap();
    let maximum = shaped_snapped_width(&engine, "XXXX");
    let character = shaped_snapped_width(&engine, "X");
    assert_eq!(
        shared_width(&mut engine, &nodes),
        maximum,
        "break-word emergency wraps do not lower min-content"
    );
    engine
        .set_property(
            engine.root(),
            StyleProperty::OverflowWrap,
            OverflowWrap::Anywhere.into(),
        )
        .unwrap();
    assert_eq!(shared_width(&mut engine, &nodes), character);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), maximum);
    engine
        .set_property(
            engine.root(),
            StyleProperty::OverflowWrap,
            OverflowWrap::Normal.into(),
        )
        .unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::WordBreak,
            WordBreak::BreakWord.into(),
        )
        .unwrap();
    assert_eq!(
        set_intrinsic_width(&mut engine, &nodes, true),
        character,
        "the legacy word-break alias contributes character opportunities to min-content"
    );
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), maximum);
}

#[test]
fn native_break_spaces_retains_space_units_and_bidi_fallback_boundaries_do_not_hide_wraps() {
    let mut engine = engine_with_families(&["Ahem"]);
    engine
        .set_property(
            engine.root(),
            StyleProperty::WhiteSpace,
            WhiteSpaceShorthand {
                collapse: WhiteSpaceCollapse::BreakSpaces,
                wrap: TextWrapMode::Wrap,
            }
            .into(),
        )
        .unwrap();
    let nodes = text_containers(&mut engine, "X  X");
    let maximum = shaped_snapped_width(&engine, "X  X");
    let minimum = shaped_snapped_width(&engine, "X ");
    assert_eq!(shared_width(&mut engine, &nodes), maximum);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, true), minimum);
    for text in nodes.texts {
        engine.set_text(text, " ").unwrap();
    }
    let space = shaped_snapped_width(&engine, " ");
    assert_eq!(shared_width(&mut engine, &nodes), space);
    assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), space);

    for direction in [Direction::Ltr, Direction::Rtl] {
        let mut engine = engine_with_families(&["Ahem", "Droid Sans Fallback", "DejaVu Sans"]);
        permit_native_wrap(&mut engine);
        engine
            .set_property(engine.root(), StyleProperty::Direction, direction.into())
            .unwrap();
        let nodes = text_containers(&mut engine, "丁A丁 אב");
        let max = shared_width(&mut engine, &nodes);
        let min = set_intrinsic_width(&mut engine, &nodes, true);
        assert!(
            min > 0.0 && min < max,
            "font/script/bidi run boundaries must preserve legal wraps"
        );
        assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), max);
    }
}

#[test]
fn leading_collapsed_spaces_and_forced_lines_share_native_intrinsic_paths() {
    for (collapse, wrap, text, measured) in [
        (WhiteSpaceCollapse::Collapse, TextWrapMode::Wrap, " X ", "X"),
        (
            WhiteSpaceCollapse::Collapse,
            TextWrapMode::Nowrap,
            " X ",
            "X",
        ),
        (
            WhiteSpaceCollapse::PreserveBreaks,
            TextWrapMode::Wrap,
            " X \n XX ",
            "XX",
        ),
        (
            WhiteSpaceCollapse::Collapse,
            TextWrapMode::Wrap,
            "\u{00a0}X",
            "\u{00a0}X",
        ),
    ] {
        let mut engine = engine_with_families(&["Ahem"]);
        engine
            .set_property(
                engine.root(),
                StyleProperty::WhiteSpace,
                WhiteSpaceShorthand { collapse, wrap }.into(),
            )
            .unwrap();
        let nodes = text_containers(&mut engine, text);
        let expected = shaped_snapped_width(&engine, measured);
        assert_eq!(shared_width(&mut engine, &nodes), expected);
        assert_eq!(set_intrinsic_width(&mut engine, &nodes, true), expected);
        assert_eq!(set_intrinsic_width(&mut engine, &nodes, false), expected);
        for text_node in nodes.texts {
            assert_eq!(engine.text_data(text_node).unwrap(), text);
        }
        let generation = engine.scene().unwrap().generation();
        assert_eq!(shared_width(&mut engine, &nodes), expected);
        assert_eq!(engine.scene().unwrap().generation(), generation);
    }
}

#[test]
fn inline_edges_and_empty_tags_keep_leading_state_but_atomic_content_ends_it() {
    let mut engine = engine_with_families(&["Ahem"]);
    permit_native_wrap(&mut engine);
    let host = engine.create_native_element(ElementTag::Span).unwrap();
    engine
        .set_property(host, StyleProperty::Display, Display::InlineBlock.into())
        .unwrap();
    engine
        .set_property(
            host,
            StyleProperty::Width,
            LengthValue::Computed(Length::max_content()).into(),
        )
        .unwrap();
    engine.append_child(engine.root(), host).unwrap();
    let leading = engine.create_text(" ").unwrap();
    engine.append_child(host, leading).unwrap();
    let edge = engine.create_native_element(ElementTag::Span).unwrap();
    engine
        .set_property(
            edge,
            StyleProperty::PaddingLeft,
            LengthValue::px(3.0).into(),
        )
        .unwrap();
    engine
        .set_property(
            edge,
            StyleProperty::PaddingRight,
            LengthValue::px(2.0).into(),
        )
        .unwrap();
    engine.append_child(host, edge).unwrap();
    let empty = engine.create_native_element(ElementTag::Span).unwrap();
    engine.append_child(edge, empty).unwrap();
    let text = engine.create_text(" X ").unwrap();
    engine.append_child(edge, text).unwrap();
    let trailing = engine.create_text(" ").unwrap();
    engine.append_child(host, trailing).unwrap();
    let expected = shaped_snapped_width(&engine, "X") + 5.0;
    assert_eq!(engine.bounds(host).unwrap().unwrap().width, expected);
    engine
        .set_property(
            host,
            StyleProperty::Width,
            LengthValue::Computed(Length::min_content()).into(),
        )
        .unwrap();
    assert_eq!(engine.bounds(host).unwrap().unwrap().width, expected);
    assert_eq!(engine.text_data(text).unwrap(), " X ");

    // Only the omitted prefix opportunity disappears. A real interior space
    // still breaks the minimum, while maximum width retains both words.
    engine.set_text(text, " X X ").unwrap();
    assert_eq!(
        engine.bounds(host).unwrap().unwrap().width,
        shaped_snapped_width(&engine, "X") + 3.0
    );
    engine
        .set_property(
            host,
            StyleProperty::Width,
            LengthValue::Computed(Length::max_content()).into(),
        )
        .unwrap();
    assert_eq!(
        engine.bounds(host).unwrap().unwrap().width,
        shaped_snapped_width(&engine, "X X") + 5.0
    );
    engine.set_text(text, " X ").unwrap();
    assert_eq!(engine.bounds(host).unwrap().unwrap().width, expected);

    // Even a real zero-advance atomic ends the Leading whitespace state.
    // Its following space is interior in max-content and belongs to the
    // flattened sequence through undecorated inline wrappers as well.
    for width in [0.0_f32, 4.0] {
        for nested_text in [false, true] {
            let atomic_host = engine.create_native_element(ElementTag::Span).unwrap();
            engine
                .set_property(
                    atomic_host,
                    StyleProperty::Display,
                    Display::InlineBlock.into(),
                )
                .unwrap();
            engine
                .set_property(
                    atomic_host,
                    StyleProperty::Width,
                    LengthValue::Computed(Length::max_content()).into(),
                )
                .unwrap();
            engine.append_child(engine.root(), atomic_host).unwrap();
            let atomic = engine.create_native_element(ElementTag::Span).unwrap();
            engine
                .set_property(atomic, StyleProperty::Display, Display::InlineBlock.into())
                .unwrap();
            engine
                .set_property(atomic, StyleProperty::Width, LengthValue::px(width).into())
                .unwrap();
            engine.append_child(atomic_host, atomic).unwrap();
            let text_parent = if nested_text {
                let inline = engine.create_native_element(ElementTag::Span).unwrap();
                engine
                    .set_property(inline, StyleProperty::Display, Display::Inline.into())
                    .unwrap();
                engine.append_child(atomic_host, inline).unwrap();
                inline
            } else {
                atomic_host
            };
            let text = engine.create_text(" X").unwrap();
            engine.append_child(text_parent, text).unwrap();
            assert_eq!(
                engine.bounds(atomic_host).unwrap().unwrap().width,
                width + shaped_snapped_width(&engine, " X")
            );
            engine
                .set_property(
                    atomic_host,
                    StyleProperty::Width,
                    LengthValue::Computed(Length::min_content()).into(),
                )
                .unwrap();
            assert_eq!(
                engine.bounds(atomic_host).unwrap().unwrap().width,
                width.max(shaped_snapped_width(&engine, "X"))
            );
            assert_eq!(engine.text_data(text).unwrap(), " X");
            engine
                .set_property(
                    atomic_host,
                    StyleProperty::Width,
                    LengthValue::Computed(Length::max_content()).into(),
                )
                .unwrap();
            assert_eq!(
                engine.bounds(atomic_host).unwrap().unwrap().width,
                width + shaped_snapped_width(&engine, " X")
            );
        }
    }
}

#[test]
fn native_wbr_keeps_max_line_and_splits_min_content_even_under_nowrap() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = Engine::new_with_font_collection(
            ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
            FontCollection::deterministic_test(),
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
        for (control, expected_min, expected_max) in [
            (None, 100.0, 100.0),
            (Some(ElementTag::WordBreak), 60.0, 100.0),
            (Some(ElementTag::Break), 60.0, 60.0),
        ] {
            let host = engine.create_native_element(ElementTag::Span).unwrap();
            engine
                .set_property(host, StyleProperty::Display, Display::InlineBlock.into())
                .unwrap();
            engine.append_child(engine.root(), host).unwrap();
            for (index, width) in [40.0, 60.0].into_iter().enumerate() {
                if index == 1 {
                    if let Some(tag) = control {
                        let marker = engine.create_native_element(tag).unwrap();
                        engine.append_child(host, marker).unwrap();
                    }
                }
                let atomic = engine.create_native_element(ElementTag::Span).unwrap();
                engine
                    .set_property(atomic, StyleProperty::Display, Display::InlineBlock.into())
                    .unwrap();
                engine
                    .set_property(atomic, StyleProperty::Width, LengthValue::px(width).into())
                    .unwrap();
                engine
                    .set_property(atomic, StyleProperty::Height, LengthValue::px(10.0).into())
                    .unwrap();
                engine.append_child(host, atomic).unwrap();
            }
            for (width, expected) in [
                (Length::min_content(), expected_min),
                (Length::max_content(), expected_max),
            ] {
                engine
                    .set_property(
                        host,
                        StyleProperty::Width,
                        LengthValue::Computed(width).into(),
                    )
                    .unwrap();
                assert_eq!(engine.bounds(host).unwrap().unwrap().width, expected);
            }
        }
    }
}

#[test]
fn native_wbr_keeps_independent_leading_states_and_preserved_space_ranges() {
    let mut engine = engine_with_families(&["Ahem"]);
    for zero_atomic in [false, true] {
        for decorated_inline in [false, true] {
            let host = engine.create_native_element(ElementTag::Span).unwrap();
            engine
                .set_property(host, StyleProperty::Display, Display::InlineBlock.into())
                .unwrap();
            engine.append_child(engine.root(), host).unwrap();
            if zero_atomic {
                let atomic = engine.create_native_element(ElementTag::Span).unwrap();
                engine
                    .set_property(atomic, StyleProperty::Display, Display::InlineBlock.into())
                    .unwrap();
                engine
                    .set_property(atomic, StyleProperty::Width, LengthValue::px(0.0).into())
                    .unwrap();
                engine
                    .set_property(atomic, StyleProperty::Height, LengthValue::px(10.0).into())
                    .unwrap();
                engine.append_child(host, atomic).unwrap();
            }
            let marker = engine.create_native_element(ElementTag::WordBreak).unwrap();
            engine.append_child(host, marker).unwrap();
            let (text_parent, edges) = if decorated_inline {
                let inline = engine.create_native_element(ElementTag::Span).unwrap();
                engine
                    .set_property(inline, StyleProperty::Display, Display::Inline.into())
                    .unwrap();
                engine
                    .set_property(
                        inline,
                        StyleProperty::PaddingLeft,
                        LengthValue::px(3.0).into(),
                    )
                    .unwrap();
                engine
                    .set_property(
                        inline,
                        StyleProperty::PaddingRight,
                        LengthValue::px(2.0).into(),
                    )
                    .unwrap();
                engine.append_child(host, inline).unwrap();
                (inline, 5.0)
            } else {
                (host, 0.0)
            };
            let text = engine.create_text(" X").unwrap();
            engine.append_child(text_parent, text).unwrap();
            for (width, measured) in [
                (Length::min_content(), "X"),
                (Length::max_content(), if zero_atomic { " X" } else { "X" }),
            ] {
                engine
                    .set_property(
                        host,
                        StyleProperty::Width,
                        LengthValue::Computed(width).into(),
                    )
                    .unwrap();
                assert_eq!(
                    engine.bounds(host).unwrap().unwrap().width,
                    shaped_snapped_width(&engine, measured) + edges
                );
            }
            engine
                .set_property(
                    host,
                    StyleProperty::WhiteSpace,
                    WhiteSpaceShorthand {
                        collapse: WhiteSpaceCollapse::Preserve,
                        wrap: TextWrapMode::Nowrap,
                    }
                    .into(),
                )
                .unwrap();
            for width in [Length::min_content(), Length::max_content()] {
                engine
                    .set_property(
                        host,
                        StyleProperty::Width,
                        LengthValue::Computed(width).into(),
                    )
                    .unwrap();
                assert_eq!(
                    engine.bounds(host).unwrap().unwrap().width,
                    shaped_snapped_width(&engine, " X") + edges
                );
            }
            assert_eq!(engine.text_data(text).unwrap(), " X");
        }
    }
}

#[test]
fn native_wbr_preserves_clone_edges_on_both_min_content_segments() {
    let mut engine = engine_with_families(&["Ahem"]);
    let host = engine.create_native_element(ElementTag::Span).unwrap();
    engine
        .set_property(host, StyleProperty::Display, Display::InlineBlock.into())
        .unwrap();
    engine.append_child(engine.root(), host).unwrap();
    let inline = engine.create_native_element(ElementTag::Span).unwrap();
    engine
        .set_property(inline, StyleProperty::Display, Display::Inline.into())
        .unwrap();
    engine
        .set_property(
            inline,
            StyleProperty::PaddingLeft,
            LengthValue::px(3.0).into(),
        )
        .unwrap();
    engine
        .set_property(
            inline,
            StyleProperty::PaddingRight,
            LengthValue::px(2.0).into(),
        )
        .unwrap();
    engine
        .set_property(
            inline,
            StyleProperty::BoxDecorationBreak,
            StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(
                openui_style::BoxDecorationBreak::Clone,
            )),
        )
        .unwrap();
    engine.append_child(host, inline).unwrap();
    for (index, width) in [40.0, 60.0].into_iter().enumerate() {
        if index == 1 {
            let marker = engine.create_native_element(ElementTag::WordBreak).unwrap();
            engine.append_child(inline, marker).unwrap();
        }
        let atomic = engine.create_native_element(ElementTag::Span).unwrap();
        engine
            .set_property(atomic, StyleProperty::Display, Display::InlineBlock.into())
            .unwrap();
        engine
            .set_property(atomic, StyleProperty::Width, LengthValue::px(width).into())
            .unwrap();
        engine
            .set_property(atomic, StyleProperty::Height, LengthValue::px(10.0).into())
            .unwrap();
        engine.append_child(inline, atomic).unwrap();
    }
    for (width, expected) in [
        (Length::min_content(), 65.0),
        (Length::max_content(), 105.0),
    ] {
        engine
            .set_property(
                host,
                StyleProperty::Width,
                LengthValue::Computed(width).into(),
            )
            .unwrap();
        assert_eq!(engine.bounds(host).unwrap().unwrap().width, expected);
    }
}
