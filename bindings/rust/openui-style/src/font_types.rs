//! Font-related types shared between openui-style and openui-text.
//!
//! These types are defined here (in openui-style) because ComputedStyle uses them,
//! and openui-text depends on openui-style — not the other way around.
//!
//! Extracted from Blink's `font_description.h`, `font_family.h`, and
//! `computed_style_constants.h`.

/// CSS `font-weight` — a numeric weight in the range 1–1000.
/// Default is 400 (normal). Bold is 700.
/// Blink: `FontSelectionValue` for weight, stored as float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontWeight(pub f32);

impl FontWeight {
    pub const NORMAL: Self = Self(400.0);
    pub const BOLD: Self = Self(700.0);
    pub const THIN: Self = Self(100.0);
    pub const LIGHT: Self = Self(300.0);
    pub const MEDIUM: Self = Self(500.0);
    pub const SEMI_BOLD: Self = Self(600.0);
    pub const EXTRA_BOLD: Self = Self(800.0);
    pub const BLACK: Self = Self(900.0);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// CSS `font-stretch` — percentage value. Default is 100% (normal).
/// Range: 50% (ultra-condensed) to 200% (ultra-expanded).
/// Blink: `FontSelectionValue` for width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontStretch(pub f32);

impl FontStretch {
    pub const ULTRA_CONDENSED: Self = Self(50.0);
    pub const EXTRA_CONDENSED: Self = Self(62.5);
    pub const CONDENSED: Self = Self(75.0);
    pub const SEMI_CONDENSED: Self = Self(87.5);
    pub const NORMAL: Self = Self(100.0);
    pub const SEMI_EXPANDED: Self = Self(112.5);
    pub const EXPANDED: Self = Self(125.0);
    pub const EXTRA_EXPANDED: Self = Self(150.0);
    pub const ULTRA_EXPANDED: Self = Self(200.0);
}

impl Default for FontStretch {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// CSS `font-style` — normal, italic, or oblique with optional angle.
/// Blink: `FontSelectionValue` for slope + `FontStyle` enum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyleEnum {
    Normal,
    Italic,
    /// Oblique with angle in degrees. CSS default oblique angle is 14°.
    Oblique(f32),
}

impl Default for FontStyleEnum {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-caps` property.
/// Blink: `FontDescription::VariantCaps` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontVariantCaps {
    /// `normal` — no variant caps.
    Normal = 0,
    /// `small-caps` → OpenType `"smcp"`.
    SmallCaps = 1,
    /// `all-small-caps` → OpenType `"smcp"` + `"c2sc"`.
    AllSmallCaps = 2,
    /// `petite-caps` → OpenType `"pcap"`.
    PetiteCaps = 3,
    /// `all-petite-caps` → OpenType `"pcap"` + `"c2pc"`.
    AllPetiteCaps = 4,
    /// `unicase` → OpenType `"unic"`.
    Unicase = 5,
    /// `titling-caps` → OpenType `"titl"`.
    TitlingCaps = 6,
}

impl Default for FontVariantCaps {
    fn default() -> Self {
        Self::Normal
    }
}

// ── font-variant-ligatures ──────────────────────────────────────────────

/// Tri-state for each ligature sub-property.
///
/// `Normal` means "use the font's default behavior" (no explicit feature tag
/// is emitted). `Enabled` forces the feature on (`value = 1`), `Disabled`
/// forces it off (`value = 0`).
///
/// Blink: individual bits inside `FontDescription::font_variant_ligatures_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LigatureState {
    /// Use font default — no feature tag emitted.
    Normal = 0,
    /// Feature explicitly enabled (`value = 1`).
    Enabled = 1,
    /// Feature explicitly disabled (`value = 0`).
    Disabled = 2,
}

impl Default for LigatureState {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-ligatures` — controls OpenType ligature features.
///
/// Each sub-property is independent. The CSS keyword `normal` leaves all
/// four at `LigatureState::Normal`; the keyword `none` sets all four to
/// `LigatureState::Disabled`.
///
/// Blink: `FontDescription` stores four separate 2-bit fields
/// (`common_ligatures_state_`, `discretionary_ligatures_state_`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontVariantLigatures {
    /// `common-ligatures` / `no-common-ligatures` → `"liga"`, `"clig"`.
    pub common: LigatureState,
    /// `discretionary-ligatures` / `no-discretionary-ligatures` → `"dlig"`.
    pub discretionary: LigatureState,
    /// `historical-ligatures` / `no-historical-ligatures` → `"hlig"`.
    pub historical: LigatureState,
    /// `contextual` / `no-contextual` → `"calt"`.
    pub contextual: LigatureState,
}

impl FontVariantLigatures {
    /// CSS `font-variant-ligatures: normal` — all sub-properties at font default.
    pub const NORMAL: Self = Self {
        common: LigatureState::Normal,
        discretionary: LigatureState::Normal,
        historical: LigatureState::Normal,
        contextual: LigatureState::Normal,
    };

    /// CSS `font-variant-ligatures: none` — all ligatures explicitly disabled.
    pub fn none() -> Self {
        Self {
            common: LigatureState::Disabled,
            discretionary: LigatureState::Disabled,
            historical: LigatureState::Disabled,
            contextual: LigatureState::Disabled,
        }
    }
}

impl Default for FontVariantLigatures {
    fn default() -> Self {
        Self::NORMAL
    }
}

// ── font-variant-numeric ────────────────────────────────────────────────

/// CSS `font-variant-numeric` figure sub-property.
///
/// Blink: `FontDescription::numeric_figure_` (2-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NumericFigure {
    /// `normal` — use font default.
    Normal = 0,
    /// `lining-nums` → OpenType `"lnum"`.
    LiningNums = 1,
    /// `oldstyle-nums` → OpenType `"onum"`.
    OldstyleNums = 2,
}

impl Default for NumericFigure {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-numeric` spacing sub-property.
///
/// Blink: `FontDescription::numeric_spacing_` (2-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NumericSpacing {
    /// `normal` — use font default.
    Normal = 0,
    /// `proportional-nums` → OpenType `"pnum"`.
    ProportionalNums = 1,
    /// `tabular-nums` → OpenType `"tnum"`.
    TabularNums = 2,
}

impl Default for NumericSpacing {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-numeric` fraction sub-property.
///
/// Blink: `FontDescription::numeric_fraction_` (2-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NumericFraction {
    /// `normal` — use font default.
    Normal = 0,
    /// `diagonal-fractions` → OpenType `"frac"`.
    DiagonalFractions = 1,
    /// `stacked-fractions` → OpenType `"afrc"`.
    StackedFractions = 2,
}

impl Default for NumericFraction {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-numeric` — controls numeric glyph features.
///
/// Blink: individual 1-2 bit fields in `FontDescription`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontVariantNumeric {
    /// `lining-nums` / `oldstyle-nums`.
    pub figure: NumericFigure,
    /// `proportional-nums` / `tabular-nums`.
    pub spacing: NumericSpacing,
    /// `diagonal-fractions` / `stacked-fractions`.
    pub fraction: NumericFraction,
    /// `ordinal` → OpenType `"ordn"`.
    pub ordinal: bool,
    /// `slashed-zero` → OpenType `"zero"`.
    pub slashed_zero: bool,
}

impl FontVariantNumeric {
    /// CSS `font-variant-numeric: normal`.
    pub const NORMAL: Self = Self {
        figure: NumericFigure::Normal,
        spacing: NumericSpacing::Normal,
        fraction: NumericFraction::Normal,
        ordinal: false,
        slashed_zero: false,
    };
}

impl Default for FontVariantNumeric {
    fn default() -> Self {
        Self::NORMAL
    }
}

// ── font-variant-east-asian ─────────────────────────────────────────────

/// CSS `font-variant-east-asian` form sub-property.
///
/// Blink: `FontDescription::east_asian_form_` (3-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EastAsianForm {
    /// `normal` — use font default.
    Normal = 0,
    /// `jis78` → OpenType `"jp78"`.
    Jis78 = 1,
    /// `jis83` → OpenType `"jp83"`.
    Jis83 = 2,
    /// `jis90` → OpenType `"jp90"`.
    Jis90 = 3,
    /// `jis04` → OpenType `"jp04"`.
    Jis04 = 4,
    /// `simplified` → OpenType `"smpl"`.
    Simplified = 5,
    /// `traditional` → OpenType `"trad"`.
    Traditional = 6,
}

impl Default for EastAsianForm {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-east-asian` width sub-property.
///
/// Blink: `FontDescription::east_asian_width_` (2-bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EastAsianWidth {
    /// `normal` — use font default.
    Normal = 0,
    /// `full-width` → OpenType `"fwid"`.
    FullWidth = 1,
    /// `proportional-width` → OpenType `"pwid"`.
    ProportionalWidth = 2,
}

impl Default for EastAsianWidth {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `font-variant-east-asian` — controls East Asian text features.
///
/// Blink: individual bit fields in `FontDescription`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontVariantEastAsian {
    /// `jis78` / `jis83` / `jis90` / `jis04` / `simplified` / `traditional`.
    pub form: EastAsianForm,
    /// `full-width` / `proportional-width`.
    pub width: EastAsianWidth,
    /// `ruby` → OpenType `"ruby"`.
    pub ruby: bool,
}

impl FontVariantEastAsian {
    /// CSS `font-variant-east-asian: normal`.
    pub const NORMAL: Self = Self {
        form: EastAsianForm::Normal,
        width: EastAsianWidth::Normal,
        ruby: false,
    };
}

impl Default for FontVariantEastAsian {
    fn default() -> Self {
        Self::NORMAL
    }
}

// ── font-variant-position ───────────────────────────────────────────────

/// CSS `font-variant-position` — controls sub/superscript glyph variants.
///
/// Blink: `FontDescription::VariantPosition` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontVariantPosition {
    /// `normal` — no positional variant.
    Normal = 0,
    /// `sub` → OpenType `"subs"`.
    Sub = 1,
    /// `super` → OpenType `"sups"`.
    Super = 2,
}

impl Default for FontVariantPosition {
    fn default() -> Self {
        Self::Normal
    }
}

// ── font-variant-alternates (basic) ─────────────────────────────────────

/// CSS `font-variant-alternates` — basic keyword values.
///
/// The full CSS spec includes function values (`stylistic()`, `swash()`, etc.)
/// that require `@font-feature-values` rules. This enum covers the keyword-only
/// subset matching Blink's `FontDescription::VariantAlternates`.
///
/// Blink: `FontDescription::font_variant_alternates_` (bit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontVariantAlternates {
    /// `normal` — no alternates.
    Normal = 0,
    /// `historical-forms` → OpenType `"hist"`.
    HistoricalForms = 1,
}

impl Default for FontVariantAlternates {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS `-webkit-font-smoothing` / `font-smooth` property.
/// Blink: `FontDescription::FontSmoothing`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontSmoothing {
    Auto = 0,
    None = 1,
    Antialiased = 2,
    SubpixelAntialiased = 3,
}

impl Default for FontSmoothing {
    fn default() -> Self {
        Self::Auto
    }
}

/// CSS `text-rendering` property.
/// Blink: `TextRenderingMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextRendering {
    Auto = 0,
    OptimizeSpeed = 1,
    OptimizeLegibility = 2,
    GeometricPrecision = 3,
}

impl Default for TextRendering {
    fn default() -> Self {
        Self::Auto
    }
}

/// An OpenType feature setting (e.g., `font-feature-settings: "liga" 1`).
/// Blink: `FontFeature` in `font_description.h`.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFeature {
    /// Four-byte OpenType tag (e.g., `b"liga"`, `b"kern"`).
    pub tag: [u8; 4],
    /// Feature value (0 = off, 1 = on, or higher for alternates).
    pub value: u32,
}

/// A font variation axis setting (e.g., `font-variation-settings: "wght" 600`).
/// Blink: `FontVariationAxis` in `font_description.h`.
#[derive(Debug, Clone, PartialEq)]
pub struct FontVariation {
    /// Four-byte OpenType variation axis tag (e.g., `b"wght"`, `b"wdth"`).
    pub tag: [u8; 4],
    /// Axis value.
    pub value: f32,
}

/// CSS `font-synthesis-weight` / `font-synthesis-style` values.
/// Blink: `FontDescription::FontSynthesisWeight` / `FontSynthesisStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontSynthesis {
    Auto = 0,
    None = 1,
}

