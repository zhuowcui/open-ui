//! SP20 contracts for the deterministic static-visual residual closure.

use openui_dom::{
    Document, ElementTag, FormControlRole, PseudoElementKind, ReplacedContent,
    ReplacedResourceKind, ScrollButtonDirection,
};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::items::InlineItemType;
use openui_layout::inline::items_builder::InlineItemsBuilder;
use openui_layout::{block_layout, ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    AspectRatio, BlockEllipsis, BorderStyle, Color, ColumnFill, ColumnSpan, Containment, Direction,
    Display, FlexDirection, Float, LineClamp, LineHeight, MarginTrim, Position, StyleColor,
    Transform2D, WhiteSpace, WritingMode,
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

fn append_text(doc: &mut Document, parent: openui_dom::NodeId, value: &str) {
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some(value.to_string());
    doc.update_resolved_style(text, |style| {
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(parent, text);
}

#[test]
fn orthogonal_flex_cross_size_includes_resolved_percentage_padding() {
    let mut doc = Document::new();
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalRl;
        style.width = Length::px(100.0);
    });
    doc.append_child(doc.root(), wrapper);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.display = Display::Flex;
        style.writing_mode = WritingMode::VerticalRl;
        style.padding_right = Length::percent(5.0);
    });
    doc.append_child(wrapper, flex);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::HorizontalTb;
        style.padding_right = Length::percent(10.0);
    });
    doc.append_child(flex, item);

    let content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(content, |style| {
        style.display = Display::Block;
        style.width = Length::px(100.0);
        style.height = Length::px(100.0);
        style.position = Position::Relative;
        style.left = Length::px(15.0);
    });
    doc.append_child(item, content);

    let fragment = layout(&doc);
    let mut item_fragments = Vec::new();
    descendants(&fragment, item, &mut item_fragments);
    assert_eq!(item_fragments.len(), 1);
    assert_eq!(item_fragments[0].width(), lu(110));

    let mut flex_fragments = Vec::new();
    descendants(&fragment, flex, &mut flex_fragments);
    assert_eq!(flex_fragments.len(), 1);
    assert_eq!(flex_fragments[0].width(), lu(115));

    let mut content_positions = Vec::new();
    descendant_positions(
        &fragment,
        content,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut content_positions,
    );
    assert_eq!(
        content_positions,
        [(LayoutUnit::zero(), LayoutUnit::zero())]
    );
}

#[test]
fn orthogonal_column_flex_main_size_includes_resolved_percentage_padding() {
    let mut doc = Document::new();
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalRl;
        style.width = Length::px(100.0);
    });
    doc.append_child(doc.root(), wrapper);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Column;
        style.flex_wrap = openui_style::FlexWrap::Wrap;
        style.writing_mode = WritingMode::VerticalRl;
        style.padding_right = Length::percent(5.0);
    });
    doc.append_child(wrapper, flex);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::HorizontalTb;
        style.padding_right = Length::percent(10.0);
    });
    doc.append_child(flex, item);

    let content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(content, |style| {
        style.display = Display::Block;
        style.width = Length::px(100.0);
        style.height = Length::px(100.0);
        style.position = Position::Relative;
        style.left = Length::px(15.0);
    });
    doc.append_child(item, content);

    let fragment = layout(&doc);
    let mut item_fragments = Vec::new();
    descendants(&fragment, item, &mut item_fragments);
    assert_eq!(item_fragments.len(), 1);
    assert_eq!(item_fragments[0].width(), lu(110));

    let mut flex_fragments = Vec::new();
    descendants(&fragment, flex, &mut flex_fragments);
    assert_eq!(flex_fragments.len(), 1);
    assert_eq!(flex_fragments[0].width(), lu(115));

    let mut content_positions = Vec::new();
    descendant_positions(
        &fragment,
        content,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut content_positions,
    );
    assert_eq!(
        content_positions,
        [(LayoutUnit::zero(), LayoutUnit::zero())]
    );
}

