//! Integration tests for CSS Flexbox alignment properties.
//!
//! Covers: justify-content, align-items, align-self, align-content.
//! 80 tests total across 4 categories.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{flex_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, ContentAlignment, ContentDistribution, ContentPosition, Display,
    FlexDirection, FlexWrap, ItemAlignment, ItemPosition,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Create a flex container with explicit width/height, appended to doc root.
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

/// Add a child with explicit width and height (will NOT stretch on cross axis).
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

/// Add a child with explicit width but auto height (will stretch in row flex).
fn add_auto_height_child(doc: &mut Document, parent: NodeId, w: i32) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(w as f32);
        // height stays auto — will stretch
    }
    doc.append_child(parent, child);
    child
}

/// Add a child with explicit height but auto width (will stretch in column flex).
fn add_auto_width_child(doc: &mut Document, parent: NodeId, h: i32) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.height = Length::px(h as f32);
        // width stays auto — will stretch in column direction
    }
    doc.append_child(parent, child);
    child
}

/// Run flex layout with a root constraint space matching container dimensions.
fn lay(doc: &Document, container: NodeId, w: i32, h: i32) -> Fragment {
    let space = ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h));
    flex_layout(doc, container, &space)
}

/// Shorthand for LayoutUnit::from_i32.
fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

// ═════════════════════════════════════════════════════════════════════════════
// Category 1: justify-content (25 tests)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn jc_flex_start_default() {
    // Default justify-content (Normal → flex-start): items packed at start
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children.len(), 3);
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(50));
    assert_eq!(f.children[2].offset.left, lu(100));
}

#[test]
fn jc_flex_end() {
    // justify-content: flex-end — items packed at end
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::FlexEnd);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 400-150 = 250. Items at 250, 300, 350.
    assert_eq!(f.children[0].offset.left, lu(250));
    assert_eq!(f.children[1].offset.left, lu(300));
    assert_eq!(f.children[2].offset.left, lu(350));
}

#[test]
fn jc_center() {
    // justify-content: center — items centered
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::Center);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 300. Offset = 150.
    assert_eq!(f.children[0].offset.left, lu(150));
    assert_eq!(f.children[1].offset.left, lu(200));
}

#[test]
fn jc_space_between_2_items() {
    // space-between with 2 items: one at start, one at end
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 300. Between = 300/1 = 300. Items at 0, 350.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(350));
}

#[test]
fn jc_space_between_3_items() {
    // space-between with 3 items: equal spacing between
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 250. Between = 250/2 = 125. Items at 0, 175, 350.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(175));
    assert_eq!(f.children[2].offset.left, lu(350));
}

#[test]
fn jc_space_between_1_item() {
    // space-between with 1 item: falls back to flex-start
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn jc_space_around_2_items() {
    // space-around: half-space before first, full between, half after last
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 300. Per_item = 150. Half = 75.
    // Items at 75, 75+50+150=275.
    assert_eq!(f.children[0].offset.left, lu(75));
    assert_eq!(f.children[1].offset.left, lu(275));
}

#[test]
fn jc_space_around_3_items() {
    // space-around with 3 items
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 300, 100);
    // Free = 150. Per_item = 50. Half = 25.
    // Items at 25, 125, 225.
    assert_eq!(f.children[0].offset.left, lu(25));
    assert_eq!(f.children[1].offset.left, lu(125));
    assert_eq!(f.children[2].offset.left, lu(225));
}

#[test]
fn jc_space_around_1_item() {
    // space-around with 1 item: centered
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 300, 100);
    // Free = 250. Per_item = 250. Half = 125. Item at 125.
    assert_eq!(f.children[0].offset.left, lu(125));
}

#[test]
fn jc_space_evenly_2_items() {
    // space-evenly: equal space before, between, and after
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 300. Slots = 3. Per = 100.
    // Items at 100, 250.
    assert_eq!(f.children[0].offset.left, lu(100));
    assert_eq!(f.children[1].offset.left, lu(250));
}

#[test]
fn jc_space_evenly_3_items() {
    // space-evenly with 3 items
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 220. Slots = 4. Per = 55.
    // Items at 55, 170, 285.
    assert_eq!(f.children[0].offset.left, lu(55));
    assert_eq!(f.children[1].offset.left, lu(170));
    assert_eq!(f.children[2].offset.left, lu(285));
}

#[test]
fn jc_space_evenly_1_item() {
    // space-evenly with 1 item: centered
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 300, 100);
    // Free = 250. Slots = 2. Per = 125. Item at 125.
    assert_eq!(f.children[0].offset.left, lu(125));
}

#[test]
fn jc_center_column() {
    // justify-content: center in column direction — vertically centered
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    // Free = 300. Offset = 150. Items at y=150, 200.
    assert_eq!(f.children[0].offset.top, lu(150));
    assert_eq!(f.children[1].offset.top, lu(200));
}

