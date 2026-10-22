//! FontMetrics — typographic metrics for a resolved font at a given size.
//!
//! Mirrors Blink's `FontMetrics` (platform/fonts/font_metrics.h).
//! Values are extracted from Skia's `SkFontMetrics` and normalized:
//! - Ascent is POSITIVE (Skia stores it as negative).
//! - Descent is POSITIVE (distance below baseline).
//! - All values are in CSS pixels at the resolved font size.

/// Typographic metrics for a resolved font at a specific size.
///
/// Blink: `FontMetrics` in `platform/fonts/font_metrics.h`.
/// Populated from `SkFontMetrics` with sign corrections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    // ── Primary vertical metrics ─────────────────────────────────────
    /// Distance above the baseline (POSITIVE).
    /// Skia's `ascent` is negative; we negate it.
    pub ascent: f32,

    /// Distance below the baseline (POSITIVE).
    /// Skia's `descent` is already positive.
    pub descent: f32,

    /// Extra leading between lines (from OS/2 or hhea table).
    /// Skia: `leading`.
    pub line_gap: f32,

    /// Total line spacing: ascent + descent + line_gap.
    /// This is the default distance between baselines.
    pub line_spacing: f32,

    // ── Reference metrics ────────────────────────────────────────────
    /// Height of lowercase 'x' (CSS `ex` unit reference).
    pub x_height: f32,

    /// Height of uppercase letters (CSS `cap` unit reference).
    pub cap_height: f32,

    /// Width of digit '0' (CSS `ch` unit reference).
    pub zero_width: f32,

    // ── Decoration metrics ───────────────────────────────────────────
    /// Distance below baseline for underline (POSITIVE = below baseline).
    pub underline_offset: f32,

    /// Thickness of the underline stroke.
    pub underline_thickness: f32,

    /// Distance above baseline for strikethrough (POSITIVE = above baseline).
    pub strikeout_position: f32,

    /// Thickness of the strikethrough stroke.
    pub strikeout_thickness: f32,

    /// Distance above the ascent for overline.
    /// Blink computes this as `-ascent` offset (i.e., at the top of the em).
    pub overline_offset: f32,

    // ── Font design metrics ──────────────────────────────────────────
    /// Units per em from the font's head table.
    pub units_per_em: u16,
}

impl FontMetrics {
    /// Ascent rounded to nearest integer, matching Blink's `FixedAscent()`.
    #[inline]
    pub fn int_ascent(&self) -> f32 {
        self.ascent.round()
    }

    /// Descent rounded to nearest integer, matching Blink's `FixedDescent()`.
    #[inline]
    pub fn int_descent(&self) -> f32 {
        self.descent.round()
    }

    /// Rounded line spacing matching Blink's behavior.
    ///
    /// Blink: lroundf(ascent) + lroundf(descent) + lroundf(line_gap) — simple_font_data.cc:175
    /// Each metric is rounded individually, then summed.
    #[inline]
    pub fn int_line_spacing(&self) -> f32 {
        self.int_ascent() + self.int_descent() + self.line_gap.round()
    }

    /// Create zeroed metrics (used as fallback when no font is resolved).
    pub fn zero() -> Self {
        Self {
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
            line_spacing: 0.0,
            x_height: 0.0,
            cap_height: 0.0,
            zero_width: 0.0,
            underline_offset: 0.0,
            underline_thickness: 0.0,
            strikeout_position: 0.0,
            strikeout_thickness: 0.0,
            overline_offset: 0.0,
            units_per_em: 0,
        }
    }
}

/// The baseline that an inline formatting context aligns its content to.
///
/// Blink: `FontBaseline` as chosen by `ComputedStyle::GetFontBaseline()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontBaseline {
    /// The alphabetic baseline of horizontal and sideways text.
    #[default]
    Alphabetic,
    /// The ideographic central baseline of vertical flow.
    Central,
}

impl FontBaseline {
    /// Vertical flow, except sideways text, uses the central baseline.
    ///
    /// Blink: `ComputedStyle::GetFontBaseline()`, which tests
    /// `FontDescription::IsVerticalAnyUpright()`.
    #[inline]
    pub fn for_orientation(orientation: openui_style::FontOrientation) -> Self {
        if orientation.uses_vertical_metrics() {
            Self::Central
        } else {
            Self::Alphabetic
        }
    }
}

impl FontMetrics {
    /// Express ascent and descent relative to `baseline`.
    ///
    /// Blink: `FontMetrics::GetFontHeight(baseline_type)`. The central
    /// baseline splits the integer height `Height() = IntAscent +
    /// IntDescent` as `Height() - Height() / 2` above and `Height() / 2`
    /// below. Other metrics stay relative to the alphabetic baseline.
    pub fn relative_to_baseline(self, baseline: FontBaseline) -> Self {
        match baseline {
            FontBaseline::Alphabetic => self,
            FontBaseline::Central => {
                let height = self.int_ascent() as i32 + self.int_descent() as i32;
                let descent = height / 2;
                Self {
                    ascent: (height - descent) as f32,
                    descent: descent as f32,
                    ..self
                }
            }
        }
    }
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(ascent: f32, descent: f32) -> FontMetrics {
        FontMetrics {
            ascent,
            descent,
            x_height: 8.0,
            ..FontMetrics::zero()
        }
    }

    #[test]
    fn central_baseline_splits_the_integer_height_with_the_odd_pixel_above() {
        // Arial 16px: IntAscent 14 + IntDescent 3 = 17 -> 9 above, 8 below.
        let central = metrics(14.484, 3.391).relative_to_baseline(FontBaseline::Central);
        assert_eq!((central.ascent, central.descent), (9.0, 8.0));
        assert_eq!(central.x_height, 8.0);
        // DejaVu Sans 16px: 15 + 4 = 19 -> 10 above, 9 below.
        let central = metrics(14.85, 3.77).relative_to_baseline(FontBaseline::Central);
        assert_eq!((central.ascent, central.descent), (10.0, 9.0));
        let alphabetic = metrics(14.484, 3.391);
        assert_eq!(
            alphabetic.relative_to_baseline(FontBaseline::Alphabetic),
            alphabetic
        );
    }

    #[test]
    fn only_vertical_upright_or_mixed_orientation_uses_the_central_baseline() {
        use openui_style::FontOrientation;
        for (orientation, expected) in [
            (FontOrientation::Horizontal, FontBaseline::Alphabetic),
            (FontOrientation::VerticalRotated, FontBaseline::Alphabetic),
            (FontOrientation::VerticalMixed, FontBaseline::Central),
            (FontOrientation::VerticalUpright, FontBaseline::Central),
        ] {
            assert_eq!(FontBaseline::for_orientation(orientation), expected);
        }
    }
}
