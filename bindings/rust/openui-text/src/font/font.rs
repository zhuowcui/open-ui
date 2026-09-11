//! Font — main entry point for font operations.
//!
//! Mirrors Blink's `Font` class (platform/fonts/font.h).
//! Combines a `FontDescription` with its resolved `FontFallbackList`
//! to provide metrics access and simple text measurement.

use std::sync::Arc;

use super::description::FontDescription;
use super::fallback::FontFallbackList;
use super::metrics::FontMetrics;
use super::platform::FontPlatformData;

/// Main font object — description + resolved fallback chain.
///
/// Blink: `Font` in `platform/fonts/font.h`.
/// Created from a `FontDescription`; resolves typefaces on construction.
pub struct Font {
    description: FontDescription,
    fallback_list: FontFallbackList,
}

impl Font {
    /// Create a font from a description. Resolves typefaces immediately.
    pub fn new(description: FontDescription) -> Self {
        let fallback_list = FontFallbackList::new(&description);
        Self {
            description,
            fallback_list,
        }
    }

    /// The font description used to create this font.
    #[inline]
    pub fn description(&self) -> &FontDescription {
        &self.description
    }

    /// The primary (first resolved) platform font data.
    #[inline]
    pub fn primary_font(&self) -> Option<&Arc<FontPlatformData>> {
        self.fallback_list.primary()
    }

    /// Typographic metrics for the primary font.
    #[inline]
    pub fn font_metrics(&self) -> Option<&FontMetrics> {
        self.primary_font().map(|f| f.metrics())
    }

    /// The fallback chain.
    #[inline]
    pub fn fallback_list(&self) -> &FontFallbackList {
        &self.fallback_list
    }

    /// Measure the advance width of a string using the primary font.
    ///
    /// This is a simplified measurement using `SkFont::measure_str` that does
    /// not account for HarfBuzz shaping, ligatures, or complex scripts. For
    /// accurate measurement, use `TextShaper::shape()` instead.
    pub fn width(&self, text: &str) -> f32 {
        if let Some(font_data) = self.primary_font() {
            let (width, _) = font_data.sk_font().measure_str(text, None);
            if self.description.native_button_text_metrics {
                width * (13.0 / 13.328_125)
            } else {
                width
            }
        } else {
            0.0
        }
    }

    /// The computed font size in pixels.
    #[inline]
    pub fn size(&self) -> f32 {
        self.description.size
    }

    /// Number of fonts in the fallback chain.
    #[inline]
    pub fn fallback_count(&self) -> usize {
        self.fallback_list.len()
    }

    /// Return the first resolved family that covers every painted scalar in a
    /// grapheme cluster. Joiners, variation selectors, and bidi controls do
    /// not require a glyph of their own. This lets inline collection split at
    /// deterministic fallback boundaries before shaping.
    pub fn fallback_index_for_text(&self, text: &str) -> Option<usize> {
        self.fallback_list
            .iter()
            .enumerate()
            .find_map(|(index, data)| {
                text.chars()
                    .filter(|ch| {
                        !ch.is_control()
                            && !matches!(
                                *ch as u32,
                                0x200C..=0x200D | 0x202A..=0x202E | 0x2066..=0x2069
                                    | 0xFE00..=0xFE0F
                            )
                    })
                    .all(|ch| data.sk_font().unichar_to_glyph(ch as i32) != 0)
                    .then_some(index)
            })
    }
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font")
            .field("description", &self.description)
            .field("fallback_count", &self.fallback_list.len())
            .finish()
    }
}
