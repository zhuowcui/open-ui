//! Validation tests for flex layout fixes:
//!   1. min-width:auto content-based minimum sizing
//!   2. align-self cross-axis positioning
//!   3. flex-wrap + column direction (intrinsic sizing + wrapping)
//!   4. gap property handling
//!
//! These tests verify correct behavior per CSS Flexbox Level 1.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{flex_layout, ConstraintSpace, Fragment};
use openui_style::{
    BoxSizing, ContentAlignment, ContentDistribution, ContentPosition, Display,
    FlexDirection, FlexWrap, ItemAlignment, ItemPosition, Overflow,
};

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

fn lu(v: i32) -> LayoutUnit { LayoutUnit::from_i32(v) }

fn make_flex(doc: &mut Document, w: i32, h: i32) -> NodeId {
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(w as f32);
        s.height = Length::px(h as f32);
    }
    doc.append_child(doc.root(), c);
    c
}

fn make_flex_w(doc: &mut Document, w: i32) -> NodeId {
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(w as f32);
    }
    doc.append_child(doc.root(), c);
    c
}

fn add(doc: &mut Document, parent: NodeId, w: i32, h: i32) -> NodeId {
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Block;
        s.width = Length::px(w as f32);
        s.height = Length::px(h as f32);
    }
    doc.append_child(parent, c);
    c
}

fn add_h(doc: &mut Document, parent: NodeId, h: i32) -> NodeId {
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Block;
        s.height = Length::px(h as f32);
    }
    doc.append_child(parent, c);
    c
}

fn lay(doc: &Document, c: NodeId, w: i32, h: i32) -> Fragment {
    let space = ConstraintSpace::for_root(lu(w), lu(h));
    flex_layout(doc, c, &space)
}

// ═══════════════════════════════════════════════════════════════
// Issue 1: min-width:auto content-based minimum (CSS §4.5)
// ═══════════════════════════════════════════════════════════════

#[test]
fn min_auto_prevents_shrink_below_content() {
    // Item with child wider than flex-basis should not shrink below child width
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.flex_shrink = 1.0;
        // width:auto, min-width:auto (default)
    }
    doc.append_child(c, item);
    // Child with explicit width provides content minimum
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Block;
        s.width = Length::px(150.0);
        s.height = Length::px(20.0);
    }
    doc.append_child(item, inner);

    let item2 = add(&mut doc, c, 100, 50);
    let _ = item2;

    let f = lay(&doc, c, 200, 100);
    // min-width:auto = 150 (content min). Item can't shrink below 150.
    assert!(f.children[0].width() >= lu(150));
}

#[test]
fn min_auto_overflow_hidden_allows_shrink() {
    // overflow:hidden → min-width:auto = 0, item CAN shrink
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    let item1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item1).style_mut();
        s.display = Display::Block;
        s.width = Length::px(150.0);
        s.flex_shrink = 1.0;
        s.overflow_x = Overflow::Hidden;
    }
    doc.append_child(c, item1);
    let item2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item2).style_mut();
        s.display = Display::Block;
        s.width = Length::px(150.0);
        s.flex_shrink = 1.0;
        s.overflow_x = Overflow::Hidden;
    }
    doc.append_child(c, item2);

    let f = lay(&doc, c, 200, 100);
    // Both items have explicit width=150, overflow:hidden → min=0
    // Total 300 > 200, shrink equally → each 100
    assert_eq!(f.children[0].width(), lu(100));
    assert_eq!(f.children[1].width(), lu(100));
}

#[test]
fn min_auto_overflow_scroll_allows_shrink() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.width = Length::px(300.0);
        s.flex_shrink = 1.0;
        s.overflow_x = Overflow::Scroll;
    }
    doc.append_child(c, item);

    let f = lay(&doc, c, 200, 100);
    // overflow:scroll → min=0. Item shrinks to container width.
    assert_eq!(f.children[0].width(), lu(200));
}

