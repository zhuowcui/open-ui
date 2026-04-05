//! WPT-equivalent tests for CSS Fonts Module.
//!
//! Each test corresponds to behaviors verified by WPT css/css-fonts tests.
//! Categories: font-family, font-weight, font-style, font-stretch,
//! font-size, font-variant-*, font-feature-settings, font-variation-settings,
//! font-synthesis, font-optical-sizing, font-metrics, font-measurement,
//! and text shaping.

use openui_style::{
    ComputedStyle, FontFamily, FontFamilyList, FontOpticalSizing, FontSmoothing, FontStretch,
    FontStyleEnum, FontSynthesis, FontVariantCaps, FontWeight, GenericFontFamily, TextRendering,
    FontVariantLigatures, FontVariantNumeric, FontVariantEastAsian, FontVariantPosition,
    FontVariantAlternates, LigatureState, NumericFigure, NumericSpacing, NumericFraction,
    EastAsianForm, EastAsianWidth, FontFeature, FontVariation, FontPalette,
};
use openui_text::font::{Font, FontCache, FontDescription, FontFallbackList, FontMetrics};
use openui_text::{TextShaper, TextDirection, ShapeResult};
use openui_text::font::features::collect_font_features;

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

fn make_font(size: f32) -> Font {
    let mut desc = FontDescription::new();
    desc.size = size;
    desc.specified_size = size;
    Font::new(desc)
}

fn make_font_with_family(family: FontFamilyList, size: f32) -> Font {
    Font::new(FontDescription::with_family_and_size(family, size))
}

fn make_font_with_weight(weight: FontWeight) -> Font {
    let mut desc = FontDescription::new();
    desc.weight = weight;
    Font::new(desc)
}

fn make_font_with_style(style: FontStyleEnum) -> Font {
    let mut desc = FontDescription::new();
    desc.style = style;
    Font::new(desc)
}

fn make_font_with_stretch(stretch: FontStretch) -> Font {
    let mut desc = FontDescription::new();
    desc.stretch = stretch;
    Font::new(desc)
}

fn shape(text: &str, font: &Font, direction: TextDirection) -> ShapeResult {
    let shaper = TextShaper::new();
    shaper.shape(text, font, direction)
}

fn features_for(mutate: impl FnOnce(&mut FontDescription)) -> Vec<FontFeature> {
    let mut desc = FontDescription::new();
    mutate(&mut desc);
    collect_font_features(&desc)
}

fn has_feature(features: &[FontFeature], tag: &[u8; 4], value: u32) -> bool {
    features.iter().any(|f| &f.tag == tag && f.value == value)
}

