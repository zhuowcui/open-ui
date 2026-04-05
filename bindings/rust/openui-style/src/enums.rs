//! CSS style enums — extracted from Blink's `computed_style_constants.h`.
//!
//! Source: out/Release/gen/third_party/blink/renderer/core/style/computed_style_base_constants.h
//! and core/style/computed_style_constants.h

use openui_geometry::WritingDirectionMode;

/// CSS `display` property.
/// Blink stores this in 6 bits (35 values). We implement the commonly used subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Display {
    /// `display: none` — element generates no boxes.
    None = 0,
    /// `display: inline` — inline-level box.
    Inline = 1,
    /// `display: block` — block-level box.
    Block = 2,
    /// `display: flex` — flex container.
    Flex = 3,
    /// `display: grid` — grid container.
    Grid = 4,
    /// `display: inline-block` — inline-level block container.
    InlineBlock = 5,
    /// `display: inline-flex` — inline-level flex container.
    InlineFlex = 6,
    /// `display: inline-grid` — inline-level grid container.
    InlineGrid = 7,
    /// `display: flow-root` — block-level, establishes new BFC.
    FlowRoot = 8,
    /// `display: table` — table layout (future).
    Table = 9,
    /// `display: list-item` — block with marker box.
    ListItem = 10,
}

impl Display {
    /// Blink's initial value: `kInline`.
    pub const INITIAL: Self = Self::Inline;

    /// True if this display creates a block-level box.
    #[inline]
    pub fn is_block_level(self) -> bool {
        matches!(self, Self::Block | Self::Flex | Self::Grid | Self::FlowRoot | Self::Table | Self::ListItem)
    }

    /// True if this display creates an inline-level box.
    #[inline]
    pub fn is_inline_level(self) -> bool {
        matches!(self, Self::Inline | Self::InlineBlock | Self::InlineFlex | Self::InlineGrid)
    }

    /// True if this is a flex container.
    #[inline]
    pub fn is_flex(self) -> bool {
        matches!(self, Self::Flex | Self::InlineFlex)
    }

    /// True if this is a grid container.
    #[inline]
    pub fn is_grid(self) -> bool {
        matches!(self, Self::Grid | Self::InlineGrid)
    }

    /// True if this creates a new formatting context (BFC, FFC, or GFC).
    #[inline]
    pub fn is_new_formatting_context(self) -> bool {
        matches!(self, Self::Flex | Self::Grid | Self::InlineFlex | Self::InlineGrid |
                 Self::InlineBlock | Self::FlowRoot | Self::Table)
    }
}

impl Default for Display {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `position` property.
/// Blink stores this in 3 bits (5 values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Position {
    Static = 0,
    Relative = 1,
    Absolute = 2,
    Fixed = 3,
    Sticky = 4,
}

impl Position {
    pub const INITIAL: Self = Self::Static;

    #[inline]
    pub fn is_positioned(self) -> bool {
        !matches!(self, Self::Static)
    }

    #[inline]
    pub fn is_absolutely_positioned(self) -> bool {
        matches!(self, Self::Absolute | Self::Fixed)
    }

    #[inline]
    pub fn is_in_flow(self) -> bool {
        matches!(self, Self::Static | Self::Relative | Self::Sticky)
    }
}

impl Default for Position {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `overflow` property values.
/// Blink stores this in 3 bits (6 values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Overflow {
    Visible = 0,
    Hidden = 1,
    Scroll = 2,
    Auto = 3,
    Clip = 4,
}

impl Overflow {
    pub const INITIAL: Self = Self::Visible;

    #[inline]
    pub fn is_scrollable(self) -> bool {
        matches!(self, Self::Scroll | Self::Auto)
    }

    #[inline]
    pub fn is_clipping(self) -> bool {
        !matches!(self, Self::Visible)
    }
}

impl Default for Overflow {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `box-sizing` property.
/// Blink stores this in 1 bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BoxSizing {
    ContentBox = 0,
    BorderBox = 1,
}

impl BoxSizing {
    pub const INITIAL: Self = Self::ContentBox;
}

impl Default for BoxSizing {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `float` property.
/// Blink stores this in 3 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Float {
    None = 0,
    Left = 1,
    Right = 2,
}

impl Float {
    pub const INITIAL: Self = Self::None;
}

impl Default for Float {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `clear` property.
/// Blink stores this in 3 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Clear {
    None = 0,
    Left = 1,
    Right = 2,
    Both = 3,
}

impl Clear {
    pub const INITIAL: Self = Self::None;
}

impl Default for Clear {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `border-style` property.
/// Blink stores this in 4 bits (10 values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BorderStyle {
    None = 0,
    Hidden = 1,
    Dotted = 2,
    Dashed = 3,
    Solid = 4,
    Double = 5,
    Groove = 6,
    Ridge = 7,
    Inset = 8,
    Outset = 9,
}

impl BorderStyle {
    pub const INITIAL: Self = Self::None;

