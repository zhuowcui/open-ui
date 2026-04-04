//! Type-safe CSS types, error handling, and geometry helpers.
//!
//! This module provides idiomatic Rust types that map to the C enums and
//! structs in `openui_sys`, along with error types and conversion utilities.

use openui_sys::{
    OuiAlignItems, OuiBitmap, OuiDisplay, OuiFlexDirection, OuiFlexWrap, OuiFontStyle,
    OuiJustifyContent, OuiOverflow, OuiPosition, OuiStatus, OuiTextAlign,
};

// ─── Length (re-export with convenience constructors) ────────

/// A typed CSS length value (e.g. `px(200.0)`, `pct(50.0)`, `auto()`).
pub type Length = openui_sys::OuiLength;

/// Create a length in pixels.
pub fn px(val: f32) -> Length {
    Length::px(val)
}

/// Create a percentage length.
pub fn pct(val: f32) -> Length {
    Length::pct(val)
}

/// Create a length in em units.
pub fn em(val: f32) -> Length {
    Length::em(val)
}

/// Create a length in rem units.
pub fn rem(val: f32) -> Length {
    Length::rem(val)
}

/// Create a length in viewport-width units.
pub fn vw(val: f32) -> Length {
    Length::vw(val)
}

/// Create a length in viewport-height units.
pub fn vh(val: f32) -> Length {
    Length::vh(val)
}

/// Create a length in fr (fractional) units.
pub fn fr(val: f32) -> Length {
    Length::fr(val)
}

/// Create an `auto` length.
pub fn auto() -> Length {
    Length::auto()
}

// ─── Rect ───────────────────────────────────────────────────

/// A bounding rectangle in layout coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X offset from the document origin.
    pub x: f32,
    /// Y offset from the document origin.
    pub y: f32,
    /// Width of the rectangle.
    pub width: f32,
    /// Height of the rectangle.
    pub height: f32,
}

impl From<openui_sys::OuiRect> for Rect {
    fn from(r: openui_sys::OuiRect) -> Self {
        Rect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }
    }
}

// ─── Bitmap ─────────────────────────────────────────────────

/// An RGBA bitmap rendered from a document.
///
/// The pixel data is owned by the Blink rendering engine and is freed
/// automatically when this value is dropped.
pub struct Bitmap {
    pub(crate) raw: OuiBitmap,
}

impl Bitmap {
    /// Access the raw RGBA pixel data as a byte slice.
    pub fn pixels(&self) -> &[u8] {
        if self.raw.pixels.is_null() {
            &[]
        } else {
            unsafe {
                std::slice::from_raw_parts(
                    self.raw.pixels,
                    (self.raw.stride * self.raw.height) as usize,
                )
            }
        }
    }

    /// Bitmap width in pixels.
    pub fn width(&self) -> i32 {
        self.raw.width
    }

    /// Bitmap height in pixels.
    pub fn height(&self) -> i32 {
        self.raw.height
    }

    /// Row stride in bytes (may be wider than `width * 4`).
    pub fn stride(&self) -> i32 {
        self.raw.stride
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        if !self.raw.pixels.is_null() {
            unsafe { openui_sys::oui_bitmap_free(&mut self.raw) };
        }
    }
}

// ─── OuiError ───────────────────────────────────────────────

/// Errors returned by Open UI operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OuiError {
    /// The library has not been initialized.
    NotInitialized,
    /// The library was already initialized.
    AlreadyInitialized,
    /// An invalid argument was supplied (e.g. interior NUL in a string).
    InvalidArgument,
    /// The specified tag name is not recognised.
    UnknownTag,
    /// The specified CSS property name is not recognised.
    UnknownProperty,
    /// The CSS value could not be parsed.
    InvalidValue,
    /// A layout pass is required before querying geometry.
    LayoutRequired,
    /// An internal Blink error occurred.
    Internal,
    /// Object creation failed (returned null pointer).
    CreationFailed,
}

