//! SP13-R regressions for authoritative multicol geometry and fragmentation.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::fragmentation::{BlockBreakToken, BreakToken};
use openui_layout::multicol::{
    balance_columns, compute_column_positions, compute_column_rule_positions,
    resolve_column_count_and_width,
};
use openui_layout::{block_layout, ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    BorderStyle, BoxDecorationBreak, BoxSizing, BreakInside, BreakValue, Clear, Color, ColumnFill,
    ColumnSpan, ColumnWrap, Containment, ContentAlignment, ContentDistribution, Direction, Display,
    FlexDirection, FlexWrap, Float, FontFamilyList, LineHeight, ListStylePosition, ListStyleType,
    Overflow, OverflowClipBox, Position, VerticalAlign, WhiteSpace, WritingMode,
};

fn lu(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn multicol_inline(direction: Direction) -> Fragment {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(200.0));
        style.update_derived(|computed| computed.height = Length::px(40.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| computed.font_size = 10.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Number(1.0));
    });
    doc.append_child(doc.root(), multicol);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("aa ".repeat(120));
    doc.update_resolved_style(text, |style| style.font_size = 10.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Number(1.0));
    doc.append_child(multicol, text);
    let space = ConstraintSpace::for_block_child(lu(200), lu(600), lu(200), lu(600), false);
    block_layout(&doc, multicol, &space)
}

fn columns(fragment: &Fragment) -> Vec<&Fragment> {
    fragment
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .collect()
}

fn contains_node(fragment: &Fragment, node_id: NodeId) -> bool {
    fragment.node_id == node_id
        || fragment
            .children
            .iter()
            .any(|child| contains_node(child, node_id))
}

fn contains_kind(fragment: &Fragment, kind: FragmentKind) -> bool {
    fragment.kind == kind
        || fragment
            .children
            .iter()
            .any(|child| contains_kind(child, kind))
}

fn find_node(fragment: &Fragment, node_id: NodeId) -> Option<&Fragment> {
    if fragment.node_id == node_id {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| find_node(child, node_id))
}

fn count_node(fragment: &Fragment, node_id: NodeId) -> usize {
    usize::from(fragment.node_id == node_id)
        + fragment
            .children
            .iter()
            .map(|child| count_node(child, node_id))
            .sum::<usize>()
}

fn find_nodes(fragment: &Fragment, node_id: NodeId) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.node_id == node_id {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(find_nodes(child, node_id));
    }
    result
}

#[test]
fn used_count_and_width_redistribute_available_space() {
    let resolved = resolve_column_count_and_width(Some(4), Some(lu(80)), lu(250), lu(10));
    assert_eq!(resolved.count, 2);
    assert_eq!(resolved.width, lu(120));
}

#[test]
fn fractional_distribution_closes_the_content_edge_exactly() {
    let positions = compute_column_positions(3, LayoutUnit::zero(), lu(7), lu(101), false);
    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0].inline_offset, LayoutUnit::zero());
    assert_eq!(positions[2].inline_offset + positions[2].width, lu(101));
    let distributed: LayoutUnit = positions
        .iter()
        .fold(LayoutUnit::zero(), |sum, position| sum + position.width);
    assert_eq!(distributed + lu(14), lu(101));
}

#[test]
fn rule_centers_share_the_resolved_column_stride() {
    assert_eq!(
        compute_column_rule_positions(3, lu(30), lu(10)),
        vec![lu(35), lu(75)]
    );
}

#[test]
fn forced_boundaries_participate_in_balancing() {
    let sizes = vec![lu(20), lu(20), lu(20), lu(20)];
    let forced = vec![false, true, false, false];
    let height = balance_columns(&sizes, &[false; 4], 2, lu(200), &forced);
    assert!(height >= lu(40));
    assert!(height <= lu(60));
}

#[test]
fn excess_forced_segments_balance_as_overflow_columns() {
    let sizes = vec![lu(20), lu(40), lu(100)];
    let forced = vec![true, true, true];
    let height = balance_columns(&sizes, &[false; 3], 2, lu(600), &forced);
    assert_eq!(height, lu(100));
}

#[test]
fn nested_forced_segment_sets_the_balancing_floor() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
    });
    doc.append_child(doc.root(), multicol);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    for height in [20.0, 40.0, 100.0] {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| style.display = Display::Block);
        doc.update_resolved_style(child, |style| style.height = Length::px(height));
        doc.update_resolved_style(child, |style| style.break_before = BreakValue::Column);
        doc.append_child(wrapper, child);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    assert!(column_fragments
        .iter()
        .all(|column| column.size.height == lu(100)));
}

#[test]
fn monolithic_avoid_item_sets_the_balance_floor() {
    let height = balance_columns(
        &[lu(20), lu(75), lu(20)],
        &[false, true, false],
        2,
        lu(200),
        &[false; 3],
    );
    assert!(height >= lu(75));
}

#[test]
fn trailing_margin_is_contained_and_balanced() {
    let sizes = [LayoutUnit::zero(), LayoutUnit::zero()];
    let margins = [LayoutUnit::zero(), LayoutUnit::zero()];
    let block_end = [LayoutUnit::zero(), LayoutUnit::from_i32(13)];
    let height = openui_layout::multicol::balance_columns_with_margins(
        &sizes,
        &margins,
        &block_end,
        &[false, false],
        2,
        openui_geometry::INDEFINITE_SIZE,
        &[false, false],
        &[false, false],
        false,
    );
    assert_eq!(height, LayoutUnit::from_f32(6.5));
}

#[test]
fn break_token_resumption_preserves_consumed_state() {
    let mut token = BlockBreakToken::new(3, lu(90));
    token.add_child_token(BreakToken::Block(BlockBreakToken::new(1, lu(30))));
    assert_eq!(token.child_index, 3);
    assert_eq!(token.consumed_block_size, lu(90));
    assert!(token.has_child_break_tokens());
}

#[test]
fn direct_inline_content_fragments_into_real_column_boxes() {
    let fragment = multicol_inline(Direction::Ltr);
    let column_boxes = columns(&fragment);
    assert!(column_boxes.len() >= 2);
    assert!(column_boxes
        .iter()
        .all(|column| !column.children.is_empty()));
    assert!(column_boxes[0].offset.left < column_boxes[1].offset.left);
}

#[test]
fn rtl_uses_the_same_geometry_in_reverse_inline_order() {
    let fragment = multicol_inline(Direction::Rtl);
    let column_boxes = columns(&fragment);
    assert!(column_boxes.len() >= 2);
    assert!(column_boxes[0].offset.left > column_boxes[1].offset.left);
    assert_eq!(column_boxes[0].size.width, column_boxes[1].size.width);
}

#[test]
fn node_id_none_is_safe_for_synthetic_column_fragments() {
    let fragment = Fragment::new_box(
        NodeId::NONE,
        openui_geometry::PhysicalSize::new(lu(10), lu(20)),
    );
    assert!(fragment.node_id.is_none());
}

#[test]
fn definite_balance_keeps_the_fragmentainer_block_size_for_flex() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.append_child(multicol, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| style.height = Length::px(150.0));
    doc.update_resolved_style(item, |style| style.min_height = Length::px(150.0));
    doc.append_child(flex, item);

    let space = ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false);
    let fragment = block_layout(&doc, multicol, &space);
    let column_boxes = columns(&fragment);
    assert_eq!(column_boxes.len(), 2);
    assert!(column_boxes
        .iter()
        .all(|column| column.size.height == lu(100)));
    let continuation = &column_boxes[1].children[0];
    assert_eq!(continuation.size.height, lu(100));
    assert!(continuation.children[0].size.height >= lu(200));
}

#[test]
fn fixed_column_flex_grows_crossing_item_decoration_on_continuation() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| style.height = Length::px(150.0));
    doc.update_resolved_style(item, |style| style.min_height = Length::px(150.0));
    doc.append_child(flex, item);

    let space = ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false);
    let fragment = block_layout(&doc, multicol, &space);
    let column_boxes = columns(&fragment);
    assert_eq!(column_boxes.len(), 2);
    let continuation = &column_boxes[1].children[0];
    assert_eq!(continuation.size.height, lu(100));
    assert!(continuation.children[0].size.height >= lu(200));
}

#[test]
fn fixed_column_flex_visual_overflow_slices_item_decoration() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.box_sizing = BoxSizing::BorderBox);
        style.update_derived(|computed| computed.width = Length::px(300.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_right_width = 10);
        style.update_derived(|computed| computed.border_bottom_width = 10);
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.box_sizing = BoxSizing::BorderBox);
        style.update_derived(|computed| computed.width = Length::px(130.0));
        style.update_derived(|computed| computed.height = Length::px(70.0));
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_right_width = 10);
        style.update_derived(|computed| computed.border_bottom_width = 10);
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(multicol, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.flex_grow = 1.0);
        style.update_derived(|computed| computed.flex_shrink = 1.0);
        style.update_derived(|computed| computed.flex_basis = Length::percent(0.0));
        style.update_derived(|computed| computed.min_height = Length::px(0.0));
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_right_width = 10);
        style.update_derived(|computed| computed.border_bottom_width = 10);
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(flex, item);
    let grandchild = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(grandchild, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.box_sizing = BoxSizing::BorderBox);
        style.update_derived(|computed| computed.height = Length::px(140.0));
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_right_width = 10);
        style.update_derived(|computed| computed.border_bottom_width = 10);
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(item, grandchild);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(300), lu(600), lu(300), lu(600), false),
    );
    let column_boxes = columns(&fragment);
    assert_eq!(column_boxes.len(), 2);

    let principal_flex = &column_boxes[0].children[0];
    assert_eq!(principal_flex.node_id, flex);
    assert_eq!(principal_flex.size.height, lu(70));
    assert_eq!(principal_flex.children[0].node_id, item);
    assert_eq!(principal_flex.children[0].size.height, lu(50));

    // The fixed flex and its flex item have exhausted their authored border
    // boxes in the first column. The second column owns only the overflowing
    // grandchild, translated through an anonymous clipped continuation.
    let continuation = &column_boxes[1].children[0];
    assert_eq!(continuation.node_id, NodeId::NONE);
    assert_eq!(continuation.border.top, LayoutUnit::zero());
    assert_eq!(continuation.border.right, LayoutUnit::zero());
    assert_eq!(continuation.border.bottom, LayoutUnit::zero());
    assert_eq!(continuation.border.left, LayoutUnit::zero());
    assert_eq!(continuation.children.len(), 1);
    assert_eq!(continuation.children[0].node_id, grandchild);
    assert_eq!(continuation.children[0].offset.left, lu(20));
    assert_eq!(continuation.children[0].offset.top, lu(-60));
}

#[test]
fn cloned_flex_decoration_covers_short_final_fragmentainer() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.box_decoration_break = BoxDecorationBreak::Clone);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_top_width = 20);
        style.update_derived(|computed| computed.border_bottom_width = 10);
    });
    doc.append_child(multicol, flex);
    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.height = Length::px(20.0));
    doc.append_child(flex, first);
    let second = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second, |style| style.height = Length::px(70.0));
    doc.update_resolved_style(second, |style| style.break_before = BreakValue::Column);
    doc.append_child(flex, second);

    let space = ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false);
    let fragment = block_layout(&doc, multicol, &space);
    let column_boxes = columns(&fragment);
    assert_eq!(column_boxes.len(), 2);
    assert!(column_boxes
        .iter()
        .all(|column| column.children[0].size.height == lu(100)));
}

#[test]
fn avoid_between_siblings_uses_last_resort_break_when_column_is_full() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(75.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.update_resolved_style(first, |style| style.height = Length::px(100.0));
    doc.update_resolved_style(first, |style| style.break_after = BreakValue::AvoidColumn);
    doc.append_child(multicol, first);
    let second = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second, |style| style.display = Display::Block);
    doc.update_resolved_style(second, |style| style.height = Length::px(150.0));
    doc.append_child(multicol, second);

    let space = ConstraintSpace::for_block_child(lu(75), lu(600), lu(75), lu(600), false);
    let fragment = block_layout(&doc, multicol, &space);
    let column_boxes = columns(&fragment);
    assert_eq!(column_boxes.len(), 3);
    assert!(!contains_node(column_boxes[0], second));
    assert!(contains_node(column_boxes[1], second));
    assert!(contains_node(column_boxes[2], second));
}

#[test]
fn propagated_avoid_uses_only_the_fragmentable_child_prefix() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(4));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.update_resolved_style(first, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, first);
    let second = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second, |style| style.display = Display::Block);
    doc.update_resolved_style(second, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, second);

    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    let avoided_prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(avoided_prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(avoided_prefix, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(avoided_prefix, |style| {
        style.break_before = BreakValue::Avoid
    });
    doc.append_child(wrapper, avoided_prefix);
    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| style.display = Display::Block);
    doc.update_resolved_style(tail, |style| style.height = Length::px(200.0));
    doc.append_child(wrapper, tail);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 4);
    assert!(contains_node(column_fragments[0], first));
    assert!(!contains_node(column_fragments[0], second));
    assert!(contains_node(column_fragments[1], second));
    assert!(contains_node(column_fragments[1], avoided_prefix));
}

