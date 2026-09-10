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
use crate::layout_systems::*;

/// CSS `aspect-ratio` property — stores the ratio and optional auto flag.
///
/// `aspect-ratio: auto` → `None` at the ComputedStyle level.
/// `aspect-ratio: 16/9` → `Some(AspectRatio { ratio: (16.0, 9.0), auto_flag: false })`.
/// `aspect-ratio: auto 16/9` → `Some(AspectRatio { ratio: (16.0, 9.0), auto_flag: true })`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AspectRatio {
    /// Width / height components. E.g., `(16.0, 9.0)` for 16:9.
    pub ratio: (f32, f32),
    /// If true, the intrinsic aspect ratio is preferred over the specified one
    /// (CSS Sizing L3: `aspect-ratio: auto 16/9`).
    pub auto_flag: bool,
}

/// A single box-shadow layer.
///
/// CSS syntax: `[inset?] <offset-x> <offset-y> [<blur-radius>] [<spread-radius>] [<color>]`
#[derive(Debug, Clone)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
    pub inset: bool,
}

/// One item in the computed value of CSS `content`.
///
/// Replaced generated content (`url()`, gradients, and other CSS images) is
/// deliberately not represented here; it remains owned by the image path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratedContentItem {
    String(String),
    Attribute(String),
    Counter {
        name: String,
        style: CounterStyle,
    },
    Counters {
        name: String,
        separator: String,
        style: CounterStyle,
    },
    OpenQuote,
    CloseQuote,
    NoOpenQuote,
    NoCloseQuote,
}

/// CSS2 counter styles used by generated-content tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterStyle {
    Decimal,
    DecimalLeadingZero,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

/// One named operation in `counter-reset`, `counter-set`, or
/// `counter-increment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterOperation {
    pub name: String,
    pub value: i32,
}

/// One nesting-level pair in the inherited CSS `quotes` property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotePair {
    pub open: String,
    pub close: String,
}

/// Computed modern/compatibility line clamp limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineClamp {
    None,
    Auto,
    Lines(u32),
}

/// The marker appended at a line-clamp point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockEllipsis {
    Auto,
    NoEllipsis,
    String(String),
}

/// Legacy WebKit box orientation. It is consumed only to activate the
/// `-webkit-line-clamp` compatibility layout, not as old-flexbox support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebkitBoxOrient {
    Horizontal,
    Vertical,
}

/// A resolved color stop in a CSS linear gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientStopPosition {
    Auto,
    Percent(f32),
    Px(f32),
    Calc { percent: f32, px: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearGradientStop {
    pub color: Color,
    pub position: GradientStopPosition,
}

/// The single linear background-image layer currently consumed by paint.
/// Angles use CSS conventions: 0deg points up and 90deg points right.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub angle_degrees: f32,
    /// Whether this image was authored with `repeating-linear-gradient()`.
    pub repeating: bool,
    pub stops: Vec<LinearGradientStop>,
}

/// Stable handle to an encoded image resource owned by the document.
///
/// Keeping this handle in computed style avoids leaking Skia types into style
/// or layout and makes generated documents deterministic and self-contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageResourceId(pub u32);

impl ImageResourceId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A resolved stop used by the production background-image model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub color: StyleColor,
    pub position: GradientStopPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientColorSpace {
    Srgb,
    Hsl,
    Oklch,
}

#[derive(Debug, Clone)]
pub struct CssLinearGradient {
    pub angle_degrees: f32,
    /// Normalized keyword direction for `to <corner>`. Corner keywords are
    /// aspect-ratio dependent and therefore cannot be lowered to an angle at
    /// computed-value time.
    pub corner_direction: Option<(f32, f32)>,
    pub repeating: bool,
    pub color_space: GradientColorSpace,
    pub stops: Vec<GradientStop>,
}

/// One component of a CSS background position.
#[derive(Debug, Clone, Copy)]
pub enum BackgroundPosition {
    /// Percentage of the remaining space (`0%`, `50%`, `100%`).
    Percent(f32),
    /// Fixed/calculated offset from the start edge.
    Length(Length),
    /// Offset from a named edge. Percentages are relative to the positioning
    /// area, as required by the four-value background-position syntax.
    Edge { end: bool, offset: Length },
}

impl BackgroundPosition {
    pub const fn start() -> Self {
        Self::Percent(0.0)
    }

    pub const fn center() -> Self {
        Self::Percent(50.0)
    }
}

/// Per-axis background tiling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundRepeat {
    Repeat,
    NoRepeat,
    Round,
    Space,
}

/// CSS background-size for one image layer.
#[derive(Debug, Clone)]
pub enum BackgroundSize {
    Auto,
    Explicit(Length, Length),
    Cover,
    Contain,
}

/// Radial-gradient shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadialGradientShape {
    Circle,
    Ellipse,
}

