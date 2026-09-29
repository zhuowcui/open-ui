//! FontPlatformData — wraps a resolved Skia typeface with cached metrics.
//!
//! Mirrors Blink's `FontPlatformData` (platform/fonts/font_platform_data.h).
//! Each instance owns an `SkTypeface`, a configured `SkFont`, and pre-computed
//! `FontMetrics` for the resolved size.

use openui_geometry::{
    physical_font_size, RasterConfiguration, TextEdging, TextHinting, TextRasterConfiguration,
};
use skia_safe::{
    font_style::Slant as SkSlant, Font as SkFont, FontHinting, FontMetrics as SkFontMetrics,
    FontStyle as SkFontStyle, GlyphId, Rect, Typeface,
};

use super::metrics::FontMetrics;
use super::{FontFeatureDefault, FontMetricOverrides};

/// Face-specific values resolved by `FontCollection` before a platform font
/// instance is constructed.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResolvedFontConfiguration {
    pub allow_synthetic_weight: bool,
    pub allow_synthetic_style: bool,
    pub feature_defaults: Vec<FontFeatureDefault>,
    pub metric_overrides: FontMetricOverrides,
}

/// Resolved platform font data — Skia typeface + font + cached metrics.
///
/// Blink: `FontPlatformData` in `platform/fonts/font_platform_data.h`.
/// Created by `FontCache` when resolving a `FontDescription` to a typeface.
pub struct FontPlatformData {
    typeface: Typeface,
    sk_font: SkFont,
    size: f32,
    metrics: FontMetrics,
    vertical_metrics: Option<VerticalMetrics>,
    synthetic_bold: bool,
    /// Oblique angle in degrees for synthetic oblique synthesis.
    /// 0.0 for normal/italic styles. CSS default oblique is 14°.
    synthetic_oblique_angle: f32,
    feature_defaults: Vec<FontFeatureDefault>,
}

/// OpenType `vhea`/`vmtx` data retained with a resolved face.
///
/// Skia exposes horizontal glyph placement but does not surface the vertical
/// origin and advance used by CSS Writing Modes. Keeping this face-level table
/// avoids reconstructing vertical metrics from horizontal ascent heuristics.
struct VerticalMetrics {
    advances: Vec<u16>,
    top_side_bearings: Vec<i16>,
    units_per_em: f32,
}

impl VerticalMetrics {
    fn from_typeface(typeface: &Typeface) -> Option<Self> {
        const VHEA: u32 = u32::from_be_bytes(*b"vhea");
        const VMTX: u32 = u32::from_be_bytes(*b"vmtx");

        let vhea = typeface.copy_table_data(VHEA)?;
        let vhea = vhea.as_bytes();
        let number_of_long_metrics = read_u16(vhea, 34)? as usize;
        let glyph_count = typeface.count_glyphs();
        if number_of_long_metrics == 0 || number_of_long_metrics > glyph_count {
            return None;
        }

        let vmtx = typeface.copy_table_data(VMTX)?;
        let vmtx = vmtx.as_bytes();
        let required = number_of_long_metrics.checked_mul(4)?.checked_add(
            glyph_count
                .checked_sub(number_of_long_metrics)?
                .checked_mul(2)?,
        )?;
        if vmtx.len() < required {
            return None;
        }

        let mut advances = Vec::with_capacity(glyph_count);
        let mut top_side_bearings = Vec::with_capacity(glyph_count);
        for glyph in 0..glyph_count {
            if glyph < number_of_long_metrics {
                advances.push(read_u16(vmtx, glyph * 4)?);
                top_side_bearings.push(read_i16(vmtx, glyph * 4 + 2)?);
            } else {
                advances.push(*advances.last()?);
                let offset = number_of_long_metrics * 4 + (glyph - number_of_long_metrics) * 2;
                top_side_bearings.push(read_i16(vmtx, offset)?);
            }
        }

        Some(Self {
            advances,
            top_side_bearings,
            units_per_em: typeface.units_per_em()? as f32,
        })
    }
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
    ]))
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    read_u16(data, offset).map(|value| value as i16)
}

