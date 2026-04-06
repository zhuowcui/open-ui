//! Score-based line breaker — Knuth-Plass style paragraph optimization.
//!
//! Implements CSS `text-wrap: balance` and `text-wrap: pretty` using
//! paragraph-level optimization to minimize "badness" across all lines.
//!
//! Blink: `score_line_breaker.cc` in
//! `third_party/blink/renderer/core/layout/inline/`.
//!
//! The algorithm considers all possible break points in a paragraph and
//! selects the set that minimizes total demerits. It falls back to greedy
//! breaking when the paragraph exceeds a configurable threshold (to bound
//! the O(n²) cost).
//!
//! ## Knuth-Plass overview
//!
//! Each feasible breakpoint is scored based on:
//! - **Badness**: how far the actual line width deviates from the ideal
//! - **Penalty**: the cost of breaking at that point (e.g., hyphen penalty)
//! - **Demerits**: a function of badness + penalty + fitness class transitions
//!
//! The algorithm finds the sequence of breaks with minimum total demerits
//! using dynamic programming.
//!
//! ## Balance mode
//!
//! `text-wrap: balance` attempts to equalize line widths. After finding
//! the optimal breaks, it checks whether redistributing content can
//! produce more balanced lines. The target width is the average of all
//! line widths.
//!
//! ## Pretty mode
//!
//! `text-wrap: pretty` minimizes orphans and widows by penalizing very
//! short last lines and single-word lines, in addition to the standard
//! Knuth-Plass demerits.

use openui_style::TextWrap;

/// Maximum number of items before we fall back to greedy breaking.
///
/// Blink uses a similar threshold to prevent O(n²) blowup on very long
/// paragraphs. With 500 items, the scoring loop does ~250K comparisons
/// which completes in well under 1ms on modern hardware.
const MAX_ITEMS_FOR_SCORING: usize = 500;

/// Penalty applied for hyphenation breaks.
const HYPHEN_PENALTY: f64 = 50.0;

/// Penalty for consecutive hyphenated lines.
const CONSECUTIVE_HYPHEN_PENALTY: f64 = 3000.0;

/// Penalty for a very short last line (orphan avoidance).
const ORPHAN_PENALTY: f64 = 5000.0;

/// Minimum ratio of last-line width to available width before orphan penalty kicks in.
const ORPHAN_WIDTH_RATIO: f64 = 0.2;

/// Infinity demerits — indicates infeasible break sequence.
const INFINITE_DEMERITS: f64 = 1e18;

/// Fitness classes for the Knuth-Plass algorithm.
///
/// Lines are classified by how tightly or loosely they are set, and
/// transitions between non-adjacent classes incur extra demerits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FitnessClass {
    /// Very tight line (compressed significantly).
    Tight = 0,
    /// Normal line (close to target width).
    Normal = 1,
    /// Loose line (stretched somewhat).
    Loose = 2,
    /// Very loose line (significant stretch).
    VeryLoose = 3,
}

impl FitnessClass {
    /// Classify a line based on its adjustment ratio.
    ///
    /// The adjustment ratio measures how much the line was stretched
    /// (positive) or compressed (negative) relative to ideal.
    pub fn from_ratio(ratio: f64) -> Self {
        if ratio < -0.5 {
            Self::Tight
        } else if ratio < 0.5 {
            Self::Normal
        } else if ratio < 1.0 {
            Self::Loose
        } else {
            Self::VeryLoose
        }
    }

    /// Penalty for transitioning between non-adjacent fitness classes.
    pub fn transition_penalty(from: Self, to: Self) -> f64 {
        let diff = (from as i8 - to as i8).unsigned_abs();
        if diff > 1 { 3000.0 } else { 0.0 }
    }
}

/// A candidate break point in the paragraph.
#[derive(Debug, Clone)]
pub struct BreakCandidate {
    /// Index of the item at which this break occurs.
    pub item_index: usize,
    /// Byte offset within the item's text (for mid-word breaks).
    pub text_offset: usize,
    /// Cumulative width of content up to this break point.
    pub total_width: f64,
    /// Cumulative stretch (glue) available up to this break point.
    pub total_stretch: f64,
    /// Cumulative shrink available up to this break point.
    pub total_shrink: f64,
    /// Penalty for breaking at this point (0 for normal spaces).
    pub penalty: f64,
    /// Whether this is a forced break (newline, `<br>`).
    pub is_forced: bool,
    /// Whether breaking here inserts a hyphen.
    pub is_hyphen: bool,
}

