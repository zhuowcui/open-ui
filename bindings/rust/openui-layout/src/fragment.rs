//! Layout fragments — the output of a layout algorithm.
//!
//! Extracted from Blink's `PhysicalFragment` / `PhysicalBoxFragment`.
//! A fragment represents a positioned piece of the layout tree, ready for
//! painting. The fragment tree mirrors the element tree but with concrete
//! sizes and offsets.

use openui_dom::NodeId;
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::ComputedStyle;
use openui_text::ShapeResult;
use std::sync::Arc;

use crate::exclusions::ExclusionArea;
use crate::inline::text_combine::TextCombineLayout;

/// What kind of fragment this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    /// A box fragment (from a block, flex, grid, or inline-block element).
    Box,
    /// A text fragment (from a text node).
    Text,
    /// The root viewport fragment.
    Viewport,
    /// A column rule between multicol columns.
    ColumnRule,
    /// An anonymous column box in a multicol container.
    /// Clips content to column boundaries (CSS Multicol §3.1).
    ColumnBox,
}

/// A positioned layout fragment, ready for painting.
///
/// Mirrors Blink's `PhysicalBoxFragment`. Contains the resolved size,
/// position relative to parent, and references back to the DOM node
/// and style for painting.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// Which DOM node produced this fragment.
    pub node_id: NodeId,

    /// What type of fragment.
    pub kind: FragmentKind,

    /// Offset from the parent fragment's top-left corner.
    pub offset: PhysicalOffset,

    /// The fragment's border-box size.
    pub size: PhysicalSize,

    /// Resolved padding (in LayoutUnit).
    pub padding: BoxStrut,

    /// Resolved border widths (in LayoutUnit).
    pub border: BoxStrut,

    /// Resolved margin (in LayoutUnit, for debug/paint use).
    pub margin: BoxStrut,

    /// Child fragments, positioned relative to this fragment.
    pub children: Vec<Fragment>,

    /// Shaped text result for text fragments (populated by inline layout).
    ///
    /// Contains glyph IDs, positions, and per-character metadata from
    /// HarfBuzz shaping. Used by the paint system to render glyphs via
    /// `to_text_blob()`.
    pub shape_result: Option<Arc<ShapeResult>>,

    /// Text content for text fragments (the original string that was shaped).
    pub text_content: Option<String>,

    /// Inherited style for anonymous fragments (e.g., ellipsis "…") that have
    /// no DOM node. Used by the painter to render with the correct color/font.
    pub inherited_style: Option<ComputedStyle>,

    /// Distance from the fragment's top edge to the text baseline.
    /// Computed during layout; used by paint to avoid recomputing from metrics.
    pub baseline_offset: f32,

    /// Text-combine-upright (tate-chū-yoko) layout data.
    ///
    /// Present only on text fragments inside a vertical writing mode where
    /// `text-combine-upright: all` is active. The paint system uses this to
    /// apply horizontal scaling and centering transforms.
    ///
    /// Blink: `LayoutTextCombine` attached to the `LayoutText` object.
    pub text_combine: Option<TextCombineLayout>,

    /// Ink overflow rectangle — the area that child content extends beyond
    /// this fragment's border-box. `None` means no overflow (children fit
    /// entirely within the border-box).
    ///
    /// Blink: `PhysicalBoxFragment::ScrollableOverflow()`.
    pub overflow_rect: Option<PhysicalRect>,

    /// Whether this fragment clips overflowing content.
    ///
    /// Set to `true` when the element's `overflow-x` or `overflow-y` is not
    /// `visible`. The paint system uses this flag to apply a clip rect before
    /// painting children.
    pub has_overflow_clip: bool,

    /// Multicol fragmentainers clip fragmented content in the block axis while
    /// still allowing inline overflow to paint into column gaps.
    pub block_axis_clip_only: bool,

    /// Optional block-axis limit for this fragment's own decorations
    /// (background/border/shadow), while leaving children free to overflow.
    pub decoration_paint_block_size: Option<LayoutUnit>,

    /// Allows selected zero-height fragments to paint outlines when their
    /// formatting context keeps the outline visible.
    pub paint_zero_block_outline: bool,

    /// Out-of-flow candidates that couldn't be resolved at this level.
    ///
    /// When a `position: static` element encounters absolutely-positioned
    /// children, it cannot be their containing block. These candidates are
    /// passed up to the nearest positioned ancestor (or the root) via this
    /// field. The parent's layout absorbs them and resolves their positions.
    pub oof_candidates: Vec<crate::out_of_flow::OutOfFlowCandidate>,

    /// End margin strut — propagated upward for parent/child margin collapsing.
    ///
    /// CSS 2.1 §8.3.1: When a block's bottom margin is not separated from
    /// its last child's margin by border, padding, or content, the margins
    /// collapse together. This field carries the unresolved trailing margin
    /// strut so the parent can merge it with subsequent sibling margins.
    ///
    /// Blink: `LayoutResult::EndMarginStrut()`.
    pub end_margin_strut: openui_geometry::MarginStrut,

    /// Start margin strut — propagated upward for parent/first-child margin collapsing.
    ///
    /// CSS 2.1 §8.3.1: When a block's top margin is not separated from
    /// its first child's margin by border, padding, or content, the child's
    /// margin collapses with the parent's. This field carries the unresolved
    /// start margin strut so the parent can absorb it.
    pub start_margin_strut: openui_geometry::MarginStrut,

    /// First baseline of this fragment, measured from the fragment's top edge.
    ///
    /// CSS Inline 3 §3: The first baseline set of a box is the alignment
    /// baseline from its first line box (if it has inline content) or
    /// from its first in-flow child's first baseline. Used by flex/grid
    /// for `align-items: baseline`.
    ///
    /// Blink: `PhysicalBoxFragment::FirstBaseline()`.
    pub first_baseline: Option<LayoutUnit>,

    /// Last baseline of this fragment, measured from the fragment's top edge.
    ///
    /// CSS Inline 3 §3: The last baseline set of a box is the alignment
    /// baseline from its last line box (if it has inline content) or
    /// from its last in-flow child's last baseline. Used by flex/grid
    /// for `align-items: last baseline`.
    ///
    /// Blink: `PhysicalBoxFragment::LastBaseline()`.
    pub last_baseline: Option<LayoutUnit>,

    /// Whether this fragment is the first fragment in an inline box
    /// continuation (should render inline-start border/padding/margin).
    ///
    /// When an inline element (e.g. `<span>`) spans multiple lines, only
    /// the first fragment gets inline-start MBP (or all fragments when
    /// `box-decoration-break: clone`).
    ///
    /// Blink: `PhysicalBoxFragment::IsFirstForNode()`.
    pub is_first_for_node: bool,

    /// Whether this fragment is the last fragment in an inline box
    /// continuation (should render inline-end border/padding/margin).
    ///
    /// When an inline element spans multiple lines, only the last fragment
    /// gets inline-end MBP (or all fragments when
    /// `box-decoration-break: clone`).
    ///
    /// Blink: `PhysicalBoxFragment::IsLastForNode()`.
    pub is_last_for_node: bool,

    /// Whether this is a per-line fragment of a non-atomic inline box.
    ///
    /// Inline continuations slice their inline-start/inline-end decoration;
    /// block fragmentation slices block-start/block-end decoration.  Keeping
    /// the axes distinct avoids applying multicol border rules to spans.
    pub is_inline_box_fragment: bool,

    /// Break token for fragmentation — records where layout was interrupted
    /// so the next fragmentainer can resume from this point.
    ///
    /// `None` means this fragment consumed all content (no fragmentation break).
    /// `Some(token)` means content was split and the next fragmentainer should
    /// use this token to resume layout.
    ///
    /// Blink: `PhysicalBoxFragment::BreakToken()`.
    pub break_token: Option<crate::fragmentation::BreakToken>,

    /// Whether a descendant float forced BFC offset resolution during layout.
    ///
    /// CSS 2.1 §9.5: When a float is encountered, the BFC block offset must
    /// be resolved immediately (Chromium: `ResolveBFCBlockOffset()`). This
    /// resolution must cascade up to the BFC root. This flag signals to the
    /// parent that it should also resolve its own pending margin strut to
    /// prevent incorrect margin-collapse propagation past the float.
    pub float_resolved_bfc: bool,

    /// Float exclusions from this block's layout that need to propagate to
    /// the nearest BFC ancestor's exclusion space.
    ///
    /// CSS 2.1 §9.5: Floats participate in the nearest BFC's formatting
    /// context, even when nested inside non-BFC wrapper blocks. When a
    /// non-BFC block contains (directly or transitively) float children,
    /// this field carries their exclusion areas so the parent can absorb
    /// them into its own exclusion space. Coordinates are relative to this
    /// block's content area origin.
    ///
    /// Empty for BFC blocks (they consume their own floats).
    pub float_exclusions: Vec<ExclusionArea>,
}

