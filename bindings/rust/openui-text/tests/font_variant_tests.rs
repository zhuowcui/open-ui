//! Comprehensive tests for font-variant-* → OpenType feature mapping.
//!
//! Tests cover every CSS font-variant keyword value and its OpenType feature tag(s),
//! verifying the feature collection logic in `openui_text::font::features`.

use openui_style::*;
use openui_text::font::features::collect_font_features;
use openui_text::FontDescription;

// ── Helpers ─────────────────────────────────────────────────────────────

/// Create a default FontDescription, apply mutations, and collect features.
fn features_for(mutate: impl FnOnce(&mut FontDescription)) -> Vec<FontFeature> {
    let mut desc = FontDescription::new();
    mutate(&mut desc);
    collect_font_features(&desc)
}

/// Check that the feature list contains a feature with the given tag and value.
fn has(features: &[FontFeature], tag: &[u8; 4], value: u32) -> bool {
    features.iter().any(|f| &f.tag == tag && f.value == value)
}

/// Check that NO feature with the given tag is present.
fn lacks(features: &[FontFeature], tag: &[u8; 4]) -> bool {
    features.iter().all(|f| &f.tag != tag)
}

// ── 1. Default / Normal ─────────────────────────────────────────────────

#[test]
fn t01_default_description_produces_no_features() {
    let features = features_for(|_| {});
    assert!(features.is_empty(), "Default description should emit no features");
}

// ── 2–13. font-variant-ligatures ────────────────────────────────────────

#[test]
fn t02_ligatures_common_enabled() {
    let f = features_for(|d| d.variant_ligatures.common = LigatureState::Enabled);
    assert!(has(&f, b"liga", 1));
    assert!(has(&f, b"clig", 1));
}

#[test]
fn t03_ligatures_common_disabled() {
    let f = features_for(|d| d.variant_ligatures.common = LigatureState::Disabled);
    assert!(has(&f, b"liga", 0));
    assert!(has(&f, b"clig", 0));
}

#[test]
fn t04_ligatures_common_normal_emits_nothing() {
    let f = features_for(|d| d.variant_ligatures.common = LigatureState::Normal);
    assert!(lacks(&f, b"liga"));
    assert!(lacks(&f, b"clig"));
}

#[test]
fn t05_ligatures_discretionary_enabled() {
    let f = features_for(|d| d.variant_ligatures.discretionary = LigatureState::Enabled);
    assert!(has(&f, b"dlig", 1));
}

#[test]
fn t06_ligatures_discretionary_disabled() {
    let f = features_for(|d| d.variant_ligatures.discretionary = LigatureState::Disabled);
    assert!(has(&f, b"dlig", 0));
}

#[test]
fn t07_ligatures_historical_enabled() {
    let f = features_for(|d| d.variant_ligatures.historical = LigatureState::Enabled);
    assert!(has(&f, b"hlig", 1));
}

#[test]
fn t08_ligatures_historical_disabled() {
    let f = features_for(|d| d.variant_ligatures.historical = LigatureState::Disabled);
    assert!(has(&f, b"hlig", 0));
}

#[test]
fn t09_ligatures_contextual_enabled() {
    let f = features_for(|d| d.variant_ligatures.contextual = LigatureState::Enabled);
    assert!(has(&f, b"calt", 1));
}

#[test]
fn t10_ligatures_contextual_disabled() {
    let f = features_for(|d| d.variant_ligatures.contextual = LigatureState::Disabled);
    assert!(has(&f, b"calt", 0));
}

#[test]
fn t11_ligatures_none_disables_all() {
    let f = features_for(|d| d.variant_ligatures = FontVariantLigatures::none());
    assert!(has(&f, b"liga", 0));
    assert!(has(&f, b"clig", 0));
    assert!(has(&f, b"dlig", 0));
    assert!(has(&f, b"hlig", 0));
    assert!(has(&f, b"calt", 0));
    assert_eq!(f.len(), 5); // liga + clig (from common) + dlig + hlig + calt
}

#[test]
fn t12_ligatures_normal_no_features() {
    let f = features_for(|d| d.variant_ligatures = FontVariantLigatures::NORMAL);
    assert!(f.is_empty());
}

