//! Public CSS typography value types.
//!
//! The renderer has historically carried most of these values in
//! `ComputedStyle`, but applications could not set them without reaching into
//! that implementation type.  These wrappers are the canonical author-facing
//! representation shared by Rust, `view!`, and the C tagged-value API.

use crate::{
    BlockEllipsis, Color, FontFamilyList, FontFeature, FontOpticalSizing, FontStretch,
    FontStyleEnum, FontSynthesis, FontVariantAlternates, FontVariantCaps, FontVariantEastAsian,
    FontVariantLigatures, FontVariantNumeric, FontVariantPosition, FontVariation, FontWeight,
    LineHeight, StyleColor, TextDecorationLine, TextDecorationStyle, TextDecorationThickness,
    TextEmphasisFill, TextEmphasisMark, TextEmphasisPosition,
};

/// Typed pseudo-element style targets supported by Chromium's typography
/// cascade.  A declaration list is validated before being attached to one of
/// these targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PseudoStyleTarget {
    FirstLine = 0,
    FirstLetter = 1,
    Marker = 2,
    Placeholder = 3,
}

/// Validated BCP 47 language input used by shaping and hyphenation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageTag(String);

impl LanguageTag {
    pub fn parse(input: impl Into<String>) -> Option<Self> {
        let value = input.into();
        let valid = !value.is_empty()
            && value.is_ascii()
            && value.split('-').all(|part| {
                !part.is_empty()
                    && part.len() <= 8
                    && part.bytes().all(|b| b.is_ascii_alphanumeric())
            });
        valid.then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontKerning {
    Auto = 0,
    Normal = 1,
    None = 2,
}

impl Default for FontKerning {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontVariantEmoji {
    Normal = 0,
    Text = 1,
    Emoji = 2,
    Unicode = 3,
}

impl Default for FontVariantEmoji {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontSizeAdjust {
    None,
    ExHeight(f32),
    CapHeight(f32),
    ChWidth(f32),
    IcWidth(f32),
    IcHeight(f32),
    FromFont,
}

impl Default for FontSizeAdjust {
    fn default() -> Self {
        Self::None
    }
}

/// Four-byte OpenType language-system override, or `normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontLanguageOverride(pub Option<[u8; 4]>);

impl FontLanguageOverride {
    pub const NORMAL: Self = Self(None);

    pub fn tag(tag: &str) -> Option<Self> {
        let bytes: [u8; 4] = tag.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
            .then_some(Self(Some(bytes)))
    }
}

/// Validated OpenType feature list. Later duplicate tags win, matching CSS.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpenTypeFeatureList(pub Vec<FontFeature>);

impl OpenTypeFeatureList {
    pub fn new(features: impl IntoIterator<Item = FontFeature>) -> Self {
        Self(features.into_iter().collect())
    }

    pub fn push(mut self, tag: [u8; 4], value: u32) -> Self {
        self.0.push(FontFeature { tag, value });
        self
    }
}

/// Validated OpenType variation-axis list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontVariationList(pub Vec<FontVariation>);

impl FontVariationList {
    pub fn new(axes: impl IntoIterator<Item = FontVariation>) -> Option<Self> {
        let axes: Vec<_> = axes.into_iter().collect();
        axes.iter()
            .all(|axis| axis.value.is_finite() && axis.tag.iter().all(u8::is_ascii_graphic))
            .then_some(Self(axes))
    }

    pub fn push(mut self, tag: [u8; 4], value: f32) -> Option<Self> {
        if !value.is_finite() || !tag.iter().all(u8::is_ascii_graphic) {
            return None;
        }
        self.0.push(FontVariation { tag, value });
        Some(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HyphenateCharacter(pub Option<String>);

impl HyphenateCharacter {
    pub const AUTO: Self = Self(None);

    pub fn character(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.is_empty()).then_some(Self(Some(value)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HyphenationLimits {
    pub word: u8,
    pub before: u8,
    pub after: u8,
}

impl HyphenationLimits {
    pub const AUTO: Self = Self {
        word: 5,
        before: 2,
        after: 2,
    };

    pub const fn new(word: u8, before: u8, after: u8) -> Self {
        Self {
            word,
            before,
            after,
        }
    }
}

impl Default for HyphenationLimits {
    fn default() -> Self {
        Self::AUTO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WhiteSpaceCollapse {
    Collapse = 0,
    Preserve = 1,
    PreserveBreaks = 2,
    BreakSpaces = 3,
}

impl Default for WhiteSpaceCollapse {
    fn default() -> Self {
        Self::Collapse
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextWrapMode {
    Wrap = 0,
    Nowrap = 1,
}

impl Default for TextWrapMode {
    fn default() -> Self {
        Self::Wrap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextWrapStyle {
    Auto = 0,
    Balance = 1,
    Pretty = 2,
    Stable = 3,
}

impl Default for TextWrapStyle {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextAutospace {
    NoAutospace = 0,
    Normal = 1,
}

impl Default for TextAutospace {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextSpacingTrim {
    Normal = 0,
    SpaceAll = 1,
    SpaceFirst = 2,
    TrimStart = 3,
}

impl Default for TextSpacingTrim {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextSizeAdjust {
    Auto,
    None,
    Percentage(f32),
}

impl Default for TextSizeAdjust {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RubyOverhang {
    Auto = 0,
    None = 1,
}

impl Default for RubyOverhang {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextBoxTrim {
    None = 0,
    TrimStart = 1,
    TrimEnd = 2,
    TrimBoth = 3,
}

impl Default for TextBoxTrim {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextBoxEdge {
    pub over: TextBoxEdgeKeyword,
    pub under: TextBoxEdgeKeyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextBoxShorthand {
    pub trim: TextBoxTrim,
    pub edge: TextBoxEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InitialLetterValue {
    #[default]
    Normal,
    Value(crate::InitialLetter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextBoxEdgeKeyword {
    Auto = 0,
    Text = 1,
    Cap = 2,
    Ex = 3,
    Ideographic = 4,
    IdeographicInk = 5,
    Alphabetic = 6,
}

impl Default for TextBoxEdge {
    fn default() -> Self {
        Self {
            over: TextBoxEdgeKeyword::Auto,
            under: TextBoxEdgeKeyword::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextShadowList(pub Vec<crate::TextShadow>);

impl TextShadowList {
    pub fn new(shadows: impl IntoIterator<Item = crate::TextShadow>) -> Self {
        Self(shadows.into_iter().collect())
    }

    pub fn push(mut self, x: f32, y: f32, blur: f32, color: Color) -> Option<Self> {
        if !x.is_finite() || !y.is_finite() || !blur.is_finite() || blur < 0.0 {
            return None;
        }
        self.0.push(crate::TextShadow {
            offset_x: x,
            offset_y: y,
            blur_radius: blur,
            color,
        });
        Some(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextEmphasisStyle {
    pub mark: TextEmphasisMark,
    pub fill: TextEmphasisFill,
}

impl Default for TextEmphasisStyle {
    fn default() -> Self {
        Self {
            mark: TextEmphasisMark::None,
            fill: TextEmphasisFill::Filled,
        }
    }
}

/// Atomic computed value for the `font-synthesis` shorthand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSynthesisShorthand {
    pub weight: FontSynthesis,
    pub style: FontSynthesis,
    pub small_caps: FontSynthesis,
    pub position: FontSynthesis,
}

impl FontSynthesisShorthand {
    pub const AUTO: Self = Self {
        weight: FontSynthesis::Auto,
        style: FontSynthesis::Auto,
        small_caps: FontSynthesis::Auto,
        position: FontSynthesis::Auto,
    };

    pub const NONE: Self = Self {
        weight: FontSynthesis::None,
        style: FontSynthesis::None,
        small_caps: FontSynthesis::None,
        position: FontSynthesis::None,
    };
}

impl Default for FontSynthesisShorthand {
    fn default() -> Self {
        Self::AUTO
    }
}

/// Atomic computed value for the `font-variant` shorthand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontVariantShorthand {
    pub ligatures: FontVariantLigatures,
    pub caps: FontVariantCaps,
    pub alternates: FontVariantAlternates,
    pub numeric: FontVariantNumeric,
    pub east_asian: FontVariantEastAsian,
    pub position: FontVariantPosition,
    pub emoji: FontVariantEmoji,
}

impl Default for FontVariantShorthand {
    fn default() -> Self {
        Self {
            ligatures: FontVariantLigatures::default(),
            caps: FontVariantCaps::default(),
            alternates: FontVariantAlternates::default(),
            numeric: FontVariantNumeric::default(),
            east_asian: FontVariantEastAsian::default(),
            position: FontVariantPosition::default(),
            emoji: FontVariantEmoji::default(),
        }
    }
}

/// Atomic computed value for the CSS `font` shorthand. Constructing this
/// value requires the mandatory size and family components, so a partial
/// shorthand can never enter the declaration list.
#[derive(Debug, Clone, PartialEq)]
pub struct FontShorthand {
    pub style: FontStyleEnum,
    pub variant_caps: FontVariantCaps,
    pub weight: FontWeight,
    pub stretch: FontStretch,
    pub size_px: f32,
    pub line_height: LineHeight,
    pub family: FontFamilyList,
}

impl FontShorthand {
    pub fn new(size_px: f32, family: FontFamilyList) -> Option<Self> {
        (size_px.is_finite() && size_px > 0.0 && !family.is_empty()).then_some(Self {
            style: FontStyleEnum::Normal,
            variant_caps: FontVariantCaps::Normal,
            weight: FontWeight::NORMAL,
            stretch: FontStretch::NORMAL,
            size_px,
            line_height: LineHeight::Normal,
            family,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WhiteSpaceShorthand {
    pub collapse: WhiteSpaceCollapse,
    pub wrap: TextWrapMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextWrapShorthand {
    pub mode: TextWrapMode,
    pub style: TextWrapStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextDecorationShorthand {
    pub line: TextDecorationLine,
    pub style: TextDecorationStyle,
    pub color: StyleColor,
    pub thickness: TextDecorationThickness,
}

impl Default for TextDecorationShorthand {
    fn default() -> Self {
        Self {
            line: TextDecorationLine::NONE,
            style: TextDecorationStyle::Solid,
            color: StyleColor::CurrentColor,
            thickness: TextDecorationThickness::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextEmphasisShorthand {
    pub style: TextEmphasisStyle,
    pub color: StyleColor,
    pub position: TextEmphasisPosition,
}

impl Default for TextEmphasisShorthand {
    fn default() -> Self {
        Self {
            style: TextEmphasisStyle::default(),
            color: StyleColor::CurrentColor,
            position: TextEmphasisPosition::INITIAL,
        }
    }
}

/// The typography-specific payload behind `StyleValue`.
#[derive(Debug, Clone, PartialEq)]
pub enum TypographyValue {
    Direction(crate::Direction),
    FontKerning(FontKerning),
    FontOpticalSizing(FontOpticalSizing),
    FontPalette(crate::FontPalette),
    FontSizeAdjust(FontSizeAdjust),
    FontStretch(FontStretch),
    FontStyle(FontStyleEnum),
    FontVariantLigatures(FontVariantLigatures),
    FontVariantCaps(FontVariantCaps),
    FontVariantEastAsian(FontVariantEastAsian),
    FontVariantNumeric(FontVariantNumeric),
    FontVariantAlternates(FontVariantAlternates),
    FontVariantPosition(FontVariantPosition),
    FontVariantEmoji(FontVariantEmoji),
    FontSynthesis(FontSynthesis),
    OpenTypeFeatures(OpenTypeFeatureList),
    FontVariations(FontVariationList),
    FontLanguageOverride(FontLanguageOverride),
    LineHeight(LineHeight),
    TextAlign(crate::TextAlign),
    TextAlignLast(crate::TextAlignLast),
    TextJustify(crate::TextJustify),
    WordBreak(crate::WordBreak),
    OverflowWrap(crate::OverflowWrap),
    LineBreak(crate::LineBreak),
    Hyphens(crate::Hyphens),
    HyphenationLimits(HyphenationLimits),
    HyphenateCharacter(HyphenateCharacter),
    WhiteSpaceCollapse(WhiteSpaceCollapse),
    TextWrapMode(TextWrapMode),
    TextWrapStyle(TextWrapStyle),
    TextAutospace(TextAutospace),
    TextSpacingTrim(TextSpacingTrim),
    TabSize(crate::TabSize),
    TextTransform(crate::TextTransform),
    TextDecorationLine(TextDecorationLine),
    TextDecorationStyle(TextDecorationStyle),
    TextDecorationThickness(TextDecorationThickness),
    StyleColor(StyleColor),
    TextDecorationSkipInk(crate::TextDecorationSkipInk),
    TextUnderlinePosition(crate::TextUnderlinePosition),
    TextEmphasisStyle(TextEmphasisStyle),
    TextEmphasisPosition(TextEmphasisPosition),
    TextShadows(TextShadowList),
    TextOverflow(crate::TextOverflow),
    TextSizeAdjust(TextSizeAdjust),
    TextCombineUpright(crate::TextCombineUpright),
    WritingMode(crate::WritingMode),
    TextOrientation(crate::TextOrientation),
    UnicodeBidi(crate::UnicodeBidi),
    VerticalAlign(crate::VerticalAlign),
    RubyAlign(crate::RubyAlign),
    RubyPosition(crate::RubyPosition),
    RubyOverhang(RubyOverhang),
    HangingPunctuation(crate::HangingPunctuation),
    InitialLetter(InitialLetterValue),
    TextRendering(crate::TextRendering),
    FontSmoothing(crate::FontSmoothing),
    LineClamp(crate::LineClamp),
    BlockEllipsis(BlockEllipsis),
    TextBoxEdge(TextBoxEdge),
    TextBoxTrim(TextBoxTrim),
    TextBox(TextBoxShorthand),
    Font(FontShorthand),
    FontVariant(FontVariantShorthand),
    FontSynthesisShorthand(FontSynthesisShorthand),
    WhiteSpace(WhiteSpaceShorthand),
    TextWrap(TextWrapShorthand),
    TextDecoration(TextDecorationShorthand),
    TextEmphasis(TextEmphasisShorthand),
}
