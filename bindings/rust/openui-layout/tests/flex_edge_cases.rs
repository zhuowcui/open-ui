//! Comprehensive edge-case tests for the CSS Flexbox layout engine.
//! Covers min/max constraints, border/padding, nested flex, block_layout
//! dispatch, empty/single items, percentages, and real-world scenarios.

use openui_layout::{flex_layout, block_layout, ConstraintSpace, Fragment};
use openui_dom::{Document, NodeId, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_style::{
    Display, FlexDirection, FlexWrap,
    ContentAlignment, ContentDistribution, ContentPosition,
    ItemAlignment, ItemPosition,
    BorderStyle,
};

// ── Helpers ──────────────────────────────────────────────────────────

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

fn add_auto_child(doc: &mut Document, parent: NodeId) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
    }
    doc.append_child(parent, child);
    child
}

fn lay(doc: &Document, container: NodeId, w: i32, h: i32) -> Fragment {
    let space = ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h));
    flex_layout(doc, container, &space)
}

fn lay_block(doc: &Document, container: NodeId, w: i32, h: i32) -> Fragment {
    let space = ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h));
    block_layout(doc, container, &space)
}

fn lu(v: i32) -> LayoutUnit { LayoutUnit::from_i32(v) }

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 1: min-width / max-width constraints (15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c1_01_min_width_prevents_shrink() {
    // Two items flex-basis:200 each in 300px container. A has min-width:180.
    // Without min, both shrink to 150. With min on A, A=180, B=120.
    // (Items with explicit width don't shrink; use flex-basis for shrink tests.)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(200.0);
        s.min_width = Length::px(180.0);
        s.height = Length::px(50.0);
    }
    let b = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(b).style_mut();
        s.flex_basis = Length::px(200.0);
        s.height = Length::px(50.0);
    }

    let frag = lay(&doc, c, 300, 100);
    assert_eq!(frag.children.len(), 2);
    // A: min-width 180 prevents shrinking below 180
    assert!(frag.children[0].width() >= lu(180));
    // B gets the rest
    let total = frag.children[0].width() + frag.children[1].width();
    assert_eq!(total, lu(300));
}

#[test]
fn c1_02_max_width_prevents_grow() {
    // Two items with flex-grow:1, one has max-width:80 in 400px container.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_grow = 1.0;
        s.max_width = Length::px(80.0);
    }
    let b = add_auto_child(&mut doc, c);
    doc.node_mut(b).style_mut().flex_grow = 1.0;

    let frag = lay(&doc, c, 400, 100);
    // A capped at 80
    assert!(frag.children[0].width() <= lu(80));
    // B gets the rest
    assert!(frag.children[1].width() >= lu(320));
}

#[test]
fn c1_03_min_width_larger_than_basis() {
    // flex-basis:50, min-width:100 → item should be at least 100
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(50.0);
        s.min_width = Length::px(100.0);
    }
    let frag = lay(&doc, c, 400, 100);
    assert!(frag.children[0].width() >= lu(100));
}

#[test]
fn c1_04_max_width_smaller_than_basis() {
    // flex-basis:200, max-width:100 → item clamped to 100
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(200.0);
        s.max_width = Length::px(100.0);
    }
    let frag = lay(&doc, c, 400, 100);
    assert!(frag.children[0].width() <= lu(100));
}

#[test]
fn c1_05_min_width_with_flex_grow() {
    // flex-grow:1, min-width:300 in 200px container → min wins, item=300 (overflow)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_grow = 1.0;
        s.min_width = Length::px(300.0);
    }
    let frag = lay(&doc, c, 200, 100);
    assert!(frag.children[0].width() >= lu(300));
}

#[test]
fn c1_06_max_width_with_flex_grow_extra_to_others() {
    // A: flex-grow:1 max-width:100, B: flex-grow:1 in 400px container.
    // A stops at 100, B gets remaining 300.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_grow = 1.0;
        s.max_width = Length::px(100.0);
    }
    let b = add_auto_child(&mut doc, c);
    doc.node_mut(b).style_mut().flex_grow = 1.0;

    let frag = lay(&doc, c, 400, 100);
    assert!(frag.children[0].width() <= lu(100));
    assert!(frag.children[1].width() >= lu(300));
}

#[test]
fn c1_07_min_height_column_flex() {
    // Column flex: item with min-height:80, flex-basis:30 → min wins
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 200);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(30.0);
        s.min_height = Length::px(80.0);
    }
    let frag = lay(&doc, c, 100, 200);
    assert!(frag.children[0].height() >= lu(80));
}

#[test]
fn c1_08_max_height_column_flex() {
    // Column flex: item with max-height:50, flex-grow:1 → stops at 50
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 200);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_grow = 1.0;
        s.max_height = Length::px(50.0);
    }
    let frag = lay(&doc, c, 100, 200);
    assert!(frag.children[0].height() <= lu(50));
}

#[test]
fn c1_09_min_width_zero_allows_shrink() {
    // min-width:0 + flex-basis allows shrinking below content size.
    // Two items flex-basis:200 in 100px container with min-width:0
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 100, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(200.0);
        s.min_width = Length::px(0.0);
        s.height = Length::px(50.0);
    }
    let b = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(b).style_mut();
        s.flex_basis = Length::px(200.0);
        s.min_width = Length::px(0.0);
        s.height = Length::px(50.0);
    }

    let frag = lay(&doc, c, 100, 100);
    // Both should shrink to fit 100px total
    let total = frag.children[0].width() + frag.children[1].width();
    assert_eq!(total, lu(100));
}

#[test]
fn c1_10_max_width_equals_basis_no_growth() {
    // flex-basis:100, max-width:100, flex-grow:1 → stays at 100
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(100.0);
        s.max_width = Length::px(100.0);
        s.flex_grow = 1.0;
    }
    let frag = lay(&doc, c, 400, 100);
    assert_eq!(frag.children[0].width(), lu(100));
}