#[test]
fn t13_ligatures_mixed_states() {
    let f = features_for(|d| {
        d.variant_ligatures = FontVariantLigatures {
            common: LigatureState::Disabled,
            discretionary: LigatureState::Enabled,
            historical: LigatureState::Normal,
            contextual: LigatureState::Disabled,
        };
    });
    assert!(has(&f, b"liga", 0));
    assert!(has(&f, b"clig", 0));
    assert!(has(&f, b"dlig", 1));
    assert!(lacks(&f, b"hlig"));
    assert!(has(&f, b"calt", 0));
}

// ── 14–20. font-variant-caps ────────────────────────────────────────────

#[test]
fn t14_caps_normal() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::Normal);
    assert!(lacks(&f, b"smcp"));
    assert!(lacks(&f, b"c2sc"));
    assert!(lacks(&f, b"pcap"));
    assert!(lacks(&f, b"c2pc"));
    assert!(lacks(&f, b"unic"));
    assert!(lacks(&f, b"titl"));
}

#[test]
fn t15_caps_small_caps() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::SmallCaps);
    assert!(has(&f, b"smcp", 1));
    assert!(lacks(&f, b"c2sc"));
    assert_eq!(f.len(), 1);
}

#[test]
fn t16_caps_all_small_caps() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::AllSmallCaps);
    assert!(has(&f, b"smcp", 1));
    assert!(has(&f, b"c2sc", 1));
    assert_eq!(f.len(), 2);
}

#[test]
fn t17_caps_petite_caps() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::PetiteCaps);
    assert!(has(&f, b"pcap", 1));
    assert!(lacks(&f, b"c2pc"));
    assert_eq!(f.len(), 1);
}

#[test]
fn t18_caps_all_petite_caps() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::AllPetiteCaps);
    assert!(has(&f, b"pcap", 1));
    assert!(has(&f, b"c2pc", 1));
    assert_eq!(f.len(), 2);
}

#[test]
fn t19_caps_unicase() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::Unicase);
    assert!(has(&f, b"unic", 1));
    assert_eq!(f.len(), 1);
}

#[test]
fn t20_caps_titling() {
    let f = features_for(|d| d.variant_caps = FontVariantCaps::TitlingCaps);
    assert!(has(&f, b"titl", 1));
    assert_eq!(f.len(), 1);
}

// ── 21–29. font-variant-numeric ─────────────────────────────────────────

#[test]
fn t21_numeric_lining_nums() {
    let f = features_for(|d| d.variant_numeric.figure = NumericFigure::LiningNums);
    assert!(has(&f, b"lnum", 1));
    assert!(lacks(&f, b"onum"));
}

#[test]
fn t22_numeric_oldstyle_nums() {
    let f = features_for(|d| d.variant_numeric.figure = NumericFigure::OldstyleNums);
    assert!(has(&f, b"onum", 1));
    assert!(lacks(&f, b"lnum"));
}

#[test]
fn t23_numeric_proportional_nums() {
    let f = features_for(|d| d.variant_numeric.spacing = NumericSpacing::ProportionalNums);
    assert!(has(&f, b"pnum", 1));
    assert!(lacks(&f, b"tnum"));
}

#[test]
fn t24_numeric_tabular_nums() {
    let f = features_for(|d| d.variant_numeric.spacing = NumericSpacing::TabularNums);
    assert!(has(&f, b"tnum", 1));
    assert!(lacks(&f, b"pnum"));
}

#[test]
fn t25_numeric_diagonal_fractions() {
    let f = features_for(|d| d.variant_numeric.fraction = NumericFraction::DiagonalFractions);
    assert!(has(&f, b"frac", 1));
    assert!(lacks(&f, b"afrc"));
}

#[test]
fn t26_numeric_stacked_fractions() {
    let f = features_for(|d| d.variant_numeric.fraction = NumericFraction::StackedFractions);
    assert!(has(&f, b"afrc", 1));
    assert!(lacks(&f, b"frac"));
}

#[test]
fn t27_numeric_ordinal() {
    let f = features_for(|d| d.variant_numeric.ordinal = true);
    assert!(has(&f, b"ordn", 1));
}

