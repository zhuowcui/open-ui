//! Canonical public property schema and typed declaration values.

use crate::{
    AnimationSnapshot, AspectRatio, BackgroundAttachment, BackgroundClip, BackgroundLayer,
    BlockEllipsis, BorderCollapse, BorderImage, BorderStyle, BoxDecorationBreak, BoxShadow,
    BoxSizing, BreakInside, BreakValue, CaptionSide, Clear, Color, ColumnFill, ColumnSpan,
    ColumnWrap, ComputedStyle, ContainIntrinsicLength, ContainerType, Containment,
    ContentAlignment, ContentDistribution, ContentPosition, ContentVisibility, CounterOperation,
    Direction, Display, EmptyCells, FlexDirection, FlexWrap, Float, FontFamilyList, FontFeature,
    FontKerning, FontLanguageOverride, FontOpticalSizing, FontPalette, FontShorthand,
    FontSizeAdjust, FontSmoothing, FontStretch, FontStyleEnum, FontSynthesis,
    FontSynthesisShorthand, FontVariantAlternates, FontVariantCaps, FontVariantEastAsian,
    FontVariantEmoji, FontVariantLigatures, FontVariantNumeric, FontVariantPosition,
    FontVariantShorthand, FontVariation, FontVariationList, FontWeight, GeneratedContentItem,
    GenericFontFamily, GridAutoFlow, GridPlacement, GridTemplateAreas, GridTrackList,
    GridTrackSize, HangingPunctuation, HyphenateCharacter, HyphenationLimits, Hyphens,
    InitialLetter, InitialLetterValue, ItemAlignment, ItemPosition, LineBreak, LineClamp,
    LineHeight, LinearGradient, ListStylePosition, ListStyleType, MarginTrim, ObjectFit,
    ObjectPosition, OpenTypeFeatureList, Overflow, OverflowAlignment, OverflowClipBox,
    OverflowWrap, Position, PositionArea, QuotePair, Resize, RubyAlign, RubyOverhang, RubyPosition,
    ScrollMarkerGroup, ScrollSnapAlign, ScrollSnapAxis, ScrollTargetGroup, ScrollbarGutter,
    ScrollbarWidth, ShapeOutside, StyleColor, TabSize, TableLayout, TextAlign, TextAlignLast,
    TextAutospace, TextBoxEdge, TextBoxShorthand, TextBoxTrim, TextCombineUpright,
    TextDecorationLine, TextDecorationShorthand, TextDecorationSkipInk, TextDecorationStyle,
    TextDecorationThickness, TextEmphasisFill, TextEmphasisMark, TextEmphasisPosition,
    TextEmphasisShorthand, TextEmphasisStyle, TextJustify, TextOrientation, TextOverflow,
    TextRendering, TextShadow, TextShadowList, TextSizeAdjust, TextSpacingTrim, TextTransform,
    TextUnderlinePosition, TextWrap, TextWrapMode, TextWrapShorthand, TextWrapStyle, Transform2D,
    TypographyValue, UnicodeBidi, VerticalAlign, Visibility, WebkitBoxOrient, WhiteSpace,
    WhiteSpaceCollapse, WhiteSpaceShorthand, WordBreak, WritingMode,
};
use openui_geometry::{Length, LengthType, RasterConfiguration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Display,
    Position,
    Overflow,
    Length,
    Edges,
    Color,
    Number,
    Integer,
    FlexDirection,
    FlexWrap,
    ItemAlignment,
    ContentAlignment,
    Gap,
    FontFamily,
    FontWeight,
    Border,
    CornerRadii,
    Cursor,
    ListStyle,
    Transform,
    PointerEvents,
    Enum,
    Compound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum InvalidationClass {
    Composite,
    Paint,
    Layout,
    Intrinsic,
    Subtree,
    Accessibility,
}

impl InvalidationClass {
    pub fn includes(self, other: Self) -> bool {
        self >= other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationKind {
    Discrete,
    Number,
    Integer,
    Length,
    LengthList,
    Color,
    Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyMetadata {
    pub property: StyleProperty,
    pub css_name: &'static str,
    pub rust_type: &'static str,
    pub value_kind: ValueKind,
    pub initial: &'static str,
    pub inherited: bool,
    pub invalidation: InvalidationClass,
    pub interpolation: InterpolationKind,
}

include!("generated_properties.rs");

/// A length before viewport and font-relative units are resolved for layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthValue {
    Computed(Length),
    Em(f32),
    Rem(f32),
    ViewportWidth(f32),
    ViewportHeight(f32),
    ViewportMin(f32),
    ViewportMax(f32),
}

impl LengthValue {
    pub const fn px(value: f32) -> Self {
        Self::Computed(Length::px(value))
    }

    pub const fn percent(value: f32) -> Self {
        Self::Computed(Length::percent(value))
    }

    pub const fn auto() -> Self {
        Self::Computed(Length::auto())
    }

    pub const fn none() -> Self {
        Self::Computed(Length::none())
    }

    pub fn resolve(self, viewport: (f32, f32), font_size: f32, root_font_size: f32) -> Length {
        match self {
            Self::Computed(value) => value,
            Self::Em(value) => Length::px(value * font_size),
            Self::Rem(value) => Length::px(value * root_font_size),
            Self::ViewportWidth(value) => Length::px(viewport.0 * value / 100.0),
            Self::ViewportHeight(value) => Length::px(viewport.1 * value / 100.0),
            Self::ViewportMin(value) => Length::px(viewport.0.min(viewport.1) * value / 100.0),
            Self::ViewportMax(value) => Length::px(viewport.0.max(viewport.1) * value / 100.0),
        }
    }
}

impl From<Length> for LengthValue {
    fn from(value: Length) -> Self {
        Self::Computed(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub const fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gap {
    pub row: LengthValue,
    pub column: LengthValue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub style: BorderStyle,
    pub color: Color,
}

impl Border {
    pub const NONE: Self = Self {
        width: 0.0,
        style: BorderStyle::None,
        color: Color::BLACK,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii(pub Edges<LengthValue>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cursor {
    Auto,
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PointerEvents {
    Auto,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransformOperation {
    Translate(LengthValue, LengthValue),
    Translate3d(LengthValue, LengthValue, LengthValue),
    Scale(f32, f32),
    Scale3d(f32, f32, f32),
    Rotate(f32),
    Rotate3d {
        x: f32,
        y: f32,
        z: f32,
        degrees: f32,
    },
    Perspective(LengthValue),
    Matrix(Transform2D),
    Matrix3d(Transform3D),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransformList(pub Vec<TransformOperation>);

/// A column-major CSS 4x4 transform matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3D(pub [f32; 16]);

impl Transform3D {
    pub const IDENTITY: Self = Self([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    pub fn projected_2d(self) -> Transform2D {
        Transform2D {
            a: self.0[0],
            b: self.0[1],
            c: self.0[4],
            d: self.0[5],
            e: self.0[12],
            f: self.0[13],
        }
    }
}

impl Default for Transform3D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Display(Display),
    Position(Position),
    Overflow(Overflow),
    Length(LengthValue),
    Edges(Edges<LengthValue>),
    Color(Color),
    Number(f32),
    Integer(i32),
    FlexDirection(FlexDirection),
    FlexWrap(FlexWrap),
    ItemAlignment(ItemAlignment),
    ContentAlignment(ContentAlignment),
    Gap(Gap),
    FontFamily(FontFamilyList),
    FontWeight(FontWeight),
    Border(Border),
    CornerRadii(CornerRadii),
    Cursor(Cursor),
    ListStyle(ListStyleType),
    Transform(TransformList),
    PointerEvents(PointerEvents),
    Typography(TypographyValue),
    Renderer(RendererStyleValue),
}

macro_rules! impl_style_value {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for StyleValue {
            fn from(value: $ty) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_style_value!(Display, Display);
impl_style_value!(Position, Position);
impl_style_value!(Overflow, Overflow);
impl_style_value!(LengthValue, Length);
impl_style_value!(Edges<LengthValue>, Edges);
impl_style_value!(Color, Color);
impl_style_value!(f32, Number);
impl_style_value!(i32, Integer);
impl_style_value!(FlexDirection, FlexDirection);
impl_style_value!(FlexWrap, FlexWrap);
impl_style_value!(ItemAlignment, ItemAlignment);
impl_style_value!(ContentAlignment, ContentAlignment);
impl_style_value!(Gap, Gap);
impl_style_value!(FontFamilyList, FontFamily);
impl_style_value!(FontWeight, FontWeight);
impl_style_value!(Border, Border);
impl_style_value!(CornerRadii, CornerRadii);
impl_style_value!(Cursor, Cursor);
impl_style_value!(ListStyleType, ListStyle);
impl_style_value!(TransformList, Transform);
impl_style_value!(PointerEvents, PointerEvents);

macro_rules! impl_typography_value {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for StyleValue {
            fn from(value: $ty) -> Self {
                Self::Typography(TypographyValue::$variant(value))
            }
        }
    };
}

impl_typography_value!(Direction, Direction);
impl_typography_value!(FontKerning, FontKerning);
impl_typography_value!(FontOpticalSizing, FontOpticalSizing);
impl_typography_value!(FontPalette, FontPalette);
impl_typography_value!(FontSizeAdjust, FontSizeAdjust);
impl_typography_value!(FontStretch, FontStretch);
impl_typography_value!(FontStyleEnum, FontStyle);
impl_typography_value!(FontVariantLigatures, FontVariantLigatures);
impl_typography_value!(FontVariantCaps, FontVariantCaps);
impl_typography_value!(FontVariantEastAsian, FontVariantEastAsian);
impl_typography_value!(FontVariantNumeric, FontVariantNumeric);
impl_typography_value!(FontVariantAlternates, FontVariantAlternates);
impl_typography_value!(FontVariantPosition, FontVariantPosition);
impl_typography_value!(FontVariantEmoji, FontVariantEmoji);
impl_typography_value!(FontSynthesis, FontSynthesis);
impl_typography_value!(OpenTypeFeatureList, OpenTypeFeatures);
impl_typography_value!(FontVariationList, FontVariations);
impl_typography_value!(FontLanguageOverride, FontLanguageOverride);
impl_typography_value!(LineHeight, LineHeight);
impl_typography_value!(TextAlign, TextAlign);
impl_typography_value!(TextAlignLast, TextAlignLast);
impl_typography_value!(TextJustify, TextJustify);
impl_typography_value!(WordBreak, WordBreak);
impl_typography_value!(OverflowWrap, OverflowWrap);
impl_typography_value!(LineBreak, LineBreak);
impl_typography_value!(Hyphens, Hyphens);
impl_typography_value!(HyphenationLimits, HyphenationLimits);
impl_typography_value!(HyphenateCharacter, HyphenateCharacter);
impl_typography_value!(WhiteSpaceCollapse, WhiteSpaceCollapse);
impl_typography_value!(TextWrapMode, TextWrapMode);
impl_typography_value!(TextWrapStyle, TextWrapStyle);
impl_typography_value!(TextAutospace, TextAutospace);
impl_typography_value!(TextSpacingTrim, TextSpacingTrim);
impl_typography_value!(TabSize, TabSize);
impl_typography_value!(TextTransform, TextTransform);
impl_typography_value!(TextDecorationLine, TextDecorationLine);
impl_typography_value!(TextDecorationStyle, TextDecorationStyle);
impl_typography_value!(TextDecorationThickness, TextDecorationThickness);
impl_typography_value!(StyleColor, StyleColor);
impl_typography_value!(TextDecorationSkipInk, TextDecorationSkipInk);
impl_typography_value!(TextUnderlinePosition, TextUnderlinePosition);
impl_typography_value!(TextEmphasisStyle, TextEmphasisStyle);
impl_typography_value!(TextEmphasisPosition, TextEmphasisPosition);
impl_typography_value!(TextShadowList, TextShadows);
impl_typography_value!(TextOverflow, TextOverflow);
impl_typography_value!(TextSizeAdjust, TextSizeAdjust);
impl_typography_value!(TextCombineUpright, TextCombineUpright);
impl_typography_value!(WritingMode, WritingMode);
impl_typography_value!(TextOrientation, TextOrientation);
impl_typography_value!(UnicodeBidi, UnicodeBidi);
impl_typography_value!(VerticalAlign, VerticalAlign);
impl_typography_value!(RubyAlign, RubyAlign);
impl_typography_value!(RubyPosition, RubyPosition);
impl_typography_value!(RubyOverhang, RubyOverhang);
impl_typography_value!(HangingPunctuation, HangingPunctuation);
impl_typography_value!(InitialLetterValue, InitialLetter);
impl_typography_value!(TextRendering, TextRendering);
impl_typography_value!(FontSmoothing, FontSmoothing);
impl_typography_value!(LineClamp, LineClamp);
impl_typography_value!(BlockEllipsis, BlockEllipsis);
impl_typography_value!(TextBoxEdge, TextBoxEdge);
impl_typography_value!(TextBoxTrim, TextBoxTrim);
impl_typography_value!(TextBoxShorthand, TextBox);
impl_typography_value!(FontShorthand, Font);
impl_typography_value!(FontVariantShorthand, FontVariant);
impl_typography_value!(FontSynthesisShorthand, FontSynthesisShorthand);
impl_typography_value!(WhiteSpaceShorthand, WhiteSpace);
impl_typography_value!(TextWrapShorthand, TextWrap);
impl_typography_value!(TextDecorationShorthand, TextDecoration);
impl_typography_value!(TextEmphasisShorthand, TextEmphasis);

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: StyleProperty,
    pub value: StyleValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    declarations: Vec<Declaration>,
}

impl Style {
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    fn with(mut self, property: StyleProperty, value: impl Into<StyleValue>) -> Self {
        self.declarations.push(Declaration {
            property,
            value: value.into(),
        });
        self
    }

    fn with_renderer(mut self, property: StyleProperty, value: RendererStyleValue) -> Self {
        self.declarations.push(Declaration {
            property,
            value: StyleValue::Renderer(value),
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralError(pub String);

impl std::fmt::Display for LiteralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LiteralError {}

fn invalid(property: StyleProperty, input: &str) -> LiteralError {
    LiteralError(format!(
        "invalid literal `{input}` for `{}` (expected {})",
        property.metadata().css_name,
        property.metadata().rust_type
    ))
}

fn length(input: &str) -> Option<LengthValue> {
    let input = input.trim();
    match input {
        "0" => Some(LengthValue::px(0.0)),
        "auto" => Some(LengthValue::auto()),
        "none" => Some(LengthValue::none()),
        _ => {
            for (suffix, make) in [
                ("px", LengthValue::px as fn(f32) -> LengthValue),
                ("%", LengthValue::percent),
                ("em", LengthValue::Em),
                ("rem", LengthValue::Rem),
                ("vw", LengthValue::ViewportWidth),
                ("vh", LengthValue::ViewportHeight),
                ("vmin", LengthValue::ViewportMin),
                ("vmax", LengthValue::ViewportMax),
            ] {
                if let Some(number) = input
                    .strip_suffix(suffix)
                    .and_then(|v| v.trim().parse::<f32>().ok())
                    .filter(|value| value.is_finite())
                {
                    return Some(make(number));
                }
            }
            None
        }
    }
}

fn edges(input: &str) -> Option<Edges<LengthValue>> {
    let values: Vec<_> = input
        .split_whitespace()
        .map(length)
        .collect::<Option<_>>()?;
    match values.as_slice() {
        [a] => Some(Edges::all(*a)),
        [v, h] => Some(Edges {
            top: *v,
            right: *h,
            bottom: *v,
            left: *h,
        }),
        [t, h, b] => Some(Edges {
            top: *t,
            right: *h,
            bottom: *b,
            left: *h,
        }),
        [t, r, b, l] => Some(Edges {
            top: *t,
            right: *r,
            bottom: *b,
            left: *l,
        }),
        _ => None,
    }
}

fn overflow_literal(input: &str) -> Option<Overflow> {
    match input {
        "visible" => Some(Overflow::Visible),
        "hidden" => Some(Overflow::Hidden),
        "scroll" => Some(Overflow::Scroll),
        "auto" => Some(Overflow::Auto),
        "clip" => Some(Overflow::Clip),
        _ => None,
    }
}

fn alignment_words(input: &str) -> (OverflowAlignment, Vec<&str>) {
    let mut words = input.split_whitespace();
    let mut tokens = Vec::new();
    let overflow = match words.next() {
        Some("safe") => OverflowAlignment::Safe,
        Some("unsafe") => OverflowAlignment::Unsafe,
        Some(word) => {
            tokens.push(word);
            OverflowAlignment::Default
        }
        None => OverflowAlignment::Default,
    };
    tokens.extend(words);
    (overflow, tokens)
}

fn item_alignment_literal(input: &str) -> Option<ItemAlignment> {
    let (overflow, words) = alignment_words(input);
    let position = match words.as_slice() {
        ["auto"] => ItemPosition::Auto,
        ["normal"] => ItemPosition::Normal,
        ["stretch"] => ItemPosition::Stretch,
        ["baseline"] | ["first", "baseline"] => ItemPosition::Baseline,
        ["last", "baseline"] => ItemPosition::LastBaseline,
        ["center"] => ItemPosition::Center,
        ["start"] => ItemPosition::Start,
        ["end"] => ItemPosition::End,
        ["self-start"] => ItemPosition::SelfStart,
        ["self-end"] => ItemPosition::SelfEnd,
        ["flex-start"] => ItemPosition::FlexStart,
        ["flex-end"] => ItemPosition::FlexEnd,
        ["left"] => ItemPosition::Left,
        ["right"] => ItemPosition::Right,
        ["legacy"] => ItemPosition::Legacy,
        _ => return None,
    };
    if overflow != OverflowAlignment::Default
        && matches!(
            position,
            ItemPosition::Auto
                | ItemPosition::Normal
                | ItemPosition::Stretch
                | ItemPosition::Baseline
                | ItemPosition::LastBaseline
                | ItemPosition::Legacy
        )
    {
        return None;
    }
    Some(ItemAlignment::with_overflow(position, overflow))
}

fn content_alignment_literal(input: &str) -> Option<ContentAlignment> {
    let (overflow, words) = alignment_words(input);
    let mut value = match words.as_slice() {
        ["normal"] => ContentAlignment::default(),
        ["center"] => ContentAlignment::new(ContentPosition::Center),
        ["start"] => ContentAlignment::new(ContentPosition::Start),
        ["end"] => ContentAlignment::new(ContentPosition::End),
        ["flex-start"] => ContentAlignment::new(ContentPosition::FlexStart),
        ["flex-end"] => ContentAlignment::new(ContentPosition::FlexEnd),
        ["left"] => ContentAlignment::new(ContentPosition::Left),
        ["right"] => ContentAlignment::new(ContentPosition::Right),
        ["baseline"] | ["first", "baseline"] => ContentAlignment::new(ContentPosition::Baseline),
        ["last", "baseline"] => ContentAlignment::new(ContentPosition::LastBaseline),
        ["space-between"] => ContentAlignment::with_distribution(ContentDistribution::SpaceBetween),
        ["space-around"] => ContentAlignment::with_distribution(ContentDistribution::SpaceAround),
        ["space-evenly"] => ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly),
        ["stretch"] => ContentAlignment::with_distribution(ContentDistribution::Stretch),
        _ => return None,
    };
    if overflow != OverflowAlignment::Default
        && (value.distribution != ContentDistribution::Default
            || matches!(
                value.position,
                ContentPosition::Normal | ContentPosition::Baseline | ContentPosition::LastBaseline
            ))
    {
        return None;
    }
    value.overflow = overflow;
    Some(value)
}

fn color(input: &str) -> Option<Color> {
    match input.trim().to_ascii_lowercase().as_str() {
        "transparent" => Some(Color::TRANSPARENT),
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "blue" => Some(Color::BLUE),
        value if value.starts_with('#') && (value.len() == 7 || value.len() == 9) => {
            u32::from_str_radix(&value[1..], 16)
                .ok()
                .map(|hex| Color::from_hex(hex, value.len() == 9))
        }
        _ => None,
    }
}

fn border(input: &str) -> Option<Border> {
    if input.trim() == "none" {
        return Some(Border::NONE);
    }
    let mut parts = input.split_whitespace();
    let width = length(parts.next()?)?;
    let LengthValue::Computed(width) = width else {
        return None;
    };
    if width.length_type() != LengthType::Fixed {
        return None;
    }
    let style = match parts.next()? {
        "solid" => BorderStyle::Solid,
        "dotted" => BorderStyle::Dotted,
        "dashed" => BorderStyle::Dashed,
        "double" => BorderStyle::Double,
        _ => return None,
    };
    let color = color(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(Border {
        width: width.value(),
        style,
        color,
    })
}

fn font_family_list(input: &str) -> Option<FontFamilyList> {
    let mut families = Vec::new();
    for raw in input.split(',') {
        let value = raw.trim();
        if value.is_empty() {
            return None;
        }
        let generic = match value.to_ascii_lowercase().as_str() {
            "serif" => Some(GenericFontFamily::Serif),
            "sans-serif" => Some(GenericFontFamily::SansSerif),
            "monospace" => Some(GenericFontFamily::Monospace),
            "cursive" => Some(GenericFontFamily::Cursive),
            "fantasy" => Some(GenericFontFamily::Fantasy),
            "system-ui" => Some(GenericFontFamily::SystemUi),
            "math" => Some(GenericFontFamily::Math),
            "emoji" => Some(GenericFontFamily::Emoji),
            "fangsong" => Some(GenericFontFamily::FangSong),
            "ui-serif" => Some(GenericFontFamily::UiSerif),
            "ui-sans-serif" => Some(GenericFontFamily::UiSansSerif),
            "ui-monospace" => Some(GenericFontFamily::UiMonospace),
            "ui-rounded" => Some(GenericFontFamily::UiRounded),
            _ => None,
        };
        families.push(match generic {
            Some(value) => crate::FontFamily::Generic(value),
            None => crate::FontFamily::Named(value.trim_matches(['\'', '"']).to_owned()),
        });
    }
    (!families.is_empty()).then_some(FontFamilyList { families })
}

fn opentype_tag(input: &str) -> Option<[u8; 4]> {
    let input = input.trim().trim_matches(['\'', '"']);
    let tag: [u8; 4] = input.as_bytes().try_into().ok()?;
    tag.iter()
        .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
        .then_some(tag)
}

fn feature_settings(input: &str) -> Option<OpenTypeFeatureList> {
    if input.trim() == "normal" {
        return Some(OpenTypeFeatureList::default());
    }
    let mut result = OpenTypeFeatureList::default();
    for item in input.split(',') {
        let mut parts = item.split_whitespace();
        let tag = opentype_tag(parts.next()?)?;
        let value = match parts.next() {
            None | Some("on") => 1,
            Some("off") => 0,
            Some(value) => value.parse().ok()?,
        };
        if parts.next().is_some() {
            return None;
        }
        result = result.push(tag, value);
    }
    Some(result)
}

fn variation_settings(input: &str) -> Option<FontVariationList> {
    if input.trim() == "normal" {
        return Some(FontVariationList::default());
    }
    let mut result = FontVariationList::default();
    for item in input.split(',') {
        let mut parts = item.split_whitespace();
        let tag = opentype_tag(parts.next()?)?;
        let value = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        result = result.push(tag, value)?;
    }
    Some(result)
}

fn font_ligatures(input: &str) -> Option<FontVariantLigatures> {
    use crate::LigatureState as S;
    if input == "normal" {
        return Some(FontVariantLigatures::NORMAL);
    }
    if input == "none" {
        return Some(FontVariantLigatures::none());
    }
    let mut value = FontVariantLigatures::NORMAL;
    for keyword in input.split_whitespace() {
        match keyword {
            "common-ligatures" => value.common = S::Enabled,
            "no-common-ligatures" => value.common = S::Disabled,
            "discretionary-ligatures" => value.discretionary = S::Enabled,
            "no-discretionary-ligatures" => value.discretionary = S::Disabled,
            "historical-ligatures" => value.historical = S::Enabled,
            "no-historical-ligatures" => value.historical = S::Disabled,
            "contextual" => value.contextual = S::Enabled,
            "no-contextual" => value.contextual = S::Disabled,
            _ => return None,
        }
    }
    Some(value)
}

fn font_numeric(input: &str) -> Option<FontVariantNumeric> {
    if input == "normal" {
        return Some(FontVariantNumeric::NORMAL);
    }
    let mut value = FontVariantNumeric::NORMAL;
    for keyword in input.split_whitespace() {
        match keyword {
            "lining-nums" => value.figure = crate::NumericFigure::LiningNums,
            "oldstyle-nums" => value.figure = crate::NumericFigure::OldstyleNums,
            "proportional-nums" => value.spacing = crate::NumericSpacing::ProportionalNums,
            "tabular-nums" => value.spacing = crate::NumericSpacing::TabularNums,
            "diagonal-fractions" => value.fraction = crate::NumericFraction::DiagonalFractions,
            "stacked-fractions" => value.fraction = crate::NumericFraction::StackedFractions,
            "ordinal" => value.ordinal = true,
            "slashed-zero" => value.slashed_zero = true,
            _ => return None,
        }
    }
    Some(value)
}

fn font_east_asian(input: &str) -> Option<FontVariantEastAsian> {
    if input == "normal" {
        return Some(FontVariantEastAsian::NORMAL);
    }
    let mut value = FontVariantEastAsian::NORMAL;
    for keyword in input.split_whitespace() {
        match keyword {
            "jis78" => value.form = crate::EastAsianForm::Jis78,
            "jis83" => value.form = crate::EastAsianForm::Jis83,
            "jis90" => value.form = crate::EastAsianForm::Jis90,
            "jis04" => value.form = crate::EastAsianForm::Jis04,
            "simplified" => value.form = crate::EastAsianForm::Simplified,
            "traditional" => value.form = crate::EastAsianForm::Traditional,
            "full-width" => value.width = crate::EastAsianWidth::FullWidth,
            "proportional-width" => value.width = crate::EastAsianWidth::ProportionalWidth,
            "ruby" => value.ruby = true,
            _ => return None,
        }
    }
    Some(value)
}

fn decoration_line(input: &str) -> Option<TextDecorationLine> {
    if input == "none" {
        return Some(TextDecorationLine::NONE);
    }
    let mut bits = 0;
    for keyword in input.split_whitespace() {
        bits |= match keyword {
            "underline" => TextDecorationLine::UNDERLINE.0,
            "overline" => TextDecorationLine::OVERLINE.0,
            "line-through" => TextDecorationLine::LINE_THROUGH.0,
            "blink" => 8,
            "spelling-error" => 16,
            "grammar-error" => 32,
            _ => return None,
        };
    }
    (bits != 0).then_some(TextDecorationLine(bits))
}

fn emphasis_style(input: &str) -> Option<TextEmphasisStyle> {
    if input == "none" {
        return Some(TextEmphasisStyle::default());
    }
    let mut result = TextEmphasisStyle {
        mark: TextEmphasisMark::Dot,
        fill: crate::TextEmphasisFill::Filled,
    };
    for keyword in input.split_whitespace() {
        match keyword {
            "filled" => result.fill = crate::TextEmphasisFill::Filled,
            "open" => result.fill = crate::TextEmphasisFill::Open,
            "dot" => result.mark = TextEmphasisMark::Dot,
            "circle" => result.mark = TextEmphasisMark::Circle,
            "double-circle" => result.mark = TextEmphasisMark::DoubleCircle,
            "triangle" => result.mark = TextEmphasisMark::Triangle,
            "sesame" => result.mark = TextEmphasisMark::Sesame,
            value => {
                let mut chars = value.trim_matches(['\'', '"']).chars();
                result.mark = TextEmphasisMark::Custom(chars.next()?);
                if chars.next().is_some() {
                    return None;
                }
            }
        }
    }
    Some(result)
}

fn text_shadows(input: &str) -> Option<TextShadowList> {
    if input.trim() == "none" {
        return Some(TextShadowList::default());
    }
    let mut result = TextShadowList::default();
    for shadow in input.split(',') {
        let mut lengths = Vec::new();
        let mut shadow_color = Color::BLACK;
        for token in shadow.split_whitespace() {
            if let Some(value) = color(token) {
                shadow_color = value;
            } else {
                let LengthValue::Computed(value) = length(token)? else {
                    return None;
                };
                if value.length_type() != LengthType::Fixed {
                    return None;
                }
                lengths.push(value.value());
            }
        }
        let (x, y, blur) = match lengths.as_slice() {
            [x, y] => (*x, *y, 0.0),
            [x, y, blur] => (*x, *y, *blur),
            _ => return None,
        };
        result = result.push(x, y, blur, shadow_color)?;
    }
    Some(result)
}

fn normalized_white_space(collapse: WhiteSpaceCollapse, wrap: TextWrapMode) -> crate::WhiteSpace {
    use crate::WhiteSpace as W;
    match (collapse, wrap) {
        (WhiteSpaceCollapse::Collapse, TextWrapMode::Wrap) => W::Normal,
        (WhiteSpaceCollapse::Collapse, TextWrapMode::Nowrap) => W::Nowrap,
        (WhiteSpaceCollapse::Preserve, TextWrapMode::Nowrap) => W::Pre,
        (WhiteSpaceCollapse::Preserve, TextWrapMode::Wrap) => W::PreWrap,
        (WhiteSpaceCollapse::PreserveBreaks, _) => W::PreLine,
        (WhiteSpaceCollapse::BreakSpaces, _) => W::BreakSpaces,
    }
}

fn normalized_text_wrap(mode: TextWrapMode, style: TextWrapStyle) -> crate::TextWrap {
    match mode {
        TextWrapMode::Nowrap => crate::TextWrap::Nowrap,
        TextWrapMode::Wrap => match style {
            TextWrapStyle::Auto => crate::TextWrap::Wrap,
            TextWrapStyle::Balance => crate::TextWrap::Balance,
            TextWrapStyle::Pretty => crate::TextWrap::Pretty,
            TextWrapStyle::Stable => crate::TextWrap::Stable,
        },
    }
}

fn parse_font_shorthand(input: &str) -> Option<FontShorthand> {
    let tokens: Vec<_> = input.split_whitespace().collect();
    let size_index = tokens.iter().position(|token| {
        let size = token.split('/').next().unwrap_or(token);
        length(size).is_some()
    })?;
    let (size_token, inline_line_height) = tokens[size_index]
        .split_once('/')
        .map_or((tokens[size_index], None), |(size, line)| {
            (size, Some(line))
        });
    let LengthValue::Computed(size) = length(size_token)? else {
        return None;
    };
    if size.length_type() != LengthType::Fixed || size.value() <= 0.0 {
        return None;
    }
    let mut family_start = size_index + 1;
    let mut line_height = inline_line_height.and_then(|value| {
        if value == "normal" {
            Some(LineHeight::Normal)
        } else if let Ok(number) = value.parse() {
            Some(LineHeight::Number(number))
        } else {
            length(value).and_then(|value| match value {
                LengthValue::Computed(value) if value.is_percent() => {
                    Some(LineHeight::Percentage(value.value()))
                }
                LengthValue::Computed(value) if value.length_type() == LengthType::Fixed => {
                    Some(LineHeight::Length(value.value()))
                }
                _ => None,
            })
        }
    });
    if inline_line_height.is_none() && tokens.get(family_start) == Some(&"/") {
        family_start += 1;
        let token = *tokens.get(family_start)?;
        line_height = if token == "normal" {
            Some(LineHeight::Normal)
        } else if let Ok(number) = token.parse() {
            Some(LineHeight::Number(number))
        } else {
            length(token).and_then(|value| match value {
                LengthValue::Computed(value) if value.is_percent() => {
                    Some(LineHeight::Percentage(value.value()))
                }
                LengthValue::Computed(value) if value.length_type() == LengthType::Fixed => {
                    Some(LineHeight::Length(value.value()))
                }
                _ => None,
            })
        };
        line_height?;
        family_start += 1;
    }
    let family = font_family_list(&tokens.get(family_start..)?.join(" "))?;
    let mut shorthand = FontShorthand::new(size.value(), family)?;
    shorthand.line_height = line_height.unwrap_or(LineHeight::Normal);
    for token in &tokens[..size_index] {
        match *token {
            "normal" => {}
            "italic" => shorthand.style = FontStyleEnum::Italic,
            "oblique" => shorthand.style = FontStyleEnum::Oblique(14.0),
            "small-caps" => shorthand.variant_caps = FontVariantCaps::SmallCaps,
            "bold" => shorthand.weight = FontWeight::BOLD,
            value if value.ends_with('%') => {
                shorthand.stretch = FontStretch(value.trim_end_matches('%').parse().ok()?);
            }
            value => shorthand.weight = FontWeight(value.parse().ok()?),
        }
    }
    Some(shorthand)
}

fn parse_typography_literal(property: StyleProperty, input: &str) -> Option<StyleValue> {
    use StyleProperty as P;
    let input = input.trim();
    let value = match property {
        P::Direction => TypographyValue::Direction(match input {
            "ltr" => Direction::Ltr,
            "rtl" => Direction::Rtl,
            _ => return None,
        }),
        P::FontKerning => TypographyValue::FontKerning(match input {
            "auto" => FontKerning::Auto,
            "normal" => FontKerning::Normal,
            "none" => FontKerning::None,
            _ => return None,
        }),
        P::FontOpticalSizing => TypographyValue::FontOpticalSizing(match input {
            "auto" => FontOpticalSizing::Auto,
            "none" => FontOpticalSizing::None,
            _ => return None,
        }),
        P::FontPalette => TypographyValue::FontPalette(match input {
            "normal" => FontPalette::Normal,
            "light" => FontPalette::Light,
            "dark" => FontPalette::Dark,
            value if value.starts_with("--") => FontPalette::Custom(value.to_owned()),
            _ => return None,
        }),
        P::FontSizeAdjust => TypographyValue::FontSizeAdjust(match input {
            "none" => FontSizeAdjust::None,
            "from-font" => FontSizeAdjust::FromFont,
            value => {
                let mut parts = value.split_whitespace();
                let first = parts.next()?;
                let (basis, ratio) = if let Ok(ratio) = first.parse::<f32>() {
                    ("ex-height", ratio)
                } else {
                    (first, parts.next()?.parse().ok()?)
                };
                if !ratio.is_finite() || ratio < 0.0 || parts.next().is_some() {
                    return None;
                }
                match basis {
                    "ex-height" => FontSizeAdjust::ExHeight(ratio),
                    "cap-height" => FontSizeAdjust::CapHeight(ratio),
                    "ch-width" => FontSizeAdjust::ChWidth(ratio),
                    "ic-width" => FontSizeAdjust::IcWidth(ratio),
                    "ic-height" => FontSizeAdjust::IcHeight(ratio),
                    _ => return None,
                }
            }
        }),
        P::FontStretch => TypographyValue::FontStretch(FontStretch(match input {
            "ultra-condensed" => 50.0,
            "extra-condensed" => 62.5,
            "condensed" => 75.0,
            "semi-condensed" => 87.5,
            "normal" => 100.0,
            "semi-expanded" => 112.5,
            "expanded" => 125.0,
            "extra-expanded" => 150.0,
            "ultra-expanded" => 200.0,
            value => value.strip_suffix('%')?.parse().ok()?,
        })),
        P::FontStyle => TypographyValue::FontStyle(match input {
            "normal" => FontStyleEnum::Normal,
            "italic" => FontStyleEnum::Italic,
            "oblique" => FontStyleEnum::Oblique(14.0),
            value if value.starts_with("oblique ") => {
                FontStyleEnum::Oblique(value[8..].trim().trim_end_matches("deg").parse().ok()?)
            }
            _ => return None,
        }),
        P::FontVariantLigatures => TypographyValue::FontVariantLigatures(font_ligatures(input)?),
        P::FontVariantCaps => TypographyValue::FontVariantCaps(match input {
            "normal" => FontVariantCaps::Normal,
            "small-caps" => FontVariantCaps::SmallCaps,
            "all-small-caps" => FontVariantCaps::AllSmallCaps,
            "petite-caps" => FontVariantCaps::PetiteCaps,
            "all-petite-caps" => FontVariantCaps::AllPetiteCaps,
            "unicase" => FontVariantCaps::Unicase,
            "titling-caps" => FontVariantCaps::TitlingCaps,
            _ => return None,
        }),
        P::FontVariantEastAsian => TypographyValue::FontVariantEastAsian(font_east_asian(input)?),
        P::FontVariantNumeric => TypographyValue::FontVariantNumeric(font_numeric(input)?),
        P::FontVariantAlternates => TypographyValue::FontVariantAlternates(match input {
            "normal" => FontVariantAlternates::Normal,
            "historical-forms" => FontVariantAlternates::HistoricalForms,
            _ => return None,
        }),
        P::FontVariantPosition => TypographyValue::FontVariantPosition(match input {
            "normal" => FontVariantPosition::Normal,
            "sub" => FontVariantPosition::Sub,
            "super" => FontVariantPosition::Super,
            _ => return None,
        }),
        P::FontVariantEmoji => TypographyValue::FontVariantEmoji(match input {
            "normal" => FontVariantEmoji::Normal,
            "text" => FontVariantEmoji::Text,
            "emoji" => FontVariantEmoji::Emoji,
            "unicode" => FontVariantEmoji::Unicode,
            _ => return None,
        }),
        P::FontSynthesisWeight
        | P::FontSynthesisStyle
        | P::FontSynthesisSmallCaps
        | P::FontSynthesisPosition => TypographyValue::FontSynthesis(match input {
            "auto" => FontSynthesis::Auto,
            "none" => FontSynthesis::None,
            _ => return None,
        }),
        P::FontFeatureSettings => TypographyValue::OpenTypeFeatures(feature_settings(input)?),
        P::FontVariationSettings => TypographyValue::FontVariations(variation_settings(input)?),
        P::FontLanguageOverride => TypographyValue::FontLanguageOverride(if input == "normal" {
            FontLanguageOverride::NORMAL
        } else {
            FontLanguageOverride::tag(input.trim_matches(['\'', '"']))?
        }),
        P::LineHeight => TypographyValue::LineHeight(if input == "normal" {
            LineHeight::Normal
        } else if let Ok(value) = input.parse::<f32>() {
            LineHeight::Number(value)
        } else {
            match length(input)? {
                LengthValue::Computed(value) if value.is_percent() => {
                    LineHeight::Percentage(value.value())
                }
                LengthValue::Computed(value) if value.length_type() == LengthType::Fixed => {
                    LineHeight::Length(value.value())
                }
                _ => return None,
            }
        }),
        P::TextAlign => TypographyValue::TextAlign(match input {
            "left" => TextAlign::Left,
            "right" => TextAlign::Right,
            "center" => TextAlign::Center,
            "justify" => TextAlign::Justify,
            "start" | "match-parent" => TextAlign::Start,
            "end" => TextAlign::End,
            _ => return None,
        }),
        P::TextAlignLast => TypographyValue::TextAlignLast(match input {
            "auto" => TextAlignLast::Auto,
            "start" | "match-parent" => TextAlignLast::Start,
            "end" => TextAlignLast::End,
            "left" => TextAlignLast::Left,
            "right" => TextAlignLast::Right,
            "center" => TextAlignLast::Center,
            "justify" => TextAlignLast::Justify,
            _ => return None,
        }),
        P::TextJustify => TypographyValue::TextJustify(match input {
            "auto" => TextJustify::Auto,
            "none" => TextJustify::None,
            "inter-word" => TextJustify::InterWord,
            "inter-character" => TextJustify::InterCharacter,
            _ => return None,
        }),
        P::WordBreak => TypographyValue::WordBreak(match input {
            "normal" => WordBreak::Normal,
            "break-all" => WordBreak::BreakAll,
            "keep-all" => WordBreak::KeepAll,
            "break-word" => WordBreak::BreakWord,
            "auto-phrase" => WordBreak::AutoPhrase,
            _ => return None,
        }),
        P::OverflowWrap | P::WordWrap => TypographyValue::OverflowWrap(match input {
            "normal" => OverflowWrap::Normal,
            "break-word" => OverflowWrap::BreakWord,
            "anywhere" => OverflowWrap::Anywhere,
            _ => return None,
        }),
        P::LineBreak => TypographyValue::LineBreak(match input {
            "auto" => LineBreak::Auto,
            "loose" => LineBreak::Loose,
            "normal" => LineBreak::Normal,
            "strict" => LineBreak::Strict,
            "anywhere" => LineBreak::Anywhere,
            "after-white-space" => LineBreak::AfterWhiteSpace,
            _ => return None,
        }),
        P::Hyphens => TypographyValue::Hyphens(match input {
            "none" => Hyphens::None,
            "manual" => Hyphens::Manual,
            "auto" => Hyphens::Auto,
            _ => return None,
        }),
        P::HyphenateLimitChars => {
            let parts: Vec<_> = input.split_whitespace().collect();
            let number = |value: &str, default| {
                if value == "auto" {
                    Some(default)
                } else {
                    value.parse().ok()
                }
            };
            let value = match parts.as_slice() {
                ["auto"] => HyphenationLimits::AUTO,
                [word] => HyphenationLimits::new(number(word, 5)?, 2, 2),
                [word, edge] => {
                    HyphenationLimits::new(number(word, 5)?, number(edge, 2)?, number(edge, 2)?)
                }
                [word, before, after] => {
                    HyphenationLimits::new(number(word, 5)?, number(before, 2)?, number(after, 2)?)
                }
                _ => return None,
            };
            TypographyValue::HyphenationLimits(value)
        }
        P::HyphenateCharacter => TypographyValue::HyphenateCharacter(if input == "auto" {
            HyphenateCharacter::AUTO
        } else {
            HyphenateCharacter::character(input.trim_matches(['\'', '"']))?
        }),
        P::WhiteSpaceCollapse => TypographyValue::WhiteSpaceCollapse(match input {
            "collapse" => WhiteSpaceCollapse::Collapse,
            "preserve" => WhiteSpaceCollapse::Preserve,
            "preserve-breaks" => WhiteSpaceCollapse::PreserveBreaks,
            "break-spaces" => WhiteSpaceCollapse::BreakSpaces,
            _ => return None,
        }),
        P::TextWrapMode => TypographyValue::TextWrapMode(match input {
            "wrap" => TextWrapMode::Wrap,
            "nowrap" => TextWrapMode::Nowrap,
            _ => return None,
        }),
        P::TextWrapStyle => TypographyValue::TextWrapStyle(match input {
            "auto" => TextWrapStyle::Auto,
            "balance" => TextWrapStyle::Balance,
            "pretty" => TextWrapStyle::Pretty,
            "stable" => TextWrapStyle::Stable,
            _ => return None,
        }),
        P::TextAutospace => TypographyValue::TextAutospace(match input {
            "no-autospace" => TextAutospace::NoAutospace,
            "normal" => TextAutospace::Normal,
            _ => return None,
        }),
        P::TextSpacingTrim => TypographyValue::TextSpacingTrim(match input {
            "normal" => TextSpacingTrim::Normal,
            "space-all" => TextSpacingTrim::SpaceAll,
            "space-first" => TextSpacingTrim::SpaceFirst,
            "trim-start" => TextSpacingTrim::TrimStart,
            _ => return None,
        }),
        P::TabSize => TypographyValue::TabSize(if let Ok(value) = input.parse::<u32>() {
            TabSize::Spaces(value)
        } else {
            let LengthValue::Computed(value) = length(input)? else {
                return None;
            };
            if value.length_type() != LengthType::Fixed {
                return None;
            }
            TabSize::Length(value.value())
        }),
        P::TextTransform => TypographyValue::TextTransform(match input {
            "none" => TextTransform::None,
            "capitalize" => TextTransform::Capitalize,
            "uppercase" => TextTransform::Uppercase,
            "lowercase" => TextTransform::Lowercase,
            "full-width" => TextTransform::FullWidth,
            "full-size-kana" => TextTransform::FullSizeKana,
            "math-auto" => TextTransform::MathAuto,
            _ => return None,
        }),
        P::TextDecorationLine => TypographyValue::TextDecorationLine(decoration_line(input)?),
        P::TextDecorationStyle => TypographyValue::TextDecorationStyle(match input {
            "solid" => TextDecorationStyle::Solid,
            "double" => TextDecorationStyle::Double,
            "dotted" => TextDecorationStyle::Dotted,
            "dashed" => TextDecorationStyle::Dashed,
            "wavy" => TextDecorationStyle::Wavy,
            _ => return None,
        }),
        P::TextDecorationColor | P::TextEmphasisColor => {
            TypographyValue::StyleColor(if input.eq_ignore_ascii_case("currentcolor") {
                StyleColor::CurrentColor
            } else {
                StyleColor::Resolved(color(input)?)
            })
        }
        P::TextDecorationThickness => TypographyValue::TextDecorationThickness(match input {
            "auto" => TextDecorationThickness::Auto,
            "from-font" => TextDecorationThickness::FromFont,
            value => {
                let LengthValue::Computed(value) = length(value)? else {
                    return None;
                };
                TextDecorationThickness::Length(value.value())
            }
        }),
        P::TextDecorationSkipInk => TypographyValue::TextDecorationSkipInk(match input {
            "none" => TextDecorationSkipInk::None,
            "auto" => TextDecorationSkipInk::Auto,
            "all" => TextDecorationSkipInk::All,
            _ => return None,
        }),
        P::TextUnderlinePosition => TypographyValue::TextUnderlinePosition(match input {
            "auto" | "from-font" => TextUnderlinePosition::Auto,
            "under" => TextUnderlinePosition::Under,
            "left" => TextUnderlinePosition::Left,
            "right" => TextUnderlinePosition::Right,
            _ => return None,
        }),
        P::TextEmphasisStyle => TypographyValue::TextEmphasisStyle(emphasis_style(input)?),
        P::TextEmphasisPosition => {
            let mut position = TextEmphasisPosition::INITIAL;
            for token in input.split_whitespace() {
                match token {
                    "over" => position.over = true,
                    "under" => position.over = false,
                    "right" => position.right = true,
                    "left" => position.right = false,
                    _ => return None,
                }
            }
            TypographyValue::TextEmphasisPosition(position)
        }
        P::TextShadow => TypographyValue::TextShadows(text_shadows(input)?),
        P::TextOverflow => TypographyValue::TextOverflow(match input {
            "clip" => TextOverflow::Clip,
            "ellipsis" => TextOverflow::Ellipsis,
            _ => return None,
        }),
        P::TextSizeAdjust => TypographyValue::TextSizeAdjust(match input {
            "auto" => TextSizeAdjust::Auto,
            "none" => TextSizeAdjust::None,
            value => {
                let percentage: f32 = value.strip_suffix('%')?.parse().ok()?;
                if !percentage.is_finite() || percentage < 0.0 {
                    return None;
                }
                TextSizeAdjust::Percentage(percentage)
            }
        }),
        P::TextCombineUpright => TypographyValue::TextCombineUpright(match input {
            "none" => TextCombineUpright::None,
            "all" => TextCombineUpright::All,
            _ => return None,
        }),
        P::WritingMode => TypographyValue::WritingMode(match input {
            "horizontal-tb" => WritingMode::HorizontalTb,
            "vertical-rl" => WritingMode::VerticalRl,
            "vertical-lr" => WritingMode::VerticalLr,
            "sideways-rl" => WritingMode::SidewaysRl,
            "sideways-lr" => WritingMode::SidewaysLr,
            _ => return None,
        }),
        P::TextOrientation => TypographyValue::TextOrientation(match input {
            "mixed" => TextOrientation::Mixed,
            "upright" => TextOrientation::Upright,
            "sideways" => TextOrientation::Sideways,
            _ => return None,
        }),
        P::UnicodeBidi => TypographyValue::UnicodeBidi(match input {
            "normal" => UnicodeBidi::Normal,
            "embed" => UnicodeBidi::Embed,
            "bidi-override" => UnicodeBidi::Override,
            "isolate" => UnicodeBidi::Isolate,
            "isolate-override" => UnicodeBidi::IsolateOverride,
            "plaintext" => UnicodeBidi::Plaintext,
            _ => return None,
        }),
        P::VerticalAlign => TypographyValue::VerticalAlign(match input {
            "baseline" => VerticalAlign::Baseline,
            "sub" => VerticalAlign::Sub,
            "super" => VerticalAlign::Super,
            "text-top" => VerticalAlign::TextTop,
            "text-bottom" => VerticalAlign::TextBottom,
            "middle" => VerticalAlign::Middle,
            "top" => VerticalAlign::Top,
            "bottom" => VerticalAlign::Bottom,
            value if value.ends_with('%') => {
                VerticalAlign::Percentage(value.trim_end_matches('%').parse().ok()?)
            }
            value => {
                let LengthValue::Computed(value) = length(value)? else {
                    return None;
                };
                VerticalAlign::Length(value.value())
            }
        }),
        P::RubyAlign => TypographyValue::RubyAlign(match input {
            "space-around" => RubyAlign::SpaceAround,
            "start" => RubyAlign::Start,
            "center" => RubyAlign::Center,
            "space-between" => RubyAlign::SpaceBetween,
            _ => return None,
        }),
        P::RubyPosition => TypographyValue::RubyPosition(match input {
            "over" => RubyPosition::Over,
            "under" => RubyPosition::Under,
            _ => return None,
        }),
        P::RubyOverhang => TypographyValue::RubyOverhang(match input {
            "auto" => RubyOverhang::Auto,
            "none" => RubyOverhang::None,
            _ => return None,
        }),
        P::HangingPunctuation => {
            if input == "none" {
                TypographyValue::HangingPunctuation(HangingPunctuation::NONE)
            } else {
                let mut value = HangingPunctuation::NONE;
                for token in input.split_whitespace() {
                    match token {
                        "first" => value.first = true,
                        "last" => value.last = true,
                        "force-end" if !value.allow_end => value.force_end = true,
                        "allow-end" if !value.force_end => value.allow_end = true,
                        _ => return None,
                    }
                }
                TypographyValue::HangingPunctuation(value)
            }
        }
        P::InitialLetter => TypographyValue::InitialLetter(if input == "normal" {
            InitialLetterValue::Normal
        } else {
            let mut parts = input.split_whitespace();
            let size = parts.next()?.parse().ok()?;
            let sink = parts.next().map(str::parse).transpose().ok()?;
            if parts.next().is_some() {
                return None;
            }
            InitialLetterValue::Value(crate::InitialLetter { size, sink })
        }),
        P::TextRendering => {
            TypographyValue::TextRendering(match input.to_ascii_lowercase().as_str() {
                "auto" => TextRendering::Auto,
                "optimizespeed" => TextRendering::OptimizeSpeed,
                "optimizelegibility" => TextRendering::OptimizeLegibility,
                "geometricprecision" => TextRendering::GeometricPrecision,
                _ => return None,
            })
        }
        P::WebkitFontSmoothing => TypographyValue::FontSmoothing(match input {
            "auto" => FontSmoothing::Auto,
            "none" => FontSmoothing::None,
            "antialiased" => FontSmoothing::Antialiased,
            "subpixel-antialiased" => FontSmoothing::SubpixelAntialiased,
            _ => return None,
        }),
        P::LineClamp => TypographyValue::LineClamp(match input {
            "none" => LineClamp::None,
            "auto" => LineClamp::Auto,
            value => LineClamp::Lines(value.parse().ok()?),
        }),
        P::BlockEllipsis => TypographyValue::BlockEllipsis(match input {
            "auto" => BlockEllipsis::Auto,
            "no-ellipsis" => BlockEllipsis::NoEllipsis,
            value => BlockEllipsis::String(value.trim_matches(['\'', '"']).to_owned()),
        }),
        P::TextBoxEdge => {
            let edge = |token| match token {
                "auto" => Some(crate::TextBoxEdgeKeyword::Auto),
                "text" => Some(crate::TextBoxEdgeKeyword::Text),
                "cap" => Some(crate::TextBoxEdgeKeyword::Cap),
                "ex" => Some(crate::TextBoxEdgeKeyword::Ex),
                "ideographic" => Some(crate::TextBoxEdgeKeyword::Ideographic),
                "ideographic-ink" => Some(crate::TextBoxEdgeKeyword::IdeographicInk),
                "alphabetic" => Some(crate::TextBoxEdgeKeyword::Alphabetic),
                _ => None,
            };
            let parts: Vec<_> = input.split_whitespace().collect();
            TypographyValue::TextBoxEdge(match parts.as_slice() {
                [one] => TextBoxEdge {
                    over: edge(one)?,
                    under: edge(one)?,
                },
                [over, under] => TextBoxEdge {
                    over: edge(over)?,
                    under: edge(under)?,
                },
                _ => return None,
            })
        }
        P::TextBoxTrim => TypographyValue::TextBoxTrim(match input {
            "none" => TextBoxTrim::None,
            "trim-start" => TextBoxTrim::TrimStart,
            "trim-end" => TextBoxTrim::TrimEnd,
            "trim-both" => TextBoxTrim::TrimBoth,
            _ => return None,
        }),
        P::TextBox => {
            if input == "normal" {
                TypographyValue::TextBox(TextBoxShorthand::default())
            } else {
                let mut tokens = input.split_whitespace();
                let trim = match tokens.next()? {
                    "none" => TextBoxTrim::None,
                    "trim-start" => TextBoxTrim::TrimStart,
                    "trim-end" => TextBoxTrim::TrimEnd,
                    "trim-both" => TextBoxTrim::TrimBoth,
                    _ => return None,
                };
                let edge_input = tokens.collect::<Vec<_>>().join(" ");
                let StyleValue::Typography(TypographyValue::TextBoxEdge(edge)) =
                    parse_typography_literal(P::TextBoxEdge, &edge_input)?
                else {
                    return None;
                };
                TypographyValue::TextBox(TextBoxShorthand { trim, edge })
            }
        }
        P::Font => TypographyValue::Font(parse_font_shorthand(input)?),
        P::FontVariant => {
            if input == "normal" || input == "none" {
                TypographyValue::FontVariant(FontVariantShorthand::default())
            } else {
                let mut value = FontVariantShorthand::default();
                for token in input.split_whitespace() {
                    if let Some(parsed) = font_ligatures(token) {
                        value.ligatures = parsed;
                        continue;
                    }
                    if let Some(parsed) = font_numeric(token) {
                        value.numeric = parsed;
                        continue;
                    }
                    if let Some(parsed) = font_east_asian(token) {
                        value.east_asian = parsed;
                        continue;
                    }
                    match token {
                        "small-caps" => value.caps = FontVariantCaps::SmallCaps,
                        "all-small-caps" => value.caps = FontVariantCaps::AllSmallCaps,
                        "petite-caps" => value.caps = FontVariantCaps::PetiteCaps,
                        "all-petite-caps" => value.caps = FontVariantCaps::AllPetiteCaps,
                        "unicase" => value.caps = FontVariantCaps::Unicase,
                        "titling-caps" => value.caps = FontVariantCaps::TitlingCaps,
                        "sub" => value.position = FontVariantPosition::Sub,
                        "super" => value.position = FontVariantPosition::Super,
                        "historical-forms" => {
                            value.alternates = FontVariantAlternates::HistoricalForms
                        }
                        "text" => value.emoji = FontVariantEmoji::Text,
                        "emoji" => value.emoji = FontVariantEmoji::Emoji,
                        "unicode" => value.emoji = FontVariantEmoji::Unicode,
                        _ => return None,
                    }
                }
                TypographyValue::FontVariant(value)
            }
        }
        P::FontSynthesis => TypographyValue::FontSynthesisShorthand(match input {
            "auto" => FontSynthesisShorthand::AUTO,
            "none" => FontSynthesisShorthand::NONE,
            value => {
                let mut result = FontSynthesisShorthand::NONE;
                for token in value.split_whitespace() {
                    match token {
                        "weight" => result.weight = FontSynthesis::Auto,
                        "style" => result.style = FontSynthesis::Auto,
                        "small-caps" => result.small_caps = FontSynthesis::Auto,
                        "position" => result.position = FontSynthesis::Auto,
                        _ => return None,
                    }
                }
                result
            }
        }),
        P::WhiteSpace => {
            let value = match input {
                "normal" => WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::Collapse,
                    wrap: TextWrapMode::Wrap,
                },
                "pre" => WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::Preserve,
                    wrap: TextWrapMode::Nowrap,
                },
                "pre-wrap" => WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::Preserve,
                    wrap: TextWrapMode::Wrap,
                },
                "pre-line" => WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::PreserveBreaks,
                    wrap: TextWrapMode::Wrap,
                },
                "nowrap" => WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::Collapse,
                    wrap: TextWrapMode::Nowrap,
                },
                "break-spaces" => WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::BreakSpaces,
                    wrap: TextWrapMode::Wrap,
                },
                _ => return None,
            };
            TypographyValue::WhiteSpace(value)
        }
        P::TextWrap => {
            let (mode, style) = match input {
                "wrap" => (TextWrapMode::Wrap, TextWrapStyle::Auto),
                "nowrap" => (TextWrapMode::Nowrap, TextWrapStyle::Auto),
                "balance" => (TextWrapMode::Wrap, TextWrapStyle::Balance),
                "pretty" => (TextWrapMode::Wrap, TextWrapStyle::Pretty),
                "stable" => (TextWrapMode::Wrap, TextWrapStyle::Stable),
                _ => return None,
            };
            TypographyValue::TextWrap(TextWrapShorthand { mode, style })
        }
        P::TextDecoration => {
            let mut value = TextDecorationShorthand::default();
            for token in input.split_whitespace() {
                if let Some(line) = decoration_line(token) {
                    value.line = TextDecorationLine(value.line.0 | line.0);
                    continue;
                }
                if token.eq_ignore_ascii_case("currentcolor") {
                    value.color = StyleColor::CurrentColor;
                    continue;
                }
                if let Some(parsed) = color(token) {
                    value.color = StyleColor::Resolved(parsed);
                    continue;
                }
                match token {
                    "solid" => value.style = TextDecorationStyle::Solid,
                    "double" => value.style = TextDecorationStyle::Double,
                    "dotted" => value.style = TextDecorationStyle::Dotted,
                    "dashed" => value.style = TextDecorationStyle::Dashed,
                    "wavy" => value.style = TextDecorationStyle::Wavy,
                    "auto" => value.thickness = TextDecorationThickness::Auto,
                    "from-font" => value.thickness = TextDecorationThickness::FromFont,
                    token => {
                        let LengthValue::Computed(length) = length(token)? else {
                            return None;
                        };
                        value.thickness = TextDecorationThickness::Length(length.value());
                    }
                }
            }
            TypographyValue::TextDecoration(value)
        }
        P::TextEmphasis => {
            let mut tokens: Vec<_> = input.split_whitespace().collect();
            let emphasis_color = tokens.last().and_then(|token| {
                if token.eq_ignore_ascii_case("currentcolor") {
                    Some(StyleColor::CurrentColor)
                } else {
                    color(token).map(StyleColor::Resolved)
                }
            });
            if emphasis_color.is_some() {
                tokens.pop();
            }
            TypographyValue::TextEmphasis(TextEmphasisShorthand {
                style: emphasis_style(&tokens.join(" "))?,
                color: emphasis_color.unwrap_or(StyleColor::CurrentColor),
                position: TextEmphasisPosition::INITIAL,
            })
        }
        _ => return None,
    };
    Some(StyleValue::Typography(value))
}

pub fn parse_literal(property: StyleProperty, input: &str) -> Result<StyleValue, LiteralError> {
    use StyleProperty as P;
    let result = match property {
        P::Display => match input {
            "none" => Some(Display::None),
            "inline" => Some(Display::Inline),
            "block" => Some(Display::Block),
            "flex" => Some(Display::Flex),
            "grid" => Some(Display::Grid),
            "inline-block" => Some(Display::InlineBlock),
            "inline-flex" => Some(Display::InlineFlex),
            "inline-grid" => Some(Display::InlineGrid),
            "flow-root" => Some(Display::FlowRoot),
            "table" => Some(Display::Table),
            "list-item" => Some(Display::ListItem),
            "contents" => Some(Display::Contents),
            _ => None,
        }
        .map(StyleValue::Display),
        P::Position => match input {
            "static" => Some(Position::Static),
            "relative" => Some(Position::Relative),
            "absolute" => Some(Position::Absolute),
            "fixed" => Some(Position::Fixed),
            "sticky" => Some(Position::Sticky),
            _ => None,
        }
        .map(StyleValue::Position),
        P::Overflow => overflow_literal(input).map(StyleValue::Overflow),
        P::Width
        | P::Height
        | P::MinWidth
        | P::MinHeight
        | P::MaxWidth
        | P::MaxHeight
        | P::Left
        | P::Top
        | P::Right
        | P::Bottom
        | P::MarginTop
        | P::MarginRight
        | P::MarginBottom
        | P::MarginLeft
        | P::PaddingTop
        | P::PaddingRight
        | P::PaddingBottom
        | P::PaddingLeft
        | P::FlexBasis
        | P::RowGap
        | P::ColumnGap
        | P::FontSize
        | P::TextIndent
        | P::TextUnderlineOffset => length(input).map(StyleValue::Length),
        P::Margin | P::Padding => edges(input).map(StyleValue::Edges),
        P::BackgroundColor | P::Color => color(input).map(StyleValue::Color),
        P::Opacity | P::FlexGrow | P::FlexShrink | P::LetterSpacing | P::WordSpacing => {
            if input == "normal" {
                Some(StyleValue::Number(0.0))
            } else if matches!(property, P::LetterSpacing | P::WordSpacing) {
                length(input).and_then(|value| match value {
                    LengthValue::Computed(value) if value.length_type() == LengthType::Fixed => {
                        Some(StyleValue::Number(value.value()))
                    }
                    _ => None,
                })
            } else {
                input.parse().ok().map(StyleValue::Number)
            }
        }
        P::ZIndex => input.parse().ok().map(StyleValue::Integer),
        P::FlexDirection => match input {
            "row" => Some(FlexDirection::Row),
            "row-reverse" => Some(FlexDirection::RowReverse),
            "column" => Some(FlexDirection::Column),
            "column-reverse" => Some(FlexDirection::ColumnReverse),
            _ => None,
        }
        .map(StyleValue::FlexDirection),
        P::FlexWrap => match input {
            "nowrap" => Some(FlexWrap::Nowrap),
            "wrap" => Some(FlexWrap::Wrap),
            "wrap-reverse" => Some(FlexWrap::WrapReverse),
            _ => None,
        }
        .map(StyleValue::FlexWrap),
        P::AlignItems => item_alignment_literal(input).map(StyleValue::ItemAlignment),
        P::JustifyContent => content_alignment_literal(input).map(StyleValue::ContentAlignment),
        P::Gap => {
            let v: Vec<_> = input
                .split_whitespace()
                .map(length)
                .collect::<Option<_>>()
                .unwrap_or_default();
            match v.as_slice() {
                [a] => Some(StyleValue::Gap(Gap {
                    row: *a,
                    column: *a,
                })),
                [r, c] => Some(StyleValue::Gap(Gap {
                    row: *r,
                    column: *c,
                })),
                _ => None,
            }
        }
        P::FontFamily => font_family_list(input).map(StyleValue::FontFamily),
        P::FontWeight => match input {
            "normal" => Some(FontWeight::NORMAL),
            "bold" => Some(FontWeight::BOLD),
            value => value
                .parse::<f32>()
                .ok()
                .filter(|v| (1.0..=1000.0).contains(v))
                .map(FontWeight),
        }
        .map(StyleValue::FontWeight),
        P::Border | P::BorderTop | P::BorderRight | P::BorderBottom | P::BorderLeft => {
            border(input).map(StyleValue::Border)
        }
        P::BorderRadius => edges(input).map(|v| StyleValue::CornerRadii(CornerRadii(v))),
        P::Cursor => match input {
            "auto" => Some(Cursor::Auto),
            "default" => Some(Cursor::Default),
            "pointer" => Some(Cursor::Pointer),
            "text" => Some(Cursor::Text),
            "move" => Some(Cursor::Move),
            "not-allowed" => Some(Cursor::NotAllowed),
            _ => None,
        }
        .map(StyleValue::Cursor),
        P::ListStyleType => match input {
            "none" => Some(ListStyleType::None),
            "disc" => Some(ListStyleType::Disc),
            _ => None,
        }
        .map(StyleValue::ListStyle),
        P::Transform => {
            if input == "none" {
                Some(StyleValue::Transform(TransformList::default()))
            } else {
                None
            }
        }
        P::PointerEvents => match input {
            "auto" => Some(PointerEvents::Auto),
            "none" => Some(PointerEvents::None),
            _ => None,
        }
        .map(StyleValue::PointerEvents),
        P::ColumnCount => {
            if input == "auto" {
                Some(StyleValue::Renderer(RendererStyleValue::ColumnCount(None)))
            } else {
                input
                    .parse::<u32>()
                    .ok()
                    .filter(|count| *count > 0)
                    .map(|count| StyleValue::Renderer(RendererStyleValue::ColumnCount(Some(count))))
            }
        }
        P::Visibility => match input {
            "visible" => Some(Visibility::Visible),
            "hidden" => Some(Visibility::Hidden),
            "collapse" => Some(Visibility::Collapse),
            _ => None,
        }
        .map(|value| StyleValue::Renderer(RendererStyleValue::Visibility(value))),
        _ => parse_typography_literal(property, input)
            .or_else(|| parse_renderer_author_literal(property, input)),
    };
    result
        .filter(|value| !matches!(value, StyleValue::Number(value) if !value.is_finite()))
        .ok_or_else(|| invalid(property, input))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyTypeError {
    pub property: StyleProperty,
    pub expected: ValueKind,
}

fn apply_typography_to_computed(
    style: &mut ComputedStyle,
    property: StyleProperty,
    value: &TypographyValue,
) -> bool {
    use StyleProperty as P;
    match (property, value) {
        (P::Direction, TypographyValue::Direction(value)) => style.fields.direction = *value,
        (P::FontKerning, TypographyValue::FontKerning(value)) => style.fields.font_kerning = *value,
        (P::FontOpticalSizing, TypographyValue::FontOpticalSizing(value)) => {
            style.fields.font_optical_sizing = *value
        }
        (P::FontPalette, TypographyValue::FontPalette(value)) => {
            style.fields.font_palette = value.clone()
        }
        (P::FontSizeAdjust, TypographyValue::FontSizeAdjust(value)) => {
            style.fields.font_size_adjust = *value
        }
        (P::FontStretch, TypographyValue::FontStretch(value)) => {
            style.fields.font_stretch = FontStretch(value.0.clamp(50.0, 200.0))
        }
        (P::FontStyle, TypographyValue::FontStyle(value)) => {
            style.fields.font_style = match value {
                FontStyleEnum::Oblique(angle) => FontStyleEnum::Oblique(angle.clamp(-90.0, 90.0)),
                value => *value,
            }
        }
        (P::FontVariantLigatures, TypographyValue::FontVariantLigatures(value)) => {
            style.fields.font_variant_ligatures = *value
        }
        (P::FontVariantCaps, TypographyValue::FontVariantCaps(value)) => {
            style.fields.font_variant_caps = *value
        }
        (P::FontVariantEastAsian, TypographyValue::FontVariantEastAsian(value)) => {
            style.fields.font_variant_east_asian = *value
        }
        (P::FontVariantNumeric, TypographyValue::FontVariantNumeric(value)) => {
            style.fields.font_variant_numeric = *value
        }
        (P::FontVariantAlternates, TypographyValue::FontVariantAlternates(value)) => {
            style.fields.font_variant_alternates = *value
        }
        (P::FontVariantPosition, TypographyValue::FontVariantPosition(value)) => {
            style.fields.font_variant_position = *value
        }
        (P::FontVariantEmoji, TypographyValue::FontVariantEmoji(value)) => {
            style.fields.font_variant_emoji = *value
        }
        (P::FontSynthesisWeight, TypographyValue::FontSynthesis(value)) => {
            style.fields.font_synthesis_weight = *value
        }
        (P::FontSynthesisStyle, TypographyValue::FontSynthesis(value)) => {
            style.fields.font_synthesis_style = *value
        }
        (P::FontSynthesisSmallCaps, TypographyValue::FontSynthesis(value)) => {
            style.fields.font_synthesis_small_caps = *value
        }
        (P::FontSynthesisPosition, TypographyValue::FontSynthesis(value)) => {
            style.fields.font_synthesis_position = *value
        }
        (P::FontFeatureSettings, TypographyValue::OpenTypeFeatures(value)) => {
            style.fields.font_feature_settings = value.0.clone()
        }
        (P::FontVariationSettings, TypographyValue::FontVariations(value)) => {
            if !value.0.iter().all(|axis| axis.value.is_finite()) {
                return false;
            }
            style.fields.font_variation_settings = value.0.clone()
        }
        (P::FontLanguageOverride, TypographyValue::FontLanguageOverride(value)) => {
            style.fields.font_language_override = *value
        }
        (P::LineHeight, TypographyValue::LineHeight(value)) => style.fields.line_height = *value,
        (P::TextAlign, TypographyValue::TextAlign(value)) => style.fields.text_align = *value,
        (P::TextAlignLast, TypographyValue::TextAlignLast(value)) => {
            style.fields.text_align_last = *value
        }
        (P::TextJustify, TypographyValue::TextJustify(value)) => style.fields.text_justify = *value,
        (P::WordBreak, TypographyValue::WordBreak(value)) => style.fields.word_break = *value,
        (P::OverflowWrap | P::WordWrap, TypographyValue::OverflowWrap(value)) => {
            style.fields.overflow_wrap = *value
        }
        (P::LineBreak, TypographyValue::LineBreak(value)) => style.fields.line_break = *value,
        (P::Hyphens, TypographyValue::Hyphens(value)) => style.fields.hyphens = *value,
        (P::HyphenateLimitChars, TypographyValue::HyphenationLimits(value)) => {
            style.fields.hyphenate_limit_chars = (value.word, value.before, value.after)
        }
        (P::HyphenateCharacter, TypographyValue::HyphenateCharacter(value)) => {
            style.fields.hyphenate_character = value.0.clone()
        }
        (P::WhiteSpaceCollapse, TypographyValue::WhiteSpaceCollapse(value)) => {
            style.fields.white_space_collapse = *value;
            style.fields.white_space = normalized_white_space(*value, style.fields.text_wrap_mode);
        }
        (P::TextWrapMode, TypographyValue::TextWrapMode(value)) => {
            style.fields.text_wrap_mode = *value;
            style.fields.white_space =
                normalized_white_space(style.fields.white_space_collapse, *value);
            style.fields.text_wrap = normalized_text_wrap(*value, style.fields.text_wrap_style);
        }
        (P::TextWrapStyle, TypographyValue::TextWrapStyle(value)) => {
            style.fields.text_wrap_style = *value;
            style.fields.text_wrap = normalized_text_wrap(style.fields.text_wrap_mode, *value);
        }
        (P::TextAutospace, TypographyValue::TextAutospace(value)) => {
            style.fields.text_autospace = *value
        }
        (P::TextSpacingTrim, TypographyValue::TextSpacingTrim(value)) => {
            style.fields.text_spacing_trim = *value
        }
        (P::TabSize, TypographyValue::TabSize(value)) => style.fields.tab_size = *value,
        (P::TextTransform, TypographyValue::TextTransform(value)) => {
            style.fields.text_transform = *value
        }
        (P::TextDecorationLine, TypographyValue::TextDecorationLine(value)) => {
            style.fields.text_decoration_line = *value
        }
        (P::TextDecorationStyle, TypographyValue::TextDecorationStyle(value)) => {
            style.fields.text_decoration_style = *value
        }
        (P::TextDecorationThickness, TypographyValue::TextDecorationThickness(value)) => {
            style.fields.text_decoration_thickness = *value
        }
        (P::TextDecorationColor, TypographyValue::StyleColor(value)) => {
            style.fields.text_decoration_color = *value
        }
        (P::TextEmphasisColor, TypographyValue::StyleColor(value)) => {
            style.fields.text_emphasis_color = *value
        }
        (P::TextDecorationSkipInk, TypographyValue::TextDecorationSkipInk(value)) => {
            style.fields.text_decoration_skip_ink = *value
        }
        (P::TextUnderlinePosition, TypographyValue::TextUnderlinePosition(value)) => {
            style.fields.text_underline_position = *value
        }
        (P::TextEmphasisStyle, TypographyValue::TextEmphasisStyle(value)) => {
            style.fields.text_emphasis_mark = value.mark;
            style.fields.text_emphasis_fill = value.fill;
        }
        (P::TextEmphasisPosition, TypographyValue::TextEmphasisPosition(value)) => {
            style.fields.text_emphasis_position = *value
        }
        (P::TextShadow, TypographyValue::TextShadows(value)) => {
            style.fields.text_shadow = value.0.clone()
        }
        (P::TextOverflow, TypographyValue::TextOverflow(value)) => {
            style.fields.text_overflow = *value
        }
        (P::TextSizeAdjust, TypographyValue::TextSizeAdjust(value)) => {
            style.fields.text_size_adjust = *value
        }
        (P::TextCombineUpright, TypographyValue::TextCombineUpright(value)) => {
            style.fields.text_combine_upright = *value
        }
        (P::WritingMode, TypographyValue::WritingMode(value)) => style.fields.writing_mode = *value,
        (P::TextOrientation, TypographyValue::TextOrientation(value)) => {
            style.fields.text_orientation = *value
        }
        (P::UnicodeBidi, TypographyValue::UnicodeBidi(value)) => style.fields.unicode_bidi = *value,
        (P::VerticalAlign, TypographyValue::VerticalAlign(value)) => {
            style.fields.vertical_align = *value
        }
        (P::RubyAlign, TypographyValue::RubyAlign(value)) => style.fields.ruby_align = *value,
        (P::RubyPosition, TypographyValue::RubyPosition(value)) => {
            style.fields.ruby_position = *value
        }
        (P::RubyOverhang, TypographyValue::RubyOverhang(value)) => {
            style.fields.ruby_overhang = *value
        }
        (P::HangingPunctuation, TypographyValue::HangingPunctuation(value)) => {
            style.fields.hanging_punctuation = *value
        }
        (P::InitialLetter, TypographyValue::InitialLetter(value)) => {
            style.fields.initial_letter = match value {
                InitialLetterValue::Normal => None,
                InitialLetterValue::Value(value) => Some(*value),
            }
        }
        (P::TextRendering, TypographyValue::TextRendering(value)) => {
            style.fields.text_rendering = *value
        }
        (P::WebkitFontSmoothing, TypographyValue::FontSmoothing(value)) => {
            style.fields.font_smoothing = *value
        }
        (P::LineClamp, TypographyValue::LineClamp(value)) => style.fields.line_clamp = *value,
        (P::BlockEllipsis, TypographyValue::BlockEllipsis(value)) => {
            style.fields.block_ellipsis = value.clone()
        }
        (P::TextBoxEdge, TypographyValue::TextBoxEdge(value)) => {
            style.fields.text_box_edge = *value
        }
        (P::TextBoxTrim, TypographyValue::TextBoxTrim(value)) => {
            style.fields.text_box_trim = *value
        }
        (P::TextBox, TypographyValue::TextBox(value)) => {
            style.fields.text_box_trim = value.trim;
            style.fields.text_box_edge = value.edge;
        }
        (P::Font, TypographyValue::Font(value)) => {
            style.fields.font_style = value.style;
            style.fields.font_variant_caps = value.variant_caps;
            style.fields.font_weight = value.weight;
            style.fields.font_stretch = value.stretch;
            style.fields.font_size = value.size_px;
            style.fields.line_height = value.line_height;
            style.fields.font_family = value.family.clone();
            // CSS Fonts requires omitted and non-shorthand font longhands to
            // reset atomically when `font` is accepted.
            style.fields.font_variant_ligatures = FontVariantLigatures::default();
            style.fields.font_variant_numeric = FontVariantNumeric::default();
            style.fields.font_variant_east_asian = FontVariantEastAsian::default();
            style.fields.font_variant_alternates = FontVariantAlternates::default();
            style.fields.font_variant_position = FontVariantPosition::default();
            style.fields.font_variant_emoji = FontVariantEmoji::default();
            style.fields.font_optical_sizing = FontOpticalSizing::Auto;
            style.fields.font_size_adjust = FontSizeAdjust::None;
            style.fields.font_kerning = FontKerning::Auto;
            style.fields.font_feature_settings.clear();
            style.fields.font_variation_settings.clear();
            style.fields.font_language_override = FontLanguageOverride::NORMAL;
        }
        (P::FontVariant, TypographyValue::FontVariant(value)) => {
            style.fields.font_variant_ligatures = value.ligatures;
            style.fields.font_variant_caps = value.caps;
            style.fields.font_variant_alternates = value.alternates;
            style.fields.font_variant_numeric = value.numeric;
            style.fields.font_variant_east_asian = value.east_asian;
            style.fields.font_variant_position = value.position;
            style.fields.font_variant_emoji = value.emoji;
        }
        (P::FontSynthesis, TypographyValue::FontSynthesisShorthand(value)) => {
            style.fields.font_synthesis_weight = value.weight;
            style.fields.font_synthesis_style = value.style;
            style.fields.font_synthesis_small_caps = value.small_caps;
            style.fields.font_synthesis_position = value.position;
        }
        (P::WhiteSpace, TypographyValue::WhiteSpace(value)) => {
            style.fields.white_space_collapse = value.collapse;
            style.fields.text_wrap_mode = value.wrap;
            style.fields.white_space = normalized_white_space(value.collapse, value.wrap);
            style.fields.text_wrap = normalized_text_wrap(value.wrap, style.fields.text_wrap_style);
        }
        (P::TextWrap, TypographyValue::TextWrap(value)) => {
            style.fields.text_wrap_mode = value.mode;
            style.fields.text_wrap_style = value.style;
            style.fields.text_wrap = normalized_text_wrap(value.mode, value.style);
            style.fields.white_space =
                normalized_white_space(style.fields.white_space_collapse, value.mode);
        }
        (P::TextDecoration, TypographyValue::TextDecoration(value)) => {
            style.fields.text_decoration_line = value.line;
            style.fields.text_decoration_style = value.style;
            style.fields.text_decoration_color = value.color;
            style.fields.text_decoration_thickness = value.thickness;
        }
        (P::TextEmphasis, TypographyValue::TextEmphasis(value)) => {
            style.fields.text_emphasis_mark = value.style.mark;
            style.fields.text_emphasis_fill = value.style.fill;
            style.fields.text_emphasis_color = value.color;
            style.fields.text_emphasis_position = value.position;
        }
        _ => return false,
    }
    true
}

/// Apply a typed declaration to the renderer's computed style.fields.
pub fn apply_to_computed(
    style: &mut ComputedStyle,
    property: StyleProperty,
    value: &StyleValue,
    viewport: (f32, f32),
) -> Result<InvalidationClass, PropertyTypeError> {
    use StyleProperty as P;
    let resolve = |v: LengthValue| v.resolve(viewport, style.fields.font_size, 16.0);
    let mismatch = || PropertyTypeError {
        property,
        expected: property.metadata().value_kind,
    };
    if let StyleValue::Renderer(value) = value {
        return value
            .apply(style, property)
            .then_some(property.metadata().invalidation)
            .ok_or_else(mismatch);
    }
    if let StyleValue::Typography(value) = value {
        return apply_typography_to_computed(style, property, value)
            .then_some(property.metadata().invalidation)
            .ok_or_else(mismatch);
    }
    if let Some(value) = RendererStyleValue::from_author_value(property, value, resolve) {
        return value
            .apply(style, property)
            .then_some(property.metadata().invalidation)
            .ok_or_else(mismatch);
    }
    match (property, value) {
        (P::Display, StyleValue::Display(v)) => style.fields.display = *v,
        (P::Position, StyleValue::Position(v)) => style.fields.position = *v,
        (P::Overflow, StyleValue::Overflow(v)) => {
            style.fields.overflow_x = *v;
            style.fields.overflow_y = *v;
        }
        (P::Width, StyleValue::Length(v)) => style.fields.width = resolve(*v),
        (P::Height, StyleValue::Length(v)) => style.fields.height = resolve(*v),
        (P::MinWidth, StyleValue::Length(v)) => style.fields.min_width = resolve(*v),
        (P::MinHeight, StyleValue::Length(v)) => style.fields.min_height = resolve(*v),
        (P::MaxWidth, StyleValue::Length(v)) => style.fields.max_width = resolve(*v),
        (P::MaxHeight, StyleValue::Length(v)) => style.fields.max_height = resolve(*v),
        (P::Margin, StyleValue::Edges(v)) => {
            style.fields.margin_top = resolve(v.top);
            style.fields.margin_right = resolve(v.right);
            style.fields.margin_bottom = resolve(v.bottom);
            style.fields.margin_left = resolve(v.left);
        }
        (P::MarginTop, StyleValue::Length(v)) => style.fields.margin_top = resolve(*v),
        (P::MarginRight, StyleValue::Length(v)) => style.fields.margin_right = resolve(*v),
        (P::MarginBottom, StyleValue::Length(v)) => style.fields.margin_bottom = resolve(*v),
        (P::MarginLeft, StyleValue::Length(v)) => style.fields.margin_left = resolve(*v),
        (P::Padding, StyleValue::Edges(v)) => {
            style.fields.padding_top = resolve(v.top);
            style.fields.padding_right = resolve(v.right);
            style.fields.padding_bottom = resolve(v.bottom);
            style.fields.padding_left = resolve(v.left);
        }
        (P::PaddingTop, StyleValue::Length(v)) => style.fields.padding_top = resolve(*v),
        (P::PaddingRight, StyleValue::Length(v)) => style.fields.padding_right = resolve(*v),
        (P::PaddingBottom, StyleValue::Length(v)) => style.fields.padding_bottom = resolve(*v),
        (P::PaddingLeft, StyleValue::Length(v)) => style.fields.padding_left = resolve(*v),
        (P::BackgroundColor, StyleValue::Color(v)) => style.fields.background_color = *v,
        (P::Color, StyleValue::Color(v)) => style.fields.color = *v,
        (P::Opacity, StyleValue::Number(v)) => style.fields.opacity = v.clamp(0.0, 1.0),
        (P::ZIndex, StyleValue::Integer(v)) => style.fields.z_index = Some(*v),
        (P::FlexDirection, StyleValue::FlexDirection(v)) => style.fields.flex_direction = *v,
        (P::FlexWrap, StyleValue::FlexWrap(v)) => style.fields.flex_wrap = *v,
        (P::FlexGrow, StyleValue::Number(v)) => style.fields.flex_grow = *v,
        (P::FlexShrink, StyleValue::Number(v)) => style.fields.flex_shrink = *v,
        (P::FlexBasis, StyleValue::Length(v)) => style.fields.flex_basis = resolve(*v),
        (P::AlignItems, StyleValue::ItemAlignment(v)) => style.fields.align_items = *v,
        (P::JustifyContent, StyleValue::ContentAlignment(v)) => style.fields.justify_content = *v,
        (P::Gap, StyleValue::Gap(v)) => {
            style.fields.row_gap = Some(resolve(v.row));
            style.fields.column_gap = Some(resolve(v.column));
        }
        (P::RowGap, StyleValue::Length(v)) => style.fields.row_gap = Some(resolve(*v)),
        (P::ColumnGap, StyleValue::Length(v)) => style.fields.column_gap = Some(resolve(*v)),
        (P::FontFamily, StyleValue::FontFamily(v)) => style.fields.font_family = v.clone(),
        (P::FontSize, StyleValue::Length(v)) => style.fields.font_size = resolve(*v).value(),
        (P::FontWeight, StyleValue::FontWeight(v)) => style.fields.font_weight = *v,
        (P::LetterSpacing, StyleValue::Number(v)) => style.fields.letter_spacing = *v,
        (P::WordSpacing, StyleValue::Number(v)) => style.fields.word_spacing = *v,
        (P::TextIndent, StyleValue::Length(v)) => style.fields.text_indent = resolve(*v),
        (P::TextUnderlineOffset, StyleValue::Length(v)) => {
            style.fields.text_underline_offset = resolve(*v)
        }
        (
            P::Border | P::BorderTop | P::BorderRight | P::BorderBottom | P::BorderLeft,
            StyleValue::Border(v),
        ) => {
            let targets = match property {
                P::Border => [true, true, true, true],
                P::BorderTop => [true, false, false, false],
                P::BorderRight => [false, true, false, false],
                P::BorderBottom => [false, false, true, false],
                _ => [false, false, false, true],
            };
            if targets[0] {
                style.fields.border_top_width = v.width as i32;
                style.fields.border_top_style = v.style;
                style.fields.border_top_color = StyleColor::Resolved(v.color);
            }
            if targets[1] {
                style.fields.border_right_width = v.width as i32;
                style.fields.border_right_style = v.style;
                style.fields.border_right_color = StyleColor::Resolved(v.color);
            }
            if targets[2] {
                style.fields.border_bottom_width = v.width as i32;
                style.fields.border_bottom_style = v.style;
                style.fields.border_bottom_color = StyleColor::Resolved(v.color);
            }
            if targets[3] {
                style.fields.border_left_width = v.width as i32;
                style.fields.border_left_style = v.style;
                style.fields.border_left_color = StyleColor::Resolved(v.color);
            }
        }
        (P::BorderRadius, StyleValue::CornerRadii(v)) => {
            let px = |value: LengthValue| resolve(value).value();
            style.fields.border_top_left_radius = (px(v.0.top), px(v.0.top));
            style.fields.border_top_right_radius = (px(v.0.right), px(v.0.right));
            style.fields.border_bottom_right_radius = (px(v.0.bottom), px(v.0.bottom));
            style.fields.border_bottom_left_radius = (px(v.0.left), px(v.0.left));
        }
        (P::ListStyleType, StyleValue::ListStyle(v)) => style.fields.list_style_type = *v,
        (P::Transform, StyleValue::Transform(v)) => {
            let mut matrix = Transform2D::IDENTITY;
            for op in &v.0 {
                match op {
                    TransformOperation::Matrix(v) => matrix = *v,
                    TransformOperation::Scale(x, y) => {
                        matrix.a *= x;
                        matrix.d *= y;
                    }
                    TransformOperation::Scale3d(x, y, _) => {
                        matrix.a *= x;
                        matrix.d *= y;
                    }
                    TransformOperation::Translate(x, y) => {
                        matrix.e += resolve(*x).value();
                        matrix.f += resolve(*y).value();
                    }
                    TransformOperation::Translate3d(x, y, _) => {
                        matrix.e += resolve(*x).value();
                        matrix.f += resolve(*y).value();
                    }
                    TransformOperation::Rotate(deg) => {
                        let r = deg.to_radians();
                        let (s, c) = r.sin_cos();
                        matrix = Transform2D {
                            a: c,
                            b: s,
                            c: -s,
                            d: c,
                            e: matrix.e,
                            f: matrix.f,
                        };
                    }
                    TransformOperation::Rotate3d { x, y, z, degrees } => {
                        let length = (*x * *x + *y * *y + *z * *z).sqrt();
                        if length > f32::EPSILON && (*z / length).abs() > 1.0 - 1e-6 {
                            let r = (*degrees * z.signum()).to_radians();
                            let (s, c) = r.sin_cos();
                            matrix = Transform2D {
                                a: c,
                                b: s,
                                c: -s,
                                d: c,
                                e: matrix.e,
                                f: matrix.f,
                            };
                        }
                    }
                    TransformOperation::Perspective(_) => {}
                    TransformOperation::Matrix3d(value) => matrix = value.projected_2d(),
                }
            }
            style.fields.transform = matrix;
        }
        (P::Cursor, StyleValue::Cursor(_)) => {}
        (P::PointerEvents, StyleValue::PointerEvents(v)) => style.fields.pointer_events = *v,
        _ => return Err(mismatch()),
    }
    Ok(property.metadata().invalidation)
}

/// Read a schema value back from a computed style.fields.
///
/// Relative lengths have already been resolved by this point and are returned
/// as pixel values. This is primarily used to capture an animation's
/// underlying value without maintaining a second handwritten style model.
pub fn value_from_computed(style: &ComputedStyle, property: StyleProperty) -> StyleValue {
    use StyleProperty as P;
    if let Some(value) = RendererStyleValue::from_computed(style, property) {
        return StyleValue::Renderer(value);
    }
    let length = |value: Length| StyleValue::Length(LengthValue::Computed(value));
    let border = |width: i32, border_style: BorderStyle, color: StyleColor| {
        StyleValue::Border(Border {
            width: width as f32,
            style: border_style,
            color: color.resolve(&style.fields.color),
        })
    };
    match property {
        P::Display => StyleValue::Display(style.fields.display),
        P::Position => StyleValue::Position(style.fields.position),
        P::Overflow => StyleValue::Overflow(style.fields.overflow_x),
        P::Width => length(style.fields.width),
        P::Height => length(style.fields.height),
        P::MinWidth => length(style.fields.min_width),
        P::MinHeight => length(style.fields.min_height),
        P::MaxWidth => length(style.fields.max_width),
        P::MaxHeight => length(style.fields.max_height),
        P::Margin => StyleValue::Edges(Edges {
            top: style.fields.margin_top.into(),
            right: style.fields.margin_right.into(),
            bottom: style.fields.margin_bottom.into(),
            left: style.fields.margin_left.into(),
        }),
        P::MarginTop => length(style.fields.margin_top),
        P::MarginRight => length(style.fields.margin_right),
        P::MarginBottom => length(style.fields.margin_bottom),
        P::MarginLeft => length(style.fields.margin_left),
        P::Padding => StyleValue::Edges(Edges {
            top: style.fields.padding_top.into(),
            right: style.fields.padding_right.into(),
            bottom: style.fields.padding_bottom.into(),
            left: style.fields.padding_left.into(),
        }),
        P::PaddingTop => length(style.fields.padding_top),
        P::PaddingRight => length(style.fields.padding_right),
        P::PaddingBottom => length(style.fields.padding_bottom),
        P::PaddingLeft => length(style.fields.padding_left),
        P::BackgroundColor => StyleValue::Color(style.fields.background_color),
        P::Color => StyleValue::Color(style.fields.color),
        P::Opacity => StyleValue::Number(style.fields.opacity),
        P::ZIndex => StyleValue::Integer(style.fields.z_index.unwrap_or_default()),
        P::FlexDirection => StyleValue::FlexDirection(style.fields.flex_direction),
        P::FlexWrap => StyleValue::FlexWrap(style.fields.flex_wrap),
        P::FlexGrow => StyleValue::Number(style.fields.flex_grow),
        P::FlexShrink => StyleValue::Number(style.fields.flex_shrink),
        P::FlexBasis => length(style.fields.flex_basis),
        P::AlignItems => StyleValue::ItemAlignment(style.fields.align_items),
        P::JustifyContent => StyleValue::ContentAlignment(style.fields.justify_content),
        P::Gap => StyleValue::Gap(Gap {
            row: style.fields.row_gap.unwrap_or_else(Length::zero).into(),
            column: style.fields.column_gap.unwrap_or_else(Length::zero).into(),
        }),
        P::RowGap => length(style.fields.row_gap.unwrap_or_else(Length::zero)),
        P::ColumnGap => length(style.fields.column_gap.unwrap_or_else(Length::zero)),
        P::FontFamily => StyleValue::FontFamily(style.fields.font_family.clone()),
        P::FontSize => StyleValue::Length(LengthValue::px(style.fields.font_size)),
        P::FontWeight => StyleValue::FontWeight(style.fields.font_weight),
        P::LetterSpacing => StyleValue::Number(style.fields.letter_spacing),
        P::WordSpacing => StyleValue::Number(style.fields.word_spacing),
        P::TextIndent => length(style.fields.text_indent),
        P::TextUnderlineOffset => length(style.fields.text_underline_offset),
        P::Border => border(
            style.fields.border_top_width,
            style.fields.border_top_style,
            style.fields.border_top_color,
        ),
        P::BorderTop => border(
            style.fields.border_top_width,
            style.fields.border_top_style,
            style.fields.border_top_color,
        ),
        P::BorderRight => border(
            style.fields.border_right_width,
            style.fields.border_right_style,
            style.fields.border_right_color,
        ),
        P::BorderBottom => border(
            style.fields.border_bottom_width,
            style.fields.border_bottom_style,
            style.fields.border_bottom_color,
        ),
        P::BorderLeft => border(
            style.fields.border_left_width,
            style.fields.border_left_style,
            style.fields.border_left_color,
        ),
        P::BorderRadius => StyleValue::CornerRadii(CornerRadii(Edges {
            top: LengthValue::px(style.fields.border_top_left_radius.0),
            right: LengthValue::px(style.fields.border_top_right_radius.0),
            bottom: LengthValue::px(style.fields.border_bottom_right_radius.0),
            left: LengthValue::px(style.fields.border_bottom_left_radius.0),
        })),
        P::Cursor => StyleValue::Cursor(Cursor::Auto),
        P::ListStyleType => StyleValue::ListStyle(style.fields.list_style_type),
        P::Transform => StyleValue::Transform(TransformList(vec![TransformOperation::Matrix(
            style.fields.transform,
        )])),
        P::PointerEvents => StyleValue::PointerEvents(style.fields.pointer_events),
        _ => typography_from_computed(style, property),
    }
}

fn typography_from_computed(style: &ComputedStyle, property: StyleProperty) -> StyleValue {
    use StyleProperty as P;
    let value = match property {
        P::Direction => TypographyValue::Direction(style.fields.direction),
        P::FontKerning => TypographyValue::FontKerning(style.fields.font_kerning),
        P::FontOpticalSizing => {
            TypographyValue::FontOpticalSizing(style.fields.font_optical_sizing)
        }
        P::FontPalette => TypographyValue::FontPalette(style.fields.font_palette.clone()),
        P::FontSizeAdjust => TypographyValue::FontSizeAdjust(style.fields.font_size_adjust),
        P::FontStretch => TypographyValue::FontStretch(style.fields.font_stretch),
        P::FontStyle => TypographyValue::FontStyle(style.fields.font_style),
        P::FontVariantLigatures => {
            TypographyValue::FontVariantLigatures(style.fields.font_variant_ligatures)
        }
        P::FontVariantCaps => TypographyValue::FontVariantCaps(style.fields.font_variant_caps),
        P::FontVariantEastAsian => {
            TypographyValue::FontVariantEastAsian(style.fields.font_variant_east_asian)
        }
        P::FontVariantNumeric => {
            TypographyValue::FontVariantNumeric(style.fields.font_variant_numeric)
        }
        P::FontVariantAlternates => {
            TypographyValue::FontVariantAlternates(style.fields.font_variant_alternates)
        }
        P::FontVariantPosition => {
            TypographyValue::FontVariantPosition(style.fields.font_variant_position)
        }
        P::FontVariantEmoji => TypographyValue::FontVariantEmoji(style.fields.font_variant_emoji),
        P::FontSynthesisWeight => {
            TypographyValue::FontSynthesis(style.fields.font_synthesis_weight)
        }
        P::FontSynthesisStyle => TypographyValue::FontSynthesis(style.fields.font_synthesis_style),
        P::FontSynthesisSmallCaps => {
            TypographyValue::FontSynthesis(style.fields.font_synthesis_small_caps)
        }
        P::FontSynthesisPosition => {
            TypographyValue::FontSynthesis(style.fields.font_synthesis_position)
        }
        P::FontFeatureSettings => TypographyValue::OpenTypeFeatures(OpenTypeFeatureList(
            style.fields.font_feature_settings.clone(),
        )),
        P::FontVariationSettings => TypographyValue::FontVariations(FontVariationList(
            style.fields.font_variation_settings.clone(),
        )),
        P::FontLanguageOverride => {
            TypographyValue::FontLanguageOverride(style.fields.font_language_override)
        }
        P::LineHeight => TypographyValue::LineHeight(style.fields.line_height),
        P::TextAlign => TypographyValue::TextAlign(style.fields.text_align),
        P::TextAlignLast => TypographyValue::TextAlignLast(style.fields.text_align_last),
        P::TextJustify => TypographyValue::TextJustify(style.fields.text_justify),
        P::WordBreak => TypographyValue::WordBreak(style.fields.word_break),
        P::OverflowWrap | P::WordWrap => TypographyValue::OverflowWrap(style.fields.overflow_wrap),
        P::LineBreak => TypographyValue::LineBreak(style.fields.line_break),
        P::Hyphens => TypographyValue::Hyphens(style.fields.hyphens),
        P::HyphenateLimitChars => {
            let (word, before, after) = style.fields.hyphenate_limit_chars;
            TypographyValue::HyphenationLimits(HyphenationLimits {
                word,
                before,
                after,
            })
        }
        P::HyphenateCharacter => TypographyValue::HyphenateCharacter(HyphenateCharacter(
            style.fields.hyphenate_character.clone(),
        )),
        P::WhiteSpaceCollapse => {
            TypographyValue::WhiteSpaceCollapse(style.fields.white_space_collapse)
        }
        P::TextWrapMode => TypographyValue::TextWrapMode(style.fields.text_wrap_mode),
        P::TextWrapStyle => TypographyValue::TextWrapStyle(style.fields.text_wrap_style),
        P::TextAutospace => TypographyValue::TextAutospace(style.fields.text_autospace),
        P::TextSpacingTrim => TypographyValue::TextSpacingTrim(style.fields.text_spacing_trim),
        P::TabSize => TypographyValue::TabSize(style.fields.tab_size),
        P::TextTransform => TypographyValue::TextTransform(style.fields.text_transform),
        P::TextDecorationLine => {
            TypographyValue::TextDecorationLine(style.fields.text_decoration_line)
        }
        P::TextDecorationStyle => {
            TypographyValue::TextDecorationStyle(style.fields.text_decoration_style)
        }
        P::TextDecorationThickness => {
            TypographyValue::TextDecorationThickness(style.fields.text_decoration_thickness)
        }
        P::TextDecorationColor => TypographyValue::StyleColor(style.fields.text_decoration_color),
        P::TextEmphasisColor => TypographyValue::StyleColor(style.fields.text_emphasis_color),
        P::TextDecorationSkipInk => {
            TypographyValue::TextDecorationSkipInk(style.fields.text_decoration_skip_ink)
        }
        P::TextUnderlinePosition => {
            TypographyValue::TextUnderlinePosition(style.fields.text_underline_position)
        }
        P::TextEmphasisStyle => TypographyValue::TextEmphasisStyle(TextEmphasisStyle {
            mark: style.fields.text_emphasis_mark,
            fill: style.fields.text_emphasis_fill,
        }),
        P::TextEmphasisPosition => {
            TypographyValue::TextEmphasisPosition(style.fields.text_emphasis_position)
        }
        P::TextShadow => {
            TypographyValue::TextShadows(TextShadowList(style.fields.text_shadow.clone()))
        }
        P::TextOverflow => TypographyValue::TextOverflow(style.fields.text_overflow),
        P::TextSizeAdjust => TypographyValue::TextSizeAdjust(style.fields.text_size_adjust),
        P::TextCombineUpright => {
            TypographyValue::TextCombineUpright(style.fields.text_combine_upright)
        }
        P::WritingMode => TypographyValue::WritingMode(style.fields.writing_mode),
        P::TextOrientation => TypographyValue::TextOrientation(style.fields.text_orientation),
        P::UnicodeBidi => TypographyValue::UnicodeBidi(style.fields.unicode_bidi),
        P::VerticalAlign => TypographyValue::VerticalAlign(style.fields.vertical_align),
        P::RubyAlign => TypographyValue::RubyAlign(style.fields.ruby_align),
        P::RubyPosition => TypographyValue::RubyPosition(style.fields.ruby_position),
        P::RubyOverhang => TypographyValue::RubyOverhang(style.fields.ruby_overhang),
        P::HangingPunctuation => {
            TypographyValue::HangingPunctuation(style.fields.hanging_punctuation)
        }
        P::InitialLetter => TypographyValue::InitialLetter(
            style
                .initial_letter
                .map_or(InitialLetterValue::Normal, InitialLetterValue::Value),
        ),
        P::TextRendering => TypographyValue::TextRendering(style.fields.text_rendering),
        P::WebkitFontSmoothing => TypographyValue::FontSmoothing(style.fields.font_smoothing),
        P::LineClamp => TypographyValue::LineClamp(style.fields.line_clamp),
        P::BlockEllipsis => TypographyValue::BlockEllipsis(style.fields.block_ellipsis.clone()),
        P::TextBoxEdge => TypographyValue::TextBoxEdge(style.fields.text_box_edge),
        P::TextBoxTrim => TypographyValue::TextBoxTrim(style.fields.text_box_trim),
        P::TextBox => TypographyValue::TextBox(TextBoxShorthand {
            trim: style.fields.text_box_trim,
            edge: style.fields.text_box_edge,
        }),
        P::Font => TypographyValue::Font(FontShorthand {
            style: style.fields.font_style,
            variant_caps: style.fields.font_variant_caps,
            weight: style.fields.font_weight,
            stretch: style.fields.font_stretch,
            size_px: style.fields.font_size,
            line_height: style.fields.line_height,
            family: style.fields.font_family.clone(),
        }),
        P::FontVariant => TypographyValue::FontVariant(FontVariantShorthand {
            ligatures: style.fields.font_variant_ligatures,
            caps: style.fields.font_variant_caps,
            alternates: style.fields.font_variant_alternates,
            numeric: style.fields.font_variant_numeric,
            east_asian: style.fields.font_variant_east_asian,
            position: style.fields.font_variant_position,
            emoji: style.fields.font_variant_emoji,
        }),
        P::FontSynthesis => TypographyValue::FontSynthesisShorthand(FontSynthesisShorthand {
            weight: style.fields.font_synthesis_weight,
            style: style.fields.font_synthesis_style,
            small_caps: style.fields.font_synthesis_small_caps,
            position: style.fields.font_synthesis_position,
        }),
        P::WhiteSpace => TypographyValue::WhiteSpace(WhiteSpaceShorthand {
            collapse: style.fields.white_space_collapse,
            wrap: style.fields.text_wrap_mode,
        }),
        P::TextWrap => TypographyValue::TextWrap(TextWrapShorthand {
            mode: style.fields.text_wrap_mode,
            style: style.fields.text_wrap_style,
        }),
        P::TextDecoration => TypographyValue::TextDecoration(TextDecorationShorthand {
            line: style.fields.text_decoration_line,
            style: style.fields.text_decoration_style,
            color: style.fields.text_decoration_color,
            thickness: style.fields.text_decoration_thickness,
        }),
        P::TextEmphasis => TypographyValue::TextEmphasis(TextEmphasisShorthand {
            style: TextEmphasisStyle {
                mark: style.fields.text_emphasis_mark,
                fill: style.fields.text_emphasis_fill,
            },
            color: style.fields.text_emphasis_color,
            position: style.fields.text_emphasis_position,
        }),
        _ => unreachable!("non-typography property routed to typography reader"),
    };
    StyleValue::Typography(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_lookup_and_metadata_are_stable() {
        assert_eq!(
            StyleProperty::from_css_name("padding"),
            Some(StyleProperty::Padding)
        );
        assert_eq!(StyleProperty::Opacity as u16, 22);
        assert_eq!(StyleProperty::Direction as u16, 47);
        assert_eq!(StyleProperty::TextEmphasis as u16, 124);
        assert_eq!(StyleProperty::TextBox as u16, 125);
        assert_eq!(StyleProperty::ShapeOutside as u16, 237);
        assert_eq!(PROPERTY_METADATA.len(), 237);
        for (index, metadata) in PROPERTY_METADATA.iter().enumerate() {
            let id = u16::try_from(index + 1).unwrap();
            assert_eq!(StyleProperty::from_u16(id), Some(metadata.property));
            assert_eq!(
                StyleProperty::from_css_name(metadata.css_name),
                Some(metadata.property)
            );
            assert_eq!(metadata.property.metadata(), metadata);
        }
        assert_eq!(StyleProperty::from_u16(0), None);
        assert_eq!(StyleProperty::from_u16(238), None);
        assert_eq!(
            StyleProperty::Opacity.metadata().invalidation,
            InvalidationClass::Composite
        );
    }

    #[test]
    fn parses_checked_literals_and_applies_them() {
        let padding = parse_literal(StyleProperty::Padding, "8px 16px").unwrap();
        let mut style = ComputedStyle::initial();
        apply_to_computed(&mut style, StyleProperty::Padding, &padding, (800.0, 600.0)).unwrap();
        assert_eq!(style.fields.padding_top, Length::px(8.0));
        assert_eq!(style.fields.padding_right, Length::px(16.0));
        assert!(parse_literal(StyleProperty::Padding, "wat").is_err());
    }

    #[test]
    fn resolves_viewport_units_deterministically() {
        assert_eq!(
            LengthValue::ViewportHeight(100.0).resolve((800.0, 600.0), 16.0, 16.0),
            Length::px(600.0)
        );
        assert_eq!(
            LengthValue::ViewportMin(25.0).resolve((1280.0, 720.0), 16.0, 16.0),
            Length::px(180.0)
        );
        assert_eq!(
            LengthValue::ViewportMax(25.0).resolve((1280.0, 720.0), 16.0, 16.0),
            Length::px(320.0)
        );
    }

    #[test]
    fn text_size_adjust_rejects_invalid_percentages() {
        assert!(parse_literal(StyleProperty::TextSizeAdjust, "-1%").is_err());
        assert!(parse_literal(StyleProperty::TextSizeAdjust, "NaN%").is_err());
        assert!(parse_literal(StyleProperty::TextSizeAdjust, "inf%").is_err());
        assert!(parse_literal(StyleProperty::TextSizeAdjust, "0%").is_ok());
    }

    #[test]
    fn every_public_typography_property_parses_applies_and_reads_back() {
        let cases = [
            ("direction", "rtl"),
            ("font-kerning", "normal"),
            ("font-optical-sizing", "none"),
            ("font-palette", "dark"),
            ("font-size-adjust", "cap-height 0.7"),
            ("font-stretch", "condensed"),
            ("font-style", "oblique 12deg"),
            ("font-variant-ligatures", "none"),
            ("font-variant-caps", "all-small-caps"),
            ("font-variant-east-asian", "jis04 ruby"),
            ("font-variant-numeric", "lining-nums tabular-nums"),
            ("font-variant-alternates", "historical-forms"),
            ("font-variant-position", "super"),
            ("font-variant-emoji", "emoji"),
            ("font-synthesis-weight", "none"),
            ("font-synthesis-style", "none"),
            ("font-synthesis-small-caps", "none"),
            ("font-synthesis-position", "none"),
            ("font-feature-settings", "\"liga\" 0, \"kern\" on"),
            ("font-variation-settings", "\"wght\" 650"),
            ("font-language-override", "\"TRK \""),
            ("line-height", "1.5"),
            ("letter-spacing", "2px"),
            ("word-spacing", "normal"),
            ("text-indent", "2em"),
            ("text-align", "justify"),
            ("text-align-last", "center"),
            ("text-justify", "inter-character"),
            ("word-break", "auto-phrase"),
            ("overflow-wrap", "anywhere"),
            ("word-wrap", "break-word"),
            ("line-break", "after-white-space"),
            ("hyphens", "auto"),
            ("hyphenate-limit-chars", "6 2 3"),
            ("hyphenate-character", "\"-\""),
            ("white-space-collapse", "preserve-breaks"),
            ("text-wrap-mode", "nowrap"),
            ("text-wrap-style", "pretty"),
            ("text-autospace", "no-autospace"),
            ("text-spacing-trim", "trim-start"),
            ("tab-size", "4"),
            ("text-transform", "math-auto"),
            ("text-decoration-line", "underline overline"),
            ("text-decoration-style", "wavy"),
            ("text-decoration-color", "red"),
            ("text-decoration-thickness", "2px"),
            ("text-decoration-skip-ink", "all"),
            ("text-underline-offset", "3px"),
            ("text-underline-position", "under"),
            ("text-emphasis-style", "open sesame"),
            ("text-emphasis-position", "under left"),
            ("text-emphasis-color", "blue"),
            ("text-shadow", "1px 2px 3px red"),
            ("text-overflow", "ellipsis"),
            ("text-size-adjust", "80%"),
            ("text-combine-upright", "all"),
            ("writing-mode", "vertical-rl"),
            ("text-orientation", "upright"),
            ("unicode-bidi", "isolate-override"),
            ("vertical-align", "10%"),
            ("ruby-align", "center"),
            ("ruby-position", "under"),
            ("ruby-overhang", "none"),
            ("hanging-punctuation", "first force-end"),
            ("initial-letter", "3 2"),
            ("text-rendering", "geometricprecision"),
            ("-webkit-font-smoothing", "antialiased"),
            ("line-clamp", "3"),
            ("block-ellipsis", "\"…\""),
            ("text-box-edge", "cap alphabetic"),
            ("text-box-trim", "trim-both"),
            ("text-box", "trim-both cap alphabetic"),
            (
                "font",
                "italic small-caps bold 18px/1.4 \"Noto Sans\", sans-serif",
            ),
            ("font-variant", "small-caps lining-nums"),
            ("font-synthesis", "weight style"),
            ("white-space", "pre-wrap"),
            ("text-wrap", "balance"),
            ("text-decoration", "underline wavy red 2px"),
            ("text-emphasis", "open dot red"),
        ];
        assert_eq!(cases.len(), 79);
        let mut style = ComputedStyle::initial();
        for (name, literal) in cases {
            let property = StyleProperty::from_css_name(name).unwrap();
            let value =
                parse_literal(property, literal).unwrap_or_else(|error| panic!("{name}: {error}"));
            apply_to_computed(&mut style, property, &value, (800.0, 600.0))
                .unwrap_or_else(|_| panic!("failed to apply {name}"));
            let _ = value_from_computed(&style, property);
        }
    }

    #[test]
    fn font_shorthand_is_atomic_and_resets_omitted_font_longhands() {
        let mut style = ComputedStyle::initial();
        style.fields.font_kerning = FontKerning::None;
        style.fields.font_feature_settings.push(crate::FontFeature {
            tag: *b"liga",
            value: 0,
        });
        let value = parse_literal(StyleProperty::Font, "italic bold 20px serif").unwrap();
        apply_to_computed(&mut style, StyleProperty::Font, &value, (800.0, 600.0)).unwrap();
        assert_eq!(style.fields.font_style, FontStyleEnum::Italic);
        assert_eq!(style.fields.font_weight, FontWeight::BOLD);
        assert_eq!(style.fields.font_size, 20.0);
        assert_eq!(style.fields.font_kerning, FontKerning::Auto);
        assert!(style.fields.font_feature_settings.is_empty());
    }
}
