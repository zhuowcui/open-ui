//! FontCache — global cache for resolved typefaces.
//!
//! Mirrors Blink's `FontCache` (platform/fonts/font_cache.h).
//! Maps (family_name, FontDescription) → `FontPlatformData` via Skia's
//! `SkFontMgr`. Cached entries are shared via `Arc`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use skia_safe::textlayout::TypefaceFontProvider;
use skia_safe::utils::OrderedFontMgr;
use skia_safe::FontMgr;

use openui_style::GenericFontFamily;

use super::description::FontDescription;
use super::platform::FontPlatformData;

/// Fonts pinned in-process for deterministic pixel parity, independent of
/// ambient system font configuration. Each is registered under its CSS family
/// name ahead of the system font manager, so `match_family_style` resolves the
/// vendored `.ttf` (byte-identical to the Chromium reference's font) instead of
/// a system fallback. See `fonts/README.md`.
const PINNED_FONTS: &[(&str, &[u8])] = &[
    ("Ahem", include_bytes!("../../fonts/Ahem.ttf")),
    ("DejaVu Sans", include_bytes!("../../fonts/DejaVuSans.ttf")),
    (
        "DejaVu Sans",
        include_bytes!("../../fonts/DejaVuSans-Bold.ttf"),
    ),
    (
        "DejaVu Sans Mono",
        include_bytes!("../../fonts/DejaVuSansMono.ttf"),
    ),
    (
        "DejaVu Sans Mono",
        include_bytes!("../../fonts/DejaVuSansMono-Bold.ttf"),
    ),
    (
        "DejaVu Serif",
        include_bytes!("../../fonts/DejaVuSerif.ttf"),
    ),
    (
        "DejaVu Serif",
        include_bytes!("../../fonts/DejaVuSerif-Bold.ttf"),
    ),
];

/// Cache key derived from the properties that affect typeface selection.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontCacheKey {
    family: String,
    /// Font size as raw f32 bits for exact equality.
    size_bits: u32,
    /// Weight × 10, truncated to integer.
    weight_tenths: i32,
    /// Stretch × 10, truncated to integer.
    stretch_tenths: i32,
    /// 0 = normal, 1 = italic, 2 = oblique.
    style_tag: u8,
    /// Oblique angle as raw f32 bits (0 for normal/italic).
    /// Different oblique angles produce different synthetic skew transforms.
    oblique_angle_bits: u32,
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
pub static GLOBAL_FONT_CACHE: LazyLock<Mutex<FontCache>> =
    LazyLock::new(|| Mutex::new(FontCache::new()));

impl FontCache {
    /// Create a new font cache with the system default font manager.
    pub fn new() -> Self {
        Self {
            font_mgr: SendFontMgr(Self::build_font_mgr()),
            cache: HashMap::new(),
        }
    }

    /// Build the typeface manager: pinned deterministic fonts first (matched by
    /// family name), then the system default manager for everything else. If no
    /// pinned font loads, fall back to the plain system manager so resolution of
    /// all other families is unchanged.
    fn build_font_mgr() -> FontMgr {
        let system = FontMgr::default();
        let mut provider = TypefaceFontProvider::new();
        let mut pinned = 0usize;
        for &(family, bytes) in PINNED_FONTS {
            if let Some(typeface) = system.new_from_data(bytes, None) {
                provider.register_typeface(typeface, Some(family));
                pinned += 1;
            }
        }
        if pinned == 0 {
            return system;
        }
        let mut ordered = OrderedFontMgr::new();
        ordered.append(provider);
        ordered.append(system);
        ordered.into()
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
        let mut family = self.font_mgr.0.match_family(family_name);
        let family_count = family.count();
        let exact_index = (0..family_count).find(|&index| family.style(index).0 == sk_style);
        let typeface = if let Some(index) = exact_index {
            family.new_typeface(index)?
        } else if family_count == 1 {
            // A single-face family cannot supply a distinct requested style.
            // Use the concrete face and let FontPlatformData synthesize the
            // CSS weight/slant instead of accepting Skia's styled proxy.
            family.new_typeface(0)?
        } else {
            family.match_style(sk_style)?
        };

        // Extract oblique angle for synthetic oblique synthesis.
        let oblique_angle = match description.style {
            openui_style::FontStyleEnum::Oblique(angle) => angle,
            _ => 0.0,
        };

        let data = Arc::new(FontPlatformData::with_synthetic_styles(
            typeface,
            description.size,
            oblique_angle,
            skia_safe::font_style::Weight::from(description.weight.0 as i32),
        ));
        self.cache.insert(key, Arc::clone(&data));
        Some(data)
    }