#[test]
fn t28_numeric_slashed_zero() {
    let f = features_for(|d| d.variant_numeric.slashed_zero = true);
    assert!(has(&f, b"zero", 1));
}

#[test]
fn t29_numeric_normal_no_features() {
    let f = features_for(|d| d.variant_numeric = FontVariantNumeric::NORMAL);
    assert!(lacks(&f, b"lnum"));
    assert!(lacks(&f, b"onum"));
    assert!(lacks(&f, b"pnum"));
    assert!(lacks(&f, b"tnum"));
    assert!(lacks(&f, b"frac"));
    assert!(lacks(&f, b"afrc"));
    assert!(lacks(&f, b"ordn"));
    assert!(lacks(&f, b"zero"));
}

// ── 30–39. font-variant-east-asian ──────────────────────────────────────

#[test]
fn t30_east_asian_jis78() {
    let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis78);
    assert!(has(&f, b"jp78", 1));
}

#[test]
fn t31_east_asian_jis83() {
    let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis83);
    assert!(has(&f, b"jp83", 1));
}

#[test]
fn t32_east_asian_jis90() {
    let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis90);
    assert!(has(&f, b"jp90", 1));
}

#[test]
fn t33_east_asian_jis04() {
    let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis04);
    assert!(has(&f, b"jp04", 1));
}

#[test]
fn t34_east_asian_simplified() {
    let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Simplified);
    assert!(has(&f, b"smpl", 1));
}

#[test]
fn t35_east_asian_traditional() {
    let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Traditional);
    assert!(has(&f, b"trad", 1));
}

#[test]
fn t36_east_asian_full_width() {
    let f = features_for(|d| d.variant_east_asian.width = EastAsianWidth::FullWidth);
    assert!(has(&f, b"fwid", 1));
}

#[test]
fn t37_east_asian_proportional_width() {
    let f = features_for(|d| d.variant_east_asian.width = EastAsianWidth::ProportionalWidth);
    assert!(has(&f, b"pwid", 1));
}

#[test]
fn t38_east_asian_ruby() {
    let f = features_for(|d| d.variant_east_asian.ruby = true);
    assert!(has(&f, b"ruby", 1));
}

#[test]
fn t39_east_asian_normal_no_features() {
    let f = features_for(|d| d.variant_east_asian = FontVariantEastAsian::NORMAL);
    assert!(f.is_empty());
}

// ── 40–42. font-variant-position ────────────────────────────────────────

#[test]
fn t40_position_sub() {
    let f = features_for(|d| d.variant_position = FontVariantPosition::Sub);
    assert!(has(&f, b"subs", 1));
    assert!(lacks(&f, b"sups"));
}

#[test]
fn t41_position_super() {
    let f = features_for(|d| d.variant_position = FontVariantPosition::Super);
    assert!(has(&f, b"sups", 1));
    assert!(lacks(&f, b"subs"));
}

#[test]
fn t42_position_normal_no_features() {
    let f = features_for(|d| d.variant_position = FontVariantPosition::Normal);
    assert!(lacks(&f, b"subs"));
    assert!(lacks(&f, b"sups"));
}

// ── 43–44. font-variant-alternates ──────────────────────────────────────

#[test]
fn t43_alternates_historical_forms() {
    let f = features_for(|d| d.variant_alternates = FontVariantAlternates::HistoricalForms);
    assert!(has(&f, b"hist", 1));
}

#[test]
fn t44_alternates_normal_no_features() {
    let f = features_for(|d| d.variant_alternates = FontVariantAlternates::Normal);
    assert!(lacks(&f, b"hist"));
}

// ── 45–47. Explicit font-feature-settings ───────────────────────────────

#[test]
fn t45_explicit_feature_settings_appended() {
    let f = features_for(|d| {
        d.feature_settings.push(FontFeature { tag: *b"kern", value: 0 });
    });
    assert!(has(&f, b"kern", 0));
    assert_eq!(f.len(), 1);
}

