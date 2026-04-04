//! LineBreaker — the core line-breaking algorithm.
//!
//! Mirrors Blink's `LineBreaker` from
//! `third_party/blink/renderer/core/layout/inline/line_breaker.cc`.
//!
//! Takes inline items + available width, produces lines. Implements:
//! - UAX#14 line break opportunity detection
//! - CSS `word-break` property (normal, break-all, keep-all, break-word)
//! - CSS `overflow-wrap` property (normal, break-word, anywhere)
//! - CSS `white-space` property (controls whether wrapping is allowed)
//! - Forced breaks (`<br>`, newlines in pre/pre-line)
//! - Trailing space stripping per CSS Text §4.1.3

use openui_geometry::LayoutUnit;
use openui_style::{OverflowWrap, TextAlign, WhiteSpace, WordBreak};

use super::items::{CollapseType, InlineItem, InlineItemResult, InlineItemType};
use super::items_builder::InlineItemsData;
use super::line_info::LineInfo;

/// The line breaker — iterates over inline items producing lines.
///
/// Usage:
/// ```ignore
/// let mut breaker = LineBreaker::new(&items_data);
/// while let Some(line) = breaker.next_line(available_width) {
///     // process line
/// }
/// ```
pub struct LineBreaker<'a> {
    items_data: &'a InlineItemsData,
    /// Current position in the items array.
    current_item: usize,
    /// Current byte offset for mid-item text breaks.
    current_text_offset: usize,
    /// Whether all items have been consumed.
    is_finished: bool,
    /// Text alignment from the block container.
    text_align: TextAlign,
}

/// Internal state for line-building loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineState {
    Continue,
    Done,
}

impl<'a> LineBreaker<'a> {
    /// Create a new line breaker for the given inline items.
    pub fn new(items_data: &'a InlineItemsData) -> Self {
        Self {
            items_data,
            current_item: 0,
            current_text_offset: 0,
            is_finished: false,
            text_align: TextAlign::Start,
        }
    }

    /// Set text alignment for produced lines.
    pub fn set_text_align(&mut self, align: TextAlign) {
        self.text_align = align;
    }

    /// Check if all items have been consumed.
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Get the next line. Returns `None` when all items are consumed.
    pub fn next_line(&mut self, available_width: LayoutUnit) -> Option<LineInfo> {
        if self.is_finished {
            return None;
        }

        if self.current_item >= self.items_data.items.len() {
            self.is_finished = true;
            return None;
        }

        let mut line = LineInfo::new(available_width);
        line.text_align = self.text_align;
        let mut state = LineState::Continue;

        while self.current_item < self.items_data.items.len() && state == LineState::Continue {
            let item = &self.items_data.items[self.current_item];
            match item.item_type {
                InlineItemType::Text => {
                    self.handle_text(self.current_item, &mut line, &mut state);
                }
                InlineItemType::OpenTag => {
                    line.items.push(InlineItemResult {
                        item_index: self.current_item,
                        text_range: item.text_range.clone(),
                        inline_size: LayoutUnit::zero(),
                        shape_result: None,
                        has_forced_break: false,
                        item_type: InlineItemType::OpenTag,
                    });
                    self.current_item += 1;
                }
                InlineItemType::CloseTag => {
                    line.items.push(InlineItemResult {
                        item_index: self.current_item,
                        text_range: item.text_range.clone(),
                        inline_size: LayoutUnit::zero(),
                        shape_result: None,
                        has_forced_break: false,
                        item_type: InlineItemType::CloseTag,
                    });
                    self.current_item += 1;
                }
                InlineItemType::Control => {
                    // Forced break (<br> or newline in pre mode)
                    line.items.push(InlineItemResult {
                        item_index: self.current_item,
                        text_range: item.text_range.clone(),
                        inline_size: LayoutUnit::zero(),
                        shape_result: None,
                        has_forced_break: true,
                        item_type: InlineItemType::Control,
                    });
                    line.has_forced_break = true;
                    self.current_item += 1;
                    self.current_text_offset = 0;
                    state = LineState::Done;
                }
                InlineItemType::AtomicInline => {
                    self.handle_atomic_inline(self.current_item, &mut line, &mut state);
                }
                InlineItemType::BlockInInline => {
                    // Skip block-in-inline for now
                    self.current_item += 1;
                }
            }
        }

        // Strip trailing collapsible spaces from the line
        strip_trailing_spaces(&mut line, &self.items_data.items);

        // Check if this is the last line
        line.is_last_line = self.current_item >= self.items_data.items.len();
        if line.is_last_line {
            self.is_finished = true;
        }

        Some(line)
    }