#[test]
fn single_line_column_group_sets_the_balancing_floor() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(60.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
        style.update_derived(|computed| computed.line_height = LineHeight::Length(40.0));
    });
    doc.append_child(doc.root(), multicol);

    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::Block);
    doc.update_resolved_style(before, |style| style.height = Length::px(90.0));
    doc.append_child(multicol, before);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.height = Length::px(30.0));
    doc.append_child(multicol, spanner);
    let line_break = doc.create_node(ElementTag::Break);
    doc.update_resolved_style(line_break, |style| style.display = Display::Inline);
    doc.update_resolved_style(line_break, |style| {
        style.line_height = LineHeight::Length(40.0)
    });
    doc.append_child(multicol, line_break);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(60), lu(600), lu(60), lu(600), false),
    );
    assert_eq!(fragment.size.height, lu(100));
    let baseline = fragment.first_baseline.expect("exported multicol baseline");
    assert!(baseline > lu(80) && baseline < lu(90));
    assert_eq!(fragment.last_baseline, Some(baseline));
}

#[test]
fn block_in_positioned_inline_preserves_descendant_oof_candidate() {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| style.display = Display::Block);
    doc.append_child(doc.root(), container);
    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
    doc.update_resolved_style(inline, |style| style.position = Position::Relative);
    doc.update_resolved_style(inline, |style| style.padding_bottom = Length::px(20.0));
    doc.append_child(container, inline);
    let block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.append_child(inline, block);
    let absolute = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(absolute, |style| style.display = Display::Block);
    doc.update_resolved_style(absolute, |style| style.position = Position::Absolute);
    doc.update_resolved_style(absolute, |style| style.width = Length::px(20.0));
    doc.update_resolved_style(absolute, |style| style.height = Length::px(10.0));
    doc.append_child(block, absolute);

    let fragment = openui_layout::inline::algorithm::inline_layout(
        &doc,
        container,
        &ConstraintSpace::for_block_child(lu(100), lu(100), lu(100), lu(100), false),
    );
    let candidate = fragment
        .oof_candidates
        .iter()
        .find(|candidate| candidate.node_id == absolute)
        .expect("block-in-inline out-of-flow descendant");
    assert!(candidate.has_inline_containing_block);
    assert!(candidate.containing_block_size.height > lu(20));
}

#[test]
fn extracted_spanner_preserves_positioned_inline_oof_and_block_extent() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.overflow_x = Overflow::Scroll);
        style.update_derived(|computed| computed.overflow_y = Overflow::Scroll);
        style.update_derived(|computed| computed.opacity = 0.1);
    });
    doc.append_child(doc.root(), multicol);

    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
    doc.update_resolved_style(inline, |style| style.position = Position::Relative);
    doc.update_resolved_style(inline, |style| style.padding_right = Length::px(10.0));
    doc.update_resolved_style(inline, |style| style.padding_bottom = Length::px(100.0));
    doc.append_child(multicol, inline);

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.append_child(inline, spanner);
    let absolute = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(absolute, |style| style.display = Display::Block);
    doc.update_resolved_style(absolute, |style| style.position = Position::Absolute);
    doc.update_resolved_style(absolute, |style| style.width = Length::px(32.0));
    doc.update_resolved_style(absolute, |style| style.height = Length::px(16.0));
    doc.append_child(spanner, absolute);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    assert!(fragment.size.height >= lu(16));
    let positioned = find_node(&fragment, absolute).expect("spanner OOF fragment");
    assert_eq!(positioned.offset.top, LayoutUnit::zero());
    assert_eq!(
        positioned.size,
        openui_geometry::PhysicalSize::new(lu(32), lu(16))
    );
}

#[test]
fn full_width_float_does_not_advance_normal_column_flow() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| style.display = Display::Block);
    doc.update_resolved_style(float, |style| style.float = Float::Left);
    doc.update_resolved_style(float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(float, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, float);

    let spacer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spacer, |style| style.display = Display::Block);
    doc.update_resolved_style(spacer, |style| style.height = Length::px(10.0));
    doc.append_child(multicol, spacer);

    let padded = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(padded, |style| style.display = Display::Block);
    doc.update_resolved_style(padded, |style| style.padding_bottom = Length::px(10.0));
    doc.append_child(multicol, padded);
    let content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(content, |style| style.display = Display::Block);
    doc.update_resolved_style(content, |style| style.height = Length::px(90.0));
    doc.append_child(padded, content);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(contains_node(column_fragments[0], float));
    assert!(contains_node(column_fragments[0], spacer));
    assert!(contains_node(column_fragments[1], padded));
}

#[test]
fn floated_column_span_all_box_remains_in_column_flow() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(220.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(20.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let floated = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(floated, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_span = ColumnSpan::All);
        style.update_derived(|computed| computed.float = Float::Right);
        style.update_derived(|computed| computed.width = Length::px(220.0));
        style.update_derived(|computed| computed.height = Length::px(20.0));
    });
    doc.append_child(multicol, floated);

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.height = Length::px(10.0));
    doc.append_child(multicol, spanner);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(220), lu(600), lu(220), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert!(!column_fragments.is_empty());
    assert!(contains_node(column_fragments[0], floated));
    assert_eq!(find_node(&fragment, spanner).unwrap().offset.top, lu(20));
    assert!(fragment
        .children
        .iter()
        .filter(|child| child.kind != FragmentKind::ColumnBox)
        .all(|child| !contains_node(child, floated)));
}

#[test]
fn full_width_float_excludes_following_inline_until_its_block_end() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.font_size = 1.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Number(1.0));
    });
    doc.append_child(doc.root(), multicol);

    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| style.display = Display::Block);
    doc.update_resolved_style(float, |style| style.float = Float::Right);
    doc.update_resolved_style(float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(float, |style| style.height = Length::px(150.0));
    doc.append_child(multicol, float);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("x".into());
    doc.update_resolved_style(text, |style| style.font_size = 1.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Number(1.0));
    doc.append_child(multicol, text);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(contains_node(column_fragments[0], float));
    assert!(contains_node(column_fragments[1], text));
}

#[test]
fn empty_flex_trailing_padding_does_not_create_an_overflow_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.padding_top = Length::px(20.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(100.0));
        style.update_derived(|computed| computed.background_color = Color::GREEN);
    });
    doc.append_child(multicol, flex);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert_eq!(column_fragments[0].size.height, lu(100));
    assert_eq!(column_fragments[1].size.height, lu(100));
}

#[test]
fn cleared_float_negative_visual_overflow_reaches_preceding_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let first_float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first_float, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.float = Float::Left);
        style.update_derived(|computed| computed.width = Length::percent(100.0));
        style.update_derived(|computed| computed.height = Length::px(200.0));
    });
    doc.append_child(multicol, first_float);
    let first_ink = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first_ink, |style| style.display = Display::Block);
    doc.update_resolved_style(first_ink, |style| style.height = Length::px(160.0));
    doc.update_resolved_style(first_ink, |style| style.background_color = Color::GREEN);
    doc.append_child(first_float, first_ink);

    let cleared_float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(cleared_float, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.float = Float::Left);
        style.update_derived(|computed| computed.clear = Clear::Left);
        style.update_derived(|computed| computed.width = Length::percent(100.0));
        style.update_derived(|computed| computed.height = Length::px(0.0));
    });
    doc.append_child(multicol, cleared_float);
    let negative_ink = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(negative_ink, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(40.0));
        style.update_derived(|computed| computed.margin_top = Length::px(-40.0));
        style.update_derived(|computed| computed.background_color = Color::GREEN);
    });
    doc.append_child(cleared_float, negative_ink);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert!(column_fragments.len() >= 2);
    let continuation = find_node(column_fragments[1], cleared_float)
        .expect("negative float visual continuation in the second column");
    let ink = find_node(continuation, negative_ink).expect("negative-margin float ink");
    assert_eq!(continuation.offset.top + ink.offset.top, lu(60));
}

#[test]
fn cleared_flow_root_negative_visual_overflow_reaches_preceding_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let first_float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first_float, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.float = Float::Left);
        style.update_derived(|computed| computed.width = Length::percent(100.0));
        style.update_derived(|computed| computed.height = Length::px(200.0));
    });
    doc.append_child(multicol, first_float);
    let first_ink = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first_ink, |style| style.display = Display::Block);
    doc.update_resolved_style(first_ink, |style| style.height = Length::px(160.0));
    doc.append_child(first_float, first_ink);

    let cleared = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(cleared, |style| {
        style.update_derived(|computed| computed.display = Display::FlowRoot);
        style.update_derived(|computed| computed.clear = Clear::Left);
        style.update_derived(|computed| computed.height = Length::px(0.0));
    });
    doc.append_child(multicol, cleared);
    let negative_ink = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(negative_ink, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(40.0));
        style.update_derived(|computed| computed.margin_top = Length::px(-40.0));
    });
    doc.append_child(cleared, negative_ink);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    let continuation = find_node(column_fragments[1], cleared)
        .expect("cleared negative overflow in the preceding column");
    let ink = find_node(continuation, negative_ink).expect("negative-margin ink");
    assert_eq!(continuation.offset.top + ink.offset.top, lu(60));
}

#[test]
fn positioned_only_opacity_source_is_not_painted_beside_its_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let opacity = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(opacity, |style| style.display = Display::Block);
    doc.update_resolved_style(opacity, |style| style.opacity = 0.5);
    doc.append_child(multicol, opacity);
    let absolute = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(absolute, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
    });
    doc.append_child(opacity, absolute);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    assert_eq!(count_node(&fragment, opacity), 1);
    assert!(columns(&fragment)
        .iter()
        .any(|column| contains_node(column, opacity)));
}

#[test]
fn definite_row_flex_visual_overflow_does_not_advance_following_flow() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| style.display = Display::Block);
    doc.update_resolved_style(item, |style| style.line_height = LineHeight::Number(0.0));
    doc.append_child(flex, item);
    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(first, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(first, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(first, |style| style.line_height = LineHeight::Number(0.0));
    doc.append_child(item, first);
    let second = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(second, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(second, |style| style.height = Length::px(100.0));
    doc.update_resolved_style(second, |style| style.line_height = LineHeight::Number(0.0));
    doc.append_child(item, second);
    let trailing = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(trailing, |style| style.display = Display::Block);
    doc.update_resolved_style(trailing, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(trailing, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, trailing);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(contains_node(column_fragments[0], flex));
    let continuation = find_node(column_fragments[1], flex).expect("visual continuation");
    assert!(continuation.has_overflow_clip);
    let trailing = find_node(column_fragments[1], trailing).expect("following principal box");
    assert_eq!(trailing.offset.top, LayoutUnit::zero());
}

#[test]
fn following_box_uses_space_after_a_fragmented_overflow_box() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(overflow, |style| style.margin_bottom = Length::px(20.0));
    doc.append_child(multicol, overflow);
    let overflow_content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow_content, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow_content, |style| style.height = Length::px(200.0));
    doc.append_child(overflow, overflow_content);
    let following = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(following, |style| style.display = Display::Block);
    doc.update_resolved_style(following, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, following);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert!(contains_node(column_fragments[0], following));
    assert!(contains_node(column_fragments[1], following));
}

#[test]
fn static_inline_position_after_full_columns_starts_in_overflow_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(4));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.width = Length::px(25.0));
    doc.append_child(multicol, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| style.height = Length::px(400.0));
    doc.append_child(flex, item);
    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| {
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.top = Length::px(0.0));
        style.update_derived(|computed| computed.width = Length::px(25.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
    });
    doc.append_child(multicol, abspos);

    let space = ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false);
    let fragment = block_layout(&doc, multicol, &space);
    let positioned = fragment
        .children
        .iter()
        .find(|child| child.node_id == abspos)
        .expect("out-of-flow fragment");
    assert_eq!(positioned.offset.left, lu(100));
}

#[test]
fn fixed_height_auto_fill_creates_inline_overflow_columns() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(40.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    for _ in 0..3 {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| style.display = Display::Block);
        doc.update_resolved_style(child, |style| style.width = Length::px(20.0));
        doc.update_resolved_style(child, |style| style.height = Length::px(50.0));
        doc.append_child(multicol, child);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(40), lu(600), lu(40), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    assert_eq!(column_fragments[0].offset.left, lu(0));
    assert_eq!(column_fragments[1].offset.left, lu(20));
    assert_eq!(column_fragments[2].offset.left, lu(40));
}