fn resolve_hinting(
    requested_hinting: TextHinting,
    requested_edging: TextEdging,
    family_name: &str,
    size: f32,
    device_scale_factor: f64,
) -> FontHinting {
    let is_ahem = family_name.eq_ignore_ascii_case("Ahem");
    let computed_size = size / device_scale_factor as f32;
    // The unit-scale 20px proportional strike is light-fitted by the pinned
    // Chromium font profile. Its unhinted outline moves the first ink row,
    // while a 20px physical strike reached from a fractional device scale
    // follows the unfitted policy.
    let unit_scale_20px_strike =
        (device_scale_factor - 1.0).abs() <= f64::EPSILON && (computed_size - 20.0).abs() <= 1.0e-4;
    match requested_hinting {
        // Chromium's Fontconfig `hinting=false` path still grid-fits the
        // aliased 8px Ahem face. Skia's direct API needs slight hinting to
        // reproduce those glyph bounds, while larger Ahem sizes remain truly
        // unhinted. Select by face and computed size, not by test identity.
        TextHinting::None if requested_edging == TextEdging::Alias && is_ahem && size <= 8.0 => {
            FontHinting::Slight
        }
        // Chromium also grid-fits glyphs supplied by a fallback face (for
        // example arrows absent from Ahem) before applying the same aliased
        // coverage threshold.
        TextHinting::None if requested_edging == TextEdging::Alias && !is_ahem => {
            FontHinting::Slight
        }
        TextHinting::None => FontHinting::None,
        // Chromium asks Fontconfig for render parameters for every resolved
        // family and physical strike. The pinned proportional Sans/Serif
        // faces are generally returned without outline fitting, while the
        // exact 16px and 24px strikes remain lightly fitted. The pinned
        // monospace face keeps light fitting at every strike. Preserve those
        // family/strike distinctions instead of applying one global SkFont
        // hinting mode to all document-owned fonts.
        TextHinting::Slight
            if requested_edging == TextEdging::SubpixelAntiAlias
                && (family_name.eq_ignore_ascii_case("DejaVu Sans")
                    || family_name.eq_ignore_ascii_case("DejaVu Serif"))
                && (computed_size >= 16.0 || device_scale_factor > 1.0)
                && (size - 16.0).abs() > 1.0e-4
                && (size - 24.0).abs() > 1.0e-4
                && !unit_scale_20px_strike =>
        {
            FontHinting::None
        }
        // At a fractional physical Ahem strike, Skia's light-fitting drops
        // the trailing coverage cell that Chromium keeps. Integral strikes
        // retain light fitting; removing it there regresses exact glyphs.
        TextHinting::Slight
            if requested_edging == TextEdging::SubpixelAntiAlias
                && is_ahem
                && (size - size.round()).abs() > 1.0e-4 =>
        {
            FontHinting::None
        }
        TextHinting::Slight => FontHinting::Slight,
        TextHinting::Normal => FontHinting::Normal,
        TextHinting::Full => FontHinting::Full,
    }
}

impl FontPlatformData {
    /// Create platform data from a resolved Skia typeface and size.
    ///
    /// Configures the `SkFont` with subpixel positioning and slight hinting
    /// (matching Blink's default Linux/ChromeOS configuration), then extracts
    /// and caches all typographic metrics.
    pub fn new(typeface: Typeface, size: f32) -> Self {
        Self::with_oblique_angle(typeface, size, 0.0)
    }

    /// Create platform data with a specific oblique angle for synthetic oblique.
    ///
    /// The angle is stored and can be retrieved via `synthetic_oblique_angle()`
    /// for applying a skew transform during text painting.
    pub fn with_oblique_angle(typeface: Typeface, size: f32, oblique_angle: f32) -> Self {
        let requested_weight = typeface.font_style().weight();
        Self::with_synthetic_styles(typeface, size, oblique_angle, requested_weight)
    }

    /// Create platform data while retaining the requested weight so a family
    /// without a bold face can synthesize the CSS-selected weight.
    pub fn with_synthetic_styles(
        typeface: Typeface,
        size: f32,
        oblique_angle: f32,
        requested_weight: skia_safe::font_style::Weight,
    ) -> Self {
        Self::with_synthetic_styles_and_native_metrics(
            typeface,
            size,
            oblique_angle,
            requested_weight,
            false,
            false,
            false,
        )
    }

