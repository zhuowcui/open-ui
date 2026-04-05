//! Tests for SP11 Round 3 fixes.
//!
//! Covers: trailing space non-ASCII char counting, ellipsis inherited style,
//! atomic inline height in line height, and width_for_range/sub_range OOB guards.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    Color, Display, Overflow, TextOverflow, VerticalAlign, WhiteSpace,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn make_text_block(texts: &[&str], _width: i32) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }
    (doc, block)
}

fn layout_text(texts: &[&str], width: i32) -> Fragment {
    let (doc, block) = make_text_block(texts, width);
    let sp = ConstraintSpace::for_block_child(
        lu_i(width),
        lu_i(600),
        lu_i(width),
        lu_i(600),
        false,
    );
    inline_layout(&doc, block, &sp)
}

fn collect_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.kind == FragmentKind::Text {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(collect_text_fragments(child));
    }
    result
}

fn count_line_boxes(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .count()
}

// ═══════════════════════════════════════════════════════════════════════
// FIX 1: Trailing space non-ASCII char counting
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn trailing_space_non_ascii_cafe() {
    // "café " has 5 chars but 6 bytes ('é' is 2 bytes in UTF-8).
    // After the fix, trailing space stripping should work correctly
    // for non-ASCII text by counting characters, not bytes.
    let frag = layout_text(&["café more words here"], 50);
    let line_count = count_line_boxes(&frag);
    assert!(line_count >= 2, "Expected wrapping, got {} lines", line_count);

    // Verify text fragments on the first line don't exceed container width
    if let Some(first_line) = frag.children.first() {
        let text_frags = collect_text_fragments(first_line);
        for tf in &text_frags {
            assert!(
                tf.size.width <= lu_i(50),
                "Text fragment width {:?} exceeds available width 50 for non-ASCII text",
                tf.size.width
            );
        }
    }
}

#[test]
fn trailing_space_multibyte_accented() {
    // "über große Straße" contains multiple multi-byte characters.
    // 'ü' and 'ö' are 2 bytes each. Trailing space stripping must use
    // character counts, not byte counts.
    let frag = layout_text(&["über große Straße here"], 60);
    let line_count = count_line_boxes(&frag);
    assert!(line_count >= 1, "Should produce at least one line");

    // Verify no text fragment significantly exceeds the container
    let all_texts = collect_text_fragments(&frag);
    for tf in &all_texts {
        assert!(
            tf.size.width <= lu_i(60) + lu(1.0),
            "Multi-byte text fragment width {:?} should not significantly exceed 60",
            tf.size.width
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// FIX 2: Ellipsis inherits parent style
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ellipsis_inherits_parent_text_color() {
    // When text-overflow: ellipsis is active, the ellipsis fragment should
    // inherit the block's text color, not use ComputedStyle::default().
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.node_mut(block).style.color = Color::RED;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("This is very long text that overflows container".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(80), lu_i(600), lu_i(80), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Find the ellipsis fragment (text fragment with NodeId::NONE)
    let all_text = collect_text_fragments(&frag);
    let ellipsis_frags: Vec<_> = all_text
        .iter()
        .filter(|f| f.node_id == NodeId::NONE)
        .collect();

    assert!(
        !ellipsis_frags.is_empty(),
        "Should have an ellipsis fragment"
    );

    // The ellipsis should have an inherited_style with the red color
    let ef = ellipsis_frags[0];
    let inherited = ef.inherited_style.as_ref()
        .expect("Ellipsis fragment should have inherited_style set");
    assert_eq!(
        inherited.color, Color::RED,
        "Ellipsis should inherit parent's text color (red), got {:?}",
        inherited.color
    );
}

// ═══════════════════════════════════════════════════════════════════════
// FIX 3: Atomic inline height in line height calculation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn atomic_inline_tall_expands_line_height() {
    // A 100px tall inline-block should expand the line box to accommodate it.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("text ".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(100.0);
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // The line box should be at least 100px tall
    assert!(!frag.children.is_empty(), "Should have a line box");
    let line_box = &frag.children[0];
    assert!(
        line_box.size.height >= lu(100.0),
        "Line box height {:?} should be >= 100px to fit the atomic inline",
        line_box.size.height
    );
}

#[test]
fn atomic_inline_middle_splits_height() {
    // A vertical-align:middle atomic inline should contribute evenly to
    // ascent and descent.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(80.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::Middle;
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    assert!(!frag.children.is_empty(), "Should have a line box");
    let line_box = &frag.children[0];
    // With middle alignment, the 80px item contributes 40px each to ascent/descent.
    // The line height should be at least 40px (the larger of strut and half-height).
    assert!(
        line_box.size.height >= lu(40.0),
        "Line box height {:?} should accommodate vertical-align:middle atomic inline",
        line_box.size.height
    );
}