#[test]
fn specified_insets_use_the_multicol_padding_box_origin() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(4));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    for _ in 0..2 {
        let block = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(block, |style| style.display = Display::Block);
        doc.update_resolved_style(block, |style| style.width = Length::px(25.0));
        doc.update_resolved_style(block, |style| style.height = Length::px(50.0));
        doc.append_child(multicol, block);
    }
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.width = Length::px(25.0));
    doc.append_child(multicol, flex);
    for index in 0..2 {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.height = Length::px(50.0));
        if index == 1 {
            doc.update_resolved_style(item, |style| style.break_after = BreakValue::Avoid);
        }
        doc.append_child(flex, item);
    }
    let tall = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tall, |style| style.display = Display::Block);
    doc.update_resolved_style(tall, |style| style.width = Length::px(25.0));
    doc.update_resolved_style(tall, |style| style.height = Length::px(150.0));
    doc.append_child(multicol, tall);
    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| {
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.left = Length::px(25.0));
        style.update_derived(|computed| computed.top = Length::px(50.0));
        style.update_derived(|computed| computed.width = Length::px(25.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
    });
    doc.append_child(multicol, abspos);

    let space = ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false);
    let fragment = block_layout(&doc, multicol, &space);
    let positioned = fragment
        .children
        .iter()
        .find(|child| child.node_id == abspos)
        .expect("out-of-flow fragment");
    assert_eq!(positioned.offset.left, lu(25));
    assert_eq!(positioned.offset.top, lu(50));
}

#[test]
fn direct_text_flex_items_create_paintable_inline_fragments() {
    let mut doc = Document::new();
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.append_child(doc.root(), flex);
    let text = doc.create_node(ElementTag::Text);
    doc.update_resolved_style(text, |style| style.font_size = 16.0);
    doc.node_mut(text).text = Some("quotes".to_string());
    doc.append_child(flex, text);

    let space = ConstraintSpace::for_block_child(lu(200), lu(100), lu(200), lu(100), false);
    let fragment = block_layout(&doc, flex, &space);
    assert!(fragment.size.height > LayoutUnit::zero());
    assert!(contains_kind(&fragment, FragmentKind::Text));
}

#[test]
fn absolute_inline_shrink_to_fit_uses_shaped_max_content_width() {
    let mut doc =
        Document::new_with_font_collection(openui_text::FontCollection::deterministic_test());
    let absolute = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(absolute, |style| {
        style.update_derived(|computed| computed.display = Display::Inline);
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.top = Length::px(-20.0));
        style.update_derived(|computed| computed.font_family = FontFamilyList::single("Ahem"));
        style.update_derived(|computed| computed.font_size = 20.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Number(1.0));
    });
    doc.append_child(doc.root(), absolute);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("re dd".to_string());
    doc.update_resolved_style(text, |style| {
        style.font_family = FontFamilyList::single("Ahem")
    });
    doc.update_resolved_style(text, |style| style.font_size = 20.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Number(1.0));
    doc.append_child(absolute, text);

    let fragment = block_layout(
        &doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    );
    let positioned = find_node(&fragment, absolute).expect("absolute fragment");
    assert_eq!(positioned.offset.top, lu(-20));
    assert!(positioned.size.width > lu(99));
    assert!(positioned.size.width <= lu(100));
    assert_eq!(positioned.size.height, lu(20));
}

#[test]
fn avoid_flex_item_moves_whole_to_the_next_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.row_gap = Some(Length::px(10.0)));
    doc.update_resolved_style(flex, |style| style.flex_wrap = openui_style::FlexWrap::Wrap);
    doc.append_child(multicol, flex);

    let mut items = Vec::new();
    for height in [50.0, 50.0, 50.0, 30.0] {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.width = Length::percent(100.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(height));
        doc.append_child(flex, item);
        items.push(item);
    }
    doc.update_resolved_style(items[1], |style| style.break_inside = BreakInside::Avoid);
    doc.update_resolved_style(items[2], |style| style.break_before = BreakValue::Column);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    let second_flex = find_node(column_fragments[1], flex).expect("second flex fragment");
    let avoid_item = find_node(second_flex, items[1]).expect("avoid item continuation");
    assert_eq!(avoid_item.offset.top, LayoutUnit::zero());
}

#[test]
fn nested_fragmentation_context_is_monolithic_in_outer_balancing() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| style.display = Display::Block);
    doc.update_resolved_style(outer, |style| style.width = Length::px(200.0));
    doc.update_resolved_style(outer, |style| style.column_count = Some(2));
    doc.update_resolved_style(outer, |style| style.column_gap = Some(Length::px(0.0)));
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| style.display = Display::Block);
    doc.update_resolved_style(inner, |style| style.box_sizing = BoxSizing::ContentBox);
    doc.update_resolved_style(inner, |style| style.height = Length::px(20.0));
    doc.update_resolved_style(inner, |style| style.padding_top = Length::px(9.0));
    doc.update_resolved_style(inner, |style| style.column_count = Some(2));
    doc.update_resolved_style(inner, |style| style.column_fill = ColumnFill::Auto);
    doc.update_resolved_style(inner, |style| style.line_height = LineHeight::Length(1.0));
    doc.append_child(outer, inner);
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("x".to_string());
    doc.update_resolved_style(text, |style| {
        style.font_family = FontFamilyList::single("Ahem")
    });
    doc.update_resolved_style(text, |style| style.font_size = 16.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Length(1.0));
    doc.append_child(inner, text);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(200), lu(600), lu(200), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 1);
    let inner_fragment = find_node(column_fragments[0], inner).expect("nested multicol fragment");
    assert!(!column_fragments[0].has_overflow_clip);
    let inner_columns = columns(inner_fragment);
    assert_eq!(inner_columns.len(), 1);
    assert!(!inner_columns[0].has_overflow_clip);
}

#[test]
fn oversized_avoid_box_moves_auto_height_nested_multicol_to_fresh_outer_column() {
    for fill in [ColumnFill::Auto, ColumnFill::Balance] {
        let mut doc = Document::new();
        let outer = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(outer, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.width = Length::px(100.0));
            style.update_derived(|computed| computed.height = Length::px(150.0));
            style.update_derived(|computed| computed.column_count = Some(2));
            style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
            style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        });
        doc.append_child(doc.root(), outer);

        let prefix = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(prefix, |style| style.display = Display::Block);
        doc.update_resolved_style(prefix, |style| style.height = Length::px(100.0));
        doc.append_child(outer, prefix);

        let inner = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(inner, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.column_count = Some(2));
            style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
            style.update_derived(|computed| computed.column_fill = fill);
        });
        doc.append_child(outer, inner);

        let avoided = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(avoided, |style| style.display = Display::Block);
        doc.update_resolved_style(avoided, |style| style.width = Length::percent(200.0));
        doc.update_resolved_style(avoided, |style| style.height = Length::px(100.0));
        doc.update_resolved_style(avoided, |style| style.break_inside = BreakInside::Avoid);
        doc.append_child(inner, avoided);

        let fragment = block_layout(
            &doc,
            outer,
            &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
        );
        let outer_columns = columns(&fragment);
        assert_eq!(outer_columns.len(), 2);
        assert!(!contains_node(outer_columns[0], inner));
        let inner_fragment =
            find_node(outer_columns[1], inner).expect("nested multicol in fresh column");
        assert_eq!(inner_fragment.size.height, lu(100));
        assert!(contains_node(inner_fragment, avoided));
    }
}

#[test]
fn full_width_float_exclusion_advances_following_inline_content_across_columns() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(200.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.direction = Direction::Rtl);
        style.update_derived(|computed| computed.font_size = 1.0);
    });
    doc.append_child(doc.root(), multicol);

    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| style.display = Display::Block);
    doc.update_resolved_style(float, |style| style.float = openui_style::Float::Right);
    doc.update_resolved_style(float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(float, |style| style.height = Length::px(150.0));
    doc.append_child(multicol, float);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("x".to_string());
    doc.update_resolved_style(text, |style| style.font_size = 1.0);
    doc.update_resolved_style(text, |style| {
        style.font_family = FontFamilyList::single("Ahem")
    });
    doc.append_child(multicol, text);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(200), lu(600), lu(200), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(column_fragments[0].offset.left > column_fragments[1].offset.left);
    assert!(contains_node(column_fragments[1], text));
    let continuation = find_node(column_fragments[1], text).expect("text continuation");
    assert_eq!(continuation.children[0].offset.top, lu(50));
}

#[test]
fn avoided_descendant_uses_fresh_fragmentainer_capacity() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::Block);
    doc.update_resolved_style(before, |style| style.height = Length::px(20.0));
    doc.append_child(multicol, before);

    let flow_root = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flow_root, |style| style.display = Display::FlowRoot);
    doc.append_child(multicol, flow_root);
    let spacer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spacer, |style| style.display = Display::Block);
    doc.update_resolved_style(spacer, |style| style.height = Length::px(40.0));
    doc.append_child(flow_root, spacer);
    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| style.display = Display::Block);
    doc.update_resolved_style(float, |style| style.float = Float::Left);
    doc.update_resolved_style(float, |style| style.break_inside = BreakInside::Avoid);
    doc.update_resolved_style(float, |style| style.width = Length::px(20.0));
    doc.update_resolved_style(float, |style| style.height = Length::px(100.0));
    doc.append_child(flow_root, float);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let continuation = find_node(column_fragments[1], flow_root).expect("flow-root continuation");
    assert_eq!(continuation.size.height, lu(100));
    assert_eq!(
        find_node(continuation, float)
            .expect("avoided float")
            .offset
            .top,
        LayoutUnit::zero()
    );
}

#[test]
fn clear_after_self_collapsing_float_wrapper_resumes_after_fragmented_float() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::Block);
    doc.update_resolved_style(before, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, before);

    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| style.display = Display::Block);
    doc.update_resolved_style(float, |style| style.float = Float::Left);
    doc.update_resolved_style(float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(float, |style| style.height = Length::px(100.0));
    doc.append_child(wrapper, float);

    let cleared = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(cleared, |style| style.display = Display::Block);
    doc.update_resolved_style(cleared, |style| style.clear = Clear::Both);
    doc.update_resolved_style(cleared, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, cleared);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(find_node(column_fragments[0], cleared).is_none());
    assert_eq!(
        find_node(column_fragments[1], cleared)
            .expect("cleared continuation")
            .offset
            .top,
        lu(50)
    );
}

#[test]
fn negative_float_margin_does_not_consume_continuation_source_space() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::Block);
    doc.update_resolved_style(before, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, before);

    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    let float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(float, |style| style.display = Display::Block);
    doc.update_resolved_style(float, |style| style.float = Float::Left);
    doc.update_resolved_style(float, |style| style.margin_top = Length::px(-10.0));
    doc.update_resolved_style(float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(float, |style| style.height = Length::px(100.0));
    doc.append_child(wrapper, float);
    let collapsing_margin = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(collapsing_margin, |style| style.display = Display::Block);
    doc.update_resolved_style(collapsing_margin, |style| {
        style.margin_top = Length::px(10.0)
    });
    doc.append_child(wrapper, collapsing_margin);

    let cleared = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(cleared, |style| style.display = Display::Block);
    doc.update_resolved_style(cleared, |style| style.clear = Clear::Both);
    doc.update_resolved_style(cleared, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, cleared);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert_eq!(
        find_node(column_fragments[1], wrapper)
            .expect("float wrapper continuation")
            .size
            .height,
        lu(50)
    );
    assert_eq!(
        find_node(column_fragments[1], cleared)
            .expect("cleared continuation")
            .offset
            .top,
        lu(50)
    );
}

#[test]
fn avoided_row_flex_break_tracks_visual_and_content_consumption_separately() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_wrap = openui_style::FlexWrap::Wrap);
        style.update_derived(|computed| computed.height = Length::px(150.0));
        style.update_derived(|computed| {
            computed.align_content =
                ContentAlignment::with_distribution(ContentDistribution::SpaceBetween)
        });
    });
    doc.append_child(multicol, flex);

    for (width, height, avoid) in [
        (50, 50, false),
        (25, 10, false),
        (25, 50, true),
        (50, 25, false),
    ] {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.width = Length::px(width as f32));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if avoid {
            doc.update_resolved_style(item, |style| style.break_inside = BreakInside::Avoid);
        }
        doc.append_child(flex, item);
    }

    let static_absolute = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(static_absolute, |style| style.position = Position::Absolute);
    doc.update_resolved_style(static_absolute, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(static_absolute, |style| style.height = Length::px(13.0));
    doc.append_child(multicol, static_absolute);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let first_flex = find_node(column_fragments[0], flex).expect("first flex fragment");
    let second_flex = find_node(column_fragments[1], flex).expect("second flex fragment");
    assert_eq!(first_flex.size.height, lu(100));
    assert_eq!(second_flex.size.height, lu(50));
    assert!(!second_flex.has_overflow_clip);

    let absolute = find_node(&fragment, static_absolute).expect("static absolute fragment");
    assert_eq!(absolute.offset.left, lu(50));
    assert_eq!(absolute.offset.top, lu(50));
}