impl Fragment {
    /// Create a new box fragment.
    pub fn new_box(node_id: NodeId, size: PhysicalSize) -> Self {
        Self {
            node_id,
            kind: FragmentKind::Box,
            offset: PhysicalOffset::zero(),
            size,
            padding: BoxStrut::zero(),
            border: BoxStrut::zero(),
            margin: BoxStrut::zero(),
            children: Vec::new(),
            shape_result: None,
            text_content: None,
            inherited_style: None,
            baseline_offset: 0.0,
            text_combine: None,
            overflow_rect: None,
            has_overflow_clip: false,
            block_axis_clip_only: false,
            decoration_paint_block_size: None,
            paint_zero_block_outline: false,
            oof_candidates: Vec::new(),
            end_margin_strut: openui_geometry::MarginStrut::new(),
            start_margin_strut: openui_geometry::MarginStrut::new(),
            first_baseline: None,
            last_baseline: None,
            is_first_for_node: true,
            is_last_for_node: true,
            is_inline_box_fragment: false,
            break_token: None,
            float_resolved_bfc: false,
            float_exclusions: Vec::new(),
        }
    }

    /// Create a new text fragment with a shape result.
    ///
    /// Blink: `PhysicalTextFragment` constructor in
    /// `core/layout/physical_fragment.h`.
    pub fn new_text(
        node_id: NodeId,
        size: PhysicalSize,
        shape_result: Arc<ShapeResult>,
        text_content: String,
    ) -> Self {
        Self {
            node_id,
            kind: FragmentKind::Text,
            offset: PhysicalOffset::zero(),
            size,
            padding: BoxStrut::zero(),
            border: BoxStrut::zero(),
            margin: BoxStrut::zero(),
            children: Vec::new(),
            shape_result: Some(shape_result),
            text_content: Some(text_content),
            inherited_style: None,
            baseline_offset: 0.0,
            text_combine: None,
            overflow_rect: None,
            has_overflow_clip: false,
            block_axis_clip_only: false,
            decoration_paint_block_size: None,
            paint_zero_block_outline: false,
            oof_candidates: Vec::new(),
            end_margin_strut: openui_geometry::MarginStrut::new(),
            start_margin_strut: openui_geometry::MarginStrut::new(),
            first_baseline: None,
            last_baseline: None,
            is_first_for_node: true,
            is_last_for_node: true,
            is_inline_box_fragment: false,
            break_token: None,
            float_resolved_bfc: false,
            float_exclusions: Vec::new(),
        }
    }

