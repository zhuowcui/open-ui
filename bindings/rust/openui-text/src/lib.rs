//! openui-text — Font resolution, metrics, text measurement, and shaping.
//!
//! Extracted from Blink's font subsystem:
//! - `third_party/blink/renderer/platform/fonts/`
//! - `third_party/blink/renderer/platform/fonts/shaping/`
//!
//! This crate provides:
//! - Font resolution via Skia's `SkFontMgr`
//! - Font metrics extraction from `SkFontMetrics`
//! - Font caching and fallback chains
//! - Simple text measurement via `SkFont::measure_str`
//! - HarfBuzz-based text shaping via Skia's `SkShaper`

pub mod font;
pub mod shaping;

pub use font::{
    Font, FontCache, FontDescription, FontFallbackList, FontMetrics, FontPlatformData,
};

pub use shaping::{
    RunSegment, RunSegmenter, ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection,
    TextShaper,
};
