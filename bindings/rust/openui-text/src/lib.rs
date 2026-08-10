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
pub mod char_orientation;
pub mod emoji;
pub mod emphasis;
pub mod font;
pub mod hyphenation;
pub mod shaping;
pub mod transform;

pub use bidi::{BidiParagraph, BidiRun};
pub use char_orientation::is_upright_in_mixed_vertical;
pub use emoji::{
    contains_emoji, is_emoji, is_emoji_modifier, is_emoji_presentation_selector,
    is_regional_indicator, is_tag_character, is_text_presentation_selector, is_zero_width_joiner,
};
pub use emphasis::{
    default_mark_for_writing_mode, default_position_for_writing_mode, resolve_emphasis_mark,
    should_draw_emphasis_mark, ResolvedEmphasisMark,
};
pub use transform::apply_text_transform;

pub use font::features::collect_font_features;
pub use font::{
    used_line_height, used_line_height_metrics, Font, FontCache, FontDescription, FontFallbackList,
    FontMetrics, FontPlatformData, FontRelativeLengthResolver, FontRelativeUnit,
    UsedLineHeightMetrics,
};

pub use shaping::{
    RunSegment, RunSegmenter, ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection,
    TextShaper,
};

pub use hyphenation::{
    find_soft_hyphens, is_soft_hyphen, last_soft_hyphen_before, strip_soft_hyphens, Hyphenation,
    SOFT_HYPHEN,
};