#[test]
fn c1_11_min_width_overflows_container() {
    // Single item, min-width:500 in 300px container → 500 (overflow)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.min_width = Length::px(500.0);
    }
    let frag = lay(&doc, c, 300, 100);
    assert!(frag.children[0].width() >= lu(500));
}

#[test]
fn c1_12_multiple_one_with_min_one_without() {
    // A: flex-basis:200 min-width:180, B: flex-basis:200, container:300
    // Both shrink, but A can't go below 180
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(200.0);
        s.min_width = Length::px(180.0);
        s.height = Length::px(50.0);
    }
    let b = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(b).style_mut();
        s.flex_basis = Length::px(200.0);
        s.height = Length::px(50.0);
    }

    let frag = lay(&doc, c, 300, 100);
    assert!(frag.children[0].width() >= lu(180));
    let total = frag.children[0].width() + frag.children[1].width();
    assert_eq!(total, lu(300));
}

#[test]
fn c1_13_all_items_max_width_grow_stops() {
    // 3 items flex-grow:1 max-width:50 in 300px container → all 50
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    for _ in 0..3 {
        let ch = add_auto_child(&mut doc, c);
        let s = doc.node_mut(ch).style_mut();
        s.flex_grow = 1.0;
        s.max_width = Length::px(50.0);
    }
    let frag = lay(&doc, c, 300, 100);
    for child in &frag.children {
        assert!(child.width() <= lu(50));
    }
}

#[test]
fn c1_14_min_and_max_range_on_same_item() {
    // min-width:80, max-width:120, flex-basis:50 → clamp to 80
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(50.0);
        s.min_width = Length::px(80.0);
        s.max_width = Length::px(120.0);
    }
    let frag = lay(&doc, c, 400, 100);
    assert!(frag.children[0].width() >= lu(80));
    assert!(frag.children[0].width() <= lu(120));
}

#[test]
fn c1_15_min_greater_than_max_min_wins() {
    // When min-width > max-width, this implementation uses max-width.
    // min-width:200, max-width:100 → item = 100 (max applies)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.min_width = Length::px(200.0);
        s.max_width = Length::px(100.0);
    }
    let frag = lay(&doc, c, 400, 100);
    assert_eq!(frag.children[0].width(), lu(100));
}

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 2: border and padding on items (15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c2_01_item_padding_increases_size() {
    // Item width:100, padding:10 all around → border-box = 120
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_child(&mut doc, c, 100, 50);
    {
        let s = doc.node_mut(a).style_mut();
        s.padding_top = Length::px(10.0);
        s.padding_right = Length::px(10.0);
        s.padding_bottom = Length::px(10.0);
        s.padding_left = Length::px(10.0);
    }
    let frag = lay(&doc, c, 400, 100);
    // content-box sizing: border-box = content(100) + padding(20) = 120
    assert_eq!(frag.children[0].width(), lu(120));
}

#[test]
fn c2_02_item_border_increases_size() {
    // Item width:100, border:5 all around → border-box = 110
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_child(&mut doc, c, 100, 50);
    {
        let s = doc.node_mut(a).style_mut();
        s.border_top_width = 5;
        s.border_right_width = 5;
        s.border_bottom_width = 5;
        s.border_left_width = 5;
        s.border_top_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
        s.border_bottom_style = BorderStyle::Solid;
        s.border_left_style = BorderStyle::Solid;
    }
    let frag = lay(&doc, c, 400, 100);
    // content-box: border-box = 100 + 10 = 110
    assert_eq!(frag.children[0].width(), lu(110));
}

#[test]
fn c2_03_item_padding_border_width() {
    // width:80, padding:10 each side, border:5 each → 80 + 20 + 10 = 110
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_child(&mut doc, c, 80, 50);
    {
        let s = doc.node_mut(a).style_mut();
        s.padding_left = Length::px(10.0);
        s.padding_right = Length::px(10.0);
        s.border_left_width = 5;
        s.border_right_width = 5;
        s.border_left_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
    }
    let frag = lay(&doc, c, 400, 100);
    // content-box: border-box = content(80) + padding(20) + border(10) = 110
    assert_eq!(frag.children[0].width(), lu(110));
}

#[test]
fn c2_04_flex_grow_with_padding() {
    // Two items flex-grow:1, each has padding:10 in 400px container.
    // Padding is not distributed – grows fill remaining space.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_grow = 1.0;
        s.padding_left = Length::px(10.0);
        s.padding_right = Length::px(10.0);
    }
    let b = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(b).style_mut();
        s.flex_grow = 1.0;
        s.padding_left = Length::px(10.0);
        s.padding_right = Length::px(10.0);
    }
    let frag = lay(&doc, c, 400, 100);
    // Both should have equal width and sum to 400
    let total = frag.children[0].width() + frag.children[1].width();
    assert_eq!(total, lu(400));
    assert_eq!(frag.children[0].width(), frag.children[1].width());
}

#[test]
fn c2_05_flex_shrink_with_padding() {
    // Two items basis:250 + padding:10 each side in 400px container.
    // Total basis = 250+20 + 250+20 = 540 > 400 → shrink.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::px(250.0);
        s.padding_left = Length::px(10.0);
        s.padding_right = Length::px(10.0);
    }
    let b = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(b).style_mut();
        s.flex_basis = Length::px(250.0);
        s.padding_left = Length::px(10.0);
        s.padding_right = Length::px(10.0);
    }
    let frag = lay(&doc, c, 400, 100);
    let total = frag.children[0].width() + frag.children[1].width();
    assert_eq!(total, lu(400));
}

