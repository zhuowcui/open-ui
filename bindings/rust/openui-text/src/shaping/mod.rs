//! Text shaping — HarfBuzz-based glyph shaping via Skia's SkShaper.
//!
//! Mirrors Blink's shaping subsystem (`platform/fonts/shaping/`).
//! Takes text + font and produces positioned glyphs for rendering.
//!
//! Architecture:
//! - `RunSegmenter` splits text into uniform runs by Unicode script.
//! - `TextShaper` shapes each run using HarfBuzz (via Skia's `SkShaper`).
//! - `ShapeResult` holds the output: glyph IDs, positions, and metadata.

mod segmenter;
mod shape_result;
mod shaper;

pub use segmenter::{RunSegment, RunSegmenter, Script};
pub use shape_result::{ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection};
pub use shaper::TextShaper;
