//! Comprehensive tests for the openui-text font system.
//!
//! Tests cover: font resolution, metrics, caching, fallback,
//! description defaults, font measurement, and ComputedStyle integration.

use std::sync::Arc;

use openui_style::{
    ComputedStyle, FontFamily, FontFamilyList, FontOpticalSizing, FontSmoothing, FontStretch,
    FontStyleEnum, FontSynthesis, FontVariantCaps, FontWeight, GenericFontFamily, TextRendering,
    LineHeight, TextAlign, TextAlignLast, TextDecorationLine,
    TextDecorationStyle, TextDecorationThickness, TextJustify, TextOverflow, TextTransform,
    TextUnderlinePosition, UnicodeBidi, VerticalAlign, WordBreak, WritingMode,
    OverflowWrap, Hyphens, TextOrientation, TabSize, StyleColor,
};

use openui_text::font::{Font, FontCache, FontDescription, FontFallbackList, FontMetrics};

// ═══════════════════════════════════════════════════════════════════════
// Font Description Defaults
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn font_description_default_family() {
    let desc = FontDescription::default();
    assert_eq!(desc.family.families.len(), 1);
    assert_eq!(
        desc.family.families[0],
        FontFamily::Generic(GenericFontFamily::SansSerif)
    );
}

#[test]
fn font_description_default_size() {
    let desc = FontDescription::default();
    assert_eq!(desc.size, 16.0);
    assert_eq!(desc.specified_size, 16.0);
}

#[test]
fn font_description_default_weight() {
    let desc = FontDescription::default();
    assert_eq!(desc.weight, FontWeight::NORMAL);
    assert_eq!(desc.weight.0, 400.0);
}

#[test]
fn font_description_default_stretch() {
    let desc = FontDescription::default();
    assert_eq!(desc.stretch, FontStretch::NORMAL);
    assert_eq!(desc.stretch.0, 100.0);
}

#[test]
fn font_description_default_style() {
    let desc = FontDescription::default();
    assert_eq!(desc.style, FontStyleEnum::Normal);
}

#[test]
fn font_description_default_variant_caps() {
    let desc = FontDescription::default();
    assert_eq!(desc.variant_caps, FontVariantCaps::Normal);
}

#[test]
fn font_description_default_spacing() {
    let desc = FontDescription::default();
    assert_eq!(desc.letter_spacing, 0.0);
    assert_eq!(desc.word_spacing, 0.0);
}

#[test]
fn font_description_default_locale() {
    let desc = FontDescription::default();
    assert!(desc.locale.is_none());
}

#[test]
fn font_description_default_smoothing() {
    let desc = FontDescription::default();
    assert_eq!(desc.font_smoothing, FontSmoothing::Auto);
}

#[test]
fn font_description_default_text_rendering() {
    let desc = FontDescription::default();
    assert_eq!(desc.text_rendering, TextRendering::Auto);
}

#[test]
fn font_description_default_feature_settings() {
    let desc = FontDescription::default();
    assert!(desc.feature_settings.is_empty());
}

#[test]
fn font_description_default_variation_settings() {
    let desc = FontDescription::default();
    assert!(desc.variation_settings.is_empty());
}

#[test]
fn font_description_default_synthesis() {
    let desc = FontDescription::default();
    assert_eq!(desc.font_synthesis_weight, FontSynthesis::Auto);
    assert_eq!(desc.font_synthesis_style, FontSynthesis::Auto);
}

#[test]
fn font_description_default_optical_sizing() {
    let desc = FontDescription::default();
    assert_eq!(desc.font_optical_sizing, FontOpticalSizing::Auto);
}

#[test]
fn font_description_with_family_and_size() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::single("Arial"),
        24.0,
    );
    assert_eq!(desc.size, 24.0);
    assert_eq!(desc.specified_size, 24.0);
    assert_eq!(desc.family.families[0], FontFamily::Named("Arial".into()));
}

