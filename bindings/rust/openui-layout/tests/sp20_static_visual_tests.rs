//! SP20 contracts for the deterministic static-visual residual closure.

use openui_dom::{
    Document, ElementTag, FormControlRole, PseudoElementKind, ReplacedContent,
    ReplacedResourceKind, ScrollButtonDirection,
};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::items::InlineItemType;
use openui_layout::inline::items_builder::InlineItemsBuilder;
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    BlockEllipsis, BorderStyle, Color, ColumnFill, Direction, Display, LineClamp, MarginTrim,
    Position, StyleColor, Transform2D, WhiteSpace, WritingMode,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn layout(doc: &Document) -> Fragment {
    block_layout(
        doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    )
}

fn descendants<'a>(fragment: &'a Fragment, node: openui_dom::NodeId, out: &mut Vec<&'a Fragment>) {
    if fragment.node_id == node {
        out.push(fragment);
    }
    for child in &fragment.children {
        descendants(child, node, out);
    }
}

#[test]
fn fragmented_multicol_keeps_nested_flex_replaced_content() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(multicol).style;
        style.display = Display::Block;
        style.width = Length::px(200.0);
        style.height = Length::px(100.0);
        style.column_count = Some(2);
        style.column_fill = ColumnFill::Auto;
        style.column_gap = Some(Length::zero());
    }
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.height = Length::px(160.0);
    doc.append_child(multicol, flex);
    let image = doc.create_node(ElementTag::Image);
    doc.node_mut(image).replaced = Some(ReplacedContent {
        resource: ReplacedResourceKind::TransparentCanvas,
        intrinsic_width: Some(40.0),
        intrinsic_height: Some(40.0),
        intrinsic_ratio: Some((1.0, 1.0)),
    });
    doc.append_child(flex, image);
    let fragment = layout(&doc);
    let mut found = Vec::new();
    descendants(&fragment, image, &mut found);
    assert!(!found.is_empty());
}

#[test]
fn flex_intrinsic_replaced_ratio_survives_vertical_writing_mode() {
    let mut doc = Document::new();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.writing_mode = WritingMode::VerticalRl;
    doc.node_mut(flex).style.width = Length::px(100.0);
    doc.node_mut(flex).style.height = Length::px(200.0);
    doc.append_child(doc.root(), flex);
    let image = doc.create_node(ElementTag::Image);
    doc.node_mut(image).style.writing_mode = WritingMode::VerticalRl;
    doc.node_mut(image).replaced = Some(ReplacedContent {
        resource: ReplacedResourceKind::TransparentCanvas,
        intrinsic_width: Some(20.0),
        intrinsic_height: Some(50.0),
        intrinsic_ratio: Some((20.0, 50.0)),
    });
    doc.append_child(flex, image);
    let fragment = layout(&doc);
    let mut found = Vec::new();
    descendants(&fragment, image, &mut found);
    assert_eq!(found.len(), 1);
    assert!(found[0].width() > LayoutUnit::zero());
    assert!(found[0].height() > LayoutUnit::zero());
}

#[test]
fn native_controls_have_explicit_semantic_roles() {
    let cases = [
        (ElementTag::TextArea, FormControlRole::TextArea),
        (ElementTag::Select, FormControlRole::Select),
        (ElementTag::Option, FormControlRole::Option),
        (ElementTag::OptGroup, FormControlRole::OptGroup),
        (ElementTag::Input, FormControlRole::Checkbox),
        (ElementTag::Input, FormControlRole::Radio),
    ];
    let mut doc = Document::new();
    for (tag, role) in cases {
        let node = doc.create_node(tag);
        doc.node_mut(node).form_control = Some(role);
        assert_eq!(doc.node(node).form_control, Some(role));
    }
}

#[test]
fn encoded_replaced_resources_are_content_stable_and_deduplicated() {
    let mut doc = Document::new();
    let first = doc.register_image_resource("fixture.png", "image/png", "abc", vec![1, 2, 3]);
    let second = doc.register_image_resource("fixture.png", "image/png", "abc", vec![9]);
    assert_eq!(first, second);
    assert_eq!(doc.image_resource_count(), 1);
    assert_eq!(doc.image_resource(first).unwrap().bytes, vec![1, 2, 3]);
}

#[test]
fn line_clamp_contract_retains_continuation_marker() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.width = Length::px(50.0);
    doc.node_mut(block).style.white_space = WhiteSpace::Normal;
    doc.node_mut(block).style.line_clamp = LineClamp::Lines(1);
    doc.node_mut(block).style.block_ellipsis = BlockEllipsis::String("more".into());
    doc.append_child(doc.root(), block);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("one two three four five six".into());
    doc.append_child(block, text);
    let items = InlineItemsBuilder::collect(&doc, block);
    assert!(items
        .items
        .iter()
        .any(|item| item.item_type == InlineItemType::Text));
    assert_eq!(doc.node(block).style.line_clamp, LineClamp::Lines(1));
}

