//! LineBreaker — the core line-breaking algorithm.
//!
//! Mirrors Blink's `LineBreaker` from
//! `third_party/blink/renderer/core/layout/inline/line_breaker.cc`.
//!
//! Takes inline items + available width, produces lines. Implements:
//! - UAX#14 line break opportunity detection
//! - CSS `word-break` property (normal, break-all, keep-all, break-word)
//! - CSS `overflow-wrap` property (normal, break-word, anywhere)
//! - CSS `line-break` property (auto, loose, normal, strict, anywhere)
//! - CSS `white-space` property (controls whether wrapping is allowed)
//! - Forced breaks (`<br>`, newlines in pre/pre-line)
//! - Trailing space stripping per CSS Text §4.1.3

use openui_geometry::{LayoutUnit, LengthType};
use openui_style::{BoxSizing, ComputedStyle, LineBreak, OverflowWrap, TextAlign, WhiteSpace, WordBreak};
use unicode_segmentation::UnicodeSegmentation;

use super::items::{CollapseType, InlineItem, InlineItemResult, InlineItemType};
use super::items_builder::InlineItemsData;
use super::line_info::LineInfo;
use crate::length_resolver::resolve_margin_or_padding;

/// The line breaker — iterates over inline items producing lines.
///
/// Usage:
/// ```ignore
/// let mut breaker = LineBreaker::new(&items_data, available_width);
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
    /// Containing block's content-box width for percentage resolution.
    /// Percentages on inline margin/border/padding resolve against this,
    /// not the per-line available width (CSS 2.2 §10.3.3).
    containing_block_width: LayoutUnit,
}

/// Internal state for line-building loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineState {
    Continue,
    Done,
}