#[test]
fn auto_row_flex_edge_avoid_keeps_first_line_in_current_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
    });
    doc.append_child(multicol, before);

    let positioned = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(positioned, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.top = Length::px(75.0));
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(25.0));
    });
    doc.append_child(multicol, positioned);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
    });
    doc.append_child(multicol, flex);
    let first_item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first_item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(25.0));
    });
    doc.append_child(flex, first_item);
    let second_item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second_item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(25.0));
        style.update_derived(|computed| computed.break_after = BreakValue::Avoid);
    });
    doc.append_child(flex, second_item);

    let following = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(following, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
    });
    doc.append_child(multicol, following);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);

    let first_flex = find_node(column_fragments[0], flex).expect("leading flex fragment");
    assert_eq!(first_flex.offset.top, lu(50));
    assert_eq!(first_flex.size.height, lu(50));
    assert_eq!(first_flex.children.len(), 1);
    assert_eq!(first_flex.children[0].node_id, first_item);
    assert_eq!(first_flex.children[0].offset.top, LayoutUnit::zero());

    let second_flex = find_node(column_fragments[1], flex).expect("final flex fragment");
    assert_eq!(second_flex.offset.top, LayoutUnit::zero());
    assert_eq!(second_flex.size.height, lu(25));
    assert_eq!(second_flex.children.len(), 1);
    assert_eq!(second_flex.children[0].node_id, second_item);
    assert_eq!(second_flex.children[0].offset.top, LayoutUnit::zero());
    assert_eq!(
        find_node(column_fragments[1], following)
            .unwrap()
            .offset
            .top,
        lu(25)
    );
}

#[test]
fn spanner_inside_transparent_inline_splits_overflowed_block_content() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(300.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.line_height = LineHeight::Length(20.0));
    });
    doc.append_child(doc.root(), multicol);

    let overflowed = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflowed, |style| style.display = Display::Block);
    doc.update_resolved_style(overflowed, |style| style.height = Length::px(15.0));
    doc.update_resolved_style(overflowed, |style| {
        style.line_height = LineHeight::Length(20.0)
    });
    doc.append_child(multicol, overflowed);
    for _ in 0..6 {
        let line_break = doc.create_node(ElementTag::Text);
        doc.node_mut(line_break).text = Some("\n".to_string());
        doc.update_resolved_style(line_break, |style| style.white_space = WhiteSpace::PreLine);
        doc.update_resolved_style(line_break, |style| {
            style.line_height = LineHeight::Length(20.0)
        });
        doc.append_child(overflowed, line_break);
    }

    let span = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(span, |style| style.display = Display::Inline);
    doc.append_child(overflowed, span);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.height = Length::px(10.0));
    doc.append_child(span, spanner);

    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| style.display = Display::Block);
    doc.update_resolved_style(tail, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, tail);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(300), lu(600), lu(300), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 6);
    assert_eq!(column_fragments[0].size.height, lu(40));
    assert_eq!(column_fragments[1].size.height, lu(40));
    assert_eq!(column_fragments[2].size.height, lu(40));
    let spanner_fragment = find_node(&fragment, spanner).expect("extracted spanner");
    assert_eq!(spanner_fragment.offset.top, lu(40));
    assert_eq!(spanner_fragment.size.height, lu(10));
    assert_eq!(column_fragments[3].offset.top, lu(50));
    assert!(column_fragments[3].size.height >= lu(33));
    assert!(column_fragments[3].size.height < lu(34));
}

#[test]
fn inline_break_token_resumes_after_oversized_inline_block_line() {
    let mut doc =
        Document::new_with_font_collection(openui_text::FontCollection::deterministic_test());
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(300.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.line_height = LineHeight::Length(20.0));
        style.update_derived(|computed| computed.orphans = 1);
        style.update_derived(|computed| computed.widows = 1);
    });
    doc.append_child(doc.root(), multicol);

    let inline_container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inline_container, |style| style.display = Display::Block);
    doc.update_resolved_style(inline_container, |style| style.width = Length::px(1.0));
    doc.update_resolved_style(inline_container, |style| {
        style.line_height = LineHeight::Length(20.0)
    });
    doc.update_resolved_style(inline_container, |style| style.orphans = 1);
    doc.update_resolved_style(inline_container, |style| style.widows = 1);
    doc.append_child(multicol, inline_container);

    let line_break = doc.create_node(ElementTag::Text);
    doc.node_mut(line_break).text = Some("\n".to_string());
    doc.update_resolved_style(line_break, |style| style.white_space = WhiteSpace::PreLine);
    doc.update_resolved_style(line_break, |style| {
        style.line_height = LineHeight::Length(20.0)
    });
    doc.append_child(inline_container, line_break);

    let inline_block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inline_block, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(inline_block, |style| style.height = Length::px(100.0));
    doc.append_child(inline_container, inline_block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("x".to_string());
    doc.update_resolved_style(text, |style| {
        style.font_family = FontFamilyList::single("Ahem")
    });
    doc.update_resolved_style(text, |style| style.font_size = 16.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Length(20.0));
    doc.append_child(inline_container, text);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(300), lu(600), lu(300), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    assert!(contains_node(column_fragments[0], inline_container));
    assert!(contains_node(column_fragments[1], inline_block));
    assert!(contains_node(column_fragments[2], text));
    let text_fragment = find_node(column_fragments[2], text).expect("resumed text");
    assert!(text_fragment.offset.top >= lu(2));
    assert!(text_fragment.offset.top < lu(3));
}

#[test]
fn inline_break_token_preserves_trailing_decoration_at_column_edge() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.orphans = 1);
        style.update_derived(|computed| computed.widows = 1);
    });
    doc.append_child(doc.root(), multicol);

    let inline_container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inline_container, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.padding_bottom = Length::px(50.0));
        style.update_derived(|computed| computed.orphans = 1);
        style.update_derived(|computed| computed.widows = 1);
    });
    doc.append_child(multicol, inline_container);
    for _ in 0..2 {
        let inline_block = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(inline_block, |style| {
            style.display = Display::InlineBlock;
            style.vertical_align = VerticalAlign::Top;
            style.width = Length::percent(100.0);
            style.height = Length::px(50.0);
        });
        doc.append_child(inline_container, inline_block);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let first = find_node(column_fragments[0], inline_container).expect("first continuation");
    let second = find_node(column_fragments[1], inline_container).expect("trailing decoration");
    assert_eq!(first.size.height, lu(100));
    assert_eq!(second.size.height, lu(100));
    assert!(!first.is_last_for_node);
    assert!(second.is_last_for_node);
}

#[test]
fn overconstrained_cloned_decoration_makes_content_progress_per_column() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(5.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
    });
    doc.append_child(doc.root(), multicol);

    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(10.0));
        style.update_derived(|computed| computed.padding_top = Length::px(4.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(4.0));
        style.update_derived(|computed| computed.border_top_width = 1);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Dotted);
        style.update_derived(|computed| computed.border_bottom_width = 1);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Dotted);
        style.update_derived(|computed| computed.box_decoration_break = BoxDecorationBreak::Clone);
    });
    doc.append_child(multicol, child);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    for column in column_fragments {
        assert_eq!(column.size.height, lu(11));
        assert_eq!(column.children.len(), 1);
        assert_eq!(column.children[0].size.height, lu(11));
    }
}

#[test]
fn early_forced_row_flex_break_is_the_continuation_origin() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
        style.update_derived(|computed| computed.row_gap = Some(Length::px(10.0)));
    });
    doc.append_child(multicol, flex);
    let mut items = Vec::new();
    for index in 0..4 {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::percent(100.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(50.0));
        if index == 1 {
            doc.update_resolved_style(item, |style| style.break_before = BreakValue::Column);
        }
        doc.append_child(flex, item);
        items.push(item);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    let second_flex = find_node(column_fragments[1], flex).expect("second flex continuation");
    assert_eq!(find_node(second_flex, items[1]).unwrap().offset.top, lu(0));
    assert_eq!(find_node(second_flex, items[2]).unwrap().offset.top, lu(60));
    let third_flex = find_node(column_fragments[2], flex).expect("third flex continuation");
    assert_eq!(find_node(third_flex, items[2]).unwrap().offset.top, lu(-40));
    assert_eq!(find_node(third_flex, items[3]).unwrap().offset.top, lu(20));
}

#[test]
fn nested_row_flex_break_propagates_only_from_its_first_line() {
    for (forced_child, expected_top) in [(1, lu(100)), (2, lu(50))] {
        let mut doc = Document::new();
        let multicol = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(multicol, |style| {
            style.display = Display::Block;
            style.width = Length::px(100.0);
            style.height = Length::px(100.0);
            style.column_count = Some(2);
            style.column_gap = Some(Length::px(0.0));
            style.column_fill = ColumnFill::Auto;
        });
        doc.append_child(doc.root(), multicol);

        let outer = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(outer, |style| {
            style.display = Display::Flex;
            style.flex_wrap = FlexWrap::Wrap;
        });
        doc.append_child(multicol, outer);
        let first = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(first, |style| {
            style.width = Length::px(50.0);
            style.height = Length::px(50.0);
        });
        doc.append_child(outer, first);

        let nested = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(nested, |style| {
            style.display = Display::Flex;
            style.flex_wrap = FlexWrap::Wrap;
            style.width = Length::px(50.0);
        });
        doc.append_child(outer, nested);
        for (index, width) in [25.0, 25.0, 50.0].into_iter().enumerate() {
            let child = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(child, |style| {
                style.width = Length::px(width);
                style.height = Length::px(25.0);
                if index == forced_child {
                    style.break_before = BreakValue::Column;
                }
            });
            doc.append_child(nested, child);
        }
        let last = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(last, |style| {
            style.width = Length::px(50.0);
            style.height = Length::px(50.0);
        });
        doc.append_child(outer, last);

        let fragment = block_layout(
            &doc,
            multicol,
            &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
        );
        let column_fragments = columns(&fragment);
        let first_outer = find_node(column_fragments[0], outer).expect("first flex slice");
        assert_eq!(
            find_node(first_outer, nested).unwrap().offset.top,
            expected_top
        );
    }
}

#[test]
fn fixed_row_flex_height_includes_an_early_forced_break_gap() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.position = Position::Relative);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
        style.update_derived(|computed| computed.height = Length::px(150.0));
        style.update_derived(|computed| {
            computed.align_content =
                ContentAlignment::with_distribution(ContentDistribution::SpaceBetween)
        });
    });
    doc.append_child(multicol, flex);
    let mut items = Vec::new();
    for (index, width, height) in [(0, 50, 25), (1, 25, 10), (2, 25, 25), (3, 50, 50)] {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(width as f32));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if index == 2 {
            doc.update_resolved_style(item, |style| style.break_before = BreakValue::Column);
        }
        doc.append_child(flex, item);
        items.push(item);
    }
    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| style.display = Display::Block);
    doc.update_resolved_style(abspos, |style| style.position = Position::Absolute);
    doc.update_resolved_style(abspos, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(abspos, |style| style.height = Length::px(25.0));
    doc.append_child(multicol, abspos);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let continuation = find_node(column_fragments[1], flex).expect("flex continuation");
    assert_eq!(continuation.size.height, lu(100));
    assert_eq!(find_node(continuation, items[2]).unwrap().offset.top, lu(0));
    assert_eq!(
        find_node(continuation, items[3]).unwrap().offset.top,
        lu(50)
    );
    let positioned = find_node(&fragment, abspos).expect("static-positioned sibling");
    assert_eq!(positioned.offset.left, lu(50));
    assert_eq!(positioned.offset.top, lu(50));
}

#[test]
fn avoided_column_flex_items_resume_from_shared_break_offsets() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(5));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
        style.update_derived(|computed| computed.height = Length::px(500.0));
    });
    doc.append_child(multicol, flex);

    let mut items = Vec::new();
    for height in [250, 200, 120, 180, 100] {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(10.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        doc.update_resolved_style(item, |style| style.break_inside = BreakInside::Avoid);
        doc.append_child(flex, item);
        items.push(item);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 5);
    let continuation = find_node(column_fragments[4], flex).expect("fifth flex continuation");
    let final_item = find_node(continuation, items[4]).expect("avoided final flex item");
    assert_eq!(final_item.offset.top, lu(0));
    assert_eq!(final_item.size.height, lu(100));
}

#[test]
fn forced_column_flex_break_extends_ink_but_not_fixed_static_flow() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(5));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::Block);
    doc.update_resolved_style(before, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, before);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
        style.update_derived(|computed| computed.height = Length::px(350.0));
    });
    doc.append_child(multicol, flex);
    let mut items = Vec::new();
    for (index, height) in [50, 50, 250, 100, 50, 50, 150].into_iter().enumerate() {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(10.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if index == 3 {
            doc.update_resolved_style(item, |style| style.break_before = BreakValue::Column);
        }
        doc.append_child(flex, item);
        items.push(item);
    }

    let after = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(after, |style| style.display = Display::Block);
    doc.update_resolved_style(after, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, after);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 5);
    let second_continuation =
        find_node(column_fragments[1], flex).expect("second flex continuation");
    assert_eq!(
        find_node(second_continuation, items[3])
            .expect("forced item")
            .offset
            .top,
        lu(0)
    );
    let following = find_node(column_fragments[4], after).expect("following block");
    assert_eq!(following.offset.top, lu(0));
}

