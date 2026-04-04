//! BidiParagraph — bidirectional text analysis using UAX#9.
//!
//! Mirrors Blink's `BidiParagraph` from
//! `third_party/blink/renderer/platform/text/bidi_paragraph.h`.
//!
//! Wraps the `unicode-bidi` crate to provide:
//! - Per-character embedding levels
//! - Contiguous bidi runs (same-level segments)
//! - Visual reordering per UAX#9 L2

use unicode_bidi::{BidiInfo, Level};

use crate::shaping::TextDirection;

/// Result of bidirectional analysis for a paragraph of text.
///
/// Blink: `BidiParagraph` in `platform/text/bidi_paragraph.h`.
pub struct BidiParagraph {
    /// The original text.
    text: String,
    /// Per-character embedding levels (one level per char, not per byte).
    levels: Vec<Level>,
    /// Paragraph base direction.
    base_direction: TextDirection,
}

/// A contiguous run of characters sharing the same bidi embedding level.
///
/// Blink: `BidiRun` / `InlineItem` bidi_level field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BidiRun {
    /// Byte offset of the start in the original text.
    pub start: usize,
    /// Byte offset of the end in the original text (exclusive).
    pub end: usize,
    /// Embedding level (even = LTR, odd = RTL).
    pub level: u8,
    /// Direction derived from the level.
    pub direction: TextDirection,
}

impl BidiParagraph {
    /// Analyze text for bidirectional properties.
    ///
    /// `base_direction`:
    /// - `Some(Ltr)` — force LTR paragraph direction
    /// - `Some(Rtl)` — force RTL paragraph direction
    /// - `None` — auto-detect from first strong character (UAX#9 P2/P3)
    pub fn new(text: &str, base_direction: Option<TextDirection>) -> Self {
        if text.is_empty() {
            return Self {
                text: String::new(),
                levels: Vec::new(),
                base_direction: base_direction.unwrap_or(TextDirection::Ltr),
            };
        }

        let default_level = match base_direction {
            Some(TextDirection::Rtl) => Some(Level::rtl()),
            Some(TextDirection::Ltr) => Some(Level::ltr()),
            None => None,
        };

        let bidi_info = BidiInfo::new(text, default_level);

        // Get the first paragraph.
        let paragraph = &bidi_info.paragraphs[0];

        // BidiInfo.levels is per-byte. Convert to per-character levels.
        let para_range = paragraph.range.clone();
        let para_text = &text[para_range.clone()];
        let per_char_levels: Vec<Level> = para_text
            .char_indices()
            .map(|(byte_idx, _)| bidi_info.levels[para_range.start + byte_idx])
            .collect();

        let detected_direction = if paragraph.level.is_rtl() {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        };

        Self {
            text: text.to_string(),
            levels: per_char_levels,
            base_direction: detected_direction,
        }
    }

    /// Get bidi runs — contiguous segments with the same embedding level.
    ///
    /// Runs are returned in logical (source) order.
    pub fn runs(&self) -> Vec<BidiRun> {
        if self.levels.is_empty() {
            return Vec::new();
        }

        let mut runs = Vec::new();
        let char_byte_offsets: Vec<usize> =
            self.text.char_indices().map(|(i, _)| i).collect();

        if char_byte_offsets.is_empty() {
            return Vec::new();
        }

        let mut current_level = self.levels[0];
        let mut run_start_char = 0usize;

        for (char_idx, &level) in self.levels.iter().enumerate() {
            if level != current_level {
                let byte_start = char_byte_offsets[run_start_char];
                let byte_end = char_byte_offsets[char_idx];
                runs.push(BidiRun {
                    start: byte_start,
                    end: byte_end,
                    level: current_level.number(),
                    direction: if current_level.is_rtl() {
                        TextDirection::Rtl
                    } else {
                        TextDirection::Ltr
                    },
                });
                current_level = level;
                run_start_char = char_idx;
            }
        }

        // Emit the final run.
        let byte_start = char_byte_offsets[run_start_char];
        let byte_end = self.text.len();
        runs.push(BidiRun {
            start: byte_start,
            end: byte_end,
            level: current_level.number(),
            direction: if current_level.is_rtl() {
                TextDirection::Rtl
            } else {
                TextDirection::Ltr
            },
        });

        runs
    }

    /// Get visual ordering of runs per UAX#9 L2.
    ///
    /// This reorders the logical runs into the order they should be
    /// painted on screen.
    pub fn visual_runs(&self) -> Vec<BidiRun> {
        let mut runs = self.runs();
        reorder_visual(&mut runs);
        runs
    }

