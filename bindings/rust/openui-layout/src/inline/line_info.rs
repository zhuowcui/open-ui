//! LineInfo — information about a single laid-out line.
//!
//! Mirrors Blink's `InlineLayoutResult` / `LineInfo` from
//! `third_party/blink/renderer/core/layout/inline/line_info.h`.

use openui_geometry::LayoutUnit;
use openui_style::TextAlign;

use super::items::InlineItemResult;

/// Information about a single line produced by the line breaker.
///
/// Contains the items placed on this line, measured widths, and metadata
/// about forced breaks and alignment.
#[derive(Clone, Debug)]
pub struct LineInfo {
    /// Items (or partial items) placed on this line.
    pub items: Vec<InlineItemResult>,

    /// Available inline size (width) for this line.
    pub available_width: LayoutUnit,

    /// Total used inline size (sum of item widths) on this line.
    pub used_width: LayoutUnit,

    /// Whether this line ends with a forced break (`<br>` or `\n` in pre).
    pub has_forced_break: bool,

    /// Whether this is the last line of the inline formatting context.
    pub is_last_line: bool,

    /// Text alignment for this line (from the block container's style).
    pub text_align: TextAlign,

    /// Whether this line has been truncated with an ellipsis.
    pub has_ellipsis: bool,

    /// Whether the ellipsis should appear at the start (left) of the line (RTL).
    pub ellipsis_at_start: bool,

    /// Width of trailing spaces that "hang" past the line box end.
    ///
    /// CSS Text Level 3 §4.2: preserved trailing spaces hang when wrapping.
    /// This value is subtracted from `used_width` for overflow/alignment
    /// purposes but the spaces are still rendered.
    ///
    /// For collapsed white-space modes (normal, nowrap, pre-line) the spaces
    /// are stripped entirely (removed from `used_width`) and `hang_width` is 0.
    /// For `pre-wrap` trailing spaces conditionally or unconditionally hang,
    /// tracked here. For `break-spaces` nothing hangs (spaces cause breaks).
    pub hang_width: LayoutUnit,
}

impl LineInfo {
    /// Create a new empty line with the given available width.
    pub fn new(available_width: LayoutUnit) -> Self {
        Self {
            items: Vec::new(),
            available_width,
            used_width: LayoutUnit::zero(),
            has_forced_break: false,
            is_last_line: false,
            text_align: TextAlign::Start,
            has_ellipsis: false,
            ellipsis_at_start: false,
            hang_width: LayoutUnit::zero(),
        }
    }

    /// Remaining width on this line.
    pub fn remaining_width(&self) -> LayoutUnit {
        self.available_width - self.used_width
    }

    /// Whether this line has any content items (text or atomic).
    pub fn has_content(&self) -> bool {
        self.items.iter().any(|item| {
            matches!(
                item.item_type,
                super::items::InlineItemType::Text | super::items::InlineItemType::AtomicInline
            )
        })
    }
}