    /// True if this style actually renders a visible border.
    #[inline]
    pub fn has_visible_border(self) -> bool {
        !matches!(self, Self::None | Self::Hidden)
    }
}

impl Default for BorderStyle {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-align` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextAlign {
    Left = 0,
    Right = 1,
    Center = 2,
    Justify = 3,
    Start = 4,
    End = 5,
}

impl TextAlign {
    pub const INITIAL: Self = Self::Start;
}

impl Default for TextAlign {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `direction` property (inherited).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    Ltr = 0,
    Rtl = 1,
}

impl Direction {
    pub const INITIAL: Self = Self::Ltr;
}

impl Default for Direction {
    fn default() -> Self { Self::INITIAL }
}

impl Direction {
    /// Create a `WritingDirectionMode` combining this direction with a writing mode.
    ///
    /// This bridges the style enums to the geometry converter without
    /// creating a circular dependency.
    #[inline]
    pub fn writing_direction(self, wm: WritingMode) -> WritingDirectionMode {
        WritingDirectionMode::new(
            wm.is_horizontal(),
            wm.is_flipped_blocks(),
            wm.is_flipped_lines(),
            self == Direction::Rtl,
        )
    }
}

impl WritingMode {
    /// Create a `WritingDirectionMode` combining this writing mode with a direction.
    #[inline]
    pub fn to_writing_direction(self, dir: Direction) -> WritingDirectionMode {
        dir.writing_direction(self)
    }
}

/// CSS `visibility` property (inherited).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Visibility {
    Visible = 0,
    Hidden = 1,
    Collapse = 2,
}

impl Visibility {
    pub const INITIAL: Self = Self::Visible;
}

impl Default for Visibility {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `white-space` collapse mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WhiteSpace {
    Normal = 0,
    Nowrap = 1,
    Pre = 2,
    PreWrap = 3,
    PreLine = 4,
    BreakSpaces = 5,
}

impl WhiteSpace {
    pub const INITIAL: Self = Self::Normal;
}

impl Default for WhiteSpace {
    fn default() -> Self { Self::INITIAL }
}

// ── Flexbox enums (extracted from Blink computed_style_constants.h) ──────

/// CSS `flex-direction` property.
/// Blink stores this as `EFlexDirection` in 2 bits.
/// Source: core/css/css_properties.json5:3489
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FlexDirection {
    Row = 0,
    RowReverse = 1,
    Column = 2,
    ColumnReverse = 3,
}

impl FlexDirection {
    pub const INITIAL: Self = Self::Row;

    #[inline]
    pub fn is_column(self) -> bool {
        matches!(self, Self::Column | Self::ColumnReverse)
    }

    #[inline]
    pub fn is_reverse(self) -> bool {
        matches!(self, Self::RowReverse | Self::ColumnReverse)
    }
}

impl Default for FlexDirection {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `flex-wrap` property.
/// Blink stores wrap mode in `StyleFlexWrapData` as `FlexWrapMode` (2 bits).
/// Source: core/style/computed_style_constants.h:638
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FlexWrap {
    Nowrap = 0,
    Wrap = 1,
    WrapReverse = 2,
}

impl FlexWrap {
    pub const INITIAL: Self = Self::Nowrap;

    #[inline]
    pub fn is_wrap(self) -> bool {
        !matches!(self, Self::Nowrap)
    }

    #[inline]
    pub fn is_wrap_reverse(self) -> bool {
        matches!(self, Self::WrapReverse)
    }
}

impl Default for FlexWrap {
    fn default() -> Self { Self::INITIAL }
}

/// Content position values for `justify-content` and `align-content`.
/// Blink: `ContentPosition` in computed_style_constants.h:445 (4 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ContentPosition {
    Normal = 0,
    Baseline = 1,
    LastBaseline = 2,
    Center = 3,
    Start = 4,
    End = 5,
    FlexStart = 6,
    FlexEnd = 7,
    Left = 8,
    Right = 9,
}

impl ContentPosition {
    pub const INITIAL: Self = Self::Normal;
}

impl Default for ContentPosition {
    fn default() -> Self { Self::INITIAL }
}

/// Content distribution values for `justify-content` and `align-content`.
/// Blink: `ContentDistributionType` in computed_style_constants.h:458 (3 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ContentDistribution {
    Default = 0,
    SpaceBetween = 1,
    SpaceAround = 2,
    SpaceEvenly = 3,
    Stretch = 4,
}

impl ContentDistribution {
    pub const INITIAL: Self = Self::Default;
}

impl Default for ContentDistribution {
    fn default() -> Self { Self::INITIAL }
}

/// Overflow alignment modifier (`safe` / `unsafe`).
/// Blink: `OverflowAlignment` in computed_style_constants.h:441 (2 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OverflowAlignment {
    Default = 0,
    Unsafe = 1,
    Safe = 2,
}

impl OverflowAlignment {
    pub const INITIAL: Self = Self::Default;
}

impl Default for OverflowAlignment {
    fn default() -> Self { Self::INITIAL }
}

/// Per-item alignment position for `align-items`, `align-self`, `justify-self`.
/// Blink: `ItemPosition` in computed_style_constants.h:422 (4 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ItemPosition {
    Legacy = 0,
    Auto = 1,
    Normal = 2,
    Stretch = 3,
    Baseline = 4,
    LastBaseline = 5,
    Center = 6,
    Start = 7,
    End = 8,
    SelfStart = 9,
    SelfEnd = 10,
    FlexStart = 11,
    FlexEnd = 12,
    Left = 13,
    Right = 14,
}

impl ItemPosition {
    pub const INITIAL: Self = Self::Normal;
}

impl Default for ItemPosition {
    fn default() -> Self { Self::INITIAL }
}

/// Compound content-alignment type for `justify-content` and `align-content`.
/// Maps to Blink's `StyleContentAlignmentData` (style_content_alignment_data.h).
/// Stores position (4 bits) + distribution (3 bits) + overflow (2 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentAlignment {
    pub position: ContentPosition,
    pub distribution: ContentDistribution,
    pub overflow: OverflowAlignment,
}

impl ContentAlignment {
    /// Default: `normal` with no distribution, no overflow modifier.
    /// Matches Blink's `StyleContentAlignmentData(ContentPosition::kNormal,
    /// ContentDistributionType::kDefault, OverflowAlignment::kDefault)`.
    pub const INITIAL: Self = Self {
        position: ContentPosition::Normal,
        distribution: ContentDistribution::Default,
        overflow: OverflowAlignment::Default,
    };

    pub fn new(position: ContentPosition) -> Self {
        Self {
            position,
            distribution: ContentDistribution::Default,
            overflow: OverflowAlignment::Default,
        }
    }

    pub fn with_distribution(distribution: ContentDistribution) -> Self {
        Self {
            position: ContentPosition::Normal,
            distribution,
            overflow: OverflowAlignment::Default,
        }
    }
}

impl Default for ContentAlignment {
    fn default() -> Self { Self::INITIAL }
}

/// Compound self-alignment type for `align-items` and `align-self`.
/// Maps to Blink's `StyleSelfAlignmentData` (style_self_alignment_data.h).
/// Stores position (4 bits) + overflow (2 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemAlignment {
    pub position: ItemPosition,
    pub overflow: OverflowAlignment,
}