impl Default for FontSynthesis {
    fn default() -> Self {
        Self::Auto
    }
}

/// CSS `font-optical-sizing` property.
/// Blink: `OpticalSizing` in `font_description.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontOpticalSizing {
    Auto = 0,
    None = 1,
}

impl Default for FontOpticalSizing {
    fn default() -> Self {
        Self::Auto
    }
}

/// A single font family — either a named family or a generic keyword.
/// Blink: `FontFamily` in `font_family.h`.
#[derive(Clone, Debug, PartialEq)]
pub enum FontFamily {
    /// A specific named font family (e.g., "Arial", "Helvetica Neue").
    Named(String),
    /// A CSS generic font family keyword.
    Generic(GenericFontFamily),
}

/// CSS generic font family keywords.
/// Blink: `FontDescription::GenericFamilyType` + CSS Fonts Level 4 additions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GenericFontFamily {
    None = 0,
    Serif = 1,
    SansSerif = 2,
    Monospace = 3,
    Cursive = 4,
    Fantasy = 5,
    SystemUi = 6,
    Math = 7,
    Emoji = 8,
    FangSong = 9,
    UiSerif = 10,
    UiSansSerif = 11,
    UiMonospace = 12,
    UiRounded = 13,
}

impl Default for GenericFontFamily {
    fn default() -> Self {
        Self::None
    }
}

/// An ordered list of font families for CSS `font-family`.
/// Blink: `FontFamily` linked list in `font_family.h`.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFamilyList {
    pub families: Vec<FontFamily>,
}

impl FontFamilyList {
    /// Create a list with a single named family.
    pub fn single(name: impl Into<String>) -> Self {
        Self {
            families: vec![FontFamily::Named(name.into())],
        }
    }

    /// Create a list with a single generic family.
    pub fn generic(family: GenericFontFamily) -> Self {
        Self {
            families: vec![FontFamily::Generic(family)],
        }
    }

    /// The default font family list (sans-serif).
    pub fn default_list() -> Self {
        Self {
            families: vec![FontFamily::Generic(GenericFontFamily::SansSerif)],
        }
    }

    /// Returns true if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }

    /// Returns the number of families in the list.
    pub fn len(&self) -> usize {
        self.families.len()
    }
}

impl Default for FontFamilyList {
    fn default() -> Self {
        Self::default_list()
    }
}