/// Radial-gradient extent or explicit radii.
#[derive(Debug, Clone)]
pub enum RadialGradientSize {
    ClosestSide,
    ClosestCorner,
    FarthestSide,
    FarthestCorner,
    Explicit(Length, Length),
}

#[derive(Debug, Clone)]
pub struct RadialGradient {
    pub repeating: bool,
    pub color_space: GradientColorSpace,
    pub shape: RadialGradientShape,
    pub size: RadialGradientSize,
    pub center_x: BackgroundPosition,
    pub center_y: BackgroundPosition,
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone)]
pub struct ConicGradient {
    /// CSS angle: 0deg points up and positive angles turn clockwise.
    pub from_degrees: f32,
    pub center_x: BackgroundPosition,
    pub center_y: BackgroundPosition,
    pub repeating: bool,
    pub color_space: GradientColorSpace,
    pub stops: Vec<GradientStop>,
}

/// Paintable CSS image independent of the raster backend.
#[derive(Debug, Clone)]
pub enum CssImage {
    Raster(ImageResourceId),
    LinearGradient(CssLinearGradient),
    RadialGradient(RadialGradient),
    ConicGradient(ConicGradient),
}

/// One comma-separated CSS background layer. Layers remain in author order;
/// paint traverses them back-to-front.
#[derive(Debug, Clone)]
pub struct BackgroundLayer {
    pub image: CssImage,
    pub repeat_x: BackgroundRepeat,
    pub repeat_y: BackgroundRepeat,
    pub position_x: BackgroundPosition,
    pub position_y: BackgroundPosition,
    pub size: BackgroundSize,
    pub origin: BackgroundClip,
    pub clip: BackgroundClip,
    pub attachment: BackgroundAttachment,
}

impl BackgroundLayer {
    pub fn new(image: CssImage) -> Self {
        Self {
            image,
            repeat_x: BackgroundRepeat::Repeat,
            repeat_y: BackgroundRepeat::Repeat,
            position_x: BackgroundPosition::start(),
            position_y: BackgroundPosition::start(),
            size: BackgroundSize::Auto,
            origin: BackgroundClip::PaddingBox,
            clip: BackgroundClip::BorderBox,
            attachment: BackgroundAttachment::Scroll,
        }
    }
}

/// Number, length/percentage, or `auto` component used by border-image.
#[derive(Debug, Clone)]
pub enum BorderImageLength {
    Number(f32),
    Length(Length),
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderImageRepeat {
    Stretch,
    Repeat,
    Round,
    Space,
}

/// Computed nine-slice border-image metadata.
#[derive(Debug, Clone)]
pub struct BorderImage {
    pub source: CssImage,
    pub slice: [BorderImageLength; 4],
    pub fill: bool,
    pub width: [BorderImageLength; 4],
    pub outset: [BorderImageLength; 4],
    pub repeat_x: BorderImageRepeat,
    pub repeat_y: BorderImageRepeat,
}

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

    /// Whether a list-item's inner display type is `flow-root`.
    ///
    /// `display: flow-root list-item` still generates a list-item principal
    /// box and marker, but unlike the ordinary `list-item` value its contents
    /// establish an independent block formatting context.
    pub list_item_is_flow_root: bool,

    /// CSS `list-style-position`. Initial: `outside`.
    pub list_style_position: ListStylePosition,

    /// CSS `list-style-type`. Initial: `disc` for the list-item subset.
    pub list_style_type: ListStyleType,

    /// CSS `position`. Initial: `static`.
    pub position: Position,

    /// Whether transform-like properties establish a containing block for
    /// positioned descendants. Set for `will-change: transform` in ported WPTs.
    pub establishes_transform_containing_block: bool,

    /// Whether `will-change: transform` requested a pre-promoted compositing
    /// layer, independently of an authored non-identity transform.
    pub will_change_transform: bool,

    /// CSS `float`. Initial: `none`.
    pub float: Float,

    /// CSS `clear`. Initial: `none`.
    pub clear: Clear,

    /// CSS `overflow-x`. Initial: `visible`.
    pub overflow_x: Overflow,

    /// CSS `overflow-y`. Initial: `visible`.
    pub overflow_y: Overflow,

    /// CSS `resize`. Initial: `none`.
    pub resize: Resize,

    /// CSS `scrollbar-width`. Initial: `auto`.
    pub scrollbar_width: ScrollbarWidth,

    /// CSS `scrollbar-gutter`. Initial: `auto`.
    pub scrollbar_gutter: ScrollbarGutter,

    /// CSS `overflow-clip-margin`. Initial: `0.0` (px).
    /// Specifies how far content may overflow before being clipped when
    /// `overflow: clip` is used. Only applies to `overflow: clip`.
    pub overflow_clip_margin: f32,