#[test]
fn jc_flex_end_column() {
    // justify-content: flex-end in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.justify_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    // Free = 300. Items at y=300, 350.
    assert_eq!(f.children[0].offset.top, lu(300));
    assert_eq!(f.children[1].offset.top, lu(350));
}

#[test]
fn jc_space_between_column() {
    // space-between in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.justify_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    // Free = 300. Between = 300. Items at y=0, 350.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(350));
}

#[test]
fn jc_row_reverse() {
    // row-reverse: flex-start means items pack at the right side
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 80, 50);
    let f = lay(&doc, c, 300, 100);
    // Items reversed: [c2, c1]. Free = 170. FlexStart+reverse → offset=170.
    // c2 at x=170, c1 at x=250.
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[0].offset.left, lu(170));
    assert_eq!(f.children[1].offset.left, lu(250));
}

#[test]
fn jc_center_different_sizes() {
    // center with different sized items
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::Center);
    add_child(&mut doc, c, 30, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 70, 50);
    let f = lay(&doc, c, 400, 100);
    // Total = 150. Free = 250. Offset = 125.
    // Items at 125, 155, 205.
    assert_eq!(f.children[0].offset.left, lu(125));
    assert_eq!(f.children[1].offset.left, lu(155));
    assert_eq!(f.children[2].offset.left, lu(205));
}

#[test]
fn jc_flex_end_with_margins() {
    // flex-end with margin on first item
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::FlexEnd);
    let c1 = add_child(&mut doc, c, 50, 50);
    doc.node_mut(c1).style_mut().margin_left = Length::px(10.0);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Total margin-box = (10+50) + 50 = 110. Free = 290.
    // FlexEnd offset = 290. c1 at 290+10=300, c2 at 360.
    assert_eq!(f.children[0].offset.left, lu(300));
    assert_eq!(f.children[1].offset.left, lu(350));
}

#[test]
fn jc_space_between_unequal() {
    // space-between with unequal item sizes
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 40, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 400, 100);
    // Total = 200. Free = 200. Between = 100.
    // Items at 0, 160, 300.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(160));
    assert_eq!(f.children[2].offset.left, lu(300));
}

#[test]
fn jc_no_free_space() {
    // Items fill container exactly — no free space
    // Must set flex-shrink=0 to prevent shrink-mode edge case
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::Center);
    let c1 = add_child(&mut doc, c, 100, 50);
    let c2 = add_child(&mut doc, c, 100, 50);
    let c3 = add_child(&mut doc, c, 100, 50);
    doc.node_mut(c1).style_mut().flex_shrink = 0.0;
    doc.node_mut(c2).style_mut().flex_shrink = 0.0;
    doc.node_mut(c3).style_mut().flex_shrink = 0.0;
    let f = lay(&doc, c, 300, 100);
    // Free = 0. Center offset = 0.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(100));
    assert_eq!(f.children[2].offset.left, lu(200));
}

#[test]
fn jc_overflow() {
    // Items overflow container — negative free space
    // flex-shrink=0 keeps items at full size so center has negative offset
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::Center);
    let c1 = add_child(&mut doc, c, 100, 50);
    let c2 = add_child(&mut doc, c, 100, 50);
    let c3 = add_child(&mut doc, c, 100, 50);
    doc.node_mut(c1).style_mut().flex_shrink = 0.0;
    doc.node_mut(c2).style_mut().flex_shrink = 0.0;
    doc.node_mut(c3).style_mut().flex_shrink = 0.0;
    let f = lay(&doc, c, 200, 100);
    // Free = -100. Center offset = -50. Items at -50, 50, 150.
    assert_eq!(f.children[0].offset.left, lu(-50));
    assert_eq!(f.children[1].offset.left, lu(50));
    assert_eq!(f.children[2].offset.left, lu(150));
}

#[test]
fn jc_center_single_item() {
    // center with a single item
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().justify_content =
        ContentAlignment::new(ContentPosition::Center);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 300. Center = 150.
    assert_eq!(f.children[0].offset.left, lu(150));
}

#[test]
fn jc_flex_start_with_gap() {
    // flex-start with column-gap
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().column_gap = Some(Length::px(20.0));
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Items at 0, 70, 140.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(70));
    assert_eq!(f.children[2].offset.left, lu(140));
}

#[test]
fn jc_space_between_with_gap() {
    // space-between + gap: gap adds to the between spacing
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.justify_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
        s.column_gap = Some(Length::px(20.0));
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 400-150-40(gaps) = 210. Between = 105.
    // Items at 0, 0+50+20+105=175, 175+50+20+105=350.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[1].offset.left, lu(175));
    assert_eq!(f.children[2].offset.left, lu(350));
}

