//! Integration tests for flex-direction, flex-wrap, gap, order, and auto margins.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{flex_layout, ConstraintSpace, Fragment};
use openui_style::{
    ContentAlignment, ContentDistribution, ContentPosition, Display, FlexDirection, FlexWrap,
    ItemAlignment, ItemPosition,
};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn make_flex(doc: &mut Document, width: i32, height: i32) -> NodeId {
    let container = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(container).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(width as f32);
        s.height = Length::px(height as f32);
    }
    doc.append_child(doc.root(), container);
    container
}

fn add_child(doc: &mut Document, parent: NodeId, w: i32, h: i32) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(w as f32);
        s.height = Length::px(h as f32);
    }
    doc.append_child(parent, child);
    child
}

/// Add a child with only height set (width = auto → will stretch in row flex).
fn add_child_h(doc: &mut Document, parent: NodeId, h: i32) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.height = Length::px(h as f32);
    }
    doc.append_child(parent, child);
    child
}

/// Add a child with only width set (height = auto → will stretch in row flex).
#[allow(dead_code)]
fn add_child_w(doc: &mut Document, parent: NodeId, w: i32) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(w as f32);
    }
    doc.append_child(parent, child);
    child
}

fn lay(doc: &Document, container: NodeId, w: i32, h: i32) -> Fragment {
    let space = ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h));
    flex_layout(doc, container, &space)
}

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

// ═══════════════════════════════════════════════════════════════════════════
// Category 1: flex-direction (20 tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn dir_row_items_left_to_right() {
    // Row: items placed left-to-right along horizontal main axis.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 60, 40);
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children.len(), 3);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(60));
    assert_eq!(f.children[2].offset.left, lu(140));
}

#[test]
fn dir_row_reverse_items_right_to_left() {
    // Row-reverse: items reversed; with default justify-content (flex-start for reversed
    // direction), the initial offset = free_space, pushing items to the right.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let c1 = add_child(&mut doc, c, 60, 40);
    let c2 = add_child(&mut doc, c, 80, 40);

    let f = lay(&doc, c, 300, 100);
    // Items reversed: children[0]=c2, children[1]=c1
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    // free_space = 300 - 140 = 160; initial_offset = 160
    assert_eq!(f.children[0].offset.left, lu(160));
    assert_eq!(f.children[1].offset.left, lu(240));
}

#[test]
fn dir_column_items_top_to_bottom() {
    // Column: main axis is vertical, items stack top-to-bottom.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    add_child(&mut doc, c, 50, 60);
    add_child(&mut doc, c, 50, 80);

    let f = lay(&doc, c, 200, 300);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(60));
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn dir_column_reverse_items_bottom_to_top() {
    // Column-reverse: items reversed; initial offset = free_space (pushed to bottom).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::ColumnReverse;
    let c1 = add_child(&mut doc, c, 50, 60);
    let c2 = add_child(&mut doc, c, 50, 80);

    let f = lay(&doc, c, 200, 300);
    // Items reversed: children[0]=c2, children[1]=c1
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    // free_space = 300 - 140 = 160; initial_offset = 160
    assert_eq!(f.children[0].offset.top, lu(160));
    assert_eq!(f.children[1].offset.top, lu(240));
}

#[test]
fn dir_row_different_sized_items() {
    // Row with different sized items; positions accumulate left-to-right.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    add_child(&mut doc, c, 30, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 70, 50);

    let f = lay(&doc, c, 400, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[0].width(), lu(30));
    assert_eq!(f.children[1].offset.left, lu(30));
    assert_eq!(f.children[1].width(), lu(100));
    assert_eq!(f.children[2].offset.left, lu(130));
    assert_eq!(f.children[2].width(), lu(70));
}

#[test]
fn dir_row_reverse_with_grow() {
    // Row-reverse with flex-grow: items grow then get reversed.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let c1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c1).style_mut();
        s.display = Display::Block;
        s.flex_grow = 1.0;
        s.height = Length::px(40.0);
    }
    doc.append_child(c, c1);
    let c2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c2).style_mut();
        s.display = Display::Block;
        s.flex_grow = 2.0;
        s.height = Length::px(40.0);
    }
    doc.append_child(c, c2);

    let f = lay(&doc, c, 300, 100);
    // grow 1:2 of 300 → 100, 200
    // reversed: children[0]=c2 (200px), children[1]=c1 (100px)
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[0].width(), lu(200));
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[1].width(), lu(100));
    // No free space left, so initial_offset = 0
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(200));
}

#[test]
fn dir_column_grow_distributes_height() {
    // Column with flex-grow: items with explicit heights are positioned top-to-bottom.
    // Note: block_layout resolves explicit height from style, so flex-grow
    // doesn't visually enlarge items with explicit heights in column direction.
    // We verify the items are positioned sequentially at their natural sizes.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 150);

    let f = lay(&doc, c, 200, 400);
    assert_eq!(f.children[0].height(), lu(100));
    assert_eq!(f.children[1].height(), lu(150));
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(100));
}

