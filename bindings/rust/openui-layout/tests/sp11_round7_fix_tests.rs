//! Tests for SP11 Round 7 fixes.
//!
//! Covers: narrow-box ellipsis, atomic inline vertical-align Length/Percentage,
//! and break-spaces trailing space preservation.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{Display, Overflow, TextOverflow, VerticalAlign, WhiteSpace};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
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
    if fragment.kind == FragmentKind::Box && !fragment.node_id.is_none() {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(collect_box_fragments(child));
    }
    result
}

// ═══════════════════════════════════════════════════════════════════════
// ISSUE 1: NARROW BOX ELLIPSIS
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn narrow_box_ellipsis_still_shows_ellipsis() {
    // A 5px wide box with text-overflow: ellipsis should still produce
    // an ellipsis fragment, not render the full text unclipped.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(block).style.overflow_x = Overflow::Hidden;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello World this is long text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    // 5px wide — too narrow for any content + ellipsis
    let sp = ConstraintSpace::for_block_child(lu_i(5), lu_i(600), lu_i(5), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Should have an ellipsis fragment (NodeId::NONE text with "…" content)
    let all_text = collect_text_fragments(&frag);
    let ellipsis_frags: Vec<_> = all_text
        .iter()
        .filter(|f| f.node_id == NodeId::NONE && f.text_content.as_deref() == Some("\u{2026}"))
        .collect();

    assert!(
        !ellipsis_frags.is_empty(),
        "5px box should still show ellipsis fragment, got {} text fragments: {:?}",
        all_text.len(),
        all_text.iter().map(|f| f.text_content.as_deref()).collect::<Vec<_>>()
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ISSUE 2: ATOMIC INLINE VERTICAL-ALIGN LENGTH / PERCENTAGE
// ═══════════════════════════════════════════════════════════════════════

fn make_atomic_inline_block(
    atomic_w: f32,
    atomic_h: f32,
    valign: VerticalAlign,
) -> (Document, NodeId, NodeId) {
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
    doc.node_mut(atomic).style.width = Length::px(atomic_w);
    doc.node_mut(atomic).style.height = Length::px(atomic_h);
    doc.node_mut(atomic).style.vertical_align = valign;
    doc.append_child(block, atomic);

    (doc, block, atomic)
}

fn do_layout(doc: &Document, block: NodeId) -> Fragment {
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    inline_layout(doc, block, &sp)
}

#[test]
fn atomic_inline_valign_length_shifted_above_baseline() {
    // vertical-align: 10px — atomic should be shifted 10px above baseline
    // compared to the default baseline alignment.
    let (doc_len, block_len, atomic_len) =
        make_atomic_inline_block(50.0, 30.0, VerticalAlign::Length(10.0));
    let frag_len = do_layout(&doc_len, block_len);
    let boxes_len = collect_box_fragments(&frag_len);
    let len_frag: Vec<_> = boxes_len.iter().filter(|f| f.node_id == atomic_len).collect();
    let len_texts = collect_text_fragments(&frag_len);

    let (doc_bl, block_bl, atomic_bl) =
        make_atomic_inline_block(50.0, 30.0, VerticalAlign::Baseline);
    let frag_bl = do_layout(&doc_bl, block_bl);
    let boxes_bl = collect_box_fragments(&frag_bl);
    let bl_frag: Vec<_> = boxes_bl.iter().filter(|f| f.node_id == atomic_bl).collect();
    let bl_texts = collect_text_fragments(&frag_bl);

    assert!(!len_frag.is_empty() && !bl_frag.is_empty());
    assert!(!len_texts.is_empty() && !bl_texts.is_empty());

    // Compare relative offset: atomic.top - text.top
    // Length(10) shifts up, so the gap between atomic top and text top should
    // differ from the baseline case by ~10px.
    let len_gap = len_frag[0].offset.top.to_f32() - len_texts[0].offset.top.to_f32();
    let bl_gap = bl_frag[0].offset.top.to_f32() - bl_texts[0].offset.top.to_f32();

    // The Length(10) gap should be more negative (atomic is higher relative to text)
    assert!(
        len_gap < bl_gap,
        "vertical-align: 10px should shift atomic up relative to text: len_gap={len_gap}, bl_gap={bl_gap}"
    );

    // Also verify the line box grew to accommodate the shift
    let len_line_h = frag_len.children[0].size.height.to_f32();
    let bl_line_h = frag_bl.children[0].size.height.to_f32();
    assert!(
        len_line_h > bl_line_h,
        "Line box should be taller with Length(10): len={len_line_h}, bl={bl_line_h}"
    );
}

#[test]
fn atomic_inline_valign_percentage_shifted() {
    // vertical-align: 50% — should shift by 50% of the element's line-height.
    let (doc_pct, block_pct, atomic_pct) =
        make_atomic_inline_block(50.0, 40.0, VerticalAlign::Percentage(50.0));
    let frag_pct = do_layout(&doc_pct, block_pct);
    let boxes_pct = collect_box_fragments(&frag_pct);
    let pct_frag: Vec<_> = boxes_pct.iter().filter(|f| f.node_id == atomic_pct).collect();
    let pct_texts = collect_text_fragments(&frag_pct);

    let (doc_bl, block_bl, atomic_bl) =
        make_atomic_inline_block(50.0, 40.0, VerticalAlign::Baseline);
    let frag_bl = do_layout(&doc_bl, block_bl);
    let boxes_bl = collect_box_fragments(&frag_bl);
    let bl_frag: Vec<_> = boxes_bl.iter().filter(|f| f.node_id == atomic_bl).collect();
    let bl_texts = collect_text_fragments(&frag_bl);

    assert!(!pct_frag.is_empty() && !bl_frag.is_empty());
    assert!(!pct_texts.is_empty() && !bl_texts.is_empty());

    // Compare relative offset: atomic.top - text.top
    let pct_gap = pct_frag[0].offset.top.to_f32() - pct_texts[0].offset.top.to_f32();
    let bl_gap = bl_frag[0].offset.top.to_f32() - bl_texts[0].offset.top.to_f32();

    // 50% of item_height (40px) = 20px shift up
    assert!(
        pct_gap < bl_gap,
        "vertical-align: 50%% should shift up: pct_gap={pct_gap}, bl_gap={bl_gap}"
    );

    // Line box should be taller with the percentage shift
    let pct_line_h = frag_pct.children[0].size.height.to_f32();
    let bl_line_h = frag_bl.children[0].size.height.to_f32();
    assert!(
        pct_line_h > bl_line_h,
        "Line box should be taller with Percentage(50): pct={pct_line_h}, bl={bl_line_h}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ISSUE 3: BREAK-SPACES TRAILING SPACE PRESERVATION
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn break_spaces_preserves_trailing_spaces() {
    // "hello   " with break-spaces — trailing spaces should be preserved
    // (not stripped), so the line width should be wider than just "hello".
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.white_space = WhiteSpace::BreakSpaces;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("hello   ".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::BreakSpaces;
    doc.append_child(block, t);

    // Wide enough to fit everything on one line
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Compare with "hello" (no trailing spaces) to verify spaces are preserved.
    let mut doc2 = Document::new();
    let root2 = doc2.root();
    let block2 = doc2.create_node(ElementTag::Div);
    doc2.node_mut(block2).style.display = Display::Block;
    doc2.node_mut(block2).style.white_space = WhiteSpace::BreakSpaces;
    doc2.append_child(root2, block2);

    let t2 = doc2.create_node(ElementTag::Text);
    doc2.node_mut(t2).text = Some("hello".to_string());
    doc2.node_mut(t2).style.display = Display::Inline;
    doc2.node_mut(t2).style.white_space = WhiteSpace::BreakSpaces;
    doc2.append_child(block2, t2);

    let sp2 = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag2 = inline_layout(&doc2, block2, &sp2);

    // The text fragments with trailing spaces should be wider
    let text_frags = collect_text_fragments(&frag);
    let text_frags2 = collect_text_fragments(&frag2);
    assert!(!text_frags.is_empty() && !text_frags2.is_empty());

    let with_spaces_width: LayoutUnit = text_frags.iter()
        .map(|f| f.size.width)
        .fold(LayoutUnit::zero(), |a, b| a + b);
    let without_spaces_width: LayoutUnit = text_frags2.iter()
        .map(|f| f.size.width)
        .fold(LayoutUnit::zero(), |a, b| a + b);

    assert!(
        with_spaces_width > without_spaces_width,
        "break-spaces should preserve trailing spaces: with={:?}, without={:?}",
        with_spaces_width, without_spaces_width
    );
}

#[test]
fn break_spaces_wrapping_preserves_space_at_line_end() {
    // With break-spaces and a narrow container, spaces should wrap to
    // the next line rather than being stripped.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.white_space = WhiteSpace::BreakSpaces;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    // "a b" with break-spaces in a very narrow container
    doc.node_mut(t).text = Some("a b c".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::BreakSpaces;
    doc.append_child(block, t);

    // Narrow container — each word + space should wrap
    let sp = ConstraintSpace::for_block_child(lu_i(25), lu_i(600), lu_i(25), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Should produce multiple line boxes (the text wraps)
    let line_boxes: Vec<_> = frag.children.iter()
        .filter(|c| c.kind == FragmentKind::Box && c.node_id.is_none())
        .collect();

    // Compare with pre-wrap to verify break-spaces preserves spaces
    let mut doc2 = Document::new();
    let root2 = doc2.root();
    let block2 = doc2.create_node(ElementTag::Div);
    doc2.node_mut(block2).style.display = Display::Block;
    doc2.node_mut(block2).style.white_space = WhiteSpace::PreWrap;
    doc2.append_child(root2, block2);

    let t2 = doc2.create_node(ElementTag::Text);
    doc2.node_mut(t2).text = Some("a b c".to_string());
    doc2.node_mut(t2).style.display = Display::Inline;
    doc2.node_mut(t2).style.white_space = WhiteSpace::PreWrap;
    doc2.append_child(block2, t2);

    let sp2 = ConstraintSpace::for_block_child(lu_i(25), lu_i(600), lu_i(25), lu_i(600), false);
    let frag2 = inline_layout(&doc2, block2, &sp2);

    let line_boxes2: Vec<_> = frag2.children.iter()
        .filter(|c| c.kind == FragmentKind::Box && c.node_id.is_none())
        .collect();

    // break-spaces should produce at least as many line boxes as pre-wrap
    // (since spaces are not collapsed at line ends)
    assert!(
        line_boxes.len() >= line_boxes2.len(),
        "break-spaces should produce >= lines as pre-wrap: break-spaces={}, pre-wrap={}",
        line_boxes.len(), line_boxes2.len()
    );
}