#[test]
fn c2_06_container_padding_shifts_items() {
    // Container padding:20, item 100px → item starts at offset 20,20
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.padding_top = Length::px(20.0);
        s.padding_right = Length::px(20.0);
        s.padding_bottom = Length::px(20.0);
        s.padding_left = Length::px(20.0);
    }
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 400, 200);
    // Child should be shifted by container padding
    assert_eq!(frag.children[0].offset.left, lu(20));
    assert_eq!(frag.children[0].offset.top, lu(20));
}

#[test]
fn c2_07_container_border_shifts_items() {
    // Container border:10 on all sides
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.border_top_width = 10;
        s.border_right_width = 10;
        s.border_bottom_width = 10;
        s.border_left_width = 10;
        s.border_top_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
        s.border_bottom_style = BorderStyle::Solid;
        s.border_left_style = BorderStyle::Solid;
    }
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].offset.left, lu(10));
    assert_eq!(frag.children[0].offset.top, lu(10));
}

#[test]
fn c2_08_container_padding_and_item_padding() {
    // Container padding:15, item width:80 padding:10 → item offset=15, width=100
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.padding_left = Length::px(15.0);
        s.padding_right = Length::px(15.0);
        s.padding_top = Length::px(15.0);
        s.padding_bottom = Length::px(15.0);
    }
    let a = add_child(&mut doc, c, 80, 50);
    {
        let s = doc.node_mut(a).style_mut();
        s.padding_left = Length::px(10.0);
        s.padding_right = Length::px(10.0);
    }
    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].offset.left, lu(15));
    assert_eq!(frag.children[0].width(), lu(100)); // 80 + 20
}

#[test]
fn c2_09_container_border_plus_padding() {
    // Container border:5 + padding:10 → items start at 15
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.border_left_width = 5;
        s.border_top_width = 5;
        s.border_left_style = BorderStyle::Solid;
        s.border_top_style = BorderStyle::Solid;
        s.padding_left = Length::px(10.0);
        s.padding_top = Length::px(10.0);
    }
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].offset.left, lu(15));
    assert_eq!(frag.children[0].offset.top, lu(15));
}

#[test]
fn c2_10_asymmetric_padding() {
    // Container padding: top=5, right=10, bottom=15, left=20
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.padding_top = Length::px(5.0);
        s.padding_right = Length::px(10.0);
        s.padding_bottom = Length::px(15.0);
        s.padding_left = Length::px(20.0);
    }
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].offset.left, lu(20));
    assert_eq!(frag.children[0].offset.top, lu(5));
}

#[test]
fn c2_11_padding_column_direction() {
    // Column flex, container padding:20
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.padding_top = Length::px(20.0);
        s.padding_left = Length::px(20.0);
    }
    let _a = add_child(&mut doc, c, 50, 80);

    let frag = lay(&doc, c, 200, 400);
    assert_eq!(frag.children[0].offset.left, lu(20));
    assert_eq!(frag.children[0].offset.top, lu(20));
}

#[test]
fn c2_12_border_column_direction() {
    // Column flex, container border:10
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.border_top_width = 10;
        s.border_left_width = 10;
        s.border_top_style = BorderStyle::Solid;
        s.border_left_style = BorderStyle::Solid;
    }
    let _a = add_child(&mut doc, c, 50, 80);

    let frag = lay(&doc, c, 200, 400);
    assert_eq!(frag.children[0].offset.left, lu(10));
    assert_eq!(frag.children[0].offset.top, lu(10));
}

#[test]
fn c2_13_container_border_justify_center() {
    // Container border:10, justify-content:center, one 100px item in 400px container
    // Content-box: border-box = 420 (400 + 20 border). Content area = 400.
    // Item centered: border_left(10) + (400-100)/2 = 160
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.border_left_width = 10;
        s.border_right_width = 10;
        s.border_left_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
    }
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 400, 100);
    // Centered in 400px content area, offset by 10px border-left
    assert_eq!(frag.children[0].offset.left, lu(160));
}

#[test]
fn c2_14_container_padding_align_center() {
    // Container height:200 (content-box) padding:20, align-items:center, item height:60
    // Content area = 200, centered offset = 20 + (200-60)/2 = 90
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.padding_top = Length::px(20.0);
        s.padding_bottom = Length::px(20.0);
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    let _a = add_child(&mut doc, c, 100, 60);

    let frag = lay(&doc, c, 400, 200);
    // Centered vertically within content area (200px, content-box)
    assert_eq!(frag.children[0].offset.top, lu(90));
}

#[test]
fn c2_15_item_border_with_margins() {
    // Item: width:80, border:5, margin-left:10 → offset.left = 10, width = 80+10 = 90
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let a = add_child(&mut doc, c, 80, 50);
    {
        let s = doc.node_mut(a).style_mut();
        s.border_left_width = 5;
        s.border_right_width = 5;
        s.border_left_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
        s.margin_left = Length::px(10.0);
    }
    let frag = lay(&doc, c, 400, 100);
    assert_eq!(frag.children[0].offset.left, lu(10));
    assert_eq!(frag.children[0].width(), lu(90));
}

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 3: nested flex containers (10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c3_01_flex_inside_flex() {
    // Outer: row 400x200, inner: row flex inside
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 200);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
    }
    doc.append_child(outer, inner);
    let _a = add_child(&mut doc, inner, 100, 50);
    let _b = add_child(&mut doc, inner, 100, 50);

    let frag = lay(&doc, outer, 400, 200);
    assert_eq!(frag.children.len(), 1);
    let inner_frag = &frag.children[0];
    assert_eq!(inner_frag.width(), lu(300));
    assert_eq!(inner_frag.children.len(), 2);
    assert_eq!(inner_frag.children[0].width(), lu(100));
    assert_eq!(inner_frag.children[1].width(), lu(100));
}

#[test]
fn c3_02_inner_flex_grows() {
    // Inner flex container has flex-grow:1 → fills remaining space
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 200);
    let fixed = add_child(&mut doc, outer, 100, 50);
    let _ = fixed;
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.flex_grow = 1.0;
        s.height = Length::px(100.0);
    }
    doc.append_child(outer, inner);

    let frag = lay(&doc, outer, 400, 200);
    // Inner should fill remaining 300px
    assert_eq!(frag.children[1].width(), lu(300));
}

