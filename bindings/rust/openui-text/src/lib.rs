//! openui-text — Font resolution, metrics, and text measurement.
//!
//! Extracted from Blink's font subsystem:
//! - `third_party/blink/renderer/platform/fonts/`
//!
//! This crate provides:
//! - Font resolution via Skia's `SkFontMgr`
//! - Font metrics extraction from `SkFontMetrics`
//! - Font caching and fallback chains
//! - Simple text measurement via `SkFont::measure_str`
//!
//! Text shaping (HarfBuzz integration) is planned for Wave 2.

pub mod font;
pub mod shaping;

pub use font::{
    Font, FontCache, FontDescription, FontFallbackList, FontMetrics, FontPlatformData,
};