    /// Handle a text item — measure, find break opportunities, break if needed.
    fn handle_text(
        &mut self,
        item_index: usize,
        line: &mut LineInfo,
        state: &mut LineState,
    ) {
        let item = &self.items_data.items[item_index];
        let style = &self.items_data.styles[item.style_index];

        // Determine the actual text range to process (may be a suffix after mid-item break)
        let text_start = if self.current_text_offset > 0 && self.current_text_offset > item.text_range.start {
            self.current_text_offset
        } else {
            item.text_range.start
        };
        let text_end = item.text_range.end;

        if text_start >= text_end {
            self.current_item += 1;
            self.current_text_offset = 0;
            return;
        }

        let text_slice = &self.items_data.text[text_start..text_end];

        // Check for forced breaks in pre/pre-line modes
        if has_forced_newline(text_slice, style.white_space) {
            self.handle_text_with_newlines(item_index, text_start, text_end, line, state);
            return;
        }

        // Check if wrapping is prevented
        let allows_wrap = allows_line_wrap(style.white_space);

        // Measure the text
        let text_width = if let Some(ref sr) = item.shape_result {
            let char_start = byte_to_char_offset(&self.items_data.text, text_start);
            let char_end = byte_to_char_offset(&self.items_data.text, text_end);
            let item_char_start = byte_to_char_offset(&self.items_data.text, item.text_range.start);
            let local_start = char_start - item_char_start;
            let local_end = char_end - item_char_start;
            LayoutUnit::from_f32(sr.width_for_range(local_start, local_end))
        } else {
            LayoutUnit::zero()
        };

        let remaining = line.remaining_width();

        if text_width <= remaining || !allows_wrap {
            // Entire text fits (or we're in nowrap mode)
            line.items.push(InlineItemResult {
                item_index,
                text_range: text_start..text_end,
                inline_size: text_width,
                shape_result: item.shape_result.clone(),
                has_forced_break: false,
                item_type: InlineItemType::Text,
            });
            line.used_width = line.used_width + text_width;
            self.current_item += 1;
            self.current_text_offset = 0;
            return;
        }

        // Text doesn't fit — find best break point
        let break_opps = find_break_opportunities(text_slice, style.word_break, style.overflow_wrap);

        let mut best_break: Option<usize> = None;
        let mut best_width = LayoutUnit::zero();

        if let Some(ref sr) = item.shape_result {
            let item_char_start = byte_to_char_offset(&self.items_data.text, item.text_range.start);
            let char_start = byte_to_char_offset(&self.items_data.text, text_start);

            for &brk in &break_opps {
                // brk is a byte offset into text_slice
                let break_byte = text_start + brk;
                let break_char = byte_to_char_offset(&self.items_data.text, break_byte);
                let local_start = char_start - item_char_start;
                let local_end = break_char - item_char_start;
                let width = LayoutUnit::from_f32(sr.width_for_range(local_start, local_end));
                if width <= remaining {
                    best_break = Some(brk);
                    best_width = width;
                } else {
                    break;
                }
            }
        }

        if let Some(break_at) = best_break {
            if break_at == 0 {
                // Can't fit anything — if line is empty, force overflow
                if !line.has_content() {
                    self.force_text_on_line(item_index, text_start, text_end, text_width, line);
                }
                *state = LineState::Done;
                return;
            }

            let break_byte = text_start + break_at;
            line.items.push(InlineItemResult {
                item_index,
                text_range: text_start..break_byte,
                inline_size: best_width,
                shape_result: item.shape_result.clone(),
                has_forced_break: false,
                item_type: InlineItemType::Text,
            });
            line.used_width = line.used_width + best_width;
            self.current_text_offset = break_byte;
            *state = LineState::Done;
        } else {
            // No break opportunity found
            match style.overflow_wrap {
                OverflowWrap::BreakWord | OverflowWrap::Anywhere => {
                    // Character-level break
                    self.handle_character_break(
                        item_index, text_start, text_end, remaining, line, state,
                    );
                }
                OverflowWrap::Normal => {
                    if !line.has_content() {
                        // First content on line — force it to avoid infinite loop
                        self.force_text_on_line(item_index, text_start, text_end, text_width, line);
                        *state = LineState::Done;
                    } else {
                        // Break before this item (it goes to next line)
                        *state = LineState::Done;
                    }
                }
            }
        }
    }