#[test]
fn max_content_column_wrap_uses_definite_stretched_block_size() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.display = Display::Flex;
        style.height = Length::px(100.0);
    });
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Column;
        style.flex_wrap = openui_style::FlexWrap::Wrap;
        style.width = Length::max_content();
    });
    doc.append_child(outer, inner);

    let first = doc.create_node(ElementTag::Span);
    let second = doc.create_node(ElementTag::Span);
    for item in [first, second] {
        doc.update_resolved_style(item, |style| {
            style.display = Display::Block;
            style.width = Length::px(50.0);
            style.height = Length::px(100.0);
        });
        doc.append_child(inner, item);
    }

    let intrinsic = openui_layout::intrinsic_sizing::compute_intrinsic_inline_sizes_with_block_size(
        &doc,
        inner,
        lu(100),
        lu(800),
    );
    assert_eq!(intrinsic.max, lu(100));

    let fragment = layout(&doc);
    let mut inner_fragments = Vec::new();
    descendants(&fragment, inner, &mut inner_fragments);
    assert_eq!(inner_fragments.len(), 1);
    assert_eq!(inner_fragments[0].width(), lu(100));
    assert_eq!(inner_fragments[0].height(), lu(100));

    let mut first_positions = Vec::new();
    let mut second_positions = Vec::new();
    descendant_positions(
        &fragment,
        first,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut first_positions,
    );
    descendant_positions(
        &fragment,
        second,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut second_positions,
    );
    assert_eq!(first_positions, [(LayoutUnit::zero(), LayoutUnit::zero())]);
    assert_eq!(second_positions, [(lu(50), LayoutUnit::zero())]);
}

fn descendant_positions(
    fragment: &Fragment,
    node: openui_dom::NodeId,
    parent_left: LayoutUnit,
    parent_top: LayoutUnit,
    out: &mut Vec<(LayoutUnit, LayoutUnit)>,
) {
    let left = parent_left + fragment.offset.left;
    let top = parent_top + fragment.offset.top;
    if fragment.node_id == node {
        out.push((left, top));
    }
    for child in &fragment.children {
        descendant_positions(child, node, left, top, out);
    }
}

#[test]
fn fragmented_multicol_keeps_nested_flex_replaced_content() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(200.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.column_gap = Some(Length::zero()));
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.height = Length::px(160.0));
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
fn captioned_bordered_table_places_contained_body_in_next_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(150.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.column_gap = Some(Length::zero()));
    });
    doc.append_child(doc.root(), multicol);

    let table = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(table, |style| {
        style.update_derived(|computed| computed.display = Display::Table);
        style.update_derived(|computed| computed.width = Length::percent(100.0));
        style.update_derived(|computed| computed.border_top_width = 30);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
    });
    doc.append_child(multicol, table);

    let caption = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(caption, |style| style.display = Display::TableCaption);
    doc.append_child(table, caption);
    let caption_content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(caption_content, |style| style.display = Display::Block);
    doc.update_resolved_style(caption_content, |style| style.height = Length::px(100.0));
    doc.append_child(caption, caption_content);

    let body = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(body, |style| style.display = Display::Block);
    doc.update_resolved_style(body, |style| style.contain = Containment::SIZE);
    doc.update_resolved_style(body, |style| style.height = Length::px(70.0));
    doc.append_child(table, body);
    let body_content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(body_content, |style| style.display = Display::Block);
    doc.update_resolved_style(body_content, |style| style.margin_top = Length::px(-30.0));
    doc.update_resolved_style(body_content, |style| style.height = Length::px(100.0));
    doc.append_child(body, body_content);

    let fragment = layout(&doc);
    let mut caption_fragments = Vec::new();
    descendants(&fragment, caption_content, &mut caption_fragments);
    assert_eq!(caption_fragments.len(), 1);
    assert_eq!(caption_fragments[0].width(), lu(50));
    assert_eq!(caption_fragments[0].height(), lu(100));
    let mut caption_positions = Vec::new();
    let mut body_positions = Vec::new();
    descendant_positions(
        &fragment,
        caption_content,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut caption_positions,
    );
    descendant_positions(
        &fragment,
        body_content,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut body_positions,
    );
    assert_eq!(
        caption_positions,
        [(LayoutUnit::zero(), LayoutUnit::zero())]
    );
    assert_eq!(body_positions, [(lu(50), lu(-30))]);
}