#[test]
fn t46_explicit_settings_override_variant_last_wins() {
    let f = features_for(|d| {
        d.variant_caps = FontVariantCaps::SmallCaps; // emits "smcp" on
        d.feature_settings.push(FontFeature { tag: *b"smcp", value: 0 }); // overrides off
    });
    // Both present; explicit comes after variant (HarfBuzz last-wins).
    let smcp_entries: Vec<_> = f.iter()
        .enumerate()
        .filter(|(_, feat)| &feat.tag == b"smcp")
        .collect();
    assert_eq!(smcp_entries.len(), 2);
    assert_eq!(smcp_entries[0].1.value, 1); // variant
    assert_eq!(smcp_entries[1].1.value, 0); // explicit
    assert!(smcp_entries[1].0 > smcp_entries[0].0);
}

#[test]
fn t47_multiple_explicit_features() {
    let f = features_for(|d| {
        d.feature_settings.push(FontFeature { tag: *b"liga", value: 0 });
        d.feature_settings.push(FontFeature { tag: *b"kern", value: 1 });
        d.feature_settings.push(FontFeature { tag: *b"smcp", value: 1 });
    });
    assert_eq!(f.len(), 3);
    assert!(has(&f, b"liga", 0));
    assert!(has(&f, b"kern", 1));
    assert!(has(&f, b"smcp", 1));
}

// ── 48–53. Combinations ─────────────────────────────────────────────────

#[test]
fn t48_multiple_variants_combined() {
    let f = features_for(|d| {
        d.variant_ligatures.discretionary = LigatureState::Enabled;
        d.variant_caps = FontVariantCaps::SmallCaps;
        d.variant_numeric.slashed_zero = true;
        d.variant_position = FontVariantPosition::Sub;
    });
    assert!(has(&f, b"dlig", 1));
    assert!(has(&f, b"smcp", 1));
    assert!(has(&f, b"zero", 1));
    assert!(has(&f, b"subs", 1));
    assert_eq!(f.len(), 4);
}

#[test]
fn t49_all_numeric_at_once() {
    let f = features_for(|d| {
        d.variant_numeric = FontVariantNumeric {
            figure: NumericFigure::OldstyleNums,
            spacing: NumericSpacing::TabularNums,
            fraction: NumericFraction::DiagonalFractions,
            ordinal: true,
            slashed_zero: true,
        };
    });
    assert!(has(&f, b"onum", 1));
    assert!(has(&f, b"tnum", 1));
    assert!(has(&f, b"frac", 1));
    assert!(has(&f, b"ordn", 1));
    assert!(has(&f, b"zero", 1));
    assert_eq!(f.len(), 5);
}

#[test]
fn t50_east_asian_form_plus_width_plus_ruby() {
    let f = features_for(|d| {
        d.variant_east_asian = FontVariantEastAsian {
            form: EastAsianForm::Jis04,
            width: EastAsianWidth::ProportionalWidth,
            ruby: true,
        };
    });
    assert!(has(&f, b"jp04", 1));
    assert!(has(&f, b"pwid", 1));
    assert!(has(&f, b"ruby", 1));
    assert_eq!(f.len(), 3);
}

#[test]
fn t51_all_variants_active_simultaneously() {
    let f = features_for(|d| {
        d.variant_ligatures = FontVariantLigatures {
            common: LigatureState::Enabled,
            discretionary: LigatureState::Enabled,
            historical: LigatureState::Enabled,
            contextual: LigatureState::Enabled,
        };
        d.variant_caps = FontVariantCaps::AllSmallCaps;
        d.variant_numeric = FontVariantNumeric {
            figure: NumericFigure::LiningNums,
            spacing: NumericSpacing::TabularNums,
            fraction: NumericFraction::DiagonalFractions,
            ordinal: true,
            slashed_zero: true,
        };
        d.variant_east_asian = FontVariantEastAsian {
            form: EastAsianForm::Jis78,
            width: EastAsianWidth::FullWidth,
            ruby: true,
        };
        d.variant_position = FontVariantPosition::Super;
        d.variant_alternates = FontVariantAlternates::HistoricalForms;
    });
    // Ligatures: liga, clig, dlig, hlig, calt = 5
    assert!(has(&f, b"liga", 1));
    assert!(has(&f, b"clig", 1));
    assert!(has(&f, b"dlig", 1));
    assert!(has(&f, b"hlig", 1));
    assert!(has(&f, b"calt", 1));
    // Caps: smcp + c2sc = 2
    assert!(has(&f, b"smcp", 1));
    assert!(has(&f, b"c2sc", 1));
    // Numeric: lnum, tnum, frac, ordn, zero = 5
    assert!(has(&f, b"lnum", 1));
    assert!(has(&f, b"tnum", 1));
    assert!(has(&f, b"frac", 1));
    assert!(has(&f, b"ordn", 1));
    assert!(has(&f, b"zero", 1));
    // East Asian: jp78, fwid, ruby = 3
    assert!(has(&f, b"jp78", 1));
    assert!(has(&f, b"fwid", 1));
    assert!(has(&f, b"ruby", 1));
    // Position: sups = 1
    assert!(has(&f, b"sups", 1));
    // Alternates: hist = 1
    assert!(has(&f, b"hist", 1));
    // Total: 5 + 2 + 5 + 3 + 1 + 1 = 17
    assert_eq!(f.len(), 17);
}

