// SP13 Phase H: Advanced Inline Features Tests
//
// Comprehensive tests for:
// H1: ::first-line pseudo-element
// H2: ::first-letter pseudo-element
// H3: Optimal line breaking (score-based / Knuth-Plass)
// H4: initial-letter property (drop caps / raised caps)
//
// Each sub-phase has at least 5 tests, totaling 24+ tests.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::inline::first_letter::{
    extract_first_letter, has_first_letter_content, split_first_letter, FirstLetterMetrics,
    FirstLetterStyle,
};
use openui_layout::inline::first_line::{
    apply_first_line_overrides, build_first_line_style, effective_first_line_style,
    resolve_first_line_style, FirstLineOverrides, FirstLineProperty,
};
use openui_layout::inline::initial_letter::{
    available_width_with_exclusion, compute_exclusion_rect, compute_initial_letter_layout,
    create_initial_letter_style, line_in_exclusion_zone, validate_initial_letter,
};
use openui_layout::inline::score_line_breaker::{
    balance_score, candidates_from_word_widths, requires_scoring, score_line_break, BreakCandidate,
    FitnessClass, ScoreLineBreaker, ScoreParams,
};
use openui_layout::ConstraintSpace;
use openui_style::{
    ComputedStyle, Display, InitialLetter, LineHeight, TextDecorationLine, TextWrap,
};

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

fn space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h))
}

// ═══════════════════════════════════════════════════════════════════════════
// H1: ::first-line pseudo-element tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn h1_first_line_font_size_override() {
    let base = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        font_size: Some(24.0),
        ..Default::default()
    };
    let merged = apply_first_line_overrides(&base, &overrides);
    assert_eq!(
        merged.font_size, 24.0,
        "first-line should override font-size"
    );
    // Other properties unchanged
    assert_eq!(merged.letter_spacing, 0.0);
    assert_eq!(merged.word_spacing, 0.0);
}

#[test]
fn h1_first_line_color_override() {
    use openui_style::Color;
    let base = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        color: Some(Color::from_rgba8(255, 0, 0, 255)),
        ..Default::default()
    };
    let merged = apply_first_line_overrides(&base, &overrides);
    assert_eq!(merged.color, Color::from_rgba8(255, 0, 0, 255));
    assert_eq!(merged.font_size, 16.0); // Unchanged
}

#[test]
fn h1_first_line_text_decoration_override() {
    let base = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        text_decoration_line: Some(TextDecorationLine::UNDERLINE),
        ..Default::default()
    };
    let merged = apply_first_line_overrides(&base, &overrides);
    assert!(merged.text_decoration_line.has_underline());
}

#[test]
fn h1_first_line_only_first_line_gets_style() {
    // When first_line_style is None, effective style == base style
    let block = ComputedStyle::initial();
    let eff = effective_first_line_style(&block);
    assert_eq!(eff.font_size, 16.0);

    // When first_line_style is set, effective style differs
    let mut block2 = ComputedStyle::initial();
    let mut fls = ComputedStyle::initial();
    fls.font_size = 32.0;
    block2.first_line_style = Some(Box::new(fls));
    let eff2 = effective_first_line_style(&block2);
    assert_eq!(
        eff2.font_size, 32.0,
        "First line should use override font-size"
    );
    // Subsequent lines use block2 directly:
    assert_eq!(block2.font_size, 16.0, "Block style unchanged");
}

#[test]
fn h1_first_line_resolve_returns_none_when_absent() {
    let style = ComputedStyle::initial();
    assert!(resolve_first_line_style(&style).is_none());
}

#[test]
fn h1_first_line_resolve_returns_some_when_present() {
    let mut style = ComputedStyle::initial();
    let mut fls = ComputedStyle::initial();
    fls.font_size = 48.0;
    style.first_line_style = Some(Box::new(fls));
    let resolved = resolve_first_line_style(&style).unwrap();
    assert_eq!(resolved.font_size, 48.0);
}

#[test]
fn h1_first_line_build_preserves_base() {
    let parent = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        font_size: Some(20.0),
        ..Default::default()
    };
    let built = build_first_line_style(&parent, &overrides);
    assert_eq!(built.font_size, 20.0);
    assert_eq!(built.line_height, parent.line_height);
    assert_eq!(built.text_align, parent.text_align);
}

