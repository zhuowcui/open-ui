//! FontPlatformData — wraps a resolved Skia typeface with cached metrics.
//!
//! Mirrors Blink's `FontPlatformData` (platform/fonts/font_platform_data.h).
//! Each instance owns an `SkTypeface`, a configured `SkFont`, and pre-computed
//! `FontMetrics` for the resolved size.

use skia_safe::{
    font_style::Slant as SkSlant, Font as SkFont, FontHinting, FontMetrics as SkFontMetrics,
    FontStyle as SkFontStyle, Typeface,
};

use super::metrics::FontMetrics;

/// Resolved platform font data — Skia typeface + font + cached metrics.
///
/// Blink: `FontPlatformData` in `platform/fonts/font_platform_data.h`.
/// Created by `FontCache` when resolving a `FontDescription` to a typeface.
pub struct FontPlatformData {
    typeface: Typeface,
    sk_font: SkFont,
    size: f32,
    metrics: FontMetrics,
}

impl FontPlatformData {
    /// Create platform data from a resolved Skia typeface and size.
    ///
    /// Configures the `SkFont` with subpixel positioning and slight hinting
    /// (matching Blink's default Linux/ChromeOS configuration), then extracts
    /// and caches all typographic metrics.
    pub fn new(typeface: Typeface, size: f32) -> Self {
        let mut sk_font = SkFont::from_typeface(&typeface, size);
        sk_font.set_subpixel(true);
        sk_font.set_hinting(FontHinting::Slight);

        let (_, sk_metrics) = sk_font.metrics();
        let metrics = Self::convert_metrics(&sk_metrics, &typeface, &sk_font);

        Self {
            typeface,
            sk_font,
            size,
            metrics,
        }
    }

    /// The underlying Skia typeface.
    #[inline]
    pub fn typeface(&self) -> &Typeface {
        &self.typeface
    }

    /// The configured Skia font (typeface + size + hinting settings).
    #[inline]
    pub fn sk_font(&self) -> &SkFont {
        &self.sk_font
    }

    /// The resolved font size in CSS pixels.
    #[inline]
    pub fn size(&self) -> f32 {
        self.size
    }

    /// Pre-computed typographic metrics.
    #[inline]
    pub fn metrics(&self) -> &FontMetrics {
        &self.metrics
    }

    /// Convert Skia's `SkFontMetrics` to our `FontMetrics`.
    ///
    /// Key corrections:
    /// - Skia's ascent is NEGATIVE (distance above baseline as negative Y).
    ///   We store it as POSITIVE.
    /// - underline/strikeout values use Optional accessors in skia-safe.
    fn convert_metrics(
        sk: &SkFontMetrics,
        typeface: &Typeface,
        sk_font: &SkFont,
    ) -> FontMetrics {
        let ascent = -sk.ascent; // Make positive
        let descent = sk.descent; // Already positive in Skia
        let line_gap = sk.leading;

        // Measure '0' width for CSS `ch` unit
        let zero_width = {
            let (w, _) = sk_font.measure_str("0", None);
            w
        };

        // Units per em from the font's head table
        let units_per_em = typeface
            .units_per_em()
            .map(|u| u as u16)
            .unwrap_or(1000);

        FontMetrics {
            ascent,
            descent,
            line_gap,
            line_spacing: ascent + descent + line_gap,
            x_height: sk.x_height,
            cap_height: sk.cap_height,
            zero_width,
            underline_offset: sk.underline_position().unwrap_or(ascent * 0.125),
            underline_thickness: sk.underline_thickness().unwrap_or(ascent * 0.05),
            strikeout_position: sk.strikeout_position().unwrap_or(ascent * 0.35),
            strikeout_thickness: sk.strikeout_thickness().unwrap_or(ascent * 0.05),
            overline_offset: ascent,
            units_per_em,
        }
    }

    /// Convert our `FontStyleEnum` + weight + stretch to Skia's `SkFontStyle`.
    pub fn to_sk_font_style(
        weight: f32,
        stretch: f32,
        style: &openui_style::FontStyleEnum,
    ) -> SkFontStyle {
        let sk_weight = weight as i32;
        let sk_width = Self::stretch_to_sk_width(stretch);
        let sk_slant = match style {
            openui_style::FontStyleEnum::Normal => SkSlant::Upright,
            openui_style::FontStyleEnum::Italic => SkSlant::Italic,
            openui_style::FontStyleEnum::Oblique(_) => SkSlant::Oblique,
        };
        SkFontStyle::new(
            skia_safe::font_style::Weight::from(sk_weight),
            skia_safe::font_style::Width::from(sk_width),
            sk_slant,
        )
    }

    /// Convert CSS `font-stretch` percentage to Skia's width scale (1–9).
    /// Mapping based on CSS Fonts spec § 3.3.
    fn stretch_to_sk_width(stretch: f32) -> i32 {
        match stretch.round() as i32 {
            ..=62 => 1,      // UltraCondensed (50%)
            63..=74 => 2,    // ExtraCondensed (62.5%)
            75..=86 => 3,    // Condensed (75%)
            87..=93 => 4,    // SemiCondensed (87.5%)
            94..=106 => 5,   // Normal (100%)
            107..=118 => 6,  // SemiExpanded (112.5%)
            119..=137 => 7,  // Expanded (125%)
            138..=174 => 8,  // ExtraExpanded (150%)
            _ => 9,          // UltraExpanded (200%)
        }
    }
}

impl std::fmt::Debug for FontPlatformData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontPlatformData")
            .field("size", &self.size)
            .field("metrics", &self.metrics)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretch_62_5_maps_to_extra_condensed() {
        // 62.5% is the CSS keyword value for extra-condensed (Skia width 2).
        // Before the fix, `62.5 as i32` truncated to 62 → UltraCondensed (1).
        assert_eq!(
            FontPlatformData::stretch_to_sk_width(62.5),
            2,
            "62.5% should map to ExtraCondensed (2), not UltraCondensed (1)"
        );
    }

    #[test]
    fn stretch_keyword_values_map_correctly() {
        assert_eq!(FontPlatformData::stretch_to_sk_width(50.0), 1);   // UltraCondensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(62.5), 2);   // ExtraCondensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(75.0), 3);   // Condensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(87.5), 4);   // SemiCondensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(100.0), 5);  // Normal
        assert_eq!(FontPlatformData::stretch_to_sk_width(112.5), 6);  // SemiExpanded
        assert_eq!(FontPlatformData::stretch_to_sk_width(125.0), 7);  // Expanded
        assert_eq!(FontPlatformData::stretch_to_sk_width(150.0), 8);  // ExtraExpanded
        assert_eq!(FontPlatformData::stretch_to_sk_width(200.0), 9);  // UltraExpanded
    }

    #[test]
    fn stretch_boundary_values_round_correctly() {
        // 62.4 rounds to 62 → UltraCondensed (1)
        assert_eq!(FontPlatformData::stretch_to_sk_width(62.4), 1);
        // 62.5 rounds to 63 → ExtraCondensed (2)
        assert_eq!(FontPlatformData::stretch_to_sk_width(62.5), 2);
        // 87.4 rounds to 87 → SemiCondensed (4)
        assert_eq!(FontPlatformData::stretch_to_sk_width(87.4), 4);
        // 87.5 rounds to 88 → SemiCondensed (4)
        assert_eq!(FontPlatformData::stretch_to_sk_width(87.5), 4);
    }
}
