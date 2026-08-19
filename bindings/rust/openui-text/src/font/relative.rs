//! Resolution of computed font metrics into CSS used values.
//!
//! Font-relative lengths cannot be resolved while parsing a declaration: the
//! selected face, computed font size, weight/style, and used line height must
//! all be known first.  This module is the shared boundary used by generated
//! WPT builders and layout/paint code.

use openui_style::{ComputedStyle, LineHeight};

use super::{Font, FontDescription, FontMetrics};

/// CSS font-relative units supported by the deterministic SP16 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRelativeUnit {
    /// Advance measure of the primary font's `0` glyph.
    Ch,
    /// Primary font's x-height.
    Ex,
    /// Used value of the element's computed line height.
    Lh,
}

/// Half-leading-adjusted vertical metrics for a used line height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsedLineHeightMetrics {
    pub line_height: f32,
    pub ascent: f32,
    pub descent: f32,
}

/// Resolve a computed `line-height` using the selected primary font.
///
/// CSS `normal` uses the face's rounded ascent, descent, and leading.  A
/// standards-based 1.2em fallback is used only when the face exposes no usable
/// vertical data. Numeric, percentage, and fixed values do not depend on the
/// font tables.
pub fn used_line_height(metrics: &FontMetrics, line_height: &LineHeight, font_size: f32) -> f32 {
    match *line_height {
        LineHeight::Normal => {
            let value = metrics.int_line_spacing();
            if value > 0.0 {
                value
            } else {
                font_size * 1.2
            }
        }
        LineHeight::Number(number) => font_size * number,
        LineHeight::Length(px) => px,
        LineHeight::Percentage(percent) => font_size * percent / 100.0,
    }
}

/// Resolve CSS 2.2 half-leading around the primary font's baseline.
pub fn used_line_height_metrics(
    metrics: &FontMetrics,
    line_height: &LineHeight,
    font_size: f32,
) -> UsedLineHeightMetrics {
    let line_height = used_line_height(metrics, line_height, font_size);
    // Blink's ordinary line-height strut uses integer face metrics. For a
    // one-device-pixel strut, retaining fractional face metrics until the
    // final baseline snap avoids losing the entire negative half-leading to
    // the earlier face-metric rounding step.
    let (font_ascent, font_descent) = if line_height <= 1.0 {
        (metrics.ascent, metrics.descent)
    } else {
        (metrics.int_ascent(), metrics.int_descent())
    };
    let leading = line_height - (font_ascent + font_descent);
    let layout_grid = 1.0 / 64.0;
    let ascent_half = (leading / 2.0 / layout_grid).floor() * layout_grid;
    let descent_half = leading - ascent_half;
    UsedLineHeightMetrics {
        line_height,
        ascent: font_ascent + ascent_half,
        descent: font_descent + descent_half,
    }
}

/// Owned font-relative values captured after computed font properties exist.
///
/// Generated builders construct this once per style boundary, then use it for
/// every `ch`, `ex`, and `lh` declaration. It intentionally owns plain floats
/// so assigning resolved lengths never aliases the mutable document style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontRelativeLengthResolver {
    font_size: f32,
    ch: f32,
    ex: f32,
    lh: f32,
}

impl FontRelativeLengthResolver {
    /// Resolve the primary face and capture all font-relative reference values.
    pub fn from_style(style: &ComputedStyle) -> Self {
        let description = FontDescription::from_computed_style(style);
        let font = Font::new(description);
        let metrics = font.font_metrics().copied().unwrap_or_default();
        Self::from_metrics(style.font_size, &style.line_height, &metrics)
    }

    /// Construct from explicit metrics, useful to layout and unit tests.
    pub fn from_metrics(font_size: f32, line_height: &LineHeight, metrics: &FontMetrics) -> Self {
        // CSS Values defines 0.5em fallbacks when the requested glyph metric is
        // unavailable. A present metric is used exactly, including fractional
        // advances supplied by Skia.
        let ch = if metrics.zero_width > 0.0 {
            metrics.zero_width
        } else {
            font_size * 0.5
        };
        let ex = if metrics.x_height > 0.0 {
            metrics.x_height
        } else {
            font_size * 0.5
        };
        Self {
            font_size,
            ch,
            ex,
            lh: used_line_height(metrics, line_height, font_size),
        }
    }

    #[inline]
    pub fn resolve(&self, value: f32, unit: FontRelativeUnit) -> f32 {
        value
            * match unit {
                FontRelativeUnit::Ch => self.ch,
                FontRelativeUnit::Ex => self.ex,
                FontRelativeUnit::Lh => self.lh,
            }
    }

    #[inline]
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> FontMetrics {
        FontMetrics {
            ascent: 14.8,
            descent: 3.8,
            line_gap: 0.0,
            line_spacing: 18.6,
            x_height: 8.75,
            zero_width: 10.18,
            ..FontMetrics::zero()
        }
    }

    #[test]
    fn resolves_ch_ex_and_lh_from_primary_metrics() {
        let resolver =
            FontRelativeLengthResolver::from_metrics(16.0, &LineHeight::Number(1.5), &metrics());
        assert_eq!(resolver.resolve(2.0, FontRelativeUnit::Ch), 20.36);
        assert_eq!(resolver.resolve(2.0, FontRelativeUnit::Ex), 17.5);
        assert_eq!(resolver.resolve(2.0, FontRelativeUnit::Lh), 48.0);
    }

    #[test]
    fn missing_glyph_metrics_use_half_em_fallbacks() {
        let resolver = FontRelativeLengthResolver::from_metrics(
            20.0,
            &LineHeight::Normal,
            &FontMetrics::zero(),
        );
        assert_eq!(resolver.resolve(1.0, FontRelativeUnit::Ch), 10.0);
        assert_eq!(resolver.resolve(1.0, FontRelativeUnit::Ex), 10.0);
        assert_eq!(resolver.resolve(1.0, FontRelativeUnit::Lh), 24.0);
    }

    #[test]
    fn used_line_height_supports_every_computed_form() {
        let metrics = metrics();
        assert_eq!(used_line_height(&metrics, &LineHeight::Normal, 16.0), 19.0);
        assert_eq!(
            used_line_height(&metrics, &LineHeight::Number(1.25), 16.0),
            20.0
        );
        assert_eq!(
            used_line_height(&metrics, &LineHeight::Percentage(150.0), 16.0),
            24.0
        );
        assert_eq!(
            used_line_height(&metrics, &LineHeight::Length(22.0), 16.0),
            22.0
        );
    }

    #[test]
    fn half_leading_preserves_the_exact_used_height() {
        let used = used_line_height_metrics(&metrics(), &LineHeight::Length(25.0), 16.0);
        assert_eq!(used.line_height, 25.0);
        assert!((used.ascent + used.descent - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn negative_half_leading_uses_fractional_face_metrics_before_snapping() {
        let used = used_line_height_metrics(&metrics(), &LineHeight::Length(1.0), 16.0);
        assert_eq!(used.line_height, 1.0);
        assert!((used.ascent - 5.9875).abs() < f32::EPSILON);
        assert!((used.ascent + used.descent - 1.0).abs() < f32::EPSILON);
    }
}
