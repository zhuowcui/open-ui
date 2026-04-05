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

        // Replace paragraph separators with neutral chars so unicode-bidi
        // treats the entire text as a single paragraph (CSS Writing Modes §2.4.1).
        let sanitized: String = text
            .chars()
            .map(|ch| match ch {
                '\n' | '\r' | '\u{0085}' | '\u{2029}' => '\u{200B}', // ZWSP
                other => other,
            })
            .collect();

        let default_level = match base_direction {
            Some(TextDirection::Rtl) => Some(Level::rtl()),
            Some(TextDirection::Ltr) => Some(Level::ltr()),
            None => None,
        };

        let bidi_info = BidiInfo::new(&sanitized, default_level);

        // Now guaranteed to be a single paragraph covering the entire text.
        let paragraph = &bidi_info.paragraphs[0];

        // BidiInfo.levels is per-byte. Convert to per-character levels.
        let para_range = paragraph.range.clone();
        let para_text = &sanitized[para_range.clone()];
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
            text: text.to_string(), // Store original text for correct byte offsets
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
    ///
    /// If `byte_offset` falls in the middle of a multi-byte UTF-8 character,
    /// the offset is adjusted back to the nearest character boundary.
    pub fn level_at_byte(&self, byte_offset: usize) -> u8 {
        let byte_offset = byte_offset.min(self.text.len());
        let safe_offset = self.text.floor_char_boundary(byte_offset);
        let char_index = self.text[..safe_offset].chars().count();
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

    #[test]
    fn level_at_byte_mid_char_no_panic() {
        // Hebrew chars are 2 bytes each. Offset 1 is mid-char → should not panic.
        let text = "שלום";
        let bidi = BidiParagraph::new(text, None);
        // Byte offset 1 is inside the first 2-byte Hebrew character.
        let level = bidi.level_at_byte(1);
        // Should return the level for the first char (RTL = 1).
        assert_eq!(level, 1, "Mid-char offset should resolve to first char's level");
    }

    #[test]
    fn level_at_byte_valid_offset() {
        // Valid byte offset at a char boundary should return the correct level.
        let text = "AB שלום";
        let bidi = BidiParagraph::new(text, None);
        // Byte offset 0 is 'A' (LTR).
        assert_eq!(bidi.level_at_byte(0), 0, "LTR char at offset 0 should have level 0");
        // Byte offset 3 is the start of the Hebrew text (after "AB ").
        assert_eq!(bidi.level_at_byte(3), 1, "RTL char at offset 3 should have level 1");
    }

    #[test]
    fn multi_paragraph_newline_bidi_levels() {
        // "Hello\nمرحبا" — Arabic after newline must get RTL level,
        // not LTR level 0 (the old bug: only first paragraph was analyzed).
        let text = "Hello\n\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        let bidi = BidiParagraph::new(text, None);

        // Levels should cover the entire text (all characters).
        let char_count = text.chars().count();
        assert_eq!(bidi.levels().len(), char_count, "levels must span entire text");

        // Arabic chars after newline must be RTL (level >= 1, odd).
        // 'H' is char 0 (LTR), '\n' is char 5, Arabic starts at char 6.
        for i in 6..char_count {
            assert!(
                bidi.levels()[i].is_rtl(),
                "Arabic char at index {} should be RTL, got level {}",
                i,
                bidi.levels()[i].number(),
            );
        }
    }

    #[test]
    fn multi_paragraph_preserves_original_text() {
        // Ensure the original text (with newlines) is stored, not the sanitized version.
        let text = "Hello\nWorld";
        let bidi = BidiParagraph::new(text, None);
        assert_eq!(bidi.text(), text);
        assert!(bidi.text().contains('\n'));
    }

    #[test]
    fn multi_paragraph_crlf_bidi_levels() {
        // Windows-style \r\n should not break bidi analysis across paragraphs.
        let text = "Hello\r\n\u{05E9}\u{05DC}\u{05D5}\u{05DD}"; // Hello\r\nשלום
        let bidi = BidiParagraph::new(text, None);

        let char_count = text.chars().count();
        assert_eq!(bidi.levels().len(), char_count);

        // Hebrew starts after "Hello\r\n" = 7 chars
        for i in 7..char_count {
            assert!(
                bidi.levels()[i].is_rtl(),
                "Hebrew char at index {} should be RTL after \\r\\n, got level {}",
                i,
                bidi.levels()[i].number(),
            );
        }
    }
}