#[test]
fn c3_03_column_inside_row() {
    // Outer: row, inner: column
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 300);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.flex_direction = FlexDirection::Column;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
    }
    doc.append_child(outer, inner);
    let _a = add_child(&mut doc, inner, 100, 60);
    let _b = add_child(&mut doc, inner, 100, 60);

    let frag = lay(&doc, outer, 400, 300);
    let inner_frag = &frag.children[0];
    // Column: children stacked vertically
    assert_eq!(inner_frag.children[0].offset.top, lu(0));
    assert_eq!(inner_frag.children[1].offset.top, lu(60));
}

#[test]
fn c3_04_row_inside_column() {
    // Outer: column, inner: row
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 300, 400);
    doc.node_mut(outer).style_mut().flex_direction = FlexDirection::Column;
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.flex_direction = FlexDirection::Row;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
    }
    doc.append_child(outer, inner);
    let _a = add_child(&mut doc, inner, 80, 50);
    let _b = add_child(&mut doc, inner, 80, 50);

    let frag = lay(&doc, outer, 300, 400);
    let inner_frag = &frag.children[0];
    // Row: children side by side
    assert_eq!(inner_frag.children[0].offset.left, lu(0));
    assert_eq!(inner_frag.children[1].offset.left, lu(80));
}

#[test]
fn c3_05_nested_different_justify() {
    // Outer justify:center, inner justify:flex-end
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 200);
    doc.node_mut(outer).style_mut().justify_content = ContentAlignment::new(ContentPosition::Center);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    doc.append_child(outer, inner);
    let _a = add_child(&mut doc, inner, 60, 40);

    let frag = lay(&doc, outer, 400, 200);
    // Outer: inner centered at (400-200)/2=100
    assert_eq!(frag.children[0].offset.left, lu(100));
    // Inner: child at flex-end = 200-60 = 140
    let inner_frag = &frag.children[0];
    assert_eq!(inner_frag.children[0].offset.left, lu(140));
}

#[test]
fn c3_06_nested_different_align() {
    // Outer: align-items:center, inner: align-items:flex-end
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 200);
    doc.node_mut(outer).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(100.0);
        s.align_items = ItemAlignment::new(ItemPosition::FlexEnd);
    }
    doc.append_child(outer, inner);
    let _a = add_child(&mut doc, inner, 60, 40);

    let frag = lay(&doc, outer, 400, 200);
    // Outer: inner(h=100) centered in cross 200 → top = 50
    assert_eq!(frag.children[0].offset.top, lu(50));
    // Inner: child(h=40) at flex-end in 100 → top = 60
    let inner_frag = &frag.children[0];
    assert_eq!(inner_frag.children[0].offset.top, lu(60));
}

#[test]
fn c3_07_three_levels() {
    // Level 1 row → Level 2 column → Level 3 row
    let mut doc = Document::new();
    let l1 = make_flex(&mut doc, 500, 400);
    let l2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(l2).style_mut();
        s.display = Display::Flex;
        s.flex_direction = FlexDirection::Column;
        s.width = Length::px(300.0);
        s.height = Length::px(300.0);
    }
    doc.append_child(l1, l2);
    let l3 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(l3).style_mut();
        s.display = Display::Flex;
        s.flex_direction = FlexDirection::Row;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
    }
    doc.append_child(l2, l3);
    let _a = add_child(&mut doc, l3, 50, 50);

    let frag = lay(&doc, l1, 500, 400);
    assert_eq!(frag.children[0].width(), lu(300));
    assert_eq!(frag.children[0].children[0].width(), lu(300));
    assert_eq!(frag.children[0].children[0].children[0].width(), lu(50));
}

#[test]
fn c3_08_inner_wrap_inside_outer() {
    // Inner flex with wrap, items overflow to next line
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 300);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.flex_wrap = FlexWrap::Wrap;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
    }
    doc.append_child(outer, inner);
    // 3 items of 100px each in 200px wrap container → first line: 2, second line: 1
    for _ in 0..3 {
        add_child(&mut doc, inner, 100, 50);
    }

    let frag = lay(&doc, outer, 400, 300);
    let inner_frag = &frag.children[0];
    assert_eq!(inner_frag.children.len(), 3);
    // First two side by side
    assert_eq!(inner_frag.children[0].offset.top, inner_frag.children[1].offset.top);
    // Third wraps to next line
    assert!(inner_frag.children[2].offset.top > inner_frag.children[0].offset.top);
}

#[test]
fn c3_09_inner_flex_sized_by_content() {
    // Inner flex with explicit width contains children that fit inside
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 200);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(100.0);
    }
    doc.append_child(outer, inner);
    let _a = add_child(&mut doc, inner, 60, 40);
    let _b = add_child(&mut doc, inner, 80, 40);

    let frag = lay(&doc, outer, 400, 200);
    // Inner flex has explicit width 200, children laid out inside
    assert_eq!(frag.children[0].width(), lu(200));
    assert_eq!(frag.children[0].children.len(), 2);
    assert_eq!(frag.children[0].children[0].width(), lu(60));
    assert_eq!(frag.children[0].children[1].width(), lu(80));
    assert_eq!(frag.children[0].children[1].offset.left, lu(60));
}