impl std::fmt::Display for OuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OuiError::NotInitialized => write!(f, "library not initialized"),
            OuiError::AlreadyInitialized => write!(f, "library already initialized"),
            OuiError::InvalidArgument => write!(f, "invalid argument"),
            OuiError::UnknownTag => write!(f, "unknown tag name"),
            OuiError::UnknownProperty => write!(f, "unknown CSS property"),
            OuiError::InvalidValue => write!(f, "invalid CSS value"),
            OuiError::LayoutRequired => write!(f, "layout pass required"),
            OuiError::Internal => write!(f, "internal error"),
            OuiError::CreationFailed => write!(f, "object creation failed"),
        }
    }
}

impl std::error::Error for OuiError {}

/// Convert an FFI status code to a Rust `Result`.
pub(crate) fn check_status(status: OuiStatus) -> Result<(), OuiError> {
    match status {
        OuiStatus::OUI_OK => Ok(()),
        OuiStatus::OUI_ERROR_NOT_INITIALIZED => Err(OuiError::NotInitialized),
        OuiStatus::OUI_ERROR_ALREADY_INITIALIZED => Err(OuiError::AlreadyInitialized),
        OuiStatus::OUI_ERROR_INVALID_ARGUMENT => Err(OuiError::InvalidArgument),
        OuiStatus::OUI_ERROR_UNKNOWN_TAG => Err(OuiError::UnknownTag),
        OuiStatus::OUI_ERROR_UNKNOWN_PROPERTY => Err(OuiError::UnknownProperty),
        OuiStatus::OUI_ERROR_INVALID_VALUE => Err(OuiError::InvalidValue),
        OuiStatus::OUI_ERROR_LAYOUT_REQUIRED => Err(OuiError::LayoutRequired),
        OuiStatus::OUI_ERROR_INTERNAL => Err(OuiError::Internal),
    }
}

// ─── CSS Enums ──────────────────────────────────────────────

/// CSS `display` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// Block-level element.
    Block,
    /// Inline element.
    Inline,
    /// Inline-level block container.
    InlineBlock,
    /// Flex container.
    Flex,
    /// Inline flex container.
    InlineFlex,
    /// Grid container.
    Grid,
    /// Inline grid container.
    InlineGrid,
    /// Table element.
    Table,
    /// Table row element.
    TableRow,
    /// Table cell element.
    TableCell,
    /// Hidden element (removed from layout).
    None,
    /// Element's children participate in its parent's layout.
    Contents,
}

impl From<Display> for OuiDisplay {
    fn from(d: Display) -> Self {
        match d {
            Display::Block => OuiDisplay::OUI_DISPLAY_BLOCK,
            Display::Inline => OuiDisplay::OUI_DISPLAY_INLINE,
            Display::InlineBlock => OuiDisplay::OUI_DISPLAY_INLINE_BLOCK,
            Display::Flex => OuiDisplay::OUI_DISPLAY_FLEX,
            Display::InlineFlex => OuiDisplay::OUI_DISPLAY_INLINE_FLEX,
            Display::Grid => OuiDisplay::OUI_DISPLAY_GRID,
            Display::InlineGrid => OuiDisplay::OUI_DISPLAY_INLINE_GRID,
            Display::Table => OuiDisplay::OUI_DISPLAY_TABLE,
            Display::TableRow => OuiDisplay::OUI_DISPLAY_TABLE_ROW,
            Display::TableCell => OuiDisplay::OUI_DISPLAY_TABLE_CELL,
            Display::None => OuiDisplay::OUI_DISPLAY_NONE,
            Display::Contents => OuiDisplay::OUI_DISPLAY_CONTENTS,
        }
    }
}

/// CSS `position` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Normal flow.
    Static,
    /// Offset relative to normal position.
    Relative,
    /// Removed from flow, positioned relative to nearest positioned ancestor.
    Absolute,
    /// Positioned relative to the viewport.
    Fixed,
    /// Toggles between relative and fixed based on scroll position.
    Sticky,
}

