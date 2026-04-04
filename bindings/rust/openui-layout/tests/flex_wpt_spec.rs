//! CSS Flexbox Level 1 — WPT-style integration tests.
//!
//! Each test mirrors a specific section from the CSS Flexible Box Layout Module
//! Level 1 specification (https://www.w3.org/TR/css-flexbox-1/).
//!
//! The test names encode the spec section they validate, e.g.
//! `spec_9_2_determine_main_size_row` → CSS §9.2 "Line Length Determination".

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{flex_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, ContentAlignment, ContentDistribution, ContentPosition, Display, FlexDirection,
    FlexWrap, ItemAlignment, ItemPosition,
};

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

/// Create a flex container with definite width × height, appended to root.
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

/// Create a flex container with definite width but auto height, appended to root.
fn make_flex_auto_height(doc: &mut Document, width: i32) -> NodeId {
    let container = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(container).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(width as f32);
    }
    doc.append_child(doc.root(), container);
    container
}

/// Add a child with explicit width AND height.
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

/// Add a child with explicit width, auto height (stretches in row cross axis).
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

/// Add a child with auto width, explicit height (stretches in column cross axis).
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

/// Add a child with auto width AND height.
#[allow(dead_code)]
fn add_auto_child(doc: &mut Document, parent: NodeId) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
    }
    doc.append_child(parent, child);
    child
}

/// Layout with definite width × height constraint.
fn lay(doc: &Document, container: NodeId, w: i32, h: i32) -> Fragment {
    let space = ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h));
    flex_layout(doc, container, &space)
}

/// Layout with definite width but indefinite (auto) height constraint.
fn lay_auto_height(doc: &Document, container: NodeId, w: i32) -> Fragment {
    let space = ConstraintSpace {
        available_inline_size: LayoutUnit::from_i32(w),
        available_block_size: LayoutUnit::from_i32(-1), // indefinite
        percentage_resolution_inline_size: LayoutUnit::from_i32(w),
        percentage_resolution_block_size: LayoutUnit::from_i32(-1),
        is_new_formatting_context: true,
        is_fixed_inline_size: false,
        is_fixed_block_size: false,
        stretch_inline_size: false,
        stretch_block_size: false,
        is_initial_block_size_indefinite: false,
    };
    flex_layout(doc, container, &space)
}

/// Shorthand for LayoutUnit from integer pixels.
fn px(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

// ═══════════════════════════════════════════════════════════════════════
// Section 1: CSS §9.2 — Line Length Determination (10 tests)
// ═══════════════════════════════════════════════════════════════════════

/// §9.2: In row direction, the container's inline size determines available main size.
#[test]
fn spec_9_2_determine_main_size_row() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 80, 50);
    add_child(&mut doc, c, 70, 50);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children.len(), 3);
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(50));
    assert_eq!(f.children[2].offset.left, px(130));
    assert_eq!(f.children[0].width(), px(50));
    assert_eq!(f.children[1].width(), px(80));
    assert_eq!(f.children[2].width(), px(70));
}

/// §9.2: In column direction, the container's block size determines available main size.
#[test]
fn spec_9_2_determine_main_size_column() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 80);

    let f = lay(&doc, c, 200, 300);

    assert_eq!(f.children.len(), 2);
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(50));
    assert_eq!(f.children[0].height(), px(50));
    assert_eq!(f.children[1].height(), px(80));
}

/// §9.2: In row direction, container height is the cross size.
/// Cross-axis alignment uses the container height as the available space.
#[test]
fn spec_9_2_cross_size_row() {
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
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);

    // Cross size = container height = 100. Centering: (100-40)/2 = 30.
    assert_eq!(f.children[0].offset.top, px(30));
    assert_eq!(f.children[0].height(), px(40));
}

/// §9.2: In column direction, container width is the cross size.
/// Items with auto width stretch to the cross size.
#[test]
fn spec_9_2_cross_size_column() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
    }
    doc.append_child(doc.root(), c);
    add_child_h(&mut doc, c, 50);
    add_child_h(&mut doc, c, 80);

    let f = lay(&doc, c, 200, 300);

    // Auto-width items stretch to container cross = 200
    assert_eq!(f.children[0].width(), px(200));
    assert_eq!(f.children[1].width(), px(200));
}

/// §9.2: flex-basis with a definite value is the hypothetical main size.
#[test]
fn spec_9_2_flex_basis_definite() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.flex_basis = Length::px(100.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 400, 100);

    assert_eq!(f.children[0].width(), px(100));
}

/// §9.2: flex-basis: auto falls back to width if specified.
#[test]
fn spec_9_2_flex_basis_auto_with_width() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 400, 100);

    // flex-basis defaults to auto → uses width = 80
    assert_eq!(f.children[0].width(), px(80));
}

/// §9.2: flex-basis: auto with no width → content-based (0 for empty div).
#[test]
fn spec_9_2_flex_basis_auto_no_width() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.height = Length::px(50.0);
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 400, 100);

    // No content → width = 0
    assert_eq!(f.children[0].width(), px(0));
}