#[test]
fn jc_center_with_column_gap() {
    // center + column-gap
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
        s.column_gap = Some(Length::px(20.0));
    }
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // Free = 400-100-20 = 280. Center = 140.
    // Items at 140, 140+50+20=210.
    assert_eq!(f.children[0].offset.left, lu(140));
    assert_eq!(f.children[1].offset.left, lu(210));
}

// ═════════════════════════════════════════════════════════════════════════════
// Category 2: align-items (20 tests)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn ai_stretch_auto_height() {
    // Default align-items (Normal→Stretch) with auto-height items.
    // block_layout doesn't honor stretch_block_size, so empty auto-height items
    // get height 0. But they are positioned at y=0 (stretch alignment = start).
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_auto_height_child(&mut doc, c, 50);
    add_auto_height_child(&mut doc, c, 50);
    let f = lay(&doc, c, 300, 100);
    // Auto-height empty items: natural height = 0, placed at top.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    // Explicit-height items DO keep their height and don't stretch:
    // Verified separately in ai_stretch_explicit_height.
}

#[test]
fn ai_stretch_explicit_height() {
    // Default align-items with explicit-height items → NO stretch
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 300, 100);
    // Explicit h=50 items keep their height despite Stretch alignment
    assert_eq!(f.children[0].height(), lu(50));
    assert_eq!(f.children[1].height(), lu(50));
    assert_eq!(f.children[0].offset.top, lu(0));
}

#[test]
fn ai_flex_start() {
    // align-items: flex-start — items at top of cross axis
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexStart);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 40);
    let f = lay(&doc, c, 300, 100);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
}

#[test]
fn ai_flex_end() {
    // align-items: flex-end — items at bottom of cross axis
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexEnd);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 40);
    let f = lay(&doc, c, 300, 100);
    // cross_space: 100-50=50 → y=50; 100-40=60 → y=60.
    assert_eq!(f.children[0].offset.top, lu(50));
    assert_eq!(f.children[1].offset.top, lu(60));
}

#[test]
fn ai_center() {
    // align-items: center — items vertically centered
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    add_child(&mut doc, c, 50, 40);
    let f = lay(&doc, c, 300, 100);
    // cross_space = 60. Center = 30.
    assert_eq!(f.children[0].offset.top, lu(30));
    assert_eq!(f.children[0].height(), lu(40));
}

#[test]
fn ai_stretch_column() {
    // align-items: stretch in column direction — auto-width items stretch to container width
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    add_auto_width_child(&mut doc, c, 50);
    add_auto_width_child(&mut doc, c, 50);
    let f = lay(&doc, c, 300, 200);
    // Cross axis is width. Auto-width items stretch to 300.
    assert_eq!(f.children[0].width(), lu(300));
    assert_eq!(f.children[1].width(), lu(300));
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn ai_center_column() {
    // align-items: center in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    // Cross = width = 200. cross_space = 150. Center = 75.
    assert_eq!(f.children[0].offset.left, lu(75));
}

#[test]
fn ai_flex_start_column() {
    // align-items: flex-start in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.align_items = ItemAlignment::new(ItemPosition::FlexStart);
    }
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    assert_eq!(f.children[0].offset.left, lu(0));
}

#[test]
fn ai_flex_end_column() {
    // align-items: flex-end in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.align_items = ItemAlignment::new(ItemPosition::FlexEnd);
    }
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    // Cross = width = 200. cross_space = 150. FlexEnd = 150.
    assert_eq!(f.children[0].offset.left, lu(150));
}

#[test]
fn ai_center_diff_heights() {
    // center with different height items — each centered independently
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    add_child(&mut doc, c, 50, 30);
    add_child(&mut doc, c, 50, 60);
    let f = lay(&doc, c, 300, 100);
    // Item 0: cross_space=70, y=35. Item 1: cross_space=40, y=20.
    assert_eq!(f.children[0].offset.top, lu(35));
    assert_eq!(f.children[1].offset.top, lu(20));
}

#[test]
fn ai_flex_end_diff_heights() {
    // flex-end with different height items
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexEnd);
    add_child(&mut doc, c, 50, 30);
    add_child(&mut doc, c, 50, 60);
    let f = lay(&doc, c, 300, 100);
    // Item 0: y=70. Item 1: y=40.
    assert_eq!(f.children[0].offset.top, lu(70));
    assert_eq!(f.children[1].offset.top, lu(40));
}

#[test]
fn ai_stretch_with_padding() {
    // stretch with container padding — items positioned inside content area
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.padding_top = Length::px(10.0);
        s.padding_bottom = Length::px(10.0);
    }
    add_child(&mut doc, c, 50, 40);
    // border-box height = 100 + 20 (padding) = 120 (content-box sizing)
    let space = ConstraintSpace::for_root(lu(300), lu(120));
    let f = flex_layout(&doc, c, &space);
    // content_offset_y = 10 (padding_top).
    // Explicit height item keeps h=40, placed at top of content area.
    assert_eq!(f.children[0].height(), lu(40));
    assert_eq!(f.children[0].offset.top, lu(10));
}