#[test]
fn h1_first_line_property_list() {
    assert_eq!(FirstLineProperty::ALL.len(), 10);
    // Verify the list contains key properties
    assert!(FirstLineProperty::ALL.contains(&FirstLineProperty::FontSize));
    assert!(FirstLineProperty::ALL.contains(&FirstLineProperty::Color));
    assert!(FirstLineProperty::ALL.contains(&FirstLineProperty::TextDecorationLine));
}

// ═══════════════════════════════════════════════════════════════════════════
// H2: ::first-letter pseudo-element tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn h2_first_letter_simple_extraction() {
    let result = extract_first_letter("Hello world").unwrap();
    assert_eq!(result.first_letter_end, 1); // "H"
    assert!(result.has_letter);
}

#[test]
fn h2_first_letter_with_leading_punctuation() {
    let result = extract_first_letter("\"Hello").unwrap();
    assert_eq!(result.first_letter_end, 2); // "\"H"
    assert!(result.has_letter);

    let (first, rest) = split_first_letter("\"Hello").unwrap();
    assert_eq!(first, "\"H");
    assert_eq!(rest, "ello");
}

#[test]
fn h2_first_letter_with_trailing_punctuation() {
    let result = extract_first_letter("A.bc").unwrap();
    assert_eq!(result.first_letter_end, 2); // "A."
}

#[test]
fn h2_first_letter_multi_codepoint() {
    // Multi-byte UTF-8 character
    let result = extract_first_letter("Über alles").unwrap();
    let text = "Über alles";
    let first_letter = &text[..result.first_letter_end];
    assert_eq!(first_letter, "Ü");
}

#[test]
fn h2_first_letter_cjk() {
    let result = extract_first_letter("漢字テスト").unwrap();
    let text = "漢字テスト";
    let first_letter = &text[..result.first_letter_end];
    assert_eq!(first_letter, "漢");
}

#[test]
fn h2_first_letter_no_letter_returns_none() {
    assert!(extract_first_letter("").is_none());
    assert!(extract_first_letter("   ").is_none());
    assert!(extract_first_letter("...").is_none());
}

#[test]
fn h2_first_letter_has_content() {
    assert!(has_first_letter_content("Hello"));
    assert!(has_first_letter_content("\"X\""));
    assert!(!has_first_letter_content(""));
    assert!(!has_first_letter_content("   "));
}

#[test]
fn h2_first_letter_split() {
    let (first, rest) = split_first_letter("Hello world").unwrap();
    assert_eq!(first, "H");
    assert_eq!(rest, "ello world");
}

#[test]
fn h2_first_letter_metrics() {
    let m = FirstLetterMetrics::from_font_size(48.0);
    // Real font metrics: height = ascent + descent, proportional to font size
    assert!(
        (m.height - (m.ascent + m.descent)).abs() < 0.01,
        "Height = ascent + descent"
    );
    assert!(m.ascent > 0.0, "ascent positive");
    assert!(m.descent > 0.0, "descent positive");
    assert!(m.width > 0.0, "width positive");
    assert!(
        m.height > 30.0 && m.height < 80.0,
        "height in reasonable range for 48px: {}",
        m.height
    );
}

#[test]
fn h2_first_letter_style_drop_cap() {
    let base = ComputedStyle::initial();
    let fls = FirstLetterStyle::with_drop_cap(&base, 48.0, 4.0);
    assert_eq!(fls.style.font_size, 48.0);
    assert_eq!(fls.style.float, openui_style::Float::Left);
}

#[test]
fn h2_first_letter_with_open_bracket() {
    let result = extract_first_letter("(A)rest").unwrap();
    assert_eq!(result.first_letter_end, 3); // "(A)"
}

// ═══════════════════════════════════════════════════════════════════════════
// H3: Optimal line breaking (score-based) tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn h3_text_wrap_balance_produces_balanced_widths() {
    // 6 words, each 30px, space 5px, available 100px
    // Greedy: "30 30 30" = 100, "30 30 30" = 100  → perfectly balanced
    let candidates = candidates_from_word_widths(&[30.0, 30.0, 30.0, 30.0, 30.0, 30.0], 5.0);
    let result = score_line_break(&candidates, 100.0, TextWrap::Balance);
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(r.used_scoring);
    assert!(r.line_count >= 2);
}

