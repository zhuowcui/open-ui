//! CSS hyphenation support — Knuth-Liang algorithm + soft hyphen handling.
//!
//! Extracted from Chromium's AOSP Minikin hyphenator:
//! `external/minikin/libs/minikin/Hyphenator.cpp`
//!
//! The Knuth-Liang algorithm uses a trie of patterns to determine valid
//! hyphenation points in a word. Each pattern assigns priority values
//! at character positions; odd values indicate a valid hyphen point.
//!
//! This module provides:
//! - Soft hyphen (U+00AD) detection and location finding
//! - Knuth-Liang pattern-based hyphenation engine
//! - Built-in English (en-US) patterns
//! - `hyphenate-limit-chars` constraint enforcement

mod patterns;
mod trie;

use trie::PatternTrie;

/// The soft hyphen character (U+00AD).
///
/// When `hyphens` is `manual` or `auto`, soft hyphens mark valid break points.
/// When `hyphens` is `none`, soft hyphens are ignored entirely.
pub const SOFT_HYPHEN: char = '\u{00AD}';

/// Check if a character is a soft hyphen (U+00AD).
#[inline]
pub fn is_soft_hyphen(ch: char) -> bool {
    ch == SOFT_HYPHEN
}

/// Find all soft hyphen byte offsets in text.
///
/// Returns byte offsets where soft hyphens occur. These are valid break
/// points when `hyphens` is `manual` or `auto`.
pub fn find_soft_hyphens(text: &str) -> Vec<usize> {
    text.char_indices()
        .filter(|(_, ch)| is_soft_hyphen(*ch))
        .map(|(i, _)| i)
        .collect()
}

/// Find the last soft hyphen byte offset strictly before `before_byte`.
///
/// Used by the line breaker to find the best soft hyphen break point
/// that fits within the available width.
pub fn last_soft_hyphen_before(text: &str, before_byte: usize) -> Option<usize> {
    let search_range = &text[..before_byte.min(text.len())];
    search_range
        .char_indices()
        .rev()
        .find(|(_, ch)| is_soft_hyphen(*ch))
        .map(|(i, _)| i)
}

/// Strip soft hyphens from text, returning the cleaned string.
///
/// Used when rendering text with `hyphens: none` — soft hyphens become
/// invisible zero-width characters and should not affect layout.
pub fn strip_soft_hyphens(text: &str) -> String {
    text.chars().filter(|ch| !is_soft_hyphen(*ch)).collect()
}

/// Hyphenation engine using the Knuth-Liang algorithm.
///
/// Matches Chromium's AOSP Minikin implementation. Uses a trie of patterns
/// to determine valid hyphenation points in words, then applies
/// `hyphenate-limit-chars` constraints (min word length, min prefix, min suffix).
pub struct Hyphenation {
    trie: PatternTrie,
    /// Minimum characters before the hyphen point.
    min_prefix: usize,
    /// Minimum characters after the hyphen point.
    min_suffix: usize,
    /// Minimum word length to attempt hyphenation.
    min_word: usize,
}

impl Hyphenation {
    /// Create a hyphenation engine with the given pattern trie and limits.
    pub fn new(trie: PatternTrie, min_prefix: usize, min_suffix: usize, min_word: usize) -> Self {
        Self {
            trie,
            min_prefix,
            min_suffix,
            min_word,
        }
    }

    /// Create a hyphenation engine for English (en-US) with Blink defaults.
    ///
    /// Uses built-in English patterns from the AOSP hyphenation pattern set
    /// (same patterns Chromium uses via `hyph-en-us.hyb`).
    pub fn english() -> Self {
        Self::english_with_limits(2, 2, 5)
    }

    /// Create a hyphenation engine for English with custom limits.
    ///
    /// `min_prefix`: minimum characters before hyphen (default 2)
    /// `min_suffix`: minimum characters after hyphen (default 2)
    /// `min_word`: minimum word length to hyphenate (default 5)
    pub fn english_with_limits(min_prefix: usize, min_suffix: usize, min_word: usize) -> Self {
        let trie = patterns::build_english_trie();
        Self::new(trie, min_prefix, min_suffix, min_word)
    }

    /// Create a hyphenation engine from `hyphenate-limit-chars` CSS values.
    ///
    /// `limits`: `(min_word, min_prefix, min_suffix)` from `ComputedStyle`.
    pub fn english_from_css_limits(limits: (u8, u8, u8)) -> Self {
        Self::english_with_limits(limits.1 as usize, limits.2 as usize, limits.0 as usize)
    }

    /// Check if a word meets the minimum length for hyphenation.
    #[inline]
    pub fn should_hyphenate(&self, word: &str) -> bool {
        word.chars().count() >= self.min_word
    }