// ═══════════════════════════════════════════════════════════════════════
// FontFamilyList
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn font_family_list_single_named() {
    let list = FontFamilyList::single("Helvetica");
    assert_eq!(list.len(), 1);
    assert!(!list.is_empty());
    assert_eq!(list.families[0], FontFamily::Named("Helvetica".into()));
}

#[test]
fn font_family_list_single_generic() {
    let list = FontFamilyList::generic(GenericFontFamily::Monospace);
    assert_eq!(list.len(), 1);
    assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Monospace));
}

#[test]
fn font_family_list_default_is_sans_serif() {
    let list = FontFamilyList::default();
    assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::SansSerif));
}

#[test]
fn font_family_list_multiple() {
    let list = FontFamilyList {
        families: vec![
            FontFamily::Named("Arial".into()),
            FontFamily::Named("Helvetica".into()),
            FontFamily::Generic(GenericFontFamily::SansSerif),
        ],
    };
    assert_eq!(list.len(), 3);
}

#[test]
fn font_family_list_empty() {
    let list = FontFamilyList {
        families: Vec::new(),
    };
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════
// FontWeight constants
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn font_weight_constants() {
    assert_eq!(FontWeight::THIN.0, 100.0);
    assert_eq!(FontWeight::LIGHT.0, 300.0);
    assert_eq!(FontWeight::NORMAL.0, 400.0);
    assert_eq!(FontWeight::MEDIUM.0, 500.0);
    assert_eq!(FontWeight::SEMI_BOLD.0, 600.0);
    assert_eq!(FontWeight::BOLD.0, 700.0);
    assert_eq!(FontWeight::EXTRA_BOLD.0, 800.0);
    assert_eq!(FontWeight::BLACK.0, 900.0);
}

// ═══════════════════════════════════════════════════════════════════════
// FontStretch constants
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn font_stretch_constants() {
    assert_eq!(FontStretch::ULTRA_CONDENSED.0, 50.0);
    assert_eq!(FontStretch::CONDENSED.0, 75.0);
    assert_eq!(FontStretch::NORMAL.0, 100.0);
    assert_eq!(FontStretch::EXPANDED.0, 125.0);
    assert_eq!(FontStretch::ULTRA_EXPANDED.0, 200.0);
}

// ═══════════════════════════════════════════════════════════════════════
// FontMetrics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn font_metrics_zero() {
    let m = FontMetrics::zero();
    assert_eq!(m.ascent, 0.0);
    assert_eq!(m.descent, 0.0);
    assert_eq!(m.line_gap, 0.0);
    assert_eq!(m.line_spacing, 0.0);
    assert_eq!(m.x_height, 0.0);
    assert_eq!(m.cap_height, 0.0);
    assert_eq!(m.zero_width, 0.0);
    assert_eq!(m.underline_offset, 0.0);
    assert_eq!(m.underline_thickness, 0.0);
    assert_eq!(m.strikeout_position, 0.0);
    assert_eq!(m.strikeout_thickness, 0.0);
    assert_eq!(m.overline_offset, 0.0);
    assert_eq!(m.units_per_em, 0);
}

#[test]
fn font_metrics_default_is_zero() {
    let m = FontMetrics::default();
    assert_eq!(m, FontMetrics::zero());
}

// ═══════════════════════════════════════════════════════════════════════
// Font Resolution — generic families
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn resolve_sans_serif() {
    let desc = FontDescription::default(); // sans-serif at 16px
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "sans-serif should resolve");
}

#[test]
fn resolve_serif() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Serif),
        16.0,
    );
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "serif should resolve");
}

#[test]
fn resolve_monospace() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Monospace),
        16.0,
    );
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "monospace should resolve");
}

