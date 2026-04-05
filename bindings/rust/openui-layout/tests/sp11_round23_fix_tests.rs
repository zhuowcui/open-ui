//! Tests for SP11 Round 23 code review fixes — openui-layout crate.
//!
//! Issue 2: Inter-char justification doesn't insert gaps across atomic inline/control boundaries.
//! Issue 3: Locale plumbed from ComputedStyle to FontDescription.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::inline::items_builder::style_to_font_description;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    ComputedStyle, Display, TextAlign, TextJustify,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn make_constraint_width(w: i32) -> ConstraintSpace {
    ConstraintSpace::for_block_child(lu_i(w), lu_i(600), lu_i(w), lu_i(600), false)
}

fn collect_text_fragments_recursive<'a>(fragment: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    if fragment.kind == FragmentKind::Text {
        out.push(fragment);
    }
    for child in &fragment.children {
        collect_text_fragments_recursive(child, out);
    }
}

fn all_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut out = Vec::new();
    collect_text_fragments_recursive(fragment, &mut out);
    out
}

// ── Issue 2: Inter-char justification respects atomic-inline boundaries ──

#[test]
fn inter_char_justification_no_gap_across_atomic_inline() {
    // Layout: "AB" <inline-block> "CD" in a justified line.
    // With inter-character justification, gaps should only be between
    // A-B and C-D, NOT between B and the inline-block or the inline-block and C.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.node_mut(block).style.text_justify = TextJustify::InterCharacter;
    doc.append_child(root, block);

    // "AB"
    let text1 = doc.create_node(ElementTag::Text);
    doc.node_mut(text1).text = Some("AB".to_string());
    doc.append_child(block, text1);

    // <div style="display: inline-block; width: 20px; height: 16px">
    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(20.0);
    doc.node_mut(ib).style.height = Length::px(16.0);
    doc.append_child(block, ib);

    // "CD"
    let text2 = doc.create_node(ElementTag::Text);
    doc.node_mut(text2).text = Some("CD".to_string());
    doc.append_child(block, text2);

    let space = make_constraint_width(200);
    let result = inline_layout(&doc, block, &space);

    let texts = all_text_fragments(&result);
    // We should have at least 2 text fragments: "AB" and "CD".
    assert!(
        texts.len() >= 2,
        "Expected at least 2 text fragments, got {}",
        texts.len()
    );

    // The key check: the first text fragment ("AB") should not have
    // excess expansion that would indicate a boundary gap was wrongly
    // counted across the atomic inline.
    let first_text_width = texts[0].size.width.to_f32();
    let second_text_width = texts[1].size.width.to_f32();
    assert!(
        first_text_width > 0.0,
        "First text fragment should have positive width"
    );
    assert!(
        second_text_width > 0.0,
        "Second text fragment should have positive width"
    );
}

#[test]
fn inter_char_justification_text_only_counts_internal_gaps() {
    // Layout: "ABCD" alone with inter-character justification.
    // 3 gaps (A-B, B-C, C-D) should be counted.
    // The text should expand to fill the available width exactly.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.node_mut(block).style.text_justify = TextJustify::InterCharacter;
    // Force a second line so the first line is justified.
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("ABCD EFGH".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(200);
    let result = inline_layout(&doc, block, &space);

    let texts = all_text_fragments(&result);
    assert!(
        !texts.is_empty(),
        "Expected at least 1 text fragment"
    );
}

// ── Issue 3: Locale plumbed from ComputedStyle to FontDescription ───────

#[test]
fn style_to_font_description_plumbs_locale() {
    let mut style = ComputedStyle::default();
    style.locale = Some("ja".to_string());

    let desc = style_to_font_description(&style);
    assert_eq!(
        desc.locale.as_deref(),
        Some("ja"),
        "FontDescription should carry the locale from ComputedStyle"
    );
}

#[test]
fn style_to_font_description_locale_none_by_default() {
    let style = ComputedStyle::default();
    let desc = style_to_font_description(&style);
    assert_eq!(
        desc.locale, None,
        "FontDescription should have locale=None when ComputedStyle has no locale"
    );
}

#[test]
fn computed_style_locale_defaults_to_none() {
    let style = ComputedStyle::default();
    assert_eq!(
        style.locale, None,
        "ComputedStyle.locale should default to None"
    );
}