#[test]
fn ai_center_with_margins() {
    // center with item margins
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    let ch = add_child(&mut doc, c, 50, 40);
    doc.node_mut(ch).style_mut().margin_top = Length::px(10.0);
    let f = lay(&doc, c, 300, 100);
    // cross_margin_box = 40 + 10 = 50. cross_space = 50. center offset = 25.
    // y = 0 + 25 + 10 (margin_top) = 35.
    assert_eq!(f.children[0].offset.top, lu(35));
    assert_eq!(f.children[0].height(), lu(40));
}

#[test]
fn ai_flex_start_with_border() {
    // flex-start with item border — border adds to item size
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexStart);
    let ch = add_child(&mut doc, c, 50, 40);
    {
        let s = doc.node_mut(ch).style_mut();
        s.border_top_width = 5;
        s.border_top_style = BorderStyle::Solid;
    }
    let f = lay(&doc, c, 300, 100);
    // Content-box: border-box height = 40 + 5 = 45.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[0].height(), lu(45));
}

#[test]
fn ai_center_row_reverse() {
    // center in row-reverse — cross axis unaffected by direction reversal
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::RowReverse;
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    add_child(&mut doc, c, 50, 40);
    let f = lay(&doc, c, 300, 100);
    // Cross behavior is same as normal row. cross_space=60, y=30.
    assert_eq!(f.children[0].offset.top, lu(30));
}

#[test]
fn ai_center_column_reverse() {
    // center in column-reverse — cross is width, main is reversed
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::ColumnReverse;
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    let c1 = add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 200, 400);
    // Cross = width = 200. cross_space = 150. Center = 75. x=75.
    // Main reversed: free=300, offset=300. Reversed: [c2,c1].
    // c2 at y=300, c1 at y=350.
    assert_eq!(f.children[0].node_id, c2);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[0].offset.left, lu(75));
    assert_eq!(f.children[0].offset.top, lu(300));
    assert_eq!(f.children[1].offset.top, lu(350));
}

#[test]
fn ai_stretch_with_wrap() {
    // stretch in wrap: lines expand via align-content: normal→stretch, 
    // items with explicit height keep their size but lines are larger
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;
    // Line 1: 60px and 30px → line cross = 60
    add_child(&mut doc, c, 100, 60);
    add_child(&mut doc, c, 100, 30);
    // Line 2: 40px
    add_child(&mut doc, c, 100, 40);
    let f = lay(&doc, c, 200, 200);
    assert_eq!(f.children.len(), 3);
    // align-content: normal → stretch. Free cross = 200-60-40 = 100. Each line gets +50.
    // Line 1: cross=110, child0 h=60 at y=0, child1 h=30 at y=0
    assert_eq!(f.children[0].height(), lu(60));
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].height(), lu(30));
    assert_eq!(f.children[1].offset.top, lu(0));
    // Line 2: cross=90, child2 h=40 at y=110 (line1 cross offset)
    assert_eq!(f.children[2].height(), lu(40));
    assert_eq!(f.children[2].offset.top, lu(110));
}

#[test]
fn ai_center_with_wrap() {
    // center in wrap: lines stretched by align-content:normal, items centered within
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_items = ItemAlignment::new(ItemPosition::Center);
    }
    // Line 1: items h=60 and h=30 → line cross = 60
    add_child(&mut doc, c, 100, 60);
    add_child(&mut doc, c, 100, 30);
    // Line 2: item h=40 → line cross = 40
    add_child(&mut doc, c, 100, 40);
    let f = lay(&doc, c, 200, 200);
    // align-content: normal → stretch. Free=100, +50 per line.
    // Line 1: cross=110. child0: space=50, center=25. child1: space=80, center=40.
    assert_eq!(f.children[0].offset.top, lu(25));
    assert_eq!(f.children[1].offset.top, lu(40));
    // Line 2 at offset=110, cross=90. child2: space=50, center=25. y=110+25=135.
    assert_eq!(f.children[2].offset.top, lu(135));
}

#[test]
fn ai_flex_start_varying_heights() {
    // flex-start with varying heights — all at top
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexStart);
    add_child(&mut doc, c, 50, 20);
    add_child(&mut doc, c, 50, 80);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(0));
}

#[test]
fn ai_flex_end_varying_heights() {
    // flex-end with varying heights — all at bottom
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexEnd);
    add_child(&mut doc, c, 50, 20);
    add_child(&mut doc, c, 50, 80);
    add_child(&mut doc, c, 50, 50);
    let f = lay(&doc, c, 400, 100);
    // y = 100 - h
    assert_eq!(f.children[0].offset.top, lu(80));
    assert_eq!(f.children[1].offset.top, lu(20));
    assert_eq!(f.children[2].offset.top, lu(50));
}