#[test]
fn forced_column_flex_break_splits_crossing_parallel_item() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(5));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
        style.update_derived(|computed| computed.height = Length::px(500.0));
    });
    doc.append_child(multicol, flex);

    for (index, height) in [50, 50, 350].into_iter().enumerate() {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(10.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if index == 1 {
            doc.update_resolved_style(item, |style| style.break_before = BreakValue::Column);
        }
        doc.append_child(flex, item);
    }
    let crossing = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(crossing, |style| style.display = Display::Block);
    doc.update_resolved_style(crossing, |style| style.width = Length::px(10.0));
    doc.update_resolved_style(crossing, |style| style.height = Length::px(100.0));
    doc.append_child(flex, crossing);
    for height in [50, 50, 250] {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(10.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        doc.append_child(flex, item);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 5);
    let first = find_node(column_fragments[0], crossing).expect("leading item fragment");
    assert_eq!(first.offset.top, lu(0));
    assert_eq!(first.size.height, lu(100));
    let second_flex = find_node(column_fragments[1], flex).expect("second flex continuation");
    let second = second_flex
        .children
        .iter()
        .find(|child| {
            child.node_id == crossing && child.offset.top == lu(0) && child.size.height == lu(50)
        })
        .expect("continued item fragment");
    assert!(!second.is_first_for_node);
}

#[test]
fn forced_column_flex_item_after_full_start_margin_fills_next_fragment() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(60.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.height = Length::px(300.0));
    doc.append_child(multicol, flex);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| style.display = Display::Block);
    doc.update_resolved_style(item, |style| style.width = Length::px(20.0));
    doc.update_resolved_style(item, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(item, |style| style.margin_top = Length::px(100.0));
    doc.update_resolved_style(item, |style| style.break_before = BreakValue::Column);
    doc.append_child(flex, item);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(60), lu(600), lu(60), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    let second_flex = find_node(column_fragments[1], flex).expect("second flex continuation");
    let forced = find_node(second_flex, item).expect("forced item fragment");
    assert_eq!(forced.offset.top, lu(0));
    assert_eq!(forced.size.height, lu(150));
}

#[test]
fn avoided_column_flex_item_carries_its_end_margin_into_resumption() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.append_child(multicol, flex);

    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.update_resolved_style(first, |style| style.height = Length::px(50.0));
    doc.append_child(flex, first);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| style.height = Length::px(100.0));
    doc.append_child(first, overflow);

    let avoided = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(avoided, |style| style.display = Display::Block);
    doc.update_resolved_style(avoided, |style| style.height = Length::px(60.0));
    doc.update_resolved_style(avoided, |style| style.margin_bottom = Length::px(20.0));
    doc.update_resolved_style(avoided, |style| style.break_inside = BreakInside::Avoid);
    doc.append_child(flex, avoided);

    let following = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(following, |style| style.display = Display::Block);
    doc.update_resolved_style(following, |style| style.height = Length::px(40.0));
    doc.update_resolved_style(following, |style| style.margin_top = Length::px(-20.0));
    doc.append_child(multicol, following);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let resumed = find_node(column_fragments[1], following).expect("following block");
    assert_eq!(resumed.offset.top, lu(60));
}

#[test]
fn positioned_column_flex_item_keeps_its_definite_continuation_size() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| {
            computed.justify_content = ContentAlignment::new(openui_style::ContentPosition::Center)
        });
    });
    doc.append_child(multicol, flex);

    let positioned = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(positioned, |style| style.display = Display::Block);
    doc.update_resolved_style(positioned, |style| style.position = Position::Absolute);
    doc.update_resolved_style(positioned, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(positioned, |style| style.height = Length::px(100.0));
    doc.append_child(flex, positioned);
    for index in 0..3 {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(50.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(50.0));
        doc.update_resolved_style(item, |style| style.flex_grow = 0.0);
        doc.update_resolved_style(item, |style| style.flex_shrink = 0.0);
        if index == 1 {
            doc.update_resolved_style(item, |style| style.break_before = BreakValue::Column);
        }
        doc.append_child(flex, item);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(find_node(column_fragments[1], flex).is_some());
    let positioned_fragments = find_nodes(&fragment, positioned);
    assert_eq!(positioned_fragments.len(), 2);
    assert_eq!(positioned_fragments[0].offset.left, LayoutUnit::zero());
    assert_eq!(positioned_fragments[0].offset.top, lu(25));
    assert_eq!(positioned_fragments[0].size.height, lu(75));
    assert_eq!(positioned_fragments[1].offset.left, lu(50));
    assert_eq!(positioned_fragments[1].offset.top, LayoutUnit::zero());
    assert_eq!(positioned_fragments[1].size.height, lu(25));
}

#[test]
fn abspos_in_fragmented_wrapped_column_flex_uses_final_item_continuation() {
    fn add_contained_child(doc: &mut Document, parent: NodeId, height: i32) {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| {
            style.display = Display::Block;
            style.width = Length::px(10.0);
            style.height = Length::px(height as f32);
            style.contain = Containment::SIZE;
        });
        doc.append_child(parent, child);
    }

    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.display = Display::Block;
        style.width = Length::px(100.0);
        style.height = Length::px(100.0);
        style.column_count = Some(5);
        style.column_gap = Some(Length::px(0.0));
        style.column_fill = ColumnFill::Auto;
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Column;
        style.flex_wrap = FlexWrap::Wrap;
        style.height = Length::px(500.0);
    });
    doc.append_child(multicol, flex);

    let add_item = |doc: &mut Document| {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| {
            style.display = Display::Block;
            style.width = Length::px(10.0);
            style.flex_grow = 0.0;
            style.flex_shrink = 0.0;
        });
        doc.append_child(flex, item);
        item
    };

    let first = add_item(&mut doc);
    add_contained_child(&mut doc, first, 80);
    add_contained_child(&mut doc, first, 30);

    let second = add_item(&mut doc);
    doc.update_resolved_style(second, |style| style.position = Position::Relative);
    add_contained_child(&mut doc, second, 70);
    add_contained_child(&mut doc, second, 40);
    let second_abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second_abspos, |style| {
        style.display = Display::Block;
        style.position = Position::Absolute;
        style.width = Length::px(10.0);
        style.height = Length::px(60.0);
    });
    doc.append_child(second, second_abspos);

    let third = add_item(&mut doc);
    doc.update_resolved_style(third, |style| style.margin_top = Length::px(10.0));
    add_contained_child(&mut doc, third, 80);
    add_contained_child(&mut doc, third, 40);

    for height in [100, 60] {
        let item = add_item(&mut doc);
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
    }

    let sixth = add_item(&mut doc);
    doc.update_resolved_style(sixth, |style| {
        style.position = Position::Relative;
        style.margin_top = Length::px(10.0);
    });
    let leading_abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(leading_abspos, |style| {
        style.display = Display::Block;
        style.position = Position::Absolute;
        style.top = Length::px(-10.0);
        style.width = Length::px(10.0);
        style.height = Length::px(10.0);
    });
    doc.append_child(sixth, leading_abspos);
    add_contained_child(&mut doc, sixth, 30);
    add_contained_child(&mut doc, sixth, 80);
    let trailing_abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(trailing_abspos, |style| {
        style.display = Display::Block;
        style.position = Position::Absolute;
        style.width = Length::px(10.0);
        style.height = Length::px(20.0);
    });
    doc.append_child(sixth, trailing_abspos);

    let seventh = add_item(&mut doc);
    add_contained_child(&mut doc, seventh, 40);
    add_contained_child(&mut doc, seventh, 70);

    let final_item = add_item(&mut doc);
    doc.update_resolved_style(final_item, |style| style.position = Position::Relative);
    add_contained_child(&mut doc, final_item, 30);
    add_contained_child(&mut doc, final_item, 80);
    add_contained_child(&mut doc, final_item, 40);
    let final_abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(final_abspos, |style| {
        style.display = Display::Block;
        style.position = Position::Absolute;
        style.left = Length::px(-10.0);
        style.bottom = Length::px(0.0);
        style.width = Length::px(20.0);
        style.height = Length::px(40.0);
    });
    doc.append_child(final_item, final_abspos);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    assert_eq!(columns(&fragment).len(), 5);
    assert_eq!(find_nodes(&fragment, final_item).len(), 5);
    let positioned = find_nodes(&fragment, final_abspos);
    assert_eq!(positioned.len(), 1);
    assert_eq!(positioned[0].offset.left, lu(100));
    assert_eq!(positioned[0].offset.top, LayoutUnit::zero());
    assert_eq!(positioned[0].size.width, lu(20));
    assert_eq!(positioned[0].size.height, lu(40));
    let positioned_data = positioned[0]
        .positioned_fragmentation
        .expect("promoted positioned metadata");
    assert_eq!(positioned_data.fragmentainer_index, Some(5));
    assert_eq!(positioned_data.containing_block_size.height, lu(170));
}

#[test]
fn column_flex_edge_avoid_keeps_following_block_with_the_flexbox() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::Block);
    doc.update_resolved_style(before, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, before);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.flex_wrap = FlexWrap::Wrap);
    doc.update_resolved_style(flex, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, flex);
    for (index, height) in [25, 25, 50].into_iter().enumerate() {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if index == 1 {
            doc.update_resolved_style(item, |style| style.break_after = BreakValue::Avoid);
        }
        doc.append_child(flex, item);
    }

    let following = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(following, |style| style.display = Display::Block);
    doc.update_resolved_style(following, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, following);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(find_node(column_fragments[0], flex).is_none());
    assert_eq!(
        find_node(column_fragments[1], flex).unwrap().offset.top,
        lu(0)
    );
    assert_eq!(
        find_node(column_fragments[1], following)
            .unwrap()
            .offset
            .top,
        lu(50)
    );
}

#[test]
fn column_flex_edge_forced_break_advances_following_block() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.flex_wrap = FlexWrap::Wrap);
    doc.update_resolved_style(flex, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, flex);
    for (index, height) in [25, 25, 50].into_iter().enumerate() {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if index == 1 {
            doc.update_resolved_style(item, |style| style.break_after = BreakValue::Column);
        }
        if index == 2 {
            doc.update_resolved_style(item, |style| style.break_after = BreakValue::Avoid);
        }
        doc.append_child(flex, item);
    }
    let mut following = Vec::new();
    for _ in 0..2 {
        let block = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(block, |style| style.display = Display::Block);
        doc.update_resolved_style(block, |style| style.height = Length::px(50.0));
        doc.append_child(multicol, block);
        following.push(block);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert_eq!(
        find_node(column_fragments[1], following[0])
            .unwrap()
            .offset
            .top,
        lu(0)
    );
    assert_eq!(
        find_node(column_fragments[1], following[1])
            .unwrap()
            .offset
            .top,
        lu(50)
    );
}

#[test]
fn wrapped_column_flex_breakpoints_bound_auto_column_balancing() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.flex_wrap = FlexWrap::Wrap);
    doc.update_resolved_style(flex, |style| style.height = Length::px(200.0));
    doc.append_child(multicol, flex);
    for (index, height) in [25, 25, 50, 25, 75, 25, 25, 50, 50, 50]
        .into_iter()
        .enumerate()
    {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        if index == 3 || index == 8 {
            doc.update_resolved_style(item, |style| style.break_before = BreakValue::Avoid);
        }
        if index == 8 {
            doc.update_resolved_style(item, |style| style.break_inside = BreakInside::Avoid);
        }
        doc.append_child(flex, item);
    }
    let following = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(following, |style| style.display = Display::Block);
    doc.update_resolved_style(following, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, following);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(100), false),
    );
    assert_eq!(fragment.size.height, lu(100));
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert!(column_fragments
        .iter()
        .all(|column| column.size.height == lu(100)));
}

#[test]
fn negative_flex_margin_preserves_forced_fragment_assignment() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(5));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.append_child(multicol, flex);

    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.update_resolved_style(first, |style| style.width = Length::px(20.0));
    doc.update_resolved_style(first, |style| style.height = Length::px(150.0));
    doc.append_child(flex, first);
    let forced = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(forced, |style| style.display = Display::Block);
    doc.update_resolved_style(forced, |style| style.width = Length::px(100.0));
    doc.update_resolved_style(forced, |style| style.height = Length::px(100.0));
    doc.update_resolved_style(forced, |style| style.margin_top = Length::px(-150.0));
    doc.update_resolved_style(forced, |style| style.break_before = BreakValue::Column);
    doc.append_child(flex, forced);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    let forced_fragment = find_node(column_fragments[2], forced).expect("forced flex item");
    assert_eq!(forced_fragment.offset.top, lu(-150));
}

#[test]
fn avoid_break_propagates_through_zero_height_flex_wrapper() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.height = Length::percent(100.0));
    doc.append_child(wrapper, flex);

    let clipped = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(clipped, |style| style.display = Display::Block);
    doc.update_resolved_style(clipped, |style| style.overflow_x = Overflow::Clip);
    doc.update_resolved_style(clipped, |style| style.overflow_y = Overflow::Clip);
    doc.append_child(flex, clipped);

    for (height, avoid) in [(10, false), (100, true)] {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| style.display = Display::Block);
        doc.update_resolved_style(child, |style| style.height = Length::px(height as f32));
        if avoid {
            doc.update_resolved_style(child, |style| style.break_inside = BreakInside::Avoid);
        }
        doc.append_child(clipped, child);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let continuation = find_node(column_fragments[1], flex).expect("flex continuation");
    assert_eq!(continuation.offset.top, lu(-10));
}

