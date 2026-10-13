//! Layout fragments — the output of a layout algorithm.
//!
//! Extracted from Blink's `PhysicalFragment` / `PhysicalBoxFragment`.
//! A fragment represents a positioned piece of the layout tree, ready for
//! painting. The fragment tree mirrors the element tree but with concrete
//! sizes and offsets.

use openui_dom::NodeId;
use openui_geometry::{
    BoxStrut, LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize, WritingDirectionMode,
};
use openui_style::{ComputedStyle, Overflow, TextOrientation, WritingMode};
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

/// Whether a fragment carries its source element's box or only layout structure.
///
/// Layout helpers can reuse an authored node for style lookup without owning
/// that element's geometry, input target, or independently applied effects.
/// Continuations of an authored box retain `AuthoredElement` ownership.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FragmentBoxOwnership {
    #[default]
    AuthoredElement,
    LayoutHelper,
}

/// Retained identity of an anonymous formatting box with native scroll state.
/// The owner remains the authored semantic element; this box contributes no
/// second element geometry, input target, accessibility node or element effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnonymousLayoutBox {
    FieldsetContent { owner: NodeId },
}

/// The table formatting role retained by layout for background painting.
///
/// These are source identities and grid indices, not copied paint rectangles.
/// Fragment projection and continuation slicing preserve them while the
/// painter uses the actual cell fragments in each retained row. In particular,
/// a spanning cell receives the background of its originating column only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentTablePart {
    Table,
    /// The table grid decoration delegated by a captioned authored wrapper.
    GridDecoration,
    Section,
    Row,
    Cell {
        column_index: usize,
    },
    Column {
        start_column: usize,
        span: usize,
    },
    ColumnGroup {
        start_column: usize,
        span: usize,
    },
}

impl FragmentTablePart {
    pub fn column_range(self) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Column { start_column, span } | Self::ColumnGroup { start_column, span } => {
                Some(start_column..start_column.saturating_add(span))
            }
            _ => None,
        }
    }

    pub fn paints_into_cells(self) -> bool {
        matches!(self, Self::Section | Self::Row)
    }
}

/// Final layout geometry used to position column/group backgrounds across
/// the real section continuations of one authored table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableColumnBackgroundPositioning {
    pub rect: PhysicalRect,
    pub writing_direction: WritingDirectionMode,
}

/// Immutable geometry of the document's viewport scroll container.
///
/// Scrollbar gutters belong to the viewport, rather than to an authored
/// element's border or padding. Layout, paint and native input consume this
/// same geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportScrollport {
    pub client_rect: PhysicalRect,
    pub content_rect: PhysicalRect,
    pub horizontal_scrollbar: bool,
    pub vertical_scrollbar: bool,
    pub scrollbar_thickness: LayoutUnit,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub negative_x: bool,
    pub negative_y: bool,
}

/// Resolved orientation of one shaped text run in physical fragment storage.
///
/// Inline layout is the authority for this value. Paint consumes it without
/// re-reading CSS or attempting to split the run from its Unicode contents.
/// `UnresolvedMixed` deliberately keeps genuinely mixed upright/rotated runs
/// on the pre-W2B path instead of guessing at per-character transforms.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextRunOrientation {
    #[default]
    Horizontal,
    Upright,
    Clockwise,
    CounterClockwise,
    UnresolvedMixed,
}

/// Resolve the paint orientation for a text run during inline construction.
///
/// Rotated vertical and sideways runs retain horizontal shaping and logical
/// inline advance; the fragment projection converts that geometry to physical
/// axes exactly once. Direction affects progression, never rotation handedness.
pub fn resolve_text_run_orientation(style: &ComputedStyle, text: &str) -> TextRunOrientation {
    if text.is_empty() || style.writing_mode == WritingMode::HorizontalTb {
        return TextRunOrientation::Horizontal;
    }

    match style.writing_mode {
        WritingMode::SidewaysRl => TextRunOrientation::Clockwise,
        WritingMode::SidewaysLr => TextRunOrientation::CounterClockwise,
        WritingMode::VerticalRl | WritingMode::VerticalLr => match style.text_orientation {
            TextOrientation::Sideways => TextRunOrientation::Clockwise,
            TextOrientation::Upright => TextRunOrientation::Upright,
            TextOrientation::Mixed => {
                let mut has_upright = false;
                let mut has_rotated = false;
                for character in text.chars() {
                    if openui_text::is_upright_in_mixed_vertical(character) {
                        has_upright = true;
                    } else {
                        has_rotated = true;
                    }
                    if has_upright && has_rotated {
                        return TextRunOrientation::UnresolvedMixed;
                    }
                }
                if has_upright {
                    TextRunOrientation::Upright
                } else {
                    TextRunOrientation::Clockwise
                }
            }
        },
        WritingMode::HorizontalTb => TextRunOrientation::Horizontal,
    }
}

