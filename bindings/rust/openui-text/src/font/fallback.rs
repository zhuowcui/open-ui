//! FontFallbackList — ordered chain of resolved fonts for a description.
//!
//! Mirrors Blink's `FontFallbackList` (platform/fonts/font_fallback_list.h).
//! Walks the `FontDescription::family` list, resolving each family via
//! `FontCache`, and stores the resulting `FontPlatformData` entries in order.

use std::sync::Arc;

use openui_style::FontFamily;

use super::cache::{FontCache, GLOBAL_FONT_CACHE};
use super::description::FontDescription;
use super::platform::FontPlatformData;

/// Ordered list of resolved platform fonts for a `FontDescription`.
///
/// Blink: `FontFallbackList` in `platform/fonts/font_fallback_list.h`.
/// The first successfully resolved font is the "primary" font.
pub struct FontFallbackList {
    platform_data: Vec<Arc<FontPlatformData>>,
}

impl FontFallbackList {
    /// Resolve all families in the description and build the fallback chain.
    pub fn new(description: &FontDescription) -> Self {
        let mut list = Self {
            platform_data: Vec::new(),
        };
        list.resolve(description);
        list
    }

    /// Try to resolve each family in order, then fall back to sans-serif.
    fn resolve(&mut self, description: &FontDescription) {
        let mut cache = GLOBAL_FONT_CACHE.lock().unwrap_or_else(|poisoned| {
            // Recover from a poisoned mutex — the data is still usable.
            // This can happen if a previous thread panicked while holding the lock.
            poisoned.into_inner()
        });

        for family in &description.family.families {
            let name = match family {
                FontFamily::Named(name) => name.as_str(),
                FontFamily::Generic(generic) => FontCache::generic_family_name(*generic),
            };

            if let Some(data) = cache.get_font_platform_data(name, description) {
                self.platform_data.push(data);
            }
        }

        // If nothing resolved, use system default sans-serif
        if self.platform_data.is_empty() {
            if let Some(data) = cache.get_font_platform_data("sans-serif", description) {
                self.platform_data.push(data);
            }
        }
    }

    /// The primary (first successfully resolved) font.
    #[inline]
    pub fn primary(&self) -> Option<&Arc<FontPlatformData>> {
        self.platform_data.first()
    }

    /// Get a font at a specific index in the fallback chain.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Arc<FontPlatformData>> {
        self.platform_data.get(index)
    }

    /// Number of resolved fonts in the chain.
    #[inline]
    pub fn len(&self) -> usize {
        self.platform_data.len()
    }

    /// Whether the fallback list is empty (no fonts resolved).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.platform_data.is_empty()
    }
}

impl std::fmt::Debug for FontFallbackList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFallbackList")
            .field("count", &self.platform_data.len())
            .finish()
    }
}