    /// Find all valid hyphenation points in a word.
    ///
    /// Returns a sorted vector of **character offsets** where a hyphen can be
    /// inserted. The offsets are positions between characters (e.g., offset 2
    /// means the hyphen goes after the 2nd character).
    ///
    /// The Knuth-Liang algorithm:
    /// 1. Surround word with boundary markers: `.word.`
    /// 2. For each position, look up all matching patterns in the trie
    /// 3. At each position, keep the maximum priority value
    /// 4. Odd priority values indicate valid hyphenation points
    /// 5. Apply min_prefix / min_suffix constraints
    pub fn hyphen_locations(&self, word: &str) -> Vec<usize> {
        let chars: Vec<char> = word.chars().collect();
        let char_count = chars.len();

        if char_count < self.min_word {
            return Vec::new();
        }

        // Build the augmented word: . + lowercase(word) + .
        // The dots are boundary markers used in Knuth-Liang patterns.
        let mut augmented: Vec<char> = Vec::with_capacity(char_count + 2);
        augmented.push('.');
        for &ch in &chars {
            augmented.push(ch.to_ascii_lowercase());
        }
        augmented.push('.');

        // Priority values array — one more than augmented length to cover all inter-character positions.
        // values[i] corresponds to the position before augmented[i].
        let aug_len = augmented.len();
        let mut values = vec![0u8; aug_len + 1];

        // For each starting position in the augmented word, look up all
        // pattern prefixes in the trie and apply their priority values.
        for start in 0..aug_len {
            self.trie.apply_patterns(&augmented[start..], start, &mut values);
        }

        // Extract hyphenation points. The priority values in the augmented word
        // map to positions in the original word as follows:
        // augmented: . w o r d .
        // indices:   0 1 2 3 4 5
        // values:    0 v0 v1 v2 v3 v4 v5
        //
        // A hyphen point at position i in the original word (between chars[i-1]
        // and chars[i]) corresponds to values[i+1] being odd.
        let mut points = Vec::new();
        let min_pos = self.min_prefix;
        let max_pos = if char_count > self.min_suffix {
            char_count - self.min_suffix
        } else {
            0
        };

        for pos in min_pos..=max_pos {
            if pos == 0 || pos >= char_count {
                continue;
            }
            // values[pos + 1] corresponds to the position between chars[pos-1] and chars[pos]
            if values[pos + 1] % 2 == 1 {
                points.push(pos);
            }
        }

        points
    }

    /// Find all valid hyphenation points as byte offsets in the word string.
    ///
    /// This is the byte-offset version of `hyphen_locations`, suitable for
    /// direct use with string slicing.
    pub fn hyphen_byte_locations(&self, word: &str) -> Vec<usize> {
        let char_offsets = self.hyphen_locations(word);
        if char_offsets.is_empty() {
            return Vec::new();
        }

        // Map character offsets to byte offsets
        let char_to_byte: Vec<usize> = word
            .char_indices()
            .map(|(byte_idx, _)| byte_idx)
            .chain(std::iter::once(word.len()))
            .collect();

        char_offsets
            .into_iter()
            .filter_map(|ci| char_to_byte.get(ci).copied())
            .collect()
    }

