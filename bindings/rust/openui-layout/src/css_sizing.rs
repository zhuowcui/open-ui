//! CSS Sizing Module Level 3 — keywords, aspect-ratio, and definite size resolution.
//!
//! Implements the sizing algorithms from CSS Sizing Level 3 (§5):
//! - Sizing keywords: `min-content`, `max-content`, `fit-content(X)`, `stretch`
//! - Aspect ratio application
//! - Preferred size resolution (§5.1)
//! - Definite size computation
//! - Automatic size computation (§5.2)
//!
//! References:
//! - <https://drafts.csswg.org/css-sizing-3/>
//! - Blink: `core/layout/length_utils.cc`, `core/layout/geometry/axis.h`

use openui_geometry::{LayoutUnit, Length, LengthType, MinMaxSizes, INDEFINITE_SIZE};
use openui_style::AspectRatio;

use crate::constraint_space::ConstraintSpace;

// ── SizingKeyword ────────────────────────────────────────────────────

/// A resolved CSS sizing keyword.
///
/// CSS Sizing Level 3 defines several intrinsic and extrinsic sizing keywords
/// that can appear as values for `width`, `height`, `min-*`, and `max-*`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizingKeyword {
    /// `auto` — context-dependent sizing.
    Auto,
    /// `min-content` — the smallest size that avoids overflow.
    MinContent,
    /// `max-content` — the ideal size with no line breaks.
    MaxContent,
    /// `fit-content(X)` — clamp between min-content and max-content at X.
    FitContent(LayoutUnit),
    /// `stretch` (formerly `fill-available`) — fill the available space.
    Stretch,
}

impl SizingKeyword {
    /// Create a `SizingKeyword` from a CSS `Length` value.
    ///
    /// Returns `None` if the length is a concrete value (fixed, percent) rather
    /// than a sizing keyword.
    pub fn from_length(length: &Length) -> Option<Self> {
        match length.length_type() {
            LengthType::Auto => Some(SizingKeyword::Auto),
            LengthType::MinContent => Some(SizingKeyword::MinContent),
            LengthType::MaxContent => Some(SizingKeyword::MaxContent),
            LengthType::FitContent => {
                Some(SizingKeyword::FitContent(LayoutUnit::from_f32(length.value())))
            }
            LengthType::Stretch => Some(SizingKeyword::Stretch),
            _ => None,
        }
    }
}

// ── resolve_sizing_keyword ───────────────────────────────────────────

/// Resolve a sizing keyword to a concrete `LayoutUnit` value.
///
/// # Arguments
///
/// * `keyword` — the sizing keyword to resolve.
/// * `intrinsic` — the min-content / max-content pair for this element.
/// * `available_size` — the available space in the relevant axis.
/// * `margins` — total margins in the relevant axis (used for `stretch`).
///
/// # CSS Sizing L3 §5
///
/// - `min-content` → `intrinsic.min`
/// - `max-content` → `intrinsic.max`
/// - `fit-content(X)` → `clamp(min-content, X, max-content)`
/// - `stretch` → `available_size - margins` (clamped to 0)
/// - `auto` → falls back to stretch in inline axis, fit-content in block axis.
///   Callers should use `compute_automatic_size` for full auto handling.
pub fn resolve_sizing_keyword(
    keyword: SizingKeyword,
    intrinsic: &MinMaxSizes,
    available_size: LayoutUnit,
    margins: LayoutUnit,
) -> LayoutUnit {
    match keyword {
        SizingKeyword::MinContent => intrinsic.min,
        SizingKeyword::MaxContent => intrinsic.max,
        SizingKeyword::FitContent(limit) => {
            // fit-content(X) = clamp(min-content, X, max-content)
            let clamped = limit.clamp(intrinsic.min, intrinsic.max);
            clamped
        }
        SizingKeyword::Stretch => {
            // stretch = available - margins, clamped to 0
            let result = available_size - margins;
            result.clamp_negative_to_zero()
        }
        SizingKeyword::Auto => {
            // Default auto: use stretch behavior (inline axis default).
            // For block axis, callers should use compute_automatic_size.
            let result = available_size - margins;
            result.clamp_negative_to_zero()
        }
    }
}

// ── apply_aspect_ratio ───────────────────────────────────────────────