    /// CSS `overflow-clip-margin` visual-box reference.
    /// Determines which box edge the clip margin expands from.
    /// Initial: `padding-box` (CSS Overflow 3 §3).
    pub overflow_clip_box: OverflowClipBox,

    /// CSS `scrollbar-color` thumb color. Initial: `auto`.
    pub scrollbar_thumb_color: Option<Color>,

    /// CSS `scrollbar-color` track color. Initial: `auto`.
    pub scrollbar_track_color: Option<Color>,

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

    // ── Outline (Blink: OutlineValue in surround_data_) ─────────────
    // Outline is drawn outside the border box. Does NOT affect layout.
    /// CSS `outline-width`. Initial: `3` (medium = 3px, same as border).
    pub outline_width: i32,

    /// CSS `outline-style`. Initial: `none`.
    pub outline_style: BorderStyle,

    /// CSS `outline-color`. Initial: `currentColor`.
    pub outline_color: StyleColor,

    /// CSS `outline-offset`. Initial: `0`.
    pub outline_offset: i32,

    // ── Border radii (Blink: LengthSize stored in SurroundData) ─────
    // Each corner stores horizontal and vertical radii as `f32` pixels.
    // Initial value: `0.0` (no rounding).
    /// CSS `border-top-left-radius`. Initial: `0.0`.
    pub border_top_left_radius: (f32, f32),
    /// CSS `border-top-right-radius`. Initial: `0.0`.
    pub border_top_right_radius: (f32, f32),
    /// CSS `border-bottom-right-radius`. Initial: `0.0`.
    pub border_bottom_right_radius: (f32, f32),
    /// CSS `border-bottom-left-radius`. Initial: `0.0`.
    pub border_bottom_left_radius: (f32, f32),

    // ── Colors ───────────────────────────────────────────────────────
    /// CSS `background-color`. Initial: `transparent`.
    pub background_color: Color,

    /// First CSS linear-gradient background-image layer. Initial: `none`.
    pub background_linear_gradient: Option<LinearGradient>,

    /// Complete comma-separated background-image list. Empty keeps the
    /// historical `background_linear_gradient` compatibility path active.
    pub background_layers: Vec<BackgroundLayer>,

    /// CSS mask image layers. The image alpha is composited with the entire
    /// element subtree after ordinary background/content painting.
    pub mask_layers: Vec<BackgroundLayer>,

    /// CSS `background-clip`. Initial: `border-box`.
    pub background_clip: BackgroundClip,

    /// CSS `background-attachment`. Initial: `scroll`.
    pub background_attachment: BackgroundAttachment,

    /// CSS border-image metadata. Initial: none.
    pub border_image: Option<BorderImage>,

    /// CSS `color` (inherited). Initial: `black` (CanvasText in Blink,
    /// but we use black for simplicity — matches most user agents).
    pub color: Color,

    /// CSS `opacity`. Initial: `1.0`.
    pub opacity: f32,

    // ── Box shadows ─────────────────────────────────────────────────
    /// CSS `box-shadow`. Initial: `none` (empty vec).
    pub box_shadow: Vec<BoxShadow>,

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

    // ── Tables ───────────────────────────────────────────────────────
    /// CSS `table-layout`. Initial: `auto`.
    pub table_layout: TableLayout,
    /// CSS `border-collapse`. Initial: `separate`.
    pub border_collapse: BorderCollapse,
    /// CSS `border-spacing`, in inline/block table axes.
    pub border_spacing: (Length, Length),
    /// CSS `caption-side`. Initial: `top`.
    pub caption_side: CaptionSide,
    /// CSS `empty-cells`. Initial: `show`.
    pub empty_cells: EmptyCells,

    // ── Grid ─────────────────────────────────────────────────────────
    pub grid_template_columns: GridTrackList,
    pub grid_template_rows: GridTrackList,
    pub grid_auto_columns: Vec<GridTrackSize>,
    pub grid_auto_rows: Vec<GridTrackSize>,
    pub grid_auto_flow: GridAutoFlow,
    pub grid_column: GridPlacement,
    pub grid_row: GridPlacement,
    pub grid_template_areas: GridTemplateAreas,
    /// Grid's `justify-items`; flex does not consume this property.
    pub justify_items: ItemAlignment,
    /// Grid's `justify-self`; flex does not consume this property.
    pub justify_self: ItemAlignment,
    /// CSS `margin-trim` edge set.
    pub margin_trim: MarginTrim,

    // ── Containment and static container queries ─────────────────────
    pub contain: Containment,
    pub content_visibility: ContentVisibility,
    pub contain_intrinsic_width: ContainIntrinsicLength,
    pub contain_intrinsic_height: ContainIntrinsicLength,
    pub container_type: ContainerType,
    pub container_names: Vec<String>,
    pub scroll_marker_group: ScrollMarkerGroup,
    pub scroll_target_group: ScrollTargetGroup,
    pub scroll_snap_align: ScrollSnapAlign,
    pub scroll_snap_axis: ScrollSnapAxis,

