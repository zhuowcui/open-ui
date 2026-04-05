//! Tests for SP11 Round 9 fixes.
//!
//! Issue 1: vertical-align: middle should note parent inline x-height limitation.
//! Issue 2: Operator precedence bug in Length/Percentage shifted_ascent.
//! Issue 3: text-top/text-bottom/sub/super missing STEP 2 line metrics.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{Display, VerticalAlign};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn collect_box_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    for child in &fragment.children {
        if child.kind == FragmentKind::Box {
            result.push(child);
        }
        result.extend(collect_box_fragments(child));
    }
    result
}

fn make_constraint() -> ConstraintSpace {
    ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false)
}

// ── Issue 2: Operator precedence — negative length shift ────────────────

#[test]
fn atomic_inline_valign_negative_length_reduces_line_height() {
    // A 48px atomic inline with vertical-align: -5px.
    // Ascent should be (48 + (-5)).max(0) = 43, not 48.
    // Descent should be 5px.
    // Total line box height should be less than 48 (plus strut).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(48.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::Length(-5.0);
    doc.append_child(block, atomic);

    let frag = inline_layout(&doc, block, &make_constraint());
    assert!(!frag.children.is_empty(), "Should have a line box");
    let line_box = &frag.children[0];

    // With the precedence fix, ascent = 43, descent = 5 → total = 48.
    // Before the fix, ascent was 48 (ignoring the shift), giving a taller line.
    // The line box should be <= 48px (item_height) since the shift moves
    // the item *down*, not making the line taller.
    assert!(
        line_box.size.height <= lu(48.0),
        "Line box height {:?} should be <= 48px for vertical-align:-5px on a 48px item",
        line_box.size.height,
    );

    // The atomic inline should be positioned below baseline (shifted down).
    let boxes = collect_box_fragments(line_box);
    assert!(!boxes.is_empty(), "Should have atomic inline box");
}

// ── Issue 3: text-top — no overflow ─────────────────────────────────────

#[test]
fn atomic_inline_text_top_no_overflow() {
    // A 48px image with text-top in 16px text.
    // Item top aligns with font ascent; the rest extends below.
    // The line box must be tall enough so the item doesn't overflow.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(48.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::TextTop;
    doc.append_child(block, atomic);

    let frag = inline_layout(&doc, block, &make_constraint());
    assert!(!frag.children.is_empty());
    let line_box = &frag.children[0];

    // Line box must be tall enough for the 48px item.
    assert!(
        line_box.size.height >= lu(48.0),
        "Line box {:?} must be >= 48px to fit text-top aligned 48px item",
        line_box.size.height,
    );

    // The item must not protrude above the line box.
    let boxes = collect_box_fragments(line_box);
    assert!(!boxes.is_empty());
    assert!(
        boxes[0].offset.top >= lu(0.0),
        "text-top item offset {:?} must be >= 0 (no overflow above)",
        boxes[0].offset.top,
    );
}

// ── Issue 3: text-bottom — correct descent contribution ─────────────────

#[test]
fn atomic_inline_text_bottom_no_overflow() {
    // A 48px item with text-bottom alignment.
    // Item bottom aligns with font descent line.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(48.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::TextBottom;
    doc.append_child(block, atomic);

    let frag = inline_layout(&doc, block, &make_constraint());
    assert!(!frag.children.is_empty());
    let line_box = &frag.children[0];

    // Line box must accommodate the 48px item.
    assert!(
        line_box.size.height >= lu(48.0),
        "Line box {:?} must be >= 48px to fit text-bottom aligned 48px item",
        line_box.size.height,
    );

    // Item bottom must not overflow below line box.
    let boxes = collect_box_fragments(line_box);
    assert!(!boxes.is_empty());
    let item_bottom = boxes[0].offset.top + boxes[0].size.height;
    assert!(
        item_bottom <= line_box.size.height + lu(1.0),
        "text-bottom item bottom {:?} should not overflow below line box {:?}",
        item_bottom,
        line_box.size.height,
    );
}

// ── Issue 3: sub — lowered correctly, line expanded ─────────────────────

#[test]
fn atomic_inline_sub_line_expanded() {
    // A 30px item with vertical-align:sub.
    // Sub shifts the item down; the line must expand to hold it.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(40.0);
    doc.node_mut(atomic).style.height = Length::px(30.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::Sub;
    doc.append_child(block, atomic);

    let frag = inline_layout(&doc, block, &make_constraint());
    assert!(!frag.children.is_empty());
    let line_box = &frag.children[0];

    // The line box must be at least 30px (item height).
    assert!(
        line_box.size.height >= lu(30.0),
        "Line box {:?} must be >= 30px for sub-aligned 30px item",
        line_box.size.height,
    );

    // The item must not overflow below the line box.
    let boxes = collect_box_fragments(line_box);
    assert!(!boxes.is_empty());
    let item_bottom = boxes[0].offset.top + boxes[0].size.height;
    assert!(
        item_bottom <= line_box.size.height + lu(1.0),
        "Sub item bottom {:?} should not overflow below line box {:?}",
        item_bottom,
        line_box.size.height,
    );
}