impl<'a> LineBreaker<'a> {
    /// Create a new line breaker for the given inline items.
    pub fn new(items_data: &'a InlineItemsData, containing_block_width: LayoutUnit) -> Self {
        Self {
            items_data,
            current_item: 0,
            current_text_offset: 0,
            is_finished: false,
            text_align: TextAlign::Start,
            containing_block_width,
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
                    let style = &self.items_data.styles[item.style_index];
                    let pct_base = self.containing_block_width;
                    let mbp = if style.direction == openui_style::Direction::Rtl {
                        resolve_margin_or_padding(&style.margin_right, pct_base)
                            + LayoutUnit::from_i32(style.effective_border_right())
                            + resolve_margin_or_padding(&style.padding_right, pct_base)
                    } else {
                        resolve_margin_or_padding(&style.margin_left, pct_base)
                            + LayoutUnit::from_i32(style.effective_border_left())
                            + resolve_margin_or_padding(&style.padding_left, pct_base)
                    };
                    line.items.push(InlineItemResult {
                        item_index: self.current_item,
                        text_range: item.text_range.clone(),
                        inline_size: mbp,
                        shape_result: None,
                        has_forced_break: false,
                        item_type: InlineItemType::OpenTag,
                    });
                    line.used_width = line.used_width + mbp;
                    self.current_item += 1;
                }
                InlineItemType::CloseTag => {
                    let style = &self.items_data.styles[item.style_index];
                    let pct_base = self.containing_block_width;
                    let mbp = if style.direction == openui_style::Direction::Rtl {
                        resolve_margin_or_padding(&style.padding_left, pct_base)
                            + LayoutUnit::from_i32(style.effective_border_left())
                            + resolve_margin_or_padding(&style.margin_left, pct_base)
                    } else {
                        resolve_margin_or_padding(&style.padding_right, pct_base)
                            + LayoutUnit::from_i32(style.effective_border_right())
                            + resolve_margin_or_padding(&style.margin_right, pct_base)
                    };
                    line.items.push(InlineItemResult {
                        item_index: self.current_item,
                        text_range: item.text_range.clone(),
                        inline_size: mbp,
                        shape_result: None,
                        has_forced_break: false,
                        item_type: InlineItemType::CloseTag,
                    });
                    line.used_width = line.used_width + mbp;
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
                    // Block-in-inline creates an anonymous block box — handled
                    // by the block layout algorithm, not the line breaker.
                    self.current_item += 1;
                }
            }
        }

        // Strip trailing collapsible spaces from the line
        strip_trailing_spaces(&mut line, &self.items_data.items, &self.items_data.text, &self.items_data.styles);

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
        let break_opps = find_break_opportunities(text_slice, style.word_break, style.overflow_wrap, style.line_break);

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
    ///
    /// For `white-space: pre` (no wrapping), text before the newline is placed
    /// unconditionally. For wrappable modes (`pre-wrap`, `pre-line`,
    /// `break-spaces`), the segment before the newline is processed through
    /// normal wrapping logic before the forced break.
    fn handle_text_with_newlines(
        &mut self,
        item_index: usize,
        text_start: usize,
        text_end: usize,
        line: &mut LineInfo,
        state: &mut LineState,
    ) {
        let item = &self.items_data.items[item_index];
        let style = &self.items_data.styles[item.style_index];
        let text_slice = &self.items_data.text[text_start..text_end];
        let wrappable = allows_line_wrap(style.white_space);

        // Find the first newline
        if let Some(nl_pos) = text_slice.find('\n') {
            let break_byte = text_start + nl_pos;

            // Text before the newline
            if nl_pos > 0 {
                if wrappable {
                    // Wrappable mode: the pre-newline segment may need soft
                    // wrapping. Measure and find break opportunities just like
                    // handle_text does for normal text.
                    let seg_start = text_start;
                    let seg_end = break_byte;
                    let seg_width = self.measure_text_range(item_index, seg_start, seg_end);
                    let remaining = line.remaining_width();

                    if seg_width <= remaining {
                        // Fits — place it all
                        line.items.push(InlineItemResult {
                            item_index,
                            text_range: seg_start..seg_end,
                            inline_size: seg_width,
                            shape_result: item.shape_result.clone(),
                            has_forced_break: false,
                            item_type: InlineItemType::Text,
                        });
                        line.used_width = line.used_width + seg_width;
                    } else {
                        // Doesn't fit — find a break opportunity within the
                        // pre-newline segment and break the line there. The
                        // newline (and remainder) will be handled on the next
                        // line via the normal loop.
                        let seg_text = &self.items_data.text[seg_start..seg_end];
                        let break_opps = find_break_opportunities(
                            seg_text, style.word_break, style.overflow_wrap, style.line_break,
                        );

                        let mut best_break: Option<usize> = None;
                        let mut best_width = LayoutUnit::zero();

                        if let Some(ref sr) = item.shape_result {
                            let item_char_start = byte_to_char_offset(
                                &self.items_data.text, item.text_range.start,
                            );
                            let char_start = byte_to_char_offset(
                                &self.items_data.text, seg_start,
                            );
                            for &brk in &break_opps {
                                let brk_byte = seg_start + brk;
                                let brk_char = byte_to_char_offset(
                                    &self.items_data.text, brk_byte,
                                );
                                let local_start = char_start - item_char_start;
                                let local_end = brk_char - item_char_start;
                                let width = LayoutUnit::from_f32(
                                    sr.width_for_range(local_start, local_end),
                                );
                                if width <= remaining {
                                    best_break = Some(brk);
                                    best_width = width;
                                } else {
                                    break;
                                }
                            }
                        }

                        if let Some(brk_at) = best_break {
                            if brk_at > 0 {
                                let brk_byte = seg_start + brk_at;
                                line.items.push(InlineItemResult {
                                    item_index,
                                    text_range: seg_start..brk_byte,
                                    inline_size: best_width,
                                    shape_result: item.shape_result.clone(),
                                    has_forced_break: false,
                                    item_type: InlineItemType::Text,
                                });
                                line.used_width = line.used_width + best_width;
                                self.current_text_offset = brk_byte;
                                *state = LineState::Done;
                                return;
                            }
                        }

                        // No break opportunity found or break at 0: check
                        // overflow-wrap before forcing content on the line.
                        match style.overflow_wrap {
                            OverflowWrap::BreakWord | OverflowWrap::Anywhere => {
                                // Character-level break for the pre-newline segment.
                                self.handle_character_break(
                                    item_index, seg_start, seg_end, remaining, line, state,
                                );
                                if *state == LineState::Done {
                                    return;
                                }
                            }
                            OverflowWrap::Normal => {
                                if !line.has_content() {
                                    self.force_text_on_line(
                                        item_index, seg_start, seg_end, seg_width, line,
                                    );
                                    // Also consume the newline character so the next
                                    // call doesn't see it at offset 0 and emit an
                                    // extra blank forced-break line.
                                    line.has_forced_break = true;
                                    let after_nl = break_byte + 1;
                                    if after_nl >= text_end {
                                        // force_text_on_line already advanced
                                        // current_item and reset offset — correct.
                                    } else {
                                        self.current_text_offset = after_nl;
                                        self.current_item -= 1; // undo force_text_on_line's increment
                                    }
                                }
                            }
                        }
                        *state = LineState::Done;
                        return;
                    }
                } else {
                    // Non-wrappable (pre): place unconditionally
                    let pre_nl_width = self.measure_text_range(
                        item_index, text_start, break_byte,
                    );
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
            if wrappable {
                let remaining = line.remaining_width();
                if text_width > remaining {
                    // Doesn't fit — process through normal wrapping.
                    // Re-enter handle_text which won't see a newline
                    // since there's none in this segment.
                    // Temporarily adjust text bounds and delegate.
                    let seg_text = &self.items_data.text[text_start..text_end];
                    let break_opps = find_break_opportunities(
                        seg_text, style.word_break, style.overflow_wrap, style.line_break,
                    );
                    let mut best_break: Option<usize> = None;
                    let mut best_width = LayoutUnit::zero();

                    if let Some(ref sr) = item.shape_result {
                        let item_char_start = byte_to_char_offset(
                            &self.items_data.text, item.text_range.start,
                        );
                        let char_start = byte_to_char_offset(
                            &self.items_data.text, text_start,
                        );
                        for &brk in &break_opps {
                            let brk_byte = text_start + brk;
                            let brk_char = byte_to_char_offset(
                                &self.items_data.text, brk_byte,
                            );
                            let local_start = char_start - item_char_start;
                            let local_end = brk_char - item_char_start;
                            let width = LayoutUnit::from_f32(
                                sr.width_for_range(local_start, local_end),
                            );
                            if width <= remaining {
                                best_break = Some(brk);
                                best_width = width;
                            } else {
                                break;
                            }
                        }
                    }

                    if let Some(brk_at) = best_break {
                        if brk_at > 0 {
                            let brk_byte = text_start + brk_at;
                            line.items.push(InlineItemResult {
                                item_index,
                                text_range: text_start..brk_byte,
                                inline_size: best_width,
                                shape_result: item.shape_result.clone(),
                                has_forced_break: false,
                                item_type: InlineItemType::Text,
                            });
                            line.used_width = line.used_width + best_width;
                            self.current_text_offset = brk_byte;
                            *state = LineState::Done;
                            return;
                        }
                    }

                    match style.overflow_wrap {
                        OverflowWrap::BreakWord | OverflowWrap::Anywhere => {
                            let remaining = line.remaining_width();
                            self.handle_character_break(
                                item_index, text_start, text_end, remaining, line, state,
                            );
                        }
                        OverflowWrap::Normal => {
                            if !line.has_content() {
                                self.force_text_on_line(
                                    item_index, text_start, text_end, text_width, line,
                                );
                            }
                            *state = LineState::Done;
                        }
                    }
                    return;
                }
            }

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

            // Walk grapheme cluster boundaries to find where to break.
            // Using grapheme clusters instead of raw char_indices() prevents
            // splitting inside ZWJ emoji, Arabic joining sequences, and
            // ligature clusters. Also check safe_to_break_before from the
            // shape result to avoid breaking inside shaped glyph clusters.
            let mut best_byte: Option<usize> = None;
            let mut best_width = LayoutUnit::zero();

            for (byte_offset, _grapheme) in text_slice.grapheme_indices(true).skip(1) {
                let break_byte = text_start + byte_offset;
                let break_char = byte_to_char_offset(&self.items_data.text, break_byte);
                let local_break = break_char - item_char_start;
                // Only break where the shaper says it's safe.
                if !sr.safe_to_break_before(local_break) {
                    continue;
                }
                let local_start = char_start - item_char_start;
                let local_end = local_break;
                let width = LayoutUnit::from_f32(sr.width_for_range(local_start, local_end));
                if width <= remaining {
                    best_byte = Some(break_byte);
                    best_width = width;
                } else {
                    break;
                }
            }

            // If no safe-to-break position was found, determine whether the
            // text contains complex-script regions by checking whether any
            // grapheme boundary in the range is marked unsafe-to-break.
            if best_byte.is_none() {
                let has_unsafe_positions =
                    text_slice.grapheme_indices(true).skip(1).any(|(byte_offset, _)| {
                        let break_byte = text_start + byte_offset;
                        let break_char =
                            byte_to_char_offset(&self.items_data.text, break_byte);
                        let local_break = break_char - item_char_start;
                        !sr.safe_to_break_before(local_break)
                    });

                if !has_unsafe_positions {
                    // Simple script (all positions are safe): fall back to
                    // any grapheme boundary that fits.
                    for (byte_offset, _grapheme) in text_slice.grapheme_indices(true).skip(1) {
                        let break_byte = text_start + byte_offset;
                        let break_char =
                            byte_to_char_offset(&self.items_data.text, break_byte);
                        let local_start = char_start - item_char_start;
                        let local_end = break_char - item_char_start;
                        let width =
                            LayoutUnit::from_f32(sr.width_for_range(local_start, local_end));
                        if width <= remaining {
                            best_byte = Some(break_byte);
                            best_width = width;
                        } else {
                            break;
                        }
                    }
                }
                // Complex script: best_byte stays None — we fall through to
                // the overflow path below, which forces the first safe cluster
                // onto the line rather than breaking mid-joining-sequence.
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
                // Can't fit any safe break.
                if !line.has_content() {
                    // Force text up to the first safe-to-break position to
                    // prevent breaking inside complex-script joining
                    // sequences. If no safe position exists at all within
                    // the text range, force the entire range — the whole
                    // word is one indivisible cluster. This may overflow the
                    // line, matching Blink's behavior of preferring overflow
                    // over incorrect shaping.
                    let force_end = text_slice
                        .grapheme_indices(true)
                        .skip(1)
                        .find_map(|(byte_offset, _)| {
                            let break_byte = text_start + byte_offset;
                            let break_char =
                                byte_to_char_offset(&self.items_data.text, break_byte);
                            let local_break = break_char - item_char_start;
                            if sr.safe_to_break_before(local_break) {
                                Some(break_byte)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(text_end);
                    let width = self.measure_text_range(item_index, text_start, force_end);
                    line.items.push(InlineItemResult {
                        item_index,
                        text_range: text_start..force_end,
                        inline_size: width,
                        shape_result: item.shape_result.clone(),
                        has_forced_break: false,
                        item_type: InlineItemType::Text,
                    });
                    line.used_width = line.used_width + width;
                    if force_end >= text_end {
                        self.current_item += 1;
                        self.current_text_offset = 0;
                    } else {
                        self.current_text_offset = force_end;
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
        let style = &self.items_data.styles[item.style_index];
        // Resolve the atomic inline's width from its computed style.
        // For elements with an explicit CSS `width`, use that value.
        // For `auto` or percentage widths without a definite containing block,
        // fall back to zero (full box layout integration is required for
        // intrinsic sizing of inline-block content).
        let width = resolve_atomic_inline_width(style, self.containing_block_width, item.intrinsic_inline_size);
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

/// Strip or hang trailing whitespace from the line measurement.
///
/// CSS Text Level 3 §4.1.3: "A sequence of collapsible spaces at the end
/// of a line is removed."
/// CSS Text Level 3 §4.2: Preserved trailing spaces in `pre-wrap` mode
/// *hang* — they are subtracted from `used_width` but NOT from the painted
/// text range, and their width is recorded in `line.hang_width`.
///
/// Also handles U+3000 IDEOGRAPHIC SPACE which hangs like regular spaces
/// (Blink: `InlineLayoutAlgorithm::HangTrailingSpaces`).
///
/// Uses the actual text on the current line (the item_result's text_range)
/// rather than the full item's collapse metadata, so that split items are
/// handled correctly.
fn strip_trailing_spaces(line: &mut LineInfo, items: &[InlineItem], text: &str, styles: &[ComputedStyle]) {
    // Walk items from the end; skip close/open tags; find last text item index.
    let target_idx = {
        let mut idx = None;
        for (i, item_result) in line.items.iter().enumerate().rev() {
            if item_result.item_type == InlineItemType::Text {
                idx = Some(i);
                break;
            }
            if item_result.item_type != InlineItemType::CloseTag
                && item_result.item_type != InlineItemType::OpenTag
            {
                break;
            }
        }
        idx
    };

    let target_idx = match target_idx {
        Some(i) => i,
        None => return,
    };

    let item_idx = line.items[target_idx].item_index;
    if item_idx >= items.len() {
        return;
    }

    let item = &items[item_idx];

    let ws = if item.style_index < styles.len() {
        styles[item.style_index].white_space
    } else {
        WhiteSpace::Normal
    };

    // CSS Text §3: break-spaces preserves all spaces, including trailing.
    // No stripping or hanging at all.
    if ws == WhiteSpace::BreakSpaces {
        return;
    }

    /// Predicate matching characters that participate in trailing-space
    /// hanging: ASCII space, tab, and ideographic space (U+3000).
    fn is_hangable_space(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\u{3000}'
    }

    if let Some(ref sr) = item.shape_result {
        let line_text_start = line.items[target_idx].text_range.start;
        let line_text_end = line.items[target_idx].text_range.end;
        if line_text_start < line_text_end {
            let at_item_end = line_text_end == item.text_range.end;
            if at_item_end && item.end_collapse_type == CollapseType::Collapsible {
                // Trailing spaces are collapsible — measure width of
                // ALL trailing whitespace from the line portion's end.
                let char_count = sr.num_characters;
                let line_text = &text[line_text_start..line_text_end];
                let trimmed = line_text.trim_end_matches(is_hangable_space);
                let num_trimmed_chars = line_text[trimmed.len()..].chars().count();
                if char_count > 0 && num_trimmed_chars > 0 {
                    let space_width = sr.width_for_range(char_count - num_trimmed_chars, char_count);
                    let space_lu = LayoutUnit::from_f32(space_width);
                    line.used_width = line.used_width - space_lu;

                    if ws == WhiteSpace::PreWrap {
                        // Pre-wrap: spaces "hang" — subtract from alignment
                        // width but do NOT trim text_range (spaces still
                        // render past the line box). Record hang_width so
                        // alignment uses the trimmed width.
                        line.hang_width = line.hang_width + space_lu;
                    } else {
                        // Normal/nowrap/pre-line: trim text_range so
                        // decorations don't extend into stripped space.
                        let new_end = line_text_start + trimmed.len();
                        line.items[target_idx].text_range = line_text_start..new_end;
                    }
                }
            } else if !at_item_end {
                // Mid-item split: check white-space mode before stripping.

                // For `pre`: all whitespace is non-collapsible — skip stripping entirely.
                if ws == WhiteSpace::Pre {
                    return;
                }

                // Mid-item split: check if the portion ends with trailing
                // whitespace by inspecting the actual text.
                let line_text = &text[line_text_start..line_text_end];
                let trimmed = line_text.trim_end_matches(is_hangable_space);
                let num_trimmed_chars = line_text[trimmed.len()..].chars().count();
                if num_trimmed_chars > 0 {
                    let item_text = &text[item.text_range.clone()];
                    let offset_in_item = line_text_end - item.text_range.start;
                    let local_end = item_text[..offset_in_item].chars().count();
                    if local_end >= num_trimmed_chars && local_end <= sr.num_characters {
                        let space_width = sr.width_for_range(local_end - num_trimmed_chars, local_end);
                        let space_lu = LayoutUnit::from_f32(space_width);
                        line.used_width = line.used_width - space_lu;

                        if ws == WhiteSpace::PreWrap {
                            // Pre-wrap: hang the trailing spaces.
                            line.hang_width = line.hang_width + space_lu;
                        } else {
                            // Normal/nowrap/pre-line: trim text_range.
                            let new_end = line_text_start + trimmed.len();
                            line.items[target_idx].text_range = line_text_start..new_end;
                        }
                    }
                }
            }
        }
    }
}

// ── Break opportunity detection ─────────────────────────────────────────

/// Find break opportunities in text based on CSS `word-break`, `overflow-wrap`,
/// and `line-break` properties.
///
/// Returns byte offsets within `text` where a line break may occur.
/// Uses UAX#14 (Unicode Line Breaking Algorithm) as the base, modified
/// by the `word-break` and `line-break` properties.
///
/// The `line-break` property (CSS Text Module Level 3 §5.2) controls CJK-specific
/// line breaking strictness:
/// - `auto`/`normal`: standard UAX#14 behavior
/// - `strict`: prohibits breaks before small kana, iteration marks, prolonged
///   sound mark, and certain CJK closing punctuation
/// - `loose`: allows additional breaks around CJK comma/period characters
/// - `anywhere`: allows breaks between every typographic character unit
///
/// The `overflow-wrap` parameter is accepted for API consistency — character-level
/// breaking for `overflow-wrap: break-word` is handled separately by
/// `LineBreaker::handle_character_break`.
pub fn find_break_opportunities(
    text: &str,
    word_break: WordBreak,
    overflow_wrap: OverflowWrap,
    line_break: LineBreak,
) -> Vec<usize> {
    // line-break: anywhere overrides everything — break at every grapheme cluster.
    if line_break == LineBreak::Anywhere {
        return text.grapheme_indices(true).map(|(i, _)| i).skip(1).collect();
    }

    let base_breaks = match word_break {
        WordBreak::Normal => {
            // UAX#14 line break opportunities
            find_uax14_breaks(text)
        }
        WordBreak::BreakWord => {
            // word-break: break-word is a legacy alias for overflow-wrap: break-word
            // with word-break: normal. Use UAX#14 breaks as the base; the caller
            // handles character-level fallback via overflow-wrap.
            let _ = overflow_wrap;
            find_uax14_breaks(text)
        }
        WordBreak::BreakAll => {
            // Break between grapheme clusters (not raw Unicode characters).
            text.grapheme_indices(true).map(|(i, _)| i).skip(1).collect()
        }
        WordBreak::KeepAll => {
            // Start from normal UAX#14 breaks, but suppress CJK-specific
            // break opportunities (breaks between two CJK characters).
            // Non-CJK breaks (hyphens, after-punctuation, etc.) are kept.
            let all_breaks = find_uax14_breaks(text);
            all_breaks
                .into_iter()
                .filter(|&pos| !is_cjk_break(text, pos))
                .collect()
        }
    };

    // Apply line-break strictness filtering to the base break set.
    match line_break {
        LineBreak::Strict => {
            apply_strict_line_break(text, base_breaks)
        }
        LineBreak::Loose => {
            apply_loose_line_break(text, base_breaks)
        }
        // Auto and Normal use standard UAX#14 behavior unchanged.
        LineBreak::Auto | LineBreak::Normal | LineBreak::Anywhere => base_breaks,
    }
}

/// Find UAX#14 line break opportunities.
///
/// Uses the `unicode-linebreak` crate for full Unicode Line Breaking Algorithm
/// coverage, including CJK ideographic break opportunities that the previous
/// manual implementation could not detect.
fn find_uax14_breaks(text: &str) -> Vec<usize> {
    use unicode_linebreak::{BreakOpportunity, linebreaks};
    let mut breaks = Vec::new();
    for (byte_offset, break_opp) in linebreaks(text) {
        match break_opp {
            BreakOpportunity::Mandatory | BreakOpportunity::Allowed => {
                // Don't include break at position 0 or at the very end (after last char)
                if byte_offset > 0 && byte_offset < text.len() {
                    breaks.push(byte_offset);
                }
            }
        }
    }
    breaks
}

/// Find break opportunities only at spaces (used internally for tests).
#[allow(dead_code)]
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

/// Check if a break opportunity at `byte_pos` is a CJK-specific break
/// (i.e., both the character before and after the break are CJK).
///
/// CSS Text Level 3: `word-break: keep-all` suppresses only soft wrap
/// opportunities between CJK characters; non-CJK breaks are preserved.
fn is_cjk_break(text: &str, byte_pos: usize) -> bool {
    let before = text[..byte_pos].chars().next_back();
    let after = text[byte_pos..].chars().next();
    match (before, after) {
        (Some(b), Some(a)) => is_cjk_character(b) && is_cjk_character(a),
        _ => false,
    }
}

/// Check if a character belongs to a CJK script block.
fn is_cjk_character(ch: char) -> bool {
    let c = ch as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&c)
    // CJK Extension A
    || (0x3400..=0x4DBF).contains(&c)
    // CJK Extension B
    || (0x20000..=0x2A6DF).contains(&c)
    // CJK Extension C-G
    || (0x2A700..=0x2CEAF).contains(&c)
    // CJK Compatibility Ideographs
    || (0xF900..=0xFAFF).contains(&c)
    // Hangul Syllables
    || (0xAC00..=0xD7AF).contains(&c)
    // Hangul Jamo
    || (0x1100..=0x11FF).contains(&c)
    // Hangul Compatibility Jamo
    || (0x3130..=0x318F).contains(&c)
    // Hiragana
    || (0x3040..=0x309F).contains(&c)
    // Katakana
    || (0x30A0..=0x30FF).contains(&c)
    // Katakana Phonetic Extensions
    || (0x31F0..=0x31FF).contains(&c)
    // CJK Radicals Supplement
    || (0x2E80..=0x2EFF).contains(&c)
    // Kangxi Radicals
    || (0x2F00..=0x2FDF).contains(&c)
    // CJK Symbols and Punctuation
    || (0x3000..=0x303F).contains(&c)
    // Bopomofo
    || (0x3100..=0x312F).contains(&c)
    // Yi Syllables
    || (0xA000..=0xA48F).contains(&c)
}

// ── line-break property: CJK strictness classification ─────────────────
//
// CSS Text Module Level 3 §5.2 and UAX#14 tailoring.
// These functions implement the strictness levels that Blink passes to
// ICU via the `@lb=` locale keyword.

/// Characters that `line-break: strict` prohibits breaking before.
///
/// Strict mode prevents line breaks before:
/// - Small kana (hiragana U+3041–U+3094 small variants, katakana U+30A1–U+30F6 small variants)
/// - Prolonged sound mark (U+30FC ー)
/// - Iteration marks (U+3005 々 ideographic, U+303B 〻 vertical)
/// - CJK closing punctuation and certain delimiters that should not begin a line
///
/// Reference: UAX#14 §6 Tailorable Line Breaking (strict context).
fn is_cjk_strict_no_break_before(ch: char) -> bool {
    matches!(ch,
        // Small hiragana (U+3041 ぁ, U+3043 ぃ, U+3045 ぅ, U+3047 ぇ, U+3049 ぉ,
        //                 U+3063 っ, U+3083 ゃ, U+3085 ゅ, U+3087 ょ, U+308E ゎ)
        'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ' |
        // Small katakana (U+30A1 ァ, U+30A3 ィ, U+30A5 ゥ, U+30A7 ェ, U+30A9 ォ,
        //                 U+30C3 ッ, U+30E3 ャ, U+30E5 ュ, U+30E7 ョ, U+30EE ヮ,
        //                 U+30F5 ヵ, U+30F6 ヶ)
        'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' | 'ッ' | 'ャ' | 'ュ' | 'ョ' | 'ヮ' | 'ヵ' | 'ヶ' |
        // Prolonged sound mark (U+30FC ー)
        'ー' |
        // Iteration marks (U+3005 々 ideographic, U+303B 〻 vertical)
        '々' | '〻' |
        // CJK closing punctuation that strict mode prohibits breaking before:
        // U+3002 。 ideographic full stop
        // U+3001 、 ideographic comma
        // U+FF09 ） fullwidth right parenthesis
        // U+3009 〉 right angle bracket
        // U+300B 》 right double angle bracket
        // U+300D 」 right corner bracket
        // U+300F 』 right white corner bracket
        // U+3011 】 right black lenticular bracket
        // U+3015 〕 right tortoise shell bracket
        // U+3017 〗 right white lenticular bracket
        // U+3019 〙 right white tortoise shell bracket
        // U+301B 〛 right white square bracket
        '。' | '、' | '）' | '〉' | '》' | '」' | '』' | '】' | '〕' | '〗' | '〙' | '〛'
    )
}

/// Characters that `line-break: loose` allows additional breaks around.
///
/// Loose mode permits line breaks before Japanese/CJK comma and period characters
/// in positions where normal/strict mode would prohibit them. This results in
/// more aggressive wrapping suitable for narrow columns.
///
/// Reference: UAX#14 §6 Tailorable Line Breaking (loose context).
fn is_cjk_loose_break_before(ch: char) -> bool {
    matches!(ch,
        // U+3001 、 ideographic comma
        // U+3002 。 ideographic full stop
        // U+FF0C ， fullwidth comma
        // U+FF0E ． fullwidth full stop
        '、' | '。' | '，' | '．'
    )
}

/// Apply `line-break: strict` filtering to a set of break opportunities.
///
/// Removes break positions where the character after the break is one of the
/// characters that strict mode prohibits breaking before.
fn apply_strict_line_break(text: &str, breaks: Vec<usize>) -> Vec<usize> {
    breaks
        .into_iter()
        .filter(|&pos| {
            // Look at the character after the break position.
            if let Some(after) = text[pos..].chars().next() {
                !is_cjk_strict_no_break_before(after)
            } else {
                true
            }
        })
        .collect()
}

/// Apply `line-break: loose` filtering to a set of break opportunities.
///
/// Adds break opportunities before CJK comma/period characters that normal
/// mode does not provide. The base UAX#14 breaks are preserved; additional
/// breaks are inserted at character boundaries preceding loose-extra characters.
fn apply_loose_line_break(text: &str, mut breaks: Vec<usize>) -> Vec<usize> {
    // Scan for positions before loose-break-before characters that are not
    // already in the break set.
    let mut extra: Vec<usize> = Vec::new();
    for (byte_pos, ch) in text.char_indices() {
        if byte_pos > 0 && is_cjk_loose_break_before(ch) {
            if !breaks.contains(&byte_pos) {
                extra.push(byte_pos);
            }
        }
    }
    breaks.extend(extra);
    breaks.sort_unstable();
    breaks.dedup();
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

/// Compute the horizontal border+padding for an element's style.
///
/// Used to convert content-box widths to border-box widths for atomic inlines.
fn compute_border_padding_inline(
    style: &ComputedStyle,
    containing_block_width: LayoutUnit,
) -> LayoutUnit {
    let border_left = LayoutUnit::from_i32(style.effective_border_left());
    let border_right = LayoutUnit::from_i32(style.effective_border_right());
    let pad_left = resolve_margin_or_padding(&style.padding_left, containing_block_width);
    let pad_right = resolve_margin_or_padding(&style.padding_right, containing_block_width);
    border_left + border_right + pad_left + pad_right
}

/// Resolve the width of an atomic inline element from its CSS `width` property.
///
/// For `Fixed`: use the specified pixel value.
/// For `Percent`: resolve against the containing block width.
/// For `Auto`: use intrinsic size if available, else min-width if specified, otherwise zero.
///
/// The returned value is always a **border-box** width: for `content-box` sizing
/// the element's own border+padding is added; for `border-box` the CSS width
/// already includes them.
fn resolve_atomic_inline_width(
    style: &ComputedStyle,
    containing_block_width: LayoutUnit,
    intrinsic_inline_size: Option<f32>,
) -> LayoutUnit {
    let border_padding = compute_border_padding_inline(style, containing_block_width);

    let base = match style.width.length_type() {
        LengthType::Fixed => {
            let css_w = LayoutUnit::from_f32(style.width.value());
            if style.box_sizing == BoxSizing::ContentBox {
                css_w + border_padding
            } else {
                css_w
            }
        }
        LengthType::Percent => {
            if containing_block_width > LayoutUnit::zero() {
                let css_w = LayoutUnit::from_f32(
                    style.width.value() / 100.0 * containing_block_width.to_f32(),
                );
                if style.box_sizing == BoxSizing::ContentBox {
                    css_w + border_padding
                } else {
                    css_w
                }
            } else {
                LayoutUnit::zero()
            }
        }
        // Auto: use intrinsic size (shrink-to-fit), then min-width as floor, then zero.
        // Intrinsic size is content-box, so always add border+padding.
        _ => {
            let intrinsic = intrinsic_inline_size
                .map(|v| LayoutUnit::from_f32(v) + border_padding)
                .unwrap_or(border_padding);

            // Apply min-width as a floor.
            let min_w = match style.min_width.length_type() {
                LengthType::Fixed => {
                    let mw = LayoutUnit::from_f32(style.min_width.value());
                    if style.box_sizing == BoxSizing::ContentBox {
                        mw + border_padding
                    } else {
                        mw
                    }
                }
                LengthType::Percent => {
                    if containing_block_width > LayoutUnit::zero() {
                        let mw = LayoutUnit::from_f32(
                            style.min_width.value() / 100.0 * containing_block_width.to_f32(),
                        );
                        if style.box_sizing == BoxSizing::ContentBox {
                            mw + border_padding
                        } else {
                            mw
                        }
                    } else {
                        LayoutUnit::zero()
                    }
                }
                _ => LayoutUnit::zero(),
            };

            // Shrink-to-fit: min(max(min_content, available), max_content).
            // Since we only have one intrinsic measure, use it clamped to
            // [min_width, containing_block_width].
            let result = if intrinsic > LayoutUnit::zero() {
                let capped = if containing_block_width > LayoutUnit::zero() {
                    intrinsic.min_of(containing_block_width)
                } else {
                    intrinsic
                };
                capped.max_of(min_w)
            } else {
                min_w
            };
            result
        }
    };

    // Clamp to max-width if specified.
    match style.max_width.length_type() {
        LengthType::Fixed => {
            let max = LayoutUnit::from_f32(style.max_width.value());
            let max = if style.box_sizing == BoxSizing::ContentBox {
                max + border_padding
            } else {
                max
            };
            if base > max { max } else { base }
        }
        LengthType::Percent => {
            if containing_block_width > LayoutUnit::zero() {
                let max = LayoutUnit::from_f32(
                    style.max_width.value() / 100.0 * containing_block_width.to_f32(),
                );
                let max = if style.box_sizing == BoxSizing::ContentBox {
                    max + border_padding
                } else {
                    max
                };
                if base > max { max } else { base }
            } else {
                base
            }
        }
        _ => base,
    }
}

/// Convert a byte offset in a string to a character offset.
///
/// This is needed because `ShapeResult` methods work with character indices,
/// while our text ranges use byte offsets.
pub fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
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
        let breaks = find_break_opportunities("abc", WordBreak::BreakAll, OverflowWrap::Normal, LineBreak::Auto);
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

    #[test]
    fn strip_trailing_spaces_trims_text_range_at_item_end() {
        // When stripping trailing space at the end of an item (at_item_end case),
        // text_range should be trimmed so decorations don't extend into the space.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..6,
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::Collapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..6, // at_item_end = true
            inline_size: LayoutUnit::from_f32(sr_arc.width),
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = LayoutUnit::from_f32(50.0);
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], text, &[ComputedStyle::default()]);

        // text_range should now exclude the trailing space: 0..5 ("hello")
        assert_eq!(
            line.items[0].text_range,
            0..5,
            "text_range should be trimmed to exclude trailing space"
        );
    }

    #[test]
    fn strip_trailing_spaces_trims_text_range_mid_item() {
        // When stripping trailing space from a mid-item split (!at_item_end case),
        // text_range should also be trimmed.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello world ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..12, // full item: "hello world "
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::Collapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        // Line portion is only "hello " (0..6), a mid-item split
        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..6, // at_item_end = false (6 != 12)
            inline_size: LayoutUnit::from_f32(40.0),
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = LayoutUnit::from_f32(40.0);
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], text, &[ComputedStyle::default()]);

        // text_range should be trimmed to exclude the trailing space: 0..5 ("hello")
        assert_eq!(
            line.items[0].text_range,
            0..5,
            "text_range should be trimmed to exclude trailing space in mid-item split"
        );
    }

    #[test]
    fn strip_trailing_spaces_multi_space_at_item_end() {
        // "hello   " (3 trailing spaces) — all 3 space widths must be subtracted.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello   ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        // Measure width of all 3 trailing spaces.
        let total_chars = sr_arc.num_characters; // 8
        let three_space_width = sr_arc.width_for_range(total_chars - 3, total_chars);
        let initial_width = sr_arc.width;

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..8,
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::Collapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..8,
            inline_size: LayoutUnit::from_f32(initial_width),
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = LayoutUnit::from_f32(initial_width);
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], text, &[ComputedStyle::default()]);

        assert_eq!(
            line.items[0].text_range,
            0..5,
            "text_range should trim all 3 trailing spaces"
        );
        let expected_width = LayoutUnit::from_f32(initial_width) - LayoutUnit::from_f32(three_space_width);
        assert_eq!(
            line.used_width, expected_width,
            "used_width should subtract width of all 3 trailing spaces"
        );
    }

    #[test]
    fn strip_trailing_spaces_single_space_at_item_end() {
        // Regression: "hello " (1 trailing space) — exactly 1 space width subtracted.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let total_chars = sr_arc.num_characters; // 6
        let one_space_width = sr_arc.width_for_range(total_chars - 1, total_chars);
        let initial_width = sr_arc.width;

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..6,
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::Collapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..6,
            inline_size: LayoutUnit::from_f32(initial_width),
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = LayoutUnit::from_f32(initial_width);
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], text, &[ComputedStyle::default()]);

        assert_eq!(
            line.items[0].text_range,
            0..5,
            "text_range should trim the single trailing space"
        );
        let expected_width = LayoutUnit::from_f32(initial_width) - LayoutUnit::from_f32(one_space_width);
        assert_eq!(
            line.used_width, expected_width,
            "used_width should subtract width of 1 trailing space"
        );
    }

    // ── Issue 6: keep-all allows hyphen breaks in Latin text ─────────

    #[test]
    fn keep_all_allows_hyphen_breaks() {
        // word-break: keep-all should allow breaks after hyphens in Latin text.
        // Only CJK soft wrap opportunities should be suppressed.
        let breaks = find_break_opportunities("well-known", WordBreak::KeepAll, OverflowWrap::Normal, LineBreak::Auto);
        assert!(
            breaks.contains(&5),
            "keep-all should allow break after hyphen in Latin text, got: {:?}",
            breaks,
        );
    }

    #[test]
    fn keep_all_suppresses_cjk_breaks() {
        // keep-all should suppress breaks between CJK characters.
        // "漢字" = two CJK ideographs (U+6F22, U+5B57)
        let normal_breaks = find_break_opportunities("漢字", WordBreak::Normal, OverflowWrap::Normal, LineBreak::Auto);
        let keepall_breaks = find_break_opportunities("漢字", WordBreak::KeepAll, OverflowWrap::Normal, LineBreak::Auto);

        // Normal should allow a break between the two CJK characters.
        // KeepAll should suppress it.
        assert!(
            keepall_breaks.len() < normal_breaks.len() || keepall_breaks.is_empty(),
            "keep-all should suppress CJK break opportunities: normal={:?}, keepall={:?}",
            normal_breaks,
            keepall_breaks,
        );
    }

    #[test]
    fn keep_all_allows_space_breaks() {
        // keep-all should still allow breaks at spaces.
        let breaks = find_break_opportunities("hello world", WordBreak::KeepAll, OverflowWrap::Normal, LineBreak::Auto);
        assert!(
            breaks.contains(&6),
            "keep-all should allow break after space, got: {:?}",
            breaks,
        );
    }

    #[test]
    fn is_cjk_character_detection() {
        // CJK Unified Ideographs
        assert!(is_cjk_character('漢')); // U+6F22
        assert!(is_cjk_character('字')); // U+5B57
        // Hiragana
        assert!(is_cjk_character('あ')); // U+3042
        // Katakana
        assert!(is_cjk_character('ア')); // U+30A2
        // Hangul
        assert!(is_cjk_character('한')); // U+D55C
        // Latin should NOT be CJK
        assert!(!is_cjk_character('A'));
        assert!(!is_cjk_character('-'));
        assert!(!is_cjk_character(' '));
    }

    // ── SP11 Round 11 Issue 6: pre-wrap wraps long text before newline ──

    #[test]
    fn pre_wrap_wraps_before_newline() {
        // In pre-wrap mode, text before a newline should be wrapped at
        // soft break opportunities if it overflows the available width.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        // "hello world\nmore" — in a narrow container, "hello world" should
        // wrap at the space, not be placed unconditionally on one line.
        let text = "hello world\nmore";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let mut style = ComputedStyle::default();
        style.white_space = WhiteSpace::PreWrap;
        let style_index = 0;

        let items_data = InlineItemsData {
            text: text.to_string(),
            items: vec![InlineItem {
                item_type: InlineItemType::Text,
                text_range: 0..text.len(),
                node_id: NodeId::NONE,
                shape_result: Some(sr_arc.clone()),
                style_index,
                end_collapse_type: super::super::items::CollapseType::NotCollapsible,
                is_end_collapsible_newline: false,
                bidi_level: 0,
            intrinsic_inline_size: None,
            }],
            styles: vec![style],
        };

        // Use a very narrow width — narrower than "hello world" but
        // wider than "hello " so we can break at the space.
        let hello_width = sr_arc.width_for_range(0, 6); // "hello " = 6 chars
        let narrow_width = LayoutUnit::from_f32(hello_width + 1.0);

        let mut breaker = LineBreaker::new(&items_data, narrow_width);
        let line1 = breaker.next_line(narrow_width);
        assert!(line1.is_some(), "Should produce at least one line");
        let line1 = line1.unwrap();

        // The first line should NOT contain all text up to the newline;
        // it should break at the space.
        let line1_end = line1.items.last()
            .map(|i| i.text_range.end)
            .unwrap_or(0);
        assert!(
            line1_end < 11, // 11 = offset of '\n'
            "Pre-wrap should soft-wrap long text before the newline; line ended at byte {}, expected < 11",
            line1_end,
        );
    }

    // ── SP11 Round 13 Issue 5: pre mode preserves trailing spaces ────

    #[test]
    fn strip_trailing_spaces_pre_preserves_spaces_mid_item() {
        // In white-space: pre, trailing spaces in a mid-item split
        // must be preserved — stripping should be skipped entirely.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello world ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let mut pre_style = ComputedStyle::default();
        pre_style.white_space = WhiteSpace::Pre;

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..12,
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        // Mid-item split: line portion is "hello " (0..6)
        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..6, // at_item_end = false (6 != 12)
            inline_size: LayoutUnit::from_f32(40.0),
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        let original_width = LayoutUnit::from_f32(40.0);
        line.used_width = original_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], text, &[pre_style]);

        // For pre mode, text_range and used_width must be unchanged.
        assert_eq!(
            line.items[0].text_range,
            0..6,
            "pre mode should preserve trailing spaces in text_range"
        );
        assert_eq!(
            line.used_width, original_width,
            "pre mode should not subtract space width from used_width"
        );
    }

    #[test]
    fn strip_trailing_spaces_pre_wrap_hangs_spaces() {
        // In white-space: pre-wrap, trailing spaces should "hang":
        // width is subtracted (for alignment) but text_range is NOT trimmed.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello world ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let mut pre_wrap_style = ComputedStyle::default();
        pre_wrap_style.white_space = WhiteSpace::PreWrap;

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..12,
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        // Mid-item split: line portion is "hello " (0..6)
        let initial_width = LayoutUnit::from_f32(40.0);
        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..6,
            inline_size: initial_width,
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], text, &[pre_wrap_style]);

        // For pre-wrap, text_range should NOT be trimmed (spaces still render / "hang").
        assert_eq!(
            line.items[0].text_range,
            0..6,
            "pre-wrap should keep text_range unchanged (spaces hang)"
        );
        // But used_width should be reduced (for alignment purposes).
        assert!(
            line.used_width < initial_width,
            "pre-wrap should subtract space width from used_width for alignment"
        );
    }

    // ── SP11 Round 13 Issue 6: overflow-wrap: break-word in pre-wrap ─

    #[test]
    fn pre_wrap_overflow_wrap_break_word_breaks_long_word() {
        // In pre-wrap with overflow-wrap: break-word, a long word before a
        // newline that doesn't fit should be broken at character boundaries
        // instead of being forced onto the line.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "abcdefghijklmnop\nmore";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let mut style = ComputedStyle::default();
        style.white_space = WhiteSpace::PreWrap;
        style.overflow_wrap = OverflowWrap::BreakWord;

        let items_data = InlineItemsData {
            text: text.to_string(),
            items: vec![InlineItem {
                item_type: InlineItemType::Text,
                text_range: 0..text.len(),
                node_id: NodeId::NONE,
                shape_result: Some(sr_arc.clone()),
                style_index: 0,
                end_collapse_type: super::super::items::CollapseType::NotCollapsible,
                is_end_collapsible_newline: false,
                bidi_level: 0,
            intrinsic_inline_size: None,
            }],
            styles: vec![style],
        };

        // Very narrow width — should only fit ~5 characters.
        let five_char_width = sr_arc.width_for_range(0, 5);
        let narrow_width = LayoutUnit::from_f32(five_char_width + 1.0);

        let mut breaker = LineBreaker::new(&items_data, narrow_width);
        let line1 = breaker.next_line(narrow_width);
        assert!(line1.is_some(), "Should produce at least one line");
        let line1 = line1.unwrap();

        let line1_end = line1.items.last()
            .map(|i| i.text_range.end)
            .unwrap_or(0);

        // With overflow-wrap: break-word, the line should break within the
        // long word, NOT force the entire word "abcdefghijklmnop" on one line.
        assert!(
            line1_end < 16,
            "overflow-wrap: break-word should break the long word; line ended at byte {}, expected < 16",
            line1_end,
        );
        assert!(
            line1_end > 0,
            "Line should contain at least some characters"
        );
    }

    // ── SP11 Round 14 Issue 2: grapheme cluster aware break-all ──────

    #[test]
    fn break_all_respects_grapheme_clusters() {
        // A ZWJ emoji sequence like 👨‍👩‍👧 (family) is a single grapheme cluster
        // composed of multiple Unicode code points joined by U+200D (ZWJ).
        // BreakAll should NOT produce break opportunities inside the cluster.
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // 👨‍👩‍👧
        let text = format!("a{}b", family);
        let breaks = find_break_opportunities(&text, WordBreak::BreakAll, OverflowWrap::Normal, LineBreak::Auto);

        // The emoji occupies multiple bytes. Break opportunities should only
        // be at grapheme boundaries: after 'a' and after the emoji, NOT inside it.
        let a_end = 1; // 'a' is 1 byte
        let emoji_end = 1 + family.len(); // byte offset after the emoji

        // There should be break opportunities at grapheme boundaries only.
        for &brk in &breaks {
            assert!(
                brk == a_end || brk == emoji_end,
                "Break at byte {} is inside a grapheme cluster; expected only at {} or {}",
                brk, a_end, emoji_end,
            );
        }
        // There should be at least one break (after 'a').
        assert!(!breaks.is_empty(), "BreakAll should still produce some break opportunities");
    }

    // ── SP11 Round 15 Issue 4: pre-wrap at_item_end preserves text_range ──

    #[test]
    fn strip_trailing_spaces_prewrap_at_item_end_preserves_text_range() {
        // In white-space: pre-wrap, trailing spaces at item end should
        // subtract width (for alignment) but NOT trim text_range
        // (spaces still render — they "hang" past the line box).
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello   ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let mut prewrap_style = ComputedStyle::default();
        prewrap_style.white_space = WhiteSpace::PreWrap;

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::Collapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };
        let items = vec![item];

        let original_width = LayoutUnit::from_f32(sr_arc.width());

        // at_item_end case: line text_range.end == item.text_range.end
        let mut line = LineInfo::new(LayoutUnit::from_i32(1000));
        line.items.push(InlineItemResult {
            item_index: 0,
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            inline_size: original_width,
            shape_result: Some(sr_arc.clone()),
            has_forced_break: false,
        });
        line.used_width = original_width;

        strip_trailing_spaces(&mut line, &items, text, &[prewrap_style]);

        // Width should be reduced (spaces subtracted for alignment).
        assert!(
            line.used_width < original_width,
            "pre-wrap should subtract trailing space width; used_width={:?}, original={:?}",
            line.used_width, original_width,
        );

        // But text_range should be PRESERVED (not trimmed) — spaces hang.
        assert_eq!(
            line.items[0].text_range.end, text.len(),
            "pre-wrap at_item_end should preserve text_range; got {}..{}, expected 0..{}",
            line.items[0].text_range.start, line.items[0].text_range.end, text.len(),
        );
    }

    #[test]
    fn strip_trailing_spaces_normal_at_item_end_trims_text_range() {
        // In white-space: normal, trailing spaces at item end should
        // both subtract width AND trim text_range.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "hello   ";
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let style = ComputedStyle::default(); // white_space: Normal

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::Collapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };
        let items = vec![item];

        let original_width = LayoutUnit::from_f32(sr_arc.width());

        let mut line = LineInfo::new(LayoutUnit::from_i32(1000));
        line.items.push(InlineItemResult {
            item_index: 0,
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            inline_size: original_width,
            shape_result: Some(sr_arc.clone()),
            has_forced_break: false,
        });
        line.used_width = original_width;

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        // Width should be reduced.
        assert!(
            line.used_width < original_width,
            "Normal mode should subtract trailing space width",
        );

        // text_range should be trimmed to exclude trailing spaces.
        assert_eq!(
            line.items[0].text_range.end, 5,
            "Normal mode at_item_end should trim text_range to 'hello' (5 bytes), got {}",
            line.items[0].text_range.end,
        );
    }

    // ── SP11 Round 18 Issue 2: complex script character break prefers overflow ──

    #[test]
    fn character_break_complex_script_does_not_split_joining_sequence() {
        // When overflow-wrap: break-word applies to Arabic text and the line is
        // too narrow, the breaker should force the entire joining cluster onto
        // the line (overflow) rather than splitting mid-sequence, which would
        // produce incorrect glyph forms.
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let text = "مرحبا"; // 5 Arabic chars, single joining sequence
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Rtl);
        let sr_arc = Arc::new(sr);

        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 1,
            intrinsic_inline_size: None,
        };

        let mut style = ComputedStyle::default();
        style.overflow_wrap = OverflowWrap::BreakWord;

        let items_data = super::super::items_builder::InlineItemsData {
            text: text.to_string(),
            items: vec![item],
            styles: vec![style],
        };

        // Available width is very narrow — much less than the word width.
        // The breaker should NOT split the Arabic text at an unsafe position.
        let narrow = LayoutUnit::from_f32(5.0);
        let mut breaker = LineBreaker::new(&items_data, narrow);
        let line = breaker.next_line(narrow);
        assert!(line.is_some(), "should produce at least one line");
        let line = line.unwrap();

        // The line should contain the ENTIRE Arabic word (overflow) rather
        // than a fragment that would have invalid shaping.
        assert!(
            !line.items.is_empty(),
            "line should have at least one item"
        );
        let item_result = &line.items[0];
        // The text_range should cover the entire word — since there are no
        // safe break points inside the joining sequence, the breaker must
        // force the whole cluster.
        assert_eq!(
            item_result.text_range.start, 0,
            "should start at beginning of text"
        );
        assert_eq!(
            item_result.text_range.end,
            text.len(),
            "should include entire Arabic word (overflow) instead of splitting joining sequence"
        );
    }

    // ── Issue 1 (R26): resolve_atomic_inline_width includes border+padding ──

    #[test]
    fn atomic_inline_width_content_box_adds_border_padding() {
        // A content-box element with width:100px, border:5px, padding:10px
        // should resolve to 100 + 2*5 + 2*10 = 130px total.
        use openui_style::BorderStyle;
        let mut style = ComputedStyle::default();
        style.width = openui_geometry::Length::px(100.0);
        style.border_left_width = 5;
        style.border_right_width = 5;
        style.border_left_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.padding_left = openui_geometry::Length::px(10.0);
        style.padding_right = openui_geometry::Length::px(10.0);
        style.box_sizing = openui_style::BoxSizing::ContentBox;

        let cb = LayoutUnit::from_i32(500);
        let w = resolve_atomic_inline_width(&style, cb, None);
        assert_eq!(
            w.to_f32(), 130.0,
            "content-box width:100 + border:10 + padding:20 = 130"
        );
    }

    #[test]
    fn atomic_inline_width_border_box_does_not_double_add() {
        // A border-box element with width:100px, border:5px, padding:10px
        // should resolve to exactly 100px (border-box already includes them).
        use openui_style::BorderStyle;
        let mut style = ComputedStyle::default();
        style.width = openui_geometry::Length::px(100.0);
        style.border_left_width = 5;
        style.border_right_width = 5;
        style.border_left_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.padding_left = openui_geometry::Length::px(10.0);
        style.padding_right = openui_geometry::Length::px(10.0);
        style.box_sizing = openui_style::BoxSizing::BorderBox;

        let cb = LayoutUnit::from_i32(500);
        let w = resolve_atomic_inline_width(&style, cb, None);
        assert_eq!(
            w.to_f32(), 100.0,
            "border-box width:100 already includes border+padding"
        );
    }

    #[test]
    fn atomic_inline_width_auto_adds_border_padding_to_intrinsic() {
        // width:auto with intrinsic=80px, border:3px each, padding:7px each
        // should return 80 + 6 + 14 = 100.
        use openui_style::BorderStyle;
        let mut style = ComputedStyle::default();
        style.width = openui_geometry::Length::auto();
        style.border_left_width = 3;
        style.border_right_width = 3;
        style.border_left_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.padding_left = openui_geometry::Length::px(7.0);
        style.padding_right = openui_geometry::Length::px(7.0);
        style.box_sizing = openui_style::BoxSizing::ContentBox;

        let cb = LayoutUnit::from_i32(500);
        let w = resolve_atomic_inline_width(&style, cb, Some(80.0));
        assert_eq!(
            w.to_f32(), 100.0,
            "auto width: intrinsic(80) + border(6) + padding(14) = 100"
        );
    }

    // ── Trailing space hanging — hang_width tracking ─────────────────

    /// Helper: create an InlineItem + InlineItemResult pair for a given text.
    fn make_trailing_space_test(
        text: &str,
        line_text_range: std::ops::Range<usize>,
        ws: WhiteSpace,
    ) -> (Vec<InlineItem>, InlineItemResult, ComputedStyle) {
        use openui_dom::NodeId;
        use openui_text::{Font, FontDescription, TextShaper, TextDirection};
        use std::sync::Arc;

        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let sr = shaper.shape(text, &font, TextDirection::Ltr);
        let sr_arc = Arc::new(sr);

        let mut style = ComputedStyle::default();
        style.white_space = ws;

        let at_item_end = line_text_range.end == text.len();
        let item = InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: if at_item_end {
                CollapseType::Collapsible
            } else {
                CollapseType::NotCollapsible
            },
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        };

        let item_result = InlineItemResult {
            item_index: 0,
            text_range: line_text_range,
            inline_size: LayoutUnit::from_f32(sr_arc.width),
            shape_result: Some(sr_arc),
            has_forced_break: false,
            item_type: InlineItemType::Text,
        };

        (vec![item], item_result, style)
    }

    #[test]
    fn hang_width_zero_for_normal_whitespace() {
        // In normal white-space mode, trailing spaces are stripped (not hung).
        // hang_width should remain 0.
        let text = "hello ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::Normal);

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = item_result.inline_size;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "normal white-space: hang_width should be 0 (spaces stripped, not hung)");
    }

    #[test]
    fn hang_width_nonzero_for_prewrap_at_item_end() {
        // In pre-wrap, trailing spaces hang. hang_width should capture their width.
        let text = "hello ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert!(line.hang_width > LayoutUnit::zero(),
            "pre-wrap: hang_width should be > 0 for trailing spaces");
        // hang_width + used_width should equal original total
        assert_eq!(line.used_width + line.hang_width, initial_width,
            "pre-wrap: used_width + hang_width should equal original width");
    }

    #[test]
    fn hang_width_nonzero_for_prewrap_mid_item() {
        // Pre-wrap mid-item split: trailing space should hang.
        let text = "hello world ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = LayoutUnit::from_f32(40.0);
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert!(line.hang_width > LayoutUnit::zero(),
            "pre-wrap mid-item: hang_width should be > 0 for trailing spaces");
        assert_eq!(line.items[0].text_range, 0..6,
            "pre-wrap mid-item: text_range should NOT be trimmed");
    }

    #[test]
    fn hang_width_zero_for_break_spaces() {
        // break-spaces never hangs — spaces are preserved and cause breaks.
        let text = "hello ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::BreakSpaces);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "break-spaces: hang_width should be 0 (no hanging)");
        assert_eq!(line.used_width, initial_width,
            "break-spaces: used_width should be unchanged");
    }

    #[test]
    fn hang_width_zero_for_pre() {
        // pre mode: whitespace is non-collapsible; mid-item split skips stripping.
        let text = "hello world ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::Pre);
        let initial_used = LayoutUnit::from_f32(40.0);

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_used;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "pre: hang_width should be 0");
        assert_eq!(line.used_width, initial_used,
            "pre: used_width should be unchanged");
    }

    #[test]
    fn hang_width_zero_for_nowrap() {
        // nowrap collapses trailing spaces (strip, not hang).
        let text = "hello ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::Nowrap);

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = item_result.inline_size;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "nowrap: hang_width should be 0 (spaces stripped)");
    }

    #[test]
    fn hang_width_zero_for_preline() {
        // pre-line collapses trailing spaces (strip, not hang).
        let text = "hello ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..6, WhiteSpace::PreLine);

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = item_result.inline_size;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "pre-line: hang_width should be 0 (spaces stripped)");
    }

    #[test]
    fn hang_width_prewrap_multi_space() {
        // Multiple trailing spaces in pre-wrap should all hang.
        let text = "hi   ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..5, WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert!(line.hang_width > LayoutUnit::zero(),
            "pre-wrap multi-space: hang_width should capture all trailing spaces");
        assert_eq!(line.used_width + line.hang_width, initial_width,
            "pre-wrap multi-space: used_width + hang_width == original");
    }

    #[test]
    fn hang_width_no_trailing_spaces() {
        // No trailing spaces: nothing to strip or hang.
        let text = "hello";
        let (items, item_result, style) = make_trailing_space_test(text, 0..5, WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "no trailing spaces: hang_width should be 0");
        assert_eq!(line.used_width, initial_width,
            "no trailing spaces: used_width unchanged");
    }

    #[test]
    fn hang_width_only_spaces_prewrap() {
        // Line content is only spaces in pre-wrap — all should hang.
        let text = "   ";
        let (items, item_result, style) = make_trailing_space_test(text, 0..3, WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, initial_width,
            "all-spaces pre-wrap: entire width should hang");
        assert_eq!(line.used_width, LayoutUnit::zero(),
            "all-spaces pre-wrap: used_width should be 0");
    }

    #[test]
    fn hang_width_tab_character_prewrap() {
        // Tab characters are hangable whitespace too.
        let text = "hi\t";
        let (items, item_result, style) = make_trailing_space_test(text, 0..3, WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert!(line.hang_width > LayoutUnit::zero(),
            "pre-wrap tab: tab character should hang");
        assert_eq!(line.items[0].text_range, 0..3,
            "pre-wrap tab: text_range preserved (hanging)");
    }

    #[test]
    fn hang_width_ideographic_space_prewrap() {
        // U+3000 IDEOGRAPHIC SPACE should be treated as a hangable space.
        let text = "hello\u{3000}";
        let (items, item_result, style) = make_trailing_space_test(text, 0..text.len(), WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert!(line.hang_width > LayoutUnit::zero(),
            "pre-wrap ideographic space: U+3000 should hang");
        assert_eq!(line.items[0].text_range.end, text.len(),
            "pre-wrap ideographic space: text_range preserved");
    }

    #[test]
    fn hang_width_ideographic_space_normal_strips() {
        // In normal white-space, ideographic space at end is stripped (not hung).
        let text = "hello\u{3000}";
        let (items, item_result, style) = make_trailing_space_test(text, 0..text.len(), WhiteSpace::Normal);

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = item_result.inline_size;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "normal ideographic space: should strip, not hang");
        // text_range should be trimmed to exclude the ideographic space
        assert!(line.items[0].text_range.end < text.len(),
            "normal ideographic space: text_range should be trimmed");
    }

    #[test]
    fn hang_width_mixed_spaces_prewrap() {
        // Mix of ASCII space and ideographic space should all hang.
        let text = "hi \u{3000}";
        let (items, item_result, style) = make_trailing_space_test(text, 0..text.len(), WhiteSpace::PreWrap);
        let initial_width = item_result.inline_size;

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = initial_width;
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &items, text, &[style]);

        assert!(line.hang_width > LayoutUnit::zero(),
            "pre-wrap mixed spaces: should hang");
        assert_eq!(line.used_width + line.hang_width, initial_width,
            "pre-wrap mixed: total preserved");
    }

    #[test]
    fn hang_width_empty_line() {
        // Empty line: nothing to strip.
        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        let items: Vec<InlineItem> = vec![];
        let styles: Vec<ComputedStyle> = vec![];
        strip_trailing_spaces(&mut line, &items, "", &styles);

        assert_eq!(line.hang_width, LayoutUnit::zero());
        assert_eq!(line.used_width, LayoutUnit::zero());
    }

    #[test]
    fn hang_width_non_text_last_item() {
        // If the last item is an AtomicInline (not Text), nothing should hang.
        use openui_dom::NodeId;

        let item = InlineItem {
            item_type: InlineItemType::AtomicInline,
            text_range: 0..0,
            node_id: NodeId::NONE,
            shape_result: None,
            style_index: 0,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: Some(50.0),
        };

        let item_result = InlineItemResult {
            item_index: 0,
            text_range: 0..0,
            inline_size: LayoutUnit::from_f32(50.0),
            shape_result: None,
            has_forced_break: false,
            item_type: InlineItemType::AtomicInline,
        };

        let mut line = LineInfo::new(LayoutUnit::from_f32(200.0));
        line.used_width = LayoutUnit::from_f32(50.0);
        line.items.push(item_result);

        strip_trailing_spaces(&mut line, &[item], "", &[ComputedStyle::default()]);

        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "atomic inline: nothing should hang");
    }

    #[test]
    fn line_info_hang_width_default_is_zero() {
        // Verify the default value of hang_width in a new LineInfo.
        let line = LineInfo::new(LayoutUnit::from_f32(500.0));
        assert_eq!(line.hang_width, LayoutUnit::zero(),
            "new LineInfo should have hang_width = 0");
    }
}