    // ── Replaced content and deterministic effects ───────────────────
    pub object_fit: ObjectFit,
    pub object_position: ObjectPosition,
    pub transform: Transform2D,
    pub transform_origin: (Length, Length),
    /// CSS basic-shape `clip-path: inset(top right bottom left)`.
    pub clip_path_inset: Option<[Length; 4]>,
    pub shape_outside: ShapeOutside,
    pub shape_margin: Length,
    pub shape_image_threshold: f32,
    pub animation_snapshot: Option<AnimationSnapshot>,

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

    // ── Generated content and counters ──────────────────────────────
    /// Computed CSS `content`; `None` represents `normal`/`none`.
    pub content: Option<Vec<GeneratedContentItem>>,
    pub counter_reset: Vec<CounterOperation>,
    pub counter_set: Vec<CounterOperation>,
    pub counter_increment: Vec<CounterOperation>,
    /// Inherited quote pairs. Empty means the locale-independent CSS default.
    pub quotes: Vec<QuotePair>,

    // ── Line clamping ───────────────────────────────────────────────
    pub line_clamp: LineClamp,
    pub block_ellipsis: BlockEllipsis,
    pub webkit_box_orient: WebkitBoxOrient,
    /// Whether the authored display was `-webkit-box`/`-webkit-inline-box`.
    pub legacy_webkit_box: bool,
    /// Whether `line_clamp` came from the legacy `-webkit-line-clamp`
    /// property. It only activates on a vertical legacy WebKit box.
    pub legacy_webkit_line_clamp: bool,

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

    // ── Fragmentation ───────────────────────────────────────────────
    /// CSS `orphans`. Initial: `2`. Inherited.
    /// Minimum number of lines in a block container that must be left
    /// at the bottom of a fragmentainer (before a fragmentation break).
    ///
    /// CSS Break 3 §4.1. Blink: `ComputedStyle::Orphans()`.
    pub orphans: u32,

    /// CSS `widows`. Initial: `2`. Inherited.
    /// Minimum number of lines in a block container that must be left
    /// at the top of a fragmentainer (after a fragmentation break).
    ///
    /// CSS Break 3 §4.1. Blink: `ComputedStyle::Widows()`.
    pub widows: u32,

    /// CSS `break-before`. Initial: `auto`.
    /// Controls forced/avoided breaks before this box.
    pub break_before: BreakValue,

    /// CSS `break-after`. Initial: `auto`.
    /// Controls forced/avoided breaks after this box.
    pub break_after: BreakValue,

    /// CSS `break-inside`. Initial: `auto`.
    /// Controls whether breaks are allowed inside this box.
    pub break_inside: BreakInside,

    /// CSS `box-decoration-break`. Initial: `slice`.
    /// Controls whether inline decorations (border, padding, background)
    /// are sliced or cloned at fragment boundaries.
    ///
    /// Blink: `BoxDecorationBreak()` in `ComputedStyle`.
    pub box_decoration_break: BoxDecorationBreak,

    // ── Multi-column Layout (CSS Multicol Level 1) ──────────────────
    /// CSS `column-count`. `None` = `auto` (no explicit count).
    pub column_count: Option<u32>,

    /// CSS `column-width`. `None` = `auto` (no explicit width).
    pub column_width: Option<Length>,

    /// CSS `column-height`. `None` = `auto` (no explicit fragmentainer height).
    pub column_height: Option<Length>,

    /// CSS `column-fill`. Initial: `balance`.
    pub column_fill: ColumnFill,

    /// CSS `column-wrap`. Initial: `wrap`.
    pub column_wrap: ColumnWrap,

    /// CSS `column-span`. Initial: `none`.
    pub column_span: ColumnSpan,

    /// CSS `column-rule-width`. Initial: `medium` (3px, matching border).
    pub column_rule_width: i32,

    /// CSS `column-rule-style`. Initial: `none`.
    pub column_rule_style: BorderStyle,

    /// CSS `column-rule-color`. Initial: `currentColor`.
    pub column_rule_color: StyleColor,

    // ── Aspect Ratio (CSS Sizing Level 3) ────────────────────────────
    /// CSS `aspect-ratio`. Initial: `auto` (None).
    /// Stores `(width, height)` ratio and an auto flag for
    /// `aspect-ratio: auto 16/9`.
    pub aspect_ratio: Option<AspectRatio>,

    // ── First-Line Pseudo (CSS 2.1 §5.12.1) ─────────────────────────
    /// Alternate style for `::first-line` pseudo-element.
    /// When `Some`, the first line of the block container uses this
    /// style for text-related properties (font, color, text-decoration, etc.).
    /// Blink: `HighlightPseudoStyle(kPseudoIdFirstLine)` in style_adjuster.cc.
    pub first_line_style: Option<Box<ComputedStyle>>,