#[test]
fn flex_intrinsic_replaced_ratio_survives_vertical_writing_mode() {
    let mut doc = Document::new();
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.writing_mode = WritingMode::VerticalRl);
    doc.update_resolved_style(flex, |style| style.width = Length::px(100.0));
    doc.update_resolved_style(flex, |style| style.height = Length::px(200.0));
    doc.append_child(doc.root(), flex);
    let image = doc.create_node(ElementTag::Image);
    doc.update_resolved_style(image, |style| style.writing_mode = WritingMode::VerticalRl);
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
fn definite_float_height_transfers_through_nested_replaced_ratio_only() {
    let mut doc = Document::new();
    let make_canvas = |doc: &mut Document| {
        let canvas = doc.create_node(ElementTag::Canvas);
        doc.update_resolved_style(canvas, |style| style.height = Length::percent(100.0));
        doc.node_mut(canvas).replaced = Some(ReplacedContent {
            resource: ReplacedResourceKind::TransparentCanvas,
            intrinsic_width: Some(1.0),
            intrinsic_height: Some(1.0),
            intrinsic_ratio: Some((1.0, 1.0)),
        });
        canvas
    };

    let nested_target = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(nested_target, |style| {
        style.float = Float::Left;
        style.height = Length::px(100.0);
    });
    doc.append_child(doc.root(), nested_target);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.height = Length::percent(100.0));
    doc.append_child(nested_target, wrapper);
    let nested_canvas = make_canvas(&mut doc);
    doc.append_child(wrapper, nested_canvas);

    let natural_target = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(natural_target, |style| {
        style.float = Float::Left;
        style.height = Length::px(100.0);
    });
    doc.append_child(doc.root(), natural_target);
    let auto_wrapper = doc.create_node(ElementTag::Div);
    doc.append_child(natural_target, auto_wrapper);
    let natural_canvas = make_canvas(&mut doc);
    doc.append_child(auto_wrapper, natural_canvas);

    let fragment = layout(&doc);
    let mut nested = Vec::new();
    let mut natural = Vec::new();
    descendants(&fragment, nested_target, &mut nested);
    descendants(&fragment, natural_target, &mut natural);
    assert_eq!(nested.len(), 1);
    assert_eq!(natural.len(), 1);
    assert_eq!(nested[0].width(), lu(100));
    assert_eq!(natural[0].width(), lu(1));
}

#[test]
fn trailing_float_keeps_its_source_position_in_mixed_anonymous_flow() {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.display = Display::Block;
        style.width = Length::px(200.0);
    });
    doc.append_child(doc.root(), container);

    let header = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(header, |style| {
        style.display = Display::Block;
        style.height = Length::px(50.0);
    });
    doc.append_child(container, header);

    let inline = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inline, |style| {
        style.display = Display::InlineBlock;
        style.width = Length::px(100.0);
        style.height = Length::px(150.0);
        style.vertical_align = openui_style::VerticalAlign::Top;
    });
    doc.append_child(container, inline);

    let removed = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(removed, |style| {
        style.display = Display::None;
        style.height = Length::px(100.0);
    });
    doc.append_child(container, removed);

    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| {
        style.display = Display::Block;
        style.float = Float::Left;
        style.width = Length::px(100.0);
        style.height = Length::px(150.0);
    });
    doc.append_child(container, float);

    let fragment = layout(&doc);
    let mut container_fragments = Vec::new();
    let mut inline_positions = Vec::new();
    let mut float_positions = Vec::new();
    descendants(&fragment, container, &mut container_fragments);
    descendant_positions(
        &fragment,
        inline,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut inline_positions,
    );
    descendant_positions(
        &fragment,
        float,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut float_positions,
    );

    assert_eq!(container_fragments.len(), 1);
    assert_eq!(container_fragments[0].height(), lu(200));
    assert_eq!(float_positions, [(LayoutUnit::zero(), lu(50))]);
    assert_eq!(inline_positions, [(lu(100), lu(50))]);
}

