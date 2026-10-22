//! Font subsystem — resolution, metrics, caching, and fallback.
//!
//! Architecture mirrors Blink's `platform/fonts/`:
//! - `FontDescription` — what CSS wants (family, weight, size, etc.)
//! - `FontPlatformData` — resolved Skia typeface + cached metrics
//! - `FontCollection` — document-owned faces and bounded instance cache
//! - `FontFallbackList` — ordered chain of resolved fonts for a description
//! - `Font` — main entry point combining description + fallback

pub mod cache;
mod collection;
mod description;
mod fallback;
pub mod features;
mod font;
mod metrics;
mod platform;
mod relative;
mod render_settings;
mod source;

pub use cache::FontCache;
#[doc(hidden)]
pub use collection::FontCacheLifetime;
pub use collection::{
    FontAxisRange, FontCollection, FontCollectionError, FontCollectionStats, FontContainerFormat,
    FontFaceDescriptor, FontFaceHandle, FontFaceInfo, FontFeatureDefault, FontMetricOverrides,
    FontPaletteBase, FontPaletteEntryOverride, FontPaletteHandle, FontPaletteValuesDescriptor,
    FontStyleRange, FontUnicodeRange,
};
pub use description::FontDescription;
pub use fallback::FontFallbackList;
pub use font::Font;
pub use metrics::{FontBaseline, FontMetrics};
pub use platform::{FontPlatformData, GlyphRasterizer};
pub use relative::{
    used_line_height, used_line_height_metrics, FontRelativeLengthResolver, FontRelativeUnit,
    UsedLineHeightMetrics,
};
pub use render_settings::{
    AppliedPlatformFontSettings, AppliedSkFontSettings, FontRenderHintStyle, FontRenderPreference,
    FontRenderStyle,
};
pub use source::{
    FontFaceSourceInfo, FontInstantiation, FontProviderConstruction, FontRenderQueryIntent,
    FontSelectionOrigin, FontSelectionSource,
};
