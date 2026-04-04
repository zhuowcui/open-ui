//! ComputedStyle — the resolved style for a single element.
//!
//! Extracted from Blink's `ComputedStyle` (core/style/computed_style.h)
//! and the generated `ComputedStyleBase` (computed_style_base.h).
//!
//! Every property here has the exact same initial value as Blink's. Fields
//! that Blink bit-packs are stored as typed enums. Lengths use `openui_geometry::Length`.

use openui_geometry::Length;

use crate::color::{Color, StyleColor};
use crate::enums::*;

/// The complete resolved style for an element.
///
/// Mirrors Blink's `ComputedStyle`. Only the properties needed for SP9
/// (block layout + box painting) are included. More will be added in later SPs.
///
/// All initial values match Blink's `computed_style_initial_values.h`.
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // ── Display & Positioning (bit-packed in Blink) ──────────────────

    /// CSS `display`. Initial: `inline` (Blink's `EDisplay::kInline`).
    pub display: Display,

    /// CSS `position`. Initial: `static`.
    pub position: Position,

    /// CSS `float`. Initial: `none`.
    pub float: Float,

    /// CSS `clear`. Initial: `none`.
    pub clear: Clear,

    /// CSS `overflow-x`. Initial: `visible`.
    pub overflow_x: Overflow,

    /// CSS `overflow-y`. Initial: `visible`.
    pub overflow_y: Overflow,

    /// CSS `box-sizing`. Initial: `content-box`.
    pub box_sizing: BoxSizing,

    /// CSS `visibility`. Initial: `visible`. Inherited.
    pub visibility: Visibility,

    /// CSS `direction`. Initial: `ltr`. Inherited.
    pub direction: Direction,

    // ── Sizing (stored as Length in Blink's box_data_) ───────────────

    /// CSS `width`. Initial: `auto`.
    pub width: Length,

    /// CSS `height`. Initial: `auto`.
    pub height: Length,

    /// CSS `min-width`. Initial: `auto` (Blink uses auto for min-*).
    pub min_width: Length,

    /// CSS `min-height`. Initial: `auto`.
    pub min_height: Length,

    /// CSS `max-width`. Initial: `none` (NOT auto — Blink uses Length::None).
    pub max_width: Length,

    /// CSS `max-height`. Initial: `none`.
    pub max_height: Length,

    // ── Margins (stored as Length in Blink's box_data_) ──────────────

    /// CSS `margin-top`. Initial: `0px`. Can be `auto`.
    pub margin_top: Length,
    pub margin_right: Length,
    pub margin_bottom: Length,
    pub margin_left: Length,

    // ── Padding (stored as Length in Blink's box_data_) ──────────────
    // Padding cannot be auto or negative per CSS spec.

    /// CSS `padding-top`. Initial: `0px`.
    pub padding_top: Length,
    pub padding_right: Length,
    pub padding_bottom: Length,
    pub padding_left: Length,

    // ── Border widths (pre-resolved integers in Blink) ───────────────
    // Blink stores border widths as `int` (32-bit), already resolved to pixels.
    // The initial computed value is 3px (medium), but since initial border-style
    // is `none`, the used value is 0. We store the resolved int.

    /// Border width in pixels. Blink initial: 3 (but used as 0 when style=none).
    pub border_top_width: i32,
    pub border_right_width: i32,
    pub border_bottom_width: i32,
    pub border_left_width: i32,

    // ── Border styles ────────────────────────────────────────────────

    pub border_top_style: BorderStyle,
    pub border_right_style: BorderStyle,
    pub border_bottom_style: BorderStyle,
    pub border_left_style: BorderStyle,

    // ── Border colors (StyleColor — defaults to currentColor) ────────

    pub border_top_color: StyleColor,
    pub border_right_color: StyleColor,
    pub border_bottom_color: StyleColor,
    pub border_left_color: StyleColor,

    // ── Colors ───────────────────────────────────────────────────────

    /// CSS `background-color`. Initial: `transparent`.
    pub background_color: Color,

    /// CSS `color` (inherited). Initial: `black` (CanvasText in Blink,
    /// but we use black for simplicity — matches most user agents).
    pub color: Color,

    /// CSS `opacity`. Initial: `1.0`.
    pub opacity: f32,

    // ── Z-index ──────────────────────────────────────────────────────

    /// CSS `z-index`. `None` means `auto` (no stacking context).
    /// Blink stores this as `int` with a separate `HasAutoZIndex()` flag.
    pub z_index: Option<i32>,

    // ── Flexbox properties ───────────────────────────────────────────
    // Source: Blink css_properties.json5 + computed_style_base.h

    /// CSS `flex-direction`. Initial: `row`. Container property.
    pub flex_direction: FlexDirection,

    /// CSS `flex-wrap`. Initial: `nowrap`. Container property.
    pub flex_wrap: FlexWrap,

    /// CSS `justify-content`. Initial: `normal`. Container property.
    /// In flex context, `normal` behaves like `flex-start`.
    pub justify_content: ContentAlignment,

    /// CSS `align-items`. Initial: `normal`. Container property.
    /// In flex context, `normal` behaves like `stretch`.
    pub align_items: ItemAlignment,

    /// CSS `align-content`. Initial: `normal`. Container property.
    /// In flex context, `normal` behaves like `stretch` for multi-line.
    pub align_content: ContentAlignment,

    /// CSS `row-gap`. `None` means `normal` (0px for flex).
    pub row_gap: Option<Length>,

    /// CSS `column-gap`. `None` means `normal` (0px for flex).
    pub column_gap: Option<Length>,

    /// CSS `flex-grow`. Initial: `0`. Item property.
    pub flex_grow: f32,

    /// CSS `flex-shrink`. Initial: `1`. Item property.
    pub flex_shrink: f32,

    /// CSS `flex-basis`. Initial: `auto`. Item property.
    pub flex_basis: Length,

    /// CSS `align-self`. Initial: `auto` (inherits from `align-items`).
    pub align_self: ItemAlignment,

    /// CSS `order`. Initial: `0`. Item property.
    pub order: i32,
}