#[test]
fn auto_height_column_flex_uses_outer_fragmentation_instead_of_wrapping() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(5));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.flex_wrap = FlexWrap::Wrap);
    doc.update_resolved_style(flex, |style| style.width = Length::px(20.0));
    doc.append_child(multicol, flex);
    for height in [50, 150, 300] {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(10.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(height as f32));
        doc.append_child(flex, item);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    assert_eq!(columns(&fragment).len(), 5);
}

#[test]
fn vertical_column_flex_keeps_items_after_fragmented_nested_multicol() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalLr;
        style.width = Length::px(100.0);
        style.height = Length::px(100.0);
        style.column_count = Some(2);
        style.column_gap = Some(Length::px(0.0));
        style.column_fill = ColumnFill::Auto;
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.display = Display::Flex;
        style.writing_mode = WritingMode::VerticalLr;
        style.flex_direction = FlexDirection::Column;
    });
    doc.append_child(multicol, flex);

    let nested_multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(nested_multicol, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalLr;
        style.height = Length::px(50.0);
        style.column_count = Some(2);
        style.column_gap = Some(Length::px(0.0));
        style.column_fill = ColumnFill::Auto;
    });
    doc.append_child(flex, nested_multicol);

    let nested_content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(nested_content, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalLr;
        style.width = Length::px(160.0);
        style.height = Length::px(50.0);
    });
    doc.append_child(nested_multicol, nested_content);

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalLr;
        style.width = Length::px(30.0);
        style.column_span = ColumnSpan::All;
        style.break_inside = BreakInside::Avoid;
    });
    doc.append_child(nested_multicol, spanner);

    let middle = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(middle, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalLr;
        style.width = Length::px(50.0);
    });
    doc.append_child(flex, middle);

    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| {
        style.display = Display::Block;
        style.writing_mode = WritingMode::VerticalLr;
        style.width = Length::px(20.0);
    });
    doc.append_child(flex, tail);

    let fragment = block_layout(
        &doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    );
    let outer = find_node(&fragment, multicol).expect("outer multicol");
    let column_fragments = columns(outer);
    assert_eq!(column_fragments.len(), 2);
    let first_nested = find_node(column_fragments[0], nested_multicol).expect("nested multicol");
    assert_eq!(first_nested.size.width, lu(130));
    assert_eq!(
        find_node(column_fragments[0], spanner)
            .expect("avoided spanner")
            .offset
            .left,
        lu(100)
    );
    assert_eq!(
        find_node(column_fragments[1], nested_multicol)
            .expect("nested continuation")
            .offset
            .left,
        lu(-100)
    );
    assert_eq!(
        find_node(column_fragments[1], middle)
            .expect("middle flex item")
            .offset
            .left,
        lu(30)
    );
    assert_eq!(
        find_node(column_fragments[1], tail)
            .expect("tail flex item")
            .offset
            .left,
        lu(80)
    );
    assert_eq!(count_node(&fragment, tail), 2);
}

#[test]
fn fragmented_overflow_clip_retains_its_authored_reference_box() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.display = Display::Block;
        style.width = Length::px(200.0);
        style.height = Length::px(50.0);
        style.column_count = Some(2);
        style.column_fill = ColumnFill::Balance;
    });
    doc.append_child(doc.root(), multicol);

    let clipped = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(clipped, |style| {
        style.display = Display::Block;
        style.height = Length::px(50.0);
        style.padding_top = Length::px(5.0);
        style.padding_right = Length::px(5.0);
        style.padding_bottom = Length::px(5.0);
        style.padding_left = Length::px(5.0);
        style.border_top_width = 5;
        style.border_right_width = 5;
        style.border_bottom_width = 5;
        style.border_left_width = 5;
        style.border_top_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.border_bottom_style = BorderStyle::Solid;
        style.border_left_style = BorderStyle::Solid;
        style.overflow_x = Overflow::Clip;
        style.overflow_y = Overflow::Clip;
        style.overflow_clip_box = OverflowClipBox::BorderBox;
    });
    doc.append_child(multicol, clipped);

    for _ in 0..2 {
        let content = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(content, |style| {
            style.display = Display::Block;
            style.position = Position::Relative;
            style.top = Length::px(-20.0);
            style.left = Length::px(-20.0);
            style.width = Length::px(100.0);
            style.height = Length::px(50.0);
        });
        doc.append_child(clipped, content);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(200), lu(600), lu(200), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    for (index, column) in column_fragments.iter().enumerate() {
        let continuation = find_node(column, clipped).expect("clipped continuation");
        assert_eq!(column.offset.left, lu(index as i32 * 108));
        assert_eq!(column.size.width, lu(92));
        assert_eq!(column.size.height, lu(35));
        assert_eq!(continuation.offset.left, lu(0));
        assert_eq!(continuation.offset.top, lu(0));
        assert_eq!(continuation.size.width, lu(92));
        assert_eq!(continuation.size.height, lu(35));
        assert_eq!(continuation.border.top, lu(5));
        assert_eq!(continuation.border.right, lu(5));
        assert_eq!(continuation.border.bottom, lu(5));
        assert_eq!(continuation.border.left, lu(5));
        assert_eq!(continuation.padding.top, lu(5));
        assert_eq!(continuation.padding.right, lu(5));
        assert_eq!(continuation.padding.bottom, lu(5));
        assert_eq!(continuation.padding.left, lu(5));
        let slice = continuation
            .decoration_slice
            .expect("fragmented clip source slice");
        assert_eq!(slice.source_block_offset, lu(index as i32 * 35));
        assert_eq!(slice.source_block_size, lu(70));
        assert_eq!(continuation.is_first_for_node, index == 0);
        assert_eq!(continuation.is_last_for_node, index == 1);
        assert!(continuation.has_overflow_clip);
    }
}

#[test]
fn column_flex_gap_is_truncated_at_fragmentainer_edges() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(90.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.flex_wrap = FlexWrap::Wrap);
    doc.update_resolved_style(flex, |style| style.row_gap = Some(Length::px(100.0)));
    doc.append_child(multicol, flex);
    let mut items = Vec::new();
    for _ in 0..3 {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.height = Length::px(100.0));
        doc.append_child(flex, item);
        items.push(item);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(90), lu(600), lu(90), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    for (column, item) in column_fragments.iter().zip(items) {
        assert_eq!(
            find_node(column, item)
                .expect("item continuation")
                .offset
                .top,
            lu(0)
        );
    }
}

#[test]
fn class_a_avoid_moves_an_overflowing_flex_item_pair_together() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(150.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.update_resolved_style(flex, |style| style.flex_direction = FlexDirection::Column);
    doc.update_resolved_style(flex, |style| style.flex_wrap = FlexWrap::Wrap);
    doc.update_resolved_style(flex, |style| style.height = Length::px(250.0));
    doc.append_child(multicol, flex);

    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.update_resolved_style(first, |style| style.height = Length::px(100.0));
    doc.append_child(flex, first);
    let previous = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(previous, |style| style.display = Display::Block);
    doc.update_resolved_style(previous, |style| style.height = Length::px(25.0));
    doc.append_child(flex, previous);
    let current = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(current, |style| style.display = Display::Block);
    doc.update_resolved_style(current, |style| style.height = Length::px(25.0));
    doc.update_resolved_style(current, |style| style.break_before = BreakValue::Avoid);
    doc.append_child(flex, current);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| style.height = Length::px(75.0));
    doc.append_child(current, overflow);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let second_flex = find_node(column_fragments[1], flex).expect("flex continuation");
    let previous_fragment = second_flex
        .children
        .iter()
        .find(|child| child.node_id == previous && child.offset.top == lu(0))
        .expect("previous item moved with avoided edge");
    assert_eq!(previous_fragment.size.height, lu(25));
    let current_fragment = second_flex
        .children
        .iter()
        .find(|child| child.node_id == current && child.offset.top == lu(25))
        .expect("current item after avoided edge");
    assert_eq!(current_fragment.size.height, lu(25));
}

#[test]
fn column_reverse_line_closes_at_the_resolved_main_end() {
    let mut doc = Document::new();
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::ColumnReverse);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::WrapReverse);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(200.0));
    });
    doc.append_child(doc.root(), flex);
    for _ in 0..8 {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.update_resolved_style(item, |style| style.width = Length::px(25.0));
        doc.update_resolved_style(item, |style| style.height = Length::px(50.0));
        doc.append_child(flex, item);
    }

    let fragment = block_layout(
        &doc,
        flex,
        &ConstraintSpace::for_block_child(lu(50), lu(200), lu(50), lu(200), false),
    );
    let first_top = fragment
        .children
        .iter()
        .map(|child| child.offset.top)
        .min()
        .unwrap();
    let final_bottom = fragment
        .children
        .iter()
        .map(|child| child.offset.top + child.size.height)
        .max()
        .unwrap();
    assert_eq!(first_top, lu(0));
    assert_eq!(final_bottom, lu(200));
}

#[test]
fn static_multicol_defers_abspos_to_the_parent_containing_block() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| style.display = Display::Block);
    doc.update_resolved_style(abspos, |style| style.position = Position::Absolute);
    doc.update_resolved_style(abspos, |style| style.left = Length::px(2.0));
    doc.update_resolved_style(abspos, |style| style.top = Length::px(3.0));
    doc.update_resolved_style(abspos, |style| style.width = Length::px(10.0));
    doc.update_resolved_style(abspos, |style| style.height = Length::px(10.0));
    doc.append_child(multicol, abspos);

    let fragment = block_layout(
        &doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    );
    let multicol_fragment = fragment
        .children
        .iter()
        .find(|child| child.node_id == multicol)
        .expect("multicol fragment");
    assert!(!contains_node(multicol_fragment, abspos));
    let abspos_fragment = fragment
        .children
        .iter()
        .find(|child| child.node_id == abspos)
        .expect("abspos in parent containing block");
    assert_eq!(abspos_fragment.offset.left, lu(2));
    assert_eq!(abspos_fragment.offset.top, lu(3));
}

#[test]
fn block_level_abspos_in_inline_uses_hypothetical_block_inline_edge() {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.update_resolved_style(block, |style| style.width = Length::px(100.0));
    doc.append_child(doc.root(), block);

    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
    doc.update_resolved_style(inline, |style| style.position = Position::Relative);
    doc.append_child(block, inline);

    let text = doc.create_node(ElementTag::Text);
    doc.update_resolved_style(text, |style| style.font_size = 10.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Number(1.0));
    doc.node_mut(text).text = Some("AA".to_string());
    doc.append_child(inline, text);

    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.width = Length::px(10.0));
        style.update_derived(|computed| computed.height = Length::px(10.0));
    });
    doc.append_child(inline, abspos);

    let fragment = block_layout(
        &doc,
        block,
        &ConstraintSpace::for_block_child(lu(100), lu(100), lu(100), lu(100), false),
    );
    let positioned = find_node(&fragment, abspos).expect("positioned block fragment");
    assert_eq!(positioned.offset.left, LayoutUnit::zero());
}

#[test]
fn inside_list_marker_precedes_block_only_content_in_normal_flow() {
    let mut doc = Document::new();
    let list_item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(list_item, |style| {
        style.update_derived(|computed| computed.display = Display::ListItem);
        style.update_derived(|computed| computed.list_style_type = ListStyleType::Disc);
        style.update_derived(|computed| computed.list_style_position = ListStylePosition::Inside);
        style.update_derived(|computed| computed.font_size = 16.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Normal);
    });
    doc.append_child(doc.root(), list_item);
    let block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.append_child(list_item, block);

    let fragment = block_layout(
        &doc,
        list_item,
        &ConstraintSpace::for_block_child(lu(300), lu(100), lu(300), lu(100), false),
    );
    let child = find_node(&fragment, block).expect("block after inside marker");
    assert_eq!(child.offset.top, lu(19));
    assert_eq!(fragment.size.height, lu(19));
}

