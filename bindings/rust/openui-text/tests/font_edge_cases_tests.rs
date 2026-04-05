//! Edge-case tests for the openui-text font system.
//!
//! Covers: weight matching, style matching, stretch matching, fallback chains,
//! generic families, size edge cases, metrics edge cases, cache consistency,
//! FontDescription equality/cloning, and system font availability.

use std::sync::Arc;

use openui_style::{
    FontFamily, FontFamilyList, FontOpticalSizing, FontSmoothing, FontStretch, FontStyleEnum,
    FontSynthesis, FontVariantCaps, FontWeight, GenericFontFamily, TextRendering,
};

use openui_text::font::{Font, FontCache, FontDescription, FontFallbackList, FontMetrics};

// ═══════════════════════════════════════════════════════════════════════
// 1. Font Weight Matching (100–900, bold keyword)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn weight_thin_100_resolves() {
    let mut desc = FontDescription::new();
    desc.weight = FontWeight::THIN;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Thin (100) must resolve");
    assert_eq!(font.description().weight.0, 100.0);
}

#[test]
fn weight_light_300_resolves() {
    let mut desc = FontDescription::new();
    desc.weight = FontWeight::LIGHT;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Light (300) must resolve");
    assert_eq!(font.description().weight.0, 300.0);
}

#[test]
fn weight_bold_700_resolves() {
    let mut desc = FontDescription::new();
    desc.weight = FontWeight::BOLD;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Bold (700) must resolve");
    assert_eq!(font.description().weight.0, 700.0);
}

#[test]
fn weight_black_900_resolves() {
    let mut desc = FontDescription::new();
    desc.weight = FontWeight::BLACK;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Black (900) must resolve");
    assert_eq!(font.description().weight.0, 900.0);
}

#[test]
fn weight_custom_550_resolves() {
    let mut desc = FontDescription::new();
    desc.weight = FontWeight(550.0);
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Custom weight 550 must resolve");
    assert_eq!(font.description().weight.0, 550.0);
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Font Style Matching (normal, italic, oblique)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn style_normal_resolves() {
    let mut desc = FontDescription::new();
    desc.style = FontStyleEnum::Normal;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
    assert_eq!(font.description().style, FontStyleEnum::Normal);
}

#[test]
fn style_italic_resolves() {
    let mut desc = FontDescription::new();
    desc.style = FontStyleEnum::Italic;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Italic must resolve");
    assert_eq!(font.description().style, FontStyleEnum::Italic);
}

#[test]
fn style_oblique_14deg_resolves() {
    let mut desc = FontDescription::new();
    desc.style = FontStyleEnum::Oblique(14.0);
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Oblique 14° must resolve");
    assert_eq!(font.description().style, FontStyleEnum::Oblique(14.0));
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Font Stretch Matching (condensed through expanded)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn stretch_ultra_condensed_resolves() {
    let mut desc = FontDescription::new();
    desc.stretch = FontStretch::ULTRA_CONDENSED;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Ultra-condensed (50%) must resolve");
    assert_eq!(font.description().stretch.0, 50.0);
}

#[test]
fn stretch_condensed_resolves() {
    let mut desc = FontDescription::new();
    desc.stretch = FontStretch::CONDENSED;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Condensed (75%) must resolve");
    assert_eq!(font.description().stretch.0, 75.0);
}

#[test]
fn stretch_expanded_resolves() {
    let mut desc = FontDescription::new();
    desc.stretch = FontStretch::EXPANDED;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Expanded (125%) must resolve");
    assert_eq!(font.description().stretch.0, 125.0);
}

#[test]
fn stretch_ultra_expanded_resolves() {
    let mut desc = FontDescription::new();
    desc.stretch = FontStretch::ULTRA_EXPANDED;
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Ultra-expanded (200%) must resolve");
    assert_eq!(font.description().stretch.0, 200.0);
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Font Fallback Chains
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fallback_nonexistent_family_still_resolves() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::single("NonExistentFontXYZ12345"),
        16.0,
    );
    let font = Font::new(desc);
    // Should fall back to sans-serif default
    assert!(
        font.primary_font().is_some(),
        "Nonexistent family must fall back to system default"
    );
    assert!(font.fallback_count() >= 1);
}

#[test]
fn fallback_chain_nonexistent_then_generic() {
    let family = FontFamilyList {
        families: vec![
            FontFamily::Named("TotallyFakeFont999".into()),
            FontFamily::Generic(GenericFontFamily::Monospace),
        ],
    };
    let desc = FontDescription::with_family_and_size(family, 16.0);
    let font = Font::new(desc);
    assert!(font.primary_font().is_some(), "Should resolve via monospace fallback");
    // Monospace should resolve so we have at least 1 font
    assert!(font.fallback_count() >= 1);
}

#[test]
fn fallback_multiple_nonexistent_families() {
    let family = FontFamilyList {
        families: vec![
            FontFamily::Named("FakeA".into()),
            FontFamily::Named("FakeB".into()),
            FontFamily::Named("FakeC".into()),
        ],
    };
    let desc = FontDescription::with_family_and_size(family, 16.0);
    let font = Font::new(desc);
    // Even with all fake families, sans-serif default kicks in
    assert!(font.primary_font().is_some(), "Triple-fake must still resolve");
}

#[test]
fn fallback_list_direct_construction() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Serif),
        20.0,
    );
    let list = FontFallbackList::new(&desc);
    assert!(!list.is_empty(), "Serif fallback list must not be empty");
    assert!(list.primary().is_some());
    assert_eq!(list.len(), list.len()); // sanity
    // get(0) should match primary()
    assert!(Arc::ptr_eq(list.primary().unwrap(), list.get(0).unwrap()));
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Multiple Generic Families
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn generic_serif_resolves_with_metrics() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Serif),
        16.0,
    );
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
    let m = font.font_metrics().expect("Serif must have metrics");
    assert!(m.ascent > 0.0, "Serif ascent must be positive");
}