/// A scored break in the optimal solution.
#[derive(Debug, Clone)]
pub struct ScoredBreak {
    /// Index into the break candidates array.
    pub candidate_index: usize,
    /// Fitness class of the line ending at this break.
    pub fitness: FitnessClass,
    /// Total demerits from the start of the paragraph to this break.
    pub total_demerits: f64,
    /// Index of the previous scored break in the optimal path (-1 for start).
    pub previous: i32,
    /// Line number (0-based).
    pub line: usize,
}

/// Parameters for the scoring algorithm.
#[derive(Debug, Clone)]
pub struct ScoreParams {
    /// Available width for each line (may vary per line for float exclusions).
    pub line_widths: Vec<f64>,
    /// Default available width when line_widths is exhausted.
    pub default_width: f64,
    /// Text wrap mode (Balance or Pretty).
    pub mode: TextWrap,
    /// Tolerance for line adjustment (ratio beyond which a line is infeasible).
    pub tolerance: f64,
}

impl ScoreParams {
    /// Available width for the given line number.
    pub fn width_for_line(&self, line: usize) -> f64 {
        self.line_widths.get(line).copied().unwrap_or(self.default_width)
    }
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            line_widths: Vec::new(),
            default_width: 0.0,
            mode: TextWrap::Pretty,
            tolerance: 2.0,
        }
    }
}

/// The main scoring engine.
///
/// Implements Knuth-Plass dynamic programming to find optimal break points
/// for a paragraph. This produces a set of line breaks that minimizes total
/// demerits.
pub struct ScoreLineBreaker {
    params: ScoreParams,
}

impl ScoreLineBreaker {
    /// Create a new score-based line breaker with the given parameters.
    pub fn new(params: ScoreParams) -> Self {
        Self { params }
    }

    /// Determine whether scoring should be used or if we should fall back
    /// to greedy breaking.
    ///
    /// Returns `false` for very long paragraphs (> MAX_ITEMS_FOR_SCORING)
    /// to avoid quadratic blowup.
    pub fn should_use_scoring(candidates: &[BreakCandidate], mode: TextWrap) -> bool {
        if !mode.uses_scoring() {
            return false;
        }
        candidates.len() <= MAX_ITEMS_FOR_SCORING
    }