#[test]
fn min_auto_clamp_by_specified_size() {
    // min-width:auto = min(content_size, specified_size) per §4.5
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 100);
    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.flex_shrink = 1.0;
    }
    doc.append_child(c, item);
    // Inner child wider than item's specified width
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Block;
        s.width = Length::px(200.0);
        s.height = Length::px(10.0);
    }
    doc.append_child(item, inner);

    let f = lay(&doc, c, 100, 100);
    // content_size=200, specified_size=50 → min = min(200,50) = 50
    assert_eq!(f.children[0].width(), lu(50));
}

#[test]
fn min_auto_column_flex_min_height() {
    // In column flex, min-height:auto prevents vertical shrinking
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(100.0);
        s.flex_direction = FlexDirection::Column;
    }
    doc.append_child(doc.root(), c);

    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.height = Length::px(150.0);
        s.flex_shrink = 1.0;
    }
    doc.append_child(c, item);

    let f = lay(&doc, c, 200, 100);
    // Column flex: item has height:150, container:100. min-height:auto =
    // min(content_suggestion, specified_suggestion) = min(0, 150) = 0 for
    // empty div. Item CAN shrink to container height (100) because content
    // is empty.
    assert!(f.children[0].height() <= lu(100));
}

#[test]
fn min_auto_explicit_min_width_zero() {
    // Explicit min-width:0 overrides auto minimum
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.width = Length::px(300.0);
        s.flex_shrink = 1.0;
        s.min_width = Length::px(0.0);
    }
    doc.append_child(c, item);

    let f = lay(&doc, c, 200, 100);
    // Explicit min-width:0 → item shrinks to container
    assert_eq!(f.children[0].width(), lu(200));
}

// ═══════════════════════════════════════════════════════════════
// Issue 2: align-self cross-axis positioning
// ═══════════════════════════════════════════════════════════════

#[test]
fn align_stretch_fills_line_cross_size() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    // Auto-height item (default stretch)
    let _item = add_h(&mut doc, c, 0); // auto height by giving 0 explicitly
    // Actually, let's use a truly auto-height item
    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
    }
    doc.append_child(c, item);

    let f = lay(&doc, c, 300, 100);
    // Stretch should fill the line cross size (100)
    assert_eq!(f.children[1].height(), lu(100));
}

#[test]
fn align_center_positions_correctly() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    doc.append_child(doc.root(), c);
    add(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.top, lu(30));
    assert_eq!(f.children[0].height(), lu(40));
}

#[test]
fn align_flex_end_positions_at_bottom() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.align_items = ItemAlignment::new(ItemPosition::FlexEnd);
    }
    doc.append_child(doc.root(), c);
    add(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.top, lu(60));
}

#[test]
fn align_self_override() {
    // align-self overrides container's align-items
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.align_items = ItemAlignment::new(ItemPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    let item = add(&mut doc, c, 50, 40);
    doc.node_mut(item).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.top, lu(30));
}

#[test]
fn align_stretch_with_margins() {
    // Stretch should fill line minus margins
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.margin_top = Length::px(10.0);
        s.margin_bottom = Length::px(10.0);
        // height:auto → stretch
    }
    doc.append_child(c, item);

    let f = lay(&doc, c, 300, 100);
    // Stretch: 100 - 10 - 10 = 80
    assert_eq!(f.children[0].height(), lu(80));
}

#[test]
fn align_stretch_explicit_height_no_stretch() {
    // Explicit height prevents stretch
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].height(), lu(40));
}

#[test]
fn align_center_in_multiline() {
    // Center alignment on a specific item in multi-line flex
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
    }
    doc.append_child(doc.root(), c);

    // Line 1: two items
    add(&mut doc, c, 100, 60);
    add(&mut doc, c, 100, 60);
    // Line 2: one tall + one short centered
    add(&mut doc, c, 100, 60);
    let short = add(&mut doc, c, 100, 30);
    doc.node_mut(short).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);

    let f = lay(&doc, c, 200, 200);
    // align-content:normal → stretch. Free=200-60-60=80. +40 per line.
    // Line 2: cross=100. Short item centered: (100-30)/2=35. y=100+35=135.
    assert_eq!(f.children[3].offset.top, lu(135));
}

#[test]
fn align_self_column_flex() {
    // align-self: center in column flex (cross axis = horizontal)
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    doc.append_child(doc.root(), c);
    add(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 200, 300);
    // Cross axis = horizontal. Center: (200-80)/2 = 60
    assert_eq!(f.children[0].offset.left, lu(60));
}

