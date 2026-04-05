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

    /// Rounded line spacing: sum of individually rounded metrics.
    ///
    /// Blink rounds each metric first, then sums:
    /// `SkScalarRoundToInt(ascent) + SkScalarRoundToInt(descent) + SkScalarRoundToInt(leading)`.
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

impl Default for FontMetrics {
    fn default() -> Self {
        Self::zero()
    }
}
