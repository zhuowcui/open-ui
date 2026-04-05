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
//! - Bidirectional text analysis (UAX#9) via `unicode-bidi`
//! - Text transform (uppercase, lowercase, capitalize, full-width)

pub mod bidi;
pub mod emphasis;
pub mod font;
pub mod shaping;
pub mod transform;

pub use bidi::{BidiParagraph, BidiRun};
pub use emphasis::{
    ResolvedEmphasisMark, default_mark_for_writing_mode, default_position_for_writing_mode,
    resolve_emphasis_mark, should_draw_emphasis_mark,
};
pub use transform::apply_text_transform;

pub use font::{
    Font, FontCache, FontDescription, FontFallbackList, FontMetrics, FontPlatformData,
};
pub use font::features::collect_font_features;

pub use shaping::{
    RunSegment, RunSegmenter, ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection,
    TextShaper,
};