impl From<Position> for OuiPosition {
    fn from(p: Position) -> Self {
        match p {
            Position::Static => OuiPosition::OUI_POSITION_STATIC,
            Position::Relative => OuiPosition::OUI_POSITION_RELATIVE,
            Position::Absolute => OuiPosition::OUI_POSITION_ABSOLUTE,
            Position::Fixed => OuiPosition::OUI_POSITION_FIXED,
            Position::Sticky => OuiPosition::OUI_POSITION_STICKY,
        }
    }
}

/// CSS `overflow` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Content is not clipped.
    Visible,
    /// Content is clipped without scrollbars.
    Hidden,
    /// Scrollbars are always visible.
    Scroll,
    /// Scrollbars appear only when needed.
    Auto,
}

impl From<Overflow> for OuiOverflow {
    fn from(o: Overflow) -> Self {
        match o {
            Overflow::Visible => OuiOverflow::OUI_OVERFLOW_VISIBLE,
            Overflow::Hidden => OuiOverflow::OUI_OVERFLOW_HIDDEN,
            Overflow::Scroll => OuiOverflow::OUI_OVERFLOW_SCROLL,
            Overflow::Auto => OuiOverflow::OUI_OVERFLOW_AUTO,
        }
    }
}

/// CSS `flex-direction` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    /// Main axis runs left-to-right.
    Row,
    /// Main axis runs right-to-left.
    RowReverse,
    /// Main axis runs top-to-bottom.
    Column,
    /// Main axis runs bottom-to-top.
    ColumnReverse,
}

impl From<FlexDirection> for OuiFlexDirection {
    fn from(d: FlexDirection) -> Self {
        match d {
            FlexDirection::Row => OuiFlexDirection::OUI_FLEX_ROW,
            FlexDirection::RowReverse => OuiFlexDirection::OUI_FLEX_ROW_REVERSE,
            FlexDirection::Column => OuiFlexDirection::OUI_FLEX_COLUMN,
            FlexDirection::ColumnReverse => OuiFlexDirection::OUI_FLEX_COLUMN_REVERSE,
        }
    }
}

/// CSS `flex-wrap` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    /// Items stay on a single line.
    NoWrap,
    /// Items wrap to new lines.
    Wrap,
    /// Items wrap in reverse order.
    WrapReverse,
}

impl From<FlexWrap> for OuiFlexWrap {
    fn from(w: FlexWrap) -> Self {
        match w {
            FlexWrap::NoWrap => OuiFlexWrap::OUI_FLEX_WRAP_NOWRAP,
            FlexWrap::Wrap => OuiFlexWrap::OUI_FLEX_WRAP_WRAP,
            FlexWrap::WrapReverse => OuiFlexWrap::OUI_FLEX_WRAP_WRAP_REVERSE,
        }
    }
}

/// CSS `align-items` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    /// Items stretch to fill the cross axis.
    Stretch,
    /// Items align to the start of the cross axis.
    FlexStart,
    /// Items align to the end of the cross axis.
    FlexEnd,
    /// Items are centred on the cross axis.
    Center,
    /// Items align along their text baseline.
    Baseline,
}

impl From<AlignItems> for OuiAlignItems {
    fn from(a: AlignItems) -> Self {
        match a {
            AlignItems::Stretch => OuiAlignItems::OUI_ALIGN_STRETCH,
            AlignItems::FlexStart => OuiAlignItems::OUI_ALIGN_FLEX_START,
            AlignItems::FlexEnd => OuiAlignItems::OUI_ALIGN_FLEX_END,
            AlignItems::Center => OuiAlignItems::OUI_ALIGN_CENTER,
            AlignItems::Baseline => OuiAlignItems::OUI_ALIGN_BASELINE,
        }
    }
}

/// CSS `justify-content` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    /// Items pack toward the start of the main axis.
    FlexStart,
    /// Items pack toward the end of the main axis.
    FlexEnd,
    /// Items are centred on the main axis.
    Center,
    /// Equal space between items, none at edges.
    SpaceBetween,
    /// Equal space around each item.
    SpaceAround,
    /// Equal space between and at the edges.
    SpaceEvenly,
}