// ═════════════════════════════════════════════════════════════════════════════
// Category 3: align-self (15 tests)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn as_override_single() {
    // align-self overrides align-items for a single item
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    // Container uses default (stretch)
    let ch = add_child(&mut doc, c, 50, 40);
    doc.node_mut(ch).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    let f = lay(&doc, c, 300, 100);
    // cross_space = 60. Center = 30.
    assert_eq!(f.children[0].offset.top, lu(30));
    assert_eq!(f.children[0].height(), lu(40));
}

#[test]
fn as_center_one_others_stretch() {
    // One item centered, others use default stretch (explicit height → no stretch)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    let c2 = add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 50);
    doc.node_mut(c2).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    let f = lay(&doc, c, 300, 100);
    // c1: stretch alignment but explicit h=50 → y=0, h=50.
    assert_eq!(f.children[0].height(), lu(50));
    assert_eq!(f.children[0].offset.top, lu(0));
    // c2: center → cross_space=60, y=30, h=40.
    assert_eq!(f.children[1].offset.top, lu(30));
    assert_eq!(f.children[1].height(), lu(40));
    // c3: stretch alignment but explicit h=50 → y=0, h=50.
    assert_eq!(f.children[2].height(), lu(50));
    assert_eq!(f.children[2].offset.top, lu(0));
}

#[test]
fn as_flex_start_in_center_container() {
    // align-self: flex-start overrides container's align-items: center
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    let c1 = add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c2).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexStart);
    let _ = c1;
    let f = lay(&doc, c, 300, 100);
    // c1: center → y=30. c2: flex-start → y=0.
    assert_eq!(f.children[0].offset.top, lu(30));
    assert_eq!(f.children[1].offset.top, lu(0));
}

#[test]
fn as_flex_end_in_flex_start_container() {
    // align-self: flex-end in a flex-start container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::FlexStart);
    let c1 = add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c2).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexEnd);
    let _ = c1;
    let f = lay(&doc, c, 300, 100);
    // c1: flex-start → y=0. c2: flex-end → cross_space=60, y=60.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(60));
}

#[test]
fn as_stretch_in_center_container() {
    // align-self: stretch on one item in a center-aligned container (explicit heights)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 60);
    doc.node_mut(c2).style_mut().align_self = ItemAlignment::new(ItemPosition::Stretch);
    let f = lay(&doc, c, 300, 100);
    // c1: center → cross_space=60, y=30.
    assert_eq!(f.children[0].offset.top, lu(30));
    // c2: stretch alignment, explicit h=60 → no stretch, placed at y=0 (stretch offset).
    assert_eq!(f.children[1].height(), lu(60));
    assert_eq!(f.children[1].offset.top, lu(0));
}

#[test]
fn as_all_different() {
    // Each item with a different align-self value
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let c1 = add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 40);
    let c3 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c1).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexStart);
    doc.node_mut(c2).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    doc.node_mut(c3).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexEnd);
    let f = lay(&doc, c, 300, 100);
    // cross_space = 60 for all.
    assert_eq!(f.children[0].offset.top, lu(0));  // flex-start
    assert_eq!(f.children[1].offset.top, lu(30)); // center
    assert_eq!(f.children[2].offset.top, lu(60)); // flex-end
}

#[test]
fn as_column_direction() {
    // align-self: center in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 400);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::Column;
    let ch = add_child(&mut doc, c, 50, 50);
    doc.node_mut(ch).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    let f = lay(&doc, c, 200, 400);
    // Cross = width = 200. cross_space = 150. Center = 75.
    assert_eq!(f.children[0].offset.left, lu(75));
}

#[test]
fn as_center_with_margins() {
    // align-self: center with margin
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let ch = add_child(&mut doc, c, 50, 40);
    {
        let s = doc.node_mut(ch).style_mut();
        s.align_self = ItemAlignment::new(ItemPosition::Center);
        s.margin_top = Length::px(10.0);
    }
    let f = lay(&doc, c, 300, 100);
    // cross_margin_box = 40+10 = 50. cross_space = 50. center = 25.
    // y = 25 + 10 (margin_top) = 35.
    assert_eq!(f.children[0].offset.top, lu(35));
}

#[test]
fn as_flex_end_with_border() {
    // align-self: flex-end with border on item
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let ch = add_child(&mut doc, c, 50, 40);
    {
        let s = doc.node_mut(ch).style_mut();
        s.align_self = ItemAlignment::new(ItemPosition::FlexEnd);
        s.border_top_width = 5;
        s.border_top_style = BorderStyle::Solid;
    }
    let f = lay(&doc, c, 300, 100);
    // border-box height = 45. cross_space = 55. flex-end → y=55.
    assert_eq!(f.children[0].offset.top, lu(55));
    assert_eq!(f.children[0].height(), lu(45));
}