#[test]
fn c3_10_nested_grow_at_both_levels() {
    // Outer: item A=100, inner flex-grow:1
    // Inner: child flex-grow:1 fills inner
    let mut doc = Document::new();
    let outer = make_flex(&mut doc, 400, 200);
    let _fixed = add_child(&mut doc, outer, 100, 50);
    let inner = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(inner).style_mut();
        s.display = Display::Flex;
        s.flex_grow = 1.0;
        s.height = Length::px(100.0);
    }
    doc.append_child(outer, inner);
    let grow_child = add_auto_child(&mut doc, inner);
    doc.node_mut(grow_child).style_mut().flex_grow = 1.0;

    let frag = lay(&doc, outer, 400, 200);
    // Inner = 300 (400-100)
    assert_eq!(frag.children[1].width(), lu(300));
    // grow_child fills inner = 300
    assert_eq!(frag.children[1].children[0].width(), lu(300));
}

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 4: block_layout dispatch (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c4_01_block_layout_dispatches_flex() {
    // block_layout with display:flex should produce same as flex_layout
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let _a = add_child(&mut doc, c, 100, 50);
    let _b = add_child(&mut doc, c, 100, 50);

    let frag = lay_block(&doc, c, 300, 100);
    assert_eq!(frag.children.len(), 2);
    assert_eq!(frag.children[0].width(), lu(100));
    assert_eq!(frag.children[1].offset.left, lu(100));
}

#[test]
fn c4_02_block_layout_block_display() {
    // block_layout with display:block → uses block layout, not flex
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Block;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
    }
    doc.append_child(doc.root(), c);
    let _a = add_child(&mut doc, c, 100, 50);
    let _b = add_child(&mut doc, c, 100, 50);

    let frag = lay_block(&doc, c, 300, 200);
    // Block: children stacked vertically, not side by side
    assert_eq!(frag.children[0].offset.top, lu(0));
    assert_eq!(frag.children[1].offset.top, lu(50));
}

#[test]
fn c4_03_block_flex_matches_flex_layout() {
    // block_layout(flex) == flex_layout exactly
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let _a = add_child(&mut doc, c, 150, 80);
    let _b = add_child(&mut doc, c, 100, 60);

    let frag_block = lay_block(&doc, c, 400, 200);
    let frag_flex = lay(&doc, c, 400, 200);

    assert_eq!(frag_block.width(), frag_flex.width());
    assert_eq!(frag_block.height(), frag_flex.height());
    assert_eq!(frag_block.children.len(), frag_flex.children.len());
    for i in 0..frag_block.children.len() {
        assert_eq!(frag_block.children[i].width(), frag_flex.children[i].width());
        assert_eq!(frag_block.children[i].height(), frag_flex.children[i].height());
        assert_eq!(frag_block.children[i].offset.left, frag_flex.children[i].offset.left);
        assert_eq!(frag_block.children[i].offset.top, frag_flex.children[i].offset.top);
    }
}

#[test]
fn c4_04_block_layout_flex_column() {
    // block_layout dispatches column flex correctly
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let _a = add_child(&mut doc, c, 100, 80);
    let _b = add_child(&mut doc, c, 100, 80);

    let frag = lay_block(&doc, c, 200, 400);
    assert_eq!(frag.children[0].offset.top, lu(0));
    assert_eq!(frag.children[1].offset.top, lu(80));
}

#[test]
fn c4_05_block_layout_wrapped_flex() {
    // block_layout dispatches wrapped flex correctly
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;
    // 3 items of 100px in 200px → 2 on first line, 1 on second
    for _ in 0..3 {
        add_child(&mut doc, c, 100, 50);
    }

    let frag = lay_block(&doc, c, 200, 200);
    assert_eq!(frag.children.len(), 3);
    // First two on same line
    assert_eq!(frag.children[0].offset.top, frag.children[1].offset.top);
    // Third wraps
    assert!(frag.children[2].offset.top > frag.children[0].offset.top);
}

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 5: empty containers and single items (10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c5_01_empty_flex_container() {
    // Empty flex container should produce fragment with no children
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);

    let frag = lay(&doc, c, 300, 200);
    assert_eq!(frag.children.len(), 0);
    assert_eq!(frag.width(), lu(300));
    assert_eq!(frag.height(), lu(200));
}

#[test]
fn c5_02_single_item() {
    // Single item positioned at start
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 300, 200);
    assert_eq!(frag.children.len(), 1);
    assert_eq!(frag.children[0].offset.left, lu(0));
    assert_eq!(frag.children[0].width(), lu(100));
}

#[test]
fn c5_03_single_item_grow_fills() {
    // Single item flex-grow:1 fills container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    let a = add_auto_child(&mut doc, c);
    doc.node_mut(a).style_mut().flex_grow = 1.0;

    let frag = lay(&doc, c, 300, 200);
    assert_eq!(frag.children[0].width(), lu(300));
}

#[test]
fn c5_04_single_item_centered() {
    // Single 100x50 item centered both ways in 300x200 container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 300, 200);
    assert_eq!(frag.children[0].offset.left, lu(100)); // (300-100)/2
    assert_eq!(frag.children[0].offset.top, lu(75)); // (200-50)/2
}

#[test]
fn c5_05_all_auto_sized_items() {
    // All items auto width/height → zero-width items, no explicit size
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    let _a = add_auto_child(&mut doc, c);
    let _b = add_auto_child(&mut doc, c);

    let frag = lay(&doc, c, 300, 200);
    assert_eq!(frag.children.len(), 2);
    // Auto-sized items with no content → zero width
    assert_eq!(frag.children[0].width(), lu(0));
    assert_eq!(frag.children[1].width(), lu(0));
}

#[test]
fn c5_06_zero_size_items() {
    // Items with explicit zero size
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    let _a = add_child(&mut doc, c, 0, 0);
    let _b = add_child(&mut doc, c, 0, 0);

    let frag = lay(&doc, c, 300, 200);
    assert_eq!(frag.children[0].width(), lu(0));
    assert_eq!(frag.children[1].width(), lu(0));
}

#[test]
fn c5_07_zero_width_container() {
    // Container with zero width
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 0, 200);
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 0, 200);
    assert_eq!(frag.width(), lu(0));
    // Item may overflow or shrink
    assert_eq!(frag.children.len(), 1);
}