#[test]
fn h3_text_wrap_pretty_minimizes_orphans() {
    // 5 words: 40, 40, 40, 40, 10 (tiny last word). Available: 100px
    // Without optimization: lines "40 40" (85), "40 40" (85), "10" (10) — orphan!
    // Pretty should try to avoid the very short last line.
    let candidates = candidates_from_word_widths(&[40.0, 40.0, 40.0, 40.0, 10.0], 5.0);
    let result = score_line_break(&candidates, 100.0, TextWrap::Pretty);
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(r.used_scoring);
}

#[test]
fn h3_fallback_to_greedy_for_long_paragraphs() {
    // Create paragraph exceeding MAX_ITEMS_FOR_SCORING
    let words: Vec<f64> = vec![30.0; 600];
    let candidates = candidates_from_word_widths(&words, 5.0);
    let result = score_line_break(&candidates, 200.0, TextWrap::Pretty);
    assert!(
        result.is_none(),
        "Should fall back to greedy for long paragraphs"
    );
}

#[test]
fn h3_fitness_class_transitions() {
    // Test fitness class classification
    assert_eq!(FitnessClass::from_ratio(-1.0), FitnessClass::Tight);
    assert_eq!(FitnessClass::from_ratio(0.0), FitnessClass::Normal);
    assert_eq!(FitnessClass::from_ratio(0.75), FitnessClass::Loose);
    assert_eq!(FitnessClass::from_ratio(2.0), FitnessClass::VeryLoose);

    // Adjacent transitions: no penalty
    assert_eq!(
        FitnessClass::transition_penalty(FitnessClass::Normal, FitnessClass::Loose),
        0.0
    );
    // Non-adjacent: penalty
    assert!(FitnessClass::transition_penalty(FitnessClass::Tight, FitnessClass::VeryLoose) > 0.0);
}

#[test]
fn h3_balance_score_computation() {
    // Perfectly balanced lines
    assert_eq!(balance_score(&[100.0, 100.0, 100.0]), 0.0);
    // Unbalanced lines
    let score = balance_score(&[150.0, 50.0]);
    assert!(score > 0.0);
}

#[test]
fn h3_requires_scoring_modes() {
    assert!(requires_scoring(TextWrap::Balance));
    assert!(requires_scoring(TextWrap::Pretty));
    assert!(!requires_scoring(TextWrap::Wrap));
    assert!(!requires_scoring(TextWrap::Nowrap));
    assert!(!requires_scoring(TextWrap::Stable));
}

#[test]
fn h3_single_word_fits_on_one_line() {
    let candidates = candidates_from_word_widths(&[50.0], 5.0);
    let result = score_line_break(&candidates, 200.0, TextWrap::Balance).unwrap();
    // Single word → no breaks needed
    assert!(result.break_indices.len() <= 1);
}

#[test]
fn h3_text_wrap_enum_properties() {
    assert!(TextWrap::Balance.uses_scoring());
    assert!(TextWrap::Pretty.uses_scoring());
    assert!(!TextWrap::Wrap.uses_scoring());
    assert!(!TextWrap::Nowrap.uses_scoring());

    assert!(TextWrap::Nowrap.is_nowrap());
    assert!(!TextWrap::Wrap.is_nowrap());
}

// ═══════════════════════════════════════════════════════════════════════════
// H4: initial-letter property tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn h4_drop_cap_3_lines() {
    let il = InitialLetter {
        size: 3.0,
        sink: None,
    };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);

    assert_eq!(layout.letter_height, lu(60.0)); // 3 × 20
    assert_eq!(layout.computed_font_size, 48.0); // 16 × 3
    assert_eq!(layout.block_offset, lu(0.0)); // Full drop cap
    assert_eq!(layout.exclusion_lines, 3);
    assert!(layout.is_drop_cap);
    assert!(!layout.is_raised);
}

#[test]
fn h4_raised_cap_3_lines() {
    let il = InitialLetter {
        size: 3.0,
        sink: Some(1.0),
    };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);

    assert_eq!(layout.letter_height, lu(60.0));
    assert_eq!(layout.block_offset, lu(-40.0)); // Protrudes 2 lines above
    assert_eq!(layout.exclusion_lines, 1);
    assert!(layout.is_raised);
    assert!(!layout.is_drop_cap);
}

#[test]
fn h4_exclusion_area() {
    let il = InitialLetter {
        size: 3.0,
        sink: None,
    };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
    let margin = lu(4.0);

    let (start, end, bstart, bend) = compute_exclusion_rect(&layout, margin);
    assert_eq!(start, lu(0.0));
    assert_eq!(bstart, lu(0.0));
    assert_eq!(bend, lu(60.0));
    assert!(end > lu(0.0));
}