#[test]
fn generic_sans_serif_resolves_with_metrics() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        16.0,
    );
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
    let m = font.font_metrics().expect("SansSerif must have metrics");
    assert!(m.ascent > 0.0);
}

#[test]
fn generic_monospace_resolves_with_metrics() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Monospace),
        16.0,
    );
    let font = Font::new(desc);
    assert!(font.primary_font().is_some());
    let m = font.font_metrics().expect("Monospace must have metrics");
    assert!(m.zero_width > 0.0, "Monospace ch-width must be positive");
}

#[test]
fn generic_cursive_resolves() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Cursive),
        16.0,
    );
    let font = Font::new(desc);
    assert!(
        font.primary_font().is_some(),
        "Cursive generic should resolve on system"
    );
}

#[test]
fn generic_fantasy_resolves() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Fantasy),
        16.0,
    );
    let font = Font::new(desc);
    assert!(
        font.primary_font().is_some(),
        "Fantasy generic should resolve on system"
    );
}

#[test]
fn generic_system_ui_resolves() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SystemUi),
        16.0,
    );
    let font = Font::new(desc);
    assert!(
        font.primary_font().is_some(),
        "SystemUi generic should resolve on system"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Font Size Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn size_very_small_1px() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        1.0,
    );
    let font = Font::new(desc);
    assert_eq!(font.size(), 1.0);
    assert!(font.primary_font().is_some(), "1px font must resolve");
    let m = font.font_metrics().unwrap();
    assert!(m.ascent > 0.0, "Even at 1px, ascent should be positive");
    assert!(m.line_spacing > 0.0);
}

#[test]
fn size_very_large_1000px() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        1000.0,
    );
    let font = Font::new(desc);
    assert_eq!(font.size(), 1000.0);
    assert!(font.primary_font().is_some(), "1000px font must resolve");
    let m = font.font_metrics().unwrap();
    assert!(m.ascent > 100.0, "1000px ascent should be large");
}

#[test]
fn size_zero_still_resolves() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        0.0,
    );
    let font = Font::new(desc);
    assert_eq!(font.size(), 0.0);
    // Font should still resolve even at size 0 (Skia handles this)
    assert!(font.primary_font().is_some());
}

#[test]
fn size_affects_text_width() {
    let small = Font::new(FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        10.0,
    ));
    let large = Font::new(FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        40.0,
    ));
    let w_small = small.width("Hello");
    let w_large = large.width("Hello");
    assert!(
        w_large > w_small,
        "40px text width ({w_large}) must exceed 10px text width ({w_small})"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Font Metrics Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn metrics_line_spacing_equals_sum() {
    let font = Font::new(FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        16.0,
    ));
    let m = font.font_metrics().expect("Must have metrics");
    let computed = m.ascent + m.descent + m.line_gap;
    assert!(
        (m.line_spacing - computed).abs() < 0.01,
        "line_spacing ({}) must equal ascent({}) + descent({}) + line_gap({})",
        m.line_spacing,
        m.ascent,
        m.descent,
        m.line_gap,
    );
}

#[test]
fn metrics_ascent_positive_descent_non_negative() {
    let font = Font::new(FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::Serif),
        24.0,
    ));
    let m = font.font_metrics().unwrap();
    assert!(m.ascent > 0.0, "Ascent must be positive, got {}", m.ascent);
    assert!(m.descent >= 0.0, "Descent must be non-negative, got {}", m.descent);
}

#[test]
fn metrics_cap_height_ge_x_height() {
    let font = Font::new(FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        16.0,
    ));
    let m = font.font_metrics().unwrap();
    // For well-formed fonts, cap height >= x height
    assert!(
        m.cap_height >= m.x_height,
        "cap_height ({}) should be >= x_height ({})",
        m.cap_height,
        m.x_height,
    );
}

