//! SP12 C3 — New Formatting Context tests.
//!
//! Tests for BFC creation detection, float avoidance, margin isolation, and
//! fresh exclusion space creation per CSS 2.1 §9.4.1 and §9.5.

use std::sync::Arc;

use openui_geometry::{BfcOffset, BfcRect, LayoutUnit, Length};
use openui_dom::NodeId;
use openui_style::{ComputedStyle, Display, Float, Overflow, Position};

use openui_layout::new_formatting_context::{
    NewFcLayoutInput,
    adjust_for_float_avoidance,
    build_new_fc_constraint_space,
    compute_float_avoidance_offset,
    creates_new_formatting_context,
    layout_new_formatting_context,
    new_fc_end_margin_strut,
    resolve_new_fc_margins,
};
use openui_layout::{ConstraintSpace, ConstraintSpaceBuilder, ExclusionSpace};
use openui_layout::exclusions::{ExclusionArea, ExclusionType};

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn make_float(
    exclusion_type: ExclusionType,
    line_start: i32,
    block_start: i32,
    line_end: i32,
    block_end: i32,
) -> ExclusionArea {
    ExclusionArea {
        rect: BfcRect::new(
            BfcOffset::new(lu(line_start), lu(block_start)),
            BfcOffset::new(lu(line_end), lu(block_end)),
        ),
        exclusion_type,
    }
}

// ── BFC creation detection tests ─────────────────────────────────────────

#[test]
fn overflow_hidden_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.overflow_x = Overflow::Hidden;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn overflow_scroll_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.overflow_x = Overflow::Scroll;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn overflow_auto_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.overflow_y = Overflow::Auto;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn overflow_clip_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.overflow_x = Overflow::Clip;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn float_left_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.float = Float::Left;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn float_right_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.float = Float::Right;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn absolute_position_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.position = Position::Absolute;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn fixed_position_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.position = Position::Fixed;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn display_flow_root_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn display_flex_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.display = Display::Flex;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn display_grid_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.display = Display::Grid;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn display_inline_block_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.display = Display::InlineBlock;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn display_table_creates_new_bfc() {
    let mut style = ComputedStyle::initial();
    style.display = Display::Table;
    assert!(creates_new_formatting_context(&style, false));
}

#[test]
fn normal_block_does_not_create_bfc() {
    let mut style = ComputedStyle::initial();
    style.display = Display::Block;
    assert!(!creates_new_formatting_context(&style, false));
}

#[test]
fn normal_inline_does_not_create_bfc() {
    let style = ComputedStyle::initial();
    assert!(!creates_new_formatting_context(&style, false));
}

#[test]
fn root_element_always_creates_bfc() {
    // Even a plain inline style creates a BFC when it's the root.
    let style = ComputedStyle::initial();
    assert!(creates_new_formatting_context(&style, true));
}

// ── Margin isolation tests ───────────────────────────────────────────────

#[test]
fn new_bfc_does_not_collapse_margins_with_parent() {
    // New BFC end margin strut must be empty — no margin contribution to parent.
    let strut = new_fc_end_margin_strut();
    assert!(strut.is_empty());
    assert_eq!(strut.sum(), LayoutUnit::zero());
}

#[test]
fn new_bfc_margins_resolved_immediately() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    style.margin_top = Length::px(20.0);
    style.margin_bottom = Length::px(10.0);
    style.margin_left = Length::px(5.0);
    style.margin_right = Length::px(15.0);

    let (top, right, bottom, left) = resolve_new_fc_margins(&style, lu(800));
    assert_eq!(top, lu(20));
    assert_eq!(right, lu(15));
    assert_eq!(bottom, lu(10));
    assert_eq!(left, lu(5));
}

#[test]
fn new_bfc_percentage_margins_resolve_against_container() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    style.margin_top = Length::percent(10.0); // 10% of 800 = 80

    let (top, _right, _bottom, _left) = resolve_new_fc_margins(&style, lu(800));
    assert_eq!(top, lu(80));
}

// ── Float avoidance tests ────────────────────────────────────────────────

#[test]
fn bfc_element_beside_left_float() {
    // Left float: 200px wide, 100px tall
    let mut space = ExclusionSpace::new();
    space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));

    let offset = BfcOffset::new(lu(0), lu(0));
    let result = compute_float_avoidance_offset(&space, &offset, lu(800), lu(300));

    // BFC element should be placed after the float (at line_offset 200).
    assert_eq!(result.inline_offset, lu(200));
    assert_eq!(result.available_inline_size, lu(600));
    assert_eq!(result.block_offset, lu(0)); // no need to drop down
}

#[test]
fn bfc_element_beside_right_float() {
    // Right float: 200px wide on the right side
    let mut space = ExclusionSpace::new();
    space.add(make_float(ExclusionType::Right, 600, 0, 800, 100));

    let offset = BfcOffset::new(lu(0), lu(0));
    let result = compute_float_avoidance_offset(&space, &offset, lu(800), lu(300));

    // Element starts at line 0, available space narrowed to 600.
    assert_eq!(result.inline_offset, lu(0));
    assert_eq!(result.available_inline_size, lu(600));
    assert_eq!(result.block_offset, lu(0));
}