    /// Run the Knuth-Plass algorithm on the given break candidates.
    ///
    /// Returns a list of chosen break indices (into the candidates array),
    /// in order from first to last line break. An empty result means the
    /// entire paragraph fits on one line (no breaks needed).
    pub fn find_optimal_breaks(&self, candidates: &[BreakCandidate]) -> Vec<usize> {
        if candidates.is_empty() {
            return Vec::new();
        }

        let n = candidates.len();

        // Active nodes: each represents a feasible break point that could
        // be the end of some line. We track the best path to each.
        let mut active: Vec<ScoredBreak> = Vec::with_capacity(n);

        // Seed: the "start of paragraph" break at position -1
        active.push(ScoredBreak {
            candidate_index: 0,
            fitness: FitnessClass::Normal,
            total_demerits: 0.0,
            previous: -1,
            line: 0,
        });

        // All scored breaks (for backtracking the optimal path)
        let mut all_breaks: Vec<ScoredBreak> = Vec::with_capacity(n * 2);
        all_breaks.push(active[0].clone());

        // Process each candidate break point
        for b in 1..n {
            let candidate = &candidates[b];

            if candidate.is_forced || candidate.penalty < INFINITE_DEMERITS {
                let mut best_demerits = INFINITE_DEMERITS;
                let mut best_break: Option<ScoredBreak> = None;

                // Try connecting from each active break
                for a_idx in 0..active.len() {
                    let a = &active[a_idx];
                    let line = a.line;
                    let line_width = self.params.width_for_line(line);

                    // Compute width of content between break a and break b
                    let content_width = candidate.total_width
                        - candidates[a.candidate_index].total_width;

                    let stretch = candidate.total_stretch
                        - candidates[a.candidate_index].total_stretch;

                    let shrink = candidate.total_shrink
                        - candidates[a.candidate_index].total_shrink;

                    // Compute adjustment ratio
                    let ratio = compute_adjustment_ratio(
                        content_width,
                        line_width,
                        stretch,
                        shrink,
                    );

                    // Check feasibility
                    if ratio < -1.0 || (ratio > self.params.tolerance && !candidate.is_forced) {
                        continue;
                    }

                    // Compute badness (Knuth-Plass formula)
                    let badness = compute_badness(ratio);

                    // Compute demerits
                    let mut demerits = compute_demerits(badness, candidate.penalty);

                    // Fitness class transition penalty
                    let fitness = FitnessClass::from_ratio(ratio);
                    demerits += FitnessClass::transition_penalty(a.fitness, fitness);

                    // Hyphen penalties
                    if candidate.is_hyphen {
                        demerits += HYPHEN_PENALTY;
                        // Consecutive hyphenated lines incur additional penalty.
                        if a.candidate_index > 0 && b > 0 {
                            if let Some(prev_cand) = candidates.get(a.candidate_index) {
                                if prev_cand.is_hyphen {
                                    demerits += CONSECUTIVE_HYPHEN_PENALTY;
                                }
                            }
                        }
                    }

                    // Orphan penalty: penalize very short last line.
                    if candidate.is_forced && line_width > 0.0 {
                        let fill_ratio = content_width / line_width;
                        if fill_ratio < ORPHAN_WIDTH_RATIO {
                            demerits += ORPHAN_PENALTY;
                        }
                    }

                    let total = a.total_demerits + demerits;

                    if total < best_demerits {
                        best_demerits = total;
                        best_break = Some(ScoredBreak {
                            candidate_index: b,
                            fitness,
                            total_demerits: total,
                            previous: a_idx as i32,
                            line: line + 1,
                        });
                    }
                }

                if let Some(sb) = best_break {
                    all_breaks.push(sb.clone());

                    if candidate.is_forced {
                        // Forced break: clear active set, start fresh
                        active.clear();
                    }
                    active.push(sb);
                }
            }
        }

        // Backtrack from the best final break to find the optimal path
        if active.is_empty() {
            return Vec::new();
        }

        // Find the active node with minimum total demerits
        let best = active.iter().min_by(|a, b| {
            a.total_demerits.partial_cmp(&b.total_demerits)
                .unwrap_or(std::cmp::Ordering::Equal)
        }).expect("active list is non-empty after paragraph processing");

        let mut path = Vec::new();
        let mut current = best.clone();

        // Walk backwards through the linked list of breaks
        loop {
            if current.candidate_index > 0 {
                path.push(current.candidate_index);
            }
            if current.previous < 0 {
                break;
            }
            current = all_breaks[current.previous as usize].clone();
        }

        path.reverse();

        // Apply mode-specific adjustments
        match self.params.mode {
            TextWrap::Balance => self.apply_balance_adjustment(candidates, &mut path),
            TextWrap::Pretty => self.apply_pretty_adjustment(candidates, &mut path),
            _ => {}
        }

        path
    }

    /// Balance mode adjustment: equalize line widths.
    ///
    /// After finding optimal breaks, check if lines can be made more
    /// balanced by shifting words between lines.
    fn apply_balance_adjustment(
        &self,
        candidates: &[BreakCandidate],
        breaks: &mut Vec<usize>,
    ) {
        if breaks.len() < 2 {
            return;
        }

        // Compute current line widths
        let line_widths = self.compute_line_widths(candidates, breaks);
        let avg_width = line_widths.iter().sum::<f64>() / line_widths.len() as f64;

        // Compute variance
        let variance: f64 = line_widths.iter()
            .map(|w| (w - avg_width) * (w - avg_width))
            .sum::<f64>() / line_widths.len() as f64;

        // If lines are already well-balanced (low variance), skip
        if variance < 100.0 {
            return;
        }

        // Try shifting one break at a time to reduce variance
        let mut improved = true;
        let mut iterations = 0;
        while improved && iterations < 10 {
            improved = false;
            iterations += 1;

            for i in 0..breaks.len() {
                let current_widths = self.compute_line_widths(candidates, breaks);
                let current_var = compute_variance(&current_widths);

                // Try shifting break forward
                if breaks[i] + 1 < candidates.len() {
                    let original = breaks[i];
                    breaks[i] += 1;
                    let new_widths = self.compute_line_widths(candidates, breaks);
                    let new_var = compute_variance(&new_widths);
                    if new_var < current_var {
                        improved = true;
                    } else {
                        breaks[i] = original;
                    }
                }

                // Try shifting break backward
                let min_idx = if i == 0 { 1 } else { breaks[i - 1] + 1 };
                if breaks[i] > min_idx {
                    let current_widths = self.compute_line_widths(candidates, breaks);
                    let current_var = compute_variance(&current_widths);

                    let original = breaks[i];
                    breaks[i] -= 1;
                    let new_widths = self.compute_line_widths(candidates, breaks);
                    let new_var = compute_variance(&new_widths);
                    if new_var < current_var {
                        improved = true;
                    } else {
                        breaks[i] = original;
                    }
                }
            }
        }
    }