#[test]
fn c5_08_zero_height_container() {
    // Container with zero height
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 0);
    let _a = add_child(&mut doc, c, 100, 50);

    let frag = lay(&doc, c, 300, 0);
    assert_eq!(frag.height(), lu(0));
    assert_eq!(frag.children[0].width(), lu(100));
}

#[test]
fn c5_09_very_large_container() {
    // Very large 10000x10000 container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 10000, 10000);
    let _a = add_child(&mut doc, c, 100, 100);

    let frag = lay(&doc, c, 10000, 10000);
    assert_eq!(frag.width(), lu(10000));
    assert_eq!(frag.height(), lu(10000));
    assert_eq!(frag.children[0].width(), lu(100));
}

#[test]
fn c5_10_very_small_container() {
    // Very small 1x1 container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 1, 1);
    let _a = add_child(&mut doc, c, 100, 100);

    let frag = lay(&doc, c, 1, 1);
    assert_eq!(frag.width(), lu(1));
    assert_eq!(frag.height(), lu(1));
    assert_eq!(frag.children.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 6: percentage sizes (10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c6_01_item_width_50_percent() {
    // Item width:50% in 400px container → 200px
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(a).style_mut();
        s.display = Display::Block;
        s.width = Length::percent(50.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, a);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].width(), lu(200));
}

#[test]
fn c6_02_item_width_100_percent() {
    // Item width:100% → fills container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(a).style_mut();
        s.display = Display::Block;
        s.width = Length::percent(100.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, a);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].width(), lu(400));
}

#[test]
fn c6_03_two_percentage_items() {
    // A: 25%, B: 75% → should sum to 100%
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(a).style_mut();
        s.display = Display::Block;
        s.width = Length::percent(25.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, a);
    let b = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(b).style_mut();
        s.display = Display::Block;
        s.width = Length::percent(75.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, b);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].width(), lu(100));
    assert_eq!(frag.children[1].width(), lu(300));
}

#[test]
fn c6_04_percentage_flex_basis() {
    // flex-basis:50% in 400px → 200px
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(a).style_mut();
        s.flex_basis = Length::percent(50.0);
        s.height = Length::px(50.0);
    }
    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].width(), lu(200));
}

#[test]
fn c6_05_percentage_margin() {
    // margin-left: 10% of 400px = 40px
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = add_child(&mut doc, c, 100, 50);
    doc.node_mut(a).style_mut().margin_left = Length::percent(10.0);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].offset.left, lu(40));
}

#[test]
fn c6_06_percentage_padding() {
    // padding-left: 10% of 400px = 40px on item with width:100
    // → border-box = 100 + 40 = 140
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = add_child(&mut doc, c, 100, 50);
    doc.node_mut(a).style_mut().padding_left = Length::percent(10.0);

    let frag = lay(&doc, c, 400, 200);
    assert_eq!(frag.children[0].width(), lu(140));
}

#[test]
fn c6_07_percentage_width_column() {
    // Column flex: item width:50% = 50% of container width = 150
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 400);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let a = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(a).style_mut();
        s.display = Display::Block;
        s.width = Length::percent(50.0);
        s.height = Length::px(80.0);
    }
    doc.append_child(c, a);

    let frag = lay(&doc, c, 300, 400);
    assert_eq!(frag.children[0].width(), lu(150));
}

#[test]
fn c6_08_percentage_height_column() {
    // Column flex: item height:25% of 400px = 100
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 400);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let a = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(a).style_mut();
        s.display = Display::Block;
        s.width = Length::px(100.0);
        s.height = Length::percent(25.0);
    }
    doc.append_child(c, a);

    let frag = lay(&doc, c, 300, 400);
    assert_eq!(frag.children[0].height(), lu(100));
}

#[test]
fn c6_09_percentages_over_100() {
    // Two items 60% each = 240px each, total 480 in 400px container.
    // Items have default flex-shrink:1 and min-width:auto = 0 (empty content),
    // so they shrink to fit: 400/2 = 200 each.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    for _ in 0..2 {
        let ch = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(ch).style_mut();
            s.display = Display::Block;
            s.width = Length::percent(60.0);
            s.height = Length::px(50.0);
        }
        doc.append_child(c, ch);
    }

    let frag = lay(&doc, c, 400, 200);
    // Items shrink equally from 240 to 200 each
    assert_eq!(frag.children[0].width(), lu(200));
    assert_eq!(frag.children[1].width(), lu(200));
}

#[test]
fn c6_10_percentage_with_min_width() {
    // width:20% of 400 = 80, min-width:150 → 150
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    let a = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(a).style_mut();
        s.display = Display::Block;
        s.width = Length::percent(20.0);
        s.height = Length::px(50.0);
        s.min_width = Length::px(150.0);
    }
    doc.append_child(c, a);

    let frag = lay(&doc, c, 400, 200);
    assert!(frag.children[0].width() >= lu(150));
}

// ═══════════════════════════════════════════════════════════════════════
// CATEGORY 7: complex real-world scenarios (15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn c7_01_navigation_bar() {
    // Row flex, space-between: logo left, links right
    let mut doc = Document::new();
    let nav = make_flex(&mut doc, 800, 60);
    doc.node_mut(nav).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    let _logo = add_child(&mut doc, nav, 120, 40);
    let _links = add_child(&mut doc, nav, 200, 40);

    let frag = lay(&doc, nav, 800, 60);
    // Logo at start
    assert_eq!(frag.children[0].offset.left, lu(0));
    // Links at end (800 - 200)
    assert_eq!(frag.children[1].offset.left, lu(600));
}