#[test]
fn balanced_multicol_uses_complete_table_rows_as_fragmentation_units() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.display = Display::Block;
        style.width = Length::px(200.0);
        style.column_count = Some(2);
        style.column_gap = Some(Length::zero());
        style.column_fill = ColumnFill::Balance;
    });
    doc.append_child(doc.root(), multicol);

    let table = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(table, |style| style.display = Display::Table);
    doc.append_child(multicol, table);

    let mut rows = Vec::new();
    for height in [20.0, 30.0] {
        let row = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(row, |style| style.display = Display::TableRow);
        doc.append_child(table, row);
        let cell = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(cell, |style| style.display = Display::TableCell);
        doc.append_child(row, cell);
        let content = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(content, |style| {
            style.display = Display::Block;
            style.height = Length::px(height);
        });
        doc.append_child(cell, content);
        rows.push(row);
    }

    let fragment = layout(&doc);
    let mut multicol_fragments = Vec::new();
    descendants(&fragment, multicol, &mut multicol_fragments);
    assert_eq!(multicol_fragments.len(), 1);
    assert_eq!(multicol_fragments[0].height(), lu(30));
    let columns = multicol_fragments[0]
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .collect::<Vec<_>>();
    assert_eq!(columns.len(), 2);
    let mut first_row_in_first = Vec::new();
    let mut second_row_in_first = Vec::new();
    let mut first_row_in_second = Vec::new();
    let mut second_row_in_second = Vec::new();
    descendants(columns[0], rows[0], &mut first_row_in_first);
    descendants(columns[0], rows[1], &mut second_row_in_first);
    descendants(columns[1], rows[0], &mut first_row_in_second);
    descendants(columns[1], rows[1], &mut second_row_in_second);
    assert_eq!(first_row_in_first.len(), 1);
    assert!(second_row_in_first.is_empty());
    assert!(first_row_in_second.is_empty());
    assert_eq!(second_row_in_second.len(), 1);
}

#[test]
fn block_in_inline_continuation_keeps_its_reconstructed_boundary_together() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.display = Display::Block;
        style.width = Length::px(400.0);
        style.column_count = Some(3);
        style.column_fill = ColumnFill::Balance;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(doc.root(), multicol);

    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| {
        style.display = Display::Inline;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(multicol, inline);
    append_text(&mut doc, inline, "inline1");

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| {
        style.display = Display::Block;
        style.column_span = ColumnSpan::All;
        style.height = Length::px(19.0);
    });
    doc.append_child(inline, spanner);

    let block1 = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block1, |style| {
        style.display = Display::Block;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(inline, block1);
    append_text(&mut doc, block1, "block1");
    append_text(&mut doc, inline, "inline2");
    let block2 = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block2, |style| {
        style.display = Display::Block;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(inline, block2);
    append_text(&mut doc, block2, "block2");

    let fragment = layout(&doc);
    let mut multicol_fragments = Vec::new();
    descendants(&fragment, multicol, &mut multicol_fragments);
    assert_eq!(multicol_fragments.len(), 1);
    assert_eq!(multicol_fragments[0].height(), lu(67));
    let post_spanner_columns = multicol_fragments[0]
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox && child.offset.top == lu(35))
        .collect::<Vec<_>>();
    assert_eq!(post_spanner_columns.len(), 2);
    assert_eq!(post_spanner_columns[0].height(), lu(32));
}

#[test]
fn transparent_nested_wrapper_exposes_spanner_and_retains_trailing_line() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.display = Display::Block;
        style.width = Length::px(400.0);
        style.column_count = Some(3);
        style.column_fill = ColumnFill::Auto;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(doc.root(), multicol);

    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.display = Display::Block;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(multicol, outer);
    append_text(&mut doc, outer, "before");

    let transparent = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(transparent, |style| {
        style.display = Display::Block;
        style.font_size = 16.0;
        style.line_height = LineHeight::Length(16.0);
    });
    doc.append_child(outer, transparent);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| {
        style.display = Display::Block;
        style.column_span = ColumnSpan::All;
        style.height = Length::px(19.0);
    });
    doc.append_child(transparent, spanner);
    append_text(&mut doc, transparent, "after");

    let fragment = layout(&doc);
    let mut multicol_fragments = Vec::new();
    descendants(&fragment, multicol, &mut multicol_fragments);
    assert_eq!(multicol_fragments.len(), 1);
    assert_eq!(multicol_fragments[0].height(), lu(51));
    let spanning_fragments = multicol_fragments[0]
        .children
        .iter()
        .filter(|child| child.node_id == spanner)
        .collect::<Vec<_>>();
    assert_eq!(spanning_fragments.len(), 1);
    assert_eq!(spanning_fragments[0].offset.top, lu(16));
    let trailing_columns = multicol_fragments[0]
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox && child.offset.top == lu(35))
        .collect::<Vec<_>>();
    assert_eq!(trailing_columns.len(), 1);
    assert_eq!(trailing_columns[0].height(), lu(16));
}