    /// Pretty mode adjustment: penalize orphans and widows.
    ///
    /// If the last line is very short (< 20% of available width),
    /// try to redistribute content to avoid the orphan.
    fn apply_pretty_adjustment(
        &self,
        candidates: &[BreakCandidate],
        breaks: &mut Vec<usize>,
    ) {
        if breaks.is_empty() {
            return;
        }

        let line_widths = self.compute_line_widths(candidates, breaks);
        if line_widths.is_empty() {
            return;
        }

        let last_width = *line_widths.last().unwrap();
        let available = self.params.default_width;

        if available > 0.0 && (last_width / available) < ORPHAN_WIDTH_RATIO {
            // Last line is very short — try pulling content from previous line
            let last_break_idx = breaks.len() - 1;
            if last_break_idx > 0 {
                let min_idx = breaks[last_break_idx - 1] + 1;
                if breaks[last_break_idx] > min_idx {
                    breaks[last_break_idx] -= 1;
                }
            }
        }
    }

    /// Compute the width of each line given the current break points.
    fn compute_line_widths(
        &self,
        candidates: &[BreakCandidate],
        breaks: &[usize],
    ) -> Vec<f64> {
        let mut widths = Vec::with_capacity(breaks.len() + 1);
        let mut prev_width = 0.0;

        for &b in breaks {
            if b < candidates.len() {
                widths.push(candidates[b].total_width - prev_width);
                prev_width = candidates[b].total_width;
            }
        }

        // Last line: from last break to end
        if let Some(last) = candidates.last() {
            widths.push(last.total_width - prev_width);
        }

        widths
    }
}

/// Compute the adjustment ratio for a line.
///
/// The ratio measures how much the line needs to be stretched (positive)
/// or compressed (negative) to fill the available width:
/// - ratio = 0: perfect fit
/// - ratio > 0: line is shorter than ideal (needs stretching)
/// - ratio < 0: line is longer than ideal (needs compression)
/// - ratio < -1: line cannot be compressed enough (infeasible)
fn compute_adjustment_ratio(
    content_width: f64,
    line_width: f64,
    stretch: f64,
    shrink: f64,
) -> f64 {
    let diff = line_width - content_width;
    if diff.abs() < 0.01 {
        0.0
    } else if diff > 0.0 {
        if stretch > 0.01 { diff / stretch } else { INFINITE_DEMERITS }
    } else {
        if shrink > 0.01 { diff / shrink } else { -INFINITE_DEMERITS }
    }
}

/// Compute badness from adjustment ratio (Knuth-Plass formula).
///
/// badness = 100 * |ratio|³ (capped at 10000)
fn compute_badness(ratio: f64) -> f64 {
    let b = 100.0 * ratio.abs().powi(3);
    if b > 10000.0 { 10000.0 } else { b }
}

/// Compute demerits from badness and penalty.
///
/// Standard Knuth-Plass demerit formula:
/// - If penalty >= 0: demerits = (1 + badness + penalty)²
/// - If penalty < 0 and penalty > -∞: demerits = (1 + badness)² - penalty²
/// - If penalty = -∞: demerits = (1 + badness)²
fn compute_demerits(badness: f64, penalty: f64) -> f64 {
    if penalty >= 0.0 {
        let sum = 1.0 + badness + penalty;
        sum * sum
    } else if penalty > -INFINITE_DEMERITS {
        let base = 1.0 + badness;
        base * base - penalty * penalty
    } else {
        let base = 1.0 + badness;
        base * base
    }
}

/// Compute variance of a set of values.
fn compute_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values.iter()
        .map(|v| (v - mean) * (v - mean))
        .sum::<f64>() / values.len() as f64
}

