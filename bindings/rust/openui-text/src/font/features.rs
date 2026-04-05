//! OpenType feature collection from CSS font-variant-* properties.
//!
//! Mirrors Blink's `FontFeatures` helper (`platform/fonts/font_features.cc`).
//! Each CSS `font-variant-*` property is mapped to one or more OpenType feature
//! tags following the CSS Fonts Level 4 specification.
//!
//! The collected features are passed to HarfBuzz during text shaping. Explicit
//! `font-feature-settings` are appended last so they override variant-derived
//! features (per the CSS cascade).

use openui_style::{
    EastAsianForm, EastAsianWidth, FontFeature, FontVariantAlternates, FontVariantCaps,
    FontVariantEastAsian, FontVariantLigatures, FontVariantNumeric, FontVariantPosition,
    LigatureState, NumericFigure, NumericFraction, NumericSpacing,
};

use super::description::FontDescription;

/// Build a `FontFeature` from a 4-byte tag and value.
#[inline]
fn feature(tag: &[u8; 4], value: u32) -> FontFeature {
    FontFeature {
        tag: *tag,
        value,
    }
}

/// Enable an OpenType feature (`value = 1`).
#[inline]
fn on(tag: &[u8; 4]) -> FontFeature {
    feature(tag, 1)
}

/// Disable an OpenType feature (`value = 0`).
#[inline]
fn off(tag: &[u8; 4]) -> FontFeature {
    feature(tag, 0)
}

/// Convert a 4-byte tag (`[u8; 4]`) to the `u32` OpenType tag representation
/// used by Skia/HarfBuzz (big-endian encoding).
#[inline]
pub fn tag_to_u32(tag: &[u8; 4]) -> u32 {
    ((tag[0] as u32) << 24) | ((tag[1] as u32) << 16) | ((tag[2] as u32) << 8) | (tag[3] as u32)
}

/// Convert a `FontFeature` to a Skia `SkShaper::Feature`, applying to the
/// entire text range `[0, text_len)`.
///
/// Skia's `SkShaper_Feature` has `{ tag: u32, value: u32, start: usize, end: usize }`.
/// We apply each feature to the whole shaping run (start=0, end=text_len).
pub fn to_skia_features(
    features: &[FontFeature],
    text_len: usize,
) -> Vec<skia_safe::shaper::Feature> {
    features
        .iter()
        .map(|f| skia_safe::shaper::Feature {
            tag: tag_to_u32(&f.tag),
            value: f.value,
            start: 0,
            end: text_len,
        })
        .collect()
}

/// Collect all OpenType features implied by a `FontDescription`.
///
/// This is the main entry point, mirroring Blink's
/// `FontFeatures::FromFontDescription()` in `font_features.cc`.
///
/// Features are added in a fixed order matching the CSS specification's
/// resolution order:
///   1. `font-variant-ligatures`
///   2. `font-variant-caps`
///   3. `font-variant-numeric`
///   4. `font-variant-east-asian`
///   5. `font-variant-position`
///   6. `font-variant-alternates`
///   7. Explicit `font-feature-settings` (override everything above)
pub fn collect_font_features(desc: &FontDescription) -> Vec<FontFeature> {
    let mut features = Vec::new();

    add_ligature_features(&desc.variant_ligatures, &mut features);
    add_caps_features(desc.variant_caps, &mut features);
    add_numeric_features(&desc.variant_numeric, &mut features);
    add_east_asian_features(&desc.variant_east_asian, &mut features);
    add_position_features(desc.variant_position, &mut features);
    add_alternates_features(desc.variant_alternates, &mut features);

    // Explicit font-feature-settings override all variant-derived features.
    // Appended last so HarfBuzz sees them with higher priority.
    features.extend_from_slice(&desc.feature_settings);

    features
}

// ── font-variant-ligatures ──────────────────────────────────────────────

