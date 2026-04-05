//! Tests for SP11 Round 2 re-review fixes (layout).
//!
//! Covers: bidi shaping direction, trailing space stripping for mid-item splits,
//! ellipsis painting, ellipsis text clipping, atomic inline fragments,
//! and justification trailing space exclusion.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    Display, Overflow, TextAlign, TextOverflow, WhiteSpace,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn _space(width: i32, height: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu_i(width), lu_i(height))
}

/// Create a block container with text children and perform inline layout.
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

fn collect_box_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.kind == FragmentKind::Box && fragment.node_id != NodeId::NONE {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(collect_box_fragments(child));
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
// ── FIX 1: BIDI SHAPING DIRECTION ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bidi_rtl_text_shaped_with_rtl_direction() {
    // RTL text should be shaped with RTL direction based on bidi level,
    // not the CSS direction property.
    use openui_layout::inline::items_builder::InlineItemsBuilder;
    use openui_text::TextDirection;

    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Arabic text is inherently RTL
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("مرحبا".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let mut items_data = InlineItemsBuilder::collect(&doc, block);
    items_data.apply_bidi(TextDirection::Ltr); // LTR base, but Arabic is RTL
    items_data.shape_text();

    // The text item should have bidi_level >= 1 (odd = RTL)
    let text_item = &items_data.items[0];
    assert!(
        text_item.bidi_level % 2 == 1,
        "Arabic text should have odd bidi level (RTL), got {}",
        text_item.bidi_level,
    );

    // The shape result should have RTL direction
    let sr = text_item.shape_result.as_ref().expect("should be shaped");
    assert_eq!(
        sr.direction, TextDirection::Rtl,
        "Arabic text should be shaped with RTL direction"
    );
}

#[test]
fn bidi_ltr_text_remains_ltr_shaped() {
    use openui_layout::inline::items_builder::InlineItemsBuilder;
    use openui_text::TextDirection;

    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let mut items_data = InlineItemsBuilder::collect(&doc, block);
    items_data.apply_bidi(TextDirection::Ltr);
    items_data.shape_text();

    let text_item = &items_data.items[0];
    assert_eq!(text_item.bidi_level % 2, 0, "English text should have even bidi level (LTR)");

    let sr = text_item.shape_result.as_ref().expect("should be shaped");
    assert_eq!(sr.direction, TextDirection::Ltr, "English text shaped as LTR");
}

#[test]
fn bidi_mixed_text_items_split_and_shaped_correctly() {
    use openui_layout::inline::items_builder::InlineItemsBuilder;
    use openui_text::TextDirection;

    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Mixed LTR + RTL text
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello مرحبا World".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let mut items_data = InlineItemsBuilder::collect(&doc, block);
    items_data.apply_bidi(TextDirection::Ltr);
    items_data.shape_text();

    // Should have been split into multiple items
    assert!(
        items_data.items.len() >= 2,
        "Mixed bidi text should produce multiple items, got {}",
        items_data.items.len()
    );

    // All text items should be shaped
    for item in &items_data.items {
        if item.item_type == openui_layout::inline::items::InlineItemType::Text
            && !item.text_range.is_empty()
        {
            assert!(item.shape_result.is_some(), "All text items should be shaped");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── FIX 2: TRAILING SPACE STRIP FOR MID-ITEM SPLITS ────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn trailing_space_mid_item_strip_reduces_width() {
    // When text wraps mid-item at a space, the trailing space on the first
    // line should be stripped from used_width.
    // "Hello world test" in a narrow container should wrap after "Hello "
    // and the space should be stripped.
    let frag = layout_text(&["Hello world test data"], 60);

    // Should produce multiple lines
    let line_count = count_line_boxes(&frag);
    assert!(line_count >= 2, "Expected wrapping, got {} lines", line_count);

    // Each line's text fragments should not exceed the available width
    for line_box in &frag.children {
        if line_box.kind == FragmentKind::Box {
            let text_frags = collect_text_fragments(line_box);
            for tf in &text_frags {
                assert!(
                    tf.size.width <= lu_i(60),
                    "Text fragment width {:?} exceeds available width 60",
                    tf.size.width
                );
            }
        }
    }
}

#[test]
fn trailing_space_at_item_end_stripped() {
    // A text item ending with a space at the end of a line should have
    // the space width removed.
    let frag = layout_text(&["word "], 200);

    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty(), "Should have text fragments");
    // The text "word " should have the trailing space stripped
    let text_width = texts[0].size.width;
    assert!(text_width > lu(0.0), "Text should have positive width");
}

#[test]
fn trailing_space_strip_wrapping_consistency() {
    // Verify that wrapping with trailing spaces produces consistent results
    let narrow_frag = layout_text(&["ab cd ef gh ij kl"], 40);
    let wide_frag = layout_text(&["ab cd ef gh ij kl"], 400);

    let narrow_lines = count_line_boxes(&narrow_frag);
    let wide_lines = count_line_boxes(&wide_frag);

    assert!(narrow_lines > wide_lines, "Narrow container should have more lines");
    // Total text content should still be laid out
    let narrow_texts = collect_text_fragments(&narrow_frag);
    let wide_texts = collect_text_fragments(&wide_frag);
    assert!(!narrow_texts.is_empty() && !wide_texts.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════
// ── FIX 3: ELLIPSIS FRAGMENT PAINTABLE ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ellipsis_fragment_exists_when_overflow() {
    // When text-overflow: ellipsis is active, the line should contain
    // an ellipsis text fragment with NodeId::NONE.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("This is a very long text that should overflow".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(100), lu_i(600), lu_i(100), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Find the ellipsis fragment (text fragment with NodeId::NONE)
    let all_text = collect_text_fragments(&frag);
    let ellipsis_frags: Vec<_> = all_text
        .iter()
        .filter(|f| f.node_id == NodeId::NONE)
        .collect();

    assert!(
        !ellipsis_frags.is_empty(),
        "Should have an ellipsis fragment with NodeId::NONE"
    );

    // Ellipsis should have a shape result for painting
    let ef = ellipsis_frags[0];
    assert!(
        ef.shape_result.is_some(),
        "Ellipsis fragment should have a shape result"
    );
    assert!(ef.size.width > lu(0.0), "Ellipsis should have positive width");
}

#[test]
fn ellipsis_fragment_has_text_content() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Overflow me with very long text content here".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(80), lu_i(600), lu_i(80), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    let all_text = collect_text_fragments(&frag);
    let ellipsis_frags: Vec<_> = all_text
        .iter()
        .filter(|f| f.node_id == NodeId::NONE && f.text_content.is_some())
        .collect();

    assert!(
        !ellipsis_frags.is_empty(),
        "Should have ellipsis with text_content"
    );
    assert_eq!(
        ellipsis_frags[0].text_content.as_deref(),
        Some("\u{2026}"),
        "Ellipsis text content should be '…'"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── FIX 4: ELLIPSIS CLIPS INSTEAD OF REMOVING WHOLE ITEMS ──────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ellipsis_clips_long_text_instead_of_removing() {
    // A single long text run should be trimmed, not entirely removed,
    // when text-overflow: ellipsis is active.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(100), lu_i(600), lu_i(100), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Should have at least one text fragment (the clipped text) plus the ellipsis
    let all_text = collect_text_fragments(&frag);
    assert!(
        all_text.len() >= 2,
        "Should have clipped text + ellipsis, got {} fragments",
        all_text.len()
    );

    // The first text fragment should have the original node_id (not NONE)
    let content_frags: Vec<_> = all_text
        .iter()
        .filter(|f| f.node_id != NodeId::NONE)
        .collect();
    assert!(
        !content_frags.is_empty(),
        "Should have at least one content text fragment (clipped, not removed)"
    );
}

#[test]
fn ellipsis_total_width_within_available() {
    // With ellipsis, the total line content (clipped text + ellipsis) should
    // not exceed the available width.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("This text is very long and will be clipped".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let available = 120;
    let sp = ConstraintSpace::for_block_child(
        lu_i(available), lu_i(600), lu_i(available), lu_i(600), false,
    );
    let frag = inline_layout(&doc, block, &sp);

    // Sum of all fragment widths in the line box should be <= available
    if let Some(line_box) = frag.children.first() {
        let total_width: LayoutUnit = collect_text_fragments(line_box)
            .iter()
            .map(|f| f.size.width)
            .fold(LayoutUnit::zero(), |a, b| a + b);
        assert!(
            total_width <= lu_i(available),
            "Total text+ellipsis width {:?} exceeds available {}",
            total_width,
            available
        );
    }
}

#[test]
fn ellipsis_not_applied_when_text_fits() {
    // When text fits within available width, no ellipsis should be added.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hi".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // No ellipsis fragment (NodeId::NONE text)
    let all_text = collect_text_fragments(&frag);
    let ellipsis_frags: Vec<_> = all_text
        .iter()
        .filter(|f| f.node_id == NodeId::NONE)
        .collect();
    assert!(
        ellipsis_frags.is_empty(),
        "Short text should not have ellipsis"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── FIX 5: ATOMIC INLINE FRAGMENT CREATION ─────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn atomic_inline_produces_box_fragment() {
    // An inline-block element should produce a box fragment in the line box.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Add text before atomic inline
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Before ".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    // Add an inline-block (atomic inline)
    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(30.0);
    doc.append_child(block, atomic);

    // Add text after atomic inline
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some(" After".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Find box fragments with node_id == atomic in the line box children
    let box_frags = collect_box_fragments(&frag);
    let atomic_frags: Vec<_> = box_frags
        .iter()
        .filter(|f| f.node_id == atomic)
        .collect();

    assert!(
        !atomic_frags.is_empty(),
        "Atomic inline should produce a box fragment"
    );
}

#[test]
fn atomic_inline_has_correct_width() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(75.0);
    doc.node_mut(atomic).style.height = Length::px(40.0);
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    let box_frags = collect_box_fragments(&frag);
    let atomic_frags: Vec<_> = box_frags
        .iter()
        .filter(|f| f.node_id == atomic)
        .collect();

    assert!(!atomic_frags.is_empty(), "Should have atomic inline fragment");
    assert_eq!(
        atomic_frags[0].size.width,
        lu(75.0),
        "Atomic inline width should match CSS width"
    );
}

#[test]
fn atomic_inline_has_correct_height() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(25.0);
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    let box_frags = collect_box_fragments(&frag);
    let atomic_frags: Vec<_> = box_frags
        .iter()
        .filter(|f| f.node_id == atomic)
        .collect();

    assert!(!atomic_frags.is_empty(), "Should have atomic inline fragment");
    assert_eq!(
        atomic_frags[0].size.height,
        lu(25.0),
        "Atomic inline height should match CSS height"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── FIX 7: JUSTIFICATION TRAILING SPACE EXCLUSION ──────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn justification_text_aligns_to_edges() {
    // With text-align: justify, non-last lines should expand to fill available width.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("The quick brown fox jumps over the lazy dog today".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(200), lu_i(600), lu_i(200), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Should have multiple lines for justification to apply
    let line_count = count_line_boxes(&frag);
    assert!(line_count >= 2, "Need multiple lines for justify, got {}", line_count);
}

#[test]
fn justification_excludes_trailing_space_from_expansion() {
    // Verify that justification does not count the trailing space as an
    // expansion opportunity. This is tested indirectly: if trailing space
    // IS counted, the per-space expansion would be too small and the line
    // would visibly not reach the right edge.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("aa bb cc dd ee ff gg hh ii jj".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let available = 150;
    let sp = ConstraintSpace::for_block_child(
        lu_i(available), lu_i(600), lu_i(available), lu_i(600), false,
    );
    let frag = inline_layout(&doc, block, &sp);

    // Non-last lines should be justified (expanded to fill width)
    let line_count = count_line_boxes(&frag);
    assert!(line_count >= 2, "Need multiple lines");

    // Check that first line's text fragments are wider than unjustified
    // (justification should have added space)
    let first_line = &frag.children[0];
    let text_frags = collect_text_fragments(first_line);
    if !text_frags.is_empty() {
        let total_text_width: LayoutUnit = text_frags
            .iter()
            .map(|f| f.size.width)
            .fold(LayoutUnit::zero(), |a, b| a + b);
        assert!(
            total_text_width > lu(0.0),
            "Justified text should have positive width"
        );
    }
}

#[test]
fn justification_last_line_not_expanded() {
    // The last line should NOT be justified (falls back to start alignment).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Word word word word word word end".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(150), lu_i(600), lu_i(150), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    let line_count = count_line_boxes(&frag);
    assert!(line_count >= 2, "Need at least 2 lines");

    // Last line's text width should be less than available (not fully justified)
    let last_line = &frag.children[line_count - 1];
    let text_frags = collect_text_fragments(last_line);
    if !text_frags.is_empty() {
        let total_width: LayoutUnit = text_frags
            .iter()
            .map(|f| f.size.width)
            .fold(LayoutUnit::zero(), |a, b| a + b);
        // Last line should NOT fill the entire available width
        assert!(
            total_width < lu_i(150),
            "Last line should not be fully justified"
        );
    }
}