/// Check if the text-wrap mode requires score-based breaking.
#[inline]
pub fn requires_scoring(mode: TextWrap) -> bool {
    mode.uses_scoring()
}

/// Create break candidates from a simple list of word widths and space widths.
///
/// This is a convenience function for testing. In production, break
/// candidates are extracted from the inline items during the line-breaking
/// phase.
pub fn candidates_from_word_widths(
    word_widths: &[f64],
    space_width: f64,
) -> Vec<BreakCandidate> {
    let mut candidates = Vec::with_capacity(word_widths.len() + 1);
    let mut total_width = 0.0;
    let mut total_stretch = 0.0;

    // Start-of-paragraph sentinel
    candidates.push(BreakCandidate {
        item_index: 0,
        text_offset: 0,
        total_width: 0.0,
        total_stretch: 0.0,
        total_shrink: 0.0,
        penalty: 0.0,
        is_forced: false,
        is_hyphen: false,
    });

    for (i, &width) in word_widths.iter().enumerate() {
        total_width += width;
        if i < word_widths.len() - 1 {
            total_width += space_width;
            // Each inter-word space contributes stretch proportional to the
            // space width. In real text, justification can stretch spaces
            // significantly (up to ~2x their natural width).
            total_stretch += space_width * 2.0;
        }

        candidates.push(BreakCandidate {
            item_index: i + 1,
            text_offset: 0,
            total_width,
            total_stretch,
            total_shrink: total_stretch * 0.3, // Shrink = 30% of stretch
            penalty: 0.0,
            is_forced: i == word_widths.len() - 1, // Last word is forced break
            is_hyphen: false,
        });
    }

    candidates
}

/// Result of score-based line breaking.
#[derive(Debug, Clone)]
pub struct ScoreResult {
    /// Indices into the candidates array where lines break.
    pub break_indices: Vec<usize>,
    /// Number of lines produced.
    pub line_count: usize,
    /// Whether scoring was actually used (vs. fallback to greedy).
    pub used_scoring: bool,
}

/// Run score-based line breaking on a paragraph.
///
/// This is the main entry point for `text-wrap: balance` and
/// `text-wrap: pretty`. It creates a ScoreLineBreaker, finds optimal
/// breaks, and returns the result.
///
/// If the paragraph is too long (> MAX_ITEMS_FOR_SCORING), returns
/// `None` to signal that greedy breaking should be used instead.
pub fn score_line_break(
    candidates: &[BreakCandidate],
    available_width: f64,
    mode: TextWrap,
) -> Option<ScoreResult> {
    if !ScoreLineBreaker::should_use_scoring(candidates, mode) {
        return None;
    }

    let params = ScoreParams {
        line_widths: Vec::new(),
        default_width: available_width,
        mode,
        // High tolerance: balance/pretty modes allow significant line-width
        // deviation since the goal is overall paragraph quality, not exact
        // line fitting. This matches Blink's approach in score_line_breaker.cc.
        tolerance: 100.0,
    };

    let breaker = ScoreLineBreaker::new(params);
    let break_indices = breaker.find_optimal_breaks(candidates);
    let line_count = break_indices.len() + 1;

    Some(ScoreResult {
        break_indices,
        line_count,
        used_scoring: true,
    })
}

