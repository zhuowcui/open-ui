//! FontFallbackList — ordered chain of resolved fonts for a description.
//!
//! Mirrors Blink's `FontFallbackList` (platform/fonts/font_fallback_list.h).
//! Walks the `FontDescription::family` list, resolving each family via
//! `FontCache`, and stores the resulting `FontPlatformData` entries in order.

use std::sync::Arc;

use super::collection::{family_name, FontCollection};
use super::description::FontDescription;
use super::platform::FontPlatformData;

/// Ordered list of resolved platform fonts for a `FontDescription`.
///
/// Blink: `FontFallbackList` in `platform/fonts/font_fallback_list.h`.
/// The first successfully resolved font is the "primary" font.
pub struct FontFallbackList {
    platform_data: Vec<Arc<FontPlatformData>>,
    collection: Arc<FontCollection>,
}

impl FontFallbackList {
    /// Resolve all families in the description and build the fallback chain.
    pub fn new(description: &FontDescription) -> Self {
        Self::new_in_collection(description, FontCollection::system())
    }

    pub fn new_in_collection(
        description: &FontDescription,
        collection: Arc<FontCollection>,
    ) -> Self {
        let mut list = Self {
            platform_data: Vec::new(),
            collection,
        };
        list.resolve(description);
        list
    }

    /// Try to resolve each family in order, then fall back to sans-serif.
    fn resolve(&mut self, description: &FontDescription) {
        for family in &description.family.families {
            if let Some(data) = self
                .collection
                .resolve_family(family_name(family), description)
            {
                self.platform_data.push(data);
            }
        }

        // CSS's final generic fallback is resolved by Fontconfig/Skia.
        if self.platform_data.is_empty() {
            if let Some(data) = self.collection.resolve_family("sans-serif", description) {
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

    /// Iterate the resolved families in authored order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Arc<FontPlatformData>> {
        self.platform_data.iter()
    }

    pub fn collection(&self) -> &Arc<FontCollection> {
        &self.collection
    }
}

impl std::fmt::Debug for FontFallbackList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFallbackList")
            .field("count", &self.platform_data.len())
            .finish()
    }
}
