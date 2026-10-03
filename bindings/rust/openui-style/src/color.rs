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

    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 128.0 / 255.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    /// Resolve a case-insensitive named sRGB color from Chromium's pinned
    /// color table. Context-dependent colors such as currentcolor are not
    /// fixed colors and must be represented with StyleColor instead.
    pub fn from_named(name: &str) -> Option<Self> {
        let rgba = match name.trim().to_ascii_lowercase().as_str() {
            "aliceblue" => 0xf0f8ffff,
            "antiquewhite" => 0xfaebd7ff,
            "aqua" => 0x00ffffff,
            "aquamarine" => 0x7fffd4ff,
            "azure" => 0xf0ffffff,
            "beige" => 0xf5f5dcff,
            "bisque" => 0xffe4c4ff,
            "black" => 0x000000ff,
            "blanchedalmond" => 0xffebcdff,
            "blue" => 0x0000ffff,
            "blueviolet" => 0x8a2be2ff,
            "brown" => 0xa52a2aff,
            "burlywood" => 0xdeb887ff,
            "cadetblue" => 0x5f9ea0ff,
            "chartreuse" => 0x7fff00ff,
            "chocolate" => 0xd2691eff,
            "coral" => 0xff7f50ff,
            "cornflowerblue" => 0x6495edff,
            "cornsilk" => 0xfff8dcff,
            "crimson" => 0xdc143cff,
            "cyan" => 0x00ffffff,
            "darkblue" => 0x00008bff,
            "darkcyan" => 0x008b8bff,
            "darkgoldenrod" => 0xb8860bff,
            "darkgray" => 0xa9a9a9ff,
            "darkgrey" => 0xa9a9a9ff,
            "darkgreen" => 0x006400ff,
            "darkkhaki" => 0xbdb76bff,
            "darkmagenta" => 0x8b008bff,
            "darkolivegreen" => 0x556b2fff,
            "darkorange" => 0xff8c00ff,
            "darkorchid" => 0x9932ccff,
            "darkred" => 0x8b0000ff,
            "darksalmon" => 0xe9967aff,
            "darkseagreen" => 0x8fbc8fff,
            "darkslateblue" => 0x483d8bff,
            "darkslategray" => 0x2f4f4fff,
            "darkslategrey" => 0x2f4f4fff,
            "darkturquoise" => 0x00ced1ff,
            "darkviolet" => 0x9400d3ff,
            "deeppink" => 0xff1493ff,
            "deepskyblue" => 0x00bfffff,
            "dimgray" => 0x696969ff,
            "dimgrey" => 0x696969ff,
            "dodgerblue" => 0x1e90ffff,
            "firebrick" => 0xb22222ff,
            "floralwhite" => 0xfffaf0ff,
            "forestgreen" => 0x228b22ff,
            "fuchsia" => 0xff00ffff,
            "gainsboro" => 0xdcdcdcff,
            "ghostwhite" => 0xf8f8ffff,
            "gold" => 0xffd700ff,
            "goldenrod" => 0xdaa520ff,
            "gray" => 0x808080ff,
            "grey" => 0x808080ff,
            "green" => 0x008000ff,
            "greenyellow" => 0xadff2fff,
            "honeydew" => 0xf0fff0ff,
            "hotpink" => 0xff69b4ff,
            "indianred" => 0xcd5c5cff,
            "indigo" => 0x4b0082ff,
            "ivory" => 0xfffff0ff,
            "khaki" => 0xf0e68cff,
            "lavender" => 0xe6e6faff,
            "lavenderblush" => 0xfff0f5ff,
            "lawngreen" => 0x7cfc00ff,
            "lemonchiffon" => 0xfffacdff,
            "lightblue" => 0xadd8e6ff,
            "lightcoral" => 0xf08080ff,
            "lightcyan" => 0xe0ffffff,
            "lightgoldenrodyellow" => 0xfafad2ff,
            "lightgray" => 0xd3d3d3ff,
            "lightgrey" => 0xd3d3d3ff,
            "lightgreen" => 0x90ee90ff,
            "lightpink" => 0xffb6c1ff,
            "lightsalmon" => 0xffa07aff,
            "lightseagreen" => 0x20b2aaff,
            "lightskyblue" => 0x87cefaff,
            "lightslateblue" => 0x8470ffff,
            "lightslategray" => 0x778899ff,
            "lightslategrey" => 0x778899ff,
            "lightsteelblue" => 0xb0c4deff,
            "lightyellow" => 0xffffe0ff,
            "lime" => 0x00ff00ff,
            "limegreen" => 0x32cd32ff,
            "linen" => 0xfaf0e6ff,
            "magenta" => 0xff00ffff,
            "maroon" => 0x800000ff,
            "mediumaquamarine" => 0x66cdaaff,
            "mediumblue" => 0x0000cdff,
            "mediumorchid" => 0xba55d3ff,
            "mediumpurple" => 0x9370dbff,
            "mediumseagreen" => 0x3cb371ff,
            "mediumslateblue" => 0x7b68eeff,
            "mediumspringgreen" => 0x00fa9aff,
            "mediumturquoise" => 0x48d1ccff,
            "mediumvioletred" => 0xc71585ff,
            "midnightblue" => 0x191970ff,
            "mintcream" => 0xf5fffaff,
            "mistyrose" => 0xffe4e1ff,
            "moccasin" => 0xffe4b5ff,
            "navajowhite" => 0xffdeadff,
            "navy" => 0x000080ff,
            "oldlace" => 0xfdf5e6ff,
            "olive" => 0x808000ff,
            "olivedrab" => 0x6b8e23ff,
            "orange" => 0xffa500ff,
            "orangered" => 0xff4500ff,
            "orchid" => 0xda70d6ff,
            "palegoldenrod" => 0xeee8aaff,
            "palegreen" => 0x98fb98ff,
            "paleturquoise" => 0xafeeeeff,
            "palevioletred" => 0xdb7093ff,
            "papayawhip" => 0xffefd5ff,
            "peachpuff" => 0xffdab9ff,
            "peru" => 0xcd853fff,
            "pink" => 0xffc0cbff,
            "plum" => 0xdda0ddff,
            "powderblue" => 0xb0e0e6ff,
            "purple" => 0x800080ff,
            "rebeccapurple" => 0x663399ff,
            "red" => 0xff0000ff,
            "rosybrown" => 0xbc8f8fff,
            "royalblue" => 0x4169e1ff,
            "saddlebrown" => 0x8b4513ff,
            "salmon" => 0xfa8072ff,
            "sandybrown" => 0xf4a460ff,
            "seagreen" => 0x2e8b57ff,
            "seashell" => 0xfff5eeff,
            "sienna" => 0xa0522dff,
            "silver" => 0xc0c0c0ff,
            "skyblue" => 0x87ceebff,
            "slateblue" => 0x6a5acdff,
            "slategray" => 0x708090ff,
            "slategrey" => 0x708090ff,
            "snow" => 0xfffafaff,
            "springgreen" => 0x00ff7fff,
            "steelblue" => 0x4682b4ff,
            "tan" => 0xd2b48cff,
            "teal" => 0x008080ff,
            "thistle" => 0xd8bfd8ff,
            "tomato" => 0xff6347ff,
            "transparent" => 0x00000000,
            "turquoise" => 0x40e0d0ff,
            "violet" => 0xee82eeff,
            "violetred" => 0xd02090ff,
            "wheat" => 0xf5deb3ff,
            "white" => 0xffffffff,
            "whitesmoke" => 0xf5f5f5ff,
            "yellow" => 0xffff00ff,
            "yellowgreen" => 0x9acd32ff,
            _ => return None,
        };
        Some(Self::from_hex(rgba, true))
    }

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
    fn default() -> Self {
        Self::BLACK
    }
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
    fn default() -> Self {
        Self::CurrentColor
    }
}

impl From<Color> for StyleColor {
    fn from(value: Color) -> Self {
        Self::Resolved(value)
    }
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