#[test]
fn direct_block_in_inline_fragments_visible_in_flow_overflow_but_not_logical_height() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(4));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
    doc.append_child(multicol, inline);
    let before = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(before, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(before, |style| style.vertical_align = VerticalAlign::Top);
    doc.update_resolved_style(before, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(before, |style| style.height = Length::px(50.0));
    doc.append_child(inline, before);
    let block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.update_resolved_style(block, |style| style.height = Length::px(100.0));
    doc.update_resolved_style(block, |style| style.overflow_x = Overflow::Visible);
    doc.update_resolved_style(block, |style| style.overflow_y = Overflow::Visible);
    doc.append_child(inline, block);
    let transparent_prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(transparent_prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(transparent_prefix, |style| style.height = Length::px(150.0));
    doc.append_child(block, transparent_prefix);
    let overflow_tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow_tail, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow_tail, |style| style.height = Length::px(200.0));
    doc.append_child(block, overflow_tail);
    let after = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(after, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(after, |style| style.vertical_align = VerticalAlign::Top);
    doc.update_resolved_style(after, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(after, |style| style.height = Length::px(50.0));
    doc.append_child(inline, after);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 4);
    assert!(column_fragments
        .iter()
        .all(|column| contains_node(column, block)));
    let after_fragment = find_node(column_fragments[1], after)
        .expect("following inline run resumes at the authored block end");
    assert_eq!(after_fragment.offset.top, lu(0));
    let after_line = find_node(column_fragments[1], inline)
        .expect("anonymous continuation is owned by the inline parent");
    assert_eq!(after_line.offset.top, lu(50));
}

#[test]
fn forced_descendant_resumes_from_its_selected_block_break_coordinate() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
    doc.append_child(multicol, inline);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.update_resolved_style(wrapper, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(wrapper, |style| style.overflow_x = Overflow::Visible);
    doc.update_resolved_style(wrapper, |style| style.overflow_y = Overflow::Visible);
    doc.append_child(inline, wrapper);
    let line = doc.create_node(ElementTag::Break);
    doc.update_resolved_style(line, |style| style.display = Display::Inline);
    doc.append_child(wrapper, line);
    let forced = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(forced, |style| style.display = Display::Block);
    doc.update_resolved_style(forced, |style| style.break_before = BreakValue::Column);
    doc.update_resolved_style(forced, |style| style.height = Length::px(100.0));
    doc.append_child(wrapper, forced);
    let after = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(after, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(after, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(after, |style| style.height = Length::px(50.0));
    doc.append_child(inline, after);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    let forced_fragment = find_node(column_fragments[1], forced)
        .expect("forced descendant continuation in the second column");
    assert_eq!(forced_fragment.offset.top, lu(0));
}

#[test]
fn nested_relative_continuations_slice_before_visual_translation() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| style.display = Display::Block);
    doc.update_resolved_style(outer, |style| style.position = Position::Relative);
    doc.update_resolved_style(outer, |style| style.height = Length::px(200.0));
    doc.update_resolved_style(outer, |style| style.top = Length::px(50.0));
    doc.append_child(multicol, outer);
    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| style.display = Display::Block);
    doc.update_resolved_style(inner, |style| style.position = Position::Relative);
    doc.update_resolved_style(inner, |style| style.height = Length::px(200.0));
    doc.update_resolved_style(inner, |style| style.top = Length::px(100.0));
    doc.append_child(outer, inner);
    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| style.display = Display::Block);
    doc.update_resolved_style(abspos, |style| style.position = Position::Absolute);
    doc.update_resolved_style(abspos, |style| style.height = Length::px(200.0));
    doc.append_child(inner, abspos);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    for column in column_fragments {
        assert!(column.has_overflow_clip);
        let outer_fragment = find_node(column, outer).expect("translated outer continuation");
        assert_eq!(outer_fragment.size.height, lu(100));
        assert!(outer_fragment.has_overflow_clip);
        assert!(find_node(column, inner).is_some());
    }
}

#[test]
fn multicol_auto_width_subtracts_non_auto_inline_margins() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(150.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| style.display = Display::Block);
    doc.update_resolved_style(outer, |style| style.height = Length::px(0.0));
    doc.update_resolved_style(outer, |style| style.margin_right = Length::px(25.0));
    doc.append_child(multicol, outer);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| style.height = Length::px(100.0));
    doc.append_child(outer, overflow);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(150), lu(600), lu(150), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    for column in column_fragments {
        assert_eq!(find_node(column, outer).unwrap().size.width, lu(50));
    }
}

#[test]
fn zero_height_fragmentainer_keeps_progress_separate_from_column_clip() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(300.0));
        style.update_derived(|computed| computed.height = Length::px(0.0));
        style.update_derived(|computed| computed.column_width = Some(Length::px(100.0)));
    });
    doc.append_child(doc.root(), multicol);
    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| style.display = Display::Block);
    doc.update_resolved_style(child, |style| style.width = Length::px(100.0));
    doc.update_resolved_style(child, |style| style.height = Length::px(100.0));
    doc.update_resolved_style(child, |style| style.outline_style = BorderStyle::Solid);
    doc.update_resolved_style(child, |style| style.outline_width = 3);
    doc.append_child(multicol, child);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(300), lu(600), lu(300), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 100);
    assert_eq!(column_fragments[0].size.height, lu(0));
    assert!(!column_fragments[0].has_overflow_clip);
    let progress_slice = find_node(column_fragments[0], child).unwrap();
    assert_eq!(progress_slice.size.height, lu(1));
    assert!(!progress_slice.has_overflow_clip);
}

#[test]
fn monolithic_floats_move_to_fresh_column_and_keep_side_placement() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(150.0));
        style.update_derived(|computed| computed.height = Length::px(120.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(prefix, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, prefix);
    let right = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(right, |style| style.display = Display::Block);
    doc.update_resolved_style(right, |style| style.float = Float::Right);
    doc.update_resolved_style(right, |style| style.width = Length::px(10.0));
    doc.update_resolved_style(right, |style| style.height = Length::px(100.0));
    doc.append_child(multicol, right);
    let left = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(left, |style| style.display = Display::Block);
    doc.update_resolved_style(left, |style| style.float = Float::Left);
    doc.update_resolved_style(left, |style| style.width = Length::px(40.0));
    doc.update_resolved_style(left, |style| style.height = Length::px(60.0));
    doc.append_child(multicol, left);
    let cleared = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(cleared, |style| style.display = Display::Block);
    doc.update_resolved_style(cleared, |style| style.clear = Clear::Left);
    doc.update_resolved_style(cleared, |style| style.height = Length::px(40.0));
    doc.append_child(multicol, cleared);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(150), lu(600), lu(150), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert_eq!(
        find_node(column_fragments[1], right).unwrap().offset.left,
        lu(40)
    );
    assert_eq!(
        find_node(column_fragments[1], left).unwrap().offset.left,
        lu(0)
    );
    assert_eq!(
        find_node(column_fragments[1], cleared).unwrap().offset.top,
        lu(60)
    );
}

#[test]
fn empty_pre_spanner_oof_uses_definite_fragmentainer_capacity() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let relative = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(relative, |style| style.display = Display::Block);
    doc.update_resolved_style(relative, |style| style.position = Position::Relative);
    doc.update_resolved_style(relative, |style| style.height = Length::px(0.0));
    doc.append_child(multicol, relative);
    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| style.display = Display::Block);
    doc.update_resolved_style(abspos, |style| style.position = Position::Absolute);
    doc.update_resolved_style(abspos, |style| style.top = Length::px(200.0));
    doc.update_resolved_style(abspos, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(abspos, |style| style.height = Length::px(200.0));
    doc.append_child(relative, abspos);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.height = Length::px(0.0));
    doc.append_child(multicol, spanner);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 4);
    assert!(column_fragments
        .iter()
        .skip(1)
        .all(|column| column.size.height == lu(100)));
    assert_eq!(find_node(&fragment, spanner).unwrap().offset.top, lu(0));
}

#[test]
fn indefinite_auto_fill_uses_forced_descendant_segments_for_column_height() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(5));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(10.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    for (index, height) in [10.0, 10.0, 10.0, 10.0, 100.0].into_iter().enumerate() {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| style.display = Display::Block);
        doc.update_resolved_style(child, |style| style.height = Length::px(height));
        if index > 0 {
            doc.update_resolved_style(child, |style| style.break_before = BreakValue::Column);
        }
        doc.append_child(wrapper, child);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(100), lu(100), lu(100), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 5);
    assert!(column_fragments
        .iter()
        .all(|column| column.size.height == lu(100)));
    assert!(column_fragments
        .iter()
        .all(|column| contains_node(column, wrapper)));
}

#[test]
fn definite_overflow_continuation_is_parallel_to_following_sibling_flow() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(40.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
    });
    doc.append_child(doc.root(), multicol);
    let prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(prefix, |style| style.height = Length::px(40.0));
    doc.append_child(multicol, prefix);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| style.height = Length::px(10.0));
    doc.update_resolved_style(overflow, |style| style.overflow_x = Overflow::Visible);
    doc.update_resolved_style(overflow, |style| style.overflow_y = Overflow::Visible);
    doc.append_child(multicol, overflow);
    let tall = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tall, |style| style.display = Display::Block);
    doc.update_resolved_style(tall, |style| style.height = Length::px(180.0));
    doc.append_child(overflow, tall);
    let forced = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(forced, |style| style.display = Display::Block);
    doc.update_resolved_style(forced, |style| style.height = Length::px(10.0));
    doc.update_resolved_style(forced, |style| style.break_before = BreakValue::Column);
    doc.append_child(overflow, forced);
    let following = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(following, |style| style.display = Display::Block);
    doc.update_resolved_style(following, |style| style.width = Length::percent(50.0));
    doc.update_resolved_style(following, |style| style.height = Length::px(250.0));
    doc.append_child(multicol, following);

    let mut space = ConstraintSpace::for_block_child(lu(40), lu(100), lu(40), lu(100), false);
    space.fragmentainer_block_size = lu(100);
    let fragment = block_layout(&doc, multicol, &space);
    let column_fragments = columns(&fragment);
    assert!(column_fragments.len() >= 3);
    let first_following = find_node(column_fragments[0], following)
        .expect("following flow starts beside the overflow continuation");
    assert_eq!(first_following.offset.top, lu(50));
    assert!(find_node(column_fragments[1], overflow).is_some());
    assert!(find_node(column_fragments[1], following).is_some());
}

#[test]
fn mixed_inline_runs_fragment_at_line_boundaries_after_a_block() {
    let mut doc =
        Document::new_with_font_collection(openui_text::FontCollection::deterministic_test());
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(220.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(20.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
        style.update_derived(|computed| computed.font_family = FontFamilyList::single("Ahem"));
        style.update_derived(|computed| computed.font_size = 20.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Length(20.0));
        style.update_derived(|computed| computed.orphans = 1);
        style.update_derived(|computed| computed.widows = 1);
    });
    doc.append_child(doc.root(), multicol);

    let append_four_line_run = |doc: &mut Document, parent: NodeId| {
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("ab cd ef gh ".to_string());
        doc.update_resolved_style(text, |style| {
            style.font_family = FontFamilyList::single("Ahem")
        });
        doc.update_resolved_style(text, |style| style.font_size = 20.0);
        doc.update_resolved_style(text, |style| style.line_height = LineHeight::Length(20.0));
        doc.append_child(parent, text);
        text
    };

    let first = append_four_line_run(&mut doc, multicol);
    let heading = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(heading, |style| style.display = Display::Block);
    doc.update_resolved_style(heading, |style| {
        style.font_family = FontFamilyList::single("Ahem")
    });
    doc.update_resolved_style(heading, |style| style.font_size = 20.0);
    doc.update_resolved_style(heading, |style| {
        style.line_height = LineHeight::Length(20.0)
    });
    doc.append_child(multicol, heading);
    let heading_text = doc.create_node(ElementTag::Text);
    doc.node_mut(heading_text).text = Some("1234".to_string());
    doc.update_resolved_style(heading_text, |style| {
        style.font_family = FontFamilyList::single("Ahem")
    });
    doc.update_resolved_style(heading_text, |style| style.font_size = 20.0);
    doc.update_resolved_style(heading_text, |style| {
        style.line_height = LineHeight::Length(20.0)
    });
    doc.append_child(heading, heading_text);
    let second = append_four_line_run(&mut doc, multicol);
    let third = append_four_line_run(&mut doc, multicol);
    let fourth = append_four_line_run(&mut doc, multicol);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(220), lu(600), lu(220), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 3);
    assert!(column_fragments
        .iter()
        .all(|column| column.size.height == lu(120)));
    assert!(contains_node(column_fragments[0], first));
    assert!(contains_node(column_fragments[0], second));
    assert!(contains_node(column_fragments[1], second));
    assert!(contains_node(column_fragments[1], third));
    assert!(contains_node(column_fragments[2], third));
    assert!(contains_node(column_fragments[2], fourth));
}

#[test]
fn leading_descendant_clear_resumes_after_a_parallel_float() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(prefix, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, prefix);
    let parallel_float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(parallel_float, |style| style.display = Display::Block);
    doc.update_resolved_style(parallel_float, |style| style.float = Float::Left);
    doc.update_resolved_style(parallel_float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(parallel_float, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, parallel_float);

    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    let cleared = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(cleared, |style| style.display = Display::Block);
    doc.update_resolved_style(cleared, |style| style.clear = Clear::Both);
    doc.update_resolved_style(cleared, |style| style.height = Length::px(10.0));
    doc.append_child(wrapper, cleared);
    let nested_float = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(nested_float, |style| style.display = Display::Block);
    doc.update_resolved_style(nested_float, |style| style.float = Float::Left);
    doc.update_resolved_style(nested_float, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(nested_float, |style| style.height = Length::px(100.0));
    doc.append_child(cleared, nested_float);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 2);
    assert_eq!(
        find_node(column_fragments[1], cleared)
            .expect("descendant clear continuation")
            .offset
            .top,
        LayoutUnit::zero()
    );
    assert!(contains_node(column_fragments[1], nested_float));
}