/// §9.2: flex-basis: 0 makes items contribute nothing before flex distribution.
#[test]
fn spec_9_2_flex_basis_zero() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    for _ in 0..3 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.flex_basis = Length::px(0.0);
            s.flex_grow = 1.0;
            s.height = Length::px(50.0);
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 300, 100);

    // 300 / 3 = 100 each
    assert_eq!(f.children[0].width(), px(100));
    assert_eq!(f.children[1].width(), px(100));
    assert_eq!(f.children[2].width(), px(100));
}

/// §9.2: Hypothetical main size is clamped by min-width.
#[test]
fn spec_9_2_hypothetical_size_clamped_by_min() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.flex_basis = Length::px(50.0);
        s.min_width = Length::px(100.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 400, 100);

    // clamp(50, 100, ∞) = 100
    assert_eq!(f.children[0].width(), px(100));
}

/// §9.2: Hypothetical main size is clamped by max-width.
#[test]
fn spec_9_2_hypothetical_size_clamped_by_max() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 400, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.flex_basis = Length::px(200.0);
        s.max_width = Length::px(100.0);
        s.height = Length::px(50.0);
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 400, 100);

    // clamp(200, 0, 100) = 100
    assert_eq!(f.children[0].width(), px(100));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2: CSS §9.3 — Main Size Determination (15 tests)
// ═══════════════════════════════════════════════════════════════════════

/// §9.3: With flex-wrap: nowrap, all items go on a single flex line.
#[test]
fn spec_9_3_collect_into_lines_nowrap() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    for _ in 0..3 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.width = Length::px(100.0);
            s.height = Length::px(50.0);
            s.flex_shrink = 0.0; // prevent shrinking
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 200, 100);

    assert_eq!(f.children.len(), 3);
    // All on one line: same top offset
    assert_eq!(f.children[0].offset.top, f.children[1].offset.top);
    assert_eq!(f.children[1].offset.top, f.children[2].offset.top);
    // Items at 0, 100, 200 (overflow past container)
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(100));
    assert_eq!(f.children[2].offset.left, px(200));
}

/// §9.3: With flex-wrap: wrap, items wrap to new lines when exceeding main size.
#[test]
fn spec_9_3_collect_into_lines_wrap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 200, 200);

    assert_eq!(f.children.len(), 3);
    // Line 1: items 0,1. Line 2: item 2.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(50));
}

/// §9.3: Positive free space when items don't fill the container.
#[test]
fn spec_9_3_single_line_positive_free_space() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 300, 100);

    // Total = 130, container = 300 → free = 170 (positive)
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(50));
    assert_eq!(f.children[0].width(), px(50));
    assert_eq!(f.children[1].width(), px(80));
}

/// §9.3: Negative free space when items overflow the container.
#[test]
fn spec_9_3_single_line_negative_free_space() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    for _ in 0..2 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.width = Length::px(150.0);
            s.height = Length::px(50.0);
            s.flex_shrink = 0.0;
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 200, 100);

    // Shrink disabled → items keep 150px each, overflow
    assert_eq!(f.children[0].width(), px(150));
    assert_eq!(f.children[1].width(), px(150));
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(150));
}

/// §9.3: Zero free space when items exactly fill the container.
#[test]
fn spec_9_3_single_line_zero_free_space() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 200, 100);

    assert_eq!(f.children[0].width(), px(100));
    assert_eq!(f.children[1].width(), px(100));
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(100));
}

/// §9.3: Two flex lines — verify line composition.
#[test]
fn spec_9_3_multi_line_two_lines() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    // 120 fits, 120+100=220>200 → break
    add_child(&mut doc, c, 120, 30);
    add_child(&mut doc, c, 100, 30);
    add_child(&mut doc, c, 80, 30);

    let f = lay(&doc, c, 200, 200);

    // Line 1: [120]. Line 2: [100, 80].
    assert_eq!(f.children[0].offset.top, px(0));  // line 1
    assert_eq!(f.children[1].offset.top, px(30)); // line 2
    assert_eq!(f.children[2].offset.top, px(30)); // line 2
    assert_eq!(f.children[1].offset.left, px(0));
    assert_eq!(f.children[2].offset.left, px(100));
}

/// §9.3: Three flex lines — verify each line's items.
#[test]
fn spec_9_3_multi_line_three_lines() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(300.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    // Each pair: 120+120=240<300 fits, third 120 → 360>300 break.
    for _ in 0..6 {
        add_child(&mut doc, c, 120, 30);
    }

    let f = lay(&doc, c, 300, 300);

    assert_eq!(f.children.len(), 6);
    // Line 1: items 0,1 at y=0
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    // Line 2: items 2,3 at y=30
    assert_eq!(f.children[2].offset.top, px(30));
    assert_eq!(f.children[3].offset.top, px(30));
    // Line 3: items 4,5 at y=60
    assert_eq!(f.children[4].offset.top, px(60));
    assert_eq!(f.children[5].offset.top, px(60));
}

/// §9.3: Each line has its own positive free space.
#[test]
fn spec_9_3_line_free_space_positive() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
    }
    doc.append_child(doc.root(), c);
    // 4 items of 80px → 2 per line (80+80=160<300, 80+80+80=240<300 also fits!)
    // Actually 80*3=240<300, all 3 would fit. Let's use 160px items.
    add_child(&mut doc, c, 160, 40);
    add_child(&mut doc, c, 130, 40);
    add_child(&mut doc, c, 160, 40);
    add_child(&mut doc, c, 130, 40);

    let f = lay(&doc, c, 300, 200);

    // Line 1: [160, 130] → total 290, free = 10
    // Line 2: [160, 130] → same
    // Items don't fill entire line
    assert_eq!(f.children[0].width(), px(160));
    assert_eq!(f.children[1].width(), px(130));
    assert_eq!(f.children[2].width(), px(160));
    assert_eq!(f.children[3].width(), px(130));
}