    /// Map a generic CSS font family to the string name passed to SkFontMgr.
    pub fn generic_family_name(generic: GenericFontFamily) -> &'static str {
        match generic {
            GenericFontFamily::Serif | GenericFontFamily::UiSerif => "DejaVu Serif",
            GenericFontFamily::SansSerif
            | GenericFontFamily::SystemUi
            | GenericFontFamily::UiSansSerif
            | GenericFontFamily::UiRounded => "DejaVu Sans",
            GenericFontFamily::Monospace | GenericFontFamily::UiMonospace => "DejaVu Sans Mono",
            GenericFontFamily::Cursive => "cursive",
            GenericFontFamily::Fantasy => "fantasy",
            GenericFontFamily::Math => "math",
            GenericFontFamily::Emoji => "emoji",
            GenericFontFamily::FangSong => "fangsong",
            GenericFontFamily::None => "DejaVu Sans",
        }
    }

    /// Build a cache key from family name and description.
    fn make_key(family_name: &str, desc: &FontDescription) -> FontCacheKey {
        let (style_tag, oblique_angle_bits) = match desc.style {
            openui_style::FontStyleEnum::Normal => (0, 0u32),
            openui_style::FontStyleEnum::Italic => (1, 0u32),
            openui_style::FontStyleEnum::Oblique(angle) => (2, angle.to_bits()),
        };
        FontCacheKey {
            family: family_name.to_ascii_lowercase(),
            size_bits: desc.size.to_bits(),
            weight_tenths: (desc.weight.0 * 10.0) as i32,
            stretch_tenths: (desc.stretch.0 * 10.0) as i32,
            style_tag,
            oblique_angle_bits,
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

    /// Find a platform font covering a specific Unicode code point.
    ///
    /// Delegates to Skia's `FontMgr::match_family_style_character()` which
    /// queries the OS font registry. This is the last-resort fallback after
    /// the CSS font-family list is exhausted.
    ///
    /// Blink: `FontCache::PlatformFallbackFontForCharacter`.
    pub fn platform_fallback_for_character(
        &mut self,
        codepoint: char,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        let sk_style = FontPlatformData::to_sk_font_style(
            description.weight.0,
            description.stretch.0,
            &description.style,
        );
        let locale_owned: String;
        let bcp47: Vec<&str> = if description.locale.as_ref().map_or(true, |l| l.is_empty()) {
            vec!["en"]
        } else {
            locale_owned = description.locale.as_ref().unwrap().clone();
            vec![locale_owned.as_str()]
        };
        let bcp47_slice: &[&str] = &bcp47;
        let typeface = self.font_mgr.0.match_family_style_character(
            "",
            sk_style,
            bcp47_slice,
            codepoint as i32,
        )?;
        // Thread the oblique angle from the font description so that oblique
        // text falling back for emoji/CJK still gets the synthetic skew.
        let oblique_angle = match description.style {
            openui_style::FontStyleEnum::Oblique(angle) => angle,
            _ => 0.0,
        };
        let data = Arc::new(FontPlatformData::with_synthetic_styles(
            typeface,
            description.size,
            oblique_angle,
            skia_safe::font_style::Weight::from(description.weight.0 as i32),
        ));
        Some(data)
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

    // ── SP11 Round 14 Issue 6: cache key precision ──────────────────

    #[test]
    fn cache_key_distinguishes_very_similar_sizes() {
        // Sizes 16.001 and 16.009 differ by less than 0.01 and would
        // collide under the old (size * 100) as i32 truncation.
        // With f32::to_bits() they must produce distinct keys.
        let mut desc1 = FontDescription::default();
        desc1.size = 16.001;
        let mut desc2 = FontDescription::default();
        desc2.size = 16.009;

        let key1 = FontCache::make_key("sans-serif", &desc1);
        let key2 = FontCache::make_key("sans-serif", &desc2);

        assert_ne!(
            key1, key2,
            "Distinct float sizes should produce distinct cache keys"
        );
    }

    #[test]
    fn cache_key_same_size_same_key() {
        let mut desc1 = FontDescription::default();
        desc1.size = 16.0;
        let mut desc2 = FontDescription::default();
        desc2.size = 16.0;

        let key1 = FontCache::make_key("sans-serif", &desc1);
        let key2 = FontCache::make_key("sans-serif", &desc2);

        assert_eq!(key1, key2, "Identical sizes should produce identical keys");
    }

    // ── Issue 6 (R26): oblique angle in cache key ───────────────────────

    #[test]
    fn cache_key_distinguishes_oblique_angles() {
        // Oblique(14) and Oblique(20) should produce distinct keys.
        let mut desc1 = FontDescription::default();
        desc1.style = openui_style::FontStyleEnum::Oblique(14.0);
        let mut desc2 = FontDescription::default();
        desc2.style = openui_style::FontStyleEnum::Oblique(20.0);

        let key1 = FontCache::make_key("sans-serif", &desc1);
        let key2 = FontCache::make_key("sans-serif", &desc2);

        assert_ne!(
            key1, key2,
            "Different oblique angles should produce distinct cache keys"
        );
    }

    #[test]
    fn cache_key_oblique_same_angle_same_key() {
        // Two oblique with same angle should produce identical keys.
        let mut desc1 = FontDescription::default();
        desc1.style = openui_style::FontStyleEnum::Oblique(14.0);
        let mut desc2 = FontDescription::default();
        desc2.style = openui_style::FontStyleEnum::Oblique(14.0);

        let key1 = FontCache::make_key("sans-serif", &desc1);
        let key2 = FontCache::make_key("sans-serif", &desc2);

        assert_eq!(
            key1, key2,
            "Same oblique angle should produce identical cache keys"
        );
    }

    #[test]
    fn generic_families_resolve_to_vendored_dejavu_faces() {
        assert_eq!(
            FontCache::generic_family_name(GenericFontFamily::SansSerif),
            "DejaVu Sans"
        );
        assert_eq!(
            FontCache::generic_family_name(GenericFontFamily::Monospace),
            "DejaVu Sans Mono"
        );
        assert_eq!(
            FontCache::generic_family_name(GenericFontFamily::Serif),
            "DejaVu Serif"
        );
    }

    #[test]
    fn regular_and_bold_select_matching_vendored_styles() {
        let mut cache = FontCache::new();
        let regular = FontDescription::default();
        let mut bold = regular.clone();
        bold.weight = openui_style::FontWeight::BOLD;
        let regular_face = cache
            .get_font_platform_data("DejaVu Sans", &regular)
            .expect("vendored regular face");
        let bold_face = cache
            .get_font_platform_data("DejaVu Sans", &bold)
            .expect("vendored bold face");
        assert_eq!(regular_face.typeface().family_name(), "DejaVu Sans");
        assert_eq!(bold_face.typeface().family_name(), "DejaVu Sans");
        assert_ne!(
            regular_face.typeface().font_style().weight(),
            bold_face.typeface().font_style().weight()
        );
    }

    #[test]
    fn vendored_dejavu_sans_has_exact_16px_metrics() {
        let mut cache = FontCache::new();
        let face = cache
            .get_font_platform_data("DejaVu Sans", &FontDescription::default())
            .expect("vendored DejaVu Sans");
        let metrics = face.metrics();
        assert_eq!(metrics.ascent, 14.8515625);
        assert_eq!(metrics.descent, 3.7734375);
        assert_eq!(metrics.line_gap, 0.0);
        assert_eq!(metrics.line_spacing, 18.625);
        assert_eq!(metrics.x_height, 9.0);
        assert_eq!(metrics.zero_width, 10.1796875);
        assert_eq!(metrics.units_per_em, 2048);
    }

    #[test]
    fn resolved_faces_are_arc_cached_and_ahem_stays_isolated() {
        let mut cache = FontCache::new();
        let description = FontDescription::default();
        let first = cache
            .get_font_platform_data("DejaVu Sans", &description)
            .expect("vendored DejaVu Sans");
        let second = cache
            .get_font_platform_data("DejaVu Sans", &description)
            .expect("cached DejaVu Sans");
        let ahem = cache
            .get_font_platform_data("Ahem", &description)
            .expect("vendored Ahem");
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(cache.len(), 2);
        assert_eq!(ahem.typeface().family_name(), "Ahem");
        assert_ne!(ahem.typeface().unique_id(), first.typeface().unique_id());
    }
}