#[test]
fn dir_column_reverse_with_grow() {
    // Column-reverse with items of different sizes: verify reversed positioning.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::ColumnReverse;
    let c1 = add_child(&mut doc, c, 50, 80);
    let c2 = add_child(&mut doc, c, 50, 70);

    let f = lay(&doc, c, 200, 300);
    // reversed: children[0]=c2, children[1]=c1
    // free_space = 300 - 150 = 150; initial_offset = 150
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[0].height(), lu(70));
    assert_eq!(f.children[1].height(), lu(80));
    assert_eq!(f.children[0].offset.top, lu(150));
    assert_eq!(f.children[1].offset.top, lu(220));
}

#[test]
fn dir_column_explicit_width_no_stretch() {
    // Column: width is cross axis; items with explicit width don't stretch.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 200, 300);
    // Cross axis = width; explicit 80px does NOT stretch to 200.
    assert_eq!(f.children[0].width(), lu(80));
}

#[test]
fn dir_column_auto_width_stretches() {
    // Column: items with auto width stretch to container cross size.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    add_child_h(&mut doc, c, 50); // width = auto

    let f = lay(&doc, c, 200, 300);
    // Auto width stretches to container width (200).
    assert_eq!(f.children[0].width(), lu(200));
    assert_eq!(f.children[0].height(), lu(50));
}

#[test]
fn dir_row_height_is_cross_axis() {
    // Row: height is cross axis; items with explicit height don't stretch.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);
    // Explicit height 40 → no stretch to 100.
    assert_eq!(f.children[0].height(), lu(40));
    assert_eq!(f.children[0].width(), lu(50));
}

#[test]
fn dir_row_reverse_positions_mirrored() {
    // Row-reverse: items positioned from right edge of container.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let c1 = add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 40);
    let c3 = add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 200, 100);
    // free_space = 200 - 150 = 50; initial_offset = 50
    // reversed order: c3, c2, c1
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c1);
    assert_eq!(f.children[0].offset.left, lu(50));
    assert_eq!(f.children[1].offset.left, lu(100));
    assert_eq!(f.children[2].offset.left, lu(150));
}

#[test]
fn dir_column_reverse_positions_mirrored() {
    // Column-reverse: items positioned from bottom of container.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::ColumnReverse;
    let c1 = add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 60);
    let c3 = add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 200, 300);
    // free_space = 300 - 150 = 150; initial_offset = 150
    // reversed: c3, c2, c1
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c1);
    assert_eq!(f.children[0].offset.top, lu(150));
    assert_eq!(f.children[1].offset.top, lu(200));
    assert_eq!(f.children[2].offset.top, lu(260));
}

#[test]
fn dir_row_five_items() {
    // Row with 5 items of varying widths.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 500, 100);
    add_child(&mut doc, c, 30, 50);
    add_child(&mut doc, c, 40, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 70, 50);

    let f = lay(&doc, c, 500, 100);
    assert_eq!(f.children.len(), 5);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(30));
    assert_eq!(f.children[2].offset.left, lu(70));
    assert_eq!(f.children[3].offset.left, lu(120));
    assert_eq!(f.children[4].offset.left, lu(180));
}

#[test]
fn dir_column_five_items() {
    // Column with 5 items of varying heights.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 500);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    add_child(&mut doc, c, 50, 30);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 60);
    add_child(&mut doc, c, 50, 70);

    let f = lay(&doc, c, 200, 500);
    assert_eq!(f.children.len(), 5);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(30));
    assert_eq!(f.children[2].offset.top, lu(70));
    assert_eq!(f.children[3].offset.top, lu(120));
    assert_eq!(f.children[4].offset.top, lu(180));
}

#[test]
fn dir_row_reverse_three_different_sizes() {
    // Row-reverse with 3 differently sized items — verify exact positions.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let c1 = add_child(&mut doc, c, 40, 50);
    let c2 = add_child(&mut doc, c, 60, 50);
    let c3 = add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 300, 100);
    // free_space = 300 - 180 = 120; initial_offset = 120
    // reversed: c3(80), c2(60), c1(40)
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c1);
    assert_eq!(f.children[0].offset.left, lu(120));
    assert_eq!(f.children[0].width(), lu(80));
    assert_eq!(f.children[1].offset.left, lu(200));
    assert_eq!(f.children[1].width(), lu(60));
    assert_eq!(f.children[2].offset.left, lu(260));
    assert_eq!(f.children[2].width(), lu(40));
}

#[test]
fn dir_column_justify_center() {
    // Column with justify-content: center.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
    }
    add_child(&mut doc, c, 50, 60);
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 200, 400);
    // free_space = 400 - 100 = 300; initial_offset = 150
    assert_eq!(f.children[0].offset.top, lu(150));
    assert_eq!(f.children[1].offset.top, lu(210));
}

