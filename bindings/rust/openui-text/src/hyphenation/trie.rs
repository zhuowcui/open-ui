//! Pattern trie for the Knuth-Liang hyphenation algorithm.
//!
//! The trie stores hyphenation patterns where each pattern is a sequence of
//! characters interleaved with priority digits. For example, the pattern
//! `.hy1p` means: at the start of a word, between 'h' and 'y' there is a
//! priority-1 break opportunity (odd = break allowed).
//!
//! Reference: Liang, F.M. (1983). "Word Hy-phen-a-tion by Com-put-er".
//! Stanford University Department of Computer Science technical report.

use std::collections::HashMap;

/// A node in the pattern trie.
///
/// Each node represents a character position in one or more patterns.
/// The `values` field stores priority values that patterns assign at
/// positions relative to this node's depth in the trie.
#[derive(Debug, Default)]
pub struct PatternTrie {
    children: HashMap<char, PatternTrie>,
    /// Priority values for this pattern endpoint.
    /// `values[i]` is the priority at position `i` relative to the pattern start.
    /// Empty if this node is not a pattern endpoint.
    values: Vec<u8>,
}

impl PatternTrie {
    /// Create a new empty trie node.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a Knuth-Liang pattern into the trie.
    ///
    /// A pattern string like `"hy1p"` is parsed as:
    /// - Characters: `['h', 'y', 'p']`
    /// - Values at positions: `[0, 0, 1, 0]` (digit 1 between y and p)
    ///
    /// The pattern `".hy1p"` starts with `.` (word boundary marker).
    pub fn insert_pattern(&mut self, pattern: &str) {
        let (chars, values) = parse_pattern(pattern);
        let mut node = self;
        for &ch in &chars {
            node = node.children.entry(ch).or_default();
        }
        node.values = values;
    }

    /// Apply all matching patterns starting at a given position.
    ///
    /// Given `text[start..]` of the augmented word (`.word.`), walks the trie
    /// matching characters. At every pattern endpoint, applies the maximum
    /// priority values to the `result` array at the correct offsets.
    ///
    /// `start`: offset into the augmented word where matching begins
    /// `result`: mutable priority array covering the full augmented word
    pub fn apply_patterns(&self, text: &[char], start: usize, result: &mut [u8]) {
        let mut node = self;

        // Check if this node itself has values (zero-length pattern — shouldn't happen, but safe)
        if !node.values.is_empty() {
            apply_values(&node.values, start, result);
        }

        for (_i, &ch) in text.iter().enumerate() {
            match node.children.get(&ch) {
                Some(child) => {
                    node = child;
                    if !node.values.is_empty() {
                        apply_values(&node.values, start, result);
                    }
                }
                None => return,
            }
        }
    }
}

/// Apply pattern priority values to the result array, keeping maximums.
///
/// `values`: priority values from a matched pattern
/// `start`: the starting position in the result array
/// `result`: the full priority array
fn apply_values(values: &[u8], start: usize, result: &mut [u8]) {
    for (i, &v) in values.iter().enumerate() {
        let pos = start + i;
        if pos < result.len() && v > result[pos] {
            result[pos] = v;
        }
    }
}

/// Parse a Knuth-Liang pattern string into characters and priority values.
///
/// Pattern format: characters interleaved with optional digits.
/// - `"hy1p"` → chars: `['h', 'y', 'p']`, values: `[0, 0, 1, 0]`
/// - `".ab2c"` → chars: `['.', 'a', 'b', 'c']`, values: `[0, 0, 0, 2, 0]`
/// - `"4ab"` → chars: `['a', 'b']`, values: `[4, 0, 0]`
///
/// The values array has length `chars.len() + 1` (one value between each pair
/// of characters plus one at each end).
fn parse_pattern(pattern: &str) -> (Vec<char>, Vec<u8>) {
    let mut chars = Vec::new();
    let mut values = Vec::new();
    let mut pending_digit: Option<u8> = None;

    for ch in pattern.chars() {
        if ch.is_ascii_digit() {
            pending_digit = Some(ch as u8 - b'0');
        } else {
            values.push(pending_digit.unwrap_or(0));
            pending_digit = None;
            chars.push(ch);
        }
    }
    // Final value after last character
    values.push(pending_digit.unwrap_or(0));

    (chars, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_pattern() {
        let (chars, values) = parse_pattern("hy1p");
        assert_eq!(chars, vec!['h', 'y', 'p']);
        assert_eq!(values, vec![0, 0, 1, 0]);
    }

    #[test]
    fn parse_pattern_with_boundary() {
        let (chars, values) = parse_pattern(".hy1p");
        assert_eq!(chars, vec!['.', 'h', 'y', 'p']);
        assert_eq!(values, vec![0, 0, 0, 1, 0]);
    }

    #[test]
    fn parse_pattern_leading_digit() {
        let (chars, values) = parse_pattern("4ab");
        assert_eq!(chars, vec!['a', 'b']);
        assert_eq!(values, vec![4, 0, 0]);
    }

    #[test]
    fn parse_pattern_multiple_digits() {
        let (chars, values) = parse_pattern("a1b2c3d");
        assert_eq!(chars, vec!['a', 'b', 'c', 'd']);
        assert_eq!(values, vec![0, 1, 2, 3, 0]);
    }

    #[test]
    fn parse_pattern_trailing_digit() {
        let (chars, values) = parse_pattern("ab4");
        assert_eq!(chars, vec!['a', 'b']);
        assert_eq!(values, vec![0, 0, 4]);
    }

    #[test]
    fn trie_insert_and_apply() {
        let mut trie = PatternTrie::new();
        trie.insert_pattern("hy1p");

        // Augmented word: ".hyp."
        let word: Vec<char> = ".hyp.".chars().collect();
        let mut result = vec![0u8; word.len() + 1];

        // Apply from position 1 (the 'h')
        trie.apply_patterns(&word[1..], 1, &mut result);

        // The pattern "hy1p" should set value 1 at position between y and p
        // In the augmented word ".hyp.", h is at index 1, y at 2, p at 3
        // Pattern values: [0, 0, 1, 0] applied starting at offset 1
        // result[1+2] = result[3] = 1
        assert_eq!(result[3], 1, "expected priority 1 between y and p");
    }

    #[test]
    fn trie_multiple_patterns_max_wins() {
        let mut trie = PatternTrie::new();
        trie.insert_pattern("ab1c");  // sets position 3 to 1
        trie.insert_pattern("b2c");   // sets position 3 to 2

        let word: Vec<char> = ".abc.".chars().collect();
        let mut result = vec![0u8; word.len() + 1];

        // Apply all starting positions
        for start in 0..word.len() {
            trie.apply_patterns(&word[start..], start, &mut result);
        }

        // Position between b and c should have max(1, 2) = 2
        // In ".abc.", b is at 2, c is at 3
        // "ab1c" from pos 1: values[0,0,1,0] → result[1]=0, result[2]=0, result[3]=1, result[4]=0
        // "b2c" from pos 2: values[0,2,0] → result[2]=0, result[3]=2, result[4]=0
        assert_eq!(result[3], 2, "max of competing patterns should win");
    }

    #[test]
    fn trie_boundary_pattern() {
        let mut trie = PatternTrie::new();
        trie.insert_pattern(".ab1c");

        let word: Vec<char> = ".abc.".chars().collect();
        let mut result = vec![0u8; word.len() + 1];

        trie.apply_patterns(&word[0..], 0, &mut result);

        // ".ab1c" from pos 0: chars ['.','a','b','c'], values [0,0,0,1,0]
        // result[0+3] = result[3] = 1
        assert_eq!(result[3], 1);
    }
}