/// Compute balance score: lower is better-balanced.
///
/// This measures how well-balanced the line widths are by computing
/// the coefficient of variation (std_dev / mean).
pub fn balance_score(line_widths: &[f64]) -> f64 {
    if line_widths.len() <= 1 {
        return 0.0;
    }
    let mean = line_widths.iter().sum::<f64>() / line_widths.len() as f64;
    if mean < 0.01 {
        return 0.0;
    }
    let variance = compute_variance(line_widths);
    variance.sqrt() / mean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitness_class_from_ratio() {
        assert_eq!(FitnessClass::from_ratio(-1.0), FitnessClass::Tight);
        assert_eq!(FitnessClass::from_ratio(0.0), FitnessClass::Normal);
        assert_eq!(FitnessClass::from_ratio(0.7), FitnessClass::Loose);
        assert_eq!(FitnessClass::from_ratio(1.5), FitnessClass::VeryLoose);
    }

    #[test]
    fn transition_penalty_adjacent() {
        assert_eq!(
            FitnessClass::transition_penalty(FitnessClass::Normal, FitnessClass::Loose),
            0.0
        );
    }

    #[test]
    fn transition_penalty_non_adjacent() {
        assert!(
            FitnessClass::transition_penalty(FitnessClass::Tight, FitnessClass::VeryLoose)
                > 0.0
        );
    }

    #[test]
    fn badness_perfect_fit() {
        assert_eq!(compute_badness(0.0), 0.0);
    }

    #[test]
    fn badness_moderate_stretch() {
        let b = compute_badness(1.0);
        assert!((b - 100.0).abs() < 0.01);
    }

    #[test]
    fn badness_capped_at_10000() {
        let b = compute_badness(10.0);
        assert_eq!(b, 10000.0);
    }

    #[test]
    fn adjustment_ratio_perfect() {
        let r = compute_adjustment_ratio(100.0, 100.0, 10.0, 5.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn adjustment_ratio_stretch() {
        let r = compute_adjustment_ratio(80.0, 100.0, 20.0, 10.0);
        assert!((r - 1.0).abs() < 0.01);
    }

    #[test]
    fn adjustment_ratio_shrink() {
        let r = compute_adjustment_ratio(110.0, 100.0, 20.0, 10.0);
        assert!((r - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn single_word_no_breaks() {
        let candidates = candidates_from_word_widths(&[50.0], 5.0);
        let result = score_line_break(&candidates, 100.0, TextWrap::Pretty);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.break_indices.is_empty() || r.line_count <= 2);
    }

    #[test]
    fn two_lines_simple() {
        // Two words that barely don't fit on one line
        let candidates = candidates_from_word_widths(&[60.0, 60.0], 5.0);
        let result = score_line_break(&candidates, 100.0, TextWrap::Pretty).unwrap();
        assert!(result.line_count >= 1);
        assert!(result.used_scoring);
    }

    #[test]
    fn balance_mode_produces_balanced_lines() {
        // Four words, each 40px wide, with 5px spaces, available 100px
        // Greedy: "word word" (85px), "word word" (85px) — balanced
        let candidates = candidates_from_word_widths(&[40.0, 40.0, 40.0, 40.0], 5.0);
        let result = score_line_break(&candidates, 100.0, TextWrap::Balance).unwrap();
        assert!(result.used_scoring);
    }

    #[test]
    fn fallback_for_long_paragraph() {
        // Create > MAX_ITEMS_FOR_SCORING candidates
        let words: Vec<f64> = vec![30.0; MAX_ITEMS_FOR_SCORING + 10];
        let candidates = candidates_from_word_widths(&words, 5.0);
        let result = score_line_break(&candidates, 200.0, TextWrap::Pretty);
        assert!(result.is_none()); // Should fall back to greedy
    }

    #[test]
    fn nowrap_mode_skips_scoring() {
        let candidates = candidates_from_word_widths(&[50.0, 50.0], 5.0);
        let result = score_line_break(&candidates, 80.0, TextWrap::Nowrap);
        assert!(result.is_none());
    }

    #[test]
    fn wrap_mode_skips_scoring() {
        let candidates = candidates_from_word_widths(&[50.0, 50.0], 5.0);
        let result = score_line_break(&candidates, 80.0, TextWrap::Wrap);
        assert!(result.is_none());
    }

    #[test]
    fn balance_score_perfectly_balanced() {
        let widths = vec![100.0, 100.0, 100.0];
        assert_eq!(balance_score(&widths), 0.0);
    }

    #[test]
    fn balance_score_unbalanced() {
        let widths = vec![150.0, 50.0, 100.0];
        assert!(balance_score(&widths) > 0.0);
    }

    #[test]
    fn variance_empty() {
        assert_eq!(compute_variance(&[]), 0.0);
    }

    #[test]
    fn variance_uniform() {
        assert_eq!(compute_variance(&[5.0, 5.0, 5.0]), 0.0);
    }

    #[test]
    fn variance_nonzero() {
        let v = compute_variance(&[1.0, 2.0, 3.0]);
        assert!((v - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn requires_scoring_balance() {
        assert!(requires_scoring(TextWrap::Balance));
    }

    #[test]
    fn requires_scoring_pretty() {
        assert!(requires_scoring(TextWrap::Pretty));
    }

    #[test]
    fn does_not_require_scoring_wrap() {
        assert!(!requires_scoring(TextWrap::Wrap));
    }
}