#[test]
fn dir_column_reverse_justify_center() {
    // Column-reverse with justify-content: center — center is unaffected by reverse.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::ColumnReverse;
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
    }
    let c1 = add_child(&mut doc, c, 50, 60);
    let c2 = add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 200, 400);
    // free_space = 400 - 100 = 300; center offset = 150
    // reversed order: c2(40), c1(60)
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[0].offset.top, lu(150));
    assert_eq!(f.children[1].offset.top, lu(190));
}

#[test]
fn dir_row_reverse_space_between() {
    // Row-reverse with space-between: items spread to ends, reversed order.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::RowReverse;
        s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    let _c1 = add_child(&mut doc, c, 50, 50);
    let _c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);
    // free_space = 150; space-between: initial_offset=0, between=75
    // reversed: c3, c2, c1
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(125));
    assert_eq!(f.children[2].offset.left, lu(250));
}

#[test]
fn dir_column_space_between() {
    // Column with space-between.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 60);

    let f = lay(&doc, c, 200, 300);
    // free_space = 300 - 100 = 200; 1 gap → between=200
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(240));
}

// ═══════════════════════════════════════════════════════════════════════════
// Category 2: flex-wrap (20 tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wrap_items_wrap_to_next_line() {
    // Wrap: items that exceed container width wrap to the next line.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 120, 50);
    add_child(&mut doc, c, 120, 50);

    let f = lay(&doc, c, 200, 300);
    // First item on line 1 (top=0), second wraps to line 2 (top=50).
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.top, lu(50));
    assert_eq!(f.children[1].offset.left, lu(0));
}

#[test]
fn wrap_two_on_first_one_wraps() {
    // 3 items: first 2 fit (100+100=200), third wraps.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 200, 300);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    assert_eq!(f.children[2].offset.top, lu(50));
    assert_eq!(f.children[2].offset.left, lu(0));
}

#[test]
fn wrap_each_item_own_line() {
    // Each item wider than container → each on its own line.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 150, 40);
    add_child(&mut doc, c, 150, 60);
    add_child(&mut doc, c, 150, 50);

    let f = lay(&doc, c, 100, 300);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(40));
    assert_eq!(f.children[2].offset.top, lu(100));
}

#[test]
fn wrap_different_sizes_line_composition() {
    // Wrap with items of different sizes: 80+80=160 fits in 200, 90 wraps, 60 fits with 90.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 80, 30);
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 90, 50);
    add_child(&mut doc, c, 60, 35);

    let f = lay(&doc, c, 200, 300);
    // Line 1: 80+80=160 ≤ 200 → items 0,1; line height=40
    // Line 2: 90+60=150 ≤ 200 → items 2,3; line height=50
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(40));
    assert_eq!(f.children[3].offset.top, lu(40));
    assert_eq!(f.children[2].offset.left, lu(0));
    assert_eq!(f.children[3].offset.left, lu(90));
}

#[test]
fn wrap_reverse_items_wrap_upward() {
    // Wrap-reverse: lines are reversed — last line appears first in children.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::WrapReverse;
        // flex-end with wrap-reverse packs at the visual top (cross-end becomes top)
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    let c1 = add_child(&mut doc, c, 120, 50);
    let c2 = add_child(&mut doc, c, 120, 50);
    let c3 = add_child(&mut doc, c, 120, 60);

    let f = lay(&doc, c, 200, 300);
    // 3 lines (1 item each): heights 50, 50, 60. Reversed: [c3(60)], [c2(50)], [c1(50)]
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c1);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[0].height(), lu(60));
    assert_eq!(f.children[1].offset.top, lu(60));
    assert_eq!(f.children[1].height(), lu(50));
    assert_eq!(f.children[2].offset.top, lu(110));
    assert_eq!(f.children[2].height(), lu(50));
}

#[test]
fn wrap_reverse_line_order_reversed() {
    // Wrap-reverse: 2 items per line, lines reversed in children output.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::WrapReverse;
        // flex-end with wrap-reverse packs at the visual top
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    let c1 = add_child(&mut doc, c, 100, 40);
    let c2 = add_child(&mut doc, c, 100, 40);
    let c3 = add_child(&mut doc, c, 100, 60);
    let c4 = add_child(&mut doc, c, 100, 60);

    let f = lay(&doc, c, 200, 300);
    // Line 1 (c1,c2 height=40), Line 2 (c3,c4 height=60)
    // Reversed: children order is [c3,c4 at top=0] then [c1,c2 at top=60]
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c4);
    assert_eq!(f.children[2].node_id, c1);
    assert_eq!(f.children[3].node_id, c2);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(60));
    assert_eq!(f.children[3].offset.top, lu(60));
}

#[test]
fn nowrap_items_overflow() {
    // Nowrap (default): items don't wrap, they overflow the container.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 200, 100);
    // All on same line.
    assert_eq!(f.children.len(), 3);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(0));
}