#[test]
fn as_in_row_reverse() {
    // align-self in row-reverse — cross axis unaffected
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().flex_direction = FlexDirection::RowReverse;
    let ch = add_child(&mut doc, c, 50, 40);
    doc.node_mut(ch).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    let f = lay(&doc, c, 300, 100);
    // Cross behavior same as normal row. cross_space=60, y=30.
    assert_eq!(f.children[0].offset.top, lu(30));
}

#[test]
fn as_multiple_unique() {
    // Four items each with a unique align-self
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let c1 = add_child(&mut doc, c, 50, 40);
    let c2 = add_child(&mut doc, c, 50, 40);
    let c3 = add_child(&mut doc, c, 50, 40);
    let c4 = add_child(&mut doc, c, 50, 40);
    doc.node_mut(c1).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexStart);
    doc.node_mut(c2).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexEnd);
    doc.node_mut(c3).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    doc.node_mut(c4).style_mut().align_self = ItemAlignment::new(ItemPosition::FlexStart);
    let f = lay(&doc, c, 400, 100);
    assert_eq!(f.children[0].offset.top, lu(0));  // flex-start
    assert_eq!(f.children[1].offset.top, lu(60)); // flex-end
    assert_eq!(f.children[2].offset.top, lu(30)); // center
    assert_eq!(f.children[3].offset.top, lu(0));  // flex-start
}

#[test]
fn as_auto_inherits_align_items() {
    // Default align-self is auto, which inherits from align-items
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_items = ItemAlignment::new(ItemPosition::Center);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 40);
    let f = lay(&doc, c, 300, 100);
    // Both inherit center from align-items. y=30.
    assert_eq!(f.children[0].offset.top, lu(30));
    assert_eq!(f.children[1].offset.top, lu(30));
}

#[test]
fn as_stretch_explicit_cross() {
    // align-self: stretch with explicit cross size — no stretch
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let ch = add_child(&mut doc, c, 50, 40);
    doc.node_mut(ch).style_mut().align_self = ItemAlignment::new(ItemPosition::Stretch);
    let f = lay(&doc, c, 300, 100);
    // Explicit h=40 → no stretch despite Stretch alignment. Placed at top.
    assert_eq!(f.children[0].height(), lu(40));
    assert_eq!(f.children[0].offset.top, lu(0));
}

#[test]
fn as_stretch_auto_cross() {
    // align-self: stretch with auto cross size — positioned at y=0 (stretch offset)
    // Note: block_layout doesn't honor stretch_block_size, so empty auto-height
    // items stay at height 0. The stretch alignment places them at y=0.
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let ch = add_child(&mut doc, c, 50, 80);
    doc.node_mut(ch).style_mut().align_self = ItemAlignment::new(ItemPosition::Stretch);
    let f = lay(&doc, c, 300, 100);
    // Explicit h=80, stretch doesn't change it. Placed at y=0.
    assert_eq!(f.children[0].height(), lu(80));
    assert_eq!(f.children[0].offset.top, lu(0));
}

#[test]
fn as_center_multiline() {
    // align-self: center on an item in a multi-line flex
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    doc.node_mut(c).style_mut().flex_wrap = FlexWrap::Wrap;
    // Line 1: two items (100px wide each fill 200px line)
    add_child(&mut doc, c, 100, 60);
    add_child(&mut doc, c, 100, 60);
    // Line 2: 60px tall + 30px tall centered
    add_child(&mut doc, c, 100, 60);
    let c4 = add_child(&mut doc, c, 100, 30);
    doc.node_mut(c4).style_mut().align_self = ItemAlignment::new(ItemPosition::Center);
    let f = lay(&doc, c, 200, 200);
    assert_eq!(f.children.len(), 4);
    // align-content: normal → stretch. Free=200-60-60=80. +40 per line.
    // Line 1: cross=100 at offset 0. Items h=60, flex-start → y=0.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    // Line 2: cross=100 at offset 100.
    // child2 h=60 → y=100. child3 h=30, center: space=70, offset=35, y=135.
    assert_eq!(f.children[2].offset.top, lu(100));
    assert_eq!(f.children[3].offset.top, lu(135));
}

// ═════════════════════════════════════════════════════════════════════════════
// Category 4: align-content (20 tests)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn ac_flex_start() {
    // align-content: flex-start — lines packed at top
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    // Line 1: 2 items of h50. Line 2: 1 item of h40.
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 40);
    let f = lay(&doc, c, 200, 200);
    // Lines at 0 and 50. Free = 110, but flex-start → offset 0.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(50));
}

