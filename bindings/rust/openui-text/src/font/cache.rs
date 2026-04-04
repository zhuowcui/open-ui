//! FontCache — global cache for resolved typefaces.
//!
//! Mirrors Blink's `FontCache` (platform/fonts/font_cache.h).
//! Maps (family_name, FontDescription) → `FontPlatformData` via Skia's
//! `SkFontMgr`. Cached entries are shared via `Arc`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use skia_safe::FontMgr;

use openui_style::GenericFontFamily;

use super::description::FontDescription;
use super::platform::FontPlatformData;

/// Cache key derived from the properties that affect typeface selection.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontCacheKey {
    family: String,
    /// Font size × 100, truncated to integer for stable hashing.
    size_hundredths: i32,
    /// Weight × 10, truncated to integer.
    weight_tenths: i32,
    /// Stretch × 10, truncated to integer.
    stretch_tenths: i32,
    /// 0 = normal, 1 = italic, 2 = oblique.
    style_tag: u8,
}

/// Wrapper to assert Send+Sync for FontMgr.
///
/// Skia's `SkFontMgr` is internally thread-safe (ref-counted, immutable after
/// creation). The skia-safe crate does not declare Send for all RCHandle types,
/// but FontMgr is safe to use from multiple threads.
struct SendFontMgr(FontMgr);

// SAFETY: SkFontMgr is internally thread-safe — it is immutable after creation
// and uses atomic reference counting. Blink itself uses FontMgr from multiple
// threads via its FontCache.
unsafe impl Send for SendFontMgr {}
unsafe impl Sync for SendFontMgr {}

/// Global font cache — resolves font descriptions to platform data.
///
/// Blink: `FontCache` singleton in `platform/fonts/font_cache.h`.
/// Uses Skia's `SkFontMgr` for typeface matching.
pub struct FontCache {
    font_mgr: SendFontMgr,
    cache: HashMap<FontCacheKey, Arc<FontPlatformData>>,
}

/// Global singleton font cache.
pub static GLOBAL_FONT_CACHE: LazyLock<Mutex<FontCache>> = LazyLock::new(|| {
    Mutex::new(FontCache::new())
});

impl FontCache {
    /// Create a new font cache with the system default font manager.
    pub fn new() -> Self {
        Self {
            font_mgr: SendFontMgr(FontMgr::default()),
            cache: HashMap::new(),
        }
    }

    /// Get or create platform font data for a specific family + description.
    ///
    /// Returns `None` if the family cannot be resolved to any typeface.
    pub fn get_font_platform_data(
        &mut self,
        family_name: &str,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        let key = Self::make_key(family_name, description);

        if let Some(data) = self.cache.get(&key) {
            return Some(Arc::clone(data));
        }

        // Resolve via Skia's SkFontMgr
        let sk_style = FontPlatformData::to_sk_font_style(
            description.weight.0,
            description.stretch.0,
            &description.style,
        );
        let typeface = self.font_mgr.0.match_family_style(family_name, sk_style)?;

        let data = Arc::new(FontPlatformData::new(typeface, description.size));
        self.cache.insert(key, Arc::clone(&data));
        Some(data)
    }

    /// Map a generic CSS font family to the string name passed to SkFontMgr.
    pub fn generic_family_name(generic: GenericFontFamily) -> &'static str {
        match generic {
            GenericFontFamily::Serif => "serif",
            GenericFontFamily::SansSerif => "sans-serif",
            GenericFontFamily::Monospace => "monospace",
            GenericFontFamily::Cursive => "cursive",
            GenericFontFamily::Fantasy => "fantasy",
            GenericFontFamily::SystemUi => "system-ui",
            GenericFontFamily::Math => "math",
            GenericFontFamily::Emoji => "emoji",
            GenericFontFamily::FangSong => "fangsong",
            GenericFontFamily::UiSerif => "ui-serif",
            GenericFontFamily::UiSansSerif => "ui-sans-serif",
            GenericFontFamily::UiMonospace => "ui-monospace",
            GenericFontFamily::UiRounded => "ui-rounded",
            GenericFontFamily::None => "sans-serif",
        }
    }

    /// Build a cache key from family name and description.
    fn make_key(family_name: &str, desc: &FontDescription) -> FontCacheKey {
        let style_tag = match desc.style {
            openui_style::FontStyleEnum::Normal => 0,
            openui_style::FontStyleEnum::Italic => 1,
            openui_style::FontStyleEnum::Oblique(_) => 2,
        };
        FontCacheKey {
            family: family_name.to_ascii_lowercase(),
            size_hundredths: (desc.size * 100.0) as i32,
            weight_tenths: (desc.weight.0 * 10.0) as i32,
            stretch_tenths: (desc.stretch.0 * 10.0) as i32,
            style_tag,
        }
    }

    /// Clear all cached entries (useful for testing).
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}