/// Map `font-variant-ligatures` sub-properties to OpenType features.
///
/// Blink: `FontFeatures::Initialize()` in `font_features.cc`, the ligature
/// section that checks `common_ligatures_state_`, etc.
///
/// - `common-ligatures` → `"liga"` on, `no-common-ligatures` → `"liga"` off + `"clig"` off
/// - `discretionary-ligatures` → `"dlig"` on, `no-discretionary-ligatures` → `"dlig"` off
/// - `historical-ligatures` → `"hlig"` on, `no-historical-ligatures` → `"hlig"` off
/// - `contextual` → `"calt"` on, `no-contextual` → `"calt"` off
fn add_ligature_features(lig: &FontVariantLigatures, out: &mut Vec<FontFeature>) {
    match lig.common {
        LigatureState::Normal => {}
        LigatureState::Enabled => {
            out.push(on(b"liga"));
            out.push(on(b"clig"));
        }
        LigatureState::Disabled => {
            out.push(off(b"liga"));
            out.push(off(b"clig"));
        }
    }

    match lig.discretionary {
        LigatureState::Normal => {}
        LigatureState::Enabled => out.push(on(b"dlig")),
        LigatureState::Disabled => out.push(off(b"dlig")),
    }

    match lig.historical {
        LigatureState::Normal => {}
        LigatureState::Enabled => out.push(on(b"hlig")),
        LigatureState::Disabled => out.push(off(b"hlig")),
    }

    match lig.contextual {
        LigatureState::Normal => {}
        LigatureState::Enabled => out.push(on(b"calt")),
        LigatureState::Disabled => out.push(off(b"calt")),
    }
}

// ── font-variant-caps ───────────────────────────────────────────────────

/// Map `font-variant-caps` to OpenType features.
///
/// Blink: `FontFeatures::Initialize()` in `font_features.cc`, caps section.
///
/// - `small-caps` → `"smcp"`
/// - `all-small-caps` → `"smcp"` + `"c2sc"`
/// - `petite-caps` → `"pcap"`
/// - `all-petite-caps` → `"pcap"` + `"c2pc"`
/// - `unicase` → `"unic"`
/// - `titling-caps` → `"titl"`
fn add_caps_features(caps: FontVariantCaps, out: &mut Vec<FontFeature>) {
    match caps {
        FontVariantCaps::Normal => {}
        FontVariantCaps::SmallCaps => {
            out.push(on(b"smcp"));
        }
        FontVariantCaps::AllSmallCaps => {
            out.push(on(b"smcp"));
            out.push(on(b"c2sc"));
        }
        FontVariantCaps::PetiteCaps => {
            out.push(on(b"pcap"));
        }
        FontVariantCaps::AllPetiteCaps => {
            out.push(on(b"pcap"));
            out.push(on(b"c2pc"));
        }
        FontVariantCaps::Unicase => {
            out.push(on(b"unic"));
        }
        FontVariantCaps::TitlingCaps => {
            out.push(on(b"titl"));
        }
    }
}

// ── font-variant-numeric ────────────────────────────────────────────────

/// Map `font-variant-numeric` sub-properties to OpenType features.
///
/// Blink: `FontFeatures::Initialize()` in `font_features.cc`, numeric section.
///
/// - `lining-nums` → `"lnum"`, `oldstyle-nums` → `"onum"`
/// - `proportional-nums` → `"pnum"`, `tabular-nums` → `"tnum"`
/// - `diagonal-fractions` → `"frac"`, `stacked-fractions` → `"afrc"`
/// - `ordinal` → `"ordn"`
/// - `slashed-zero` → `"zero"`
fn add_numeric_features(num: &FontVariantNumeric, out: &mut Vec<FontFeature>) {
    match num.figure {
        NumericFigure::Normal => {}
        NumericFigure::LiningNums => out.push(on(b"lnum")),
        NumericFigure::OldstyleNums => out.push(on(b"onum")),
    }

    match num.spacing {
        NumericSpacing::Normal => {}
        NumericSpacing::ProportionalNums => out.push(on(b"pnum")),
        NumericSpacing::TabularNums => out.push(on(b"tnum")),
    }

    match num.fraction {
        NumericFraction::Normal => {}
        NumericFraction::DiagonalFractions => out.push(on(b"frac")),
        NumericFraction::StackedFractions => out.push(on(b"afrc")),
    }

    if num.ordinal {
        out.push(on(b"ordn"));
    }

    if num.slashed_zero {
        out.push(on(b"zero"));
    }
}