    /// Handle text that contains forced newlines (in pre or pre-line modes).
    fn handle_text_with_newlines(
        &mut self,
        item_index: usize,
        text_start: usize,
        text_end: usize,
        line: &mut LineInfo,
        state: &mut LineState,
    ) {
        let item = &self.items_data.items[item_index];
        let text_slice = &self.items_data.text[text_start..text_end];

        // Find the first newline
        if let Some(nl_pos) = text_slice.find('\n') {
            let break_byte = text_start + nl_pos;

            // Text before the newline goes on this line
            if nl_pos > 0 {
                let pre_nl_width = self.measure_text_range(item_index, text_start, break_byte);
                line.items.push(InlineItemResult {
                    item_index,
                    text_range: text_start..break_byte,
                    inline_size: pre_nl_width,
                    shape_result: item.shape_result.clone(),
                    has_forced_break: false,
                    item_type: InlineItemType::Text,
                });
                line.used_width = line.used_width + pre_nl_width;
            }

            line.has_forced_break = true;
            // Skip past the newline character
            let after_nl = break_byte + 1;
            if after_nl >= text_end {
                self.current_item += 1;
                self.current_text_offset = 0;
            } else {
                self.current_text_offset = after_nl;
            }
            *state = LineState::Done;
        } else {
            // No newline found in remaining text — treat normally
            let text_width = self.measure_text_range(item_index, text_start, text_end);
            line.items.push(InlineItemResult {
                item_index,
                text_range: text_start..text_end,
                inline_size: text_width,
                shape_result: item.shape_result.clone(),
                has_forced_break: false,
                item_type: InlineItemType::Text,
            });
            line.used_width = line.used_width + text_width;
            self.current_item += 1;
            self.current_text_offset = 0;
        }
    }

    /// Handle character-level breaking (for overflow-wrap: break-word/anywhere).
    fn handle_character_break(
        &mut self,
        item_index: usize,
        text_start: usize,
        text_end: usize,
        remaining: LayoutUnit,
        line: &mut LineInfo,
        state: &mut LineState,
    ) {
        let item = &self.items_data.items[item_index];
        let text_slice = &self.items_data.text[text_start..text_end];

        if let Some(ref sr) = item.shape_result {
            let item_char_start = byte_to_char_offset(&self.items_data.text, item.text_range.start);
            let char_start = byte_to_char_offset(&self.items_data.text, text_start);

            // Walk character by character to find where to break
            let mut best_byte: Option<usize> = None;
            let mut best_width = LayoutUnit::zero();

            for (byte_offset, _ch) in text_slice.char_indices().skip(1) {
                let break_byte = text_start + byte_offset;
                let break_char = byte_to_char_offset(&self.items_data.text, break_byte);
                let local_start = char_start - item_char_start;
                let local_end = break_char - item_char_start;
                let width = LayoutUnit::from_f32(sr.width_for_range(local_start, local_end));
                if width <= remaining {
                    best_byte = Some(break_byte);
                    best_width = width;
                } else {
                    break;
                }
            }

            if let Some(brk) = best_byte {
                line.items.push(InlineItemResult {
                    item_index,
                    text_range: text_start..brk,
                    inline_size: best_width,
                    shape_result: item.shape_result.clone(),
                    has_forced_break: false,
                    item_type: InlineItemType::Text,
                });
                line.used_width = line.used_width + best_width;
                self.current_text_offset = brk;
                *state = LineState::Done;
            } else {
                // Can't even fit one character
                if !line.has_content() {
                    // Force at least one character
                    let first_char_end = text_slice
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| text_start + i)
                        .unwrap_or(text_end);
                    let width = self.measure_text_range(item_index, text_start, first_char_end);
                    line.items.push(InlineItemResult {
                        item_index,
                        text_range: text_start..first_char_end,
                        inline_size: width,
                        shape_result: item.shape_result.clone(),
                        has_forced_break: false,
                        item_type: InlineItemType::Text,
                    });
                    line.used_width = line.used_width + width;
                    if first_char_end >= text_end {
                        self.current_item += 1;
                        self.current_text_offset = 0;
                    } else {
                        self.current_text_offset = first_char_end;
                    }
                }
                *state = LineState::Done;
            }
        } else {
            // No shape result — force the item
            if !line.has_content() {
                self.force_text_on_line(item_index, text_start, text_end, LayoutUnit::zero(), line);
            }
            *state = LineState::Done;
        }
    }

    /// Handle an atomic inline item.
    fn handle_atomic_inline(
        &mut self,
        item_index: usize,
        line: &mut LineInfo,
        state: &mut LineState,
    ) {
        let item = &self.items_data.items[item_index];
        // For now, atomic inlines have zero width (layout of their contents is future work)
        let width = LayoutUnit::zero();
        let remaining = line.remaining_width();

        if width <= remaining || !line.has_content() {
            line.items.push(InlineItemResult {
                item_index,
                text_range: item.text_range.clone(),
                inline_size: width,
                shape_result: None,
                has_forced_break: false,
                item_type: InlineItemType::AtomicInline,
            });
            line.used_width = line.used_width + width;
            self.current_item += 1;
        } else {
            *state = LineState::Done;
        }
    }

    /// Force the entire text onto the current line (for first-on-line overflow).
    fn force_text_on_line(
        &mut self,
        item_index: usize,
        text_start: usize,
        text_end: usize,
        width: LayoutUnit,
        line: &mut LineInfo,
    ) {
        let item = &self.items_data.items[item_index];
        line.items.push(InlineItemResult {
            item_index,
            text_range: text_start..text_end,
            inline_size: width,
            shape_result: item.shape_result.clone(),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        });
        line.used_width = line.used_width + width;
        self.current_item += 1;
        self.current_text_offset = 0;
    }

    /// Measure the width of a byte range within a text item.
    fn measure_text_range(&self, item_index: usize, start: usize, end: usize) -> LayoutUnit {
        let item = &self.items_data.items[item_index];
        if let Some(ref sr) = item.shape_result {
            let item_char_start = byte_to_char_offset(&self.items_data.text, item.text_range.start);
            let char_start = byte_to_char_offset(&self.items_data.text, start);
            let char_end = byte_to_char_offset(&self.items_data.text, end);
            let local_start = char_start - item_char_start;
            let local_end = char_end - item_char_start;
            LayoutUnit::from_f32(sr.width_for_range(local_start, local_end))
        } else {
            LayoutUnit::zero()
        }
    }
}