    /// The content box rect (border-box minus border minus padding).
    pub fn content_offset(&self) -> PhysicalOffset {
        PhysicalOffset::new(
            self.border.left + self.padding.left,
            self.border.top + self.padding.top,
        )
    }

    /// The content box size.
    pub fn content_size(&self) -> PhysicalSize {
        PhysicalSize::new(
            self.size.width
                - self.border.left
                - self.border.right
                - self.padding.left
                - self.padding.right,
            self.size.height
                - self.border.top
                - self.border.bottom
                - self.padding.top
                - self.padding.bottom,
        )
    }

    /// The padding box size (border-box minus border).
    pub fn padding_box_size(&self) -> PhysicalSize {
        PhysicalSize::new(
            self.size.width - self.border.left - self.border.right,
            self.size.height - self.border.top - self.border.bottom,
        )
    }

    /// Width of the border-box.
    #[inline]
    pub fn width(&self) -> LayoutUnit {
        self.size.width
    }

    /// Height of the border-box.
    #[inline]
    pub fn height(&self) -> LayoutUnit {
        self.size.height
    }

    /// Set whether this fragment clips overflowing content.
    pub fn set_overflow_clip(&mut self, clip: bool) {
        self.has_overflow_clip = clip;
    }

    /// The scrollable overflow area of this fragment.
    ///
    /// Returns the explicitly computed `overflow_rect` if present, otherwise
    /// falls back to the border-box rect (offset=zero, size=border-box).
    ///
    /// Mirrors Blink's `PhysicalBoxFragment::ScrollableOverflow()`.
    pub fn scrollable_overflow(&self) -> PhysicalRect {
        self.overflow_rect
            .unwrap_or_else(|| PhysicalRect::new(PhysicalOffset::zero(), self.size))
    }

    /// The border-box rect with offset at zero (local coordinates).
    #[inline]
    pub fn border_box_rect(&self) -> PhysicalRect {
        PhysicalRect::new(PhysicalOffset::zero(), self.size)
    }
}