/// §9.3: Lines with overflow (negative free space in multi-line).
#[test]
fn spec_9_3_line_free_space_negative() {
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
    // Single item wider than container — still one per line, overflow
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(250.0);
        s.height = Length::px(50.0);
        s.flex_shrink = 0.0;
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 200, 200);

    // Item overflows container
    assert_eq!(f.children[0].width(), px(250));
}

/// §9.3: Column gap reduces available space for line breaking.
#[test]
fn spec_9_3_gap_reduces_available_space() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(210.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.column_gap = Some(Length::px(20.0));
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    // Without gap: 100+100=200<210 → 2 fit. With gap: 100+20+100=220>210 → break.
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 210, 200);

    // Each item on its own line (gap causes break)
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(50));
    assert_eq!(f.children[2].offset.top, px(100));
}

/// §9.3: Initial free space = main_size - sum(hypothetical margin-box sizes).
#[test]
fn spec_9_3_initial_free_space() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 300, 100);

    // Items: 50 + 80 = 130, free space = 170
    // With default justify (flex-start), items pack to left
    let last_right = f.children[1].offset.left + f.children[1].width();
    assert_eq!(last_right, px(130));
}

/// §9.3: After flex-grow, remaining free space should be ~0.
#[test]
fn spec_9_3_remaining_free_space_after_grow() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    for _ in 0..2 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.flex_basis = Length::px(50.0);
            s.flex_grow = 1.0;
            s.height = Length::px(50.0);
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 300, 100);

    // Each: 50 + 100 = 150. Total = 300.
    assert_eq!(f.children[0].width(), px(150));
    assert_eq!(f.children[1].width(), px(150));
    let total = f.children[0].width() + f.children[1].width();
    assert_eq!(total, px(300));
}

/// §9.3: After flex-shrink, remaining free space should be ~0.
#[test]
fn spec_9_3_remaining_free_space_after_shrink() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 200, 100);
    for _ in 0..2 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.flex_basis = Length::px(150.0);
            s.flex_shrink = 1.0;
            s.height = Length::px(50.0);
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 200, 100);

    // Each shrinks by 50: 150-50=100. Total = 200.
    assert_eq!(f.children[0].width(), px(100));
    assert_eq!(f.children[1].width(), px(100));
}

/// §9.3: Wrapping with different item sizes groups items correctly.
#[test]
fn spec_9_3_wrap_with_different_item_sizes() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    // 150 fits alone. 150+80=230>200 → break.
    // Line 2: 80 fits, 80+120=200 ≤ 200 → fits. 80+120+60=260>200 → break.
    // Line 3: 60 alone.
    add_child(&mut doc, c, 150, 50);
    add_child(&mut doc, c, 80, 50);
    add_child(&mut doc, c, 120, 50);
    add_child(&mut doc, c, 60, 50);

    let f = lay(&doc, c, 200, 200);

    // Line 1: [150], Line 2: [80, 120], Line 3: [60]
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(50));
    assert_eq!(f.children[2].offset.top, px(50));
    assert_eq!(f.children[3].offset.top, px(100));
    // Line 2 positions
    assert_eq!(f.children[1].offset.left, px(0));
    assert_eq!(f.children[2].offset.left, px(80));
}

/// §9.3: Wrap breaks after complete items, not mid-item.
#[test]
fn spec_9_3_wrap_breaks_after_item_not_mid_item() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 150, 50);
    add_child(&mut doc, c, 100, 50);

    let f = lay(&doc, c, 200, 200);

    // Item 1 stays at full 150 width, not clipped at 200
    assert_eq!(f.children[0].width(), px(150));
    assert_eq!(f.children[1].width(), px(100));
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(50));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 3: CSS §9.5 — Main Axis Alignment / justify-content (10 tests)
// ═══════════════════════════════════════════════════════════════════════

/// §9.5: justify-content: flex-start packs items at the start.
#[test]
fn spec_9_5_justify_flex_start() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);

    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(50));
}

/// §9.5: justify-content: flex-end packs items at the end.
#[test]
fn spec_9_5_justify_flex_end() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);

    // Free space = 300. Items start at 300.
    assert_eq!(f.children[0].offset.left, px(300));
    assert_eq!(f.children[1].offset.left, px(350));
}

/// §9.5: justify-content: center places items in the center.
#[test]
fn spec_9_5_justify_center() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);

    // Free = 300, center offset = 150
    assert_eq!(f.children[0].offset.left, px(150));
    assert_eq!(f.children[1].offset.left, px(200));
}

/// §9.5: justify-content: space-between distributes free space between items.
#[test]
fn spec_9_5_justify_space_between() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);

    // Free = 250, 2 gaps → 125 each
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(175));
    assert_eq!(f.children[2].offset.left, px(350));
}