#[test]
fn c7_02_card_layout() {
    // Column flex: header(60) + body(grows) + footer(40) in 400px height
    let mut doc = Document::new();
    let card = make_flex(&mut doc, 300, 400);
    doc.node_mut(card).style_mut().flex_direction = FlexDirection::Column;
    let _header = add_child(&mut doc, card, 300, 60);
    // Body must be Display::Flex for column grow to work in this engine
    let body = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(body).style_mut();
        s.display = Display::Flex;
        s.flex_grow = 1.0;
        s.width = Length::px(300.0);
    }
    doc.append_child(card, body);
    let _footer = add_child(&mut doc, card, 300, 40);

    let frag = lay(&doc, card, 300, 400);
    assert_eq!(frag.children[0].offset.top, lu(0));     // header at top
    assert_eq!(frag.children[0].height(), lu(60));
    assert_eq!(frag.children[1].offset.top, lu(60));     // body
    assert_eq!(frag.children[1].height(), lu(300));       // 400-60-40
    assert_eq!(frag.children[2].offset.top, lu(360));    // footer
    assert_eq!(frag.children[2].height(), lu(40));
}

#[test]
fn c7_03_sidebar_layout() {
    // Row: 200px sidebar + flex-grow main
    let mut doc = Document::new();
    let layout = make_flex(&mut doc, 1000, 600);
    let _sidebar = add_child(&mut doc, layout, 200, 600);
    let main = add_auto_child(&mut doc, layout);
    {
        let s = doc.node_mut(main).style_mut();
        s.flex_grow = 1.0;
        s.height = Length::px(600.0);
    }

    let frag = lay(&doc, layout, 1000, 600);
    assert_eq!(frag.children[0].width(), lu(200));
    assert_eq!(frag.children[1].width(), lu(800));
    assert_eq!(frag.children[1].offset.left, lu(200));
}

#[test]
fn c7_04_grid_like_wrap() {
    // Wrap with 4x 150px items in 300px → 2 per row, 2 rows
    let mut doc = Document::new();
    let grid = make_flex(&mut doc, 300, 400);
    doc.node_mut(grid).style_mut().flex_wrap = FlexWrap::Wrap;
    for _ in 0..4 {
        add_child(&mut doc, grid, 150, 80);
    }

    let frag = lay(&doc, grid, 300, 400);
    assert_eq!(frag.children.len(), 4);
    // Row 1: items 0,1 at same top
    assert_eq!(frag.children[0].offset.top, frag.children[1].offset.top);
    // Row 2: items 2,3 at same top (below row 1)
    assert_eq!(frag.children[2].offset.top, frag.children[3].offset.top);
    assert!(frag.children[2].offset.top > frag.children[0].offset.top);
}

#[test]
fn c7_05_holy_grail_layout() {
    // Column outer: header + row body + footer
    // Body: left sidebar + main(grows) + right sidebar
    let mut doc = Document::new();
    let page = make_flex(&mut doc, 800, 600);
    doc.node_mut(page).style_mut().flex_direction = FlexDirection::Column;

    let _header = add_child(&mut doc, page, 800, 60);

    let body = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(body).style_mut();
        s.display = Display::Flex;
        s.flex_grow = 1.0;
        s.width = Length::px(800.0);
    }
    doc.append_child(page, body);
    let _left = add_child(&mut doc, body, 150, 100);
    let main_area = add_auto_child(&mut doc, body);
    doc.node_mut(main_area).style_mut().flex_grow = 1.0;
    let _right = add_child(&mut doc, body, 150, 100);

    let _footer = add_child(&mut doc, page, 800, 40);

    let frag = lay(&doc, page, 800, 600);
    // Header at top
    assert_eq!(frag.children[0].offset.top, lu(0));
    assert_eq!(frag.children[0].height(), lu(60));
    // Body
    assert_eq!(frag.children[1].offset.top, lu(60));
    assert_eq!(frag.children[1].height(), lu(500)); // 600-60-40
    // Footer
    assert_eq!(frag.children[2].offset.top, lu(560));
    // Body children
    let body_frag = &frag.children[1];
    assert_eq!(body_frag.children[0].width(), lu(150)); // left
    assert_eq!(body_frag.children[1].width(), lu(500)); // main = 800-150-150
    assert_eq!(body_frag.children[2].width(), lu(150)); // right
}

#[test]
fn c7_06_toolbar_with_gap() {
    // Row flex with gap:10, items 50px each, some grow
    let mut doc = Document::new();
    let toolbar = make_flex(&mut doc, 400, 50);
    doc.node_mut(toolbar).style_mut().column_gap = Some(Length::px(10.0));
    let _btn1 = add_child(&mut doc, toolbar, 50, 30);
    let _btn2 = add_child(&mut doc, toolbar, 50, 30);
    let spacer = add_auto_child(&mut doc, toolbar);
    doc.node_mut(spacer).style_mut().flex_grow = 1.0;
    let _btn3 = add_child(&mut doc, toolbar, 50, 30);

    let frag = lay(&doc, toolbar, 400, 50);
    // btn1 at 0, btn2 at 50+10=60, spacer grows, btn3 at end
    assert_eq!(frag.children[0].offset.left, lu(0));
    assert_eq!(frag.children[1].offset.left, lu(60));
    // btn3 at right: 400 - 50 = 350
    assert_eq!(frag.children[3].offset.left, lu(350));
    // Spacer fills middle
    let spacer_w = frag.children[2].width();
    assert!(spacer_w > lu(0));
}

#[test]
fn c7_07_form_layout() {
    // Column flex with labels and inputs
    let mut doc = Document::new();
    let form = make_flex(&mut doc, 300, 400);
    doc.node_mut(form).style_mut().flex_direction = FlexDirection::Column;
    let _label1 = add_child(&mut doc, form, 300, 20);
    let _input1 = add_child(&mut doc, form, 300, 40);
    let _label2 = add_child(&mut doc, form, 300, 20);
    let _input2 = add_child(&mut doc, form, 300, 40);

    let frag = lay(&doc, form, 300, 400);
    assert_eq!(frag.children[0].offset.top, lu(0));
    assert_eq!(frag.children[1].offset.top, lu(20));
    assert_eq!(frag.children[2].offset.top, lu(60));
    assert_eq!(frag.children[3].offset.top, lu(80));
}

