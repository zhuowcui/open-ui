//! CSS Length type — extracted from Blink's `platform/geometry/length.h`.
//!
//! Blink's Length stores a type discriminant and a float value. It supports
//! auto, percentage, fixed (px), min-content, max-content, stretch, fit-content,
//! calc(), flex (fr), none, and content. We replicate this exactly.
//!
//! Source: third_party/blink/renderer/platform/geometry/length.h

/// The discriminant for a CSS length value.
///
/// Extracted from Blink's `Length::Type` enum. The numeric values match Blink's
/// ordering for consistency during development but are not ABI-relevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LengthType {
    /// `auto` — let the layout algorithm decide.
    Auto = 0,
    /// Percentage of the containing block's relevant dimension.
    Percent = 1,
    /// Fixed pixel value (already resolved from CSS `px`).
    Fixed = 2,
    /// `min-content` intrinsic size.
    MinContent = 3,
    /// `max-content` intrinsic size.
    MaxContent = 4,
    /// `min-intrinsic` — Blink-internal, rarely used.
    MinIntrinsic = 5,
    /// `stretch` (formerly `fill-available`).
    Stretch = 6,
    /// `fit-content` — clamp between min-content and max-content.
    FitContent = 7,
    /// `calc()` expression — stored as a handle to a calculation tree.
    /// For SP9 we store the pre-resolved float; full calc() comes later.
    Calculated = 8,
    /// `fr` fractional unit (CSS Grid).
    Flex = 9,
    /// `none` — used for max-width/max-height initial value.
    None = 10,
    /// `content` — used in certain grid contexts.
    Content = 11,
}

/// A CSS length value, storing a type and a numeric value.
///
/// This mirrors Blink's `Length` class (platform/geometry/length.h).
/// The value field holds pixels for `Fixed`, a ratio (0.0–100.0+) for `Percent`,
/// or 0.0 for types like `Auto`/`None` that carry no numeric payload.
#[derive(Debug, Clone, Copy)]
pub struct Length {
    value: f32,
    length_type: LengthType,
}

impl Length {
    // ── Constructors matching Blink's static factories ───────────────

    /// `Length()` — default is `Fixed(0)` in Blink.
    #[inline]
    pub const fn zero() -> Self {
        Self { value: 0.0, length_type: LengthType::Fixed }
    }

    /// `Length::Auto()` — the `auto` keyword.
    #[inline]
    pub const fn auto() -> Self {
        Self { value: 0.0, length_type: LengthType::Auto }
    }

    /// `Length::None()` — used as initial value for max-width/max-height.
    #[inline]
    pub const fn none() -> Self {
        Self { value: 0.0, length_type: LengthType::None }
    }

    /// Fixed pixel value.
    #[inline]
    pub const fn px(value: f32) -> Self {
        Self { value, length_type: LengthType::Fixed }
    }

    /// Percentage value (0.0 = 0%, 100.0 = 100%).
    #[inline]
    pub const fn percent(value: f32) -> Self {
        Self { value, length_type: LengthType::Percent }
    }

    /// `min-content` intrinsic keyword.
    #[inline]
    pub const fn min_content() -> Self {
        Self { value: 0.0, length_type: LengthType::MinContent }
    }

    /// `max-content` intrinsic keyword.
    #[inline]
    pub const fn max_content() -> Self {
        Self { value: 0.0, length_type: LengthType::MaxContent }
    }

    /// `stretch` keyword.
    #[inline]
    pub const fn stretch() -> Self {
        Self { value: 0.0, length_type: LengthType::Stretch }
    }

    /// `fit-content` keyword.
    #[inline]
    pub const fn fit_content() -> Self {
        Self { value: 0.0, length_type: LengthType::FitContent }
    }

    /// `fr` fractional unit for CSS Grid.
    #[inline]
    pub const fn flex(value: f32) -> Self {
        Self { value, length_type: LengthType::Flex }
    }

    // ── Type queries matching Blink's `Is*()` methods ────────────────

    #[inline]
    pub const fn is_auto(&self) -> bool { matches!(self.length_type, LengthType::Auto) }

    #[inline]
    pub const fn is_fixed(&self) -> bool { matches!(self.length_type, LengthType::Fixed) }

    #[inline]
    pub const fn is_percent(&self) -> bool { matches!(self.length_type, LengthType::Percent) }

    #[inline]
    pub const fn is_none(&self) -> bool { matches!(self.length_type, LengthType::None) }

    #[inline]
    pub const fn is_calculated(&self) -> bool { matches!(self.length_type, LengthType::Calculated) }

    #[inline]
    pub const fn is_min_content(&self) -> bool { matches!(self.length_type, LengthType::MinContent) }

    #[inline]
    pub const fn is_max_content(&self) -> bool { matches!(self.length_type, LengthType::MaxContent) }

    #[inline]
    pub const fn is_stretch(&self) -> bool { matches!(self.length_type, LengthType::Stretch) }

    #[inline]
    pub const fn is_fit_content(&self) -> bool { matches!(self.length_type, LengthType::FitContent) }

    #[inline]
    pub const fn is_content_or_intrinsic(&self) -> bool {
        matches!(
            self.length_type,
            LengthType::MinContent | LengthType::MaxContent | LengthType::FitContent | LengthType::Content
        )
    }

    /// True if the length is `Fixed` or `Percent` — the two types that can be
    /// resolved against a containing block dimension.
    #[inline]
    pub const fn is_specified(&self) -> bool {
        matches!(self.length_type, LengthType::Fixed | LengthType::Percent | LengthType::Calculated)
    }

    // ── Value access ─────────────────────────────────────────────────

    /// The raw numeric value. Interpretation depends on `length_type()`.
    #[inline]
    pub const fn value(&self) -> f32 { self.value }

    /// The type discriminant.
    #[inline]
    pub const fn length_type(&self) -> LengthType { self.length_type }

    // ── Equality — Blink compares type + value ───────────────────────
}

impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        self.length_type == other.length_type && self.value == other.value
    }
}

impl Eq for Length {}

impl Default for Length {
    /// Blink's default `Length()` is `Fixed(0)`.
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_zero_fixed() {
        let l = Length::default();
        assert!(l.is_fixed());
        assert_eq!(l.value(), 0.0);
    }

    #[test]
    fn auto_is_auto() {
        let l = Length::auto();
        assert!(l.is_auto());
        assert!(!l.is_fixed());
        assert!(!l.is_specified());
    }

    #[test]
    fn percent_stores_value() {
        let l = Length::percent(50.0);
        assert!(l.is_percent());
        assert!(l.is_specified());
        assert_eq!(l.value(), 50.0);
    }

    #[test]
    fn none_for_max_dimensions() {
        let l = Length::none();
        assert!(l.is_none());
        assert!(!l.is_auto());
    }

    #[test]
    fn equality() {
        assert_eq!(Length::px(10.0), Length::px(10.0));
        assert_ne!(Length::px(10.0), Length::percent(10.0));
        assert_ne!(Length::auto(), Length::none());
    }
}