#[test]
fn bfc_element_between_both_floats() {
    // Left float 200px, right float 200px → 400px available in middle.
    let mut space = ExclusionSpace::new();
    space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));
    space.add(make_float(ExclusionType::Right, 600, 0, 800, 100));

    let offset = BfcOffset::new(lu(0), lu(0));
    let result = compute_float_avoidance_offset(&space, &offset, lu(800), lu(300));

    assert_eq!(result.inline_offset, lu(200));
    assert_eq!(result.available_inline_size, lu(400));
    assert_eq!(result.block_offset, lu(0));
}

#[test]
fn bfc_element_drops_below_float_when_too_wide() {
    // Left float takes 700px of 800px container.
    // BFC element needs 500px — can't fit beside float.
    let mut space = ExclusionSpace::new();
    space.add(make_float(ExclusionType::Left, 0, 0, 700, 100));

    let offset = BfcOffset::new(lu(0), lu(0));
    let result = compute_float_avoidance_offset(&space, &offset, lu(800), lu(500));

    // Must drop below the float (block_offset >= 100).
    assert!(result.block_offset >= lu(100));
    assert_eq!(result.available_inline_size, lu(800));
}

#[test]
fn bfc_element_drops_below_both_floats_when_too_wide() {
    // Left float 0-400 tall 0-150, right float 500-800 tall 0-200.
    // Element needs 600px — doesn't fit beside either.
    let mut space = ExclusionSpace::new();
    space.add(make_float(ExclusionType::Left, 0, 0, 400, 150));
    space.add(make_float(ExclusionType::Right, 500, 0, 800, 200));

    let offset = BfcOffset::new(lu(0), lu(0));
    let result = compute_float_avoidance_offset(&space, &offset, lu(800), lu(600));

    // Must drop below both floats (block_offset >= 200).
    assert!(result.block_offset >= lu(200));
    assert_eq!(result.available_inline_size, lu(800));
}

#[test]
fn adjust_for_float_avoidance_no_floats() {
    let space = ExclusionSpace::new();
    let offset = BfcOffset::new(lu(10), lu(20));
    let (inline_off, avail, block_off) =
        adjust_for_float_avoidance(&space, &offset, lu(800), lu(400));

    // With no floats, original position and full width returned.
    assert_eq!(inline_off, lu(10));
    assert_eq!(avail, lu(800));
    assert_eq!(block_off, lu(20));
}

#[test]
fn adjust_for_float_avoidance_with_left_float() {
    let mut space = ExclusionSpace::new();
    space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));

    let offset = BfcOffset::new(lu(0), lu(0));
    let (inline_off, avail, block_off) =
        adjust_for_float_avoidance(&space, &offset, lu(800), lu(300));

    assert_eq!(inline_off, lu(200));
    assert_eq!(avail, lu(600));
    assert_eq!(block_off, lu(0));
}

// ── Fresh exclusion space tests ──────────────────────────────────────────

#[test]
fn fresh_exclusion_space_for_new_bfc_child() {
    // Parent has floats in its exclusion space.
    let mut parent_excl = ExclusionSpace::new();
    parent_excl.add(make_float(ExclusionType::Left, 0, 0, 200, 100));

    let parent_space = ConstraintSpaceBuilder::new()
        .set_available_size(lu(800), lu(600))
        .set_exclusion_space(Some(Arc::new(parent_excl)))
        .build();

    // Build child constraint space for new BFC.
    let child_space = build_new_fc_constraint_space(&parent_space, lu(600), lu(600), lu(800));

    // Child must have NO exclusion space (fresh BFC).
    assert!(child_space.exclusion_space.is_none());
    assert!(child_space.is_new_formatting_context);
    assert!(child_space.floats_bfc_block_offset.is_none());
    // BFC offset resets to zero for the child's own coordinate system.
    assert_eq!(child_space.bfc_offset, BfcOffset::zero());
}

#[test]
fn nested_bfc_contains_its_own_floats() {
    // Simulate: parent BFC has a float, child establishes new BFC.
    // The child's constraint space should be clean — no parent floats leak in.
    let mut parent_excl = ExclusionSpace::new();
    parent_excl.add(make_float(ExclusionType::Left, 0, 0, 300, 200));
    parent_excl.add(make_float(ExclusionType::Right, 500, 0, 800, 150));
    assert!(parent_excl.has_floats());
    assert_eq!(parent_excl.num_exclusions(), 2);

    let parent_space = ConstraintSpaceBuilder::new()
        .set_available_size(lu(800), lu(600))
        .set_exclusion_space(Some(Arc::new(parent_excl)))
        .build();

    let child_space = build_new_fc_constraint_space(&parent_space, lu(200), lu(600), lu(800));

    // Child has no exclusion space — it will create its own when it encounters floats.
    assert!(child_space.exclusion_space.is_none());

    // If the child creates its own ExclusionSpace, parent's floats are not visible.
    let child_excl = ExclusionSpace::new();
    assert!(!child_excl.has_floats());
    assert_eq!(child_excl.num_exclusions(), 0);
}

