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
    Normal = 0,
    SmallCaps = 1,
    AllSmallCaps = 2,
    PetiteCaps = 3,
    AllPetiteCaps = 4,
    Unicase = 5,
    TitlingCaps = 6,
}

impl Default for FontVariantCaps {
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
