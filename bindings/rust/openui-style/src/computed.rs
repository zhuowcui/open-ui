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
use crate::font_types::*;

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

    // ── Inset properties (position offsets) ──────────────────────────
    // Blink: stored in `surround_data_` as Length values.
    // Initial value is `auto` for all four (CSS 2.1 §9.3.2).

    /// CSS `top`. Initial: `auto`. Used with positioned elements.
    pub top: Length,

    /// CSS `right`. Initial: `auto`.
    pub right: Length,

    /// CSS `bottom`. Initial: `auto`.
    pub bottom: Length,

    /// CSS `left`. Initial: `auto`.
    pub left: Length,

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

    // ── Text & Font properties ───────────────────────────────────────

    /// CSS `text-align`. Initial: `start`. Inherited.
    pub text_align: TextAlign,

    /// CSS `white-space`. Initial: `normal`. Inherited.
    pub white_space: WhiteSpace,

    // ── Font Properties ──────────────────────────────────────────────

    /// CSS `font-family`. Initial: platform-dependent (we use sans-serif).
    pub font_family: FontFamilyList,

    /// CSS `font-size`. Initial: `medium` (16px). Inherited.
    pub font_size: f32,

    /// CSS `font-weight`. Initial: `normal` (400). Inherited.
    pub font_weight: FontWeight,

    /// CSS `font-style`. Initial: `normal`. Inherited.
    pub font_style: FontStyleEnum,

    /// CSS `font-stretch`. Initial: `normal` (100%). Inherited.
    pub font_stretch: FontStretch,

    /// CSS `font-variant-caps`. Initial: `normal`. Inherited.
    pub font_variant_caps: FontVariantCaps,

    /// CSS `font-variant-ligatures`. Initial: `normal`. Inherited.
    pub font_variant_ligatures: FontVariantLigatures,

    /// CSS `font-variant-numeric`. Initial: `normal`. Inherited.
    pub font_variant_numeric: FontVariantNumeric,

    /// CSS `font-variant-east-asian`. Initial: `normal`. Inherited.
    pub font_variant_east_asian: FontVariantEastAsian,

    /// CSS `font-variant-position`. Initial: `normal`. Inherited.
    pub font_variant_position: FontVariantPosition,

    /// CSS `font-variant-alternates`. Initial: `normal`. Inherited.
    pub font_variant_alternates: FontVariantAlternates,

    /// CSS `font-size-adjust`. Initial: `none`.
    pub font_size_adjust: Option<f32>,

    /// CSS `font-optical-sizing`. Initial: `auto`. Inherited.
    pub font_optical_sizing: FontOpticalSizing,

    /// CSS `font-synthesis-weight`. Initial: `auto`. Inherited.
    pub font_synthesis_weight: FontSynthesis,

    /// CSS `font-synthesis-style`. Initial: `auto`. Inherited.
    pub font_synthesis_style: FontSynthesis,

    /// CSS `font-feature-settings`. Initial: `normal` (empty). Inherited.
    pub font_feature_settings: Vec<FontFeature>,

    /// CSS `font-variation-settings`. Initial: `normal` (empty). Inherited.
    pub font_variation_settings: Vec<FontVariation>,

    // ── Line Height ──────────────────────────────────────────────────

    /// CSS `line-height`. Initial: `normal`. Inherited.
    pub line_height: LineHeight,

    // ── Text Spacing ─────────────────────────────────────────────────

    /// CSS `letter-spacing`. Initial: `normal` (0). Inherited.
    pub letter_spacing: f32,

    /// CSS `word-spacing`. Initial: `normal` (0). Inherited.
    pub word_spacing: f32,

    /// CSS `text-indent`. Initial: `0`. Inherited.
    pub text_indent: Length,

    // ── Text Layout ──────────────────────────────────────────────────

    /// CSS `text-align-last`. Initial: `auto`. Inherited.
    pub text_align_last: TextAlignLast,

    /// CSS `text-justify`. Initial: `auto`. Inherited.
    pub text_justify: TextJustify,

    /// CSS `word-break`. Initial: `normal`. Inherited.
    pub word_break: WordBreak,

    /// CSS `overflow-wrap`. Initial: `normal`. Inherited.
    pub overflow_wrap: OverflowWrap,

    /// CSS `line-break`. Initial: `auto`. Inherited.
    /// Controls line breaking rules for CJK text.
    /// CSS Text Module Level 3 §5.2.
    pub line_break: LineBreak,

    /// CSS `hyphens`. Initial: `manual`. Inherited.
    pub hyphens: Hyphens,

    /// CSS `hyphenate-limit-chars`. Initial: `(5, 2, 2)`. Inherited.
    ///
    /// Controls minimum character counts for hyphenation:
    /// `(min_word, min_prefix, min_suffix)`
    ///
    /// Blink defaults from `third_party/blink/renderer/core/style/computed_style.h`:
    /// - min_word = 5 (minimum word length to hyphenate)
    /// - min_prefix = 2 (minimum characters before hyphen)
    /// - min_suffix = 2 (minimum characters after hyphen)
    pub hyphenate_limit_chars: (u8, u8, u8),

    // ── Text Decoration ──────────────────────────────────────────────

    /// CSS `text-decoration-line`. Initial: `none`.
    pub text_decoration_line: TextDecorationLine,

    /// CSS `text-decoration-style`. Initial: `solid`.
    pub text_decoration_style: TextDecorationStyle,

    /// CSS `text-decoration-color`. Initial: `currentColor`.
    pub text_decoration_color: StyleColor,

    /// CSS `text-decoration-thickness`. Initial: `auto`.
    pub text_decoration_thickness: TextDecorationThickness,

    /// CSS `text-underline-offset`. Initial: `auto`.
    pub text_underline_offset: Length,

    /// CSS `text-underline-position`. Initial: `auto`. Inherited.
    pub text_underline_position: TextUnderlinePosition,

    /// CSS `text-decoration-skip-ink`. Initial: `auto`. Inherited.
    pub text_decoration_skip_ink: TextDecorationSkipInk,

    // ── Text Transform ───────────────────────────────────────────────

    /// CSS `text-transform`. Initial: `none`. Inherited.
    pub text_transform: TextTransform,

    /// CSS `text-overflow`. Initial: `clip`.
    pub text_overflow: TextOverflow,

    // ── Vertical Alignment ───────────────────────────────────────────

    /// CSS `vertical-align`. Initial: `baseline`.
    pub vertical_align: VerticalAlign,

    // ── Writing & Bidi ───────────────────────────────────────────────

    /// CSS `unicode-bidi`. Initial: `normal`.
    pub unicode_bidi: UnicodeBidi,

    /// CSS `writing-mode`. Initial: `horizontal-tb`. Inherited.
    pub writing_mode: WritingMode,

    /// CSS `text-orientation`. Initial: `mixed`. Inherited.
    pub text_orientation: TextOrientation,

    // ── Text Rendering ───────────────────────────────────────────────

    /// CSS `text-rendering`. Initial: `auto`. Inherited.
    pub text_rendering: TextRendering,

    /// CSS `-webkit-font-smoothing`. Initial: `auto`. Inherited.
    pub font_smoothing: FontSmoothing,

    // ── Text Shadow ──────────────────────────────────────────────────

    /// CSS `text-shadow`. Initial: `none` (empty). Inherited.
    pub text_shadow: Vec<TextShadow>,

    // ── Hanging Punctuation ─────────────────────────────────────────

    /// CSS `hanging-punctuation`. Initial: `none`. Inherited.
    /// NOTE: Stored for spec compliance; not applied during layout
    /// (matching Chromium, which does not implement this property).
    pub hanging_punctuation: HangingPunctuation,

    // ── Text Emphasis ────────────────────────────────────────────────

    /// CSS `text-emphasis-style` mark shape. Initial: `none`. Inherited.
    pub text_emphasis_mark: TextEmphasisMark,

    /// CSS `text-emphasis-style` fill mode. Initial: `filled`. Inherited.
    pub text_emphasis_fill: TextEmphasisFill,

    /// CSS `text-emphasis-position`. Initial: `over right`. Inherited.
    pub text_emphasis_position: TextEmphasisPosition,

    /// CSS `text-emphasis-color`. Initial: `currentColor`. Inherited.
    pub text_emphasis_color: StyleColor,

    // ── Text Combine ─────────────────────────────────────────────────

    /// CSS `text-combine-upright`. Initial: `none`.
    pub text_combine_upright: TextCombineUpright,

    // ── Ruby Annotation ─────────────────────────────────────────────

    /// CSS `ruby-position`. Initial: `over`. Inherited.
    /// Determines where annotation text is placed relative to base text.
    pub ruby_position: RubyPosition,

    /// CSS `ruby-align`. Initial: `space-around`. Inherited.
    /// Controls how annotation content is distributed within its box.
    pub ruby_align: RubyAlign,

    // ── Tab Size ─────────────────────────────────────────────────────

    /// CSS `tab-size`. Initial: `8`. Inherited.
    pub tab_size: TabSize,

    // ── Font Palette ─────────────────────────────────────────────────

    /// CSS `font-palette`. Initial: `normal`.
    /// Controls which color palette is used for COLR/CPAL color fonts.
    pub font_palette: FontPalette,

    // ── Locale ───────────────────────────────────────────────────────

    /// BCP 47 locale derived from the `lang` HTML attribute.
    /// Used for locale-dependent shaping (e.g., CJK font selection).
    pub locale: Option<String>,
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

            top: Length::auto(),
            right: Length::auto(),
            bottom: Length::auto(),
            left: Length::auto(),

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

            // Text & Font — inherited text properties
            text_align: TextAlign::INITIAL,         // start
            white_space: WhiteSpace::INITIAL,       // normal

            // Font properties
            font_family: FontFamilyList::default(),      // sans-serif
            font_size: 16.0,                              // CSS medium
            font_weight: FontWeight::NORMAL,              // 400
            font_style: FontStyleEnum::Normal,
            font_stretch: FontStretch::NORMAL,            // 100%
            font_variant_caps: FontVariantCaps::Normal,
            font_variant_ligatures: FontVariantLigatures::NORMAL,
            font_variant_numeric: FontVariantNumeric::NORMAL,
            font_variant_east_asian: FontVariantEastAsian::NORMAL,
            font_variant_position: FontVariantPosition::Normal,
            font_variant_alternates: FontVariantAlternates::Normal,
            font_size_adjust: None,
            font_optical_sizing: FontOpticalSizing::Auto,
            font_synthesis_weight: FontSynthesis::Auto,
            font_synthesis_style: FontSynthesis::Auto,
            font_feature_settings: Vec::new(),
            font_variation_settings: Vec::new(),

            // Line height
            line_height: LineHeight::Normal,

            // Text spacing
            letter_spacing: 0.0,
            word_spacing: 0.0,
            text_indent: Length::zero(),

            // Text layout
            text_align_last: TextAlignLast::INITIAL,     // auto
            text_justify: TextJustify::INITIAL,           // auto
            word_break: WordBreak::INITIAL,               // normal
            overflow_wrap: OverflowWrap::INITIAL,         // normal
            line_break: LineBreak::INITIAL,               // auto
            hyphens: Hyphens::INITIAL,                    // manual
            hyphenate_limit_chars: (5, 2, 2),              // Blink defaults

            // Text decoration
            text_decoration_line: TextDecorationLine::NONE,
            text_decoration_style: TextDecorationStyle::INITIAL,  // solid
            text_decoration_color: StyleColor::CurrentColor,
            text_decoration_thickness: TextDecorationThickness::Auto,
            text_underline_offset: Length::auto(),
            text_underline_position: TextUnderlinePosition::INITIAL,  // auto
            text_decoration_skip_ink: TextDecorationSkipInk::INITIAL, // auto

            // Text transform
            text_transform: TextTransform::INITIAL,       // none
            text_overflow: TextOverflow::INITIAL,          // clip

            // Vertical alignment
            vertical_align: VerticalAlign::Baseline,

            // Writing & bidi
            unicode_bidi: UnicodeBidi::INITIAL,            // normal
            writing_mode: WritingMode::INITIAL,            // horizontal-tb
            text_orientation: TextOrientation::INITIAL,    // mixed

            // Text rendering
            text_rendering: TextRendering::Auto,
            font_smoothing: FontSmoothing::Auto,

            // Text shadow
            text_shadow: Vec::new(),

            // Hanging punctuation
            hanging_punctuation: HangingPunctuation::NONE,

            // Text emphasis
            text_emphasis_mark: TextEmphasisMark::INITIAL,       // none
            text_emphasis_fill: TextEmphasisFill::INITIAL,       // filled
            text_emphasis_position: TextEmphasisPosition::INITIAL, // over right
            text_emphasis_color: StyleColor::CurrentColor,
            text_combine_upright: TextCombineUpright::INITIAL,   // none

            // Ruby annotation
            ruby_position: RubyPosition::INITIAL,                   // over
            ruby_align: RubyAlign::INITIAL,                         // space-around

            // Tab size
            tab_size: TabSize::Spaces(8),

            // Font palette
            font_palette: FontPalette::INITIAL,  // normal

            // Locale
            locale: None,
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

    #[test]
    fn hanging_punctuation_initial_value() {
        let s = ComputedStyle::initial();
        assert_eq!(s.hanging_punctuation, HangingPunctuation::NONE);
        assert!(s.hanging_punctuation.is_none());
    }

    #[test]
    fn hanging_punctuation_none_is_default() {
        let hp = HangingPunctuation::default();
        assert!(hp.is_none());
        assert!(!hp.first);
        assert!(!hp.last);
        assert!(!hp.force_end);
        assert!(!hp.allow_end);
    }

    #[test]
    fn hanging_punctuation_first() {
        let hp = HangingPunctuation {
            first: true,
            ..HangingPunctuation::NONE
        };
        assert!(!hp.is_none());
        assert!(hp.first);
    }

    #[test]
    fn hanging_punctuation_last() {
        let hp = HangingPunctuation {
            last: true,
            ..HangingPunctuation::NONE
        };
        assert!(!hp.is_none());
        assert!(hp.last);
    }

    #[test]
    fn hanging_punctuation_force_end() {
        let hp = HangingPunctuation {
            force_end: true,
            ..HangingPunctuation::NONE
        };
        assert!(!hp.is_none());
        assert!(hp.force_end);
        assert!(!hp.allow_end);
    }

    #[test]
    fn hanging_punctuation_allow_end() {
        let hp = HangingPunctuation {
            allow_end: true,
            ..HangingPunctuation::NONE
        };
        assert!(!hp.is_none());
        assert!(hp.allow_end);
        assert!(!hp.force_end);
    }

    #[test]
    fn hanging_punctuation_combined() {
        let hp = HangingPunctuation {
            first: true,
            last: true,
            force_end: true,
            allow_end: false,
        };
        assert!(!hp.is_none());
        assert!(hp.first);
        assert!(hp.last);
        assert!(hp.force_end);
    }

    #[test]
    fn hanging_punctuation_equality() {
        let a = HangingPunctuation { first: true, last: false, force_end: false, allow_end: false };
        let b = HangingPunctuation { first: true, last: false, force_end: false, allow_end: false };
        let c = HangingPunctuation { first: false, last: true, force_end: false, allow_end: false };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn hanging_punctuation_stored_on_style() {
        let mut s = ComputedStyle::initial();
        s.hanging_punctuation = HangingPunctuation {
            first: true,
            last: true,
            force_end: false,
            allow_end: true,
        };
        assert!(s.hanging_punctuation.first);
        assert!(s.hanging_punctuation.last);
        assert!(!s.hanging_punctuation.force_end);
        assert!(s.hanging_punctuation.allow_end);
        assert!(!s.hanging_punctuation.is_none());
    }

    // ── Text Emphasis ──────────────────────────────────────────────

    #[test]
    fn text_emphasis_initial_values() {
        let s = ComputedStyle::initial();
        assert_eq!(s.text_emphasis_mark, TextEmphasisMark::None);
        assert_eq!(s.text_emphasis_fill, TextEmphasisFill::Filled);
        assert_eq!(s.text_emphasis_position, TextEmphasisPosition::INITIAL);
        assert!(s.text_emphasis_position.over);
        assert!(s.text_emphasis_position.right);
        assert_eq!(s.text_emphasis_color, StyleColor::CurrentColor);
    }

    #[test]
    fn text_emphasis_mark_set_dot() {
        let mut s = ComputedStyle::initial();
        s.text_emphasis_mark = TextEmphasisMark::Dot;
        assert_eq!(s.text_emphasis_mark, TextEmphasisMark::Dot);
    }

    #[test]
    fn text_emphasis_fill_open() {
        let mut s = ComputedStyle::initial();
        s.text_emphasis_fill = TextEmphasisFill::Open;
        assert_eq!(s.text_emphasis_fill, TextEmphasisFill::Open);
    }

    #[test]
    fn text_emphasis_position_under_left() {
        let mut s = ComputedStyle::initial();
        s.text_emphasis_position = TextEmphasisPosition { over: false, right: false };
        assert!(!s.text_emphasis_position.over);
        assert!(!s.text_emphasis_position.right);
    }

    #[test]
    fn text_emphasis_color_custom() {
        let mut s = ComputedStyle::initial();
        let red = Color::from_rgba8(255, 0, 0, 255);
        s.text_emphasis_color = StyleColor::Resolved(red);
        match s.text_emphasis_color {
            StyleColor::Resolved(c) => assert_eq!(c, Color::from_rgba8(255, 0, 0, 255)),
            _ => panic!("expected Resolved color"),
        }
    }

    #[test]
    fn text_emphasis_custom_char() {
        let mut s = ComputedStyle::initial();
        s.text_emphasis_mark = TextEmphasisMark::Custom('★');
        assert_eq!(s.text_emphasis_mark, TextEmphasisMark::Custom('★'));
    }

    // ── Text Combine Upright ───────────────────────────────────────

    #[test]
    fn text_combine_upright_initial() {
        let s = ComputedStyle::initial();
        assert_eq!(s.text_combine_upright, TextCombineUpright::None);
    }

    #[test]
    fn text_combine_upright_all() {
        let mut s = ComputedStyle::initial();
        s.text_combine_upright = TextCombineUpright::All;
        assert_eq!(s.text_combine_upright, TextCombineUpright::All);
    }

    #[test]
    fn text_combine_upright_default_trait() {
        assert_eq!(TextCombineUpright::default(), TextCombineUpright::None);
    }
}