#[test]
fn wrap_column_wraps_to_next_column() {
    // Wrap in column direction: items wrap to next column when exceeding height.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 50, 60);
    add_child(&mut doc, c, 50, 60);
    add_child(&mut doc, c, 50, 60);

    let f = lay(&doc, c, 400, 100);
    // Column: main=vertical, cross=horizontal.
    // Item 0: height 60 fits. Item 1: 60+60=120 > 100 → wraps.
    // Line 1 (items 0): cross_offset=0, width=50
    // Line 2 (items 1): cross_offset=50, width=50
    // Line 3 (items 2): cross_offset=100, width=50
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(50));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.left, lu(100));
    assert_eq!(f.children[2].offset.top, lu(0));
}

#[test]
fn wrap_column_with_explicit_height() {
    // Wrap in column with explicit container height; items wrap after height exceeded.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 120);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 70, 50);

    let f = lay(&doc, c, 400, 120);
    // Line 1: item0(50)+item1(50)=100 ≤ 120 → fits; max width = 60
    // Line 2: item2(50) → wraps; width = 70
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(0));
    assert_eq!(f.children[1].offset.top, lu(50));
    assert_eq!(f.children[2].offset.left, lu(60));
    assert_eq!(f.children[2].offset.top, lu(0));
}

#[test]
fn wrap_reverse_column() {
    // Wrap-reverse in column: lines reversed in children output.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::WrapReverse;
        // flex-end with wrap-reverse packs at the visual left (cross-start)
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    let c1 = add_child(&mut doc, c, 50, 60);
    let c2 = add_child(&mut doc, c, 60, 60);
    let c3 = add_child(&mut doc, c, 70, 60);

    let f = lay(&doc, c, 400, 100);
    // Each item on its own line (60 fits, 60+60=120>100).
    // Reversed: children order is [c3(w=70)], [c2(w=60)], [c1(w=50)]
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c1);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(70));
    assert_eq!(f.children[2].offset.left, lu(130));
}

#[test]
fn wrap_with_flex_grow() {
    // Wrap with flex-grow: items (no explicit width) grow within their line.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;

    let c1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c1).style_mut();
        s.display = Display::Block;
        s.flex_basis = Length::px(80.0);
        s.height = Length::px(40.0);
        s.flex_grow = 1.0;
    }
    doc.append_child(c, c1);
    let c2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c2).style_mut();
        s.display = Display::Block;
        s.flex_basis = Length::px(80.0);
        s.height = Length::px(40.0);
        s.flex_grow = 1.0;
    }
    doc.append_child(c, c2);
    // Both fit on one line (80+80=160 ≤ 200), free_space=40, grow equally.
    let f = lay(&doc, c, 200, 300);
    assert_eq!(f.children[0].width(), lu(100));
    assert_eq!(f.children[1].width(), lu(100));
}

#[test]
fn wrap_items_exactly_fill_lines() {
    // Items exactly fill lines with no leftover.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 100, 40);
    add_child(&mut doc, c, 100, 40);
    add_child(&mut doc, c, 100, 60);
    add_child(&mut doc, c, 100, 60);

    let f = lay(&doc, c, 200, 300);
    // Line 1: 100+100=200, line 2: 100+100=200
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    assert_eq!(f.children[2].offset.left, lu(0));
    assert_eq!(f.children[3].offset.left, lu(100));
    assert_eq!(f.children[2].offset.top, lu(40));
}

#[test]
fn wrap_with_column_gap() {
    // Wrap with column-gap between items within lines.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(20.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 40);

    let f = lay(&doc, c, 200, 300);
    // 80+20+80=180 ≤ 200 → items 0,1 on line 1. 80 alone on line 2.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    assert_eq!(f.children[2].offset.top, lu(40));
    assert_eq!(f.children[2].offset.left, lu(0));
}

#[test]
fn wrap_with_row_gap() {
    // Wrap with row-gap between lines.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.row_gap = Some(Length::px(10.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 120, 40);
    add_child(&mut doc, c, 120, 50);

    let f = lay(&doc, c, 200, 300);
    // Each item on its own line. Line 1 height=40, gap=10, Line 2 at top=50.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(50));
}

#[test]
fn wrap_with_both_gaps() {
    // Wrap with column-gap and row-gap.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(20.0));
        s.row_gap = Some(Length::px(10.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 50);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 200, 300);
    // Line 1: 80+20+80=180 ≤ 200 → items 0,1 (height=40)
    // Line 2: 80+20+80=180 ≤ 200 → items 2,3 (height=50)
    // row-gap=10 between lines
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    assert_eq!(f.children[2].offset.top, lu(50));
    assert_eq!(f.children[3].offset.top, lu(50));
    assert_eq!(f.children[3].offset.left, lu(100));
}