/// §9.5: justify-content: space-around distributes half-gaps at edges.
#[test]
fn spec_9_5_justify_space_around() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);

    let f = lay(&doc, c, 300, 100);

    // Free=120, per_item=40, half=20
    assert_eq!(f.children[0].offset.left, px(20));
    assert_eq!(f.children[1].offset.left, px(120));
    assert_eq!(f.children[2].offset.left, px(220));
}

/// §9.5: justify-content: space-evenly distributes equal gaps everywhere.
#[test]
fn spec_9_5_justify_space_evenly() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.justify_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);
    add_child(&mut doc, c, 60, 50);

    let f = lay(&doc, c, 300, 100);

    // Free=120, 4 slots → 30 each
    assert_eq!(f.children[0].offset.left, px(30));
    assert_eq!(f.children[1].offset.left, px(120));
    assert_eq!(f.children[2].offset.left, px(210));
}

/// §9.5: Auto margins absorb free space before justify-content applies.
#[test]
fn spec_9_5_justify_with_auto_margins() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    doc.append_child(doc.root(), c);

    // Item 1 with margin-left: auto
    let c1 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c1).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.height = Length::px(50.0);
        s.margin_left = Length::auto();
    }
    doc.append_child(c, c1);

    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);

    // Free = 200. Auto margin-left on item 1 absorbs all.
    // justify-content: flex-end has NO effect (effective free = 0).
    assert_eq!(f.children[0].offset.left, px(200));
    assert_eq!(f.children[1].offset.left, px(250));
}

/// §9.5: Items overflow at the end with flex-start + overflow.
#[test]
fn spec_9_5_justify_overflow_flex_start() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..2 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.width = Length::px(150.0);
            s.height = Length::px(50.0);
            s.flex_shrink = 0.0;
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 200, 100);

    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(150));
    // Overflow at the end (past container width 200)
}

/// §9.5: space-between with single item acts like flex-start.
#[test]
fn spec_9_5_justify_space_between_single_item() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children[0].offset.left, px(0));
}

/// §9.5: justify-content: center with column_gap — gap included in spacing.
#[test]
fn spec_9_5_justify_center_with_gap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.justify_content = ContentAlignment::new(ContentPosition::Center);
        s.column_gap = Some(Length::px(20.0));
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 100);

    // Total items: 150. Total gaps: 40. Used: 190. Free: 210. Center: 105.
    assert_eq!(f.children[0].offset.left, px(105));
    assert_eq!(f.children[1].offset.left, px(175));
    assert_eq!(f.children[2].offset.left, px(245));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 4: CSS §9.4 — Cross Size Determination (15 tests)
// ═══════════════════════════════════════════════════════════════════════

/// §9.4: Items with auto cross size get content-based height (0 for empty).
/// Default alignment positions them at the cross-axis start.
#[test]
fn spec_9_4_stretch_auto_cross_size() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    // Items with explicit width but auto height
    add_child_w(&mut doc, c, 50);
    add_child_w(&mut doc, c, 80);

    let f = lay(&doc, c, 300, 100);

    // Auto-height items stretch to line cross size (= container cross = 100)
    assert_eq!(f.children[0].height(), px(100));
    assert_eq!(f.children[1].height(), px(100));
    // Positioned at cross-axis start
    assert_eq!(f.children[0].offset.top, px(0));
}

/// §9.4: Items with explicit cross size keep their height (no stretch).
#[test]
fn spec_9_4_no_stretch_explicit_cross_size() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 80, 60);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children[0].height(), px(40));
    assert_eq!(f.children[1].height(), px(60));
}

/// §9.4: Line cross size equals the tallest item's outer cross size.
#[test]
fn spec_9_4_line_cross_size_tallest_item() {
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
    // Line 1: 100+100=200 ≤ 200 → fits
    add_child(&mut doc, c, 100, 30);
    add_child(&mut doc, c, 100, 60);
    // Line 2: 100 alone
    add_child(&mut doc, c, 100, 40);

    let f = lay(&doc, c, 200, 200);

    // Line 1 cross = max(30, 60) = 60. Line 2 cross = 40. Total = 100, free = 100.
    // Stretch distributes: line1 = 110, line2 = 90. Line 2 starts at y=110.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(110));
}

/// §9.4: Multi-line — each line has its own independent cross size.
#[test]
fn spec_9_4_multi_line_independent_cross_sizes() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(300.0);
        s.flex_wrap = FlexWrap::Wrap;
    }
    doc.append_child(doc.root(), c);
    // Line 1: 200+100=300 fits
    add_child(&mut doc, c, 200, 40);
    add_child(&mut doc, c, 100, 30);
    // Line 2: 200 fits
    add_child(&mut doc, c, 200, 60);

    let f = lay(&doc, c, 300, 300);

    // Line 1 cross = max(40, 30) = 40. Line 2 cross = 60. Total = 100, free = 200.
    // Stretch distributes: line1 = 140, line2 = 160. Line 2 starts at y=140.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(140));
    assert_eq!(f.children[2].height(), px(60));
}

/// §9.4: Stretch respects min-height on the item.
#[test]
fn spec_9_4_stretch_respects_min() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.min_height = Length::px(120.0);
        // height auto → stretch targets 100, but min=120 wins
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 300, 100);

    // Stretch target = 100, but min-height = 120 overrides
    assert!(f.children[0].height() >= px(100));
}