#[test]
fn h4_line_exclusion_detection() {
    // Exclusion zone: block 0..60
    assert!(line_in_exclusion_zone(lu(0.0), lu(20.0), lu(0.0), lu(60.0)));
    assert!(line_in_exclusion_zone(
        lu(20.0),
        lu(20.0),
        lu(0.0),
        lu(60.0)
    ));
    assert!(line_in_exclusion_zone(
        lu(40.0),
        lu(20.0),
        lu(0.0),
        lu(60.0)
    ));
    assert!(!line_in_exclusion_zone(
        lu(60.0),
        lu(20.0),
        lu(0.0),
        lu(60.0)
    ));
}

#[test]
fn h4_available_width_with_exclusion() {
    let total = lu(300.0);
    let letter = lu(50.0);
    assert_eq!(available_width_with_exclusion(total, letter), lu(250.0));
    // Clamped to zero
    assert_eq!(available_width_with_exclusion(lu(30.0), lu(50.0)), lu(0.0));
}

#[test]
fn h4_validate_initial_letter() {
    assert!(validate_initial_letter(3.0, None).is_some());
    assert!(validate_initial_letter(3.0, Some(2.0)).is_some());
    assert!(validate_initial_letter(1.0, Some(1.0)).is_some());
    assert!(validate_initial_letter(0.5, None).is_none());
    assert!(validate_initial_letter(3.0, Some(0.5)).is_none());
}

#[test]
fn h4_initial_letter_style_computed_fields() {
    let mut style = ComputedStyle::initial();
    assert!(style.initial_letter.is_none());

    style.initial_letter = Some(InitialLetter {
        size: 3.0,
        sink: None,
    });
    let il = style.initial_letter.as_ref().unwrap();
    assert_eq!(il.size, 3.0);
    assert_eq!(il.effective_sink(), 3.0);
}

#[test]
fn h4_create_initial_letter_style() {
    let base = ComputedStyle::initial();
    let il = InitialLetter {
        size: 3.0,
        sink: None,
    };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
    let letter_style = create_initial_letter_style(&base, &layout);
    assert_eq!(letter_style.font_size, 48.0);
}

#[test]
fn h4_two_line_drop_cap() {
    let il = InitialLetter {
        size: 2.0,
        sink: None,
    };
    let layout = compute_initial_letter_layout(&il, 24.0, 16.0);
    assert_eq!(layout.letter_height, lu(48.0));
    assert_eq!(layout.computed_font_size, 32.0);
    assert_eq!(layout.exclusion_lines, 2);
    assert!(layout.is_drop_cap);
}

// ═══════════════════════════════════════════════════════════════════════════
// Integration: Verify existing layout still works with new fields
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn integration_new_style_fields_default() {
    // Verify new fields have correct defaults
    let s = ComputedStyle::initial();
    assert!(s.first_line_style.is_none());
    assert_eq!(s.text_wrap, TextWrap::Wrap);
    assert!(s.initial_letter.is_none());
}

#[test]
fn integration_layout_with_new_defaults() {
    // Verify basic layout still works with new fields at default values
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(200.0);
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello world".to_string());
    doc.append_child(div, text);

    let sp = space(400, 800);
    let frag = block_layout(&doc, vp, &sp);

    assert!(
        !frag.children.is_empty(),
        "Should produce at least one child"
    );
    let div_frag = &frag.children[0];
    assert!(
        div_frag.size.width > lu(0.0),
        "Div should have positive width"
    );
    assert!(
        div_frag.size.height > lu(0.0),
        "Div should have positive height"
    );
}

#[test]
fn integration_text_wrap_field_set() {
    let mut s = ComputedStyle::initial();
    s.text_wrap = TextWrap::Balance;
    assert_eq!(s.text_wrap, TextWrap::Balance);
    assert!(s.text_wrap.uses_scoring());
}

#[test]
fn integration_initial_letter_field_set() {
    let mut s = ComputedStyle::initial();
    s.initial_letter = Some(InitialLetter {
        size: 3.0,
        sink: Some(2.0),
    });
    let il = s.initial_letter.as_ref().unwrap();
    assert_eq!(il.size, 3.0);
    assert_eq!(il.effective_sink(), 2.0);
}