fn lacks_feature(features: &[FontFeature], tag: &[u8; 4]) -> bool {
    features.iter().all(|f| &f.tag != tag)
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_family — 16 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_family {
    use super::*;

    #[test]
    fn default_family_is_sans_serif() {
        let desc = FontDescription::default();
        assert_eq!(desc.family.families.len(), 1);
        assert_eq!(
            desc.family.families[0],
            FontFamily::Generic(GenericFontFamily::SansSerif)
        );
    }

    #[test]
    fn family_list_single_named() {
        let list = FontFamilyList::single("Arial");
        assert_eq!(list.families.len(), 1);
        assert_eq!(list.families[0], FontFamily::Named("Arial".to_string()));
    }

    #[test]
    fn family_list_generic_serif() {
        let list = FontFamilyList::generic(GenericFontFamily::Serif);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Serif));
    }

    #[test]
    fn family_list_generic_monospace() {
        let list = FontFamilyList::generic(GenericFontFamily::Monospace);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Monospace));
    }

    #[test]
    fn family_list_generic_cursive() {
        let list = FontFamilyList::generic(GenericFontFamily::Cursive);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Cursive));
    }

    #[test]
    fn family_list_generic_fantasy() {
        let list = FontFamilyList::generic(GenericFontFamily::Fantasy);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Fantasy));
    }

    #[test]
    fn family_list_generic_system_ui() {
        let list = FontFamilyList::generic(GenericFontFamily::SystemUi);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::SystemUi));
    }

    #[test]
    fn family_list_generic_math() {
        let list = FontFamilyList::generic(GenericFontFamily::Math);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Math));
    }

    #[test]
    fn family_list_generic_emoji() {
        let list = FontFamilyList::generic(GenericFontFamily::Emoji);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::Emoji));
    }

    #[test]
    fn family_list_generic_fangsong() {
        let list = FontFamilyList::generic(GenericFontFamily::FangSong);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::FangSong));
    }

    #[test]
    fn family_list_generic_ui_serif() {
        let list = FontFamilyList::generic(GenericFontFamily::UiSerif);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::UiSerif));
    }

    #[test]
    fn family_list_generic_ui_sans_serif() {
        let list = FontFamilyList::generic(GenericFontFamily::UiSansSerif);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::UiSansSerif));
    }

    #[test]
    fn family_list_generic_ui_monospace() {
        let list = FontFamilyList::generic(GenericFontFamily::UiMonospace);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::UiMonospace));
    }

    #[test]
    fn family_list_generic_ui_rounded() {
        let list = FontFamilyList::generic(GenericFontFamily::UiRounded);
        assert_eq!(list.families[0], FontFamily::Generic(GenericFontFamily::UiRounded));
    }

    #[test]
    fn named_font_resolves_primary() {
        let font = make_font_with_family(FontFamilyList::single("sans-serif"), 16.0);
        assert!(font.primary_font().is_some(), "Named font should resolve");
    }

    #[test]
    fn fallback_chain_multiple_families() {
        let list = FontFamilyList {
            families: vec![
                FontFamily::Named("NonExistentFontXYZ123".to_string()),
                FontFamily::Generic(GenericFontFamily::SansSerif),
            ],
        };
        let font = make_font_with_family(list, 16.0);
        assert!(
            font.primary_font().is_some(),
            "Fallback chain should resolve via generic family"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_weight — 16 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_weight {
    use super::*;

    #[test]
    fn default_is_normal_400() {
        let desc = FontDescription::default();
        assert_eq!(desc.weight, FontWeight::NORMAL);
        assert_eq!(desc.weight.0, 400.0);
    }

    #[test]
    fn thin_is_100() {
        assert_eq!(FontWeight::THIN.0, 100.0);
    }

    #[test]
    fn light_is_300() {
        assert_eq!(FontWeight::LIGHT.0, 300.0);
    }

    #[test]
    fn normal_is_400() {
        assert_eq!(FontWeight::NORMAL.0, 400.0);
    }

    #[test]
    fn medium_is_500() {
        assert_eq!(FontWeight::MEDIUM.0, 500.0);
    }

    #[test]
    fn semi_bold_is_600() {
        assert_eq!(FontWeight::SEMI_BOLD.0, 600.0);
    }

    #[test]
    fn bold_is_700() {
        assert_eq!(FontWeight::BOLD.0, 700.0);
    }

    #[test]
    fn extra_bold_is_800() {
        assert_eq!(FontWeight::EXTRA_BOLD.0, 800.0);
    }

    #[test]
    fn black_is_900() {
        assert_eq!(FontWeight::BLACK.0, 900.0);
    }

    #[test]
    fn custom_weight_150() {
        let w = FontWeight(150.0);
        assert_eq!(w.0, 150.0);
    }

    #[test]
    fn custom_weight_1000() {
        let w = FontWeight(1000.0);
        assert_eq!(w.0, 1000.0);
    }

    #[test]
    fn custom_weight_1() {
        let w = FontWeight(1.0);
        assert_eq!(w.0, 1.0);
    }

    #[test]
    fn weight_stored_in_description() {
        let mut desc = FontDescription::new();
        desc.weight = FontWeight::BOLD;
        assert_eq!(desc.weight.0, 700.0);
    }

    #[test]
    fn normal_weight_font_resolves() {
        let font = make_font_with_weight(FontWeight::NORMAL);
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn bold_weight_font_resolves() {
        let font = make_font_with_weight(FontWeight::BOLD);
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn bold_text_wider_than_normal() {
        let normal = make_font_with_weight(FontWeight::NORMAL);
        let bold = make_font_with_weight(FontWeight::BOLD);
        let w_normal = normal.width("Hello World");
        let w_bold = bold.width("Hello World");
        // Bold glyphs are typically wider; at minimum both should be positive
        assert!(w_normal > 0.0, "Normal weight text should have positive width");
        assert!(w_bold > 0.0, "Bold weight text should have positive width");
        assert!(w_bold > w_normal, "bold text should be wider than normal");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_style — 11 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_style {
    use super::*;

    #[test]
    fn default_is_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.style, FontStyleEnum::Normal);
    }

    #[test]
    fn italic_variant() {
        let style = FontStyleEnum::Italic;
        assert_eq!(style, FontStyleEnum::Italic);
    }

    #[test]
    fn oblique_default_angle_14() {
        let style = FontStyleEnum::Oblique(14.0);
        if let FontStyleEnum::Oblique(angle) = style {
            assert_eq!(angle, 14.0);
        } else {
            panic!("Expected Oblique");
        }
    }

    #[test]
    fn oblique_custom_angle_20() {
        let style = FontStyleEnum::Oblique(20.0);
        if let FontStyleEnum::Oblique(angle) = style {
            assert_eq!(angle, 20.0);
        } else {
            panic!("Expected Oblique");
        }
    }

    #[test]
    fn oblique_negative_angle() {
        let style = FontStyleEnum::Oblique(-10.0);
        if let FontStyleEnum::Oblique(angle) = style {
            assert_eq!(angle, -10.0);
        } else {
            panic!("Expected Oblique");
        }
    }

    #[test]
    fn oblique_zero_angle() {
        let style = FontStyleEnum::Oblique(0.0);
        if let FontStyleEnum::Oblique(angle) = style {
            assert_eq!(angle, 0.0);
        } else {
            panic!("Expected Oblique");
        }
    }

    #[test]
    fn style_stored_in_description() {
        let mut desc = FontDescription::new();
        desc.style = FontStyleEnum::Italic;
        assert_eq!(desc.style, FontStyleEnum::Italic);
    }

    #[test]
    fn normal_style_resolves() {
        let font = make_font_with_style(FontStyleEnum::Normal);
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn italic_style_resolves() {
        let font = make_font_with_style(FontStyleEnum::Italic);
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn oblique_style_resolves() {
        let font = make_font_with_style(FontStyleEnum::Oblique(14.0));
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn style_equality() {
        assert_ne!(FontStyleEnum::Normal, FontStyleEnum::Italic);
        assert_ne!(FontStyleEnum::Italic, FontStyleEnum::Oblique(14.0));
        assert_eq!(FontStyleEnum::Oblique(14.0), FontStyleEnum::Oblique(14.0));
        assert_ne!(FontStyleEnum::Oblique(14.0), FontStyleEnum::Oblique(20.0));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_stretch — 12 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_stretch {
    use super::*;

    #[test]
    fn default_is_normal_100() {
        let desc = FontDescription::default();
        assert_eq!(desc.stretch, FontStretch::NORMAL);
        assert_eq!(desc.stretch.0, 100.0);
    }

    #[test]
    fn ultra_condensed_is_50() {
        assert_eq!(FontStretch::ULTRA_CONDENSED.0, 50.0);
    }

    #[test]
    fn extra_condensed_is_62_5() {
        assert_eq!(FontStretch::EXTRA_CONDENSED.0, 62.5);
    }

    #[test]
    fn condensed_is_75() {
        assert_eq!(FontStretch::CONDENSED.0, 75.0);
    }

    #[test]
    fn semi_condensed_is_87_5() {
        assert_eq!(FontStretch::SEMI_CONDENSED.0, 87.5);
    }

    #[test]
    fn normal_is_100() {
        assert_eq!(FontStretch::NORMAL.0, 100.0);
    }

    #[test]
    fn semi_expanded_is_112_5() {
        assert_eq!(FontStretch::SEMI_EXPANDED.0, 112.5);
    }

    #[test]
    fn expanded_is_125() {
        assert_eq!(FontStretch::EXPANDED.0, 125.0);
    }

    #[test]
    fn extra_expanded_is_150() {
        assert_eq!(FontStretch::EXTRA_EXPANDED.0, 150.0);
    }

    #[test]
    fn ultra_expanded_is_200() {
        assert_eq!(FontStretch::ULTRA_EXPANDED.0, 200.0);
    }

    #[test]
    fn custom_stretch_110() {
        let s = FontStretch(110.0);
        assert_eq!(s.0, 110.0);
    }

    #[test]
    fn stretch_stored_in_description() {
        let mut desc = FontDescription::new();
        desc.stretch = FontStretch::CONDENSED;
        assert_eq!(desc.stretch.0, 75.0);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_size — 13 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_size {
    use super::*;

    #[test]
    fn default_is_16px() {
        let desc = FontDescription::default();
        assert_eq!(desc.size, 16.0);
        assert_eq!(desc.specified_size, 16.0);
    }

    #[test]
    fn with_family_and_size_sets_both() {
        let desc = FontDescription::with_family_and_size(
            FontFamilyList::default_list(),
            24.0,
        );
        assert_eq!(desc.size, 24.0);
        assert_eq!(desc.specified_size, 24.0);
    }

    #[test]
    fn font_size_method_returns_correct_value() {
        let font = make_font(20.0);
        assert_eq!(font.size(), 20.0);
    }

    #[test]
    fn size_1px() {
        let font = make_font(1.0);
        assert_eq!(font.size(), 1.0);
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn size_200px() {
        let font = make_font(200.0);
        assert_eq!(font.size(), 200.0);
        assert!(font.primary_font().is_some());
    }

    #[test]
    fn size_affects_width() {
        let small = make_font(10.0);
        let large = make_font(40.0);
        let w_small = small.width("Hello");
        let w_large = large.width("Hello");
        assert!(
            w_large > w_small,
            "Larger font ({}) should produce wider text than smaller font ({})",
            w_large,
            w_small
        );
    }

    #[test]
    fn width_scales_approximately_with_size() {
        let base = make_font(16.0);
        let doubled = make_font(32.0);
        let w_base = base.width("Test");
        let w_doubled = doubled.width("Test");
        // Width should roughly double (within 20% tolerance)
        let ratio = w_doubled / w_base;
        assert!(
            ratio > 1.5 && ratio < 2.5,
            "Width ratio should be approximately 2.0, got {}",
            ratio
        );
    }

    #[test]
    fn size_8px() {
        let font = make_font(8.0);
        assert_eq!(font.size(), 8.0);
        let w = font.width("A");
        assert!(w > 0.0);
    }

    #[test]
    fn size_72px() {
        let font = make_font(72.0);
        assert_eq!(font.size(), 72.0);
        let w = font.width("A");
        assert!(w > 0.0);
    }

    #[test]
    fn size_12px() {
        let font = make_font(12.0);
        assert_eq!(font.size(), 12.0);
    }

    #[test]
    fn size_48px() {
        let font = make_font(48.0);
        assert_eq!(font.size(), 48.0);
    }

    #[test]
    fn size_affects_metrics() {
        let small = make_font(10.0);
        let large = make_font(40.0);
        let m_small = small.font_metrics();
        let m_large = large.font_metrics();
        assert!(m_small.is_some());
        assert!(m_large.is_some());
        let ms = m_small.unwrap();
        let ml = m_large.unwrap();
        assert!(
            ml.ascent > ms.ascent,
            "Larger font ascent ({}) should exceed smaller font ascent ({})",
            ml.ascent,
            ms.ascent
        );
    }

    #[test]
    fn different_sizes_give_different_line_spacing() {
        let small = make_font(10.0);
        let large = make_font(40.0);
        let ms = small.font_metrics().unwrap();
        let ml = large.font_metrics().unwrap();
        assert!(
            ml.line_spacing > ms.line_spacing,
            "Larger font line_spacing ({}) should exceed smaller ({})",
            ml.line_spacing,
            ms.line_spacing
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_variant_caps — 8 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_variant_caps {
    use super::*;

    #[test]
    fn default_is_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.variant_caps, FontVariantCaps::Normal);
    }

    #[test]
    fn small_caps() {
        let mut desc = FontDescription::new();
        desc.variant_caps = FontVariantCaps::SmallCaps;
        assert_eq!(desc.variant_caps, FontVariantCaps::SmallCaps);
    }

    #[test]
    fn all_small_caps() {
        let mut desc = FontDescription::new();
        desc.variant_caps = FontVariantCaps::AllSmallCaps;
        assert_eq!(desc.variant_caps, FontVariantCaps::AllSmallCaps);
    }

    #[test]
    fn petite_caps() {
        let mut desc = FontDescription::new();
        desc.variant_caps = FontVariantCaps::PetiteCaps;
        assert_eq!(desc.variant_caps, FontVariantCaps::PetiteCaps);
    }

    #[test]
    fn all_petite_caps() {
        let mut desc = FontDescription::new();
        desc.variant_caps = FontVariantCaps::AllPetiteCaps;
        assert_eq!(desc.variant_caps, FontVariantCaps::AllPetiteCaps);
    }

    #[test]
    fn unicase() {
        let mut desc = FontDescription::new();
        desc.variant_caps = FontVariantCaps::Unicase;
        assert_eq!(desc.variant_caps, FontVariantCaps::Unicase);
    }

    #[test]
    fn titling_caps() {
        let mut desc = FontDescription::new();
        desc.variant_caps = FontVariantCaps::TitlingCaps;
        assert_eq!(desc.variant_caps, FontVariantCaps::TitlingCaps);
    }

    #[test]
    fn small_caps_emits_smcp_feature() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::SmallCaps);
        assert!(has_feature(&f, b"smcp", 1), "SmallCaps should emit 'smcp'=1");
    }

    #[test]
    fn all_small_caps_emits_smcp_and_c2sc() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::AllSmallCaps);
        assert!(has_feature(&f, b"smcp", 1), "AllSmallCaps should emit 'smcp'=1");
        assert!(has_feature(&f, b"c2sc", 1), "AllSmallCaps should emit 'c2sc'=1");
    }

    #[test]
    fn petite_caps_emits_pcap() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::PetiteCaps);
        assert!(has_feature(&f, b"pcap", 1), "PetiteCaps should emit 'pcap'=1");
    }

    #[test]
    fn all_petite_caps_emits_pcap_and_c2pc() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::AllPetiteCaps);
        assert!(has_feature(&f, b"pcap", 1), "AllPetiteCaps should emit 'pcap'=1");
        assert!(has_feature(&f, b"c2pc", 1), "AllPetiteCaps should emit 'c2pc'=1");
    }

    #[test]
    fn unicase_emits_unic() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::Unicase);
        assert!(has_feature(&f, b"unic", 1), "Unicase should emit 'unic'=1");
    }

    #[test]
    fn titling_caps_emits_titl() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::TitlingCaps);
        assert!(has_feature(&f, b"titl", 1), "TitlingCaps should emit 'titl'=1");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_variant_ligatures — 10 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_variant_ligatures {
    use super::*;

    #[test]
    fn default_all_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.variant_ligatures.common, LigatureState::Normal);
        assert_eq!(desc.variant_ligatures.discretionary, LigatureState::Normal);
        assert_eq!(desc.variant_ligatures.historical, LigatureState::Normal);
        assert_eq!(desc.variant_ligatures.contextual, LigatureState::Normal);
    }

    #[test]
    fn normal_emits_no_features() {
        let f = features_for(|_| {});
        assert!(lacks_feature(&f, b"liga"));
        assert!(lacks_feature(&f, b"clig"));
        assert!(lacks_feature(&f, b"dlig"));
        assert!(lacks_feature(&f, b"hlig"));
        assert!(lacks_feature(&f, b"calt"));
    }

    #[test]
    fn common_enabled() {
        let f = features_for(|d| d.variant_ligatures.common = LigatureState::Enabled);
        assert!(has_feature(&f, b"liga", 1));
        assert!(has_feature(&f, b"clig", 1));
    }

    #[test]
    fn common_disabled() {
        let f = features_for(|d| d.variant_ligatures.common = LigatureState::Disabled);
        assert!(has_feature(&f, b"liga", 0));
        assert!(has_feature(&f, b"clig", 0));
    }

    #[test]
    fn discretionary_enabled() {
        let f = features_for(|d| d.variant_ligatures.discretionary = LigatureState::Enabled);
        assert!(has_feature(&f, b"dlig", 1));
    }

    #[test]
    fn discretionary_disabled() {
        let f = features_for(|d| d.variant_ligatures.discretionary = LigatureState::Disabled);
        assert!(has_feature(&f, b"dlig", 0));
    }

    #[test]
    fn historical_enabled() {
        let f = features_for(|d| d.variant_ligatures.historical = LigatureState::Enabled);
        assert!(has_feature(&f, b"hlig", 1));
    }

    #[test]
    fn historical_disabled() {
        let f = features_for(|d| d.variant_ligatures.historical = LigatureState::Disabled);
        assert!(has_feature(&f, b"hlig", 0));
    }

    #[test]
    fn contextual_enabled() {
        let f = features_for(|d| d.variant_ligatures.contextual = LigatureState::Enabled);
        assert!(has_feature(&f, b"calt", 1));
    }

    #[test]
    fn contextual_disabled() {
        let f = features_for(|d| d.variant_ligatures.contextual = LigatureState::Disabled);
        assert!(has_feature(&f, b"calt", 0));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_variant_numeric — 9 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_variant_numeric {
    use super::*;

    #[test]
    fn default_all_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.variant_numeric.figure, NumericFigure::Normal);
        assert_eq!(desc.variant_numeric.spacing, NumericSpacing::Normal);
        assert_eq!(desc.variant_numeric.fraction, NumericFraction::Normal);
        assert!(!desc.variant_numeric.ordinal);
        assert!(!desc.variant_numeric.slashed_zero);
    }

    #[test]
    fn lining_nums_emits_lnum() {
        let f = features_for(|d| d.variant_numeric.figure = NumericFigure::LiningNums);
        assert!(has_feature(&f, b"lnum", 1));
    }

    #[test]
    fn oldstyle_nums_emits_onum() {
        let f = features_for(|d| d.variant_numeric.figure = NumericFigure::OldstyleNums);
        assert!(has_feature(&f, b"onum", 1));
    }

    #[test]
    fn proportional_nums_emits_pnum() {
        let f = features_for(|d| d.variant_numeric.spacing = NumericSpacing::ProportionalNums);
        assert!(has_feature(&f, b"pnum", 1));
    }

    #[test]
    fn tabular_nums_emits_tnum() {
        let f = features_for(|d| d.variant_numeric.spacing = NumericSpacing::TabularNums);
        assert!(has_feature(&f, b"tnum", 1));
    }

    #[test]
    fn diagonal_fractions_emits_frac() {
        let f = features_for(|d| d.variant_numeric.fraction = NumericFraction::DiagonalFractions);
        assert!(has_feature(&f, b"frac", 1));
    }

    #[test]
    fn stacked_fractions_emits_afrc() {
        let f = features_for(|d| d.variant_numeric.fraction = NumericFraction::StackedFractions);
        assert!(has_feature(&f, b"afrc", 1));
    }

    #[test]
    fn ordinal_emits_ordn() {
        let f = features_for(|d| d.variant_numeric.ordinal = true);
        assert!(has_feature(&f, b"ordn", 1));
    }

    #[test]
    fn slashed_zero_emits_zero() {
        let f = features_for(|d| d.variant_numeric.slashed_zero = true);
        assert!(has_feature(&f, b"zero", 1));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_variant_east_asian — 9 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_variant_east_asian {
    use super::*;

    #[test]
    fn default_all_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.variant_east_asian.form, EastAsianForm::Normal);
        assert_eq!(desc.variant_east_asian.width, EastAsianWidth::Normal);
        assert!(!desc.variant_east_asian.ruby);
    }

    #[test]
    fn jis78_emits_jp78() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis78);
        assert!(has_feature(&f, b"jp78", 1));
    }

    #[test]
    fn jis83_emits_jp83() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis83);
        assert!(has_feature(&f, b"jp83", 1));
    }

    #[test]
    fn jis90_emits_jp90() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis90);
        assert!(has_feature(&f, b"jp90", 1));
    }

    #[test]
    fn jis04_emits_jp04() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis04);
        assert!(has_feature(&f, b"jp04", 1));
    }

    #[test]
    fn simplified_emits_smpl() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Simplified);
        assert!(has_feature(&f, b"smpl", 1));
    }

    #[test]
    fn traditional_emits_trad() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Traditional);
        assert!(has_feature(&f, b"trad", 1));
    }

    #[test]
    fn full_width_emits_fwid() {
        let f = features_for(|d| d.variant_east_asian.width = EastAsianWidth::FullWidth);
        assert!(has_feature(&f, b"fwid", 1));
    }

    #[test]
    fn ruby_emits_ruby() {
        let f = features_for(|d| d.variant_east_asian.ruby = true);
        assert!(has_feature(&f, b"ruby", 1));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_feature_settings — 6 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_feature_settings {
    use super::*;

    #[test]
    fn default_empty() {
        let desc = FontDescription::default();
        assert!(desc.feature_settings.is_empty());
    }

    #[test]
    fn custom_feature_kern_on() {
        let f = features_for(|d| {
            d.feature_settings.push(FontFeature {
                tag: *b"kern",
                value: 1,
            });
        });
        assert!(has_feature(&f, b"kern", 1));
    }

    #[test]
    fn custom_feature_kern_off() {
        let f = features_for(|d| {
            d.feature_settings.push(FontFeature {
                tag: *b"kern",
                value: 0,
            });
        });
        assert!(has_feature(&f, b"kern", 0));
    }

    #[test]
    fn explicit_features_appended_after_variants() {
        let f = features_for(|d| {
            d.variant_ligatures.common = LigatureState::Enabled;
            d.feature_settings.push(FontFeature {
                tag: *b"liga",
                value: 0,
            });
        });
        // Both should be present; the explicit one comes last and overrides
        let liga_features: Vec<_> = f.iter().filter(|ft| &ft.tag == b"liga").collect();
        assert!(
            liga_features.len() >= 2,
            "Both variant-derived and explicit features should be present"
        );
        // Last one should be the explicit override
        let last = liga_features.last().unwrap();
        assert_eq!(last.value, 0, "Explicit feature-settings should override");
    }

    #[test]
    fn multiple_custom_features() {
        let f = features_for(|d| {
            d.feature_settings.push(FontFeature { tag: *b"liga", value: 1 });
            d.feature_settings.push(FontFeature { tag: *b"kern", value: 1 });
            d.feature_settings.push(FontFeature { tag: *b"smcp", value: 1 });
        });
        assert!(has_feature(&f, b"liga", 1));
        assert!(has_feature(&f, b"kern", 1));
        assert!(has_feature(&f, b"smcp", 1));
    }

    #[test]
    fn collect_with_no_overrides_matches_variants() {
        let f = features_for(|d| {
            d.variant_numeric.ordinal = true;
            d.variant_numeric.slashed_zero = true;
        });
        assert!(has_feature(&f, b"ordn", 1));
        assert!(has_feature(&f, b"zero", 1));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_variation_settings — 5 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_variation_settings {
    use super::*;

    #[test]
    fn default_empty() {
        let desc = FontDescription::default();
        assert!(desc.variation_settings.is_empty());
    }

    #[test]
    fn custom_wght_variation() {
        let mut desc = FontDescription::new();
        desc.variation_settings.push(FontVariation {
            tag: *b"wght",
            value: 700.0,
        });
        assert_eq!(desc.variation_settings.len(), 1);
        assert_eq!(desc.variation_settings[0].tag, *b"wght");
        assert_eq!(desc.variation_settings[0].value, 700.0);
    }

    #[test]
    fn custom_wdth_variation() {
        let mut desc = FontDescription::new();
        desc.variation_settings.push(FontVariation {
            tag: *b"wdth",
            value: 125.0,
        });
        assert_eq!(desc.variation_settings[0].tag, *b"wdth");
        assert_eq!(desc.variation_settings[0].value, 125.0);
    }

    #[test]
    fn multiple_variations() {
        let mut desc = FontDescription::new();
        desc.variation_settings.push(FontVariation { tag: *b"wght", value: 450.0 });
        desc.variation_settings.push(FontVariation { tag: *b"wdth", value: 80.0 });
        desc.variation_settings.push(FontVariation { tag: *b"opsz", value: 12.0 });
        assert_eq!(desc.variation_settings.len(), 3);
    }

    #[test]
    fn variation_font_resolves() {
        let mut desc = FontDescription::new();
        desc.variation_settings.push(FontVariation { tag: *b"wght", value: 600.0 });
        let font = Font::new(desc);
        assert!(font.primary_font().is_some());
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_synthesis — 5 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_synthesis {
    use super::*;

    #[test]
    fn default_weight_is_auto() {
        let desc = FontDescription::default();
        assert_eq!(desc.font_synthesis_weight, FontSynthesis::Auto);
    }

    #[test]
    fn default_style_is_auto() {
        let desc = FontDescription::default();
        assert_eq!(desc.font_synthesis_style, FontSynthesis::Auto);
    }

    #[test]
    fn weight_none() {
        let mut desc = FontDescription::new();
        desc.font_synthesis_weight = FontSynthesis::None;
        assert_eq!(desc.font_synthesis_weight, FontSynthesis::None);
    }

    #[test]
    fn style_none() {
        let mut desc = FontDescription::new();
        desc.font_synthesis_style = FontSynthesis::None;
        assert_eq!(desc.font_synthesis_style, FontSynthesis::None);
    }

    #[test]
    fn both_none_resolves() {
        let mut desc = FontDescription::new();
        desc.font_synthesis_weight = FontSynthesis::None;
        desc.font_synthesis_style = FontSynthesis::None;
        let font = Font::new(desc);
        assert!(font.primary_font().is_some());
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_optical_sizing — 3 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_optical_sizing {
    use super::*;

    #[test]
    fn default_is_auto() {
        let desc = FontDescription::default();
        assert_eq!(desc.font_optical_sizing, FontOpticalSizing::Auto);
    }

    #[test]
    fn none_disables() {
        let mut desc = FontDescription::new();
        desc.font_optical_sizing = FontOpticalSizing::None;
        assert_eq!(desc.font_optical_sizing, FontOpticalSizing::None);
    }

    #[test]
    fn optical_sizing_none_resolves() {
        let mut desc = FontDescription::new();
        desc.font_optical_sizing = FontOpticalSizing::None;
        let font = Font::new(desc);
        assert!(font.primary_font().is_some());
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_metrics_validation — 12 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_metrics_validation {
    use super::*;

    #[test]
    fn resolved_font_has_metrics() {
        let font = make_font(16.0);
        assert!(font.font_metrics().is_some());
    }

    #[test]
    fn ascent_is_positive() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(m.ascent > 0.0, "Ascent should be positive, got {}", m.ascent);
    }

    #[test]
    fn descent_is_positive() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(m.descent > 0.0, "Descent should be positive, got {}", m.descent);
    }

    #[test]
    fn line_gap_non_negative() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(m.line_gap >= 0.0, "Line gap should be non-negative, got {}", m.line_gap);
    }

    #[test]
    fn line_spacing_equals_ascent_plus_descent_plus_gap() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        let expected = m.ascent + m.descent + m.line_gap;
        let diff = (m.line_spacing - expected).abs();
        assert!(
            diff < 0.01,
            "line_spacing ({}) should equal ascent ({}) + descent ({}) + line_gap ({})",
            m.line_spacing,
            m.ascent,
            m.descent,
            m.line_gap
        );
    }

    #[test]
    fn x_height_positive() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(m.x_height > 0.0, "x_height should be positive, got {}", m.x_height);
    }

    #[test]
    fn cap_height_greater_than_x_height() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(
            m.cap_height >= m.x_height,
            "cap_height ({}) should be >= x_height ({})",
            m.cap_height,
            m.x_height
        );
    }

    #[test]
    fn units_per_em_nonzero() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(m.units_per_em > 0, "units_per_em should be > 0, got {}", m.units_per_em);
    }

    #[test]
    fn metrics_consistent_across_same_font() {
        let font1 = make_font(16.0);
        let font2 = make_font(16.0);
        let m1 = font1.font_metrics().unwrap();
        let m2 = font2.font_metrics().unwrap();
        assert_eq!(m1.ascent, m2.ascent);
        assert_eq!(m1.descent, m2.descent);
        assert_eq!(m1.line_spacing, m2.line_spacing);
    }

    #[test]
    fn underline_thickness_positive() {
        let font = make_font(16.0);
        let m = font.font_metrics().unwrap();
        assert!(
            m.underline_thickness > 0.0,
            "underline_thickness should be positive, got {}",
            m.underline_thickness
        );
    }

    #[test]
    fn zero_metrics_all_zero() {
        let m = FontMetrics::zero();
        assert_eq!(m.ascent, 0.0);
        assert_eq!(m.descent, 0.0);
        assert_eq!(m.line_gap, 0.0);
        assert_eq!(m.line_spacing, 0.0);
        assert_eq!(m.x_height, 0.0);
        assert_eq!(m.cap_height, 0.0);
        assert_eq!(m.zero_width, 0.0);
        assert_eq!(m.units_per_em, 0);
    }

    #[test]
    fn default_metrics_equals_zero() {
        let def = FontMetrics::default();
        let zero = FontMetrics::zero();
        assert_eq!(def, zero);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_measurement — 12 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_measurement {
    use super::*;

    #[test]
    fn non_empty_text_has_positive_width() {
        let font = make_font(16.0);
        let w = font.width("Hello");
        assert!(w > 0.0, "Non-empty text should have positive width, got {}", w);
    }

    #[test]
    fn empty_string_has_zero_width() {
        let font = make_font(16.0);
        let w = font.width("");
        assert_eq!(w, 0.0, "Empty string should have zero width");
    }

    #[test]
    fn longer_text_is_wider() {
        let font = make_font(16.0);
        let short = font.width("Hi");
        let long = font.width("Hello World");
        assert!(
            long > short,
            "Longer text ({}) should be wider than shorter text ({})",
            long,
            short
        );
    }

    #[test]
    fn space_has_measurable_width() {
        let font = make_font(16.0);
        let w = font.width(" ");
        assert!(w > 0.0, "Space should have positive width, got {}", w);
    }

    #[test]
    fn single_char_has_positive_width() {
        let font = make_font(16.0);
        let w = font.width("A");
        assert!(w > 0.0);
    }

    #[test]
    fn width_scales_with_font_size() {
        let small = make_font(12.0);
        let big = make_font(36.0);
        let ws = small.width("Test");
        let wb = big.width("Test");
        assert!(wb > ws, "Bigger font width ({}) > smaller font width ({})", wb, ws);
    }

    #[test]
    fn digits_have_positive_width() {
        let font = make_font(16.0);
        let w = font.width("0123456789");
        assert!(w > 0.0);
    }

    #[test]
    fn punctuation_has_positive_width() {
        let font = make_font(16.0);
        let w = font.width(".,;:!?");
        assert!(w > 0.0);
    }

    #[test]
    fn fallback_count_at_least_one() {
        let font = make_font(16.0);
        assert!(
            font.fallback_count() >= 1,
            "Should have at least one font in fallback chain"
        );
    }

    #[test]
    fn repeated_text_proportional() {
        let font = make_font(16.0);
        let single = font.width("X");
        let triple = font.width("XXX");
        // With no kerning/ligatures, XXX should be roughly 3x X
        let ratio = triple / single;
        assert!(
            ratio > 2.5 && ratio < 3.5,
            "Triple X width ratio should be ~3.0, got {}",
            ratio
        );
    }

    #[test]
    fn tab_character_width() {
        let font = make_font(16.0);
        // Tab is a control character; measure should not panic
        let w = font.width("\t");
        assert!(w >= 0.0, "Tab width should be non-negative");
    }

    #[test]
    fn newline_width() {
        let font = make_font(16.0);
        // Newline is a control character
        let w = font.width("\n");
        assert!(w >= 0.0, "Newline width should be non-negative");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod text_shaping — 14 tests
// ═══════════════════════════════════════════════════════════════════════

mod text_shaping {
    use super::*;

    #[test]
    fn shape_result_character_count() {
        let font = make_font(16.0);
        let result = shape("Hello", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 5);
    }

    #[test]
    fn shape_result_has_positive_width() {
        let font = make_font(16.0);
        let result = shape("Hello", &font, TextDirection::Ltr);
        assert!(result.width() > 0.0, "Shaped text should have positive width");
    }

    #[test]
    fn shape_empty_string() {
        let font = make_font(16.0);
        let result = shape("", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 0);
        assert_eq!(result.num_glyphs(), 0);
        assert_eq!(result.width(), 0.0);
        assert!(result.runs.is_empty());
    }

    #[test]
    fn ltr_direction_preserved() {
        let font = make_font(16.0);
        let result = shape("Hello", &font, TextDirection::Ltr);
        assert_eq!(result.direction, TextDirection::Ltr);
    }

    #[test]
    fn rtl_direction_preserved() {
        let font = make_font(16.0);
        let result = shape("Hello", &font, TextDirection::Rtl);
        assert_eq!(result.direction, TextDirection::Rtl);
    }

    #[test]
    fn shape_result_has_runs() {
        let font = make_font(16.0);
        let result = shape("Hello", &font, TextDirection::Ltr);
        assert!(!result.runs.is_empty(), "Shaped text should have at least one run");
    }

    #[test]
    fn shape_result_has_character_data() {
        let font = make_font(16.0);
        let result = shape("Hello", &font, TextDirection::Ltr);
        assert_eq!(
            result.character_data.len(),
            result.num_characters,
            "character_data length should match num_characters"
        );
    }

    #[test]
    fn character_positions_monotonic_ltr() {
        let font = make_font(16.0);
        let result = shape("Hello World", &font, TextDirection::Ltr);
        for i in 1..result.character_data.len() {
            assert!(
                result.character_data[i].x_position >= result.character_data[i - 1].x_position,
                "LTR character positions should be monotonically increasing at index {}",
                i
            );
        }
    }

    #[test]
    fn longer_text_wider_shape_result() {
        let font = make_font(16.0);
        let short = shape("Hi", &font, TextDirection::Ltr);
        let long = shape("Hello World", &font, TextDirection::Ltr);
        assert!(
            long.width() > short.width(),
            "Longer shaped text ({}) should be wider than shorter ({})",
            long.width(),
            short.width()
        );
    }

    #[test]
    fn shape_single_character() {
        let font = make_font(16.0);
        let result = shape("A", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 1);
        assert!(result.num_glyphs() >= 1);
        assert!(result.width() > 0.0);
    }

    #[test]
    fn shape_with_spaces() {
        let font = make_font(16.0);
        let result = shape("a b c", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 5);
        assert!(result.width() > 0.0);
    }

    #[test]
    fn shape_digits() {
        let font = make_font(16.0);
        let result = shape("12345", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 5);
        assert!(result.width() > 0.0);
    }

    #[test]
    fn shape_result_glyph_count() {
        let font = make_font(16.0);
        let result = shape("Test", &font, TextDirection::Ltr);
        // At minimum, simple Latin should have 1 glyph per character
        assert!(
            result.num_glyphs() >= 4,
            "Expected at least 4 glyphs, got {}",
            result.num_glyphs()
        );
    }

    #[test]
    fn shape_width_consistent_with_font_width() {
        let font = make_font(16.0);
        let text = "Hello";
        let shaped_w = shape(text, &font, TextDirection::Ltr).width();
        let font_w = font.width(text);
        // Both should be positive and in the same ballpark
        assert!(shaped_w > 0.0);
        assert!(font_w > 0.0);
        // Allow a generous tolerance since shaping and measure may differ slightly
        let ratio = shaped_w / font_w;
        assert!(
            ratio > 0.5 && ratio < 2.0,
            "Shaped width ({}) and font.width ({}) should be in similar range (ratio: {})",
            shaped_w,
            font_w,
            ratio
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod computed_style_defaults — 5 tests
// ═══════════════════════════════════════════════════════════════════════

mod computed_style_defaults {
    use super::*;

    #[test]
    fn default_font_family_is_sans_serif() {
        let style = ComputedStyle::default();
        assert_eq!(style.font_family.families.len(), 1);
        assert_eq!(
            style.font_family.families[0],
            FontFamily::Generic(GenericFontFamily::SansSerif)
        );
    }

    #[test]
    fn default_font_size_is_16() {
        let style = ComputedStyle::default();
        assert_eq!(style.font_size, 16.0);
    }

    #[test]
    fn default_font_weight_is_normal() {
        let style = ComputedStyle::default();
        assert_eq!(style.font_weight, FontWeight::NORMAL);
    }

    #[test]
    fn default_font_style_is_normal() {
        let style = ComputedStyle::default();
        assert_eq!(style.font_style, FontStyleEnum::Normal);
    }

    #[test]
    fn default_font_palette_is_normal() {
        let style = ComputedStyle::default();
        assert_eq!(style.font_palette, FontPalette::Normal);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_smoothing_and_rendering — 6 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_smoothing_and_rendering {
    use super::*;

    #[test]
    fn default_smoothing_is_auto() {
        let desc = FontDescription::default();
        assert_eq!(desc.font_smoothing, FontSmoothing::Auto);
    }

    #[test]
    fn smoothing_none() {
        let s = FontSmoothing::None;
        assert_eq!(s, FontSmoothing::None);
    }

    #[test]
    fn smoothing_antialiased() {
        let s = FontSmoothing::Antialiased;
        assert_eq!(s, FontSmoothing::Antialiased);
    }

    #[test]
    fn smoothing_subpixel() {
        let s = FontSmoothing::SubpixelAntialiased;
        assert_eq!(s, FontSmoothing::SubpixelAntialiased);
    }

    #[test]
    fn default_text_rendering_is_auto() {
        let desc = FontDescription::default();
        assert_eq!(desc.text_rendering, TextRendering::Auto);
    }

    #[test]
    fn text_rendering_variants() {
        assert_ne!(TextRendering::Auto, TextRendering::OptimizeSpeed);
        assert_ne!(TextRendering::OptimizeSpeed, TextRendering::OptimizeLegibility);
        assert_ne!(TextRendering::OptimizeLegibility, TextRendering::GeometricPrecision);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_variant_position_and_alternates — 5 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_variant_position_and_alternates {
    use super::*;

    #[test]
    fn default_position_is_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.variant_position, FontVariantPosition::Normal);
    }

    #[test]
    fn sub_emits_subs() {
        let f = features_for(|d| d.variant_position = FontVariantPosition::Sub);
        assert!(has_feature(&f, b"subs", 1));
    }

    #[test]
    fn super_emits_sups() {
        let f = features_for(|d| d.variant_position = FontVariantPosition::Super);
        assert!(has_feature(&f, b"sups", 1));
    }

    #[test]
    fn default_alternates_is_normal() {
        let desc = FontDescription::default();
        assert_eq!(desc.variant_alternates, FontVariantAlternates::Normal);
    }

    #[test]
    fn historical_forms_emits_hist() {
        let f = features_for(|d| d.variant_alternates = FontVariantAlternates::HistoricalForms);
        assert!(has_feature(&f, b"hist", 1));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// mod font_spacing — 4 tests
// ═══════════════════════════════════════════════════════════════════════

mod font_spacing {
    use super::*;

    #[test]
    fn default_letter_spacing_zero() {
        let desc = FontDescription::default();
        assert_eq!(desc.letter_spacing, 0.0);
    }

    #[test]
    fn default_word_spacing_zero() {
        let desc = FontDescription::default();
        assert_eq!(desc.word_spacing, 0.0);
    }

    #[test]
    fn custom_letter_spacing() {
        let mut desc = FontDescription::new();
        desc.letter_spacing = 2.5;
        assert_eq!(desc.letter_spacing, 2.5);
    }

    #[test]
    fn custom_word_spacing() {
        let mut desc = FontDescription::new();
        desc.word_spacing = 4.0;
        assert_eq!(desc.word_spacing, 4.0);
    }
}