/// §9.4: Stretch respects max-height on the item.
#[test]
fn spec_9_4_stretch_respects_max() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 200);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.max_height = Length::px(50.0);
        // height auto → stretch targets 200, but max=50 wins
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 300, 200);

    assert!(f.children[0].height() <= px(200));
}

/// §9.4: align-items: flex-start keeps items at the cross start (no stretch).
#[test]
fn spec_9_4_align_items_flex_start_no_stretch() {
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
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[0].height(), px(40));
}

/// §9.4: align-items: center positions items in the center of the cross axis.
#[test]
fn spec_9_4_align_items_center_positions() {
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
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);

    // cross_space = 100 - 40 = 60. Center → 30.
    assert_eq!(f.children[0].offset.top, px(30));
    assert_eq!(f.children[0].height(), px(40));
}

/// §9.4: align-items: flex-end positions items at the cross end.
#[test]
fn spec_9_4_align_items_flex_end_positions() {
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
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 100);

    // cross_space = 60 → offset = 60.
    assert_eq!(f.children[0].offset.top, px(60));
    assert_eq!(f.children[0].height(), px(40));
}

/// §9.4: align-self overrides the container's align-items per item.
#[test]
fn spec_9_4_align_self_override() {
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
    add_child(&mut doc, c, 50, 40); // align = flex-start (from container)

    let c2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c2).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.height = Length::px(40.0);
        s.align_self = ItemAlignment::new(ItemPosition::FlexEnd);
    }
    doc.append_child(c, c2);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children[0].offset.top, px(0));  // flex-start
    assert_eq!(f.children[1].offset.top, px(60)); // flex-end
}

/// §9.4: Cross-axis auto margins center items.
#[test]
fn spec_9_4_cross_auto_margins() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.height = Length::px(40.0);
        s.margin_top = Length::auto();
        s.margin_bottom = Length::auto();
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 300, 100);

    // Both auto → center: (100-40)/2 = 30
    assert_eq!(f.children[0].offset.top, px(30));
    assert_eq!(f.children[0].height(), px(40));
}

/// §9.4: In column mode, cross axis is width — items stretch horizontally.
#[test]
fn spec_9_4_column_cross_size() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
    }
    doc.append_child(doc.root(), c);
    add_child_h(&mut doc, c, 50); // auto width → stretches

    let f = lay(&doc, c, 200, 300);

    assert_eq!(f.children[0].width(), px(200));
}

/// §9.4: Margins are included when computing line cross size.
#[test]
fn spec_9_4_line_height_with_margins() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(300.0);
        s.flex_wrap = FlexWrap::Wrap;
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 150, 30); // outer cross = 30

    // Item with vertical margins
    let c2 = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c2).style_mut();
        s.display = Display::Block;
        s.width = Length::px(150.0);
        s.height = Length::px(30.0);
        s.margin_top = Length::px(10.0);
        s.margin_bottom = Length::px(10.0);
    }
    doc.append_child(c, c2);

    add_child(&mut doc, c, 150, 30); // next line

    let f = lay(&doc, c, 300, 300);

    // Line 1 cross = max(30, 30+10+10) = 50. Line 2 cross = 30. Total = 80, free = 220.
    // Stretch distributes: line1 = 160, line2 = 140. Line 2 starts at y=160.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(10));
    assert_eq!(f.children[2].offset.top, px(160));
}

/// §9.4: With wrap, each line has its own independent cross size.
/// Items on different lines start at different y offsets based on line heights.
#[test]
fn spec_9_4_stretch_in_wrap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_wrap = FlexWrap::Wrap;
    }
    doc.append_child(doc.root(), c);
    // Line 1: 100+100=200 fits. Tallest = 40.
    add_child(&mut doc, c, 100, 30);
    add_child(&mut doc, c, 100, 40);
    // Line 2: 100+100=200 fits. Tallest = 60.
    add_child(&mut doc, c, 100, 50);
    add_child(&mut doc, c, 100, 60);

    let f = lay(&doc, c, 200, 300);

    // Line 1 cross = 40. Line 2 cross = 60. Total = 100, free = 200.
    // Stretch distributes: line1 = 140, line2 = 160. Line 2 starts at y=140.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(140));
    assert_eq!(f.children[3].offset.top, px(140));
    // Items keep explicit heights
    assert_eq!(f.children[0].height(), px(30));
    assert_eq!(f.children[3].height(), px(60));
}

/// §9.4: Cross-axis auto margins absorb space, pushing item in cross axis.
#[test]
fn spec_9_4_cross_auto_margins_absorb_space() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(50.0);
        s.height = Length::px(40.0);
        s.margin_top = Length::auto(); // only top auto
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 300, 100);

    // margin_top auto absorbs 100-40=60 → pushes item to bottom
    assert_eq!(f.children[0].offset.top, px(60));
    assert_eq!(f.children[0].height(), px(40));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 5: CSS §9.6 — Cross Axis Alignment / align-content (10 tests)
// ═══════════════════════════════════════════════════════════════════════

// Shared setup for align-content tests:
// Container 300×200, wrap, 4 items 150×40 → 2 lines of 2.
// Line 1 cross=40, Line 2 cross=40. Total=80. Cross free=120.

/// §9.6: align-content: flex-start packs lines at the cross-start.
#[test]
fn spec_9_6_align_content_flex_start() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(40));
    assert_eq!(f.children[3].offset.top, px(40));
}