/// Source-space coordinates for decorations sliced across fragmentainers.
///
/// `box-decoration-break:slice` keeps one background positioning area for the
/// unfragmented principal box. Layout records the consumed block coordinate on
/// each continuation so paint can sample that shared area without reconstructing
/// fragmentation decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecorationSlice {
    pub source_block_offset: LayoutUnit,
    pub source_block_size: LayoutUnit,
}

/// One independently painted piece of a collapsed table structural border.
///
/// Collapsed-border conflict resolution can assign adjacent portions of one
/// row edge to different table cells. Each winning portion has independent
/// dash geometry, so paint must retain both its local border box and used
/// side widths instead of clipping one full-row border path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapsedBorderSegment {
    pub rect: PhysicalRect,
    pub border: BoxStrut,
}

/// Logical positioning inputs retained for an out-of-flow fragment that may
/// later enter a fragmentation context.  A multicol ancestor maps these
/// coordinates through its column geometry instead of reverse-engineering a
/// logical position from the already-translated paint offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionedFragmentationData {
    pub static_position: PhysicalOffset,
    /// Physical-axis edge affinities retained with the static-position point.
    pub static_position_horizontal_edge: crate::out_of_flow::StaticPositionEdge,
    pub static_position_vertical_edge: crate::out_of_flow::StaticPositionEdge,
    pub containing_block_offset: PhysicalOffset,
    /// DOM node whose padding box supplies the used containing block.
    pub containing_block_node: NodeId,
    pub containing_block_size: PhysicalSize,
    pub has_inline_containing_block: bool,
    /// Positioned inline whose first/last fragments define the containing
    /// block. Kept so a later fragmentation context can resolve that geometry
    /// from the fragments it actually consumes.
    pub inline_containing_block_node: Option<NodeId>,
    /// Normal-flow block advance contributed by block-in-inline interruptions
    /// before the positioned element's static position.
    pub block_in_inline_static_advance: LayoutUnit,
    /// Visual translation accumulated from relatively positioned ancestors.
    /// It is applied after logical fragmentation so paint movement never
    /// changes the selected fragmentainer.
    pub visual_offset: PhysicalOffset,
    /// Physical reconstruction applied only after continuation selection.
    /// Nested fragmentation uses this when an inner mapper has consumed a
    /// source block-end coordinate that an outer mapper must retain as an
    /// equivalent inline-row advance.
    pub continuation_visual_offset: PhysicalOffset,
    /// Source-space origin of the transformed containing block, when that
    /// ancestor was elided while promoting this positioned fragment.
    pub transform_containing_block_source_offset: Option<PhysicalOffset>,
    /// Logical fragmentainer selected by the owning multicol. This is set on
    /// continuations and lets an ancestor resume nested rows without deriving
    /// flow order from a translated paint offset.
    pub fragmentainer_index: Option<u32>,
    /// Source offset of a containing-block portion split around a spanner.
    /// Such positioned boxes are deliberately materialized in each portion;
    /// the portion clip, rather than ancestor promotion, selects their ink.
    pub split_containing_block_source_offset: Option<LayoutUnit>,
}

/// A transformed ancestor removed when a positioned descendant is promoted
/// into an owning multicol's continuation list.  `source_*` describes the
/// unfragmented box in multicol coordinates; `fragment_*` is resolved for
/// each generated continuation. Paint reapplies these boxes outer-to-inner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromotedTransformAncestor {
    pub node_id: NodeId,
    /// Normal-flow source anchor before authored visual displacement.
    pub source_offset: PhysicalOffset,
    pub source_size: PhysicalSize,
    /// This ancestor's own cumulative displacement, applied after slicing.
    /// It is a vector in the same physical/logical basis as `source_offset`.
    pub visual_offset: PhysicalOffset,
    pub fragment_offset: PhysicalOffset,
    pub fragment_size: PhysicalSize,
    /// Actual owned box, excluding a caption/helper extent, in source space.
    pub source_reference_box: PhysicalRect,
    /// Owned reference box in this physical continuation's local coordinates.
    pub fragment_reference_box: PhysicalRect,
    /// Sliced decorations use the original block reference dimension.
    pub reference_block_size: Option<LayoutUnit>,
    pub reference_block_axis_is_x: bool,
}