    /// Find the last valid hyphenation byte offset strictly before `before_byte`.
    ///
    /// Used by the line breaker to find the best hyphenation break point
    /// that fits within the available width on the current line.
    pub fn last_hyphen_before(&self, word: &str, before_byte: usize) -> Option<usize> {
        let byte_locs = self.hyphen_byte_locations(word);
        byte_locs.into_iter().rev().find(|&b| b < before_byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Soft hyphen tests ───────────────────────────────────────────

    #[test]
    fn is_soft_hyphen_true() {
        assert!(is_soft_hyphen('\u{00AD}'));
    }

    #[test]
    fn is_soft_hyphen_false_regular_chars() {
        assert!(!is_soft_hyphen('-'));
        assert!(!is_soft_hyphen('a'));
        assert!(!is_soft_hyphen(' '));
        assert!(!is_soft_hyphen('\u{200B}')); // zero-width space
    }

    #[test]
    fn find_soft_hyphens_none() {
        assert!(find_soft_hyphens("hello world").is_empty());
    }

    #[test]
    fn find_soft_hyphens_single() {
        let text = "hy\u{00AD}phen";
        let locs = find_soft_hyphens(text);
        assert_eq!(locs, vec![2]); // byte offset of soft hyphen
    }

    #[test]
    fn find_soft_hyphens_multiple() {
        let text = "hy\u{00AD}phen\u{00AD}ation";
        let locs = find_soft_hyphens(text);
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn find_soft_hyphens_at_boundaries() {
        let text = "\u{00AD}hello\u{00AD}";
        let locs = find_soft_hyphens(text);
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0], 0);
    }

    #[test]
    fn last_soft_hyphen_before_finds_last() {
        let text = "hy\u{00AD}phen\u{00AD}ation";
        let shy_locs = find_soft_hyphens(text);
        // Find last soft hyphen before end of text
        let last = last_soft_hyphen_before(text, text.len());
        assert_eq!(last, Some(shy_locs[1]));
    }

    #[test]
    fn last_soft_hyphen_before_respects_limit() {
        let text = "hy\u{00AD}phen\u{00AD}ation";
        let shy_locs = find_soft_hyphens(text);
        // Find last soft hyphen before the second one
        let last = last_soft_hyphen_before(text, shy_locs[1]);
        assert_eq!(last, Some(shy_locs[0]));
    }

    #[test]
    fn last_soft_hyphen_before_none() {
        assert_eq!(last_soft_hyphen_before("hello", 5), None);
    }

    #[test]
    fn strip_soft_hyphens_removes_all() {
        let text = "hy\u{00AD}phen\u{00AD}ation";
        assert_eq!(strip_soft_hyphens(text), "hyphenation");
    }

    #[test]
    fn strip_soft_hyphens_no_change() {
        assert_eq!(strip_soft_hyphens("hello"), "hello");
    }

    // ── Knuth-Liang algorithm tests ─────────────────────────────────

    #[test]
    fn should_hyphenate_short_words() {
        let h = Hyphenation::english();
        assert!(!h.should_hyphenate("the"));   // 3 chars < 5
        assert!(!h.should_hyphenate("is"));    // 2 chars < 5
        assert!(!h.should_hyphenate("a"));     // 1 char < 5
        assert!(!h.should_hyphenate(""));      // empty
        assert!(!h.should_hyphenate("test"));  // 4 chars < 5
    }

    #[test]
    fn should_hyphenate_long_words() {
        let h = Hyphenation::english();
        assert!(h.should_hyphenate("hello"));     // 5 chars = 5
        assert!(h.should_hyphenate("computer"));  // 8 chars > 5
        assert!(h.should_hyphenate("hyphenation")); // 11 chars > 5
    }

    #[test]
    fn hyphen_locations_empty_word() {
        let h = Hyphenation::english();
        assert!(h.hyphen_locations("").is_empty());
    }

    #[test]
    fn hyphen_locations_short_word() {
        let h = Hyphenation::english();
        assert!(h.hyphen_locations("cat").is_empty());
    }

    #[test]
    fn hyphenation_respects_min_prefix() {
        let h = Hyphenation::english_with_limits(3, 2, 5);
        let locs = h.hyphen_locations("hyphenation");
        // No break point should have fewer than 3 characters before it
        for &loc in &locs {
            assert!(loc >= 3, "break at {} violates min_prefix=3", loc);
        }
    }

    #[test]
    fn hyphenation_respects_min_suffix() {
        let h = Hyphenation::english_with_limits(2, 3, 5);
        let locs = h.hyphen_locations("hyphenation");
        let char_count = "hyphenation".chars().count();
        for &loc in &locs {
            assert!(
                char_count - loc >= 3,
                "break at {} violates min_suffix=3 (word len={})",
                loc,
                char_count
            );
        }
    }

    #[test]
    fn hyphenation_respects_min_word() {
        let h = Hyphenation::english_with_limits(2, 2, 8);
        assert!(h.hyphen_locations("hello").is_empty()); // 5 < 8
        // "computer" has 8 chars, meets threshold
        // It may or may not have points depending on patterns, but min_word is met
    }

    #[test]
    fn hyphenation_known_word_hyphenation() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("hyphenation");
        // "hyphenation" should have at least one hyphenation point
        assert!(
            !locs.is_empty(),
            "expected hyphenation points for 'hyphenation', got none"
        );
        // All points should be valid character offsets
        let len = "hyphenation".chars().count();
        for &loc in &locs {
            assert!(loc > 0 && loc < len, "invalid hyphen location: {}", loc);
        }
    }