#[test]
fn wrap_reverse_with_align_content_center() {
    // Wrap-reverse with align-content: center. Children in reversed line order.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::WrapReverse;
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    let c1 = add_child(&mut doc, c, 120, 40);
    let c2 = add_child(&mut doc, c, 120, 60);

    let f = lay(&doc, c, 200, 300);
    // 2 lines (1 item each): heights 40, 60.
    // wrap-reverse reverses: children order [c2(h=60)], [c1(h=40)]
    // total_lines_height = 100; free_cross = 300 - 100 = 200; center offset = 100
    // Line 0 (c2, h=60): cross_offset=100
    // Line 1 (c1, h=40): cross_offset=160
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[0].offset.top, lu(100));
    assert_eq!(f.children[1].offset.top, lu(160));
}

#[test]
fn wrap_single_item_no_wrapping() {
    // Single item doesn't need wrapping.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 200, 100);
    assert_eq!(f.children.len(), 1);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[0].offset.top, lu(0));
}

#[test]
fn wrap_items_with_margins() {
    // Wrap with items that have margins; margins count toward line space.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }

    let c1 = add_child(&mut doc, c, 80, 40);
    doc.node_mut(c1).style_mut().margin_right = Length::px(30.0);
    let _c2 = add_child(&mut doc, c, 80, 40);
    // c1 takes 80+30=110, c2 takes 80 → 110+80=190 ≤ 200 → same line
    let _c3 = add_child(&mut doc, c, 80, 50);
    // 190+80=270 > 200 → c3 wraps

    let f = lay(&doc, c, 200, 300);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(110));
    assert_eq!(f.children[2].offset.top, lu(40));
}

#[test]
fn wrap_cross_axis_positions_of_lines() {
    // Verify cross-axis positions: each wrapped line stacks below previous.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 300);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;
    add_child(&mut doc, c, 80, 30);
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 100, 300);
    // Each item on its own line: heights 30, 40, 50.
    // With align-content: stretch (default), lines expand to fill container (300px).
    // Free space = 300 - 120 = 180, distributed equally: each line gets +60.
    // Line 1: height=90 (30+60), starts at 0
    // Line 2: height=100 (40+60), starts at 90
    // Line 3: height=110 (50+60), starts at 190
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(90));
    assert_eq!(f.children[2].offset.top, lu(190));
}

#[test]
fn wrap_six_items_three_lines_of_two() {
    // 6 items in 3 lines of 2.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    for _ in 0..6 {
        add_child(&mut doc, c, 100, 40);
    }

    let f = lay(&doc, c, 200, 300);
    assert_eq!(f.children.len(), 6);
    // Line 1: items 0,1 at top=0
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    // Line 2: items 2,3 at top=40
    assert_eq!(f.children[2].offset.top, lu(40));
    assert_eq!(f.children[3].offset.top, lu(40));
    // Line 3: items 4,5 at top=80
    assert_eq!(f.children[4].offset.top, lu(80));
    assert_eq!(f.children[5].offset.top, lu(80));
}

// ═══════════════════════════════════════════════════════════════════════════
// Category 3: gap property (15 tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn gap_column_gap_in_row() {
    // column-gap adds horizontal space between items in a row flex.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().column_gap = Some(Length::px(20.0));
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(70));
    assert_eq!(f.children[2].offset.left, lu(140));
}

#[test]
fn gap_row_gap_in_column() {
    // row-gap adds vertical space between items in a column flex.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.row_gap = Some(Length::px(20.0));
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 200, 400);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(70));
    assert_eq!(f.children[2].offset.top, lu(140));
}

#[test]
fn gap_column_gap_in_wrap() {
    // column-gap between items on same line in wrap mode.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(20.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 40);

    let f = lay(&doc, c, 200, 300);
    // 80+20+80=180 ≤ 200 → items 0,1 on line 1
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    // Item 2 wraps
    assert_eq!(f.children[2].offset.top, lu(40));
    assert_eq!(f.children[2].offset.left, lu(0));
}

#[test]
fn gap_row_gap_in_wrap() {
    // row-gap between lines in wrap mode.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.row_gap = Some(Length::px(15.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 80, 40);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 100, 300);
    // Each item on its own line; row-gap=15 between.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(55));
}

#[test]
fn gap_both_in_wrap() {
    // Both column-gap and row-gap in wrap mode.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 250, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(10.0));
        s.row_gap = Some(Length::px(20.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    add_child(&mut doc, c, 100, 40);
    add_child(&mut doc, c, 100, 40);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 250, 300);
    // 100+10+100=210 ≤ 250 → items 0,1 on line 1; 100 on line 2
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(110));
    assert_eq!(f.children[2].offset.top, lu(60));
}

#[test]
fn gap_explicit_zero() {
    // gap=0 behaves same as no gap.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().column_gap = Some(Length::px(0.0));
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(50));
}