impl ComputedStyle {
    /// Create a style with all initial values matching Blink's defaults.
    pub fn initial() -> Self {
        Self {
            display: Display::INITIAL,       // inline
            position: Position::INITIAL,     // static
            float: Float::INITIAL,           // none
            clear: Clear::INITIAL,           // none
            overflow_x: Overflow::INITIAL,   // visible
            overflow_y: Overflow::INITIAL,   // visible
            box_sizing: BoxSizing::INITIAL,  // content-box
            visibility: Visibility::INITIAL, // visible
            direction: Direction::INITIAL,   // ltr

            width: Length::auto(),
            height: Length::auto(),
            min_width: Length::auto(),
            min_height: Length::auto(),
            max_width: Length::none(),   // NOT auto — Blink uses kNone
            max_height: Length::none(),  // NOT auto

            margin_top: Length::zero(),
            margin_right: Length::zero(),
            margin_bottom: Length::zero(),
            margin_left: Length::zero(),

            padding_top: Length::zero(),
            padding_right: Length::zero(),
            padding_bottom: Length::zero(),
            padding_left: Length::zero(),

            // Blink initial border width is 3 (medium), but since border-style
            // defaults to none, the used width is 0. We store 3 to match Blink's
            // computed value; the layout/paint code checks border-style.
            border_top_width: 3,
            border_right_width: 3,
            border_bottom_width: 3,
            border_left_width: 3,

            border_top_style: BorderStyle::INITIAL,    // none
            border_right_style: BorderStyle::INITIAL,
            border_bottom_style: BorderStyle::INITIAL,
            border_left_style: BorderStyle::INITIAL,

            border_top_color: StyleColor::default(),   // currentColor
            border_right_color: StyleColor::default(),
            border_bottom_color: StyleColor::default(),
            border_left_color: StyleColor::default(),

            background_color: Color::TRANSPARENT,
            color: Color::BLACK,
            opacity: 1.0,
            z_index: None, // auto

            // Flexbox — container properties
            flex_direction: FlexDirection::INITIAL,   // row
            flex_wrap: FlexWrap::INITIAL,             // nowrap
            justify_content: ContentAlignment::INITIAL, // normal
            align_items: ItemAlignment::INITIAL_ITEMS,  // normal (→ stretch in flex)
            align_content: ContentAlignment::INITIAL,   // normal
            row_gap: None,     // normal = 0px for flex
            column_gap: None,  // normal = 0px for flex

            // Flexbox — item properties
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::auto(),
            align_self: ItemAlignment::INITIAL_SELF,  // auto (→ inherits align-items)
            order: 0,
        }
    }

    // ── Convenience: effective border width (0 if style is none/hidden) ──

    /// Effective border-top-width: 0 if border-style is none/hidden.
    /// This matches Blink's "used value" computation.
    #[inline]
    pub fn effective_border_top(&self) -> i32 {
        if self.border_top_style.has_visible_border() { self.border_top_width } else { 0 }
    }