    /// Alternate style for the `::first-letter` pseudo-element.
    pub first_letter_style: Option<Box<ComputedStyle>>,

    /// Internal computed-style marker carried by the extracted first-letter
    /// text fragment so paint can draw its pseudo box decorations.
    pub is_first_letter_pseudo: bool,

    // ── Text Wrap (CSS Text Level 4) ─────────────────────────────────
    /// CSS `text-wrap`. Initial: `wrap`. Inherited.
    /// Controls paragraph-level line breaking strategy.
    pub text_wrap: TextWrap,

    // ── Initial Letter (CSS Inline Level 3 §5) ──────────────────────
    /// CSS `initial-letter`. Initial: `None` (normal).
    /// When `Some`, the first letter is sized/sunk as a drop-cap or raised cap.
    pub initial_letter: Option<InitialLetter>,
}

impl ComputedStyle {
    /// Create a style with all initial values matching Blink's defaults.
    pub fn initial() -> Self {
        Self {
            display: Display::INITIAL, // inline
            list_item_is_flow_root: false,
            list_style_position: ListStylePosition::Outside,
            list_style_type: ListStyleType::Disc,
            position: Position::INITIAL, // static
            establishes_transform_containing_block: false,
            will_change_transform: false,
            float: Float::INITIAL,         // none
            clear: Clear::INITIAL,         // none
            overflow_x: Overflow::INITIAL, // visible
            overflow_y: Overflow::INITIAL, // visible
            resize: Resize::INITIAL,
            scrollbar_width: ScrollbarWidth::INITIAL,
            scrollbar_gutter: ScrollbarGutter::INITIAL,
            overflow_clip_margin: 0.0,
            overflow_clip_box: OverflowClipBox::default(),
            scrollbar_thumb_color: None,
            scrollbar_track_color: None,
            box_sizing: BoxSizing::INITIAL,  // content-box
            visibility: Visibility::INITIAL, // visible
            direction: Direction::INITIAL,   // ltr

            width: Length::auto(),
            height: Length::auto(),
            min_width: Length::auto(),
            min_height: Length::auto(),
            max_width: Length::none(),  // NOT auto — Blink uses kNone
            max_height: Length::none(), // NOT auto

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

            border_top_style: BorderStyle::INITIAL, // none
            border_right_style: BorderStyle::INITIAL,
            border_bottom_style: BorderStyle::INITIAL,
            border_left_style: BorderStyle::INITIAL,

            border_top_color: StyleColor::default(), // currentColor
            border_right_color: StyleColor::default(),
            border_bottom_color: StyleColor::default(),
            border_left_color: StyleColor::default(),

            outline_width: 3,                     // medium (3px)
            outline_style: BorderStyle::INITIAL,  // none
            outline_color: StyleColor::default(), // currentColor
            outline_offset: 0,

            border_top_left_radius: (0.0, 0.0),
            border_top_right_radius: (0.0, 0.0),
            border_bottom_right_radius: (0.0, 0.0),
            border_bottom_left_radius: (0.0, 0.0),

            background_color: Color::TRANSPARENT,
            background_linear_gradient: None,
            background_layers: Vec::new(),
            mask_layers: Vec::new(),
            background_clip: BackgroundClip::BorderBox,
            background_attachment: BackgroundAttachment::Scroll,
            border_image: None,
            color: Color::BLACK,
            opacity: 1.0,
            box_shadow: Vec::new(),
            z_index: None, // auto

            // Flexbox — container properties
            flex_direction: FlexDirection::INITIAL,     // row
            flex_wrap: FlexWrap::INITIAL,               // nowrap
            justify_content: ContentAlignment::INITIAL, // normal
            align_items: ItemAlignment::INITIAL_ITEMS,  // normal (→ stretch in flex)
            align_content: ContentAlignment::INITIAL,   // normal
            row_gap: None,                              // normal = 0px for flex
            column_gap: None,                           // normal = 0px for flex

            // Flexbox — item properties
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::auto(),
            align_self: ItemAlignment::INITIAL_SELF, // auto (→ inherits align-items)
            order: 0,

            // Tables
            table_layout: TableLayout::Auto,
            border_collapse: BorderCollapse::Separate,
            // CSS Tables initial value. HTML's 2px spacing is a UA rule for
            // semantic <table> elements, not the computed-style initial.
            border_spacing: (Length::zero(), Length::zero()),
            caption_side: CaptionSide::Top,
            empty_cells: EmptyCells::Show,

            // Grid
            grid_template_columns: GridTrackList::None,
            grid_template_rows: GridTrackList::None,
            grid_auto_columns: vec![GridTrackSize::auto()],
            grid_auto_rows: vec![GridTrackSize::auto()],
            grid_auto_flow: GridAutoFlow::default(),
            grid_column: GridPlacement::default(),
            grid_row: GridPlacement::default(),
            grid_template_areas: GridTemplateAreas::default(),
            justify_items: ItemAlignment::INITIAL_ITEMS,
            justify_self: ItemAlignment::INITIAL_SELF,
            margin_trim: MarginTrim::NONE,

            // Containment and static container queries
            contain: Containment::NONE,
            content_visibility: ContentVisibility::Visible,
            contain_intrinsic_width: ContainIntrinsicLength::NONE,
            contain_intrinsic_height: ContainIntrinsicLength::NONE,
            container_type: ContainerType::Normal,
            container_names: Vec::new(),
            scroll_marker_group: ScrollMarkerGroup::None,
            scroll_target_group: ScrollTargetGroup::None,
            scroll_snap_align: ScrollSnapAlign::None,
            scroll_snap_axis: ScrollSnapAxis::None,

            // Replaced content and deterministic effects
            object_fit: ObjectFit::Fill,
            object_position: ObjectPosition::default(),
            transform: Transform2D::IDENTITY,
            transform_origin: (Length::percent(50.0), Length::percent(50.0)),
            clip_path_inset: None,
            shape_outside: ShapeOutside::None,
            shape_margin: Length::zero(),
            shape_image_threshold: 0.0,
            animation_snapshot: None,

            // Text & Font — inherited text properties
            text_align: TextAlign::INITIAL,   // start
            white_space: WhiteSpace::INITIAL, // normal

            // Font properties
            font_family: FontFamilyList::default(), // sans-serif
            font_size: 16.0,                        // CSS medium
            font_weight: FontWeight::NORMAL,        // 400
            font_style: FontStyleEnum::Normal,
            font_stretch: FontStretch::NORMAL, // 100%
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
            text_align_last: TextAlignLast::INITIAL, // auto
            text_justify: TextJustify::INITIAL,      // auto
            word_break: WordBreak::INITIAL,          // normal
            overflow_wrap: OverflowWrap::INITIAL,    // normal
            line_break: LineBreak::INITIAL,          // auto
            hyphens: Hyphens::INITIAL,               // manual
            hyphenate_limit_chars: (5, 2, 2),        // Blink defaults

            // Text decoration
            text_decoration_line: TextDecorationLine::NONE,
            text_decoration_style: TextDecorationStyle::INITIAL, // solid
            text_decoration_color: StyleColor::CurrentColor,
            text_decoration_thickness: TextDecorationThickness::Auto,
            text_underline_offset: Length::auto(),
            text_underline_position: TextUnderlinePosition::INITIAL, // auto
            text_decoration_skip_ink: TextDecorationSkipInk::INITIAL, // auto

            // Text transform
            text_transform: TextTransform::INITIAL, // none
            text_overflow: TextOverflow::INITIAL,   // clip
            content: None,
            counter_reset: Vec::new(),
            counter_set: Vec::new(),
            counter_increment: Vec::new(),
            quotes: Vec::new(),
            line_clamp: LineClamp::None,
            block_ellipsis: BlockEllipsis::Auto,
            webkit_box_orient: WebkitBoxOrient::Horizontal,
            legacy_webkit_box: false,
            legacy_webkit_line_clamp: false,

            // Vertical alignment
            vertical_align: VerticalAlign::Baseline,

            // Writing & bidi
            unicode_bidi: UnicodeBidi::INITIAL,         // normal
            writing_mode: WritingMode::INITIAL,         // horizontal-tb
            text_orientation: TextOrientation::INITIAL, // mixed

            // Text rendering
            text_rendering: TextRendering::Auto,
            font_smoothing: FontSmoothing::Auto,

            // Text shadow
            text_shadow: Vec::new(),

            // Hanging punctuation
            hanging_punctuation: HangingPunctuation::NONE,

            // Text emphasis
            text_emphasis_mark: TextEmphasisMark::INITIAL, // none
            text_emphasis_fill: TextEmphasisFill::INITIAL, // filled
            text_emphasis_position: TextEmphasisPosition::INITIAL, // over right
            text_emphasis_color: StyleColor::CurrentColor,
            text_combine_upright: TextCombineUpright::INITIAL, // none

            // Ruby annotation
            ruby_position: RubyPosition::INITIAL, // over
            ruby_align: RubyAlign::INITIAL,       // space-around

            // Tab size
            tab_size: TabSize::Spaces(8),

            // Font palette
            font_palette: FontPalette::INITIAL, // normal

            // Locale
            locale: None,

            // Fragmentation
            orphans: 2,                                        // CSS initial
            widows: 2,                                         // CSS initial
            break_before: BreakValue::INITIAL,                 // auto
            break_after: BreakValue::INITIAL,                  // auto
            break_inside: BreakInside::INITIAL,                // auto
            box_decoration_break: BoxDecorationBreak::INITIAL, // slice

            // Multi-column layout
            column_count: None,                       // auto
            column_width: None,                       // auto
            column_height: None,                      // auto
            column_fill: ColumnFill::INITIAL,         // balance
            column_wrap: ColumnWrap::INITIAL,         // wrap
            column_span: ColumnSpan::INITIAL,         // none
            column_rule_width: 3,                     // medium (3px)
            column_rule_style: BorderStyle::INITIAL,  // none
            column_rule_color: StyleColor::default(), // currentColor

            // Aspect ratio
            aspect_ratio: None, // auto (no specified ratio)

            // First-line pseudo
            first_line_style: None,   // no ::first-line
            first_letter_style: None, // no ::first-letter
            is_first_letter_pseudo: false,

            // Text wrap
            text_wrap: TextWrap::INITIAL, // wrap

            // Initial letter
            initial_letter: None, // normal (no drop-cap)
        }
    }