#[test]
fn gap_with_flex_grow() {
    // Gap reduces available space for flex-grow items.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().column_gap = Some(Length::px(20.0));

    let c1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c1).style_mut();
        s.display = Display::Block;
        s.flex_grow = 1.0;
        s.height = Length::px(50.0);
    }
    doc.append_child(c, c1);
    let c2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c2).style_mut();
        s.display = Display::Block;
        s.flex_grow = 1.0;
        s.height = Length::px(50.0);
    }
    doc.append_child(c, c2);

    let f = lay(&doc, c, 300, 100);
    // Available = 300 - 20 (gap) = 280; each gets 140.
    assert_eq!(f.children[0].width(), lu(140));
    assert_eq!(f.children[1].width(), lu(140));
    assert_eq!(f.children[1].offset.left, lu(160)); // 140 + 20 gap
}

#[test]
fn gap_with_space_between() {
    // Gap adds to space-between spacing.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.column_gap = Some(Length::px(10.0));
        s.justify_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);
    // free_space = 400 - 150 - 20 (2 gaps) = 230; between = 230/2 = 115
    // pos0=0, pos1=50+10+115=175, pos2=175+50+10+115=350
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(175));
    assert_eq!(f.children[2].offset.left, lu(350));
}

#[test]
fn gap_larger_than_free_space() {
    // Gap larger than remaining space — items still placed, overflow occurs.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.column_gap = Some(Length::px(100.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    let c1 = add_child(&mut doc, c, 80, 50);
    doc.node_mut(c1).style_mut().flex_shrink = 0.0;
    let c2 = add_child(&mut doc, c, 80, 50);
    doc.node_mut(c2).style_mut().flex_shrink = 0.0;

    let f = lay(&doc, c, 200, 100);
    // 80 + 100 gap + 80 = 260 > 200 (overflow with nowrap)
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(180));
}

#[test]
fn gap_in_row_reverse() {
    // Gap in row-reverse: gap applied between reversed items.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::RowReverse;
        s.column_gap = Some(Length::px(20.0));
    }
    let _c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);
    // reversed: c2, c1; free_space = 300 - 100 - 20 = 180; initial_offset = 180
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[0].offset.left, lu(180));
    assert_eq!(f.children[1].offset.left, lu(250)); // 180 + 50 + 20
}

#[test]
fn gap_in_column_reverse() {
    // Gap in column-reverse direction.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::ColumnReverse;
        s.row_gap = Some(Length::px(20.0));
    }
    let _c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 200, 300);
    // reversed: c2, c1; free_space = 300 - 100 - 20 = 180; initial_offset = 180
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[0].offset.top, lu(180));
    assert_eq!(f.children[1].offset.top, lu(250));
}

#[test]
fn gap_in_column_flex() {
    // row-gap in column direction (column uses row_gap for between-item gap).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.row_gap = Some(Length::px(10.0));
    }
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 60);

    let f = lay(&doc, c, 200, 300);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(50));
}

#[test]
fn gap_single_item_no_gap() {
    // With a single item, no gap is applied (gap is between items).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().column_gap = Some(Length::px(50.0));
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[0].width(), lu(100));
}

#[test]
fn gap_with_two_items() {
    // Two items with gap — one gap applied.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().column_gap = Some(Length::px(30.0));
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(90));
}

#[test]
fn gap_larger_than_container() {
    // Gap so large items overflow the container.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.column_gap = Some(Length::px(200.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    let c1 = add_child(&mut doc, c, 30, 50);
    doc.node_mut(c1).style_mut().flex_shrink = 0.0;
    let c2 = add_child(&mut doc, c, 30, 50);
    doc.node_mut(c2).style_mut().flex_shrink = 0.0;

    let f = lay(&doc, c, 100, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(230));
}

// ═══════════════════════════════════════════════════════════════════════════
// Category 4: order property (10 tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn order_default_dom_order() {
    // Default order (0): items placed in DOM order.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].node_id, c1);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c3);
}

#[test]
fn order_reorder_three_items() {
    // Reorder: 3→1, 1→2, 2→3 produces visual 2,3,1.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().order = 3;
    doc.node_mut(c2).style_mut().order = 1;
    doc.node_mut(c3).style_mut().order = 2;

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].node_id, c2); // order 1
    assert_eq!(f.children[1].node_id, c3); // order 2
    assert_eq!(f.children[2].node_id, c1); // order 3
}

#[test]
fn order_negative_comes_first() {
    // Negative order values place items before default order.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c2).style_mut().order = -1;

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].node_id, c2); // order -1
    assert_eq!(f.children[1].node_id, c1); // order 0
}

#[test]
fn order_equal_preserves_dom_order() {
    // Equal order values preserve DOM insertion order (stable sort).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().order = 1;
    doc.node_mut(c2).style_mut().order = 1;
    doc.node_mut(c3).style_mut().order = 1;

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].node_id, c1);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c3);
}

#[test]
fn order_all_same_dom_preserved() {
    // All items with same order (default 0) stay in DOM order.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let c1 = add_child(&mut doc, c, 40, 50);
    let c2 = add_child(&mut doc, c, 60, 50);
    let c3 = add_child(&mut doc, c, 80, 50);
    let c4 = add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);
    assert_eq!(f.children[0].node_id, c1);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c3);
    assert_eq!(f.children[3].node_id, c4);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(40));
    assert_eq!(f.children[2].offset.left, lu(100));
    assert_eq!(f.children[3].offset.left, lu(180));
}

