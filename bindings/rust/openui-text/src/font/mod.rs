//! Font subsystem — resolution, metrics, caching, and fallback.
//!
//! Architecture mirrors Blink's `platform/fonts/`:
//! - `FontDescription` — what CSS wants (family, weight, size, etc.)
//! - `FontPlatformData` — resolved Skia typeface + cached metrics
//! - `FontCache` — global cache mapping descriptions to platform data
//! - `FontFallbackList` — ordered chain of resolved fonts for a description
//! - `Font` — main entry point combining description + fallback

pub mod cache;
mod description;
mod fallback;
mod font;
mod metrics;
mod platform;

pub use cache::FontCache;
pub use description::FontDescription;
pub use fallback::FontFallbackList;
pub use font::Font;
pub use metrics::FontMetrics;
pub use platform::FontPlatformData;