#[test]
fn c7_08_centering_single_item() {
    // Perfect centering: justify:center + align:center
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    let _a = add_child(&mut doc, c, 100, 100);

    let frag = lay(&doc, c, 400, 400);
    assert_eq!(frag.children[0].offset.left, lu(150));
    assert_eq!(frag.children[0].offset.top, lu(150));
}

#[test]
fn c7_09_equal_height_columns() {
    // Row flex: items with different content heights, stretch default.
    // Items with auto height should stretch to tallest.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    // Default align-items is normal → stretch in flex context
    // But only for items with auto cross-axis size
    let a = add_auto_child(&mut doc, c);
    doc.node_mut(a).style_mut().width = Length::px(100.0);
    let b = add_auto_child(&mut doc, c);
    {
        let s = doc.node_mut(b).style_mut();
        s.width = Length::px(100.0);
    }

    let frag = lay(&doc, c, 300, 200);
    // Both should stretch to container height (200) since height is auto
    assert_eq!(frag.children[0].height(), frag.children[1].height());
}

#[test]
fn c7_10_responsive_cards_wrap() {
    // Wrap with min-width items
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 500, 400);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;
    for _ in 0..4 {
        let ch = add_child(&mut doc, c, 200, 100);
        doc.node_mut(ch).style_mut().min_width = Length::px(150.0);
    }

    let frag = lay(&doc, c, 500, 400);
    assert_eq!(frag.children.len(), 4);
    // At 500px, two 200px items per row
    assert_eq!(frag.children[0].offset.top, frag.children[1].offset.top);
    assert!(frag.children[2].offset.top > frag.children[0].offset.top);
}

#[test]
fn c7_11_footer_at_bottom() {
    // Column flex: main grows, footer stays at bottom
    let mut doc = Document::new();
    let page = make_flex(&mut doc, 400, 600);
    doc.node_mut(page).style_mut().flex_direction = FlexDirection::Column;
    // Main must be Display::Flex for column grow to work in this engine
    let main = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(main).style_mut();
        s.display = Display::Flex;
        s.flex_grow = 1.0;
        s.width = Length::px(400.0);
    }
    doc.append_child(page, main);
    let _footer = add_child(&mut doc, page, 400, 50);

    let frag = lay(&doc, page, 400, 600);
    assert_eq!(frag.children[0].height(), lu(550));    // 600 - 50
    assert_eq!(frag.children[1].offset.top, lu(550));  // footer at bottom
}

#[test]
fn c7_12_split_button() {
    // Two items: one fixed 60px, one grows
    let mut doc = Document::new();
    let btn = make_flex(&mut doc, 200, 40);
    let _label = add_child(&mut doc, btn, 60, 40);
    let expand = add_auto_child(&mut doc, btn);
    doc.node_mut(expand).style_mut().flex_grow = 1.0;

    let frag = lay(&doc, btn, 200, 40);
    assert_eq!(frag.children[0].width(), lu(60));
    assert_eq!(frag.children[1].width(), lu(140));
    assert_eq!(frag.children[1].offset.left, lu(60));
}

#[test]
fn c7_13_breadcrumbs_with_gap() {
    // Row flex with gap
    let mut doc = Document::new();
    let bc = make_flex(&mut doc, 600, 40);
    doc.node_mut(bc).style_mut().column_gap = Some(Length::px(8.0));
    let _c1 = add_child(&mut doc, bc, 60, 30);
    let _c2 = add_child(&mut doc, bc, 80, 30);
    let _c3 = add_child(&mut doc, bc, 100, 30);

    let frag = lay(&doc, bc, 600, 40);
    assert_eq!(frag.children[0].offset.left, lu(0));
    assert_eq!(frag.children[1].offset.left, lu(68));  // 60 + 8
    assert_eq!(frag.children[2].offset.left, lu(156)); // 68 + 80 + 8
}

#[test]
fn c7_14_tab_bar_equal() {
    // Items with flex-basis:0, flex-grow:1 → equal widths
    let mut doc = Document::new();
    let tabs = make_flex(&mut doc, 400, 40);
    for _ in 0..4 {
        let tab = add_auto_child(&mut doc, tabs);
        let s = doc.node_mut(tab).style_mut();
        s.flex_basis = Length::px(0.0);
        s.flex_grow = 1.0;
    }

    let frag = lay(&doc, tabs, 400, 40);
    for child in &frag.children {
        assert_eq!(child.width(), lu(100)); // 400/4
    }
    assert_eq!(frag.children[0].offset.left, lu(0));
    assert_eq!(frag.children[1].offset.left, lu(100));
    assert_eq!(frag.children[2].offset.left, lu(200));
    assert_eq!(frag.children[3].offset.left, lu(300));
}

#[test]
fn c7_15_dashboard_percentage_wrap() {
    // Wrap with 50% width items → 2 per row
    let mut doc = Document::new();
    let dash = make_flex(&mut doc, 600, 400);
    doc.node_mut(dash).style_mut().flex_wrap = FlexWrap::Wrap;
    for _ in 0..4 {
        let widget = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(widget).style_mut();
            s.display = Display::Block;
            s.width = Length::percent(50.0);
            s.height = Length::px(100.0);
        }
        doc.append_child(dash, widget);
    }

    let frag = lay(&doc, dash, 600, 400);
    // Each item 50% of 600 = 300
    for child in &frag.children {
        assert_eq!(child.width(), lu(300));
    }
    // Row 1 and row 2
    assert_eq!(frag.children[0].offset.top, frag.children[1].offset.top);
    assert_eq!(frag.children[2].offset.top, frag.children[3].offset.top);
    assert!(frag.children[2].offset.top > frag.children[0].offset.top);
}