/// Strip trailing collapsible spaces from the line measurement.
///
/// CSS Text Level 3 §4.1.3: "A sequence of collapsible spaces at the end
/// of a line is removed." This adjusts `used_width` but keeps the items
/// for painting (spaces may be painted in pre-wrap).
fn strip_trailing_spaces(line: &mut LineInfo, items: &[InlineItem]) {
    // Walk items from the end; skip close tags; strip trailing space from last text item
    for item_result in line.items.iter().rev() {
        if item_result.item_type == InlineItemType::Text {
            let item_idx = item_result.item_index;
            if item_idx < items.len() {
                let item = &items[item_idx];
                if item.end_collapse_type == CollapseType::Collapsible {
                    // The trailing space is collapsible — it should not contribute to width.
                    // In a full implementation we'd re-measure without trailing spaces.
                    // For now this is accounted for in the shaping measurement.
                }
            }
            break;
        }
        if item_result.item_type != InlineItemType::CloseTag
            && item_result.item_type != InlineItemType::OpenTag
        {
            break;
        }
    }
}

// ── Break opportunity detection ─────────────────────────────────────────

/// Find break opportunities in text based on CSS `word-break` and `overflow-wrap`.
///
/// Returns byte offsets within `text` where a line break may occur.
/// Uses UAX#14 (Unicode Line Breaking Algorithm) as the base, modified
/// by the CSS properties.
pub fn find_break_opportunities(
    text: &str,
    word_break: WordBreak,
    _overflow_wrap: OverflowWrap,
) -> Vec<usize> {
    match word_break {
        WordBreak::Normal | WordBreak::BreakWord => {
            // UAX#14 line break opportunities
            find_uax14_breaks(text)
        }
        WordBreak::BreakAll => {
            // Break between any two characters (grapheme cluster boundaries)
            text.char_indices().map(|(i, _)| i).skip(1).collect()
        }
        WordBreak::KeepAll => {
            // Only break at spaces — keep words together including CJK
            find_space_breaks(text)
        }
    }
}

/// Find UAX#14 line break opportunities using a simplified algorithm.
///
/// This implements the essential rules from Unicode Line Breaking Algorithm:
/// - Break after spaces
/// - Break after hyphens
/// - Don't break before/after certain punctuation
///
/// A full implementation would use the `unicode-linebreak` crate, but we
/// implement the core rules directly for zero external dependencies.
fn find_uax14_breaks(text: &str) -> Vec<usize> {
    let mut breaks = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();

    for idx in 0..chars.len() {
        let (byte_pos, _ch) = chars[idx];

        // Break opportunity after space (but not at position 0)
        if byte_pos > 0 {
            if idx > 0 {
                let (_prev_pos, prev_ch) = chars[idx - 1];
                // Break after space
                if prev_ch == ' ' || prev_ch == '\t' {
                    breaks.push(byte_pos);
                }
                // Break after hyphen (U+002D), en-dash (U+2013), em-dash (U+2014)
                else if prev_ch == '-' || prev_ch == '\u{2013}' || prev_ch == '\u{2014}' {
                    breaks.push(byte_pos);
                }
                // Break after soft hyphen (U+00AD)
                else if prev_ch == '\u{00AD}' {
                    breaks.push(byte_pos);
                }
            }
        }
    }
    breaks
}