impl ItemAlignment {
    /// `align-items` initial: `normal` (resolves to `stretch` in flex context).
    pub const INITIAL_ITEMS: Self = Self {
        position: ItemPosition::Normal,
        overflow: OverflowAlignment::Default,
    };

    /// `align-self` initial: `auto` (inherits from parent `align-items`).
    pub const INITIAL_SELF: Self = Self {
        position: ItemPosition::Auto,
        overflow: OverflowAlignment::Default,
    };

    pub fn new(position: ItemPosition) -> Self {
        Self {
            position,
            overflow: OverflowAlignment::Default,
        }
    }

    pub fn with_overflow(position: ItemPosition, overflow: OverflowAlignment) -> Self {
        Self { position, overflow }
    }
}

impl Default for ItemAlignment {
    fn default() -> Self { Self::INITIAL_ITEMS }
}

// ── Text layout enums (extracted from Blink computed_style_constants.h) ──

/// CSS `line-height` computed value.
/// Blink: `Length` with `kAuto` for `normal`, or a fixed/percentage value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    /// `normal` — typically ~1.2× font-size, computed by the font.
    Normal,
    /// Unitless multiplier (e.g., `line-height: 1.5`).
    Number(f32),
    /// Absolute length in pixels (e.g., `line-height: 24px`).
    Length(f32),
    /// Percentage of font-size (e.g., `line-height: 150%` stored as 150.0).
    Percentage(f32),
}

impl Default for LineHeight {
    fn default() -> Self { Self::Normal }
}

/// CSS `text-align-last` property.
/// Blink: `ETextAlignLast` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextAlignLast {
    Auto = 0,
    Start = 1,
    End = 2,
    Left = 3,
    Right = 4,
    Center = 5,
    Justify = 6,
}

impl TextAlignLast {
    pub const INITIAL: Self = Self::Auto;
}