/// Apply an aspect ratio to compute the missing dimension.
///
/// CSS Sizing L3 §5.1: If one dimension is definite and the other is auto,
/// the auto dimension is computed from the definite one using the aspect ratio.
///
/// # Arguments
///
/// * `width` — the current width (`INDEFINITE_SIZE` if auto).
/// * `height` — the current height (`INDEFINITE_SIZE` if auto).
/// * `ratio` — the aspect ratio as `(width_component, height_component)`.
///
/// # Returns
///
/// `(resolved_width, resolved_height)` — the pair with the missing dimension filled in.
///
/// # Rules
///
/// - Width definite, height auto: `height = width * (ratio.1 / ratio.0)`
/// - Height definite, width auto: `width = height * (ratio.0 / ratio.1)`
/// - Both definite: aspect ratio is ignored, both returned as-is.
/// - Both auto/indefinite: both returned as-is (cannot resolve).
/// - Zero ratio component: returns dimensions unchanged (avoids division by zero).
pub fn apply_aspect_ratio(
    width: LayoutUnit,
    height: LayoutUnit,
    ratio: (f32, f32),
) -> (LayoutUnit, LayoutUnit) {
    // Guard against degenerate ratios.
    if ratio.0 == 0.0 || ratio.1 == 0.0 {
        return (width, height);
    }

    let w_definite = !width.is_indefinite();
    let h_definite = !height.is_indefinite();

    match (w_definite, h_definite) {
        (true, false) => {
            // height = width * (ratio.1 / ratio.0)
            let h = LayoutUnit::from_f32(width.to_f32() * ratio.1 / ratio.0);
            (width, h)
        }
        (false, true) => {
            // width = height * (ratio.0 / ratio.1)
            let w = LayoutUnit::from_f32(height.to_f32() * ratio.0 / ratio.1);
            (w, height)
        }
        _ => {
            // Both definite → ignore ratio. Both indefinite → can't resolve.
            (width, height)
        }
    }
}

/// Apply aspect ratio respecting the `auto` flag.
///
/// When `auto_flag` is true, the specified ratio is only used if the element
/// has no intrinsic aspect ratio. This function takes an optional intrinsic
/// ratio: if present and auto_flag is set, the intrinsic ratio wins.
pub fn apply_aspect_ratio_with_auto(
    width: LayoutUnit,
    height: LayoutUnit,
    aspect: &AspectRatio,
    intrinsic_ratio: Option<(f32, f32)>,
) -> (LayoutUnit, LayoutUnit) {
    let ratio = if aspect.auto_flag {
        intrinsic_ratio.unwrap_or(aspect.ratio)
    } else {
        aspect.ratio
    };
    apply_aspect_ratio(width, height, ratio)
}

// ── compute_definite_size ────────────────────────────────────────────

/// Determine whether a CSS length produces a definite size and resolve it.
///
/// A size is definite if it can be resolved to a concrete pixel value without
/// performing layout. CSS Sizing L3 §4:
///
/// - Fixed lengths (`px`) are always definite.
/// - Percentages are definite if the containing block size is definite.
/// - Intrinsic keywords (`min-content`, `max-content`) are *not* definite
///   (they require layout to resolve).
/// - `stretch` is definite inside flex/grid when the container size is known,
///   checked via `ConstraintSpace` flags.
/// - `auto` is definite in flex/grid when the size is fixed externally.
///
/// Returns `Some(resolved_value)` if definite, `None` if indefinite.
pub fn compute_definite_size(
    length: &Length,
    containing_block_size: LayoutUnit,
    space: &ConstraintSpace,
    is_inline_axis: bool,
) -> Option<LayoutUnit> {
    match length.length_type() {
        LengthType::Fixed => {
            Some(LayoutUnit::from_f32(length.value()))
        }
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                None
            } else {
                Some(LayoutUnit::from_f32(
                    length.value() / 100.0 * containing_block_size.to_f32(),
                ))
            }
        }
        LengthType::Stretch => {
            // Stretch is definite inside flex/grid when the container stretches.
            let is_stretching = if is_inline_axis {
                space.stretch_inline_size
            } else {
                space.stretch_block_size
            };
            if is_stretching {
                let avail = if is_inline_axis {
                    space.available_inline_size
                } else {
                    space.available_block_size
                };
                if avail.is_indefinite() { None } else { Some(avail) }
            } else {
                None
            }
        }
        LengthType::Auto => {
            // Auto is definite only when the constraint space fixes the size
            // (e.g., flex child with fixed main axis).
            let is_fixed = if is_inline_axis {
                space.is_fixed_inline_size
            } else {
                space.is_fixed_block_size
            };
            if is_fixed {
                let avail = if is_inline_axis {
                    space.available_inline_size
                } else {
                    space.available_block_size
                };
                if avail.is_indefinite() { None } else { Some(avail) }
            } else {
                None
            }
        }
        // min-content, max-content, fit-content are indefinite during layout.
        _ => None,
    }
}