    /// The paragraph's base direction.
    #[inline]
    pub fn base_direction(&self) -> TextDirection {
        self.base_direction
    }

    /// Per-character embedding levels.
    #[inline]
    pub fn levels(&self) -> &[Level] {
        &self.levels
    }

    /// The original text.
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the bidi level for a character at the given char index.
    #[inline]
    pub fn level_at(&self, char_index: usize) -> u8 {
        if char_index < self.levels.len() {
            self.levels[char_index].number()
        } else {
            0
        }
    }

    /// Get the bidi level for a byte offset in the text.
    pub fn level_at_byte(&self, byte_offset: usize) -> u8 {
        let byte_offset = byte_offset.min(self.text.len());
        let char_index = self.text[..byte_offset].chars().count();
        self.level_at(char_index)
    }
}

/// UAX#9 L2: Reorder runs for visual display.
///
/// From highest level to lowest odd level, reverse any contiguous
/// sequence of runs at that level or higher.
fn reorder_visual(runs: &mut Vec<BidiRun>) {
    if runs.is_empty() {
        return;
    }

    let max_level = runs.iter().map(|r| r.level).max().unwrap_or(0);
    if max_level == 0 {
        return; // All LTR, no reordering needed
    }

    let min_odd_level = runs
        .iter()
        .map(|r| r.level)
        .filter(|l| l % 2 == 1)
        .min()
        .unwrap_or(max_level);

    for level in (min_odd_level..=max_level).rev() {
        let mut i = 0;
        while i < runs.len() {
            if runs[i].level >= level {
                let start = i;
                while i < runs.len() && runs[i].level >= level {
                    i += 1;
                }
                runs[start..i].reverse();
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text() {
        let bidi = BidiParagraph::new("", None);
        assert_eq!(bidi.runs().len(), 0);
        assert_eq!(bidi.visual_runs().len(), 0);
        assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    }

    #[test]
    fn pure_ltr() {
        let bidi = BidiParagraph::new("Hello world", None);
        assert_eq!(bidi.base_direction(), TextDirection::Ltr);
        let runs = bidi.runs();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].level, 0);
        assert_eq!(runs[0].direction, TextDirection::Ltr);
    }

    #[test]
    fn pure_rtl_hebrew() {
        let bidi = BidiParagraph::new("שלום עולם", None);
        assert_eq!(bidi.base_direction(), TextDirection::Rtl);
        let runs = bidi.runs();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].level, 1);
        assert_eq!(runs[0].direction, TextDirection::Rtl);
    }

    #[test]
    fn forced_ltr_direction() {
        let bidi = BidiParagraph::new("שלום", Some(TextDirection::Ltr));
        assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    }

    #[test]
    fn forced_rtl_direction() {
        let bidi = BidiParagraph::new("Hello", Some(TextDirection::Rtl));
        assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    }

    #[test]
    fn mixed_ltr_rtl() {
        let bidi = BidiParagraph::new("Hello שלום world", None);
        assert_eq!(bidi.base_direction(), TextDirection::Ltr);
        let runs = bidi.runs();
        assert!(runs.len() >= 2);
        assert_eq!(runs[0].direction, TextDirection::Ltr);
    }

    #[test]
    fn reorder_visual_pure_ltr() {
        let mut runs = vec![
            BidiRun { start: 0, end: 5, level: 0, direction: TextDirection::Ltr },
        ];
        reorder_visual(&mut runs);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].start, 0);
    }

    #[test]
    fn reorder_visual_single_rtl() {
        let mut runs = vec![
            BidiRun { start: 0, end: 5, level: 1, direction: TextDirection::Rtl },
        ];
        reorder_visual(&mut runs);
        assert_eq!(runs.len(), 1);
    }

    #[test]
    fn reorder_visual_mixed() {
        let mut runs = vec![
            BidiRun { start: 0, end: 6, level: 0, direction: TextDirection::Ltr },
            BidiRun { start: 6, end: 10, level: 1, direction: TextDirection::Rtl },
            BidiRun { start: 10, end: 16, level: 0, direction: TextDirection::Ltr },
        ];
        reorder_visual(&mut runs);
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs[1].start, 6);
        assert_eq!(runs[2].start, 10);
    }

    #[test]
    fn levels_per_char_count() {
        // Verify levels length matches char count, not byte count
        let bidi = BidiParagraph::new("שלום 42", None);
        let char_count = "שלום 42".chars().count();
        assert_eq!(bidi.levels().len(), char_count);
    }
}