    #[inline]
    pub fn effective_border_right(&self) -> i32 {
        if self.border_right_style.has_visible_border() { self.border_right_width } else { 0 }
    }

    #[inline]
    pub fn effective_border_bottom(&self) -> i32 {
        if self.border_bottom_style.has_visible_border() { self.border_bottom_width } else { 0 }
    }

    #[inline]
    pub fn effective_border_left(&self) -> i32 {
        if self.border_left_style.has_visible_border() { self.border_left_width } else { 0 }
    }

    /// True if this element establishes a new formatting context.
    /// Mirrors Blink's `CreatesNewFormattingContext()`.
    pub fn creates_new_formatting_context(&self) -> bool {
        // Flex/grid containers, inline-block, flow-root, overflow != visible,
        // absolutely positioned, floated — all create new BFC.
        self.display.is_new_formatting_context()
            || self.position.is_absolutely_positioned()
            || self.float != Float::None
            || (self.overflow_x != Overflow::Visible || self.overflow_y != Overflow::Visible)
    }

    /// True if this element is in the normal flow (not floated, not abs-pos).
    #[inline]
    pub fn is_in_flow(&self) -> bool {
        self.position.is_in_flow() && self.float == Float::None
    }

    /// True if this element participates in its parent's layout.
    #[inline]
    pub fn is_out_of_flow(&self) -> bool {
        self.position.is_absolutely_positioned() || self.float != Float::None
    }
}

impl Default for ComputedStyle {
    fn default() -> Self { Self::initial() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_values_match_blink() {
        let s = ComputedStyle::initial();

        // Display
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.position, Position::Static);
        assert_eq!(s.float, Float::None);
        assert_eq!(s.clear, Clear::None);

        // Sizing
        assert!(s.width.is_auto());
        assert!(s.height.is_auto());
        assert!(s.min_width.is_auto());
        assert!(s.max_width.is_none()); // NOT auto
        assert!(s.max_height.is_none());

        // Box model
        assert_eq!(s.box_sizing, BoxSizing::ContentBox);
        assert_eq!(s.margin_top, Length::zero());
        assert_eq!(s.padding_top, Length::zero());

        // Borders — computed width is 3 (medium), but used is 0 since style=none
        assert_eq!(s.border_top_width, 3);
        assert_eq!(s.border_top_style, BorderStyle::None);
        assert_eq!(s.effective_border_top(), 0);

        // Colors
        assert!(s.background_color.is_transparent());
        assert_eq!(s.color, Color::BLACK);
        assert_eq!(s.opacity, 1.0);

        // Z-index
        assert_eq!(s.z_index, None); // auto

        // Overflow
        assert_eq!(s.overflow_x, Overflow::Visible);

        // Flexbox — container properties
        assert_eq!(s.flex_direction, FlexDirection::Row);
        assert_eq!(s.flex_wrap, FlexWrap::Nowrap);
        assert_eq!(s.justify_content, ContentAlignment::INITIAL);
        assert_eq!(s.align_items, ItemAlignment::INITIAL_ITEMS);
        assert_eq!(s.align_content, ContentAlignment::INITIAL);
        assert!(s.row_gap.is_none());
        assert!(s.column_gap.is_none());

        // Flexbox — item properties
        assert_eq!(s.flex_grow, 0.0);
        assert_eq!(s.flex_shrink, 1.0);
        assert!(s.flex_basis.is_auto());
        assert_eq!(s.align_self, ItemAlignment::INITIAL_SELF);
        assert_eq!(s.order, 0);
    }

    #[test]
    fn formatting_context_detection() {
        let mut s = ComputedStyle::initial();

        // Default inline does NOT create new FC
        assert!(!s.creates_new_formatting_context());

        // Flex creates new FC
        s.display = Display::Flex;
        assert!(s.creates_new_formatting_context());

        // Absolutely positioned creates new FC
        let mut s2 = ComputedStyle::initial();
        s2.position = Position::Absolute;
        assert!(s2.creates_new_formatting_context());

        // Overflow hidden creates new FC
        let mut s3 = ComputedStyle::initial();
        s3.overflow_x = Overflow::Hidden;
        assert!(s3.creates_new_formatting_context());
    }

    #[test]
    fn in_flow_detection() {
        let mut s = ComputedStyle::initial();
        assert!(s.is_in_flow());

        s.position = Position::Absolute;
        assert!(!s.is_in_flow());
        assert!(s.is_out_of_flow());
    }
}