impl PromotedTransformAncestor {
    /// Intersect the retained owned box with a logical-block source slice.
    pub(crate) fn resolve_reference_slice(&mut self, consumed: LayoutUnit) {
        let source = self.source_reference_box;
        let top = (source.y() - consumed)
            .max_of(LayoutUnit::zero())
            .min_of(self.fragment_size.height);
        let bottom = (source.bottom() - consumed)
            .max_of(top)
            .min_of(self.fragment_size.height);
        self.fragment_reference_box = PhysicalRect::new(
            PhysicalOffset::new(source.x(), top),
            PhysicalSize::new(source.width(), (bottom - top).clamp_negative_to_zero()),
        );
    }
}

/// Authoritative geometry of a multicol fragment. Nested fragmentation uses
/// this metadata to translate inner overflow-column continuations back into
/// logical block flow without rediscovering column widths from paint offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MulticolFragmentationData {
    pub fragmentainer_block_size: LayoutUnit,
    pub continuation_fragmentainer_block_size: LayoutUnit,
    pub column_inline_start: LayoutUnit,
    pub column_block_start: LayoutUnit,
    pub column_inline_stride: LayoutUnit,
    pub declared_column_count: u32,
}

/// Principal-box continuation geometry retained by layout, independently of
/// decoration slicing and visual overflow. Logical consumption is measured in
/// the fragmenting context's axes; `stitched_size` is always physical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FragmentBoxGeometry {
    pub source_size: PhysicalSize,
    /// Original physical wrapper extent before an ancestor slices this box.
    pub source_fragment_size: PhysicalSize,
    /// Original physical principal border box, relative to that wrapper.
    /// Preserve any authored principal origin independently of wrapper extent.
    pub source_border_box_rect: PhysicalRect,
    /// Unsliced used edges captured before continuation decoration removal.
    pub source_border: BoxStrut,
    pub source_padding: BoxStrut,
    pub stitched_size: PhysicalSize,
    pub writing_direction: WritingDirectionMode,
    pub inline_size: LayoutUnit,
    pub block_size: LayoutUnit,
    pub consumed_block_size: LayoutUnit,
    pub has_next: bool,
    pub is_repeated: bool,
    /// Distinguishes an actual continuation token from the unsplit source.
    pub continuation_record: bool,
}