// ═══════════════════════════════════════════════════════════════════════
// Font Metrics — resolved fonts
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn resolved_font_ascent_positive() {
    let font = Font::new(FontDescription::default());
    let metrics = font.font_metrics().expect("should have metrics");
    assert!(metrics.ascent > 0.0, "ascent should be positive, got {}", metrics.ascent);
}

#[test]
fn resolved_font_descent_positive() {
    let font = Font::new(FontDescription::default());
    let metrics = font.font_metrics().expect("should have metrics");
    assert!(metrics.descent > 0.0, "descent should be positive, got {}", metrics.descent);
}

#[test]
fn resolved_font_line_spacing_positive() {
    let font = Font::new(FontDescription::default());
    let metrics = font.font_metrics().expect("should have metrics");
    assert!(
        metrics.line_spacing > 0.0,
        "line_spacing should be positive, got {}",
        metrics.line_spacing
    );
}

#[test]
fn resolved_font_line_spacing_equals_sum() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    let expected = m.ascent + m.descent + m.line_gap;
    assert!(
        (m.line_spacing - expected).abs() < 0.001,
        "line_spacing ({}) should equal ascent ({}) + descent ({}) + line_gap ({})",
        m.line_spacing, m.ascent, m.descent, m.line_gap
    );
}

#[test]
fn resolved_font_x_height_positive() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    assert!(m.x_height > 0.0, "x_height should be positive, got {}", m.x_height);
}

#[test]
fn resolved_font_cap_height_positive() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    assert!(m.cap_height > 0.0, "cap_height should be positive, got {}", m.cap_height);
}

#[test]
fn resolved_font_zero_width_positive() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    assert!(m.zero_width > 0.0, "zero_width (ch unit ref) should be positive, got {}", m.zero_width);
}

#[test]
fn resolved_font_cap_height_ge_x_height() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    assert!(
        m.cap_height >= m.x_height,
        "cap_height ({}) should be >= x_height ({})",
        m.cap_height, m.x_height
    );
}

#[test]
fn resolved_font_underline_thickness_positive() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    assert!(
        m.underline_thickness > 0.0,
        "underline_thickness should be positive, got {}",
        m.underline_thickness
    );
}

#[test]
fn resolved_font_units_per_em_positive() {
    let font = Font::new(FontDescription::default());
    let m = font.font_metrics().expect("should have metrics");
    assert!(m.units_per_em > 0, "units_per_em should be positive, got {}", m.units_per_em);
}

// ═══════════════════════════════════════════════════════════════════════
// Font Weight Matching
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn weight_400_resolves() {
    let mut desc = FontDescription::default();
    desc.weight = FontWeight::NORMAL;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
}

#[test]
fn weight_700_resolves() {
    let mut desc = FontDescription::default();
    desc.weight = FontWeight::BOLD;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
}

#[test]
fn weight_100_resolves() {
    let mut desc = FontDescription::default();
    desc.weight = FontWeight::THIN;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
}

#[test]
fn weight_900_resolves() {
    let mut desc = FontDescription::default();
    desc.weight = FontWeight::BLACK;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// Font Style Matching
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn style_italic_resolves() {
    let mut desc = FontDescription::default();
    desc.style = FontStyleEnum::Italic;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
}

#[test]
fn style_oblique_resolves() {
    let mut desc = FontDescription::default();
    desc.style = FontStyleEnum::Oblique(14.0);
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// Font Cache
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn cache_returns_same_arc_for_same_description() {
    let desc = FontDescription::default();
    let font1 = Font::new(desc.clone());
    let font2 = Font::new(desc);
    let p1 = font1.primary_font().expect("should resolve");
    let p2 = font2.primary_font().expect("should resolve");
    assert!(Arc::ptr_eq(p1, p2), "same description should return same Arc");
}

#[test]
fn cache_returns_different_arc_for_different_size() {
    let desc1 = FontDescription::with_family_and_size(FontFamilyList::default(), 12.0);
    let desc2 = FontDescription::with_family_and_size(FontFamilyList::default(), 24.0);
    let font1 = Font::new(desc1);
    let font2 = Font::new(desc2);
    let p1 = font1.primary_font().expect("should resolve");
    let p2 = font2.primary_font().expect("should resolve");
    assert!(!Arc::ptr_eq(p1, p2), "different sizes should produce different entries");
}

#[test]
fn cache_generic_family_name_mapping() {
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Serif), "serif");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::SansSerif), "sans-serif");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Monospace), "monospace");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Cursive), "cursive");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Fantasy), "fantasy");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::SystemUi), "system-ui");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::None), "sans-serif");
}

