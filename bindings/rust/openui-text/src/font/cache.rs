//! Compatibility facade over a document-owned [`FontCollection`].

use std::sync::Arc;

#[cfg(test)]
use openui_geometry::RasterConfiguration;
#[cfg(test)]
use openui_style::FontStyleEnum;
use openui_style::GenericFontFamily;

use super::collection::{generic_family_name, FontCollection};
use super::description::FontDescription;
use super::platform::FontPlatformData;

#[cfg(test)]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontCacheKey {
    family: String,
    size_bits: u32,
    weight_bits: u32,
    stretch_bits: u32,
    style_tag: u8,
    oblique_angle_bits: u32,
    native_control_text: bool,
    embedded_document_text: bool,
    native_button_text_metrics: bool,
    raster_configuration: RasterConfiguration,
    device_scale_factor_bits: u64,
}

/// A non-global cache facade retained for low-level embedders and tests.
pub struct FontCache {
    collection: Arc<FontCollection>,
}

impl FontCache {
    pub fn new() -> Self {
        Self {
            collection: FontCollection::system(),
        }
    }

    pub fn from_collection(collection: Arc<FontCollection>) -> Self {
        Self { collection }
    }

    pub fn collection(&self) -> &Arc<FontCollection> {
        &self.collection
    }

    pub fn get_font_platform_data(
        &mut self,
        family_name: &str,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        self.collection.resolve_family(family_name, description)
    }

    pub fn generic_family_name(generic: GenericFontFamily) -> &'static str {
        generic_family_name(generic)
    }

    pub fn clear(&mut self) {
        self.collection.clear_instances();
    }

    pub fn len(&self) -> usize {
        self.collection.stats().cached_instances
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn platform_fallback_for_character(
        &mut self,
        codepoint: char,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        self.collection
            .fallback_for_character(codepoint, description)
    }

    #[cfg(test)]
    fn make_key(family_name: &str, description: &FontDescription) -> FontCacheKey {
        let (style_tag, oblique_angle_bits) = match description.style {
            FontStyleEnum::Normal => (0, 0),
            FontStyleEnum::Italic => (1, 0),
            FontStyleEnum::Oblique(angle) => (2, angle.to_bits()),
        };
        FontCacheKey {
            family: family_name.to_ascii_lowercase(),
            size_bits: description.size.to_bits(),
            weight_bits: description.weight.0.to_bits(),
            stretch_bits: description.stretch.0.to_bits(),
            style_tag,
            oblique_angle_bits,
            native_control_text: description.native_control_text,
            embedded_document_text: description.embedded_document_text,
            native_button_text_metrics: description.native_button_text_metrics,
            raster_configuration: description.raster_configuration,
            device_scale_factor_bits: description.device_scale_factor.to_bits(),
        }
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_float_bits_participate_in_cache_identity() {
        let mut first = FontDescription::default();
        first.size = 16.001;
        let mut second = first.clone();
        second.size = 16.009;
        assert_ne!(
            FontCache::make_key("sans-serif", &first),
            FontCache::make_key("sans-serif", &second)
        );
    }

    #[test]
    fn raster_configuration_and_scale_participate_in_cache_identity() {
        let first = FontDescription::default();
        let mut scaled = first.clone();
        scaled.device_scale_factor = 2.0;
        let mut aliased = first.clone();
        aliased.raster_configuration = RasterConfiguration::deterministic_aliased(false);
        assert_ne!(
            FontCache::make_key("sans-serif", &first),
            FontCache::make_key("sans-serif", &scaled)
        );
        assert_ne!(
            FontCache::make_key("sans-serif", &first),
            FontCache::make_key("sans-serif", &aliased)
        );
    }

    #[test]
    fn cache_is_bounded_and_clearable() {
        let mut cache = FontCache::new();
        for size in 1..=300 {
            let mut description = FontDescription::default();
            description.size = size as f32;
            assert!(cache
                .get_font_platform_data("sans-serif", &description)
                .is_some());
        }
        assert!(cache.len() <= 256);
        cache.clear();
        assert!(cache.is_empty());
    }
}