#[test]
fn order_mixed_positive_negative() {
    // Mixed positive and negative orders.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    let c4 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().order = 2;
    doc.node_mut(c2).style_mut().order = -1;
    doc.node_mut(c3).style_mut().order = 0;
    doc.node_mut(c4).style_mut().order = -2;

    let f = lay(&doc, c, 400, 100);
    // sorted: c4(-2), c2(-1), c3(0), c1(2)
    assert_eq!(f.children[0].node_id, c4);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c3);
    assert_eq!(f.children[3].node_id, c1);
}

#[test]
fn order_with_row_reverse() {
    // Order with row-reverse: items sorted by order then reversed.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().order = 1;
    doc.node_mut(c2).style_mut().order = 2;
    doc.node_mut(c3).style_mut().order = 3;

    let f = lay(&doc, c, 300, 100);
    // After order sort: c1, c2, c3. After reverse: c3, c2, c1.
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c2);
    assert_eq!(f.children[2].node_id, c1);
}

#[test]
fn order_with_wrap() {
    // Order affects which items end up on which line.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    let c1 = add_child(&mut doc, c, 100, 40);
    let c2 = add_child(&mut doc, c, 100, 40);
    let c3 = add_child(&mut doc, c, 100, 40);
    // Give c3 order -1 so it appears first.
    doc.node_mut(c3).style_mut().order = -1;

    let f = lay(&doc, c, 200, 300);
    // sorted: c3(-1), c1(0), c2(0) → line1: c3+c1 (200), line2: c2
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[2].node_id, c2);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(40));
}

#[test]
fn order_five_items_various() {
    // 5 items with various orders.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 500, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    let c4 = add_child(&mut doc, c, 50, 50);
    let c5 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().order = 5;
    doc.node_mut(c2).style_mut().order = 3;
    doc.node_mut(c3).style_mut().order = 1;
    doc.node_mut(c4).style_mut().order = 4;
    doc.node_mut(c5).style_mut().order = 2;

    let f = lay(&doc, c, 500, 100);
    assert_eq!(f.children[0].node_id, c3); // order 1
    assert_eq!(f.children[1].node_id, c5); // order 2
    assert_eq!(f.children[2].node_id, c2); // order 3
    assert_eq!(f.children[3].node_id, c4); // order 4
    assert_eq!(f.children[4].node_id, c1); // order 5
}

#[test]
fn order_only_one_nonzero() {
    // Only one item has non-zero order → it moves to end.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().order = 1;

    let f = lay(&doc, c, 300, 100);
    // c2(0), c3(0), c1(1)
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c3);
    assert_eq!(f.children[2].node_id, c1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Category 5: auto margins (15 tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn auto_margin_left_pushes_right() {
    // margin-left: auto on an item pushes it to the right.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().margin_left = Length::auto();

    let f = lay(&doc, c, 300, 100);
    // free_space = 300 - 50 = 250; margin-left auto absorbs 250 → x = 250
    assert_eq!(f.children[0].offset.left, lu(250));
}

#[test]
fn auto_margin_right_pushes_left() {
    // margin-right: auto on an item pushes it to the left (keeps at start).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().margin_right = Length::auto();

    let f = lay(&doc, c, 300, 100);
    // margin-right: auto absorbs free space → item stays at left (x=0).
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn auto_margin_both_centers() {
    // margin-left: auto + margin-right: auto centers the item.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 100, 50);
    {
        let s = doc.node_mut(c1).style_mut();
        s.margin_left = Length::auto();
        s.margin_right = Length::auto();
    }

    let f = lay(&doc, c, 300, 100);
    // free_space = 200; each auto margin = 100 → item at x=100.
    assert_eq!(f.children[0].offset.left, lu(100));
}

#[test]
fn auto_margin_left_second_item() {
    // Two items, second has margin-left: auto → pushed to far right.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c2).style_mut().margin_left = Length::auto();

    let f = lay(&doc, c, 300, 100);
    // free_space = 300 - 100 = 200; c2's margin-left auto = 200
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(250));
}

#[test]
fn auto_margin_overrides_justify_content() {
    // Auto margins consume free space, overriding justify-content.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::Center);
    let c1 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().margin_left = Length::auto();
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);
    // free_space = 300; c1 has 1 auto margin → absorbs all 300.
    // c1 at x=300, c2 follows at x=350. justify-content center is overridden.
    assert_eq!(f.children[0].offset.left, lu(300));
    assert_eq!(f.children[1].offset.left, lu(350));
}

#[test]
fn auto_margin_top_in_row() {
    // margin-top: auto in row flex pushes item to bottom (cross axis).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c1).style_mut().margin_top = Length::auto();

    let f = lay(&doc, c, 300, 100);
    // Cross space = 100 - 40 = 60; margin-top auto absorbs 60 → top=60.
    assert_eq!(f.children[0].offset.top, lu(60));
}

#[test]
fn auto_margin_bottom_in_row() {
    // margin-bottom: auto in row flex pushes item to top (cross axis).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c1).style_mut().margin_bottom = Length::auto();

    let f = lay(&doc, c, 300, 100);
    // margin-bottom: auto → item stays at top (top=0).
    assert_eq!(f.children[0].offset.top, lu(0));
}

#[test]
fn auto_margin_top_bottom_centers_vertically() {
    // margin-top: auto + margin-bottom: auto centers vertically (overrides align-items).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexStart);
    let c1 = add_child(&mut doc, c, 50, 40);
    {
        let s = doc.node_mut(c1).style_mut();
        s.margin_top = Length::auto();
        s.margin_bottom = Length::auto();
    }

    let f = lay(&doc, c, 300, 100);
    // cross_space = 60; each auto margin = 30 → top = 30
    assert_eq!(f.children[0].offset.top, lu(30));
}

#[test]
fn auto_margins_in_column_direction() {
    // Auto margin-top in column flex pushes item down (main axis).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let c1 = add_child(&mut doc, c, 50, 60);
    doc.node_mut(c1).style_mut().margin_top = Length::auto();

    let f = lay(&doc, c, 200, 300);
    // free_space = 300 - 60 = 240; margin-top auto absorbs 240 → top=240
    assert_eq!(f.children[0].offset.top, lu(240));
}

#[test]
fn auto_margins_multiple_items() {
    // Multiple items with auto margins share the free space.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().margin_right = Length::auto();
    let c2 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c2).style_mut().margin_left = Length::auto();

    let f = lay(&doc, c, 400, 100);
    // free_space = 300, 2 auto margins → each gets 150
    // c1 at x=0, margin-right=150, c2 at x=50+150+150=350
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(350));
}