// ═══════════════════════════════════════════════════════════════════════
// Font Fallback
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fallback_nonexistent_family_falls_back() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::single("NonExistentFontXYZ123"),
        16.0,
    );
    let font = Font::new(desc);
    // Should still resolve via fallback to system default
    assert!(
        font.primary_font().is_some(),
        "should fall back to system default when family doesn't exist"
    );
}

#[test]
fn fallback_multiple_families_first_match_wins() {
    let list = FontFamilyList {
        families: vec![
            FontFamily::Generic(GenericFontFamily::SansSerif),
            FontFamily::Generic(GenericFontFamily::Serif),
        ],
    };
    let desc = FontDescription::with_family_and_size(list, 16.0);
    let font = Font::new(desc);
    assert!(font.fallback_count() >= 1, "should resolve at least one font");
}

#[test]
fn fallback_list_has_correct_length() {
    let list = FontFamilyList {
        families: vec![
            FontFamily::Generic(GenericFontFamily::SansSerif),
            FontFamily::Generic(GenericFontFamily::Serif),
            FontFamily::Generic(GenericFontFamily::Monospace),
        ],
    };
    let desc = FontDescription::with_family_and_size(list, 16.0);
    let fallback = FontFallbackList::new(&desc);
    // At least the generics should resolve to something
    assert!(fallback.len() >= 1);
    assert!(!fallback.is_empty());
}

#[test]
fn fallback_list_get_out_of_bounds() {
    let desc = FontDescription::default();
    let fallback = FontFallbackList::new(&desc);
    assert!(fallback.get(999).is_none());
}

// ═══════════════════════════════════════════════════════════════════════
// Font Measurement
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn width_of_hello_positive() {
    let font = Font::new(FontDescription::default());
    let w = font.width("Hello");
    assert!(w > 0.0, "width of 'Hello' should be positive, got {}", w);
}

#[test]
fn width_of_empty_string_zero() {
    let font = Font::new(FontDescription::default());
    let w = font.width("");
    assert_eq!(w, 0.0, "width of empty string should be 0, got {}", w);
}

#[test]
fn width_longer_string_greater() {
    let font = Font::new(FontDescription::default());
    let w1 = font.width("Hi");
    let w2 = font.width("Hello World");
    assert!(
        w2 > w1,
        "'Hello World' ({}) should be wider than 'Hi' ({})",
        w2, w1
    );
}

#[test]
fn width_single_char() {
    let font = Font::new(FontDescription::default());
    let w = font.width("A");
    assert!(w > 0.0, "single char should have positive width");
}

// ═══════════════════════════════════════════════════════════════════════
// Font Size effects
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn different_sizes_different_ascent() {
    let font12 = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 12.0));
    let font48 = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 48.0));
    let m12 = font12.font_metrics().expect("metrics");
    let m48 = font48.font_metrics().expect("metrics");
    assert!(
        m48.ascent > m12.ascent,
        "48px ascent ({}) should be greater than 12px ascent ({})",
        m48.ascent, m12.ascent
    );
}

#[test]
fn different_sizes_different_line_spacing() {
    let font12 = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 12.0));
    let font48 = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 48.0));
    let m12 = font12.font_metrics().expect("metrics");
    let m48 = font48.font_metrics().expect("metrics");
    assert!(
        m48.line_spacing > m12.line_spacing,
        "48px line_spacing ({}) should be greater than 12px line_spacing ({})",
        m48.line_spacing, m12.line_spacing
    );
}

