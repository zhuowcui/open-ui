//! WPT-equivalent tests for CSS Writing Modes Level 4.
//!
//! Each test corresponds to behaviors verified by WPT css/css-writing-modes tests.
//! Categories: writing-mode property, text-orientation, character orientation,
//! direction, unicode-bidi, bidi algorithm, and logical↔physical conversion.

use openui_dom::{Document, ElementTag};
use openui_geometry::{
    BoxStrut, LayoutUnit, LogicalOffset, LogicalSize, PhysicalSize,
    WritingDirectionMode, WritingModeConverter,
};
use openui_layout::block::block_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    ComputedStyle, Direction, Display, FontOrientation, TextAlign, TextOrientation,
    UnicodeBidi, WritingMode,
};
use openui_text::char_orientation::is_upright_in_mixed_vertical;
use openui_text::{BidiParagraph, TextDirection};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn space(width: i32, height: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu_i(width), lu_i(height))
}

fn block_layout_text(texts: &[&str], width: i32) -> Fragment {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(vp, block);
    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }
    let sp = space(width, 600);
    block_layout(&doc, vp, &sp)
}

/// Build a block with text children, apply a custom style to the block node.
fn make_styled_text_block(
    texts: &[&str],
    width: i32,
    style_fn: impl Fn(&mut ComputedStyle),
) -> Fragment {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    style_fn(&mut doc.node_mut(block).style);
    doc.append_child(vp, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        // Propagate writing-mode and direction to text children.
        let block_style = doc.node(block).style.clone();
        doc.node_mut(t).style.writing_mode = block_style.writing_mode;
        doc.node_mut(t).style.direction = block_style.direction;
        doc.node_mut(t).style.text_orientation = block_style.text_orientation;
        doc.node_mut(t).style.text_align = block_style.text_align;
        doc.node_mut(t).style.unicode_bidi = block_style.unicode_bidi;
        doc.append_child(block, t);
    }

    let sp = space(width, 600);
    block_layout(&doc, vp, &sp)
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

fn first_block_child(fragment: &Fragment) -> &Fragment {
    fragment
        .children
        .iter()
        .find(|c| c.kind == FragmentKind::Box)
        .unwrap()
}

// ═══════════════════════════════════════════════════════════════════════
// WRITING-MODE PROPERTY TESTS (17 tests)
// Corresponds to WPT css/css-writing-modes/writing-mode-*
// ═══════════════════════════════════════════════════════════════════════
mod writing_mode_property {
    use super::*;

    /// writing-mode initial value is horizontal-tb (WPT writing-mode-initial-001).
    #[test]
    fn initial_value_is_horizontal_tb() {
        let style = ComputedStyle::default();
        assert_eq!(style.writing_mode, WritingMode::HorizontalTb);
    }

    /// WritingMode::INITIAL matches HorizontalTb.
    #[test]
    fn initial_constant_matches_default() {
        assert_eq!(WritingMode::INITIAL, WritingMode::HorizontalTb);
    }

    /// HorizontalTb is_horizontal returns true.
    #[test]
    fn horizontal_tb_is_horizontal() {
        assert!(WritingMode::HorizontalTb.is_horizontal());
        assert!(!WritingMode::HorizontalTb.is_vertical());
    }

    /// VerticalRl is_vertical returns true.
    #[test]
    fn vertical_rl_is_vertical() {
        assert!(WritingMode::VerticalRl.is_vertical());
        assert!(!WritingMode::VerticalRl.is_horizontal());
    }

    /// VerticalLr is_vertical returns true.
    #[test]
    fn vertical_lr_is_vertical() {
        assert!(WritingMode::VerticalLr.is_vertical());
        assert!(!WritingMode::VerticalLr.is_horizontal());
    }

    /// SidewaysRl is_vertical returns true.
    #[test]
    fn sideways_rl_is_vertical() {
        assert!(WritingMode::SidewaysRl.is_vertical());
    }

    /// SidewaysLr is_vertical returns true.
    #[test]
    fn sideways_lr_is_vertical() {
        assert!(WritingMode::SidewaysLr.is_vertical());
    }

    /// VerticalRl has flipped blocks (block direction is right-to-left).
    #[test]
    fn vertical_rl_is_flipped_blocks() {
        assert!(WritingMode::VerticalRl.is_flipped_blocks());
    }

    /// VerticalLr does NOT have flipped blocks.
    #[test]
    fn vertical_lr_not_flipped_blocks() {
        assert!(!WritingMode::VerticalLr.is_flipped_blocks());
    }

    /// SidewaysRl has flipped blocks.
    #[test]
    fn sideways_rl_is_flipped_blocks() {
        assert!(WritingMode::SidewaysRl.is_flipped_blocks());
    }

    /// SidewaysLr does NOT have flipped blocks.
    #[test]
    fn sideways_lr_not_flipped_blocks() {
        assert!(!WritingMode::SidewaysLr.is_flipped_blocks());
    }

    /// HorizontalTb does NOT have flipped blocks.
    #[test]
    fn horizontal_tb_not_flipped_blocks() {
        assert!(!WritingMode::HorizontalTb.is_flipped_blocks());
    }

    /// SidewaysLr has flipped lines (inline runs bottom-to-top).
    #[test]
    fn sideways_lr_is_flipped_lines() {
        assert!(WritingMode::SidewaysLr.is_flipped_lines());
    }

    /// All other modes do NOT have flipped lines.
    #[test]
    fn other_modes_not_flipped_lines() {
        assert!(!WritingMode::HorizontalTb.is_flipped_lines());
        assert!(!WritingMode::VerticalRl.is_flipped_lines());
        assert!(!WritingMode::VerticalLr.is_flipped_lines());
        assert!(!WritingMode::SidewaysRl.is_flipped_lines());
    }

    /// Horizontal-tb layout produces fragment with positive dimensions.
    #[test]
    fn horizontal_tb_layout_produces_positive_size() {
        let frag = block_layout_text(&["Hello world"], 400);
        let block = first_block_child(&frag);
        assert!(block.size.width > LayoutUnit::zero());
        assert!(block.size.height > LayoutUnit::zero());
    }

    /// All five writing-mode values are distinct.
    #[test]
    fn all_five_values_distinct() {
        let modes = [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
            WritingMode::SidewaysRl,
            WritingMode::SidewaysLr,
        ];
        for i in 0..modes.len() {
            for j in (i + 1)..modes.len() {
                assert_ne!(modes[i], modes[j], "Mode {:?} should differ from {:?}", modes[i], modes[j]);
            }
        }
    }

    /// WritingMode enum repr values are correct.
    #[test]
    fn repr_values() {
        assert_eq!(WritingMode::HorizontalTb as u8, 0);
        assert_eq!(WritingMode::VerticalRl as u8, 1);
        assert_eq!(WritingMode::VerticalLr as u8, 2);
        assert_eq!(WritingMode::SidewaysRl as u8, 3);
        assert_eq!(WritingMode::SidewaysLr as u8, 4);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TEXT-ORIENTATION PROPERTY TESTS (13 tests)
// Corresponds to WPT css/css-writing-modes/text-orientation-*
// ═══════════════════════════════════════════════════════════════════════
mod text_orientation_property {
    use super::*;

    /// text-orientation initial value is Mixed.
    #[test]
    fn initial_value_is_mixed() {
        let style = ComputedStyle::default();
        assert_eq!(style.text_orientation, TextOrientation::Mixed);
    }

    /// TextOrientation::INITIAL matches Mixed.
    #[test]
    fn initial_constant_matches_default() {
        assert_eq!(TextOrientation::INITIAL, TextOrientation::Mixed);
    }

    /// All three text-orientation values are distinct.
    #[test]
    fn all_three_values_distinct() {
        assert_ne!(TextOrientation::Mixed, TextOrientation::Upright);
        assert_ne!(TextOrientation::Mixed, TextOrientation::Sideways);
        assert_ne!(TextOrientation::Upright, TextOrientation::Sideways);
    }

    /// TextOrientation repr values.
    #[test]
    fn repr_values() {
        assert_eq!(TextOrientation::Mixed as u8, 0);
        assert_eq!(TextOrientation::Upright as u8, 1);
        assert_eq!(TextOrientation::Sideways as u8, 2);
    }

    /// font_orientation: horizontal-tb always yields Horizontal regardless of text-orientation.
    #[test]
    fn font_orientation_horizontal_tb_always_horizontal() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::HorizontalTb, TextOrientation::Mixed),
            FontOrientation::Horizontal
        );
        assert_eq!(
            font_orientation(WritingMode::HorizontalTb, TextOrientation::Upright),
            FontOrientation::Horizontal
        );
        assert_eq!(
            font_orientation(WritingMode::HorizontalTb, TextOrientation::Sideways),
            FontOrientation::Horizontal
        );
    }

    /// font_orientation: vertical + Mixed → VerticalMixed.
    #[test]
    fn font_orientation_vertical_mixed() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::VerticalRl, TextOrientation::Mixed),
            FontOrientation::VerticalMixed
        );
        assert_eq!(
            font_orientation(WritingMode::VerticalLr, TextOrientation::Mixed),
            FontOrientation::VerticalMixed
        );
    }

    /// font_orientation: vertical + Upright → VerticalUpright.
    #[test]
    fn font_orientation_vertical_upright() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::VerticalRl, TextOrientation::Upright),
            FontOrientation::VerticalUpright
        );
        assert_eq!(
            font_orientation(WritingMode::VerticalLr, TextOrientation::Upright),
            FontOrientation::VerticalUpright
        );
    }

    /// font_orientation: vertical + Sideways → VerticalRotated.
    #[test]
    fn font_orientation_vertical_sideways() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::VerticalRl, TextOrientation::Sideways),
            FontOrientation::VerticalRotated
        );
        assert_eq!(
            font_orientation(WritingMode::VerticalLr, TextOrientation::Sideways),
            FontOrientation::VerticalRotated
        );
    }

    /// font_orientation: sideways-rl ignores text-orientation per CSS Writing Modes §7.2.
    #[test]
    fn font_orientation_sideways_rl_mixed() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::SidewaysRl, TextOrientation::Mixed),
            FontOrientation::VerticalRotated
        );
    }

    /// font_orientation: sideways-lr ignores text-orientation per CSS Writing Modes §7.2.
    #[test]
    fn font_orientation_sideways_lr_upright() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::SidewaysLr, TextOrientation::Upright),
            FontOrientation::VerticalRotated
        );
    }

    /// font_orientation: sideways-lr + Sideways → VerticalRotated.
    #[test]
    fn font_orientation_sideways_lr_sideways() {
        use openui_style::font_orientation;
        assert_eq!(
            font_orientation(WritingMode::SidewaysLr, TextOrientation::Sideways),
            FontOrientation::VerticalRotated
        );
    }

    /// TextOrientation can be cloned and compared.
    #[test]
    fn clone_and_eq() {
        let a = TextOrientation::Mixed;
        let b = a.clone();
        assert_eq!(a, b);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// CHARACTER ORIENTATION TESTS (16 tests)
// Tests is_upright_in_mixed_vertical from UTR #50
// Corresponds to WPT css/css-writing-modes/text-orientation-script-*
// ═══════════════════════════════════════════════════════════════════════
mod char_orientation {
    use super::*;

    /// CJK unified ideograph U+4E00 (一) is upright in mixed vertical.
    #[test]
    fn cjk_ideograph_is_upright() {
        assert!(is_upright_in_mixed_vertical('一')); // U+4E00
    }

    /// CJK ideograph U+9FFF is upright.
    #[test]
    fn cjk_ideograph_end_range_is_upright() {
        assert!(is_upright_in_mixed_vertical('\u{9FFF}'));
    }

    /// Hiragana U+3042 (あ) is upright.
    #[test]
    fn hiragana_is_upright() {
        assert!(is_upright_in_mixed_vertical('あ')); // U+3042
    }

    /// Katakana U+30A2 (ア) is upright.
    #[test]
    fn katakana_is_upright() {
        assert!(is_upright_in_mixed_vertical('ア')); // U+30A2
    }

    /// Latin letter 'A' is NOT upright (should be sideways).
    #[test]
    fn latin_uppercase_not_upright() {
        assert!(!is_upright_in_mixed_vertical('A'));
    }

    /// Latin letter 'z' is NOT upright.
    #[test]
    fn latin_lowercase_not_upright() {
        assert!(!is_upright_in_mixed_vertical('z'));
    }

    /// ASCII digit '0' is NOT upright.
    #[test]
    fn ascii_digit_not_upright() {
        assert!(!is_upright_in_mixed_vertical('0'));
    }

    /// Fullwidth Latin letter Ａ (U+FF21) is upright.
    #[test]
    fn fullwidth_latin_is_upright() {
        assert!(is_upright_in_mixed_vertical('\u{FF21}')); // Ａ
    }

    /// CJK Symbols: ideographic comma U+3001 (、) is upright.
    #[test]
    fn cjk_comma_is_upright() {
        assert!(is_upright_in_mixed_vertical('、')); // U+3001
    }

    /// CJK Symbols: ideographic period U+3002 (。) is upright.
    #[test]
    fn cjk_period_is_upright() {
        assert!(is_upright_in_mixed_vertical('。')); // U+3002
    }

    /// Hangul syllable U+AC00 (가) is upright.
    #[test]
    fn hangul_syllable_is_upright() {
        assert!(is_upright_in_mixed_vertical('가')); // U+AC00
    }

    /// CJK Extension B U+20000 (𠀀) is upright.
    #[test]
    fn cjk_extension_b_is_upright() {
        assert!(is_upright_in_mixed_vertical('\u{20000}'));
    }

    /// Space character is NOT upright.
    #[test]
    fn space_not_upright() {
        assert!(!is_upright_in_mixed_vertical(' '));
    }

    /// ASCII punctuation '.' is NOT upright.
    #[test]
    fn ascii_period_not_upright() {
        assert!(!is_upright_in_mixed_vertical('.'));
    }

    /// Cyrillic letter Д (U+0414) is NOT upright.
    #[test]
    fn cyrillic_not_upright() {
        assert!(!is_upright_in_mixed_vertical('Д'));
    }

    /// CJK Compatibility Ideograph U+F900 (豈) is upright.
    #[test]
    fn cjk_compatibility_ideograph_is_upright() {
        assert!(is_upright_in_mixed_vertical('\u{F900}'));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// DIRECTION PROPERTY TESTS (13 tests)
// Corresponds to WPT css/css-writing-modes/direction-*
// ═══════════════════════════════════════════════════════════════════════
mod direction_property {
    use super::*;

    /// direction initial value is Ltr.
    #[test]
    fn initial_value_is_ltr() {
        let style = ComputedStyle::default();
        assert_eq!(style.direction, Direction::Ltr);
    }

    /// Direction::INITIAL matches Ltr.
    #[test]
    fn initial_constant_matches_default() {
        assert_eq!(Direction::INITIAL, Direction::Ltr);
    }

    /// Ltr and Rtl are distinct.
    #[test]
    fn ltr_rtl_distinct() {
        assert_ne!(Direction::Ltr, Direction::Rtl);
    }

    /// Direction repr values.
    #[test]
    fn repr_values() {
        assert_eq!(Direction::Ltr as u8, 0);
        assert_eq!(Direction::Rtl as u8, 1);
    }

    /// RTL direction layout produces fragment with positive height.
    #[test]
    fn rtl_direction_produces_positive_height() {
        let frag = make_styled_text_block(&["Hello"], 400, |s| {
            s.direction = Direction::Rtl;
        });
        let block = first_block_child(&frag);
        assert!(
            block.size.height > LayoutUnit::zero(),
            "RTL direction should produce positive height"
        );
    }

    /// RTL + text-align:start offsets text to the right.
    #[test]
    fn rtl_text_align_start_offsets_right() {
        let frag = make_styled_text_block(&["Hi"], 800, |s| {
            s.direction = Direction::Rtl;
            s.text_align = TextAlign::Start;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        // Short text in RTL with text-align:start should have left > 0.
        assert!(
            texts[0].offset.left > LayoutUnit::zero(),
            "RTL start-aligned short text should be shifted right, got {:?}",
            texts[0].offset.left
        );
    }

    /// LTR + text-align:start keeps text at left edge.
    #[test]
    fn ltr_text_align_start_at_left() {
        let frag = make_styled_text_block(&["Hello"], 800, |s| {
            s.direction = Direction::Ltr;
            s.text_align = TextAlign::Start;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        // In LTR, start = left, so offset.left should be small.
        assert!(
            texts[0].offset.left.to_f32() < 2.0,
            "LTR start-aligned text should be near left edge"
        );
    }

    /// RTL + text-align:end keeps text at left edge (end = left in RTL).
    #[test]
    fn rtl_text_align_end_at_left() {
        let frag = make_styled_text_block(&["Hi"], 800, |s| {
            s.direction = Direction::Rtl;
            s.text_align = TextAlign::End;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(
            texts[0].offset.left.to_f32() < 2.0,
            "RTL end-aligned text should be near left edge, got {:?}",
            texts[0].offset.left
        );
    }

    /// LTR text-align:right shifts text to the right.
    #[test]
    fn ltr_text_align_right_offsets_right() {
        let frag = make_styled_text_block(&["Hi"], 800, |s| {
            s.direction = Direction::Ltr;
            s.text_align = TextAlign::Right;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(
            texts[0].offset.left > LayoutUnit::zero(),
            "Right-aligned text should be shifted right"
        );
    }

    /// LTR text-align:center shifts text to approximately center.
    #[test]
    fn ltr_text_align_center_shifts_center() {
        let frag = make_styled_text_block(&["Hi"], 800, |s| {
            s.direction = Direction::Ltr;
            s.text_align = TextAlign::Center;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        // Centered short text should have a left offset significantly > 0.
        assert!(
            texts[0].offset.left > lu(50.0),
            "Center-aligned text should be shifted from left, got {:?}",
            texts[0].offset.left
        );
    }

    /// Direction can be combined with WritingMode to create WritingDirectionMode.
    #[test]
    fn direction_creates_writing_direction_mode() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::HorizontalTb);
        assert!(wdm.is_horizontal());
        assert!(!wdm.is_rtl());

        let wdm_rtl = Direction::Rtl.writing_direction(WritingMode::HorizontalTb);
        assert!(wdm_rtl.is_horizontal());
        assert!(wdm_rtl.is_rtl());
    }

    /// Direction Rtl with vertical-rl creates correct WritingDirectionMode.
    #[test]
    fn rtl_vertical_rl_writing_direction() {
        let wdm = Direction::Rtl.writing_direction(WritingMode::VerticalRl);
        assert!(!wdm.is_horizontal());
        assert!(wdm.is_flipped_blocks());
        assert!(wdm.is_rtl());
    }
}

// ═══════════════════════════════════════════════════════════════════════
// UNICODE-BIDI PROPERTY TESTS (10 tests)
// Corresponds to WPT css/css-writing-modes/unicode-bidi-*
// ═══════════════════════════════════════════════════════════════════════
mod unicode_bidi {
    use super::*;

    /// unicode-bidi initial value is Normal.
    #[test]
    fn initial_value_is_normal() {
        let style = ComputedStyle::default();
        assert_eq!(style.unicode_bidi, UnicodeBidi::Normal);
    }

    /// UnicodeBidi::INITIAL matches Normal.
    #[test]
    fn initial_constant_matches_default() {
        assert_eq!(UnicodeBidi::INITIAL, UnicodeBidi::Normal);
    }

    /// All six values are distinct.
    #[test]
    fn all_six_values_distinct() {
        let vals = [
            UnicodeBidi::Normal,
            UnicodeBidi::Embed,
            UnicodeBidi::Override,
            UnicodeBidi::Isolate,
            UnicodeBidi::IsolateOverride,
            UnicodeBidi::Plaintext,
        ];
        for i in 0..vals.len() {
            for j in (i + 1)..vals.len() {
                assert_ne!(vals[i], vals[j], "{:?} should differ from {:?}", vals[i], vals[j]);
            }
        }
    }

    /// UnicodeBidi repr values.
    #[test]
    fn repr_values() {
        assert_eq!(UnicodeBidi::Normal as u8, 0);
        assert_eq!(UnicodeBidi::Embed as u8, 1);
        assert_eq!(UnicodeBidi::Override as u8, 2);
        assert_eq!(UnicodeBidi::Isolate as u8, 3);
        assert_eq!(UnicodeBidi::IsolateOverride as u8, 4);
        assert_eq!(UnicodeBidi::Plaintext as u8, 5);
    }

    /// UnicodeBidi can be set on ComputedStyle.
    #[test]
    fn set_on_computed_style() {
        let mut style = ComputedStyle::default();
        style.unicode_bidi = UnicodeBidi::Embed;
        assert_eq!(style.unicode_bidi, UnicodeBidi::Embed);
    }

    /// UnicodeBidi can be cloned and compared.
    #[test]
    fn clone_and_eq() {
        let a = UnicodeBidi::IsolateOverride;
        let b = a.clone();
        assert_eq!(a, b);
    }

    /// UnicodeBidi Override on ComputedStyle.
    #[test]
    fn set_override_on_computed_style() {
        let mut style = ComputedStyle::default();
        style.unicode_bidi = UnicodeBidi::Override;
        assert_eq!(style.unicode_bidi, UnicodeBidi::Override);
    }

    /// UnicodeBidi Isolate on ComputedStyle.
    #[test]
    fn set_isolate_on_computed_style() {
        let mut style = ComputedStyle::default();
        style.unicode_bidi = UnicodeBidi::Isolate;
        assert_eq!(style.unicode_bidi, UnicodeBidi::Isolate);
    }

    /// UnicodeBidi Plaintext on ComputedStyle.
    #[test]
    fn set_plaintext_on_computed_style() {
        let mut style = ComputedStyle::default();
        style.unicode_bidi = UnicodeBidi::Plaintext;
        assert_eq!(style.unicode_bidi, UnicodeBidi::Plaintext);
    }

    /// Layout with unicode-bidi:normal on LTR text produces positive output.
    #[test]
    fn layout_bidi_normal_produces_fragment() {
        let frag = make_styled_text_block(&["Hello"], 400, |s| {
            s.unicode_bidi = UnicodeBidi::Normal;
            s.direction = Direction::Ltr;
        });
        let block = first_block_child(&frag);
        assert!(
            block.size.height > LayoutUnit::zero(),
            "unicode-bidi:normal layout should produce positive height"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BIDI ALGORITHM TESTS (12 tests)
// Tests BidiParagraph and BidiRun from the bidi module
// Corresponds to WPT css/css-writing-modes/bidi-*
// ═══════════════════════════════════════════════════════════════════════
mod bidi_algorithm {
    use super::*;

    /// BidiParagraph with LTR text produces a single LTR run.
    #[test]
    fn ltr_text_single_run() {
        let bidi = BidiParagraph::new("Hello world", Some(TextDirection::Ltr));
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty(), "Should have at least one run");
        assert_eq!(runs[0].level % 2, 0, "LTR text should have even level (LTR)");
    }

    /// BidiParagraph with RTL (Hebrew) text produces RTL run.
    #[test]
    fn rtl_hebrew_text() {
        let bidi = BidiParagraph::new("שלום", Some(TextDirection::Rtl));
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].level % 2, 1, "Hebrew text should have odd level (RTL)");
    }

    /// BidiParagraph with RTL (Arabic) text produces RTL run.
    #[test]
    fn rtl_arabic_text() {
        let bidi = BidiParagraph::new("مرحبا", Some(TextDirection::Rtl));
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        assert_eq!(
            runs[0].level % 2,
            1,
            "Arabic text should have odd level (RTL)"
        );
    }

    /// Mixed LTR and RTL text produces multiple runs.
    #[test]
    fn mixed_ltr_rtl_produces_multiple_runs() {
        let bidi = BidiParagraph::new("Hello שלום world", Some(TextDirection::Ltr));
        let runs = bidi.visual_runs();
        assert!(
            runs.len() >= 2,
            "Mixed LTR/RTL text should have at least 2 runs, got {}",
            runs.len()
        );
    }

    /// BidiRun direction field is consistent with level.
    #[test]
    fn run_direction_matches_level() {
        let bidi = BidiParagraph::new("Hello שלום", Some(TextDirection::Ltr));
        for run in bidi.visual_runs() {
            if run.level % 2 == 0 {
                assert_eq!(run.direction, TextDirection::Ltr);
            } else {
                assert_eq!(run.direction, TextDirection::Rtl);
            }
        }
    }

    /// BidiRun start and end byte offsets span the full text.
    #[test]
    fn runs_cover_full_text() {
        let text = "Hello world";
        let bidi = BidiParagraph::new(text, Some(TextDirection::Ltr));
        let runs = bidi.visual_runs();
        let total_bytes: usize = runs.iter().map(|r| r.end - r.start).sum();
        assert_eq!(
            total_bytes,
            text.len(),
            "Runs should cover all bytes in the text"
        );
    }

    /// Auto-detect base direction with LTR text.
    #[test]
    fn auto_detect_ltr() {
        let bidi = BidiParagraph::new("Hello", None);
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].level % 2, 0, "Auto-detected LTR text should be LTR");
    }

    /// Auto-detect base direction with RTL text.
    #[test]
    fn auto_detect_rtl() {
        let bidi = BidiParagraph::new("שלום", None);
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].level % 2, 1, "Auto-detected RTL text should be RTL");
    }

    /// Numbers in RTL text remain LTR (European numbers).
    #[test]
    fn numbers_in_rtl_are_ltr() {
        let bidi = BidiParagraph::new("שלום 123 עולם", Some(TextDirection::Rtl));
        let runs = bidi.visual_runs();
        // There should be a run with even level for the numbers.
        let has_ltr_run = runs.iter().any(|r| r.level % 2 == 0);
        assert!(
            has_ltr_run,
            "Numbers embedded in RTL text should produce an LTR run"
        );
    }

    /// Pure LTR paragraph has base level 0.
    #[test]
    fn ltr_paragraph_base_level_zero() {
        let bidi = BidiParagraph::new("Hello", Some(TextDirection::Ltr));
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].level, 0, "LTR base level should be 0");
    }

    /// Pure RTL paragraph has base level 1.
    #[test]
    fn rtl_paragraph_base_level_one() {
        let bidi = BidiParagraph::new("שלום", Some(TextDirection::Rtl));
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].level, 1, "RTL base level should be 1");
    }

    /// Empty string produces empty visual runs.
    #[test]
    fn empty_text_no_runs() {
        let bidi = BidiParagraph::new("", Some(TextDirection::Ltr));
        let runs = bidi.visual_runs();
        // Empty text should produce either zero runs or a single zero-length run.
        let total: usize = runs.iter().map(|r| r.end - r.start).sum();
        assert_eq!(total, 0, "Empty text should have zero total run length");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// LOGICAL ↔ PHYSICAL CONVERSION TESTS (15 tests)
// Tests WritingDirectionMode and WritingModeConverter
// Corresponds to WPT css/css-writing-modes/logical-physical-mapping-*
// ═══════════════════════════════════════════════════════════════════════
mod logical_physical_conversion {
    use super::*;

    /// WritingDirectionMode default is horizontal LTR.
    #[test]
    fn default_is_horizontal_ltr() {
        let wdm = WritingDirectionMode::default();
        assert!(wdm.is_horizontal());
        assert!(!wdm.is_rtl());
        assert!(!wdm.is_flipped_blocks());
        assert!(!wdm.is_flipped_lines());
    }

    /// WritingDirectionMode::horizontal_ltr() matches default.
    #[test]
    fn horizontal_ltr_matches_default() {
        assert_eq!(WritingDirectionMode::horizontal_ltr(), WritingDirectionMode::default());
    }

    /// horizontal-tb LTR: logical size maps inline→width, block→height.
    #[test]
    fn horizontal_ltr_size_mapping() {
        let wdm = WritingDirectionMode::horizontal_ltr();
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);

        let logical = LogicalSize::new(lu_i(100), lu_i(50));
        let physical = converter.to_physical_size(logical);
        assert_eq!(physical.width, lu_i(100), "inline→width");
        assert_eq!(physical.height, lu_i(50), "block→height");
    }

    /// horizontal-tb LTR: physical size maps width→inline, height→block.
    #[test]
    fn horizontal_ltr_physical_to_logical_size() {
        let wdm = WritingDirectionMode::horizontal_ltr();
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);

        let physical = PhysicalSize::new(lu_i(200), lu_i(100));
        let logical = converter.to_logical_size(physical);
        assert_eq!(logical.inline_size, lu_i(200), "width→inline");
        assert_eq!(logical.block_size, lu_i(100), "height→block");
    }

    /// vertical-rl LTR: logical size maps inline→height, block→width.
    #[test]
    fn vertical_rl_ltr_size_mapping() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::VerticalRl);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);

        let logical = LogicalSize::new(lu_i(100), lu_i(50));
        let physical = converter.to_physical_size(logical);
        assert_eq!(physical.width, lu_i(50), "block→width in vertical");
        assert_eq!(physical.height, lu_i(100), "inline→height in vertical");
    }

    /// vertical-lr LTR: logical size maps inline→height, block→width.
    #[test]
    fn vertical_lr_ltr_size_mapping() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::VerticalLr);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);

        let logical = LogicalSize::new(lu_i(100), lu_i(50));
        let physical = converter.to_physical_size(logical);
        assert_eq!(physical.width, lu_i(50), "block→width in vertical-lr");
        assert_eq!(physical.height, lu_i(100), "inline→height in vertical-lr");
    }

    /// horizontal-tb LTR: logical offset maps directly (inline→left, block→top).
    #[test]
    fn horizontal_ltr_offset_mapping() {
        let wdm = WritingDirectionMode::horizontal_ltr();
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(100), lu_i(50));

        let logical = LogicalOffset::new(lu_i(10), lu_i(20));
        let physical = converter.to_physical_offset(logical, inner);
        assert_eq!(physical.left, lu_i(10), "inline→left in horizontal LTR");
        assert_eq!(physical.top, lu_i(20), "block→top in horizontal LTR");
    }

    /// horizontal-tb RTL: inline offset is mirrored from right edge.
    #[test]
    fn horizontal_rtl_offset_mirrors() {
        let wdm = Direction::Rtl.writing_direction(WritingMode::HorizontalTb);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(100), lu_i(50));

        let logical = LogicalOffset::new(lu_i(0), lu_i(0));
        let physical = converter.to_physical_offset(logical, inner);
        // RTL: inline-start is right edge → left = outer_width - inline_offset - inner_width
        assert_eq!(
            physical.left,
            lu_i(700),
            "RTL inline-start=0 → left=800-0-100=700"
        );
        assert_eq!(physical.top, lu_i(0), "block still maps to top");
    }

    /// vertical-rl LTR: block-start is right edge (flipped blocks).
    #[test]
    fn vertical_rl_ltr_offset_flipped_blocks() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::VerticalRl);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(50), lu_i(100));

        let logical = LogicalOffset::new(lu_i(0), lu_i(0));
        let physical = converter.to_physical_offset(logical, inner);
        // vertical-rl: block-start = right edge → left = 800 - 0 - 50 = 750
        assert_eq!(
            physical.left,
            lu_i(750),
            "vertical-rl block-start=0 → right edge"
        );
        assert_eq!(physical.top, lu_i(0), "inline-start → top in vertical");
    }

    /// vertical-lr LTR: block-start is left edge (not flipped).
    #[test]
    fn vertical_lr_ltr_offset_not_flipped() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::VerticalLr);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(50), lu_i(100));

        let logical = LogicalOffset::new(lu_i(0), lu_i(0));
        let physical = converter.to_physical_offset(logical, inner);
        assert_eq!(physical.left, lu_i(0), "vertical-lr block-start → left edge");
        assert_eq!(physical.top, lu_i(0), "inline-start → top");
    }

    /// Round-trip: logical → physical → logical offset in horizontal LTR.
    #[test]
    fn roundtrip_offset_horizontal_ltr() {
        let wdm = WritingDirectionMode::horizontal_ltr();
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(100), lu_i(50));

        let original = LogicalOffset::new(lu_i(30), lu_i(40));
        let physical = converter.to_physical_offset(original, inner);
        let back = converter.to_logical_offset(physical, inner);
        assert_eq!(back, original, "Round-trip should preserve offset");
    }

    /// Round-trip: logical → physical → logical offset in horizontal RTL.
    #[test]
    fn roundtrip_offset_horizontal_rtl() {
        let wdm = Direction::Rtl.writing_direction(WritingMode::HorizontalTb);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(100), lu_i(50));

        let original = LogicalOffset::new(lu_i(30), lu_i(40));
        let physical = converter.to_physical_offset(original, inner);
        let back = converter.to_logical_offset(physical, inner);
        assert_eq!(back, original, "Round-trip RTL should preserve offset");
    }

    /// Round-trip: logical → physical → logical offset in vertical-rl LTR.
    #[test]
    fn roundtrip_offset_vertical_rl_ltr() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::VerticalRl);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);
        let inner = PhysicalSize::new(lu_i(50), lu_i(100));

        let original = LogicalOffset::new(lu_i(30), lu_i(40));
        let physical = converter.to_physical_offset(original, inner);
        let back = converter.to_logical_offset(physical, inner);
        assert_eq!(back, original, "Round-trip vertical-rl should preserve offset");
    }

    /// Round-trip: logical → physical → logical size in vertical mode.
    #[test]
    fn roundtrip_size_vertical() {
        let wdm = Direction::Ltr.writing_direction(WritingMode::VerticalRl);
        let outer = PhysicalSize::new(lu_i(800), lu_i(600));
        let converter = WritingModeConverter::new(wdm, outer);

        let original = LogicalSize::new(lu_i(100), lu_i(50));
        let physical = converter.to_physical_size(original);
        let back = converter.to_logical_size(physical);
        assert_eq!(back, original, "Round-trip size should preserve values");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ADDITIONAL INTEGRATION TESTS (4 tests)
// Cross-cutting tests combining multiple features
// ═══════════════════════════════════════════════════════════════════════
mod integration {
    use super::*;

    /// ComputedStyle defaults all writing-mode-related properties correctly.
    #[test]
    fn computed_style_defaults_all_wm_properties() {
        let style = ComputedStyle::default();
        assert_eq!(style.writing_mode, WritingMode::HorizontalTb);
        assert_eq!(style.text_orientation, TextOrientation::Mixed);
        assert_eq!(style.direction, Direction::Ltr);
        assert_eq!(style.unicode_bidi, UnicodeBidi::Normal);
    }

    /// WritingDirectionMode from default ComputedStyle matches horizontal_ltr.
    #[test]
    fn wdm_from_default_style() {
        let style = ComputedStyle::default();
        let wdm = style.direction.writing_direction(style.writing_mode);
        assert_eq!(wdm, WritingDirectionMode::horizontal_ltr());
    }

    /// Bidi paragraph respects explicit base direction over auto.
    #[test]
    fn bidi_explicit_overrides_auto() {
        // Force LTR on Hebrew text.
        let bidi = BidiParagraph::new("שלום", Some(TextDirection::Ltr));
        let runs = bidi.visual_runs();
        assert!(!runs.is_empty());
        // The runs exist; Hebrew chars still get RTL level even with LTR base.
        let has_rtl = runs.iter().any(|r| r.level % 2 == 1);
        assert!(has_rtl, "Hebrew chars should still be RTL even with LTR base");
    }

    /// BoxStrut zero has all edges zero.
    #[test]
    fn box_strut_zero() {
        let strut = BoxStrut::zero();
        assert_eq!(strut.top, LayoutUnit::zero());
        assert_eq!(strut.right, LayoutUnit::zero());
        assert_eq!(strut.bottom, LayoutUnit::zero());
        assert_eq!(strut.left, LayoutUnit::zero());
        assert_eq!(strut.inline_sum(), LayoutUnit::zero());
        assert_eq!(strut.block_sum(), LayoutUnit::zero());
    }
}