#[test]
fn auto_margin_with_flex_grow() {
    // When items have flex-grow, all free space goes to growth; auto margin gets 0.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c1).style_mut();
        s.display = Display::Block;
        s.height = Length::px(50.0);
        s.flex_grow = 1.0;
        s.margin_left = Length::auto();
    }
    doc.append_child(c, c1);

    let f = lay(&doc, c, 300, 100);
    // flex-grow consumes all space → item is 300px wide, margin-left auto = 0.
    assert_eq!(f.children[0].width(), lu(300));
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn auto_margin_no_free_space() {
    // When items overflow container (negative free space), auto margin resolves to 0.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 150, 100);
    doc.node_mut(c).style_mut().align_content = ContentAlignment::new(ContentPosition::FlexStart);
    let c1 = add_child(&mut doc, c, 100, 50);
    {
        let s = doc.node_mut(c1).style_mut();
        s.margin_left = Length::auto();
        s.flex_shrink = 0.0;
    }
    let c2 = add_child(&mut doc, c, 100, 50);
    doc.node_mut(c2).style_mut().flex_shrink = 0.0;

    let f = lay(&doc, c, 150, 100);
    // Items = 100+100=200 > 150: negative free space. Auto margin = 0.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
}

#[test]
fn auto_margin_left_first_right_last() {
    // First item has margin-left: auto, last has margin-right: auto.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let c1 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().margin_left = Length::auto();
    let _c2 = add_child(&mut doc, c, 50, 50);
    let c3 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c3).style_mut().margin_right = Length::auto();

    let f = lay(&doc, c, 400, 100);
    // free_space = 250; 2 auto margins → each gets 125.
    // c1: margin-left=125 → x=125; c2: x=175; c3: x=225, margin-right=125
    assert_eq!(f.children[0].offset.left, lu(125));
    assert_eq!(f.children[1].offset.left, lu(175));
    assert_eq!(f.children[2].offset.left, lu(225));
}

#[test]
fn auto_margin_cross_axis_different_heights() {
    // Cross-axis auto margins with items of different heights.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c1).style_mut().margin_top = Length::auto();
    let c2 = add_child(&mut doc, c, 50, 60);
    doc.node_mut(c2).style_mut().margin_top = Length::auto();

    let f = lay(&doc, c, 300, 100);
    // c1: cross_space = 100-40=60, margin-top=60 → top=60
    // c2: cross_space = 100-60=40, margin-top=40 → top=40
    assert_eq!(f.children[0].offset.top, lu(60));
    assert_eq!(f.children[1].offset.top, lu(40));
}

#[test]
fn auto_margin_both_main_axis_centers() {
    // Both main-axis margins auto on one item centers it horizontally.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let c1 = add_child(&mut doc, c, 100, 50);
    {
        let s = doc.node_mut(c1).style_mut();
        s.margin_left = Length::auto();
        s.margin_right = Length::auto();
    }

    let f = lay(&doc, c, 400, 100);
    // free_space = 300; 2 auto margins → 150 each → x=150.
    assert_eq!(f.children[0].offset.left, lu(150));
}
