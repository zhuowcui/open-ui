//! FontDescription — what CSS wants the font to look like.
//!
//! Mirrors Blink's `FontDescription` (platform/fonts/font_description.h).
//! This is the input to font resolution: it carries all CSS font properties
//! needed to select and configure a typeface.

use openui_style::{
    FontFamilyList, FontOpticalSizing, FontOrientation, FontSmoothing, FontStretch, FontStyleEnum,
    FontSynthesis, FontVariantAlternates, FontVariantCaps, FontVariantEastAsian,
    FontVariantLigatures, FontVariantNumeric, FontVariantPosition, FontWeight, TextRendering,
    FontFeature, FontVariation,
};

/// Complete description of desired font properties, derived from CSS.
///
/// Mirrors Blink's `FontDescription`. Used as the key for font resolution:
/// the `FontCache` maps a `FontDescription` to a resolved `FontPlatformData`.
#[derive(Debug, Clone)]
pub struct FontDescription {
    /// Ordered list of font families (CSS `font-family`).
    pub family: FontFamilyList,
    /// Computed font size in pixels (CSS `font-size`, after cascade/inheritance).
    pub size: f32,
    /// Specified font size before min-font-size clamping.
    pub specified_size: f32,
    /// Font weight: 100–900 (CSS `font-weight`).
    pub weight: FontWeight,
    /// Font stretch as percentage (CSS `font-stretch`).
    pub stretch: FontStretch,
    /// Font style: normal, italic, or oblique (CSS `font-style`).
    pub style: FontStyleEnum,
    /// Small-caps and other variant caps (CSS `font-variant-caps`).
    pub variant_caps: FontVariantCaps,
    /// Ligature control (CSS `font-variant-ligatures`).
    pub variant_ligatures: FontVariantLigatures,
    /// Numeric glyph variants (CSS `font-variant-numeric`).
    pub variant_numeric: FontVariantNumeric,
    /// East Asian glyph variants (CSS `font-variant-east-asian`).
    pub variant_east_asian: FontVariantEastAsian,
    /// Sub/superscript glyph variants (CSS `font-variant-position`).
    pub variant_position: FontVariantPosition,
    /// Alternate glyph forms (CSS `font-variant-alternates`).
    pub variant_alternates: FontVariantAlternates,
    /// Extra spacing between characters in pixels (CSS `letter-spacing`).
    pub letter_spacing: f32,
    /// Extra spacing at word boundaries in pixels (CSS `word-spacing`).
    pub word_spacing: f32,
    /// BCP47 locale for language-specific shaping.
    pub locale: Option<String>,
    /// Font smoothing mode (CSS `-webkit-font-smoothing`).
    pub font_smoothing: FontSmoothing,
    /// Text rendering hint (CSS `text-rendering`).
    pub text_rendering: TextRendering,
    /// OpenType feature settings (CSS `font-feature-settings`).
    pub feature_settings: Vec<FontFeature>,
    /// Variable font variation axes (CSS `font-variation-settings`).
    pub variation_settings: Vec<FontVariation>,
    /// Whether to synthesize bold (CSS `font-synthesis-weight`).
    pub font_synthesis_weight: FontSynthesis,
    /// Whether to synthesize italic (CSS `font-synthesis-style`).
    pub font_synthesis_style: FontSynthesis,
    /// Optical sizing mode (CSS `font-optical-sizing`).
    pub font_optical_sizing: FontOpticalSizing,
    /// Resolved font orientation for vertical text layout.
    ///
    /// Derived from `writing-mode` + `text-orientation`. Controls whether
    /// glyphs are rendered upright, rotated, or in mixed mode. Defaults to
    /// `Horizontal` (standard left-to-right flow).
    pub orientation: FontOrientation,
}

impl FontDescription {
    /// Create a description with CSS initial values.
    pub fn new() -> Self {
        Self {
            family: FontFamilyList::default(),
            size: 16.0,
            specified_size: 16.0,
            weight: FontWeight::NORMAL,
            stretch: FontStretch::NORMAL,
            style: FontStyleEnum::Normal,
            variant_caps: FontVariantCaps::Normal,
            variant_ligatures: FontVariantLigatures::default(),
            variant_numeric: FontVariantNumeric::default(),
            variant_east_asian: FontVariantEastAsian::default(),
            variant_position: FontVariantPosition::Normal,
            variant_alternates: FontVariantAlternates::Normal,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            locale: None,
            font_smoothing: FontSmoothing::Auto,
            text_rendering: TextRendering::Auto,
            feature_settings: Vec::new(),
            variation_settings: Vec::new(),
            font_synthesis_weight: FontSynthesis::Auto,
            font_synthesis_style: FontSynthesis::Auto,
            font_optical_sizing: FontOpticalSizing::Auto,
            orientation: FontOrientation::Horizontal,
        }
    }

    /// Create a description for a specific family and size.
    pub fn with_family_and_size(family: FontFamilyList, size: f32) -> Self {
        let mut desc = Self::new();
        desc.family = family;
        desc.size = size;
        desc.specified_size = size;
        desc
    }
}

impl Default for FontDescription {
    fn default() -> Self {
        Self::new()
    }
}