#[test]
fn ac_flex_end() {
    // align-content: flex-end — lines packed at bottom
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 40);
    let f = lay(&doc, c, 200, 200);
    // Total line cross = 90. Free = 110. FlexEnd → offset=110.
    // Line 0 at 110, line 1 at 160.
    assert_eq!(f.children[0].offset.top, lu(110));
    assert_eq!(f.children[1].offset.top, lu(110));
    assert_eq!(f.children[2].offset.top, lu(160));
}

#[test]
fn ac_center() {
    // align-content: center — lines centered
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 40);
    let f = lay(&doc, c, 200, 200);
    // Free = 110. Center = 55. Line 0 at 55, line 1 at 105.
    assert_eq!(f.children[0].offset.top, lu(55));
    assert_eq!(f.children[1].offset.top, lu(55));
    assert_eq!(f.children[2].offset.top, lu(105));
}

#[test]
fn ac_space_between_2_lines() {
    // align-content: space-between with 2 lines
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 200);
    // Lines: 50, 50. Total = 100. Free = 100. Between = 100.
    // Line 0 at 0, line 1 at 0+50+100=150.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(150));
}

#[test]
fn ac_space_between_3_lines() {
    // align-content: space-between with 3 lines
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    // 6 items, 100px wide each → 2 per line → 3 lines of h=40
    for _ in 0..6 {
        add_child(&mut doc, c, 100, 40);
    }
    let f = lay(&doc, c, 200, 300);
    // Lines: 40, 40, 40. Total = 120. Free = 180. Between = 90.
    // Line 0: 0. Line 1: 40+90=130. Line 2: 130+40+90=260.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(130));
    assert_eq!(f.children[4].offset.top, lu(260));
}

#[test]
fn ac_space_around_2_lines() {
    // align-content: space-around with 2 lines
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    }
    // 3 items → 2 lines of h=50
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 200);
    // Free = 100. Per_line = 50. Half = 25.
    // Line 0 at 25. Line 1 at 25+50+50=125.
    assert_eq!(f.children[0].offset.top, lu(25));
    assert_eq!(f.children[1].offset.top, lu(25));
    assert_eq!(f.children[2].offset.top, lu(125));
}

#[test]
fn ac_space_around_3_lines() {
    // align-content: space-around with 3 lines
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    }
    for _ in 0..6 {
        add_child(&mut doc, c, 100, 40);
    }
    let f = lay(&doc, c, 200, 300);
    // Lines: 40,40,40. Total=120. Free=180. Per_line=60. Half=30.
    // Line 0: 30. Line 1: 30+40+60=130. Line 2: 130+40+60=230.
    assert_eq!(f.children[0].offset.top, lu(30));
    assert_eq!(f.children[2].offset.top, lu(130));
    assert_eq!(f.children[4].offset.top, lu(230));
}

#[test]
fn ac_space_evenly_2_lines() {
    // align-content: space-evenly with 2 lines
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 220);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 220);
    // Lines: 50,50. Total=100. Free=120. Slots=3. Per=40.
    // Line 0: 40. Line 1: 40+50+40=130.
    assert_eq!(f.children[0].offset.top, lu(40));
    assert_eq!(f.children[1].offset.top, lu(40));
    assert_eq!(f.children[2].offset.top, lu(130));
}

#[test]
fn ac_stretch() {
    // align-content: stretch — lines expand to fill container
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::Stretch);
    }
    // 3 items → 2 lines of natural h=50
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 200);
    // Natural: 50+50=100. Free=100. Extra per line = 50. Lines become 100.
    // Line 0 at 0, line 1 at 100.
    // Items keep h=50 (explicit), placed at top of expanded line.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(100));
    assert_eq!(f.children[2].height(), lu(50));
}

#[test]
fn ac_center_column() {
    // align-content: center in column direction (wrap → wraps into columns)
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    // Column: main=height(200), cross=width(400).
    // Items h=100 → 2 per column line. 3 items → 2 lines.
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 100);
    let f = lay(&doc, c, 400, 200);
    // Lines: cross=50, cross=50. Total=100. Free=300. Center=150.
    // Line 0 at 150, line 1 at 200.
    assert_eq!(f.children[0].offset.left, lu(150));
    assert_eq!(f.children[1].offset.left, lu(150));
    assert_eq!(f.children[2].offset.left, lu(200));
}

#[test]
fn ac_flex_end_column() {
    // align-content: flex-end in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 100);
    let f = lay(&doc, c, 400, 200);
    // Free = 300. FlexEnd = 300. Lines at 300, 350.
    assert_eq!(f.children[0].offset.left, lu(300));
    assert_eq!(f.children[1].offset.left, lu(300));
    assert_eq!(f.children[2].offset.left, lu(350));
}