#[test]
fn metrics_zero_returns_all_zeroes() {
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

// ═══════════════════════════════════════════════════════════════════════
// 8. Font Cache Consistency
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn cache_repeated_lookup_returns_same_arc() {
    let mut cache = FontCache::new();
    let desc = FontDescription::new();
    let first = cache.get_font_platform_data("sans-serif", &desc);
    let second = cache.get_font_platform_data("sans-serif", &desc);
    assert!(first.is_some());
    assert!(second.is_some());
    assert!(
        Arc::ptr_eq(first.as_ref().unwrap(), second.as_ref().unwrap()),
        "Repeated lookups must return the same Arc"
    );
}

#[test]
fn cache_different_sizes_return_different_arcs() {
    let mut cache = FontCache::new();
    let desc_12 = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        12.0,
    );
    let desc_24 = FontDescription::with_family_and_size(
        FontFamilyList::generic(GenericFontFamily::SansSerif),
        24.0,
    );
    let a = cache.get_font_platform_data("sans-serif", &desc_12);
    let b = cache.get_font_platform_data("sans-serif", &desc_24);
    assert!(a.is_some());
    assert!(b.is_some());
    assert!(
        !Arc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap()),
        "Different sizes must produce different cache entries"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 9. FontDescription Equality and Cloning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn description_clone_preserves_all_fields() {
    let mut desc = FontDescription::new();
    desc.weight = FontWeight::BOLD;
    desc.stretch = FontStretch::CONDENSED;
    desc.style = FontStyleEnum::Italic;
    desc.variant_caps = FontVariantCaps::SmallCaps;
    desc.letter_spacing = 2.5;
    desc.word_spacing = 4.0;
    desc.locale = Some("ja-JP".into());
    desc.font_smoothing = FontSmoothing::Antialiased;
    desc.text_rendering = TextRendering::OptimizeLegibility;
    desc.font_synthesis_weight = FontSynthesis::None;
    desc.font_synthesis_style = FontSynthesis::None;
    desc.font_optical_sizing = FontOpticalSizing::None;

    let cloned = desc.clone();
    assert_eq!(cloned.weight, FontWeight::BOLD);
    assert_eq!(cloned.stretch, FontStretch::CONDENSED);
    assert_eq!(cloned.style, FontStyleEnum::Italic);
    assert_eq!(cloned.variant_caps, FontVariantCaps::SmallCaps);
    assert_eq!(cloned.letter_spacing, 2.5);
    assert_eq!(cloned.word_spacing, 4.0);
    assert_eq!(cloned.locale, Some("ja-JP".into()));
    assert_eq!(cloned.font_smoothing, FontSmoothing::Antialiased);
    assert_eq!(cloned.text_rendering, TextRendering::OptimizeLegibility);
    assert_eq!(cloned.font_synthesis_weight, FontSynthesis::None);
    assert_eq!(cloned.font_synthesis_style, FontSynthesis::None);
    assert_eq!(cloned.font_optical_sizing, FontOpticalSizing::None);
}

#[test]
fn description_with_family_and_size_sets_both_sizes() {
    let desc = FontDescription::with_family_and_size(
        FontFamilyList::single("Helvetica"),
        32.0,
    );
    assert_eq!(desc.size, 32.0);
    assert_eq!(desc.specified_size, 32.0);
    assert_eq!(desc.family.families.len(), 1);
    assert_eq!(desc.family.families[0], FontFamily::Named("Helvetica".into()));
    // Other fields should be defaults
    assert_eq!(desc.weight, FontWeight::NORMAL);
    assert_eq!(desc.stretch, FontStretch::NORMAL);
    assert_eq!(desc.style, FontStyleEnum::Normal);
}

#[test]
fn description_mutation_does_not_affect_clone() {
    let mut original = FontDescription::new();
    let clone_before = original.clone();

    original.weight = FontWeight::EXTRA_BOLD;
    original.size = 72.0;
    original.locale = Some("zh-CN".into());

    // Clone taken before mutation must remain unaffected
    assert_eq!(clone_before.weight, FontWeight::NORMAL);
    assert_eq!(clone_before.size, 16.0);
    assert_eq!(clone_before.locale, None);
}

// ═══════════════════════════════════════════════════════════════════════
// 10. System Font Availability
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn system_fonts_all_generic_families_resolve() {
    let generics = [
        GenericFontFamily::Serif,
        GenericFontFamily::SansSerif,
        GenericFontFamily::Monospace,
        GenericFontFamily::Cursive,
        GenericFontFamily::Fantasy,
        GenericFontFamily::SystemUi,
    ];
    for generic in &generics {
        let desc = FontDescription::with_family_and_size(
            FontFamilyList::generic(*generic),
            16.0,
        );
        let font = Font::new(desc);
        assert!(
            font.primary_font().is_some(),
            "{:?} generic family must resolve to a system font",
            generic,
        );
    }
}

#[test]
fn system_font_cache_name_mapping() {
    // Verify FontCache maps all required generics to string names
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Serif), "serif");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::SansSerif), "sans-serif");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Monospace), "monospace");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Cursive), "cursive");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::Fantasy), "fantasy");
    assert_eq!(FontCache::generic_family_name(GenericFontFamily::SystemUi), "system-ui");
}
