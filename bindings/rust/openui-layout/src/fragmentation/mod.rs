//! Block fragmentation — break tokens, appeal ranking, and fragmentainer space.
//!
//! Ported from Blink's `BlockBreakToken` (`block_break_token.h`) and
//! `BreakAppeal` (`break_appeal.h`) system.
//!
//! Block fragmentation allows content to be split across multiple pages (print)
//! or columns (multicol). The break token system tracks where layout was
//! interrupted so it can resume in the next fragmentainer.
//!
//! Key concepts:
//! - **`BlockBreakToken`**: Records which child was being laid out when the
//!   fragmentainer ran out of space, how much block size was consumed, and
//!   child-level break tokens for nested fragmentation.
//! - **`BreakAppeal`**: Ranks the desirability of a break point (Blink's
//!   `kBreakAppeal*` constants). Higher = better.
//! - **`FragmentainerSpace`**: Tracks available space in the current
//!   fragmentainer column/page.

use openui_geometry::LayoutUnit;
use openui_style::{BreakValue, BreakInside};
use crate::layout_result::BreakBetween;

// ── Break Token ─────────────────────────────────────────────────────────

/// Tracks where layout was interrupted in a block formatting context.
///
/// Source: `NGBlockBreakToken` in Blink (`block_break_token.h`).
///
/// When a block box doesn't fit entirely in its fragmentainer (page/column),
/// layout produces a `BlockBreakToken` recording the point of interruption.
/// The next fragmentainer uses this token to resume layout from where it
/// left off.
#[derive(Debug, Clone)]
pub struct BlockBreakToken {
    /// Index of the child where the break occurred.
    /// Layout resumes from this child in the next fragmentainer.
    pub child_index: usize,

    /// Block size consumed in the current fragmentainer before the break.
    /// Used to calculate remaining space and offsets in the next fragment.
    pub consumed_block_size: LayoutUnit,

    /// Break tokens for child elements that were also fragmented.
    /// Enables nested fragmentation (e.g., a column inside a page break).
    pub child_break_tokens: Vec<BreakToken>,

    /// Whether this is a break-before token (break occurs before the child
    /// at `child_index` rather than after/during it).
    pub is_break_before: bool,
}

impl BlockBreakToken {
    /// Create a new break token at the given child index.
    pub fn new(child_index: usize, consumed_block_size: LayoutUnit) -> Self {
        Self {
            child_index,
            consumed_block_size,
            child_break_tokens: Vec::new(),
            is_break_before: false,
        }
    }

    /// Create a break-before token (the break occurs before this child).
    pub fn break_before(child_index: usize, consumed_block_size: LayoutUnit) -> Self {
        Self {
            child_index,
            consumed_block_size,
            child_break_tokens: Vec::new(),
            is_break_before: true,
        }
    }

    /// Add a child break token for nested fragmentation.
    pub fn add_child_token(&mut self, token: BreakToken) {
        self.child_break_tokens.push(token);
    }

    /// Whether any children were also fragmented.
    pub fn has_child_break_tokens(&self) -> bool {
        !self.child_break_tokens.is_empty()
    }
}

/// A break token variant — currently only block-level, but inline break
/// tokens will be added in future SPs.
///
/// Source: `NGBreakToken` hierarchy in Blink.
#[derive(Debug, Clone)]
pub enum BreakToken {
    /// Break token from a block-level element.
    Block(BlockBreakToken),
    // Inline break tokens will be added in future SPs.
}

// ── Break Appeal ────────────────────────────────────────────────────────

/// Ranks the desirability of a break point.
///
/// Source: `BreakAppeal` in Blink (`break_appeal.h`).
///
/// When a fragmentainer overflows, Blink evaluates all possible break points
/// and selects the one with the highest appeal. The ordering is:
///
/// `Perfect > Default > Violating > LastResort`
///
/// - **`Perfect`**: An explicit forced break (`break-before: page`, etc.).
/// - **`Default`**: A normal class-B break point with no special preference.
/// - **`Violating`**: Breaking here violates a soft preference but is allowed.
/// - **`LastResort`**: Breaking here violates `break-avoid` or would break
///   inside an element with `break-inside: avoid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BreakAppeal {
    /// Breaking here would violate a `break-avoid` request. Only used as a
    /// last resort when no better break point exists.
    LastResort = 0,
    /// Breaking here is possible but violates a soft preference (e.g.,
    /// orphans/widows constraints).
    Violating = 1,
    /// Default — no special preference for or against breaking here.
    Default = 2,
    /// Breaking here is explicitly requested (`break-before: always`,
    /// `break-before: page`, `break-after: column`, etc.).
    Perfect = 3,
}

impl BreakAppeal {
    /// Whether this appeal indicates a forced (perfect) break.
    #[inline]
    pub fn is_forced(self) -> bool {
        self == Self::Perfect
    }