// ═══════════════════════════════════════════════════════════════
// Issue 3: flex-wrap + column direction
// ═══════════════════════════════════════════════════════════════

#[test]
fn column_wrap_basic() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 80, 50);
    add(&mut doc, c, 80, 50);
    add(&mut doc, c, 80, 60);

    let f = lay(&doc, c, 300, 100);
    // Items 0,1: 50+50=100 fits. Item 2: 60 would exceed → wraps.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(50));
    assert_eq!(f.children[2].offset.top, lu(0));
    assert_eq!(f.children[2].offset.left, lu(80));
}

#[test]
fn column_wrap_line_cross_offset() {
    // Cross offsets (x) accumulate correctly with different item widths
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 50, 60);  // col 1
    add(&mut doc, c, 60, 60);  // col 1 (50+60=110 > 100, wraps) → col 2
    add(&mut doc, c, 70, 60);  // col 3

    let f = lay(&doc, c, 400, 100);
    // Col 1: item0 (w=50), cross_offset=0
    // Col 2: item1 (w=60), cross_offset=50
    // Col 3: item2 (w=70), cross_offset=50+60=110
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(50));
    assert_eq!(f.children[2].offset.left, lu(110));
}

#[test]
fn column_wrap_intrinsic_width() {
    // CSS Flexbox §9.9: Column wrap container intrinsic inline size
    // should account for all wrapped columns.
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.max_height = Length::px(100.0);
    }
    doc.append_child(doc.root(), c);

    // Two items each 100px tall, 50px wide. max-height:100 → each on its own column.
    let c1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c1).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.flex_grow = 0.0;
        s.flex_shrink = 0.0;
        s.flex_basis = Length::px(100.0);
        s.min_height = Length::px(0.0);
    }
    doc.append_child(c, c1);

    let c2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c2).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.flex_grow = 0.0;
        s.flex_shrink = 0.0;
        s.flex_basis = Length::px(100.0);
        s.min_height = Length::px(0.0);
    }
    doc.append_child(c, c2);

    // Use intrinsic sizing: the container should compute its width from content
    let sizes = openui_layout::intrinsic_sizing::compute_intrinsic_block_sizes(&doc, c);
    // 2 columns × 50px each = 100px max-content inline
    assert_eq!(sizes.max_content_inline_size, lu(100));
}

#[test]
fn column_wrap_two_items_per_column() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(120.0);
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);

    // 3 items of h=50, container h=120. Items 0,1 fit (50+50=100<120), item 2 wraps.
    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 60, 50);
    add(&mut doc, c, 70, 50);

    let f = lay(&doc, c, 300, 120);
    // Col 1: items 0,1. Max width = max(50,60) = 60.
    // Col 2: item 2 wraps. Width = 70.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.left, lu(0));
    assert_eq!(f.children[1].offset.top, lu(50));
    assert_eq!(f.children[2].offset.left, lu(60));
    assert_eq!(f.children[2].offset.top, lu(0));
}

#[test]
fn column_wrap_with_gap_between_lines() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(10.0)); // gap between columns
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 50, 60);
    add(&mut doc, c, 50, 60);

    let f = lay(&doc, c, 400, 100);
    // Each item on its own column (60+60>100). Column gap = 10.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(60)); // 50 (col width) + 10 (gap)
}

// ═══════════════════════════════════════════════════════════════
// Issue 4: gap property in flex
// ═══════════════════════════════════════════════════════════════

#[test]
fn gap_column_gap_basic() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.column_gap = Some(Length::px(20.0));
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);
    // Positions: 0, 50+20=70, 70+50+20=140
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(70));
    assert_eq!(f.children[2].offset.left, lu(140));
}

#[test]
fn gap_row_gap_between_lines() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.row_gap = Some(Length::px(10.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 100, 50);
    add(&mut doc, c, 100, 50);
    add(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 200, 400);
    // Line 1: items 0,1. Line 2: item 2.
    // Row gap = 10 between lines.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(60)); // 50 + 10
}