// ── font-variant-east-asian ─────────────────────────────────────────────

/// Map `font-variant-east-asian` sub-properties to OpenType features.
///
/// Blink: `FontFeatures::Initialize()` in `font_features.cc`, East Asian section.
///
/// - `jis78` → `"jp78"`, `jis83` → `"jp83"`, `jis90` → `"jp90"`, `jis04` → `"jp04"`
/// - `simplified` → `"smpl"`, `traditional` → `"trad"`
/// - `full-width` → `"fwid"`, `proportional-width` → `"pwid"`
/// - `ruby` → `"ruby"`
fn add_east_asian_features(ea: &FontVariantEastAsian, out: &mut Vec<FontFeature>) {
    match ea.form {
        EastAsianForm::Normal => {}
        EastAsianForm::Jis78 => out.push(on(b"jp78")),
        EastAsianForm::Jis83 => out.push(on(b"jp83")),
        EastAsianForm::Jis90 => out.push(on(b"jp90")),
        EastAsianForm::Jis04 => out.push(on(b"jp04")),
        EastAsianForm::Simplified => out.push(on(b"smpl")),
        EastAsianForm::Traditional => out.push(on(b"trad")),
    }

    match ea.width {
        EastAsianWidth::Normal => {}
        EastAsianWidth::FullWidth => out.push(on(b"fwid")),
        EastAsianWidth::ProportionalWidth => out.push(on(b"pwid")),
    }

    if ea.ruby {
        out.push(on(b"ruby"));
    }
}

// ── font-variant-position ───────────────────────────────────────────────

/// Map `font-variant-position` to OpenType features.
///
/// Blink: `FontFeatures::Initialize()` in `font_features.cc`, position section.
///
/// - `sub` → `"subs"`
/// - `super` → `"sups"`
fn add_position_features(pos: FontVariantPosition, out: &mut Vec<FontFeature>) {
    match pos {
        FontVariantPosition::Normal => {}
        FontVariantPosition::Sub => out.push(on(b"subs")),
        FontVariantPosition::Super => out.push(on(b"sups")),
    }
}

// ── font-variant-alternates ─────────────────────────────────────────────