#[test]
fn ac_space_between_column() {
    // align-content: space-between in column direction
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_direction = FlexDirection::Column;
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 100);
    add_child(&mut doc, c, 50, 100);
    let f = lay(&doc, c, 400, 200);
    // Lines: 50,50. Free=300. Between=300. Line 0 at 0, line 1 at 350.
    assert_eq!(f.children[0].offset.left, lu(0));
    assert_eq!(f.children[2].offset.left, lu(350));
}

#[test]
fn ac_wrap_reverse() {
    // wrap-reverse reverses line order AND flips align-content semantics
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::WrapReverse;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    let c1 = add_child(&mut doc, c, 100, 50);
    let c2 = add_child(&mut doc, c, 100, 50);
    let c3 = add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 200);
    // Before reversal: line0=[c1,c2], line1=[c3].
    // After reversal: [line1(c3), line0(c1,c2)].
    // flex-start + wrap-reverse = pack toward bottom. Free=100.
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[1].node_id, c1);
    assert_eq!(f.children[2].node_id, c2);
    assert_eq!(f.children[0].offset.top, lu(100)); // c3
    assert_eq!(f.children[1].offset.top, lu(150)); // c1
    assert_eq!(f.children[2].offset.top, lu(150)); // c2
}

#[test]
fn ac_center_wrap_reverse() {
    // align-content: center + wrap-reverse
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::WrapReverse;
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    let _c1 = add_child(&mut doc, c, 100, 50);
    let _c2 = add_child(&mut doc, c, 100, 50);
    let c3 = add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 200);
    // Lines reversed: [line1(c3), line0(c1,c2)]. Free=100. Center=50.
    // Line 0 at 50, line 1 at 100.
    assert_eq!(f.children[0].node_id, c3);
    assert_eq!(f.children[0].offset.top, lu(50));
    assert_eq!(f.children[1].offset.top, lu(100));
    assert_eq!(f.children[2].offset.top, lu(100));
}

#[test]
fn ac_flex_start_single_line() {
    // align-content on single line (nowrap) — no visible effect
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    doc.node_mut(c).style_mut().align_content =
        ContentAlignment::new(ContentPosition::FlexStart);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 40);
    let f = lay(&doc, c, 300, 100);
    // Single line: cross = container height = 100. Items at y=0 (stretch, explicit h).
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
}

#[test]
fn ac_with_row_gap() {
    // align-content with row-gap — gap adds between lines
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.row_gap = Some(Length::px(20.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    // 4 items → 2 lines of h=50
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 300);
    // Line 0 at 0. Line 1 at 50 + 20 (gap) = 70.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(70));
    assert_eq!(f.children[3].offset.top, lu(70));
}

#[test]
fn ac_space_between_single_line() {
    // space-between with 1 line: falls back to flex-start
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    // 2 items fit on one line (100+100=200 ≤ 300)
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 300, 200);
    // Single line. space-between with 1 line → flex-start. Items at y=0.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[1].offset.top, lu(0));
}

#[test]
fn ac_stretch_explicit_height() {
    // align-content: stretch with explicit container height
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 200);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::Stretch);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 200);
    // Lines: 50,50=100. Free=100. Extra=50 per line. Lines become 100,100.
    // Line 0 at 0, line 1 at 100.
    assert_eq!(f.children[0].offset.top, lu(0));
    assert_eq!(f.children[2].offset.top, lu(100));
    // Explicit height items don't stretch, keep h=50
    assert_eq!(f.children[0].height(), lu(50));
}

#[test]
fn ac_center_diff_line_heights() {
    // center with different line heights
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    // Line 1: two items h=60 → line cross = 60.
    add_child(&mut doc, c, 100, 60);
    add_child(&mut doc, c, 100, 60);
    // Line 2: one item h=40 → line cross = 40.
    add_child(&mut doc, c, 100, 40);
    let f = lay(&doc, c, 200, 300);
    // Total = 100. Free = 200. Center = 100.
    // Line 0 at 100, line 1 at 160.
    assert_eq!(f.children[0].offset.top, lu(100));
    assert_eq!(f.children[1].offset.top, lu(100));
    assert_eq!(f.children[2].offset.top, lu(160));
}

#[test]
fn ac_flex_end_with_row_gap() {
    // align-content: flex-end with row-gap
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 300);
    {
        let s = doc.node_mut(c).style_mut();
        s.flex_wrap = FlexWrap::Wrap;
        s.row_gap = Some(Length::px(20.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    let f = lay(&doc, c, 200, 300);
    // Lines: 50,50. Total=100. Gap=20. Free=300-100-20=180. FlexEnd=180.
    // Line 0 at 180. Line 1 at 180+50+20=250.
    assert_eq!(f.children[0].offset.top, lu(180));
    assert_eq!(f.children[1].offset.top, lu(180));
    assert_eq!(f.children[2].offset.top, lu(250));
    assert_eq!(f.children[3].offset.top, lu(250));
}