// ── compute_automatic_size ───────────────────────────────────────────

/// CSS Sizing L3 §5.2 — automatic size in a given axis.
///
/// - For the **inline axis**: the automatic size is `stretch` (fill-available).
/// - For the **block axis**: the automatic size is `fit-content` (shrink-to-content).
///
/// These defaults can be overridden by formatting context rules (e.g., flex
/// children may stretch in the cross axis).
///
/// # Arguments
///
/// * `is_inline_axis` — true for the inline axis, false for block axis.
/// * `intrinsic` — the min/max intrinsic sizes.
/// * `available_size` — available space in the axis.
/// * `margins` — total margins in the axis.
pub fn compute_automatic_size(
    is_inline_axis: bool,
    intrinsic: &MinMaxSizes,
    available_size: LayoutUnit,
    margins: LayoutUnit,
) -> LayoutUnit {
    if is_inline_axis {
        // Inline axis: stretch (fill-available)
        let result = available_size - margins;
        result.clamp_negative_to_zero()
    } else {
        // Block axis: fit-content — clamp max-content to available space
        let stretch = (available_size - margins).clamp_negative_to_zero();
        intrinsic.max.min_of(stretch).max_of(intrinsic.min)
    }
}

// ── resolve_preferred_size ───────────────────────────────────────────

/// CSS Sizing L3 §5.1 — resolve the preferred size for a single axis.
///
/// Resolves `width`/`height` considering:
/// 1. Concrete values (fixed, percent)
/// 2. Sizing keywords (min-content, max-content, fit-content, stretch)
/// 3. Aspect ratio (if the other axis is definite and this axis is auto)
/// 4. min/max constraints
///
/// # Arguments
///
/// * `preferred` — the CSS `width` or `height` value.
/// * `min_size` — the CSS `min-width` or `min-height` value.
/// * `max_size` — the CSS `max-width` or `max-height` value.
/// * `containing_block_size` — size of the containing block in this axis.
/// * `intrinsic` — min/max intrinsic sizes for this element.
/// * `available_size` — available space in the axis.
/// * `margins` — total margins in the axis.
/// * `other_axis_size` — definite size in the other axis (for aspect ratio).
/// * `aspect_ratio` — optional aspect ratio.
/// * `is_width` — true if resolving width (affects aspect ratio direction).
///
/// Returns the final resolved size as a `LayoutUnit`.
pub fn resolve_preferred_size(
    preferred: &Length,
    min_size: &Length,
    max_size: &Length,
    containing_block_size: LayoutUnit,
    intrinsic: &MinMaxSizes,
    available_size: LayoutUnit,
    margins: LayoutUnit,
    other_axis_size: LayoutUnit,
    aspect_ratio: Option<&AspectRatio>,
    is_width: bool,
) -> LayoutUnit {
    // Step 1: Resolve the preferred size.
    let resolved = resolve_size_value(
        preferred,
        containing_block_size,
        intrinsic,
        available_size,
        margins,
    );

    // Step 2: If preferred is auto/indefinite, try aspect ratio.
    let resolved = if resolved.is_indefinite() {
        if let Some(ar) = aspect_ratio {
            if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 && !other_axis_size.is_indefinite() {
                if is_width {
                    // width = other (height) * (ratio.0 / ratio.1)
                    LayoutUnit::from_f32(other_axis_size.to_f32() * ar.ratio.0 / ar.ratio.1)
                } else {
                    // height = other (width) * (ratio.1 / ratio.0)
                    LayoutUnit::from_f32(other_axis_size.to_f32() * ar.ratio.1 / ar.ratio.0)
                }
            } else {
                // Fall back to automatic sizing.
                compute_automatic_size(is_width, intrinsic, available_size, margins)
            }
        } else {
            // No aspect ratio — use automatic sizing.
            compute_automatic_size(is_width, intrinsic, available_size, margins)
        }
    } else {
        resolved
    };

    // Step 3: Resolve min and max constraints.
    let resolved_min = resolve_min_value(min_size, containing_block_size, intrinsic);
    let resolved_max = resolve_max_value(max_size, containing_block_size, intrinsic);

    // Step 4: Clamp: max(min, min(value, max)).
    resolved.max_of(resolved_min).min_of(resolved_max)
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Resolve a size value (for preferred, not min/max) from a CSS Length.
fn resolve_size_value(
    length: &Length,
    containing_block_size: LayoutUnit,
    intrinsic: &MinMaxSizes,
    available_size: LayoutUnit,
    margins: LayoutUnit,
) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()),
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                INDEFINITE_SIZE
            } else {
                LayoutUnit::from_f32(length.value() / 100.0 * containing_block_size.to_f32())
            }
        }
        LengthType::MinContent => intrinsic.min,
        LengthType::MaxContent => intrinsic.max,
        LengthType::FitContent => {
            let limit = LayoutUnit::from_f32(length.value());
            limit.clamp(intrinsic.min, intrinsic.max)
        }
        LengthType::Stretch => {
            (available_size - margins).clamp_negative_to_zero()
        }
        LengthType::Auto | LengthType::None => INDEFINITE_SIZE,
        _ => INDEFINITE_SIZE,
    }
}