#[test]
fn different_sizes_different_width() {
    let font12 = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 12.0));
    let font48 = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 48.0));
    let w12 = font12.width("Hello");
    let w48 = font48.width("Hello");
    assert!(
        w48 > w12,
        "48px width ({}) should be greater than 12px width ({})",
        w48, w12
    );
}

#[test]
fn font_size_accessor() {
    let font = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 20.0));
    assert_eq!(font.size(), 20.0);
}

// ═══════════════════════════════════════════════════════════════════════
// GenericFontFamily — all variants resolve
// ═══════════════════════════════════════════════════════════════════════

fn try_generic(generic: GenericFontFamily) -> bool {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(generic),
        16.0,
    );
    Font::new(desc).primary_font().is_some()
}

#[test]
fn generic_serif_resolves() {
    assert!(try_generic(GenericFontFamily::Serif));
}

#[test]
fn generic_sans_serif_resolves() {
    assert!(try_generic(GenericFontFamily::SansSerif));
}

#[test]
fn generic_monospace_resolves() {
    assert!(try_generic(GenericFontFamily::Monospace));
}

#[test]
fn generic_none_resolves_to_sans_serif() {
    // GenericFontFamily::None maps to "sans-serif"
    assert!(try_generic(GenericFontFamily::None));
}

// ═══════════════════════════════════════════════════════════════════════
// FontPlatformData
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn platform_data_size_matches_request() {
    let font = Font::new(FontDescription::with_family_and_size(FontFamilyList::default(), 32.0));
    let data = font.primary_font().expect("should resolve");
    assert_eq!(data.size(), 32.0);
}

#[test]
fn platform_data_typeface_not_null() {
    let font = Font::new(FontDescription::default());
    let data = font.primary_font().expect("should resolve");
    // If we got here, typeface was successfully resolved
    let _tf = data.typeface();
}

#[test]
fn platform_data_sk_font_accessible() {
    let font = Font::new(FontDescription::default());
    let data = font.primary_font().expect("should resolve");
    let sk = data.sk_font();
    assert!(sk.size() > 0.0);
}

// ═══════════════════════════════════════════════════════════════════════
// ComputedStyle — font/text field defaults
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn computed_style_font_family_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_family, FontFamilyList::default());
}

#[test]
fn computed_style_font_size_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_size, 16.0);
}

#[test]
fn computed_style_font_weight_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_weight, FontWeight::NORMAL);
}

#[test]
fn computed_style_font_style_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_style, FontStyleEnum::Normal);
}

#[test]
fn computed_style_font_stretch_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_stretch, FontStretch::NORMAL);
}

#[test]
fn computed_style_line_height_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.line_height, LineHeight::Normal);
}

#[test]
fn computed_style_letter_spacing_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.letter_spacing, 0.0);
}

#[test]
fn computed_style_word_spacing_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.word_spacing, 0.0);
}

#[test]
fn computed_style_text_align_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_align, TextAlign::Start);
}

#[test]
fn computed_style_text_align_last_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_align_last, TextAlignLast::Auto);
}

#[test]
fn computed_style_text_decoration_line_default() {
    let s = ComputedStyle::initial();
    assert!(s.text_decoration_line.is_none());
    assert_eq!(s.text_decoration_line, TextDecorationLine::NONE);
}

#[test]
fn computed_style_text_decoration_style_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_decoration_style, TextDecorationStyle::Solid);
}

#[test]
fn computed_style_text_decoration_color_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_decoration_color, StyleColor::CurrentColor);
}

#[test]
fn computed_style_text_transform_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_transform, TextTransform::None);
}

#[test]
fn computed_style_text_overflow_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_overflow, TextOverflow::Clip);
}