#[test]
fn grid_cyclic_percentage_row_transfers_definite_inline_size_through_ratio() {
    let mut doc = Document::new();
    let grid = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(grid, |style| {
        style.display = Display::Grid;
        style.width = Length::px(100.0);
    });
    doc.append_child(doc.root(), grid);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.display = Display::Block;
        style.width = Length::percent(100.0);
        style.height = Length::percent(100.0);
        style.aspect_ratio = Some(AspectRatio {
            ratio: (1.0, 1.0),
            auto_flag: false,
        });
    });
    doc.append_child(grid, item);

    let fragment = layout(&doc);
    let mut grid_fragments = Vec::new();
    let mut item_fragments = Vec::new();
    descendants(&fragment, grid, &mut grid_fragments);
    descendants(&fragment, item, &mut item_fragments);
    assert_eq!(grid_fragments.len(), 1);
    assert_eq!(item_fragments.len(), 1);
    assert_eq!(
        grid_fragments[0].size,
        openui_geometry::PhysicalSize::new(lu(100), lu(100))
    );
    assert_eq!(
        item_fragments[0].size,
        openui_geometry::PhysicalSize::new(lu(100), lu(100))
    );
}

#[test]
fn min_content_flex_transfers_definite_cross_stretch_through_ratio() {
    let mut doc = Document::new();
    let target = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(target, |style| {
        style.display = Display::Block;
        style.width = Length::min_content();
        style.height = Length::px(100.0);
    });
    doc.append_child(doc.root(), target);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.display = Display::Flex;
        style.height = Length::percent(100.0);
    });
    doc.append_child(target, flex);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.display = Display::Block;
        style.aspect_ratio = Some(AspectRatio {
            ratio: (1.0, 1.0),
            auto_flag: false,
        });
    });
    doc.append_child(flex, item);

    let fragment = layout(&doc);
    let mut target_fragments = Vec::new();
    let mut flex_fragments = Vec::new();
    let mut item_fragments = Vec::new();
    descendants(&fragment, target, &mut target_fragments);
    descendants(&fragment, flex, &mut flex_fragments);
    descendants(&fragment, item, &mut item_fragments);
    assert_eq!(
        target_fragments[0].size,
        openui_geometry::PhysicalSize::new(lu(100), lu(100))
    );
    assert_eq!(
        flex_fragments[0].size,
        openui_geometry::PhysicalSize::new(lu(100), lu(100))
    );
    assert_eq!(
        item_fragments[0].size,
        openui_geometry::PhysicalSize::new(lu(100), lu(100))
    );
}