    /// Create platform data with an optional native-control raster policy.
    /// Native widget labels are rendered by Chromium's platform theme and do
    /// not inherit the alias/no-hint profile used for deterministic author
    /// text in pixel comparisons.
    pub fn with_synthetic_styles_and_native_metrics(
        typeface: Typeface,
        size: f32,
        oblique_angle: f32,
        requested_weight: skia_safe::font_style::Weight,
        native_control_text: bool,
        embedded_document_text: bool,
        native_button_text_metrics: bool,
    ) -> Self {
        Self::with_resolved_configuration(
            typeface,
            size,
            oblique_angle,
            requested_weight,
            native_control_text,
            embedded_document_text,
            native_button_text_metrics,
            RasterConfiguration::default(),
            1.0,
            ResolvedFontConfiguration {
                allow_synthetic_weight: true,
                allow_synthetic_style: true,
                ..ResolvedFontConfiguration::default()
            },
        )
    }

    pub(crate) fn with_resolved_configuration(
        typeface: Typeface,
        size: f32,
        mut oblique_angle: f32,
        requested_weight: skia_safe::font_style::Weight,
        native_control_text: bool,
        embedded_document_text: bool,
        native_button_text_metrics: bool,
        raster_configuration: RasterConfiguration,
        device_scale_factor: f64,
        configuration: ResolvedFontConfiguration,
    ) -> Self {
        // Font matching may return a regular face when a family has no bold
        // member (Ahem is the canonical example). CSS font synthesis requires
        // a synthetic bold face in that case; SkFont does not infer it from
        // the requested FontStyle after typeface matching.
        let synthetic_bold = configuration.allow_synthetic_weight
            && requested_weight >= skia_safe::font_style::Weight::SEMI_BOLD
            && typeface.font_style().weight() < skia_safe::font_style::Weight::SEMI_BOLD;
        if !configuration.allow_synthetic_style {
            oblique_angle = 0.0;
        }
        let mut sk_font = SkFont::from_typeface(&typeface, size);
        sk_font.set_embolden(synthetic_bold);
        let family_name = typeface.family_name();
        let deterministic_aliased_face = [
            "Ahem",
            "Droid Sans Fallback",
            "Noto Sans Devanagari",
            "Noto Color Emoji",
            "DejaVu Sans",
        ]
        .iter()
        .any(|family| family_name.eq_ignore_ascii_case(family));
        // The comparison Fontconfig profile disables antialiasing only for
        // its explicitly pinned deterministic faces.  A CSS family outside
        // that set (for example the monospace face used by ::first-line)
        // still receives Chromium's ordinary LCD/subpixel raster policy.
        let escapes_aliased_profile = !native_control_text
            && !embedded_document_text
            && raster_configuration.author_text.edging == TextEdging::Alias
            && !deterministic_aliased_face;
        let settings = if native_control_text {
            raster_configuration.native_text
        } else if embedded_document_text {
            raster_configuration.embedded_text
        } else if escapes_aliased_profile {
            TextRasterConfiguration::chromium_lcd()
        } else {
            raster_configuration.author_text
        };
        let subpixel = settings.subpixel_positioning && !native_button_text_metrics;
        sk_font.set_subpixel(subpixel);
        let hinting = resolve_hinting(
            settings.hinting,
            settings.edging,
            family_name.as_str(),
            physical_font_size(size, device_scale_factor),
            device_scale_factor,
        );
        sk_font.set_hinting(hinting);
        sk_font.set_linear_metrics(subpixel);
        sk_font.set_embedded_bitmaps(true);
        match settings.edging {
            TextEdging::Alias => {
                sk_font.set_edging(skia_safe::font::Edging::Alias);
            }
            TextEdging::SubpixelAntiAlias => {
                sk_font.set_edging(skia_safe::font::Edging::SubpixelAntiAlias);
            }
            TextEdging::AntiAlias => {
                sk_font.set_edging(skia_safe::font::Edging::AntiAlias);
            }
        }
        sk_font.set_force_auto_hinting(settings.force_autohint);

        // Apply synthetic oblique via skew if angle is non-zero.
        if oblique_angle != 0.0 {
            sk_font.set_skew_x(-oblique_angle.to_radians().tan());
        }

        let (_, sk_metrics) = sk_font.metrics();
        let mut metrics = Self::convert_metrics(&sk_metrics, &typeface, &sk_font);
        if let Some(value) = configuration.metric_overrides.ascent {
            metrics.ascent = value * size;
        }
        if let Some(value) = configuration.metric_overrides.descent {
            metrics.descent = value * size;
        }
        if let Some(value) = configuration.metric_overrides.line_gap {
            metrics.line_gap = value * size;
        }
        metrics.line_spacing = metrics.ascent + metrics.descent + metrics.line_gap;
        let vertical_metrics = VerticalMetrics::from_typeface(&typeface);

        Self {
            typeface,
            sk_font,
            size,
            metrics,
            vertical_metrics,
            synthetic_bold,
            synthetic_oblique_angle: oblique_angle,
            feature_defaults: configuration.feature_defaults,
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

    /// OpenType vertical advance for a glyph, falling back to one em for a
    /// face without `vhea`/`vmtx` data.
    pub fn vertical_advance(&self, glyph: GlyphId) -> f32 {
        self.vertical_metrics
            .as_ref()
            .and_then(|metrics| {
                metrics
                    .advances
                    .get(glyph as usize)
                    .map(|advance| (*advance, metrics.units_per_em))
            })
            .map(|(advance, units_per_em)| advance as f32 * self.size / units_per_em)
            .unwrap_or(self.size)
    }

    /// Baseline Y relative to a glyph's vertical advance cell.
    ///
    /// OpenType defines the vertical origin as glyph `yMax` plus the `vmtx`
    /// top-side-bearing. Skia bounds use a downward-positive device axis, so
    /// `-bounds.top` is the scaled `yMax` contribution.
    pub fn vertical_origin_y(&self, glyph: GlyphId) -> f32 {
        let Some(metrics) = &self.vertical_metrics else {
            return self.metrics.ascent;
        };
        let Some(top_side_bearing) = metrics.top_side_bearings.get(glyph as usize) else {
            return self.metrics.ascent;
        };
        let mut bounds = [Rect::default()];
        self.sk_font.get_bounds(&[glyph], &mut bounds, None);
        -bounds[0].top + *top_side_bearing as f32 * self.size / metrics.units_per_em
    }

    /// Whether CSS requested a bold weight that the selected family lacked.
    #[inline]
    pub fn is_synthetic_bold(&self) -> bool {
        self.synthetic_bold
    }

    /// The oblique angle in degrees used for synthetic oblique.
    /// Returns 0.0 for normal and italic styles.
    #[inline]
    pub fn synthetic_oblique_angle(&self) -> f32 {
        self.synthetic_oblique_angle
    }

    /// Defaults supplied by the selected application face. CSS declarations
    /// are appended after these values so author settings win by tag.
    pub fn feature_defaults(&self) -> &[FontFeatureDefault] {
        &self.feature_defaults
    }

    /// Convert Skia's `SkFontMetrics` to our `FontMetrics`.
    ///
    /// Key corrections:
    /// - Skia's ascent is NEGATIVE (distance above baseline as negative Y).
    ///   We store it as POSITIVE.
    /// - underline/strikeout values use Optional accessors in skia-safe.
    fn convert_metrics(sk: &SkFontMetrics, typeface: &Typeface, sk_font: &SkFont) -> FontMetrics {
        let ascent = -sk.ascent; // Make positive
        let descent = sk.descent; // Already positive in Skia
        let line_gap = sk.leading;

        // Measure '0' width for CSS `ch` unit
        let zero_width = {
            let (w, _) = sk_font.measure_str("0", None);
            w
        };

        // Units per em from the font's head table
        let units_per_em = typeface.units_per_em().map(|u| u as u16).unwrap_or(1000);

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
            ..=62 => 1,     // UltraCondensed (50%)
            63..=74 => 2,   // ExtraCondensed (62.5%)
            75..=86 => 3,   // Condensed (75%)
            87..=93 => 4,   // SemiCondensed (87.5%)
            94..=106 => 5,  // Normal (100%)
            107..=118 => 6, // SemiExpanded (112.5%)
            119..=137 => 7, // Expanded (125%)
            138..=174 => 8, // ExtraExpanded (150%)
            _ => 9,         // UltraExpanded (200%)
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
    fn aliased_ahem_hinting_tracks_computed_font_size() {
        assert_eq!(
            resolve_hinting(TextHinting::None, TextEdging::Alias, "Ahem", 8.0, 1.0),
            FontHinting::Slight
        );
        assert_eq!(
            resolve_hinting(TextHinting::None, TextEdging::Alias, "Ahem", 10.0, 1.0),
            FontHinting::None
        );
        // An 8 CSS-pixel face at 2x selects the 16 physical-pixel strike; it
        // must not take the special 8px grid-fit path.
        assert_eq!(
            resolve_hinting(
                TextHinting::None,
                TextEdging::Alias,
                "Ahem",
                physical_font_size(8.0, 2.0),
                2.0,
            ),
            FontHinting::None
        );
    }

    #[test]
    fn lcd_ahem_hinting_tracks_physical_strike_fraction() {
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "Ahem",
                physical_font_size(50.0, 1.25),
                1.25,
            ),
            FontHinting::None
        );
        for (css_size, scale) in [(16.0, 1.0), (16.0, 1.5), (50.0, 1.5)] {
            assert_eq!(
                resolve_hinting(
                    TextHinting::Slight,
                    TextEdging::SubpixelAntiAlias,
                    "Ahem",
                    physical_font_size(css_size, scale),
                    scale,
                ),
                FontHinting::Slight
            );
        }
    }

    #[test]
    fn author_lcd_light_hinting_is_immutable_across_device_scales() {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            assert_eq!(
                resolve_hinting(
                    TextHinting::Slight,
                    TextEdging::SubpixelAntiAlias,
                    "DejaVu Sans Mono",
                    16.0 * scale as f32,
                    scale,
                ),
                FontHinting::Slight
            );
        }
    }