/// Original layout geometry used by a view timeline's sticky range.
/// Paint and hit testing continue to consume the resolved fragment offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickyTimelineGeometry {
    pub scroll_container: NodeId,
    pub parent_relative_normal_offset: PhysicalOffset,
    pub scroll_container_relative_normal_offset: PhysicalOffset,
    pub containing_block_rect: PhysicalRect,
    pub insets: crate::sticky::StickyConstraintRect,
    /// Stitched/line-union authored box before sticky and scroll movement.
    pub sticky_box_rect: PhysicalRect,
    pub containing_block: NodeId,
    pub nearest_sticky_box: Option<NodeId>,
    pub nearest_sticky_containing_block: Option<NodeId>,
    /// Box-only and box-plus-containing-block channels remain independent.
    pub total_sticky_box_offset: PhysicalOffset,
    pub total_containing_block_offset: PhysicalOffset,
    pub is_fixed_to_view: bool,
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

    /// Ownership of the source element's box, independent of style lookup.
    /// Anonymous layout boxes and decoration helpers retain their source
    /// `node_id` without creating another authored element box.
    pub box_ownership: FragmentBoxOwnership,

    /// Layout-owned anonymous formatting/scroll identity, separate from the
    /// source node used for inherited style and semantic scroll state.
    pub anonymous_layout_box: Option<AnonymousLayoutBox>,

    /// Offset from the parent fragment's top-left corner.
    pub offset: PhysicalOffset,

    /// The fragment's layout extent, including any continuation space needed
    /// to carry visible in-flow overflow.
    pub size: PhysicalSize,

    /// The fragment's own border box in local coordinates when its layout
    /// extent also carries visible child overflow. Geometry and input use
    /// this box; children retain their independently positioned fragments.
    /// `None` means the border box fills `size`. This is layout ownership
    /// data, independent of decoration ink and paint clipping.
    pub principal_box_rect: Option<PhysicalRect>,

    /// Full matrix resolved against this physical fragment's actual reference
    /// box after layout; the owned authored operation list remains in style.
    pub resolved_transform: Option<crate::ResolvedFragmentTransform>,

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

    /// Layout-resolved orientation for the complete text paint stack.
    pub text_run_orientation: TextRunOrientation,

    /// Inherited style for anonymous fragments (e.g., ellipsis "…") that have
    /// no DOM node. Used by the painter to render with the correct color/font.
    pub inherited_style: Option<ComputedStyle>,

    /// Whether anonymous text is the generated marker of a block/line clamp.
    /// Paint consumes this after the anonymous fragment has lost its DOM
    /// ancestry, so it can retain the same glyph-strike policy as its line.
    pub is_line_clamp_marker: bool,

    /// Fragment-local generated-marker paint state. Column markers share one
    /// DOM pseudo node but produce one box per generated column.
    pub paint_background_color_override: Option<openui_style::Color>,

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

    /// Layout-owned viewport client and content extents. Scrollbar space is
    /// separate from authored borders and padding.
    pub viewport_scrollport: Option<ViewportScrollport>,

    /// Classic element scrollbar gutters, independent of CSS box edges.
    pub element_scrollbars: Option<crate::ElementScrollbars>,

    /// Layout-owned client, reachable content and scroll directions for the
    /// principal box. Native queries and input consume this same snapshot.
    pub scroll_area: Option<crate::ScrollArea>,

    /// Whether this fragment clips overflowing content.
    ///
    /// Set to `true` when the element's `overflow-x` or `overflow-y` is not
    /// `visible`. The paint system uses this flag to apply a clip rect before
    /// painting children.
    pub has_overflow_clip: bool,

    /// Multicol fragmentainers clip fragmented content in the block axis while
    /// still allowing inline overflow to paint into column gaps.
    pub block_axis_clip_only: bool,

    /// A fragmentainer containing an oversized monolithic line clips at its
    /// inline edges while allowing that line to overflow the block end.
    pub inline_axis_clip_only: bool,

    /// Additional block-axis ink allowance for column descendants. This is
    /// visual overflow (for example an outset box shadow), not scrollable
    /// layout overflow, so it expands the fragmentainer paint clip without
    /// changing column geometry or continuation decisions.
    pub column_block_start_ink_overflow: LayoutUnit,
    pub column_block_end_ink_overflow: LayoutUnit,

    /// Visual translation inherited from inline ancestors that were removed
    /// by block-in-inline anonymous-box reconstruction. Fragmentation uses
    /// this to slice in normal-flow coordinates while moving each slice and
    /// its clip with the interrupted inline continuation.
    pub fragmentation_visual_offset: PhysicalOffset,

    /// Source and consumed principal-box geometry for native dimension/timeline
    /// consumers. Paint and hit testing continue to use the actual local box.
    pub box_geometry: Option<FragmentBoxGeometry>,

    /// Sticky source geometry before scroll-dependent visual displacement.
    /// Timeline geometry ignores this displacement, including that of ancestors.
    pub sticky_timeline_geometry: Option<StickyTimelineGeometry>,

    /// The positioning inputs used to create an out-of-flow fragment.
    pub positioned_fragmentation: Option<PositionedFragmentationData>,

    /// Transformed containing-block ancestors elided by positioned
    /// fragmentation promotion.
    pub promoted_transform_ancestors: Vec<PromotedTransformAncestor>,

    /// Whether promoted transform replay paints this continuation's own
    /// decoration box instead of reconstructing the complete unsliced source
    /// decoration. Rotation/shear fragments use their local box; axis-aligned
    /// transforms retain the source decoration positioning area.
    pub promoted_transform_uses_fragment_decoration: bool,

    /// Resolved column geometry when this fragment is a multicol container.
    pub multicol_fragmentation: Option<MulticolFragmentationData>,

    /// Writing direction of the fragmentation context that produced this
    /// physical fragment. `None` means the fragment is not a fragmentainer or
    /// an in-flow continuation. Paint uses this to map logical block slicing
    /// to the physical X or Y axis without rediscovering layout state.
    pub fragmentation_writing_direction: Option<WritingDirectionMode>,

    /// Optional block-axis limit for this fragment's own decorations
    /// (background/border/shadow), while leaving children free to overflow.
    pub decoration_paint_block_size: Option<LayoutUnit>,

    /// The decoration limit only trims a fragmentainer-expanded block tail;
    /// preserve the normal fractional coverage of its inline edge.
    pub decoration_limit_preserves_inline_coverage: bool,

    /// Whether this structural fragment delegates its background and border
    /// to a synthetic child with the same source style. Table wrappers use
    /// this when captions sit outside the table-grid decoration box.
    pub skip_box_decoration: bool,

    /// Suppress authored corner radii for formatting-model-specific used
    /// values. Collapsed tables ignore radii on the table box and internal
    /// table boxes while preserving radii on ordinary descendants.
    pub ignore_border_radius: bool,

    /// Repaint the used border above descendants. Collapsed table structural
    /// borders participate in a grid-wide border layer above cell backgrounds.
    pub paint_border_after_children: bool,

    /// Independently sized collapsed-table border pieces painted above cell
    /// backgrounds. Empty means `border` uses the fragment's full border box.
    pub collapsed_border_segments: Vec<CollapsedBorderSegment>,
    /// Resolved full-width table paint sources, owned once by the table.
    /// Geometry clients use each cell's already-resolved Fragment::border.
    pub collapsed_table_border_data: Option<std::sync::Arc<crate::CollapsedTableBorderData>>,
    /// Actual source rows and local boundaries of this section continuation.
    /// The full resolved graph remains owned once by the enclosing table.
    pub collapsed_table_section: Option<crate::resolved_table_borders::CollapsedTableSectionData>,
    /// This grid participant's authored border is painted by the table's
    /// resolved graph, independently of its used layout border strut.
    pub collapsed_table_border_participant: bool,
    /// A cell originating in a collapsed row or spanning only collapsed
    /// columns is retained for geometry but contributes no painted subtree.
    pub is_hidden_for_paint: bool,

    /// Optional local rectangles that clip this fragment's table-structural
    /// decoration while retaining the fragment border box as the background
    /// positioning area. Row, row-group, and column backgrounds use these to
    /// exclude separated-border spacing gaps.
    pub decoration_clip_rects: Vec<PhysicalRect>,

    /// Number of table rows occupied by a laid-out table-cell fragment.
    /// Non-cell fragments and cells clamped to one available row use one.
    pub table_row_span: usize,

    /// Layout-owned table background source/target classification. Anonymous
    /// structure retains its role without gaining authored box ownership.
    pub table_part: Option<FragmentTablePart>,

    /// Column/group source inline offset relative to the first table track.
    /// This remains logical through fragment projection and source cloning.
    pub table_column_background_inline_offset: Option<LayoutUnit>,

    /// Full stitched section union in this table continuation's local physical
    /// coordinates. Final document layout owns this positioning rectangle;
    /// actual retained cells independently determine the painted area.
    pub table_column_background_positioning: Option<TableColumnBackgroundPositioning>,

    /// Shared source-space decoration geometry for a sliced continuation.
    /// `None` means this fragment owns an independent positioning area (the
    /// normal case, including `box-decoration-break:clone`).
    pub decoration_slice: Option<DecorationSlice>,

    /// Allows selected zero-height fragments to paint outlines when their
    /// formatting context keeps the outline visible.
    pub paint_zero_block_outline: bool,

    /// Internal, non-painting inline-fragmentation item representing trailing
    /// block-end border/padding. Keeping it in the break-token source list
    /// prevents decoration at an exact fragmentainer edge from being dropped
    /// after all line boxes have otherwise been consumed.
    pub is_block_end_decoration_marker: bool,

    /// Whether a trailing decoration continuation fills its fragmentainer.
    /// Authored block-end border/padding does; extra used min-block-size space
    /// retains only the unconsumed source extent.
    pub fills_fragmentainer_block_end_decoration: bool,

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
    /// Whether this box contributes geometry and input for its source element.
    ///
    /// Authored viewport boxes preserve the root element's geometry. Text,
    /// column fragments and helpers can reference the same node for paint
    /// without owning another element box.
    #[inline]
    pub fn owns_element_box(&self) -> bool {
        matches!(self.kind, FragmentKind::Box | FragmentKind::Viewport)
            && self.box_ownership == FragmentBoxOwnership::AuthoredElement
            && !self.node_id.is_none()
    }

    /// Authored boxes and explicitly retained anonymous formatting boxes both
    /// own layout geometry. Element APIs still use `owns_element_box()`.
    #[inline]
    pub fn owns_layout_box(&self) -> bool {
        self.owns_element_box()
            || matches!(
                self.anonymous_layout_box,
                Some(AnonymousLayoutBox::FieldsetContent { .. })
            )
    }

    /// Whether this box applies its owner's scroll state to its descendants.
    /// A fieldset wrapper delegates that operation to its content box.
    #[inline]
    pub fn owns_scroll_box(&self) -> bool {
        fn delegates(fragment: &Fragment, owner: NodeId) -> bool {
            matches!(fragment.anonymous_layout_box,
                Some(AnonymousLayoutBox::FieldsetContent { owner: content_owner }) if content_owner == owner)
                || fragment
                    .children
                    .iter()
                    .any(|child| delegates(child, owner))
        }
        self.owns_layout_box()
            && (self.anonymous_layout_box.is_some()
                || !self
                    .children
                    .iter()
                    .any(|child| delegates(child, self.node_id)))
    }

    #[inline]
    pub fn layout_style<'a>(&'a self, authored: &'a ComputedStyle) -> &'a ComputedStyle {
        if self.anonymous_layout_box.is_some() {
            self.inherited_style.as_ref().unwrap_or(authored)
        } else {
            authored
        }
    }

    /// Actual formatting box that owns this element's retained scroll state.
    /// Fieldset wrappers delegate to their anonymous content box; layout-only
    /// table helpers and line/column boxes cannot acquire that ownership.
    pub fn find_scroll_box(&self, node: NodeId) -> Option<&Self> {
        if self.node_id == node && self.owns_scroll_box() && self.scroll_area.is_some() {
            return Some(self);
        }
        self.children
            .iter()
            .find_map(|child| child.find_scroll_box(node))
    }

    /// Membership in the retained box tree, rather than DOM parentage. The
    /// rendered legend belongs to the fieldset wrapper, outside its content.
    pub fn contains_element_box(&self, node: NodeId) -> bool {
        (self.node_id == node && self.owns_element_box())
            || self
                .children
                .iter()
                .any(|child| child.contains_element_box(node))
    }

    /// All retained continuations participate in content membership. An
    /// authored wrapper that delegates scrolling cannot make its sibling
    /// legend part of the anonymous scroll box.
    pub fn scroll_box_contains_element(&self, owner: NodeId, target: NodeId) -> bool {
        (self.node_id == owner
            && self.owns_scroll_box()
            && self.scroll_area.is_some()
            && self.contains_element_box(target))
            || self
                .children
                .iter()
                .any(|child| child.scroll_box_contains_element(owner, target))
    }

    /// Nearest retained scroll ancestor of an authored box. Its own overflow
    /// is excluded, matching the source operation for a view timeline.
    pub fn nearest_scroll_ancestor(&self, target: NodeId) -> Option<NodeId> {
        fn walk(
            fragment: &Fragment,
            target: NodeId,
            ancestor: Option<NodeId>,
        ) -> Option<Option<NodeId>> {
            if fragment.owns_element_box() && fragment.node_id == target {
                return Some(ancestor);
            }
            let next = if fragment.owns_scroll_box()
                && fragment
                    .scroll_area
                    .is_some_and(|area| area.is_scroll_container())
            {
                Some(fragment.node_id)
            } else {
                ancestor
            };
            fragment
                .children
                .iter()
                .find_map(|child| walk(child, target, next))
        }
        walk(self, target, None).flatten()
    }

    /// Create a new box fragment.
    pub fn new_box(node_id: NodeId, size: PhysicalSize) -> Self {
        Self {
            node_id,
            kind: FragmentKind::Box,
            box_ownership: FragmentBoxOwnership::AuthoredElement,
            anonymous_layout_box: None,
            offset: PhysicalOffset::zero(),
            size,
            principal_box_rect: None,
            resolved_transform: None,
            padding: BoxStrut::zero(),
            border: BoxStrut::zero(),
            margin: BoxStrut::zero(),
            children: Vec::new(),
            shape_result: None,
            text_content: None,
            text_run_orientation: TextRunOrientation::Horizontal,
            inherited_style: None,
            is_line_clamp_marker: false,
            paint_background_color_override: None,
            baseline_offset: 0.0,
            text_combine: None,
            overflow_rect: None,
            viewport_scrollport: None,
            element_scrollbars: None,
            scroll_area: None,
            has_overflow_clip: false,
            block_axis_clip_only: false,
            inline_axis_clip_only: false,
            column_block_start_ink_overflow: LayoutUnit::zero(),
            column_block_end_ink_overflow: LayoutUnit::zero(),
            fragmentation_visual_offset: PhysicalOffset::zero(),
            box_geometry: None,
            sticky_timeline_geometry: None,
            positioned_fragmentation: None,
            promoted_transform_ancestors: Vec::new(),
            promoted_transform_uses_fragment_decoration: false,
            multicol_fragmentation: None,
            fragmentation_writing_direction: None,
            decoration_paint_block_size: None,
            decoration_limit_preserves_inline_coverage: false,
            skip_box_decoration: false,
            ignore_border_radius: false,
            paint_border_after_children: false,
            collapsed_border_segments: Vec::new(),
            collapsed_table_border_data: None,
            collapsed_table_section: None,
            collapsed_table_border_participant: false,
            is_hidden_for_paint: false,
            decoration_clip_rects: Vec::new(),
            table_row_span: 1,
            table_part: None,
            table_column_background_inline_offset: None,
            table_column_background_positioning: None,
            decoration_slice: None,
            paint_zero_block_outline: false,
            is_block_end_decoration_marker: false,
            fills_fragmentainer_block_end_decoration: false,
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
            box_ownership: FragmentBoxOwnership::LayoutHelper,
            anonymous_layout_box: None,
            offset: PhysicalOffset::zero(),
            size,
            principal_box_rect: None,
            resolved_transform: None,
            padding: BoxStrut::zero(),
            border: BoxStrut::zero(),
            margin: BoxStrut::zero(),
            children: Vec::new(),
            shape_result: Some(shape_result),
            text_content: Some(text_content),
            text_run_orientation: TextRunOrientation::Horizontal,
            inherited_style: None,
            is_line_clamp_marker: false,
            paint_background_color_override: None,
            baseline_offset: 0.0,
            text_combine: None,
            overflow_rect: None,
            viewport_scrollport: None,
            element_scrollbars: None,
            scroll_area: None,
            has_overflow_clip: false,
            block_axis_clip_only: false,
            inline_axis_clip_only: false,
            column_block_start_ink_overflow: LayoutUnit::zero(),
            column_block_end_ink_overflow: LayoutUnit::zero(),
            fragmentation_visual_offset: PhysicalOffset::zero(),
            box_geometry: None,
            sticky_timeline_geometry: None,
            positioned_fragmentation: None,
            promoted_transform_ancestors: Vec::new(),
            promoted_transform_uses_fragment_decoration: false,
            multicol_fragmentation: None,
            fragmentation_writing_direction: None,
            decoration_paint_block_size: None,
            decoration_limit_preserves_inline_coverage: false,
            skip_box_decoration: false,
            ignore_border_radius: false,
            paint_border_after_children: false,
            collapsed_border_segments: Vec::new(),
            collapsed_table_border_data: None,
            collapsed_table_section: None,
            collapsed_table_border_participant: false,
            is_hidden_for_paint: false,
            decoration_clip_rects: Vec::new(),
            table_row_span: 1,
            table_part: None,
            table_column_background_inline_offset: None,
            table_column_background_positioning: None,
            decoration_slice: None,
            paint_zero_block_outline: false,
            is_block_end_decoration_marker: false,
            fills_fragmentainer_block_end_decoration: false,
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

    /// Physical content origin, excluding actual border, padding and gutters.
    pub fn content_offset(&self) -> PhysicalOffset {
        let border = crate::ScrollArea::used_border(self);
        let padding = crate::ScrollArea::used_padding(self);
        let gutters = self
            .element_scrollbars
            .map_or(BoxStrut::zero(), |bars| bars.insets);
        PhysicalOffset::new(
            border.left + padding.left + gutters.left,
            border.top + padding.top + gutters.top,
        )
    }

    /// Physical content size. Gutters consume inner space without changing
    /// the authored border or padding or the resource's outer replaced size.
    pub fn content_size(&self) -> PhysicalSize {
        let border = crate::ScrollArea::used_border(self);
        let padding = crate::ScrollArea::used_padding(self);
        let gutters = self
            .element_scrollbars
            .map_or(BoxStrut::zero(), |bars| bars.insets);
        PhysicalSize::new(
            (self.size.width - border.inline_sum() - padding.inline_sum() - gutters.inline_sum())
                .clamp_negative_to_zero(),
            (self.size.height - border.block_sum() - padding.block_sum() - gutters.block_sum())
                .clamp_negative_to_zero(),
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

    /// The fragment's own border-box rect in local coordinates.
    #[inline]
    pub fn border_box_rect(&self) -> PhysicalRect {
        self.principal_box_rect
            .unwrap_or_else(|| PhysicalRect::new(PhysicalOffset::zero(), self.size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::Direction;

    #[test]
    fn element_box_ownership_preserves_authored_continuations_and_viewports() {
        let mut doc = openui_dom::Document::new();
        let node = doc.create_node(openui_dom::ElementTag::Div);
        let authored = Fragment::new_box(node, PhysicalSize::zero());
        assert!(authored.owns_element_box());

        let mut continuation = authored.clone();
        continuation.is_first_for_node = false;
        continuation.is_last_for_node = false;
        assert!(continuation.owns_element_box());

        let mut viewport = authored.clone();
        viewport.kind = FragmentKind::Viewport;
        assert!(viewport.owns_element_box());

        let mut helper = authored.clone();
        helper.box_ownership = FragmentBoxOwnership::LayoutHelper;
        assert!(!helper.owns_element_box());
        assert_eq!(helper.node_id, node);

        for kind in [
            FragmentKind::Text,
            FragmentKind::ColumnRule,
            FragmentKind::ColumnBox,
        ] {
            let mut nonbox = authored.clone();
            nonbox.kind = kind;
            assert!(!nonbox.owns_element_box());
        }
        assert!(!Fragment::new_box(NodeId::NONE, PhysicalSize::zero()).owns_element_box());
    }

    #[test]
    fn text_run_orientation_matrix_is_layout_authoritative() {
        for direction in [Direction::Ltr, Direction::Rtl] {
            for text_orientation in [
                TextOrientation::Mixed,
                TextOrientation::Upright,
                TextOrientation::Sideways,
            ] {
                let mut style = ComputedStyle::default();
                style.update_derived(|computed| computed.direction = direction);
                style.update_derived(|computed| computed.text_orientation = text_orientation);

                style.update_derived(|computed| computed.writing_mode = WritingMode::HorizontalTb);
                assert_eq!(
                    resolve_text_run_orientation(&style, "AHEM"),
                    TextRunOrientation::Horizontal
                );

                style.update_derived(|computed| computed.writing_mode = WritingMode::SidewaysRl);
                assert_eq!(
                    resolve_text_run_orientation(&style, "AHEM"),
                    TextRunOrientation::Clockwise
                );

                style.update_derived(|computed| computed.writing_mode = WritingMode::SidewaysLr);
                assert_eq!(
                    resolve_text_run_orientation(&style, "AHEM"),
                    TextRunOrientation::CounterClockwise
                );

                for writing_mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
                    style.update_derived(|computed| computed.writing_mode = writing_mode);
                    let expected = match text_orientation {
                        TextOrientation::Mixed => TextRunOrientation::Clockwise,
                        TextOrientation::Upright => TextRunOrientation::Upright,
                        TextOrientation::Sideways => TextRunOrientation::Clockwise,
                    };
                    assert_eq!(
                        resolve_text_run_orientation(&style, "Latin AHEM"),
                        expected,
                        "{writing_mode:?} {direction:?} {text_orientation:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn vertical_mixed_keeps_upright_and_unsplit_mixed_runs_explicit() {
        for writing_mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            let mut style = ComputedStyle::default();
            style.update_derived(|computed| computed.writing_mode = writing_mode);
            style.update_derived(|computed| computed.text_orientation = TextOrientation::Mixed);
            assert_eq!(
                resolve_text_run_orientation(&style, "文"),
                TextRunOrientation::Upright
            );
            assert_eq!(
                resolve_text_run_orientation(&style, "A文"),
                TextRunOrientation::UnresolvedMixed
            );
        }
    }
}