#[test]
fn block_in_inline_descendant_spanner_is_extracted_once() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.orphans = 1);
        style.update_derived(|computed| computed.widows = 1);
    });
    doc.append_child(doc.root(), multicol);
    let inline = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
    doc.append_child(multicol, inline);
    let block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block, |style| style.display = Display::Block);
    doc.update_resolved_style(block, |style| style.height = Length::px(30.0));
    doc.append_child(inline, block);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| style.height = Length::px(200.0));
    doc.append_child(block, overflow);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.margin_top = Length::px(-20.0));
    doc.update_resolved_style(spanner, |style| style.height = Length::px(20.0));
    doc.append_child(block, spanner);
    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| style.display = Display::InlineBlock);
    doc.update_resolved_style(tail, |style| style.vertical_align = VerticalAlign::Top);
    doc.update_resolved_style(tail, |style| style.width = Length::percent(100.0));
    doc.update_resolved_style(tail, |style| style.height = Length::px(50.0));
    doc.append_child(multicol, tail);

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    assert_eq!(count_node(&fragment, spanner), 1);
    assert!(fragment
        .children
        .iter()
        .any(|child| child.node_id == spanner));
    assert!(fragment
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .any(|column| contains_node(column, tail)));
}

#[test]
fn nested_post_spanner_row_resumes_as_sequential_block_flow() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(110.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.line_height = LineHeight::Length(50.0));
    });
    doc.append_child(doc.root(), outer);
    let prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(prefix, |style| style.height = Length::px(60.0));
    doc.append_child(outer, prefix);
    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(outer, inner);
    let inner_prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner_prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(inner_prefix, |style| style.height = Length::px(40.0));
    doc.append_child(inner, inner_prefix);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.update_resolved_style(wrapper, |style| style.height = Length::px(100.0));
    doc.append_child(inner, wrapper);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.height = Length::px(20.0));
    doc.append_child(wrapper, spanner);
    let line = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(line, |style| style.display = Display::Block);
    doc.update_resolved_style(line, |style| style.width = Length::percent(200.0));
    doc.update_resolved_style(line, |style| style.height = Length::px(50.0));
    doc.append_child(wrapper, line);
    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| style.display = Display::Block);
    doc.update_resolved_style(tail, |style| style.width = Length::percent(200.0));
    doc.update_resolved_style(tail, |style| style.height = Length::px(50.0));
    doc.append_child(wrapper, tail);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(100), lu(100), lu(100), lu(100), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 2);
    let continuation = find_node(outer_columns[1], inner).expect("nested continuation");
    assert_eq!(continuation.size.height, lu(100));
    let inner_columns: Vec<_> = continuation
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .collect();
    assert_eq!(inner_columns.len(), 1);
    assert_eq!(inner_columns[0].size.height, lu(100));
    assert_eq!(find_node(inner_columns[0], line).unwrap().offset.top, lu(0));
    assert_eq!(
        find_node(inner_columns[0], tail).unwrap().offset.top,
        lu(50)
    );
}

#[test]
fn definite_nested_multicol_preserves_in_flow_overflow_after_principal_box_end() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(200.0));
        style.update_derived(|computed| computed.height = Length::px(300.0));
        style.update_derived(|computed| computed.column_count = Some(1));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.column_rule_width = 6);
        style.update_derived(|computed| computed.column_rule_style = BorderStyle::Solid);
    });
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(500.0));
        style.update_derived(|computed| computed.column_count = Some(1));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.column_rule_width = 3);
        style.update_derived(|computed| computed.column_rule_style = BorderStyle::Solid);
    });
    doc.append_child(outer, inner);

    let overflowing = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflowing, |style| style.display = Display::Block);
    doc.update_resolved_style(overflowing, |style| style.height = Length::px(600.0));
    doc.append_child(inner, overflowing);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(200), lu(600), lu(200), lu(600), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 3);
    assert!(outer_columns
        .iter()
        .all(|column| contains_node(column, overflowing)));
    assert!(!contains_node(outer_columns[2], inner));
    assert_eq!(
        find_node(outer_columns[2], overflowing).unwrap().offset.top,
        lu(0)
    );
    assert_eq!(
        find_node(outer_columns[2], overflowing)
            .unwrap()
            .size
            .height,
        lu(100)
    );
}

#[test]
fn definite_nested_multicol_resumes_complete_inner_column_rows() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(400.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(4));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), outer);
    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(300.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(outer, inner);
    let content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(content, |style| style.display = Display::Block);
    doc.update_resolved_style(content, |style| style.height = Length::px(340.0));
    doc.append_child(inner, content);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(400), lu(600), lu(400), lu(600), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 3);
    let continuation = find_node(outer_columns[1], inner).expect("second inner row");
    assert_eq!(columns(continuation).len(), 2);
}

#[test]
fn overwide_nested_multicol_keeps_inline_overflow_continuation_geometry() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(120.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), outer);
    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.padding_top = Length::px(10.0));
        style.update_derived(|computed| computed.padding_right = Length::px(10.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(10.0));
        style.update_derived(|computed| computed.padding_left = Length::px(10.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(16.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(outer, inner);
    let content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(content, |style| style.display = Display::Block);
    doc.update_resolved_style(content, |style| style.height = Length::px(400.0));
    doc.append_child(inner, content);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 2);
    assert!(outer_columns
        .iter()
        .any(|column| contains_node(column, content)));
    let first_inner = find_node(outer_columns[0], inner).expect("first inner fragment");
    assert!(first_inner.size.width > outer_columns[0].size.width);
}

#[test]
fn nested_spanner_visible_overflow_continues_in_the_ancestor_columns() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
    });
    doc.append_child(outer, inner);

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_span = ColumnSpan::All);
        style.update_derived(|computed| computed.height = Length::px(50.0));
    });
    doc.append_child(inner, spanner);
    for height in [50.0, 70.0] {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.height = Length::px(height));
        });
        doc.append_child(spanner, child);
    }
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(80.0));
    });
    doc.append_child(spanner, overflow);

    let second_spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(second_spanner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_span = ColumnSpan::All);
        style.update_derived(|computed| computed.height = Length::px(10.0));
    });
    doc.append_child(inner, second_spanner);
    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(120.0));
    });
    doc.append_child(inner, tail);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 2);
    let continued_inner = find_node(outer_columns[1], inner).expect("nested continuation");
    assert_eq!(
        continued_inner
            .decoration_slice
            .expect("continuation source slice")
            .source_block_offset,
        lu(100)
    );
    assert!(contains_node(continued_inner, overflow));
    fn has_inline_overflow_slice(fragment: &Fragment, node: NodeId) -> bool {
        (fragment.node_id == node && fragment.inline_axis_clip_only && fragment.skip_box_decoration)
            || fragment
                .children
                .iter()
                .any(|child| has_inline_overflow_slice(child, node))
    }
    assert!(has_inline_overflow_slice(continued_inner, spanner));
}

#[test]
fn direct_spanner_sequence_uses_source_slices_in_ancestor_columns() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(4));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
    });
    doc.append_child(outer, inner);

    let mut add_spanner = |height: f32, child_heights: &[f32]| {
        let spanner = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(spanner, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.column_span = ColumnSpan::All);
            style.update_derived(|computed| computed.height = Length::px(height));
        });
        doc.append_child(inner, spanner);
        for child_height in child_heights {
            let child = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(child, |style| {
                style.update_derived(|computed| computed.display = Display::Block);
                style.update_derived(|computed| computed.height = Length::px(*child_height));
            });
            doc.append_child(spanner, child);
        }
        spanner
    };
    let first_spanner = add_spanner(10.0, &[10.0, 360.0, 30.0]);
    let second_spanner = add_spanner(100.0, &[100.0, 240.0, 20.0]);
    let third_spanner = add_spanner(240.0, &[]);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(100), lu(600), lu(100), lu(600), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 4);

    let third_inner = find_node(outer_columns[2], inner).expect("third nested slice");
    let third_active = find_node(third_inner, third_spanner).expect("active third spanner");
    assert_eq!(third_active.offset.top, lu(0));
    assert_eq!(third_active.size.height, lu(100));
    assert_eq!(
        third_active
            .decoration_slice
            .expect("third spanner source slice")
            .source_block_offset,
        lu(90)
    );

    let fourth_inner = find_node(outer_columns[3], inner).expect("fourth nested slice");
    let fourth_active = find_node(fourth_inner, third_spanner).expect("final third spanner slice");
    assert_eq!(fourth_active.size.height, lu(50));
    assert_eq!(
        fourth_active
            .decoration_slice
            .expect("final third spanner source slice")
            .source_block_offset,
        lu(190)
    );
    for spanner in [first_spanner, second_spanner] {
        assert!(find_nodes(fourth_inner, spanner).iter().any(|slice| {
            slice.skip_box_decoration && slice.has_overflow_clip && !slice.inline_axis_clip_only
        }));
    }
}

#[test]
fn nested_spanner_rows_resolve_before_ancestor_fragmentation() {
    let mut doc = Document::new();
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(400.0));
        style.update_derived(|computed| computed.height = Length::px(110.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.column_rule_width = 6);
        style.update_derived(|computed| computed.column_rule_style = BorderStyle::Solid);
    });
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(270.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_rule_width = 3);
        style.update_derived(|computed| computed.column_rule_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_width = 10);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_width = 10);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(outer, inner);
    let prefix = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(prefix, |style| style.display = Display::Block);
    doc.update_resolved_style(prefix, |style| style.height = Length::px(200.0));
    doc.append_child(inner, prefix);
    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| style.column_span = ColumnSpan::All);
    doc.update_resolved_style(spanner, |style| style.height = Length::px(50.0));
    doc.append_child(inner, spanner);
    let tail = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(tail, |style| style.display = Display::Block);
    doc.update_resolved_style(tail, |style| style.height = Length::px(240.0));
    doc.append_child(inner, tail);

    let fragment = block_layout(
        &doc,
        outer,
        &ConstraintSpace::for_block_child(lu(400), lu(600), lu(400), lu(600), false),
    );
    let outer_columns = columns(&fragment);
    assert_eq!(outer_columns.len(), 3);
    let first = find_node(outer_columns[0], inner).expect("first inner slice");
    let first_inner_columns = columns(first);
    assert_eq!(first_inner_columns.len(), 2);
    assert!(first_inner_columns
        .iter()
        .all(|column| column.size.height == lu(100)));
    let first_rules: Vec<_> = first
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnRule)
        .collect();
    assert_eq!(first_rules.len(), 1);
    assert_eq!(first_rules[0].offset.top, lu(10));
    assert_eq!(first_rules[0].size.height, lu(100));
    assert!(contains_node(outer_columns[1], spanner));
    assert!(contains_node(outer_columns[1], tail));
    assert!(contains_node(outer_columns[2], tail));
    let second = find_node(outer_columns[1], inner).expect("second inner slice");
    let second_rules: Vec<_> = second
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnRule)
        .collect();
    assert_eq!(second_rules.len(), 1);
    assert_eq!(second_rules[0].offset.top, lu(50));
    assert_eq!(second_rules[0].size.height, lu(60));
    let third = find_node(outer_columns[2], inner).expect("third inner slice");
    let third_rules: Vec<_> = third
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnRule)
        .collect();
    assert_eq!(third_rules.len(), 1);
    assert_eq!(third_rules[0].offset.top, lu(0));
    assert_eq!(third_rules[0].size.height, lu(60));
}

#[test]
fn wrapped_authored_column_height_is_retained_before_spanners() {
    let mut doc = Document::new();
    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.column_height = Some(Length::px(5.0)));
        style.update_derived(|computed| computed.column_wrap = ColumnWrap::Wrap);
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.row_gap = Some(Length::px(5.0)));
    });
    doc.append_child(doc.root(), multicol);

    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.append_child(multicol, wrapper);
    let mut children = Vec::new();
    for (height, spans) in [
        (20.0, false),
        (5.0, true),
        (10.0, false),
        (5.0, true),
        (20.0, false),
        (12.0, true),
        (16.0, false),
    ] {
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| {
            style.display = Display::Block;
            style.height = Length::px(height);
            if spans {
                style.column_span = ColumnSpan::All;
            }
        });
        doc.append_child(wrapper, child);
        children.push(child);
    }

    let fragment = block_layout(
        &doc,
        multicol,
        &ConstraintSpace::for_block_child(lu(100), lu(100), lu(100), lu(100), false),
    );
    let column_fragments = columns(&fragment);
    assert_eq!(column_fragments.len(), 14);
    assert_eq!(
        column_fragments
            .iter()
            .map(|column| column.offset.top)
            .collect::<Vec<_>>(),
        [0, 0, 10, 10, 30, 30, 50, 50, 60, 60, 82, 82, 90, 90]
            .map(lu)
            .to_vec()
    );
    assert_eq!(
        column_fragments
            .iter()
            .map(|column| find_node(column, wrapper)
                .expect("wrapper slice")
                .size
                .height)
            .collect::<Vec<_>>(),
        [5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 3, 3, 5, 5].map(lu).to_vec()
    );
    for (spanner, expected_top) in [(children[1], 20), (children[3], 40), (children[5], 70)] {
        let fragments = find_nodes(&fragment, spanner);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].offset.top, lu(expected_top));
    }
}
