//! CSS style enums — extracted from Blink's `computed_style_constants.h`.
//!
//! Source: out/Release/gen/third_party/blink/renderer/core/style/computed_style_base_constants.h
//! and core/style/computed_style_constants.h

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