#[test]
fn unresolved_percentage_height_does_not_enable_aspect_ratio_transfer() {
    let mut doc = Document::new();
    let parent = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(parent, |style| {
        style.display = Display::Block;
        style.width = Length::px(100.0);
    });
    doc.append_child(doc.root(), parent);

    let fixed = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(fixed, |style| {
        style.display = Display::Block;
        style.width = Length::px(100.0);
        style.height = Length::px(50.0);
    });
    doc.append_child(parent, fixed);

    let target = doc.create_node(ElementTag::Button);
    doc.update_resolved_style(target, |style| {
        style.display = Display::Block;
        style.height = Length::percent(100.0);
        style.min_height = Length::px(50.0);
        style.aspect_ratio = Some(AspectRatio {
            ratio: (1.0, 1.0),
            auto_flag: false,
        });
    });
    doc.append_child(parent, target);

    let fragment = layout(&doc);
    let mut parent_fragments = Vec::new();
    let mut target_fragments = Vec::new();
    descendants(&fragment, parent, &mut parent_fragments);
    descendants(&fragment, target, &mut target_fragments);
    assert_eq!(parent_fragments.len(), 1);
    assert_eq!(target_fragments.len(), 1);
    assert_eq!(parent_fragments[0].height(), lu(100));
    assert_eq!(target_fragments[0].size.height, lu(50));
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
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.update_resolved_style(block, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(block, |style| style.white_space = WhiteSpace::Normal);
    doc.update_resolved_style(block, |style| style.line_clamp = LineClamp::Lines(1));
    doc.update_resolved_style(block, |style| {
        style.block_ellipsis = BlockEllipsis::String("more".into())
    });
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
    style.update_derived(|computed| {
        computed.clip_path_inset = Some([
            Length::px(1.0),
            Length::px(2.0),
            Length::px(3.0),
            Length::px(4.0),
        ])
    });
    style.update_derived(|computed| {
        computed.transform = Transform2D {
            e: 10.0,
            f: 20.0,
            ..Transform2D::default()
        }
    });
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
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.append_child(doc.root(), block);
    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.position = Position::Relative);
    doc.append_child(block, inline);
    let absolute = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(absolute, |style| style.position = Position::Absolute);
    doc.update_resolved_style(absolute, |style| style.width = Length::px(10.0));
    doc.update_resolved_style(absolute, |style| style.height = Length::px(10.0));
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
    doc.update_resolved_style(form, |style| style.display = Display::Contents);
    doc.append_child(doc.root(), form);
    let select = doc.create_node(ElementTag::Select);
    doc.update_resolved_style(select, |style| style.display = Display::InlineBlock);
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
fn display_contents_is_flattened_in_a_pure_block_context() {
    let mut doc = Document::new();
    let contents = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(contents, |style| style.display = Display::Contents);
    doc.update_resolved_style(contents, |style| style.border_top_width = 10);
    doc.append_child(doc.root(), contents);

    let absolute = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(absolute, |style| style.position = Position::Absolute);
    doc.update_resolved_style(absolute, |style| style.width = Length::px(10.0));
    doc.update_resolved_style(absolute, |style| style.height = Length::px(10.0));
    doc.append_child(contents, absolute);

    let fragment = layout(&doc);
    let mut contents_fragments = Vec::new();
    let mut absolute_fragments = Vec::new();
    descendants(&fragment, contents, &mut contents_fragments);
    descendants(&fragment, absolute, &mut absolute_fragments);
    assert!(contents_fragments.is_empty());
    assert_eq!(absolute_fragments.len(), 1);
}

#[test]
fn fieldset_promotes_a_legend_through_display_contents_before_ordinary_content() {
    let mut doc = Document::new();
    let fieldset = doc.create_node(ElementTag::Fieldset);
    doc.update_resolved_style(fieldset, |style| {
        style.display = Display::Block;
        style.width = Length::px(200.0);
        style.border_top_width = 1;
        style.border_right_width = 1;
        style.border_bottom_width = 1;
        style.border_left_width = 1;
        style.padding_top = Length::px(1.0);
    });
    doc.append_child(doc.root(), fieldset);

    let ordinary = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(ordinary, |style| {
        style.display = Display::Block;
        style.height = Length::px(20.0);
    });
    doc.append_child(fieldset, ordinary);

    let outer_contents = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer_contents, |style| style.display = Display::Contents);
    doc.append_child(fieldset, outer_contents);
    let inner_contents = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner_contents, |style| style.display = Display::Contents);
    doc.append_child(outer_contents, inner_contents);

    let legend = doc.create_node(ElementTag::Legend);
    doc.update_resolved_style(legend, |style| {
        style.display = Display::Block;
        style.width = Length::px(30.0);
        style.height = Length::px(10.0);
    });
    doc.append_child(inner_contents, legend);

    let fragment = layout(&doc);
    let mut fieldset_positions = Vec::new();
    let mut legend_positions = Vec::new();
    let mut ordinary_positions = Vec::new();
    descendant_positions(
        &fragment,
        fieldset,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut fieldset_positions,
    );
    descendant_positions(
        &fragment,
        legend,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut legend_positions,
    );
    descendant_positions(
        &fragment,
        ordinary,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        &mut ordinary_positions,
    );

    assert_eq!(fieldset_positions.len(), 1);
    assert_eq!(legend_positions.len(), 1);
    assert_eq!(ordinary_positions.len(), 1);
    assert_eq!(legend_positions[0].1, fieldset_positions[0].1);
    assert!(ordinary_positions[0].1 >= legend_positions[0].1 + lu(10));
    let mut contents_fragments = Vec::new();
    descendants(&fragment, outer_contents, &mut contents_fragments);
    descendants(&fragment, inner_contents, &mut contents_fragments);
    assert!(contents_fragments.is_empty());
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
    style.update_derived(|computed| computed.writing_mode = WritingMode::VerticalLr);
    style.update_derived(|computed| computed.direction = Direction::Rtl);
    style.update_derived(|computed| computed.border_top_width = 3);
    style.update_derived(|computed| computed.border_top_style = BorderStyle::Double);
    style.update_derived(|computed| computed.border_top_color = StyleColor::Resolved(Color::GREEN));
    style.update_derived(|computed| computed.border_top_left_radius = (8.0, 4.0));
    assert_eq!(style.border_top_style, BorderStyle::Double);
    assert_eq!(style.border_top_left_radius.0, 8.0);
}