    /// Initial pseudo-element style with the inherited properties copied from
    /// its originating element. Generated builders have no runtime cascade,
    /// so they use this boundary before applying pseudo declarations.
    pub fn for_pseudo(origin: &Self) -> Self {
        let mut style = Self::initial();
        style.color = origin.color;
        style.visibility = origin.visibility;
        style.direction = origin.direction;
        style.font_family = origin.font_family.clone();
        style.font_size = origin.font_size;
        style.font_weight = origin.font_weight;
        style.font_style = origin.font_style;
        style.font_stretch = origin.font_stretch;
        style.font_variant_caps = origin.font_variant_caps;
        style.font_variant_ligatures = origin.font_variant_ligatures;
        style.font_variant_numeric = origin.font_variant_numeric;
        style.font_variant_east_asian = origin.font_variant_east_asian;
        style.font_variant_position = origin.font_variant_position;
        style.font_variant_alternates = origin.font_variant_alternates;
        style.font_optical_sizing = origin.font_optical_sizing;
        style.font_synthesis_weight = origin.font_synthesis_weight;
        style.font_synthesis_style = origin.font_synthesis_style;
        style.font_feature_settings = origin.font_feature_settings.clone();
        style.font_variation_settings = origin.font_variation_settings.clone();
        style.line_height = origin.line_height;
        style.letter_spacing = origin.letter_spacing;
        style.word_spacing = origin.word_spacing;
        style.text_align = origin.text_align;
        style.white_space = origin.white_space;
        style.text_align_last = origin.text_align_last;
        style.text_justify = origin.text_justify;
        style.word_break = origin.word_break;
        style.overflow_wrap = origin.overflow_wrap;
        style.line_break = origin.line_break;
        style.hyphens = origin.hyphens;
        style.hyphenate_limit_chars = origin.hyphenate_limit_chars;
        style.text_transform = origin.text_transform;
        style.text_underline_position = origin.text_underline_position;
        style.text_decoration_skip_ink = origin.text_decoration_skip_ink;
        style.unicode_bidi = origin.unicode_bidi;
        style.writing_mode = origin.writing_mode;
        style.text_orientation = origin.text_orientation;
        style.text_rendering = origin.text_rendering;
        style.font_smoothing = origin.font_smoothing;
        style.text_shadow = origin.text_shadow.clone();
        style.quotes = origin.quotes.clone();
        style.text_emphasis_mark = origin.text_emphasis_mark;
        style.text_emphasis_fill = origin.text_emphasis_fill;
        style.text_emphasis_position = origin.text_emphasis_position;
        style.text_emphasis_color = origin.text_emphasis_color;
        style.text_combine_upright = origin.text_combine_upright;
        style.ruby_position = origin.ruby_position;
        style.ruby_align = origin.ruby_align;
        style.tab_size = origin.tab_size;
        style.font_palette = origin.font_palette.clone();
        style.locale = origin.locale.clone();
        style.orphans = origin.orphans;
        style.widows = origin.widows;
        style.text_wrap = origin.text_wrap;
        style
    }