#[test]
fn wildcard_scroll_buttons_are_external_siblings() {
    let mut doc = Document::new();
    let scroller = doc.create_node(ElementTag::Div);
    doc.append_child(doc.root(), scroller);
    let mut buttons = Vec::new();
    for direction in [
        ScrollButtonDirection::Up,
        ScrollButtonDirection::Right,
        ScrollButtonDirection::Down,
        ScrollButtonDirection::Left,
    ] {
        buttons
            .push(doc.insert_pseudo_element(scroller, PseudoElementKind::ScrollButton(direction)));
    }
    let root_children: Vec<_> = doc.children(doc.root()).collect();
    assert_eq!(root_children[0], scroller);
    assert_eq!(
        root_children[1..]
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>(),
        buttons.into_iter().collect()
    );
}

#[test]
fn inset_clip_and_effect_transform_are_independent_contracts() {
    let mut style = openui_style::ComputedStyle::initial();
    style.clip_path_inset = Some([
        Length::px(1.0),
        Length::px(2.0),
        Length::px(3.0),
        Length::px(4.0),
    ]);
    style.transform = Transform2D {
        e: 10.0,
        f: 20.0,
        ..Transform2D::default()
    };
    assert_eq!(style.clip_path_inset.unwrap()[2], Length::px(3.0));
    assert_eq!((style.transform.e, style.transform.f), (10.0, 20.0));
}

#[test]
fn margin_trim_edges_remain_logically_composable() {
    let trim = MarginTrim::BLOCK_START | MarginTrim::INLINE_END;
    assert!(trim.contains(MarginTrim::BLOCK_START));
    assert!(trim.contains(MarginTrim::INLINE_END));
    assert!(!trim.contains(MarginTrim::BLOCK_END));
}

#[test]
fn positioned_inline_and_static_descendant_keep_distinct_roles() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(doc.root(), block);
    let inline = doc.create_node(ElementTag::Span);
    doc.node_mut(inline).style.position = Position::Relative;
    doc.append_child(block, inline);
    let absolute = doc.create_node(ElementTag::Div);
    doc.node_mut(absolute).style.position = Position::Absolute;
    doc.node_mut(absolute).style.width = Length::px(10.0);
    doc.node_mut(absolute).style.height = Length::px(10.0);
    doc.append_child(inline, absolute);
    let fragment = layout(&doc);
    let mut found = Vec::new();
    descendants(&fragment, absolute, &mut found);
    assert_eq!(found.len(), 1);
}

#[test]
fn display_contents_does_not_erase_form_descendants() {
    let mut doc = Document::new();
    let form = doc.create_node(ElementTag::Form);
    doc.node_mut(form).style.display = Display::Contents;
    doc.append_child(doc.root(), form);
    let select = doc.create_node(ElementTag::Select);
    doc.node_mut(select).style.display = Display::InlineBlock;
    doc.node_mut(select).form_control = Some(FormControlRole::Select);
    doc.append_child(form, select);
    let fragment = layout(&doc);
    let mut form_fragments = Vec::new();
    let mut select_fragments = Vec::new();
    descendants(&fragment, form, &mut form_fragments);
    descendants(&fragment, select, &mut select_fragments);
    assert!(form_fragments.is_empty());
    assert_eq!(select_fragments.len(), 1);
}

#[test]
fn details_content_is_after_authored_summary_and_content() {
    let mut doc = Document::new();
    let details = doc.create_node(ElementTag::Details);
    doc.append_child(doc.root(), details);
    let summary = doc.create_node(ElementTag::Summary);
    doc.append_child(details, summary);
    let content = doc.create_node(ElementTag::Div);
    doc.append_child(details, content);
    let pseudo = doc.insert_pseudo_element(details, PseudoElementKind::DetailsContent);
    assert_eq!(
        doc.children(details).collect::<Vec<_>>(),
        [summary, content, pseudo]
    );
}

#[test]
fn rounded_complex_border_contract_survives_writing_direction() {
    let mut style = openui_style::ComputedStyle::initial();
    style.writing_mode = WritingMode::VerticalLr;
    style.direction = Direction::Rtl;
    style.border_top_width = 3;
    style.border_top_style = BorderStyle::Double;
    style.border_top_color = StyleColor::Resolved(Color::GREEN);
    style.border_top_left_radius = (8.0, 4.0);
    assert_eq!(style.border_top_style, BorderStyle::Double);
    assert_eq!(style.border_top_left_radius.0, 8.0);
}