/// Map `font-variant-alternates` to OpenType features.
///
/// Only the keyword `historical-forms` is supported (function values like
/// `stylistic()` require `@font-feature-values` which we don't implement).
///
/// - `historical-forms` → `"hist"`
fn add_alternates_features(alt: FontVariantAlternates, out: &mut Vec<FontFeature>) {
    match alt {
        FontVariantAlternates::Normal => {}
        FontVariantAlternates::HistoricalForms => out.push(on(b"hist")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a default description and collect features.
    fn features_for(mutate: impl FnOnce(&mut FontDescription)) -> Vec<FontFeature> {
        let mut desc = FontDescription::new();
        mutate(&mut desc);
        collect_font_features(&desc)
    }

    /// Helper: check that the collected features contain a specific tag+value.
    fn has(features: &[FontFeature], tag: &[u8; 4], value: u32) -> bool {
        features.iter().any(|f| &f.tag == tag && f.value == value)
    }

    /// Helper: check that NO feature with the given tag is present.
    fn lacks(features: &[FontFeature], tag: &[u8; 4]) -> bool {
        features.iter().all(|f| &f.tag != tag)
    }

    // ── Default / Normal ────────────────────────────────────────────

    #[test]
    fn default_description_produces_no_features() {
        let features = features_for(|_| {});
        assert!(features.is_empty());
    }

    // ── font-variant-ligatures ──────────────────────────────────────

    #[test]
    fn ligatures_common_enabled() {
        let f = features_for(|d| d.variant_ligatures.common = LigatureState::Enabled);
        assert!(has(&f, b"liga", 1));
        assert!(has(&f, b"clig", 1));
    }

    #[test]
    fn ligatures_common_disabled() {
        let f = features_for(|d| d.variant_ligatures.common = LigatureState::Disabled);
        assert!(has(&f, b"liga", 0));
        assert!(has(&f, b"clig", 0));
    }

    #[test]
    fn ligatures_discretionary_enabled() {
        let f = features_for(|d| d.variant_ligatures.discretionary = LigatureState::Enabled);
        assert!(has(&f, b"dlig", 1));
    }

    #[test]
    fn ligatures_discretionary_disabled() {
        let f = features_for(|d| d.variant_ligatures.discretionary = LigatureState::Disabled);
        assert!(has(&f, b"dlig", 0));
    }

    #[test]
    fn ligatures_historical_enabled() {
        let f = features_for(|d| d.variant_ligatures.historical = LigatureState::Enabled);
        assert!(has(&f, b"hlig", 1));
    }

    #[test]
    fn ligatures_historical_disabled() {
        let f = features_for(|d| d.variant_ligatures.historical = LigatureState::Disabled);
        assert!(has(&f, b"hlig", 0));
    }

    #[test]
    fn ligatures_contextual_enabled() {
        let f = features_for(|d| d.variant_ligatures.contextual = LigatureState::Enabled);
        assert!(has(&f, b"calt", 1));
    }

    #[test]
    fn ligatures_contextual_disabled() {
        let f = features_for(|d| d.variant_ligatures.contextual = LigatureState::Disabled);
        assert!(has(&f, b"calt", 0));
    }

    #[test]
    fn ligatures_none_disables_all() {
        let f = features_for(|d| d.variant_ligatures = FontVariantLigatures::none());
        assert!(has(&f, b"liga", 0));
        assert!(has(&f, b"clig", 0));
        assert!(has(&f, b"dlig", 0));
        assert!(has(&f, b"hlig", 0));
        assert!(has(&f, b"calt", 0));
    }

    #[test]
    fn ligatures_normal_no_features() {
        let f = features_for(|d| d.variant_ligatures = FontVariantLigatures::NORMAL);
        assert!(lacks(&f, b"liga"));
        assert!(lacks(&f, b"clig"));
        assert!(lacks(&f, b"dlig"));
        assert!(lacks(&f, b"hlig"));
        assert!(lacks(&f, b"calt"));
    }

    // ── font-variant-caps ───────────────────────────────────────────

    #[test]
    fn caps_small_caps() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::SmallCaps);
        assert!(has(&f, b"smcp", 1));
        assert!(lacks(&f, b"c2sc"));
    }

    #[test]
    fn caps_all_small_caps() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::AllSmallCaps);
        assert!(has(&f, b"smcp", 1));
        assert!(has(&f, b"c2sc", 1));
    }

    #[test]
    fn caps_petite_caps() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::PetiteCaps);
        assert!(has(&f, b"pcap", 1));
        assert!(lacks(&f, b"c2pc"));
    }

    #[test]
    fn caps_all_petite_caps() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::AllPetiteCaps);
        assert!(has(&f, b"pcap", 1));
        assert!(has(&f, b"c2pc", 1));
    }

    #[test]
    fn caps_unicase() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::Unicase);
        assert!(has(&f, b"unic", 1));
    }

    #[test]
    fn caps_titling() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::TitlingCaps);
        assert!(has(&f, b"titl", 1));
    }

    #[test]
    fn caps_normal_no_features() {
        let f = features_for(|d| d.variant_caps = FontVariantCaps::Normal);
        assert!(lacks(&f, b"smcp"));
        assert!(lacks(&f, b"c2sc"));
        assert!(lacks(&f, b"pcap"));
        assert!(lacks(&f, b"c2pc"));
        assert!(lacks(&f, b"unic"));
        assert!(lacks(&f, b"titl"));
    }

    // ── font-variant-numeric ────────────────────────────────────────

    #[test]
    fn numeric_lining_nums() {
        let f = features_for(|d| d.variant_numeric.figure = NumericFigure::LiningNums);
        assert!(has(&f, b"lnum", 1));
    }

    #[test]
    fn numeric_oldstyle_nums() {
        let f = features_for(|d| d.variant_numeric.figure = NumericFigure::OldstyleNums);
        assert!(has(&f, b"onum", 1));
    }

    #[test]
    fn numeric_proportional_nums() {
        let f = features_for(|d| d.variant_numeric.spacing = NumericSpacing::ProportionalNums);
        assert!(has(&f, b"pnum", 1));
    }

    #[test]
    fn numeric_tabular_nums() {
        let f = features_for(|d| d.variant_numeric.spacing = NumericSpacing::TabularNums);
        assert!(has(&f, b"tnum", 1));
    }

    #[test]
    fn numeric_diagonal_fractions() {
        let f = features_for(|d| d.variant_numeric.fraction = NumericFraction::DiagonalFractions);
        assert!(has(&f, b"frac", 1));
    }

    #[test]
    fn numeric_stacked_fractions() {
        let f = features_for(|d| d.variant_numeric.fraction = NumericFraction::StackedFractions);
        assert!(has(&f, b"afrc", 1));
    }

    #[test]
    fn numeric_ordinal() {
        let f = features_for(|d| d.variant_numeric.ordinal = true);
        assert!(has(&f, b"ordn", 1));
    }

    #[test]
    fn numeric_slashed_zero() {
        let f = features_for(|d| d.variant_numeric.slashed_zero = true);
        assert!(has(&f, b"zero", 1));
    }

    #[test]
    fn numeric_normal_no_features() {
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

    #[test]
    fn numeric_all_at_once() {
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

    // ── font-variant-east-asian ─────────────────────────────────────

    #[test]
    fn east_asian_jis78() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis78);
        assert!(has(&f, b"jp78", 1));
    }

    #[test]
    fn east_asian_jis83() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis83);
        assert!(has(&f, b"jp83", 1));
    }

    #[test]
    fn east_asian_jis90() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis90);
        assert!(has(&f, b"jp90", 1));
    }

    #[test]
    fn east_asian_jis04() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Jis04);
        assert!(has(&f, b"jp04", 1));
    }

    #[test]
    fn east_asian_simplified() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Simplified);
        assert!(has(&f, b"smpl", 1));
    }

    #[test]
    fn east_asian_traditional() {
        let f = features_for(|d| d.variant_east_asian.form = EastAsianForm::Traditional);
        assert!(has(&f, b"trad", 1));
    }

    #[test]
    fn east_asian_full_width() {
        let f = features_for(|d| d.variant_east_asian.width = EastAsianWidth::FullWidth);
        assert!(has(&f, b"fwid", 1));
    }

    #[test]
    fn east_asian_proportional_width() {
        let f = features_for(|d| d.variant_east_asian.width = EastAsianWidth::ProportionalWidth);
        assert!(has(&f, b"pwid", 1));
    }

    #[test]
    fn east_asian_ruby() {
        let f = features_for(|d| d.variant_east_asian.ruby = true);
        assert!(has(&f, b"ruby", 1));
    }

    #[test]
    fn east_asian_normal_no_features() {
        let f = features_for(|d| d.variant_east_asian = FontVariantEastAsian::NORMAL);
        assert!(lacks(&f, b"jp78"));
        assert!(lacks(&f, b"jp83"));
        assert!(lacks(&f, b"jp90"));
        assert!(lacks(&f, b"jp04"));
        assert!(lacks(&f, b"smpl"));
        assert!(lacks(&f, b"trad"));
        assert!(lacks(&f, b"fwid"));
        assert!(lacks(&f, b"pwid"));
        assert!(lacks(&f, b"ruby"));
    }

    #[test]
    fn east_asian_combined_form_width_ruby() {
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

    // ── font-variant-position ───────────────────────────────────────

    #[test]
    fn position_sub() {
        let f = features_for(|d| d.variant_position = FontVariantPosition::Sub);
        assert!(has(&f, b"subs", 1));
    }

    #[test]
    fn position_super() {
        let f = features_for(|d| d.variant_position = FontVariantPosition::Super);
        assert!(has(&f, b"sups", 1));
    }

    #[test]
    fn position_normal_no_features() {
        let f = features_for(|d| d.variant_position = FontVariantPosition::Normal);
        assert!(lacks(&f, b"subs"));
        assert!(lacks(&f, b"sups"));
    }

    // ── font-variant-alternates ─────────────────────────────────────

    #[test]
    fn alternates_historical_forms() {
        let f = features_for(|d| d.variant_alternates = FontVariantAlternates::HistoricalForms);
        assert!(has(&f, b"hist", 1));
    }

    #[test]
    fn alternates_normal_no_features() {
        let f = features_for(|d| d.variant_alternates = FontVariantAlternates::Normal);
        assert!(lacks(&f, b"hist"));
    }

    // ── Explicit font-feature-settings override ─────────────────────

    #[test]
    fn explicit_feature_settings_appended() {
        let f = features_for(|d| {
            d.feature_settings.push(FontFeature { tag: *b"kern", value: 0 });
        });
        assert!(has(&f, b"kern", 0));
    }

    #[test]
    fn explicit_settings_override_variant() {
        // Variant enables "smcp", explicit settings disable it.
        let f = features_for(|d| {
            d.variant_caps = FontVariantCaps::SmallCaps;
            d.feature_settings.push(FontFeature { tag: *b"smcp", value: 0 });
        });
        // Both should be present; the explicit one comes last (HarfBuzz uses last-wins).
        let smcp_positions: Vec<_> = f.iter()
            .enumerate()
            .filter(|(_, feat)| &feat.tag == b"smcp")
            .collect();
        assert_eq!(smcp_positions.len(), 2);
        // First is from variant (value=1), second is from explicit (value=0).
        assert_eq!(smcp_positions[0].1.value, 1);
        assert_eq!(smcp_positions[1].1.value, 0);
        // Explicit comes after variant.
        assert!(smcp_positions[1].0 > smcp_positions[0].0);
    }

    // ── Combinations ────────────────────────────────────────────────

    #[test]
    fn multiple_variants_combined() {
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
    fn all_variants_active_simultaneously() {
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
        // Caps: smcp, c2sc = 2
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
    fn feature_order_is_deterministic() {
        // Features should always appear in the same order: ligatures, caps,
        // numeric, east-asian, position, alternates, then explicit.
        let f = features_for(|d| {
            d.variant_caps = FontVariantCaps::SmallCaps;
            d.variant_ligatures.common = LigatureState::Disabled;
            d.feature_settings.push(FontFeature { tag: *b"kern", value: 1 });
        });
        let tags: Vec<[u8; 4]> = f.iter().map(|feat| feat.tag).collect();
        // liga off, clig off (ligatures) → smcp (caps) → kern (explicit)
        assert_eq!(tags, vec![*b"liga", *b"clig", *b"smcp", *b"kern"]);
    }

    #[test]
    fn mixed_ligature_states() {
        // Some enabled, some disabled, some normal.
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

    #[test]
    fn multiple_explicit_features() {
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

    // ── Default trait impls ─────────────────────────────────────────

    #[test]
    fn ligatures_default_is_normal() {
        assert_eq!(FontVariantLigatures::default(), FontVariantLigatures::NORMAL);
    }

    #[test]
    fn numeric_default_is_normal() {
        assert_eq!(FontVariantNumeric::default(), FontVariantNumeric::NORMAL);
    }

    #[test]
    fn east_asian_default_is_normal() {
        assert_eq!(FontVariantEastAsian::default(), FontVariantEastAsian::NORMAL);
    }

    #[test]
    fn position_default_is_normal() {
        assert_eq!(FontVariantPosition::default(), FontVariantPosition::Normal);
    }

    #[test]
    fn alternates_default_is_normal() {
        assert_eq!(FontVariantAlternates::default(), FontVariantAlternates::Normal);
    }

    #[test]
    fn caps_default_is_normal() {
        assert_eq!(FontVariantCaps::default(), FontVariantCaps::Normal);
    }
}