    /// Initial anonymous-box style with inherited properties copied from its
    /// parent formatting box. Anonymous table wrappers participate in the
    /// computed-value inheritance chain even though they have no DOM node on
    /// which the generated porter can materialize the cascade.
    pub fn for_anonymous_box(parent: &Self) -> Self {
        Self::for_pseudo(parent)
    }

    // ── Convenience: effective border width (0 if style is none/hidden) ──

    /// Effective border-top-width: 0 if border-style is none/hidden.
    /// This matches Blink's "used value" computation.
    #[inline]
    pub fn effective_border_top(&self) -> i32 {
        if self.border_top_style.has_visible_border() {
            self.border_top_width
        } else {
            0
        }
    }

    #[inline]
    pub fn effective_border_right(&self) -> i32 {
        if self.border_right_style.has_visible_border() {
            self.border_right_width
        } else {
            0
        }
    }

    #[inline]
    pub fn effective_border_bottom(&self) -> i32 {
        if self.border_bottom_style.has_visible_border() {
            self.border_bottom_width
        } else {
            0
        }
    }

    #[inline]
    pub fn effective_border_left(&self) -> i32 {
        if self.border_left_style.has_visible_border() {
            self.border_left_width
        } else {
            0
        }
    }

    /// Effective outline width: 0 if outline-style is none/hidden.
    #[inline]
    pub fn effective_outline_width(&self) -> i32 {
        if self.outline_style.has_visible_border() {
            self.outline_width
        } else {
            0
        }
    }