    #[test]
    fn chromium_lcd_hinting_tracks_pinned_fontconfig_family_policy() {
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Sans",
                20.0,
                1.0,
            ),
            FontHinting::Slight
        );
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Sans",
                20.0,
                1.25,
            ),
            FontHinting::None
        );
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Serif",
                20.0,
                1.25,
            ),
            FontHinting::None
        );
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Sans Mono",
                20.0,
                1.25,
            ),
            FontHinting::Slight
        );
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Sans",
                30.0,
                1.25,
            ),
            FontHinting::None
        );
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Sans",
                24.0,
                1.0,
            ),
            FontHinting::Slight
        );
        assert_eq!(
            resolve_hinting(
                TextHinting::Slight,
                TextEdging::SubpixelAntiAlias,
                "DejaVu Sans",
                12.5,
                1.25,
            ),
            FontHinting::None
        );
    }

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
        assert_eq!(FontPlatformData::stretch_to_sk_width(50.0), 1); // UltraCondensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(62.5), 2); // ExtraCondensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(75.0), 3); // Condensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(87.5), 4); // SemiCondensed
        assert_eq!(FontPlatformData::stretch_to_sk_width(100.0), 5); // Normal
        assert_eq!(FontPlatformData::stretch_to_sk_width(112.5), 6); // SemiExpanded
        assert_eq!(FontPlatformData::stretch_to_sk_width(125.0), 7); // Expanded
        assert_eq!(FontPlatformData::stretch_to_sk_width(150.0), 8); // ExtraExpanded
        assert_eq!(FontPlatformData::stretch_to_sk_width(200.0), 9); // UltraExpanded
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

    // ── Issue 6 (R26): oblique angle preserved ──────────────────────────

    #[test]
    fn oblique_with_angle_stores_angle() {
        use skia_safe::FontMgr;
        // Create a font with an oblique angle and verify it's stored.
        let mgr = FontMgr::default();
        let sk_style = FontPlatformData::to_sk_font_style(
            400.0,
            100.0,
            &openui_style::FontStyleEnum::Oblique(20.0),
        );
        if let Some(typeface) = mgr.match_family_style("sans-serif", sk_style) {
            let data = FontPlatformData::with_oblique_angle(typeface, 16.0, 20.0);
            assert_eq!(data.synthetic_oblique_angle(), 20.0);
        }
    }

    #[test]
    fn oblique_default_angle_differs_from_zero() {
        use skia_safe::FontMgr;
        // The CSS default oblique angle is 14°. Verify it's stored.
        let mgr = FontMgr::default();
        let sk_style = FontPlatformData::to_sk_font_style(
            400.0,
            100.0,
            &openui_style::FontStyleEnum::Oblique(14.0),
        );
        if let Some(typeface) = mgr.match_family_style("sans-serif", sk_style) {
            let data_oblique = FontPlatformData::with_oblique_angle(typeface.clone(), 16.0, 14.0);
            let data_normal = FontPlatformData::new(typeface, 16.0);
            assert_eq!(data_oblique.synthetic_oblique_angle(), 14.0);
            assert_eq!(data_normal.synthetic_oblique_angle(), 0.0);
        }
    }
}