impl Default for TextAlignLast {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-justify` property.
/// Blink: `TextJustify` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextJustify {
    Auto = 0,
    None = 1,
    InterWord = 2,
    InterCharacter = 3,
}

impl TextJustify {
    pub const INITIAL: Self = Self::Auto;
}

impl Default for TextJustify {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `word-break` property.
/// Blink: `EWordBreak` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WordBreak {
    Normal = 0,
    BreakAll = 1,
    KeepAll = 2,
    BreakWord = 3,
}

impl WordBreak {
    pub const INITIAL: Self = Self::Normal;
}

impl Default for WordBreak {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `overflow-wrap` property (was `word-wrap`).
/// Blink: `EOverflowWrap` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OverflowWrap {
    Normal = 0,
    BreakWord = 1,
    Anywhere = 2,
}

impl OverflowWrap {
    pub const INITIAL: Self = Self::Normal;
}

impl Default for OverflowWrap {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `line-break` property.
/// Controls line breaking rules, especially for CJK (Chinese, Japanese, Korean) text.
///
/// Blink: `LineBreak` in `computed_style_base_constants.h`.
/// Blink converts this to a `LineBreakStrictness` enum and passes it as a
/// Unicode locale keyword (`@lb=loose|normal|strict`) to ICU's BreakIterator.
///
/// CSS Text Module Level 3 §5.2 — <https://www.w3.org/TR/css-text-3/#line-break-property>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LineBreak {
    /// Default browser behavior. Blink maps this to default strictness (same
    /// as `Normal` in practice — no ICU keyword).
    Auto = 0,
    /// Looser line breaking rules: allows more break opportunities in CJK text.
    /// Blink ICU keyword: `@lb=loose`.
    Loose = 1,
    /// Standard line breaking rules per UAX#14.
    /// Blink ICU keyword: `@lb=normal`.
    Normal = 2,
    /// Stricter line breaking rules: fewer breaks allowed in CJK text.
    /// Prohibits breaks before small kana, iteration marks, prolonged sound
    /// mark, and certain CJK punctuation.
    /// Blink ICU keyword: `@lb=strict`.
    Strict = 3,
    /// Allow breaks at every typographic character unit.
    /// Blink maps this to character-level breaking (no ICU keyword — uses
    /// character break type instead of line break type).
    Anywhere = 4,
}

impl LineBreak {
    /// Initial value: `auto` (Blink's `LineBreak::kAuto`).
    pub const INITIAL: Self = Self::Auto;
}

impl Default for LineBreak {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `hyphens` property.
/// Blink: `Hyphens` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Hyphens {
    None = 0,
    Manual = 1,
    Auto = 2,
}

impl Hyphens {
    pub const INITIAL: Self = Self::Manual;
}

impl Default for Hyphens {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-decoration-line` — bitflag set.
/// Blink: `TextDecorationLine` bitfield in `text_decoration.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextDecorationLine(pub u8);

impl TextDecorationLine {
    pub const NONE: Self = Self(0);
    pub const UNDERLINE: Self = Self(1);
    pub const OVERLINE: Self = Self(2);
    pub const LINE_THROUGH: Self = Self(4);

    #[inline]
    pub fn has_underline(self) -> bool { self.0 & 1 != 0 }
    #[inline]
    pub fn has_overline(self) -> bool { self.0 & 2 != 0 }
    #[inline]
    pub fn has_line_through(self) -> bool { self.0 & 4 != 0 }
    #[inline]
    pub fn is_none(self) -> bool { self.0 == 0 }
}

impl Default for TextDecorationLine {
    fn default() -> Self { Self::NONE }
}

/// CSS `text-decoration-style` property.
/// Blink: `ETextDecorationStyle` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextDecorationStyle {
    Solid = 0,
    Double = 1,
    Dotted = 2,
    Dashed = 3,
    Wavy = 4,
}

impl TextDecorationStyle {
    pub const INITIAL: Self = Self::Solid;
}

impl Default for TextDecorationStyle {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-decoration-thickness` computed value.
/// Blink: `TextDecorationThickness` in `text_decoration_thickness.h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextDecorationThickness {
    Auto,
    FromFont,
    Length(f32),
}

impl Default for TextDecorationThickness {
    fn default() -> Self { Self::Auto }
}

/// CSS `text-decoration-skip-ink` property.
///
/// Controls whether decoration lines (underline, overline) skip over glyph ink.
/// Blink: `ETextDecorationSkipInk` in `computed_style_base_constants.h`.
///
/// - `None`: decoration line is drawn continuously through glyph ink.
/// - `Auto` (default): decoration line skips glyph ink for non-CJK characters.
/// - `All`: decoration line skips glyph ink for all characters including CJK.
///
/// Per CSS spec, skip-ink does NOT apply to `line-through` decorations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextDecorationSkipInk {
    /// No skipping — draw continuous decoration line.
    None = 0,
    /// Skip glyph ink for non-CJK characters (default).
    Auto = 1,
    /// Skip glyph ink for all characters including CJK.
    All = 2,
}

impl TextDecorationSkipInk {
    /// Blink's initial value: `kAuto`.
    pub const INITIAL: Self = Self::Auto;
}

impl Default for TextDecorationSkipInk {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-underline-position` property.
/// Blink: `TextUnderlinePosition` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextUnderlinePosition {
    Auto = 0,
    Under = 1,
    Left = 2,
    Right = 3,
}

impl TextUnderlinePosition {
    pub const INITIAL: Self = Self::Auto;
}

impl Default for TextUnderlinePosition {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-transform` property.
/// Blink: `ETextTransform` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextTransform {
    None = 0,
    Capitalize = 1,
    Uppercase = 2,
    Lowercase = 3,
    FullWidth = 4,
    FullSizeKana = 5,
}