#[test]
fn computed_style_vertical_align_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.vertical_align, VerticalAlign::Baseline);
}

#[test]
fn computed_style_unicode_bidi_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.unicode_bidi, UnicodeBidi::Normal);
}

#[test]
fn computed_style_writing_mode_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.writing_mode, WritingMode::HorizontalTb);
}

#[test]
fn computed_style_text_orientation_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_orientation, TextOrientation::Mixed);
}

#[test]
fn computed_style_text_rendering_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_rendering, TextRendering::Auto);
}

#[test]
fn computed_style_font_smoothing_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_smoothing, FontSmoothing::Auto);
}

#[test]
fn computed_style_text_shadow_default() {
    let s = ComputedStyle::initial();
    assert!(s.text_shadow.is_empty());
}

#[test]
fn computed_style_tab_size_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.tab_size, TabSize::Spaces(8));
}

#[test]
fn computed_style_word_break_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.word_break, WordBreak::Normal);
}

#[test]
fn computed_style_overflow_wrap_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.overflow_wrap, OverflowWrap::Normal);
}

#[test]
fn computed_style_hyphens_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.hyphens, Hyphens::Manual);
}

#[test]
fn computed_style_font_variant_caps_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_variant_caps, FontVariantCaps::Normal);
}

#[test]
fn computed_style_font_optical_sizing_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_optical_sizing, FontOpticalSizing::Auto);
}

#[test]
fn computed_style_font_synthesis_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.font_synthesis_weight, FontSynthesis::Auto);
    assert_eq!(s.font_synthesis_style, FontSynthesis::Auto);
}

#[test]
fn computed_style_text_justify_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_justify, TextJustify::Auto);
}

#[test]
fn computed_style_text_underline_position_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_underline_position, TextUnderlinePosition::Auto);
}

#[test]
fn computed_style_text_decoration_thickness_default() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_decoration_thickness, TextDecorationThickness::Auto);
}

// ═══════════════════════════════════════════════════════════════════════
// TextDecorationLine bitflags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn text_decoration_line_none() {
    let l = TextDecorationLine::NONE;
    assert!(l.is_none());
    assert!(!l.has_underline());
    assert!(!l.has_overline());
    assert!(!l.has_line_through());
}

#[test]
fn text_decoration_line_underline() {
    let l = TextDecorationLine::UNDERLINE;
    assert!(!l.is_none());
    assert!(l.has_underline());
    assert!(!l.has_overline());
}

#[test]
fn text_decoration_line_combined() {
    let l = TextDecorationLine(TextDecorationLine::UNDERLINE.0 | TextDecorationLine::LINE_THROUGH.0);
    assert!(l.has_underline());
    assert!(l.has_line_through());
    assert!(!l.has_overline());
}

// ═══════════════════════════════════════════════════════════════════════
// WritingMode helpers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn writing_mode_horizontal() {
    assert!(WritingMode::HorizontalTb.is_horizontal());
    assert!(!WritingMode::HorizontalTb.is_vertical());
}

#[test]
fn writing_mode_vertical() {
    assert!(WritingMode::VerticalRl.is_vertical());
    assert!(!WritingMode::VerticalRl.is_horizontal());
    assert!(WritingMode::VerticalLr.is_vertical());
}

// ═══════════════════════════════════════════════════════════════════════
// Font Debug / Display
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn font_debug_does_not_panic() {
    let font = Font::new(FontDescription::default());
    let debug = format!("{:?}", font);
    assert!(!debug.is_empty());
}

#[test]
fn font_description_debug_does_not_panic() {
    let desc = FontDescription::default();
    let debug = format!("{:?}", desc);
    assert!(debug.contains("FontDescription"));
}

#[test]
fn font_metrics_debug_does_not_panic() {
    let m = FontMetrics::default();
    let debug = format!("{:?}", m);
    assert!(debug.contains("FontMetrics"));
}