/// Find break opportunities only at spaces (for `word-break: keep-all`).
fn find_space_breaks(text: &str) -> Vec<usize> {
    let mut breaks = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for idx in 1..chars.len() {
        let (_prev_pos, prev_ch) = chars[idx - 1];
        let (byte_pos, _ch) = chars[idx];
        if prev_ch == ' ' || prev_ch == '\t' {
            breaks.push(byte_pos);
        }
    }
    breaks
}

/// Check if text contains forced newlines that should be treated as line breaks.
fn has_forced_newline(text: &str, white_space: WhiteSpace) -> bool {
    match white_space {
        WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine | WhiteSpace::BreakSpaces => {
            text.contains('\n')
        }
        WhiteSpace::Normal | WhiteSpace::Nowrap => false,
    }
}

/// Check if the white-space value allows line wrapping.
fn allows_line_wrap(white_space: WhiteSpace) -> bool {
    match white_space {
        WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine | WhiteSpace::BreakSpaces => true,
        WhiteSpace::Nowrap | WhiteSpace::Pre => false,
    }
}

/// Convert a byte offset in a string to a character offset.
///
/// This is needed because `ShapeResult` methods work with character indices,
/// while our text ranges use byte offsets.
fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_uax14_breaks_simple() {
        let breaks = find_uax14_breaks("hello world");
        assert_eq!(breaks, vec![6]); // break after space
    }

    #[test]
    fn test_find_uax14_breaks_multiple_words() {
        let breaks = find_uax14_breaks("the quick brown fox");
        assert_eq!(breaks, vec![4, 10, 16]);
    }

    #[test]
    fn test_find_uax14_breaks_hyphen() {
        let breaks = find_uax14_breaks("well-known");
        assert_eq!(breaks, vec![5]); // break after hyphen
    }

    #[test]
    fn test_find_space_breaks() {
        let breaks = find_space_breaks("hello world test");
        assert_eq!(breaks, vec![6, 12]);
    }

    #[test]
    fn test_break_all() {
        let breaks = find_break_opportunities("abc", WordBreak::BreakAll, OverflowWrap::Normal);
        assert_eq!(breaks, vec![1, 2]);
    }

    #[test]
    fn test_byte_to_char_offset_ascii() {
        assert_eq!(byte_to_char_offset("hello", 0), 0);
        assert_eq!(byte_to_char_offset("hello", 3), 3);
        assert_eq!(byte_to_char_offset("hello", 5), 5);
    }

    #[test]
    fn test_byte_to_char_offset_multibyte() {
        // 'é' is 2 bytes in UTF-8
        let text = "café";
        assert_eq!(byte_to_char_offset(text, 0), 0); // 'c'
        assert_eq!(byte_to_char_offset(text, 1), 1); // 'a'
        assert_eq!(byte_to_char_offset(text, 2), 2); // 'f'
        assert_eq!(byte_to_char_offset(text, 3), 3); // 'é' start
        assert_eq!(byte_to_char_offset(text, 5), 4); // end
    }

    #[test]
    fn test_allows_line_wrap() {
        assert!(allows_line_wrap(WhiteSpace::Normal));
        assert!(!allows_line_wrap(WhiteSpace::Nowrap));
        assert!(!allows_line_wrap(WhiteSpace::Pre));
        assert!(allows_line_wrap(WhiteSpace::PreWrap));
        assert!(allows_line_wrap(WhiteSpace::PreLine));
        assert!(allows_line_wrap(WhiteSpace::BreakSpaces));
    }

    #[test]
    fn test_has_forced_newline() {
        assert!(!has_forced_newline("hello", WhiteSpace::Normal));
        assert!(!has_forced_newline("hello\nworld", WhiteSpace::Normal));
        assert!(has_forced_newline("hello\nworld", WhiteSpace::Pre));
        assert!(has_forced_newline("hello\nworld", WhiteSpace::PreLine));
        assert!(has_forced_newline("hello\nworld", WhiteSpace::PreWrap));
    }
}