impl TextTransform {
    pub const INITIAL: Self = Self::None;
}

impl Default for TextTransform {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-overflow` property.
/// Blink: `ETextOverflow` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextOverflow {
    Clip = 0,
    Ellipsis = 1,
}

impl TextOverflow {
    pub const INITIAL: Self = Self::Clip;
}

impl Default for TextOverflow {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `vertical-align` computed value.
/// Blink: `EVerticalAlign` enum + numeric offset in `ComputedStyle`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    Baseline,
    Sub,
    Super,
    TextTop,
    TextBottom,
    Middle,
    Top,
    Bottom,
    /// Absolute offset from baseline in pixels.
    Length(f32),
    /// Percentage of line-height.
    Percentage(f32),
}

impl Default for VerticalAlign {
    fn default() -> Self { Self::Baseline }
}

/// CSS `unicode-bidi` property.
/// Blink: `UnicodeBidi` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum UnicodeBidi {
    Normal = 0,
    Embed = 1,
    Override = 2,
    Isolate = 3,
    IsolateOverride = 4,
    Plaintext = 5,
}

impl UnicodeBidi {
    pub const INITIAL: Self = Self::Normal;
}

impl Default for UnicodeBidi {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `writing-mode` property.
/// Blink: `WritingMode` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WritingMode {
    HorizontalTb = 0,
    VerticalRl = 1,
    VerticalLr = 2,
    SidewaysRl = 3,
    SidewaysLr = 4,
}

impl WritingMode {
    pub const INITIAL: Self = Self::HorizontalTb;

    /// True if text flows horizontally.
    #[inline]
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::HorizontalTb)
    }

    /// True if text flows vertically.
    #[inline]
    pub fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }

    /// Whether block direction goes right-to-left (vertical-rl, sideways-rl).
    ///
    /// Blink: `IsFlippedBlocksWritingMode()` in `writing_mode_utils.h`.
    /// Used during offset conversion — in vertical-rl the block-start edge
    /// is the physical *right* edge, so converting a logical block-offset
    /// to a physical left requires subtracting from the container width.
    #[inline]
    pub fn is_flipped_blocks(self) -> bool {
        matches!(self, Self::VerticalRl | Self::SidewaysRl)
    }

    /// Whether inline direction is bottom-to-top (sideways-lr).
    ///
    /// Blink: `IsFlippedLinesWritingMode()` in `writing_mode_utils.h`.
    /// sideways-lr is the only mode where the inline direction runs from
    /// bottom to top rather than top to bottom.
    #[inline]
    pub fn is_flipped_lines(self) -> bool {
        matches!(self, Self::SidewaysLr)
    }
}

impl Default for WritingMode {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-orientation` property.
/// Blink: `ETextOrientation` in computed_style_constants.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextOrientation {
    Mixed = 0,
    Upright = 1,
    Sideways = 2,
}

impl TextOrientation {
    pub const INITIAL: Self = Self::Mixed;
}

impl Default for TextOrientation {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-shadow` value.
/// Blink: `ShadowData` in `shadow_data.h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub color: crate::color::Color,
}

/// CSS `tab-size` computed value.
/// Blink: `TabSize` struct in `tab_size.h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabSize {
    /// Number of spaces (CSS default: 8).
    Spaces(u32),
    /// Explicit length in pixels.
    Length(f32),
}

impl Default for TabSize {
    fn default() -> Self { Self::Spaces(8) }
}

/// CSS `hanging-punctuation` property (CSS Text Module Level 3 §9).
///
/// NOTE: Not currently applied during layout — matching Chromium, which does
/// not implement this property. Stored for spec compliance and future use.
///
/// The property is a set of keywords (each may be on or off). `first` and
/// `force-end`/`allow-end` can appear together; `force-end` and `allow-end`
/// are mutually exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HangingPunctuation {
    /// `first` — opening bracket or quote at the start of the first line hangs.
    pub first: bool,
    /// `last` — closing bracket or quote at the end of the last line hangs.
    pub last: bool,
    /// `force-end` — stop/comma at the end of a line always hangs.
    pub force_end: bool,
    /// `allow-end` — stop/comma at the end of a line hangs if it doesn't fit.
    pub allow_end: bool,
}

impl HangingPunctuation {
    /// Initial value: `none` (all flags false).
    pub const NONE: Self = Self {
        first: false,
        last: false,
        force_end: false,
        allow_end: false,
    };

    /// Whether any hanging punctuation behaviour is requested.
    pub fn is_none(self) -> bool {
        !self.first && !self.last && !self.force_end && !self.allow_end
    }
}

impl Default for HangingPunctuation {
    fn default() -> Self { Self::NONE }
}

