use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::inline::items_builder::InlineItemsBuilder;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{BlockEllipsis, Color, Display, LineClamp, WhiteSpace};

fn space(width: i32) -> ConstraintSpace {
    ConstraintSpace::for_block_child(
        LayoutUnit::from_i32(width),
        LayoutUnit::from_i32(600),
        LayoutUnit::from_i32(width),
        LayoutUnit::from_i32(600),
        false,
    )
}

fn text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut fragments = Vec::new();
    if fragment.kind == FragmentKind::Text {
        fragments.push(fragment);
    }
    for child in &fragment.children {
        fragments.extend(text_fragments(child));
    }
    fragments
}

#[test]
fn first_letter_crosses_nested_inline_and_includes_punctuation() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    let mut first_letter = openui_style::ComputedStyle::for_pseudo(&doc.node(block).style);
    first_letter.color = Color::RED;
    doc.node_mut(block).style.first_letter_style = Some(Box::new(first_letter));
    doc.append_child(doc.root(), block);

    let span = doc.create_node(ElementTag::Span);
    doc.append_child(block, span);
    let leading = doc.create_node(ElementTag::Text);
    doc.node_mut(leading).text = Some("(A".into());
    doc.append_child(span, leading);
    let trailing = doc.create_node(ElementTag::Text);
    doc.node_mut(trailing).text = Some(")bc".into());
    doc.append_child(block, trailing);

    let data = InlineItemsBuilder::collect(&doc, block);
    let highlighted: String = data
        .items
        .iter()
        .filter(|item| data.styles[item.style_index].color == Color::RED)
        .map(|item| data.text[item.text_range.clone()].to_string())
        .collect();
    assert_eq!(highlighted, "(A)");
}

#[test]
fn first_line_font_metrics_only_change_line_one() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    let mut first_line = openui_style::ComputedStyle::for_pseudo(&doc.node(block).style);
    first_line.font_size = 32.0;
    first_line.line_height = openui_style::LineHeight::Length(40.0);
    doc.node_mut(block).style.first_line_style = Some(Box::new(first_line));
    doc.append_child(doc.root(), block);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("one two three four five six seven eight".into());
    doc.append_child(block, text);

    let fragment = inline_layout(&doc, block, &space(100));
    assert!(fragment.children.len() >= 2);
    assert!(fragment.children[0].size.height > fragment.children[1].size.height);
}

#[test]
fn modern_line_clamp_discards_later_lines_and_uses_custom_marker() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.width = Length::px(80.0);
    doc.node_mut(block).style.white_space = WhiteSpace::Normal;
    doc.node_mut(block).style.line_clamp = LineClamp::Lines(2);
    doc.node_mut(block).style.block_ellipsis = BlockEllipsis::String("more".into());
    doc.append_child(doc.root(), block);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text =
        Some("one two three four five six seven eight nine ten eleven twelve".into());
    doc.append_child(block, text);

    let fragment = inline_layout(&doc, block, &space(80));
    assert_eq!(fragment.children.len(), 2);
    let anonymous: Vec<_> = text_fragments(&fragment)
        .into_iter()
        .filter(|fragment| fragment.node_id.is_none())
        .collect();
    assert_eq!(anonymous.len(), 1);
    assert_eq!(anonymous[0].text_content.as_deref(), Some("more"));
}

#[test]
fn line_clamp_no_ellipsis_still_discards_content() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_clamp = LineClamp::Lines(1);
    doc.node_mut(block).style.block_ellipsis = BlockEllipsis::NoEllipsis;
    doc.append_child(doc.root(), block);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("one two three four five six seven".into());
    doc.append_child(block, text);
    let fragment = inline_layout(&doc, block, &space(50));
    assert_eq!(fragment.children.len(), 1);
    assert!(text_fragments(&fragment)
        .iter()
        .all(|fragment| !fragment.node_id.is_none()));
}