    /// Whether this appeal is better than or equal to `Default`.
    #[inline]
    pub fn is_acceptable(self) -> bool {
        self >= Self::Default
    }
}

// ── Fragmentainer Space ─────────────────────────────────────────────────

/// Tracks available space in the current fragmentainer (page or column).
///
/// Source: derived from `NGFragmentainerSpaceAtBfcStart` and
/// `NGConstraintSpace` fragmentation fields in Blink.
#[derive(Debug, Clone, Copy)]
pub struct FragmentainerSpace {
    /// Total block size of this fragmentainer.
    pub block_size: LayoutUnit,

    /// How much block size has been consumed so far.
    pub block_offset: LayoutUnit,

    /// Whether we are at the very start of this fragmentainer (no content
    /// has been placed yet). This affects orphan/widow handling and
    /// whether a break-before is honoured or deferred.
    pub is_at_block_start: bool,
}

impl FragmentainerSpace {
    /// Create a new fragmentainer space.
    pub fn new(block_size: LayoutUnit) -> Self {
        Self {
            block_size,
            block_offset: LayoutUnit::zero(),
            is_at_block_start: true,
        }
    }

    /// Remaining block space in this fragmentainer.
    #[inline]
    pub fn remaining(&self) -> LayoutUnit {
        self.block_size - self.block_offset
    }

    /// Whether this fragmentainer is exhausted (no remaining space).
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.remaining() <= LayoutUnit::zero()
    }

    /// Consume some block space, updating the offset.
    pub fn consume(&mut self, amount: LayoutUnit) {
        self.block_offset = self.block_offset + amount;
        self.is_at_block_start = false;
    }
}

// ── Break point selection ───────────────────────────────────────────────

/// Information about a child relevant to break-point evaluation.
#[derive(Debug, Clone)]
pub struct ChildBreakInfo {
    /// CSS `break-before` on this child.
    pub break_before: BreakValue,
    /// CSS `break-after` on this child.
    pub break_after: BreakValue,
    /// CSS `break-inside` on this child.
    pub break_inside: BreakInside,
    /// Block size of this child.
    pub block_size: LayoutUnit,
}

/// Result of `find_best_break_point`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakPoint {
    /// Index of the child *before which* the break occurs. A value equal to
    /// `children.len()` means all children fit (no break needed).
    pub child_index: usize,
    /// The appeal of this break point.
    pub appeal: BreakAppeal,
}

/// Find the best break point among `children` given the available `space`.
///
/// This implements a simplified version of Blink's break-point selection
/// algorithm:
///
/// 1. Walk children, accumulating block size.
/// 2. At each child boundary, evaluate break appeal:
///    - `break-before: always|page|column` → `Perfect` (forced).
///    - Previous child's `break-after: always|page|column` → `Perfect`.
///    - `break-inside: avoid` on the previous child demotes a break
///      *inside* to `LastResort` — but a break *between* children is still
///      `Default` unless `break-before/after: avoid`.
///    - `break-before: avoid` or previous `break-after: avoid` → `LastResort`.
///    - Otherwise → `Default`.
/// 3. If no break is needed (all content fits), return index ==
///    `children.len()` with `Default` appeal.
/// 4. Among break points at or before the fragmentainer boundary, pick the
///    one with the highest appeal (ties broken by latest index — prefer
///    placing more content in this fragmentainer).
pub fn find_best_break_point(
    children: &[ChildBreakInfo],
    space: &FragmentainerSpace,
) -> BreakPoint {
    if children.is_empty() {
        return BreakPoint {
            child_index: 0,
            appeal: BreakAppeal::Default,
        };
    }

    let mut accumulated = LayoutUnit::zero();
    let remaining = space.remaining();

    // Check if all content fits.
    let total: LayoutUnit = children.iter().fold(LayoutUnit::zero(), |acc, c| acc + c.block_size);
    let all_fits = total <= remaining;

    // First pass: check for forced breaks (honoured even when content fits).
    for i in 1..children.len() {
        if children[i].break_before.is_forced() || children[i - 1].break_after.is_forced() {
            return BreakPoint {
                child_index: i,
                appeal: BreakAppeal::Perfect,
            };
        }
    }

    // If all content fits and there are no forced breaks, no break needed.
    if all_fits {
        return BreakPoint {
            child_index: children.len(),
            appeal: BreakAppeal::Default,
        };
    }

    // Collect candidate break points (between children).
    // A break "before child i" means children 0..i go in this fragmentainer.
    let mut best: Option<BreakPoint> = None;

    for i in 0..children.len() {
        // Check break-before on child i (break between child i-1 and child i).
        if i > 0 {
            let appeal = appeal_between(
                &children[i - 1],
                &children[i],
            );

            let candidate = BreakPoint {
                child_index: i,
                appeal,
            };

            // Only consider breaks at or before the fragmentainer boundary.
            if accumulated <= remaining {
                best = Some(pick_better(best, candidate));
            }
        }

        accumulated = accumulated + children[i].block_size;

        // If we've exceeded the fragmentainer and haven't picked a break yet,
        // consider a break right before this child that overflows.
        if accumulated > remaining && i > 0 {
            if best.is_none() {
                best = Some(BreakPoint {
                    child_index: i,
                    appeal: appeal_between(
                        &children[i - 1],
                        &children[i],
                    ),
                });
            }
            break;
        }
    }

    // If accumulated exceeds remaining and the break is before the first child
    // (single child overflows), break before child 1 or force at 1.
    best.unwrap_or(BreakPoint {
        child_index: 1.min(children.len()),
        appeal: BreakAppeal::LastResort,
    })
}