    /// True if this element has a visible outline.
    #[inline]
    pub fn has_outline(&self) -> bool {
        self.effective_outline_width() > 0
    }

    /// True if this element establishes a new formatting context.
    /// Mirrors Blink's `CreatesNewFormattingContext()`.
    pub fn creates_new_formatting_context(&self) -> bool {
        // Flex/grid containers, inline-block, flow-root, overflow != visible,
        // absolutely positioned, floated — all create new BFC.
        // Per CSS Overflow 3: overflow:clip does NOT establish a BFC.
        self.display.is_new_formatting_context()
            || (self.display == Display::ListItem && self.list_item_is_flow_root)
            || self.has_layout_containment()
            || self.position.is_absolutely_positioned()
            || self.float != Float::None
            || self.is_scroll_container()
    }

    /// Size containment in the element's logical inline axis. A size query
    /// container establishes the same containment even when `contain` does
    /// not spell it out explicitly.
    #[inline]
    pub fn has_inline_size_containment(&self) -> bool {
        self.contain.contains(Containment::SIZE)
            || self.contain.contains(Containment::INLINE_SIZE)
            || matches!(
                self.container_type,
                ContainerType::InlineSize | ContainerType::Size
            )
    }

    /// Size containment in the element's logical block axis.
    #[inline]
    pub fn has_block_size_containment(&self) -> bool {
        self.contain.contains(Containment::SIZE) || self.container_type == ContainerType::Size
    }

    /// Layout containment, including the implicit containment established by
    /// size query containers.
    #[inline]
    pub fn has_layout_containment(&self) -> bool {
        self.contain.contains(Containment::LAYOUT)
            || self.contain.contains(Containment::PAINT)
            || self.container_type != ContainerType::Normal
    }

    #[inline]
    pub fn has_paint_containment(&self) -> bool {
        self.contain.contains(Containment::PAINT)
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

    /// True if any border-radius corner is non-zero.
    #[inline]
    pub fn has_border_radius(&self) -> bool {
        self.border_top_left_radius != (0.0, 0.0)
            || self.border_top_right_radius != (0.0, 0.0)
            || self.border_bottom_right_radius != (0.0, 0.0)
            || self.border_bottom_left_radius != (0.0, 0.0)
    }

    /// Chromium: `IsScrollContainer()` — true when overflow creates a scroll
    /// container (overflow is not visible/clip on either axis, i.e., auto or
    /// scroll). Used for automatic minimum size with aspect-ratio.
    pub fn is_scroll_container(&self) -> bool {
        // Per CSS Overflow 3, overflow: hidden/scroll/auto all create a scroll
        // container.  overflow: clip does NOT.
        self.overflow_x != Overflow::Visible && self.overflow_x != Overflow::Clip
            || self.overflow_y != Overflow::Visible && self.overflow_y != Overflow::Clip
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
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
        assert_eq!(s.overflow_clip_margin, 0.0);
        assert_eq!(s.scrollbar_thumb_color, None);
        assert_eq!(s.scrollbar_track_color, None);

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

        // Table, Grid and containment
        assert_eq!(s.table_layout, TableLayout::Auto);
        assert_eq!(s.border_collapse, BorderCollapse::Separate);
        assert!(matches!(s.grid_template_columns, GridTrackList::None));
        assert!(matches!(s.grid_template_rows, GridTrackList::None));
        assert_eq!(s.contain, Containment::NONE);
        assert_eq!(s.content_visibility, ContentVisibility::Visible);
        assert_eq!(s.container_type, ContainerType::Normal);
        assert_eq!(s.scroll_marker_group, ScrollMarkerGroup::None);
        assert_eq!(s.scroll_target_group, ScrollTargetGroup::None);
        assert_eq!(s.object_fit, ObjectFit::Fill);
        assert_eq!(s.transform, Transform2D::IDENTITY);
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
        let a = HangingPunctuation {
            first: true,
            last: false,
            force_end: false,
            allow_end: false,
        };
        let b = HangingPunctuation {
            first: true,
            last: false,
            force_end: false,
            allow_end: false,
        };
        let c = HangingPunctuation {
            first: false,
            last: true,
            force_end: false,
            allow_end: false,
        };
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
        s.text_emphasis_position = TextEmphasisPosition {
            over: false,
            right: false,
        };
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