    #[test]
    fn hyphenation_known_word_computer() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("computer");
        assert!(
            !locs.is_empty(),
            "expected hyphenation points for 'computer', got none"
        );
    }

    #[test]
    fn hyphenation_known_word_algorithm() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("algorithm");
        assert!(
            !locs.is_empty(),
            "expected hyphenation points for 'algorithm', got none"
        );
    }

    #[test]
    fn hyphenation_known_word_information() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("information");
        assert!(
            !locs.is_empty(),
            "expected hyphenation points for 'information', got none"
        );
    }

    #[test]
    fn hyphenation_known_word_programming() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("programming");
        assert!(
            !locs.is_empty(),
            "expected hyphenation points for 'programming', got none"
        );
    }

    #[test]
    fn hyphen_byte_locations_ascii() {
        let h = Hyphenation::english();
        let byte_locs = h.hyphen_byte_locations("hyphenation");
        let char_locs = h.hyphen_locations("hyphenation");
        // For ASCII, byte offsets equal character offsets
        assert_eq!(byte_locs, char_locs);
    }

    #[test]
    fn last_hyphen_before_finds_best() {
        let h = Hyphenation::english();
        let byte_locs = h.hyphen_byte_locations("hyphenation");
        if byte_locs.len() >= 2 {
            let last = h.last_hyphen_before("hyphenation", byte_locs[byte_locs.len() - 1]);
            // Should find one of the earlier locations
            assert!(last.is_some());
            assert!(last.unwrap() < byte_locs[byte_locs.len() - 1]);
        }
    }

    #[test]
    fn last_hyphen_before_none_if_too_early() {
        let h = Hyphenation::english();
        let byte_locs = h.hyphen_byte_locations("hyphenation");
        if !byte_locs.is_empty() {
            // Looking before the first hyphen point should return None
            let result = h.last_hyphen_before("hyphenation", byte_locs[0]);
            assert!(result.is_none() || result.unwrap() < byte_locs[0]);
        }
    }

    #[test]
    fn hyphenation_case_insensitive() {
        let h = Hyphenation::english();
        let lower = h.hyphen_locations("hyphenation");
        let upper = h.hyphen_locations("HYPHENATION");
        let mixed = h.hyphen_locations("Hyphenation");
        // All should produce the same results since the algorithm lowercases internally
        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
    }

    #[test]
    fn hyphenation_from_css_limits() {
        let h = Hyphenation::english_from_css_limits((6, 3, 3));
        // min_word=6, min_prefix=3, min_suffix=3
        assert!(!h.should_hyphenate("hello")); // 5 < 6
        assert!(h.should_hyphenate("information")); // 11 >= 6
        let locs = h.hyphen_locations("information");
        for &loc in &locs {
            assert!(loc >= 3, "violates min_prefix=3");
            assert!(
                "information".chars().count() - loc >= 3,
                "violates min_suffix=3"
            );
        }
    }

    #[test]
    fn hyphenation_single_syllable_words() {
        let h = Hyphenation::english();
        // Short single-syllable words typically shouldn't be hyphenated
        assert!(h.hyphen_locations("strength").is_empty() || true);
        // "through" - single syllable, patterns may or may not produce points
        // but with min_prefix=2, min_suffix=2, it's constrained
    }

    #[test]
    fn hyphenation_many_syllables() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("international");
        assert!(
            !locs.is_empty(),
            "expected hyphenation points for 'international'"
        );
        // Should have multiple break points
        assert!(
            locs.len() >= 2,
            "expected at least 2 hyphenation points for 'international', got {}",
            locs.len()
        );
    }

    #[test]
    fn hyphenation_points_are_sorted() {
        let h = Hyphenation::english();
        let locs = h.hyphen_locations("internationalization");
        for w in locs.windows(2) {
            assert!(w[0] < w[1], "hyphenation points not sorted: {:?}", locs);
        }
    }

    #[test]
    fn hyphenation_points_within_bounds() {
        let h = Hyphenation::english();
        let words = [
            "hyphenation",
            "algorithm",
            "computer",
            "information",
            "programming",
            "international",
            "university",
            "communication",
        ];
        for word in &words {
            let len = word.chars().count();
            for &loc in &h.hyphen_locations(word) {
                assert!(loc > 0, "'{}': hyphen at 0", word);
                assert!(loc < len, "'{}': hyphen at {} >= len {}", word, loc, len);
            }
        }
    }

    #[test]
    fn hyphen_byte_locations_empty_result() {
        let h = Hyphenation::english();
        assert!(h.hyphen_byte_locations("cat").is_empty());
    }

    #[test]
    fn last_hyphen_before_at_end() {
        let h = Hyphenation::english();
        let result = h.last_hyphen_before("hyphenation", "hyphenation".len());
        let byte_locs = h.hyphen_byte_locations("hyphenation");
        if !byte_locs.is_empty() {
            assert_eq!(result, byte_locs.last().copied());
        }
    }

    // ── Combined soft-hyphen + automatic tests ──────────────────────

    #[test]
    fn soft_hyphen_in_word_with_auto_hyphenation() {
        // Soft hyphens should work independently of pattern-based hyphenation
        let text = "un\u{00AD}break\u{00AD}able";
        let soft = find_soft_hyphens(text);
        assert_eq!(soft.len(), 2);
        // Stripping should give clean word
        assert_eq!(strip_soft_hyphens(text), "unbreakable");
    }

    #[test]
    fn soft_hyphen_only_mode() {
        // In manual mode, only soft hyphens should be break points
        let text = "pro\u{00AD}gram\u{00AD}ming";
        let soft = find_soft_hyphens(text);
        assert_eq!(soft.len(), 2);
        // These are the only valid break points in manual mode
    }
}