// ── Text Emphasis (CSS Text Decoration Module Level 3 §3) ───────────────

/// CSS `text-emphasis-style` — shape of emphasis marks.
///
/// Blink: `TextEmphasisMark` in `computed_style_constants.h`.
///
/// CSS Text Decoration Module Level 3 §3.4
/// <https://www.w3.org/TR/css-text-decor-3/#text-emphasis-style-property>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextEmphasisMark {
    /// No emphasis marks.
    None,
    /// Dot: '•' (filled) / '◦' (open). Default for CJK horizontal text.
    Dot,
    /// Circle: '●' (filled) / '○' (open).
    Circle,
    /// Double circle: '◉' (filled) / '◎' (open).
    DoubleCircle,
    /// Triangle: '▲' (filled) / '△' (open).
    Triangle,
    /// Sesame: '﹅' (filled) / '﹆' (open). Default for CJK vertical text.
    Sesame,
    /// Custom single character specified by the author.
    Custom(char),
}

impl TextEmphasisMark {
    pub const INITIAL: Self = Self::None;

    /// Returns the Unicode character for this mark shape and fill.
    ///
    /// Custom marks ignore the fill parameter and return the stored character.
    /// Returns `None` for `TextEmphasisMark::None`.
    pub fn character(self, fill: TextEmphasisFill) -> Option<char> {
        match (self, fill) {
            (Self::None, _) => None,
            (Self::Dot, TextEmphasisFill::Filled) => Some('\u{2022}'),       // •
            (Self::Dot, TextEmphasisFill::Open) => Some('\u{25E6}'),         // ◦
            (Self::Circle, TextEmphasisFill::Filled) => Some('\u{25CF}'),    // ●
            (Self::Circle, TextEmphasisFill::Open) => Some('\u{25CB}'),      // ○
            (Self::DoubleCircle, TextEmphasisFill::Filled) => Some('\u{25C9}'), // ◉
            (Self::DoubleCircle, TextEmphasisFill::Open) => Some('\u{25CE}'),   // ◎
            (Self::Triangle, TextEmphasisFill::Filled) => Some('\u{25B2}'),  // ▲
            (Self::Triangle, TextEmphasisFill::Open) => Some('\u{25B3}'),    // △
            (Self::Sesame, TextEmphasisFill::Filled) => Some('\u{FE45}'),    // ﹅
            (Self::Sesame, TextEmphasisFill::Open) => Some('\u{FE46}'),      // ﹆
            (Self::Custom(ch), _) => Some(ch),
        }
    }
}

impl Default for TextEmphasisMark {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-emphasis-style` fill mode — filled or open marks.
///
/// Blink: `TextEmphasisFill` in `computed_style_constants.h`.
///
/// CSS Text Decoration Module Level 3 §3.4
/// <https://www.w3.org/TR/css-text-decor-3/#text-emphasis-style-property>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextEmphasisFill {
    /// Filled marks (default per CSS spec).
    Filled = 0,
    /// Open (outline) marks.
    Open = 1,
}

impl TextEmphasisFill {
    pub const INITIAL: Self = Self::Filled;
}

impl Default for TextEmphasisFill {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `text-emphasis-position` — placement of emphasis marks.
///
/// Blink: `TextEmphasisPosition` in `computed_style_constants.h`.
/// Stored as two independent axes: over/under and right/left.
///
/// CSS Text Decoration Module Level 3 §3.5
/// <https://www.w3.org/TR/css-text-decor-3/#text-emphasis-position-property>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextEmphasisPosition {
    /// `true` = over/above the text, `false` = under/below.
    /// Initial: `true` (over) for horizontal text.
    pub over: bool,
    /// `true` = right side (for vertical text), `false` = left side.
    /// Initial: `true` (right) for vertical text.
    pub right: bool,
}

impl TextEmphasisPosition {
    /// Initial value: `over right` per CSS spec.
    pub const INITIAL: Self = Self { over: true, right: true };
}

impl Default for TextEmphasisPosition {
    fn default() -> Self { Self::INITIAL }
}

// ── Text Combine Upright (CSS Writing Modes Level 3 §9.1) ───────────────

/// CSS `text-combine-upright` — tate-chū-yoko (horizontal-in-vertical).
///
/// Combines multiple characters into a single upright glyph in vertical text.
///
/// Blink: `TextCombine` in `computed_style_constants.h`.
///
/// CSS Writing Modes Level 3 §9.1
/// <https://www.w3.org/TR/css-writing-modes-3/#text-combine-upright>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextCombineUpright {
    /// No combination — characters rendered individually.
    None = 0,
    /// All consecutive characters are combined into a single upright glyph.
    All = 1,
}

impl TextCombineUpright {
    pub const INITIAL: Self = Self::None;
}

impl Default for TextCombineUpright {
    fn default() -> Self { Self::INITIAL }
}

// ── Font Palette ────────────────────────────────────────────────────────

/// CSS `font-palette` property.
///
/// Controls which color palette is used for color fonts (COLR/CPAL).
/// Color fonts can define multiple palettes of colors; this property
/// selects which one to use for rendering.
///
/// Blink: `FontPalette` in `third_party/blink/renderer/platform/fonts/font_palette.h`.
/// CSS Fonts Level 4: <https://drafts.csswg.org/css-fonts-4/#font-palette-prop>
#[derive(Debug, Clone, PartialEq)]
pub enum FontPalette {
    /// Use the font's default palette (palette index 0).
    Normal,
    /// Use a light-mode optimized palette.
    /// The font declares which palette is suitable for light backgrounds.
    Light,
    /// Use a dark-mode optimized palette.
    /// The font declares which palette is suitable for dark backgrounds.
    Dark,
    /// Use a custom palette defined via `@font-palette-values` by name.
    Custom(String),
}

impl FontPalette {
    /// CSS initial value: `normal`.
    pub const INITIAL: Self = Self::Normal;

    /// True if this is the default (normal) palette.
    #[inline]
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }
}

impl Default for FontPalette {
    fn default() -> Self { Self::INITIAL }
}

// ── Font Orientation (from WritingMode + TextOrientation) ───────────────

/// Resolved font orientation — determines how glyphs are laid out in the
/// inline direction.
///
/// Derived from the combination of `writing-mode` and `text-orientation`.
/// Blink: `FontOrientation` in `platform/fonts/font_orientation.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FontOrientation {
    /// Horizontal layout — standard for `writing-mode: horizontal-tb`.
    Horizontal = 0,
    /// All glyphs rotated 90° clockwise — `text-orientation: sideways` or
    /// `sideways-rl` / `sideways-lr` writing modes.
    VerticalRotated = 1,
    /// CJK glyphs upright, other scripts rotated — `text-orientation: mixed`
    /// (the default for vertical writing modes).
    VerticalMixed = 2,
    /// All glyphs upright — `text-orientation: upright`.
    VerticalUpright = 3,
}

impl FontOrientation {
    /// True when the font uses vertical metrics (upright or mixed modes
    /// for characters that are upright).
    #[inline]
    pub fn is_vertical(self) -> bool {
        !matches!(self, Self::Horizontal)
    }