#[test]
fn child_space_inherits_fragmentation_from_parent() {
    let parent_space = ConstraintSpaceBuilder::new()
        .set_available_size(lu(800), lu(600))
        .set_fragmentainer_block_size(lu(500))
        .build();

    let child_space = build_new_fc_constraint_space(&parent_space, lu(400), lu(600), lu(800));

    // Fragmentation state should be inherited even for new BFC.
    assert_eq!(child_space.fragmentainer_block_size, lu(500));
}

// ── Layout integration tests ─────────────────────────────────────────────

#[test]
fn layout_new_fc_resolves_margins() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    style.margin_top = Length::px(10.0);
    style.margin_bottom = Length::px(20.0);
    style.margin_left = Length::px(5.0);
    style.margin_right = Length::px(15.0);

    let parent_space = ConstraintSpace::for_root(lu(800), lu(600));
    let input = NewFcLayoutInput {
        style: &style,
        node_id: NodeId::NONE,
        parent_space: &parent_space,
        child_bfc_offset: BfcOffset::new(lu(0), lu(0)),
        container_inline_size: lu(800),
        container_block_size: lu(600),
    };

    let result = layout_new_formatting_context(&input);

    assert_eq!(result.margin_top, lu(10));
    assert_eq!(result.margin_bottom, lu(20));
    assert_eq!(result.margin_left, lu(5));
    assert_eq!(result.margin_right, lu(15));
    assert!(!result.is_pushed_by_floats);
}

#[test]
fn layout_new_fc_with_float_avoidance() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;

    let mut parent_excl = ExclusionSpace::new();
    parent_excl.add(make_float(ExclusionType::Left, 0, 0, 200, 100));

    let parent_space = ConstraintSpaceBuilder::new()
        .set_available_size(lu(800), lu(600))
        .set_exclusion_space(Some(Arc::new(parent_excl)))
        .build();

    let input = NewFcLayoutInput {
        style: &style,
        node_id: NodeId::NONE,
        parent_space: &parent_space,
        child_bfc_offset: BfcOffset::new(lu(0), lu(0)),
        container_inline_size: lu(800),
        container_block_size: lu(600),
    };

    let result = layout_new_formatting_context(&input);

    // Element should avoid the left float.
    assert_eq!(result.bfc_line_offset, lu(200));
    assert_eq!(result.bfc_block_offset, lu(0));
    assert!(!result.is_pushed_by_floats);
}

#[test]
fn layout_new_fc_pushed_below_float() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    style.width = Length::px(700.0);

    let mut parent_excl = ExclusionSpace::new();
    parent_excl.add(make_float(ExclusionType::Left, 0, 0, 700, 100));

    let parent_space = ConstraintSpaceBuilder::new()
        .set_available_size(lu(800), lu(600))
        .set_exclusion_space(Some(Arc::new(parent_excl)))
        .build();

    let input = NewFcLayoutInput {
        style: &style,
        node_id: NodeId::NONE,
        parent_space: &parent_space,
        child_bfc_offset: BfcOffset::new(lu(0), lu(0)),
        container_inline_size: lu(800),
        container_block_size: lu(600),
    };

    let result = layout_new_formatting_context(&input);

    // Element is 700px wide, float takes 700px — must drop below.
    assert!(result.bfc_block_offset >= lu(100));
    assert!(result.is_pushed_by_floats);
}

#[test]
fn layout_new_fc_auto_width_fills_available() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    // width: auto (default)

    let parent_space = ConstraintSpace::for_root(lu(800), lu(600));
    let input = NewFcLayoutInput {
        style: &style,
        node_id: NodeId::NONE,
        parent_space: &parent_space,
        child_bfc_offset: BfcOffset::new(lu(0), lu(0)),
        container_inline_size: lu(800),
        container_block_size: lu(600),
    };

    let result = layout_new_formatting_context(&input);

    // Auto width with no floats fills the container.
    assert_eq!(result.fragment.size.width, lu(800));
}

#[test]
fn layout_new_fc_auto_width_shrinks_beside_float() {
    let mut style = ComputedStyle::initial();
    style.display = Display::FlowRoot;
    // width: auto (default)

    let mut parent_excl = ExclusionSpace::new();
    parent_excl.add(make_float(ExclusionType::Left, 0, 0, 200, 100));

    let parent_space = ConstraintSpaceBuilder::new()
        .set_available_size(lu(800), lu(600))
        .set_exclusion_space(Some(Arc::new(parent_excl)))
        .build();

    let input = NewFcLayoutInput {
        style: &style,
        node_id: NodeId::NONE,
        parent_space: &parent_space,
        child_bfc_offset: BfcOffset::new(lu(0), lu(0)),
        container_inline_size: lu(800),
        container_block_size: lu(600),
    };

    let result = layout_new_formatting_context(&input);

    // Auto width shrinks to fit beside the float (600px available).
    assert_eq!(result.fragment.size.width, lu(600));
    assert_eq!(result.bfc_line_offset, lu(200));
    assert!(!result.is_pushed_by_floats);
}