#[test]
fn t52_feature_order_ligatures_caps_numeric_eastasian_position_alternates_explicit() {
    let f = features_for(|d| {
        d.variant_caps = FontVariantCaps::SmallCaps;
        d.variant_ligatures.common = LigatureState::Disabled;
        d.variant_numeric.ordinal = true;
        d.variant_east_asian.ruby = true;
        d.variant_position = FontVariantPosition::Sub;
        d.variant_alternates = FontVariantAlternates::HistoricalForms;
        d.feature_settings.push(FontFeature { tag: *b"kern", value: 1 });
    });
    let tags: Vec<[u8; 4]> = f.iter().map(|feat| feat.tag).collect();
    assert_eq!(
        tags,
        vec![*b"liga", *b"clig", *b"smcp", *b"ordn", *b"ruby", *b"subs", *b"hist", *b"kern"]
    );
}

#[test]
fn t53_ligatures_none_plus_explicit_liga_on() {
    // font-variant-ligatures: none + font-feature-settings: "liga" 1
    let f = features_for(|d| {
        d.variant_ligatures = FontVariantLigatures::none();
        d.feature_settings.push(FontFeature { tag: *b"liga", value: 1 });
    });
    // The variant disables liga (0), but explicit re-enables it (1).
    // Both present; explicit comes last.
    let liga_entries: Vec<_> = f.iter().filter(|feat| &feat.tag == b"liga").collect();
    assert_eq!(liga_entries.len(), 2);
    assert_eq!(liga_entries[0].value, 0); // from variant
    assert_eq!(liga_entries[1].value, 1); // from explicit
}

// ── 54–58. Default trait / constructors ─────────────────────────────────

#[test]
fn t54_ligatures_default_is_normal() {
    assert_eq!(FontVariantLigatures::default(), FontVariantLigatures::NORMAL);
}

#[test]
fn t55_numeric_default_is_normal() {
    assert_eq!(FontVariantNumeric::default(), FontVariantNumeric::NORMAL);
}

#[test]
fn t56_east_asian_default_is_normal() {
    assert_eq!(FontVariantEastAsian::default(), FontVariantEastAsian::NORMAL);
}

#[test]
fn t57_position_default_is_normal() {
    assert_eq!(FontVariantPosition::default(), FontVariantPosition::Normal);
}

#[test]
fn t58_alternates_default_is_normal() {
    assert_eq!(FontVariantAlternates::default(), FontVariantAlternates::Normal);
}

// ── 59–60. tag_to_u32 conversion ────────────────────────────────────────

#[test]
fn t59_tag_to_u32_liga() {
    use openui_text::font::features::tag_to_u32;
    // "liga" = 0x6C696761
    let tag = tag_to_u32(b"liga");
    assert_eq!(tag, 0x6C696761);
}

#[test]
fn t60_tag_to_u32_smcp() {
    use openui_text::font::features::tag_to_u32;
    // "smcp" = 0x736D6370
    let tag = tag_to_u32(b"smcp");
    assert_eq!(tag, 0x736D6370);
}