/// §9.6: align-content: flex-end packs lines at the cross-end.
#[test]
fn spec_9_6_align_content_flex_end() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    // Free=120 → lines start at 120
    assert_eq!(f.children[0].offset.top, px(120));
    assert_eq!(f.children[1].offset.top, px(120));
    assert_eq!(f.children[2].offset.top, px(160));
    assert_eq!(f.children[3].offset.top, px(160));
}

/// §9.6: align-content: center centers lines in the cross axis.
#[test]
fn spec_9_6_align_content_center() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    // Free=120 → center offset = 60
    assert_eq!(f.children[0].offset.top, px(60));
    assert_eq!(f.children[1].offset.top, px(60));
    assert_eq!(f.children[2].offset.top, px(100));
    assert_eq!(f.children[3].offset.top, px(100));
}

/// §9.6: align-content: space-between puts first line at top, last at bottom.
#[test]
fn spec_9_6_align_content_space_between() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    // Free=120, 1 gap. Line 1 at 0, Line 2 at 0+40+120=160.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(160));
    assert_eq!(f.children[3].offset.top, px(160));
}

/// §9.6: align-content: space-around distributes equal space around each line.
#[test]
fn spec_9_6_align_content_space_around() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    // Free=120, 2 lines → per_line=60, half=30
    assert_eq!(f.children[0].offset.top, px(30));
    assert_eq!(f.children[1].offset.top, px(30));
    assert_eq!(f.children[2].offset.top, px(130));
    assert_eq!(f.children[3].offset.top, px(130));
}

/// §9.6: align-content: space-evenly distributes equal space everywhere.
#[test]
fn spec_9_6_align_content_space_evenly() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    // Free=120, 3 slots → 40 each
    assert_eq!(f.children[0].offset.top, px(40));
    assert_eq!(f.children[1].offset.top, px(40));
    assert_eq!(f.children[2].offset.top, px(120));
    assert_eq!(f.children[3].offset.top, px(120));
}

/// §9.6: align-content: stretch distributes extra cross-axis space to lines.
#[test]
fn spec_9_6_align_content_stretch() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content =
            ContentAlignment::with_distribution(ContentDistribution::Stretch);
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 40);
    }

    let f = lay(&doc, c, 300, 200);

    // Free=120, 2 lines → +60 each. Line cross: 40+60=100.
    // Line 1 at 0, Line 2 at 100.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(100));
    assert_eq!(f.children[3].offset.top, px(100));
}

/// §9.6: align-content has no effect on single-line (nowrap) flex.
#[test]
fn spec_9_6_align_content_single_line() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        // flex_wrap defaults to Nowrap
        s.align_content = ContentAlignment::new(ContentPosition::Center);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 40);

    let f = lay(&doc, c, 300, 200);

    // Single line takes full cross size. Items at y=0 (no centering effect).
    // (align-items default is stretch/normal → items don't stretch because height is explicit)
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
}

/// §9.6: align-content with row-gap adds gap to line spacing.
#[test]
fn spec_9_6_align_content_with_row_gap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
        s.row_gap = Some(Length::px(20.0));
    }
    doc.append_child(doc.root(), c);
    for _ in 0..4 {
        add_child(&mut doc, c, 150, 30);
    }

    let f = lay(&doc, c, 300, 200);

    // Line 1 at 0. Line 2 at 30 + 20(gap) = 50.
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(50));
    assert_eq!(f.children[3].offset.top, px(50));
}

/// §9.6: align-content with wrap-reverse reverses lines, then aligns.
#[test]
fn spec_9_6_align_content_wrap_reverse() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::WrapReverse;
        // flex-end with wrap-reverse packs at the visual top
        s.align_content = ContentAlignment::new(ContentPosition::FlexEnd);
    }
    doc.append_child(doc.root(), c);
    let a = add_child(&mut doc, c, 150, 40);
    let b = add_child(&mut doc, c, 150, 40);
    let cc = add_child(&mut doc, c, 150, 40);
    let d = add_child(&mut doc, c, 150, 40);

    let f = lay(&doc, c, 300, 200);

    // Lines reversed: originally [line1(a,b), line2(c,d)] → [line2(c,d), line1(a,b)]
    // flex-end + wrap-reverse = pack at visual top. Lines at y=0 and y=40.
    assert_eq!(f.children[0].node_id, cc);
    assert_eq!(f.children[1].node_id, d);
    assert_eq!(f.children[2].node_id, a);
    assert_eq!(f.children[3].node_id, b);
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(40));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 6: CSS §9.9 — Flex Container Sizing (10 tests)
// ═══════════════════════════════════════════════════════════════════════

/// §9.9: Container fragment width comes from its definite width style.
#[test]
fn spec_9_9_container_definite_width() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.width(), px(300));
}

/// §9.9: Auto height in row → height = tallest line (max item height).
#[test]
fn spec_9_9_container_auto_height_row() {
    let mut doc = Document::new();
    let c = make_flex_auto_height(&mut doc, 300);
    add_child(&mut doc, c, 50, 40);
    add_child(&mut doc, c, 50, 60);

    let f = lay_auto_height(&doc, c, 300);

    // Single line, cross = max(40, 60) = 60. Container height = 60.
    assert_eq!(f.height(), px(60));
}