    /// True when all glyphs are rotated (horizontal or sideways).
    /// HarfBuzz should shape with `HB_DIRECTION_LTR` and the canvas
    /// rotation handles the vertical appearance.
    #[inline]
    pub fn uses_horizontal_shaping(self) -> bool {
        matches!(self, Self::Horizontal | Self::VerticalRotated)
    }

    /// True when at least some glyphs use vertical metrics.
    #[inline]
    pub fn uses_vertical_metrics(self) -> bool {
        matches!(self, Self::VerticalMixed | Self::VerticalUpright)
    }
}

impl Default for FontOrientation {
    fn default() -> Self { Self::Horizontal }
}

/// Derive the font orientation from CSS `writing-mode` and `text-orientation`.
///
/// Blink: `ComputedStyleUtils::ResolvedFontOrientation` and
/// `FontOrientation` constructor logic in `font_description.h`.
pub fn font_orientation(writing_mode: WritingMode, text_orientation: TextOrientation) -> FontOrientation {
    if writing_mode.is_horizontal() {
        FontOrientation::Horizontal
    } else if matches!(writing_mode, WritingMode::SidewaysRl | WritingMode::SidewaysLr) {
        // CSS Writing Modes Level 4 §7.2: text-orientation has no effect in
        // sideways-rl / sideways-lr — all glyphs are rotated.
        FontOrientation::VerticalRotated
    } else {
        match text_orientation {
            TextOrientation::Mixed => FontOrientation::VerticalMixed,
            TextOrientation::Upright => FontOrientation::VerticalUpright,
            TextOrientation::Sideways => FontOrientation::VerticalRotated,
        }
    }
}

// ── Ruby Annotation ─────────────────────────────────────────────────────

/// CSS `ruby-position` property.
///
/// Determines where ruby annotation text is rendered relative to the base text.
/// Blink: `ERubyPosition` in `computed_style_constants.h`.
///
/// Reference: CSS Ruby Annotation Layout Module Level 1 §3.1
/// <https://drafts.csswg.org/css-ruby-1/#ruby-position-property>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RubyPosition {
    /// Annotation appears above (horizontal) or to the right (vertical) of the
    /// base text. This is the default for Japanese furigana.
    Over = 0,
    /// Annotation appears below (horizontal) or to the left (vertical) of the
    /// base text. Used for Zhuyin (bopomofo) in Traditional Chinese.
    Under = 1,
}

impl RubyPosition {
    /// Blink's initial value: `kOver`.
    pub const INITIAL: Self = Self::Over;

    /// True when the annotation is positioned above/right of base text.
    #[inline]
    pub fn is_over(self) -> bool {
        self == Self::Over
    }