impl From<JustifyContent> for OuiJustifyContent {
    fn from(j: JustifyContent) -> Self {
        match j {
            JustifyContent::FlexStart => OuiJustifyContent::OUI_JUSTIFY_FLEX_START,
            JustifyContent::FlexEnd => OuiJustifyContent::OUI_JUSTIFY_FLEX_END,
            JustifyContent::Center => OuiJustifyContent::OUI_JUSTIFY_CENTER,
            JustifyContent::SpaceBetween => OuiJustifyContent::OUI_JUSTIFY_SPACE_BETWEEN,
            JustifyContent::SpaceAround => OuiJustifyContent::OUI_JUSTIFY_SPACE_AROUND,
            JustifyContent::SpaceEvenly => OuiJustifyContent::OUI_JUSTIFY_SPACE_EVENLY,
        }
    }
}

/// CSS `text-align` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Left-aligned text.
    Left,
    /// Right-aligned text.
    Right,
    /// Centred text.
    Center,
    /// Justified text.
    Justify,
}

impl From<TextAlign> for OuiTextAlign {
    fn from(a: TextAlign) -> Self {
        match a {
            TextAlign::Left => OuiTextAlign::OUI_TEXT_ALIGN_LEFT,
            TextAlign::Right => OuiTextAlign::OUI_TEXT_ALIGN_RIGHT,
            TextAlign::Center => OuiTextAlign::OUI_TEXT_ALIGN_CENTER,
            TextAlign::Justify => OuiTextAlign::OUI_TEXT_ALIGN_JUSTIFY,
        }
    }
}

/// CSS `font-style` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    /// Normal (upright) text.
    Normal,
    /// Italic text.
    Italic,
    /// Oblique text.
    Oblique,
}

impl From<FontStyle> for OuiFontStyle {
    fn from(s: FontStyle) -> Self {
        match s {
            FontStyle::Normal => OuiFontStyle::OUI_FONT_STYLE_NORMAL,
            FontStyle::Italic => OuiFontStyle::OUI_FONT_STYLE_ITALIC,
            FontStyle::Oblique => OuiFontStyle::OUI_FONT_STYLE_OBLIQUE,
        }
    }
}

// ─── Compile-time trait assertions ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_implements_std_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<OuiError>();
    }

    #[test]
    fn error_display() {
        assert_eq!(OuiError::NotInitialized.to_string(), "library not initialized");
        assert_eq!(OuiError::CreationFailed.to_string(), "object creation failed");
    }

    #[test]
    fn length_constructors() {
        let l = px(200.0);
        assert_eq!(l.value, 200.0);

        let l = pct(50.0);
        assert_eq!(l.value, 50.0);

        let l = auto();
        assert_eq!(l.unit, openui_sys::OuiUnit::OUI_UNIT_AUTO);
    }

    #[test]
    fn rect_from_oui_rect() {
        let oui_rect = openui_sys::OuiRect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        };
        let rect: Rect = oui_rect.into();
        assert_eq!(rect.x, 1.0);
        assert_eq!(rect.height, 4.0);
    }

    #[test]
    fn css_enum_conversions() {
        let _: openui_sys::OuiDisplay = Display::Flex.into();
        let _: openui_sys::OuiPosition = Position::Absolute.into();
        let _: openui_sys::OuiOverflow = Overflow::Hidden.into();
        let _: openui_sys::OuiFlexDirection = FlexDirection::Column.into();
        let _: openui_sys::OuiFlexWrap = FlexWrap::Wrap.into();
        let _: openui_sys::OuiAlignItems = AlignItems::Center.into();
        let _: openui_sys::OuiJustifyContent = JustifyContent::SpaceBetween.into();
        let _: openui_sys::OuiTextAlign = TextAlign::Center.into();
        let _: openui_sys::OuiFontStyle = FontStyle::Italic.into();
    }

    #[test]
    fn check_status_ok() {
        assert!(check_status(openui_sys::OuiStatus::OUI_OK).is_ok());
    }

    #[test]
    fn check_status_error() {
        assert_eq!(
            check_status(openui_sys::OuiStatus::OUI_ERROR_INVALID_ARGUMENT),
            Err(OuiError::InvalidArgument),
        );
    }
}