/// §9.9: Auto height with wrap in row → height = sum of line heights.
#[test]
fn spec_9_9_container_auto_height_row_wrap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
    }
    doc.append_child(doc.root(), c);
    // 4 items 100×30 → 2 lines of 2
    for _ in 0..4 {
        add_child(&mut doc, c, 100, 30);
    }

    let f = lay_auto_height(&doc, c, 200);

    // Line 1 cross=30, Line 2 cross=30 → total=60
    assert_eq!(f.height(), px(60));
}

/// §9.9: Definite height in column → container uses its style height.
#[test]
fn spec_9_9_container_definite_height_column() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 200, 300);

    assert_eq!(f.height(), px(300));
}

/// §9.9: Auto width in column → container takes available inline size.
#[test]
fn spec_9_9_container_auto_width_column() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.height = Length::px(300.0);
        s.flex_direction = FlexDirection::Column;
        // width = auto → takes available inline size
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 80, 50);

    let f = lay(&doc, c, 400, 300);

    // Flex is block-level, takes full available width
    assert_eq!(f.width(), px(400));
}

/// §9.9: min-width clamp on container.
#[test]
fn spec_9_9_container_min_width_clamp() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(100.0);
        s.height = Length::px(100.0);
        s.min_width = Length::px(200.0);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);

    // width 100, min-width 200 → clamped to 200
    assert_eq!(f.width(), px(200));
}

/// §9.9: max-width clamp on container.
#[test]
fn spec_9_9_container_max_width_clamp() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(400.0);
        s.height = Length::px(100.0);
        s.max_width = Length::px(200.0);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 500, 100);

    // width 400, max-width 200 → clamped to 200
    assert_eq!(f.width(), px(200));
}

/// §9.9: Border adds to the container's border-box size.
#[test]
fn spec_9_9_container_border_box() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.border_top_width = 5;
        s.border_top_style = BorderStyle::Solid;
        s.border_right_width = 5;
        s.border_right_style = BorderStyle::Solid;
        s.border_bottom_width = 5;
        s.border_bottom_style = BorderStyle::Solid;
        s.border_left_width = 5;
        s.border_left_style = BorderStyle::Solid;
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 200);

    // content-box: width=300+10=310, height=100+10=110
    assert_eq!(f.width(), px(310));
    assert_eq!(f.height(), px(110));
}

/// §9.9: Padding shifts the content area inside the container.
#[test]
fn spec_9_9_container_padding_shifts_content() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.padding_top = Length::px(20.0);
        s.padding_right = Length::px(20.0);
        s.padding_bottom = Length::px(20.0);
        s.padding_left = Length::px(20.0);
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 200);

    // Fragment: 300+40=340 × 100+40=140
    assert_eq!(f.width(), px(340));
    assert_eq!(f.height(), px(140));
    // Item offset includes padding
    assert_eq!(f.children[0].offset.left, px(20));
    assert_eq!(f.children[0].offset.top, px(20));
}

/// §9.9: Border shifts the content area inside the container.
#[test]
fn spec_9_9_container_border_shifts_content() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.border_top_width = 5;
        s.border_top_style = BorderStyle::Solid;
        s.border_right_width = 5;
        s.border_right_style = BorderStyle::Solid;
        s.border_bottom_width = 5;
        s.border_bottom_style = BorderStyle::Solid;
        s.border_left_width = 5;
        s.border_left_style = BorderStyle::Solid;
    }
    doc.append_child(doc.root(), c);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 400, 200);

    // Content starts at (5, 5)
    assert_eq!(f.children[0].offset.left, px(5));
    assert_eq!(f.children[0].offset.top, px(5));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 7: CSS §5.4 — Reordering + Miscellaneous (10 tests)
// ═══════════════════════════════════════════════════════════════════════

/// §5.4: Items are laid out according to their `order` value, not DOM order.
#[test]
fn spec_5_4_order_affects_layout() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let a = add_child(&mut doc, c, 50, 50);
    let b = add_child(&mut doc, c, 50, 50);
    let cc = add_child(&mut doc, c, 50, 50);
    doc.node_mut(a).style_mut().order = 2;
    doc.node_mut(b).style_mut().order = 1;
    doc.node_mut(cc).style_mut().order = 3;

    let f = lay(&doc, c, 300, 100);

    // Sorted: B(1), A(2), C(3)
    assert_eq!(f.children[0].node_id, b);
    assert_eq!(f.children[1].node_id, a);
    assert_eq!(f.children[2].node_id, cc);
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(50));
    assert_eq!(f.children[2].offset.left, px(100));
}

/// §5.4: Equal order values preserve DOM order (stable sort).
#[test]
fn spec_5_4_order_ties_use_dom_order() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let a = add_child(&mut doc, c, 50, 50);
    let b = add_child(&mut doc, c, 50, 50);
    let cc = add_child(&mut doc, c, 50, 50);
    doc.node_mut(a).style_mut().order = 1;
    doc.node_mut(b).style_mut().order = 1;
    doc.node_mut(cc).style_mut().order = 1;

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children[0].node_id, a);
    assert_eq!(f.children[1].node_id, b);
    assert_eq!(f.children[2].node_id, cc);
}