/// Resolve a min-size value. `auto` for min-width/min-height resolves to 0
/// in most cases (the automatic minimum from CSS Sizing L3 §5.1 only applies
/// inside flex/grid, which handle it separately).
fn resolve_min_value(
    length: &Length,
    containing_block_size: LayoutUnit,
    intrinsic: &MinMaxSizes,
) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()),
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                LayoutUnit::zero()
            } else {
                LayoutUnit::from_f32(length.value() / 100.0 * containing_block_size.to_f32())
            }
        }
        LengthType::MinContent => intrinsic.min,
        LengthType::MaxContent => intrinsic.max,
        // auto / none → 0 (default automatic minimum)
        _ => LayoutUnit::zero(),
    }
}

/// Resolve a max-size value. `none` → effectively infinite (LayoutUnit::max()).
fn resolve_max_value(
    length: &Length,
    containing_block_size: LayoutUnit,
    intrinsic: &MinMaxSizes,
) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()),
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                LayoutUnit::max()
            } else {
                LayoutUnit::from_f32(length.value() / 100.0 * containing_block_size.to_f32())
            }
        }
        LengthType::MinContent => intrinsic.min,
        LengthType::MaxContent => intrinsic.max,
        // none → unconstrained
        LengthType::None => LayoutUnit::max(),
        _ => LayoutUnit::max(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    #[test]
    fn sizing_keyword_from_length_auto() {
        assert_eq!(SizingKeyword::from_length(&Length::auto()), Some(SizingKeyword::Auto));
    }

    #[test]
    fn sizing_keyword_from_length_fixed_returns_none() {
        assert_eq!(SizingKeyword::from_length(&Length::px(42.0)), None);
    }

    #[test]
    fn sizing_keyword_from_length_min_content() {
        assert_eq!(SizingKeyword::from_length(&Length::min_content()), Some(SizingKeyword::MinContent));
    }

    #[test]
    fn sizing_keyword_from_length_max_content() {
        assert_eq!(SizingKeyword::from_length(&Length::max_content()), Some(SizingKeyword::MaxContent));
    }

    #[test]
    fn sizing_keyword_from_length_stretch() {
        assert_eq!(SizingKeyword::from_length(&Length::stretch()), Some(SizingKeyword::Stretch));
    }

    #[test]
    fn resolve_min_content_keyword() {
        let intrinsic = MinMaxSizes::new(lu(80), lu(200));
        let result = resolve_sizing_keyword(SizingKeyword::MinContent, &intrinsic, lu(500), lu(0));
        assert_eq!(result, lu(80));
    }

    #[test]
    fn resolve_max_content_keyword() {
        let intrinsic = MinMaxSizes::new(lu(80), lu(200));
        let result = resolve_sizing_keyword(SizingKeyword::MaxContent, &intrinsic, lu(500), lu(0));
        assert_eq!(result, lu(200));
    }

    #[test]
    fn resolve_stretch_keyword() {
        let intrinsic = MinMaxSizes::new(lu(50), lu(150));
        let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(400), lu(30));
        assert_eq!(result, lu(370));
    }

    #[test]
    fn resolve_stretch_clamps_negative_to_zero() {
        let intrinsic = MinMaxSizes::zero();
        let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(20), lu(50));
        assert_eq!(result, lu(0));
    }

    #[test]
    fn resolve_fit_content_below_min() {
        // fit-content(30) with min=50, max=200 → clamped up to min → 50
        let intrinsic = MinMaxSizes::new(lu(50), lu(200));
        let result = resolve_sizing_keyword(
            SizingKeyword::FitContent(lu(30)),
            &intrinsic,
            lu(500),
            lu(0),
        );
        assert_eq!(result, lu(50));
    }

    #[test]
    fn resolve_fit_content_above_max() {
        // fit-content(300) with min=50, max=200 → clamped down to max → 200
        let intrinsic = MinMaxSizes::new(lu(50), lu(200));
        let result = resolve_sizing_keyword(
            SizingKeyword::FitContent(lu(300)),
            &intrinsic,
            lu(500),
            lu(0),
        );
        assert_eq!(result, lu(200));
    }

    #[test]
    fn resolve_fit_content_between_min_and_max() {
        // fit-content(120) with min=50, max=200 → 120 (within range)
        let intrinsic = MinMaxSizes::new(lu(50), lu(200));
        let result = resolve_sizing_keyword(
            SizingKeyword::FitContent(lu(120)),
            &intrinsic,
            lu(500),
            lu(0),
        );
        assert_eq!(result, lu(120));
    }

    #[test]
    fn aspect_ratio_width_to_height() {
        // width=160, height=indefinite, ratio 16:9 → height = 160 * 9/16 = 90
        let (w, h) = apply_aspect_ratio(lu(160), INDEFINITE_SIZE, (16.0, 9.0));
        assert_eq!(w, lu(160));
        assert_eq!(h, lu(90));
    }

    #[test]
    fn aspect_ratio_height_to_width() {
        // height=90, width=indefinite, ratio 16:9 → width = 90 * 16/9 = 160
        let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(90), (16.0, 9.0));
        assert_eq!(w, lu(160));
        assert_eq!(h, lu(90));
    }

    #[test]
    fn aspect_ratio_both_definite_ignored() {
        // Both definite → ratio is ignored.
        let (w, h) = apply_aspect_ratio(lu(100), lu(100), (16.0, 9.0));
        assert_eq!(w, lu(100));
        assert_eq!(h, lu(100));
    }

    #[test]
    fn aspect_ratio_zero_ratio_returns_unchanged() {
        let (w, h) = apply_aspect_ratio(lu(100), INDEFINITE_SIZE, (0.0, 9.0));
        assert_eq!(w, lu(100));
        assert_eq!(h, INDEFINITE_SIZE);
    }

    #[test]
    fn aspect_ratio_with_auto_flag_uses_intrinsic() {
        let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: true };
        let intrinsic_ratio = Some((4.0, 3.0));
        // auto_flag is true and intrinsic ratio exists → use intrinsic (4:3)
        let (w, h) = apply_aspect_ratio_with_auto(
            INDEFINITE_SIZE, lu(120), &ar, intrinsic_ratio,
        );
        // width = 120 * 4/3 = 160
        assert_eq!(w, lu(160));
        assert_eq!(h, lu(120));
    }

    #[test]
    fn aspect_ratio_with_auto_flag_no_intrinsic_falls_back() {
        let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: true };
        // No intrinsic ratio → use specified (16:9)
        let (w, h) = apply_aspect_ratio_with_auto(
            INDEFINITE_SIZE, lu(90), &ar, None,
        );
        // width = 90 * 16/9 = 160
        assert_eq!(w, lu(160));
        assert_eq!(h, lu(90));
    }
}