/// Compute the break appeal for a break *between* `prev` and `next`.
fn appeal_between(prev: &ChildBreakInfo, next: &ChildBreakInfo) -> BreakAppeal {
    // Forced break?
    if next.break_before.is_forced() {
        return BreakAppeal::Perfect;
    }
    if prev.break_after.is_forced() {
        return BreakAppeal::Perfect;
    }

    // Avoid break?
    if next.break_before.is_avoid() || prev.break_after.is_avoid() {
        return BreakAppeal::LastResort;
    }

    BreakAppeal::Default
}

/// Pick the better of two break point candidates, preferring higher appeal
/// and then later index (more content in this fragmentainer).
fn pick_better(current: Option<BreakPoint>, candidate: BreakPoint) -> BreakPoint {
    match current {
        None => candidate,
        Some(cur) => {
            if candidate.appeal > cur.appeal {
                candidate
            } else if candidate.appeal == cur.appeal && candidate.child_index >= cur.child_index {
                candidate
            } else {
                cur
            }
        }
    }
}

// ── Break-before / break-after helpers ──────────────────────────────────

/// Whether to force a break before an element with the given `break_before` value.
///
/// Maps CSS `break-before` values:
/// - `always`, `page`, `column`, `left`, `right` → `true`
/// - `auto`, `avoid`, `avoid-page`, `avoid-column` → `false`
pub fn should_break_before(value: BreakValue) -> bool {
    value.is_forced()
}

/// Whether to force a break after an element with the given `break_after` value.
///
/// Same mapping as `should_break_before` but applied to the `break-after` property.
pub fn should_break_after(value: BreakValue) -> bool {
    value.is_forced()
}

/// Compute the break appeal for a break-before with the given value.
pub fn break_before_appeal(value: BreakValue) -> BreakAppeal {
    if value.is_forced() {
        BreakAppeal::Perfect
    } else if value.is_avoid() {
        BreakAppeal::LastResort
    } else {
        BreakAppeal::Default
    }
}

/// Compute the break appeal for a break-after with the given value.
pub fn break_after_appeal(value: BreakValue) -> BreakAppeal {
    if value.is_forced() {
        BreakAppeal::Perfect
    } else if value.is_avoid() {
        BreakAppeal::LastResort
    } else {
        BreakAppeal::Default
    }
}

// ── BreakBetween join/merge ─────────────────────────────────────────────

/// Join two `BreakBetween` values at a boundary (e.g., `break-after` of one
/// child and `break-before` of the next). The result is the strongest
/// applicable value.
///
/// Source: `JoinFragmentainerBreakValues()` in Blink
/// (`break_token.cc`).
///
/// Rules (simplified):
/// - Forced breaks win over avoids and auto.
/// - Among forced breaks, more specific wins (left/right > page > column).
/// - `Avoid` wins over `Auto`.
/// - `AvoidPage` / `AvoidColumn` are more specific avoids.
pub fn join_break_between(a: BreakBetween, b: BreakBetween) -> BreakBetween {
    // If either is auto, the other wins.
    if a == BreakBetween::Auto {
        return b;
    }
    if b == BreakBetween::Auto {
        return a;
    }

    // Forced break values (order by specificity: Left/Right > Page > Column)
    let a_force = break_between_force_rank(a);
    let b_force = break_between_force_rank(b);

    if a_force > 0 || b_force > 0 {
        // Take the stronger forced break.
        return if a_force >= b_force { a } else { b };
    }

    // Both are avoid-type — take the more specific.
    let a_avoid = break_between_avoid_rank(a);
    let b_avoid = break_between_avoid_rank(b);
    if a_avoid >= b_avoid { a } else { b }
}

/// Rank forced break values (0 = not forced).
fn break_between_force_rank(v: BreakBetween) -> u8 {
    match v {
        BreakBetween::Column => 1,
        BreakBetween::Page => 2,
        BreakBetween::Recto | BreakBetween::Verso => 3,
        BreakBetween::Left | BreakBetween::Right => 4,
        _ => 0,
    }
}

