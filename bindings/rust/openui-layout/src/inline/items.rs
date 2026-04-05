//! InlineItem types — the flat representation of inline content.
//!
//! Mirrors Blink's `InlineItem` from
//! `third_party/blink/renderer/core/layout/inline/inline_item.h`.
//!
//! The inline formatting context flattens the DOM tree into a linear sequence
//! of InlineItems. Each item represents a piece of text, an inline element
//! boundary (open/close tag), a forced break, or an atomic inline.

use openui_dom::NodeId;
use openui_text::ShapeResult;
use std::ops::Range;
use std::sync::Arc;

/// Type of inline item (matches Blink's `InlineItem::InlineItemType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineItemType {
    /// Text content from a text node.
    Text,
    /// Start of an inline element (`<span>`, `<b>`, etc.).
    OpenTag,
    /// End of an inline element (`</span>`, `</b>`, etc.).
    CloseTag,
    /// Replaced element or inline-block (`<img>`, `display: inline-block`).
    AtomicInline,
    /// Forced break or soft break hint (`<br>`, `<wbr>`).
    Control,
    /// Block-level child inside an inline context.
    BlockInInline,
}

/// White-space collapse state for a character or item boundary.
///
/// Tracks whether trailing whitespace is collapsible, which is needed
/// for stripping trailing spaces from lines per CSS Text §4.1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapseType {
    /// Not collapsible (normal content character).
    NotCollapsible,
    /// Collapsible but not yet collapsed (space that may be removed).
    Collapsible,
    /// Already collapsed (space was merged with adjacent whitespace).
    Collapsed,
}

/// A single inline item in the flat representation.
///
/// Mirrors Blink's `InlineItem`. The inline items builder collects these
/// by walking the DOM tree in document order, producing open/close tag
/// items at element boundaries and text items for text nodes.
#[derive(Clone, Debug)]
pub struct InlineItem {
    /// What kind of inline item this is.
    pub item_type: InlineItemType,

    /// Byte range into the collected text string.
    pub text_range: Range<usize>,

    /// DOM node that produced this item.
    pub node_id: NodeId,

    /// Shaped text result (only for Text items, populated after shaping).
    pub shape_result: Option<Arc<ShapeResult>>,

    /// Index into the styles array for the style applied to this item.
    pub style_index: usize,

    /// White-space collapse state at the end of this item.
    pub end_collapse_type: CollapseType,

    /// Whether the collapsible space at the end contains a newline.
    pub is_end_collapsible_newline: bool,

    /// BiDi embedding level (0 = LTR default).
    pub bidi_level: u8,

    /// Intrinsic inline size for AtomicInline items (computed from children).
    /// Used when CSS `width` is `auto` to provide shrink-to-fit width.
    pub intrinsic_inline_size: Option<f32>,
}

/// Result of measuring/breaking an InlineItem for a specific line.
///
/// When the line breaker processes items, it produces one `InlineItemResult`
/// per item (or partial item) that fits on the current line.
#[derive(Clone, Debug)]
pub struct InlineItemResult {
    /// Index into the original items array.
    pub item_index: usize,

    /// The byte range of text placed on this line (subset of the item's text_range).
    pub text_range: Range<usize>,

    /// Measured inline size (advance width) for this portion.
    pub inline_size: openui_geometry::LayoutUnit,

    /// Shape result for the portion on this line.
    pub shape_result: Option<Arc<ShapeResult>>,

    /// Whether this item triggered a forced line break.
    pub has_forced_break: bool,

    /// The inline item type (copied for convenience).
    pub item_type: InlineItemType,
}
