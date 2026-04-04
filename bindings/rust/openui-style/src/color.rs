//! Color type — extracted from Blink's platform/graphics/color.h.
//!
//! Blink internally uses `Color` with float components and a `ColorSpace` enum.
//! For our initial implementation we use sRGB with f32 components (matching
//! Blink's default path). Pre-multiplied alpha is NOT used for storage — Blink
//! stores straight alpha and only pre-multiplies at paint time.

/// An RGBA color in sRGB color space with f32 components [0.0, 1.0].
///
/// This matches Blink's `Color` class in its default sRGB mode.
/// Skia's `SkColor4f` uses the same {r, g, b, a} f32 layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    // ── Named constants matching CSS color keywords ──────────────────

    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 128.0 / 255.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

    /// Construct from 0–255 integer components.
    #[inline]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Construct from f32 components (already normalized to [0, 1]).
    #[inline]
    pub const fn from_rgba_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct from CSS hex `#RRGGBB` or `#RRGGBBAA`.
    pub fn from_hex(hex: u32, has_alpha: bool) -> Self {
        if has_alpha {
            Self::from_rgba8(
                ((hex >> 24) & 0xFF) as u8,
                ((hex >> 16) & 0xFF) as u8,
                ((hex >> 8) & 0xFF) as u8,
                (hex & 0xFF) as u8,
            )
        } else {
            Self::from_rgba8(
                ((hex >> 16) & 0xFF) as u8,
                ((hex >> 8) & 0xFF) as u8,
                (hex & 0xFF) as u8,
                255,
            )
        }
    }

    /// Convert to Skia-compatible packed u32 (ARGB premultiplied is NOT
    /// needed — skia-safe accepts SkColor4f which is straight alpha).
    #[inline]
    pub fn to_sk_color4f(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Is this color fully transparent?
    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.a == 0.0
    }

    /// Is this color fully opaque?
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.a >= 1.0
    }
}

impl Default for Color {
    /// Initial value for CSS `color` property is black.
    fn default() -> Self { Self::BLACK }
}

/// Blink's `StyleColor` wraps `Color` with a `currentColor` flag.
/// `currentColor` means "inherit the computed value of `color`".
/// Border colors default to `currentColor`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleColor {
    /// A resolved color value.
    Resolved(Color),
    /// `currentColor` — resolves to the inherited `color` property.
    CurrentColor,
}

impl StyleColor {
    /// Resolve to a concrete color. If `currentColor`, uses the inherited color.
    #[inline]
    pub fn resolve(&self, current_color: &Color) -> Color {
        match self {
            Self::Resolved(c) => *c,
            Self::CurrentColor => *current_color,
        }
    }
}

impl Default for StyleColor {
    /// Border colors default to `currentColor` in CSS.
    fn default() -> Self { Self::CurrentColor }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rgba8() {
        let c = Color::from_rgba8(255, 128, 0, 255);
        assert!((c.r - 1.0).abs() < 0.001);
        assert!((c.g - 0.502).abs() < 0.01);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn current_color_resolves() {
        let sc = StyleColor::CurrentColor;
        let inherited = Color::RED;
        assert_eq!(sc.resolve(&inherited), Color::RED);
    }

    #[test]
    fn hex_parsing() {
        let c = Color::from_hex(0xFF8000, false);
        assert!((c.r - 1.0).abs() < 0.001);
        assert!((c.g - 0.502).abs() < 0.01);
        assert_eq!(c.b, 0.0);
    }
}