#[test]
fn gap_does_not_participate_in_grow() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.column_gap = Some(Length::px(20.0));
    }
    doc.append_child(doc.root(), c);

    for _ in 0..2 {
        let item = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(item).style_mut();
            s.display = Display::Block;
            s.flex_grow = 1.0;
            s.height = Length::px(50.0);
        }
        doc.append_child(c, item);
    }

    let f = lay(&doc, c, 400, 100);
    // Available = 400 - 20 (1 gap) = 380. Each gets 190.
    assert_eq!(f.children[0].width(), lu(190));
    assert_eq!(f.children[1].width(), lu(190));
}

#[test]
fn gap_single_item_no_gap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.column_gap = Some(Length::px(20.0));
    }
    doc.append_child(doc.root(), c);

    let item = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(item).style_mut();
        s.display = Display::Block;
        s.flex_grow = 1.0;
        s.height = Length::px(50.0);
    }
    doc.append_child(c, item);

    let f = lay(&doc, c, 400, 100);
    // Single item: no gap applied, gets full width
    assert_eq!(f.children[0].width(), lu(400));
}

#[test]
fn gap_in_column_flex() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
        s.row_gap = Some(Length::px(10.0)); // row_gap = main axis gap for column
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 200, 300);
    // Column: items stack vertically. Gap between them = 10.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(60)); // 50 + 10
    assert_eq!(f.children[2].offset.top, lu(120)); // 60 + 50 + 10
}

#[test]
fn gap_with_wrap_both_gaps() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(10.0));
        s.row_gap = Some(Length::px(5.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);

    // 3 items of 100px wide. Container 200px. Items 0,1 on line 1 (100+10+100=210>200).
    // Actually 100+10+100=210 > 200 → item 1 wraps.
    // Wait, 100+10=110, +100=210>200. So item 1 wraps.
    // Line 1: [item0] width=100.  Line 2: [item1] width=100.  Line 3: [item2].
    // Hmm, let me use smaller items.
    add(&mut doc, c, 90, 50);
    add(&mut doc, c, 90, 50);
    add(&mut doc, c, 90, 50);

    let f = lay(&doc, c, 200, 400);
    // 90+10+90=190 <= 200 → items 0,1 on line 1. Item 2 wraps.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100)); // 90 + 10 gap
    // Row gap between lines
    assert_eq!(f.children[2].offset.top, lu(55)); // 50 + 5 gap
}

#[test]
fn gap_not_before_first_or_after_last() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(100.0);
        s.column_gap = Some(Length::px(20.0));
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 200, 100);
    // First (and only) item starts at x=0, no gap before it
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn gap_with_space_between() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.column_gap = Some(Length::px(10.0));
        s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    doc.append_child(doc.root(), c);

    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 50, 50);
    add(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);
    // Items: 3×50=150. Gaps: 2×10=20. Free: 400-150-20=230. space-between: 115 each.
    assert_eq!(f.children[0].offset.left, lu(0));
    // item1: 0 + 50 + 10(gap) + 115(space) = 175
    assert_eq!(f.children[1].offset.left, lu(175));
    // item2: 175 + 50 + 10(gap) + 115(space) = 350
    assert_eq!(f.children[2].offset.left, lu(350));
}

// ═══════════════════════════════════════════════════════════════
// Cross-cutting: inline children intrinsic sizing
// ═══════════════════════════════════════════════════════════════

#[test]
fn inline_children_max_content_sums() {
    // A div with two inline-block children of width:10px each should have
    // max-content inline size = 20 (both on one line), not 10 (max of one).
    let mut doc = Document::new();
    let parent = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(parent).style_mut();
        s.display = Display::Block;
    }
    doc.append_child(doc.root(), parent);

    for _ in 0..2 {
        let ib = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(ib).style_mut();
            s.display = Display::InlineBlock;
            s.width = Length::px(10.0);
            s.height = Length::px(10.0);
        }
        doc.append_child(parent, ib);
    }

    let sizes = openui_layout::intrinsic_sizing::compute_intrinsic_block_sizes(&doc, parent);
    // Two inline-blocks of 10px → max-content = 20 (sum)
    assert_eq!(sizes.max_content_inline_size, lu(20));
    // min-content = 10 (widest single item)
    assert_eq!(sizes.min_content_inline_size, lu(10));
}