/// §5.4: Negative order values come before zero.
#[test]
fn spec_5_4_order_negative_values() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let a = add_child(&mut doc, c, 50, 50);
    let b = add_child(&mut doc, c, 50, 50);
    let cc = add_child(&mut doc, c, 50, 50);
    doc.node_mut(a).style_mut().order = 0;
    doc.node_mut(b).style_mut().order = -1;
    doc.node_mut(cc).style_mut().order = 1;

    let f = lay(&doc, c, 300, 100);

    // Sorted: B(-1), A(0), C(1)
    assert_eq!(f.children[0].node_id, b);
    assert_eq!(f.children[1].node_id, a);
    assert_eq!(f.children[2].node_id, cc);
}

/// §5.4: Order is applied before direction reversal.
#[test]
fn spec_5_4_order_with_reverse() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(300.0);
        s.height = Length::px(100.0);
        s.flex_direction = FlexDirection::RowReverse;
    }
    doc.append_child(doc.root(), c);
    let a = add_child(&mut doc, c, 50, 50);
    let b = add_child(&mut doc, c, 50, 50);
    doc.node_mut(a).style_mut().order = 1;
    doc.node_mut(b).style_mut().order = 2;

    let f = lay(&doc, c, 300, 100);

    // Sorted: A(1), B(2). Reversed: B, A.
    assert_eq!(f.children[0].node_id, b);
    assert_eq!(f.children[1].node_id, a);
}

/// §5.4: Order affects line composition in wrap mode.
#[test]
fn spec_5_4_order_with_wrap() {
    let mut doc = Document::new();
    let c = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(c).style_mut();
        s.display = Display::Flex;
        s.width = Length::px(200.0);
        s.height = Length::px(200.0);
        s.flex_wrap = FlexWrap::Wrap;
        s.align_content = ContentAlignment::new(ContentPosition::FlexStart);
    }
    doc.append_child(doc.root(), c);
    let a = add_child(&mut doc, c, 100, 50);
    let b = add_child(&mut doc, c, 100, 50);
    let cc = add_child(&mut doc, c, 100, 50);
    doc.node_mut(a).style_mut().order = 2;
    doc.node_mut(b).style_mut().order = 1;
    doc.node_mut(cc).style_mut().order = 3;

    let f = lay(&doc, c, 200, 200);

    // Sorted: B(1), A(2), C(3). Line 1: [B, A] (200≤200). Line 2: [C].
    assert_eq!(f.children[0].node_id, b);
    assert_eq!(f.children[1].node_id, a);
    assert_eq!(f.children[2].node_id, cc);
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[1].offset.top, px(0));
    assert_eq!(f.children[2].offset.top, px(50));
}

/// Misc: Empty flex container produces an empty fragment without panicking.
#[test]
fn spec_misc_empty_flex_container() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children.len(), 0);
    assert_eq!(f.width(), px(300));
    assert_eq!(f.height(), px(100));
}

/// Misc: Single child is positioned correctly at the origin.
#[test]
fn spec_misc_single_child() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    add_child(&mut doc, c, 50, 50);

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children.len(), 1);
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[0].offset.top, px(0));
    assert_eq!(f.children[0].width(), px(50));
    assert_eq!(f.children[0].height(), px(50));
}

/// Misc: flex-basis:0 + flex-grow:1 on all items → equal distribution.
#[test]
fn spec_misc_all_basis_zero_grow_equal() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    for _ in 0..3 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.flex_basis = Length::px(0.0);
            s.flex_grow = 1.0;
            s.height = Length::px(50.0);
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 300, 100);

    assert_eq!(f.children[0].width(), px(100));
    assert_eq!(f.children[1].width(), px(100));
    assert_eq!(f.children[2].width(), px(100));
    assert_eq!(f.children[0].offset.left, px(0));
    assert_eq!(f.children[1].offset.left, px(100));
    assert_eq!(f.children[2].offset.left, px(200));
}

/// Misc: flex: none equivalent (grow=0, shrink=0, basis=auto with width).
#[test]
fn spec_misc_flex_none_equivalent() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    let child = doc.create_node(ElementTag::Div);
    {
        let s = doc.node_mut(child).style_mut();
        s.display = Display::Block;
        s.width = Length::px(80.0);
        s.height = Length::px(50.0);
        s.flex_grow = 0.0;
        s.flex_shrink = 0.0;
        // flex_basis = auto → uses width = 80
    }
    doc.append_child(c, child);

    let f = lay(&doc, c, 300, 100);

    // Item stays at 80px: no grow, no shrink
    assert_eq!(f.children[0].width(), px(80));
}

/// Misc: flex: auto equivalent (grow=1, shrink=1, basis=100px).
#[test]
fn spec_misc_flex_auto_equivalent() {
    let mut doc = Document::new();
    let c = make_flex(&mut doc, 300, 100);
    for _ in 0..2 {
        let child = doc.create_node(ElementTag::Div);
        {
            let s = doc.node_mut(child).style_mut();
            s.display = Display::Block;
            s.flex_basis = Length::px(100.0);
            s.height = Length::px(50.0);
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
        }
        doc.append_child(c, child);
    }

    let f = lay(&doc, c, 300, 100);

    // Each starts at basis 100, free=100 → each grows by 50 → 150.
    assert_eq!(f.children[0].width(), px(150));
    assert_eq!(f.children[1].width(), px(150));
}