    /// True when the annotation is positioned below/left of base text.
    #[inline]
    pub fn is_under(self) -> bool {
        self == Self::Under
    }
}

impl Default for RubyPosition {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `ruby-align` property.
///
/// Controls how inline-level content of a ruby annotation box is aligned
/// with respect to its base.
/// Blink: `ERubyAlign` in `computed_style_constants.h`.
///
/// Reference: CSS Ruby Annotation Layout Module Level 1 §4.1
/// <https://drafts.csswg.org/css-ruby-1/#ruby-align-property>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RubyAlign {
    /// The content is expanded so that it is evenly distributed across the
    /// width of the ruby annotation box, with space inserted before the first
    /// and after the last character. This is the default value.
    SpaceAround = 0,
    /// The content is centered within the ruby annotation box.
    Center = 1,
    /// The content is expanded so that it is evenly distributed across the
    /// width, but no extra space is inserted before the first or after the
    /// last character (like CSS `text-align: justify`).
    SpaceBetween = 2,
    /// The content is aligned to the start edge of the ruby annotation box.
    Start = 3,
}

impl RubyAlign {
    /// Blink's initial value: `kSpaceAround`.
    pub const INITIAL: Self = Self::SpaceAround;
}

impl Default for RubyAlign {
    fn default() -> Self { Self::INITIAL }
}

// ── Fragmentation enums (extracted from Blink computed_style_constants.h) ──

/// CSS `break-before` / `break-after` computed value.
///
/// Blink: `EBreakBetween` in `computed_style_base_constants.h`.
/// Maps both modern `break-before`/`break-after` and legacy
/// `page-break-before`/`page-break-after` properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BreakValue {
    Auto = 0,
    Avoid = 1,
    AvoidPage = 2,
    AvoidColumn = 3,
    Page = 4,
    Column = 5,
    Left = 6,
    Right = 7,
    /// `break-before: always` / `break-after: always` — force a break.
    Always = 8,
}

impl BreakValue {
    pub const INITIAL: Self = Self::Auto;

    /// True if this value forces a break.
    #[inline]
    pub fn is_forced(self) -> bool {
        matches!(self, Self::Always | Self::Page | Self::Column | Self::Left | Self::Right)
    }

    /// True if this value requests avoiding a break.
    #[inline]
    pub fn is_avoid(self) -> bool {
        matches!(self, Self::Avoid | Self::AvoidPage | Self::AvoidColumn)
    }

    /// Convert from legacy `page-break-before`/`page-break-after` keywords.
    /// `always` → `Page`, `avoid` → `Avoid`, `left` → `Left`, `right` → `Right`.
    #[inline]
    pub fn from_legacy_page_break(value: &str) -> Self {
        match value {
            "auto" => Self::Auto,
            "always" => Self::Page,
            "avoid" => Self::Avoid,
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Auto,
        }
    }
}

impl Default for BreakValue {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `break-inside` computed value.
///
/// Blink: `EBreakInside` in `computed_style_base_constants.h`.
/// Maps both modern `break-inside` and legacy `page-break-inside`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BreakInside {
    Auto = 0,
    Avoid = 1,
    AvoidPage = 2,
    AvoidColumn = 3,
}

impl BreakInside {
    pub const INITIAL: Self = Self::Auto;

    /// True if this value requests avoiding breaks inside.
    #[inline]
    pub fn is_avoid(self) -> bool {
        matches!(self, Self::Avoid | Self::AvoidPage | Self::AvoidColumn)
    }

    /// Convert from legacy `page-break-inside` keywords.
    #[inline]
    pub fn from_legacy_page_break_inside(value: &str) -> Self {
        match value {
            "auto" => Self::Auto,
            "avoid" => Self::Avoid,
            _ => Self::Auto,
        }
    }
}

impl Default for BreakInside {
    fn default() -> Self { Self::INITIAL }
}

// ── Multicol enums (CSS Multi-column Layout Module Level 1) ───────────

/// CSS `column-fill` property.
///
/// Blink: `EColumnFill` in `computed_style_base_constants.h`.
/// Controls whether columns are balanced or filled sequentially.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColumnFill {
    /// Balance column heights so they are approximately equal.
    Balance = 0,
    /// Balance all columns, including in overflow (fragmented) containers.
    BalanceAll = 1,
    /// Fill columns sequentially; do not balance.
    Auto = 2,
}

impl ColumnFill {
    pub const INITIAL: Self = Self::Balance;
}

impl Default for ColumnFill {
    fn default() -> Self { Self::INITIAL }
}

/// CSS `column-span` property.
///
/// Blink: `EColumnSpan` in `computed_style_base_constants.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColumnSpan {
    /// Element does not span columns.
    None = 0,
    /// Element spans all columns of the nearest multicol ancestor.
    All = 1,
}

impl ColumnSpan {
    pub const INITIAL: Self = Self::None;
}

impl Default for ColumnSpan {
    fn default() -> Self { Self::INITIAL }
}