/// Rank avoid values (0 = not avoid).
fn break_between_avoid_rank(v: BreakBetween) -> u8 {
    match v {
        BreakBetween::Avoid => 1,
        BreakBetween::AvoidColumn => 2,
        BreakBetween::AvoidPage => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmentainer_remaining() {
        let space = FragmentainerSpace {
            block_size: LayoutUnit::from_i32(600),
            block_offset: LayoutUnit::from_i32(200),
            is_at_block_start: false,
        };
        assert_eq!(space.remaining(), LayoutUnit::from_i32(400));
    }

    #[test]
    fn fragmentainer_exhausted() {
        let space = FragmentainerSpace {
            block_size: LayoutUnit::from_i32(100),
            block_offset: LayoutUnit::from_i32(100),
            is_at_block_start: false,
        };
        assert!(space.is_exhausted());
    }

    #[test]
    fn fragmentainer_not_exhausted() {
        let space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
        assert!(!space.is_exhausted());
        assert!(space.is_at_block_start);
    }

    #[test]
    fn fragmentainer_consume() {
        let mut space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
        space.consume(LayoutUnit::from_i32(100));
        assert_eq!(space.remaining(), LayoutUnit::from_i32(400));
        assert!(!space.is_at_block_start);
    }

    #[test]
    fn break_appeal_ordering() {
        assert!(BreakAppeal::Perfect > BreakAppeal::Default);
        assert!(BreakAppeal::Default > BreakAppeal::Violating);
        assert!(BreakAppeal::Violating > BreakAppeal::LastResort);
        assert!(BreakAppeal::Perfect > BreakAppeal::LastResort);
    }

    #[test]
    fn break_appeal_methods() {
        assert!(BreakAppeal::Perfect.is_forced());
        assert!(!BreakAppeal::Default.is_forced());
        assert!(BreakAppeal::Default.is_acceptable());
        assert!(!BreakAppeal::LastResort.is_acceptable());
    }

    #[test]
    fn block_break_token_creation() {
        let token = BlockBreakToken::new(3, LayoutUnit::from_i32(200));
        assert_eq!(token.child_index, 3);
        assert_eq!(token.consumed_block_size, LayoutUnit::from_i32(200));
        assert!(!token.is_break_before);
        assert!(!token.has_child_break_tokens());
    }

    #[test]
    fn block_break_token_break_before() {
        let token = BlockBreakToken::break_before(0, LayoutUnit::zero());
        assert!(token.is_break_before);
        assert_eq!(token.child_index, 0);
    }

    #[test]
    fn should_break_before_values() {
        assert!(should_break_before(BreakValue::Always));
        assert!(should_break_before(BreakValue::Page));
        assert!(should_break_before(BreakValue::Column));
        assert!(should_break_before(BreakValue::Left));
        assert!(should_break_before(BreakValue::Right));
        assert!(!should_break_before(BreakValue::Auto));
        assert!(!should_break_before(BreakValue::Avoid));
        assert!(!should_break_before(BreakValue::AvoidPage));
        assert!(!should_break_before(BreakValue::AvoidColumn));
    }

    #[test]
    fn should_break_after_values() {
        assert!(should_break_after(BreakValue::Always));
        assert!(should_break_after(BreakValue::Page));
        assert!(should_break_after(BreakValue::Column));
        assert!(!should_break_after(BreakValue::Auto));
        assert!(!should_break_after(BreakValue::Avoid));
    }

    #[test]
    fn join_break_between_forced_wins() {
        let result = join_break_between(BreakBetween::Page, BreakBetween::Avoid);
        assert_eq!(result, BreakBetween::Page);
    }

    #[test]
    fn join_break_between_auto_passthrough() {
        assert_eq!(
            join_break_between(BreakBetween::Auto, BreakBetween::Page),
            BreakBetween::Page,
        );
        assert_eq!(
            join_break_between(BreakBetween::Avoid, BreakBetween::Auto),
            BreakBetween::Avoid,
        );
    }

    #[test]
    fn join_break_between_stronger_forced() {
        // Left/Right (rank 4) > Page (rank 2)
        assert_eq!(
            join_break_between(BreakBetween::Left, BreakBetween::Page),
            BreakBetween::Left,
        );
        // Page (rank 2) > Column (rank 1)
        assert_eq!(
            join_break_between(BreakBetween::Column, BreakBetween::Page),
            BreakBetween::Page,
        );
    }

    #[test]
    fn join_break_between_avoid_specificity() {
        // AvoidPage (rank 3) > Avoid (rank 1)
        assert_eq!(
            join_break_between(BreakBetween::Avoid, BreakBetween::AvoidPage),
            BreakBetween::AvoidPage,
        );
    }
}
