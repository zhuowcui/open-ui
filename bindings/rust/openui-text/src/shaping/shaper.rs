//! TextShaper — HarfBuzz-based text shaping via Skia's SkShaper.
//!
//! Mirrors Blink's `HarfBuzzShaper` (`platform/fonts/shaping/harfbuzz_shaper.h`).
//! Uses Skia's `SkShaper` (which wraps HarfBuzz) to convert text + font into
//! positioned glyphs with cluster information for cursor placement and hit testing.

use std::sync::Arc;

use skia_safe::{
    shaper::{run_handler, RunHandler as SkRunHandler},
    GlyphId, Point, Shaper, Vector,
};

use crate::font::{Font, FontPlatformData};

use super::shape_result::{ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection};

/// Collects shaping output from Skia's SkShaper callbacks.
///
/// Implements Skia's `RunHandler` trait to receive glyph data during shaping.
struct ShapeCollector {
    /// The font platform data for associating with runs.
    font_data: Arc<FontPlatformData>,
    /// Text direction.
    direction: TextDirection,
    /// Collected runs.
    runs: Vec<CollectedRun>,
    /// Storage for the current run being filled by the shaper.
    current_glyphs: Vec<GlyphId>,
    current_positions: Vec<Point>,
    current_offsets: Vec<Point>,
    current_clusters: Vec<u32>,
    /// Info about the current run.
    current_glyph_count: usize,
    current_utf8_range: std::ops::Range<usize>,
    current_advance: Vector,
}

/// A fully collected glyph run.
struct CollectedRun {
    glyphs: Vec<GlyphId>,
    positions: Vec<Point>,
    offsets: Vec<Point>,
    clusters: Vec<u32>,
    utf8_range: std::ops::Range<usize>,
    advance: Vector,
}

impl ShapeCollector {
    fn new(font_data: Arc<FontPlatformData>, direction: TextDirection) -> Self {
        Self {
            font_data,
            direction,
            runs: Vec::new(),
            current_glyphs: Vec::new(),
            current_positions: Vec::new(),
            current_offsets: Vec::new(),
            current_clusters: Vec::new(),
            current_glyph_count: 0,
            current_utf8_range: 0..0,
            current_advance: Vector::default(),
        }
    }

    /// Convert collected runs into a ShapeResult.
    fn into_shape_result(self, text: &str) -> ShapeResult {
        let num_characters = text.chars().count();
        if num_characters == 0 {
            return ShapeResult::empty(self.direction);
        }

        let mut result_runs = Vec::new();
        let mut total_width = 0.0f32;

        // Build a byte-offset-to-char-index table for cluster mapping.
        let byte_to_char: Vec<usize> = Self::build_byte_to_char_map(text);

        // Per-character advance accumulator.
        let mut char_advances = vec![0.0f32; num_characters];

        for collected in &self.runs {
            let mut run_glyphs = Vec::with_capacity(collected.glyphs.len());
            let mut run_advances = Vec::with_capacity(collected.glyphs.len());
            let mut run_offsets = Vec::with_capacity(collected.glyphs.len());
            let mut run_clusters = Vec::with_capacity(collected.glyphs.len());

            for i in 0..collected.glyphs.len() {
                run_glyphs.push(collected.glyphs[i]);

                // Compute advance from position differences.
                let advance = if i + 1 < collected.positions.len() {
                    collected.positions[i + 1].x - collected.positions[i].x
                } else {
                    // Last glyph: use the run's total advance minus position.
                    collected.advance.x - collected.positions[i].x
                        + collected.positions[0].x
                };
                run_advances.push(advance);

                let offset = if !collected.offsets.is_empty() {
                    (collected.offsets[i].x, collected.offsets[i].y)
                } else {
                    (0.0, 0.0)
                };
                run_offsets.push(offset);

                // Map this glyph's cluster (byte offset) to a character index
                // and distribute the advance to that character.
                if !collected.clusters.is_empty() {
                    let cluster_byte = collected.clusters[i] as usize;
                    if cluster_byte < byte_to_char.len() {
                        let char_idx = byte_to_char[cluster_byte];
                        if char_idx < num_characters {
                            char_advances[char_idx] += advance;
                        }
                    }
                }
            }

            // Determine start character index from the run's utf8 range.
            let start_char = if collected.utf8_range.start < byte_to_char.len() {
                byte_to_char[collected.utf8_range.start]
            } else {
                0
            };
            let end_char = if collected.utf8_range.end <= byte_to_char.len() {
                if collected.utf8_range.end == byte_to_char.len() {
                    num_characters
                } else {
                    byte_to_char[collected.utf8_range.end]
                }
            } else {
                num_characters
            };

            // Build per-glyph cluster mapping (character index relative to run start).
            for i in 0..collected.glyphs.len() {
                if !collected.clusters.is_empty() {
                    let cluster_byte = collected.clusters[i] as usize;
                    if cluster_byte < byte_to_char.len() {
                        let char_idx = byte_to_char[cluster_byte];
                        run_clusters.push(char_idx.saturating_sub(start_char));
                    } else {
                        run_clusters.push(0);
                    }
                } else {
                    // No cluster data: assume 1:1 mapping.
                    run_clusters.push(i);
                }
            }

            let run_num_chars = end_char.saturating_sub(start_char);
            let run_num_glyphs = run_glyphs.len();

            total_width += run_advances.iter().sum::<f32>();

            result_runs.push(ShapeResultRun {
                font_data: Arc::clone(&self.font_data),
                glyphs: run_glyphs,
                advances: run_advances,
                offsets: run_offsets,
                clusters: run_clusters,
                start_index: start_char,
                num_characters: run_num_chars,
                num_glyphs: run_num_glyphs,
                direction: self.direction,
            });
        }

        // If no runs collected advances (e.g., no cluster data), distribute
        // width evenly as fallback.
        if char_advances.iter().all(|a| *a == 0.0) && total_width > 0.0 && num_characters > 0 {
            let per_char = total_width / num_characters as f32;
            for a in &mut char_advances {
                *a = per_char;
            }
        }

        // Build character data from accumulated advances.
        let mut character_data = Vec::with_capacity(num_characters);
        let mut x = 0.0f32;
        let chars: Vec<char> = text.chars().collect();

        // Determine cluster bases from cluster info.
        let cluster_bases = Self::compute_cluster_bases(text, &self.runs, &byte_to_char);
        let safe_breaks = Self::compute_safe_breaks(&self.runs, &chars, &byte_to_char);

        for i in 0..num_characters {
            character_data.push(ShapeResultCharacterData {
                x_position: x,
                is_cluster_base: cluster_bases.get(i).copied().unwrap_or(true),
                safe_to_break_before: safe_breaks.get(i).copied().unwrap_or(true),
            });
            x += char_advances[i];
        }

        ShapeResult {
            runs: result_runs,
            width: total_width,
            num_characters,
            direction: self.direction,
            character_data,
        }
    }

    /// Build a mapping from byte offset to character index.
    fn build_byte_to_char_map(text: &str) -> Vec<usize> {
        let mut map = vec![0usize; text.len() + 1];
        let mut char_idx = 0;
        for (byte_idx, _) in text.char_indices() {
            map[byte_idx] = char_idx;
            char_idx += 1;
        }
        // Fill in byte offsets within multi-byte characters.
        if !text.is_empty() {
            map[text.len()] = char_idx;
            let mut last = 0;
            for i in 0..=text.len() {
                if i == 0 || map[i] != 0 || i == text.len() {
                    last = map[i];
                } else {
                    map[i] = last;
                }
            }
        }
        map
    }

    /// Determine which characters are cluster bases.
    ///
    /// A character is a cluster base if at least one glyph's cluster value
    /// maps to it. Characters that no glyph points to (e.g., the second
    /// character in a ligature) are non-base. This matches HarfBuzz's
    /// cluster model used by Blink's `ShapeResult`.
    fn compute_cluster_bases(
        text: &str,
        runs: &[CollectedRun],
        byte_to_char: &[usize],
    ) -> Vec<bool> {
        let num_chars = text.chars().count();
        if num_chars == 0 {
            return Vec::new();
        }

        let mut bases = vec![false; num_chars];
        let mut had_clusters = false;

        for run in runs {
            if run.clusters.is_empty() {
                continue;
            }
            had_clusters = true;

            // Each unique cluster value maps to a cluster-base character.
            let mut seen_clusters = std::collections::HashSet::new();
            for &cluster in &run.clusters {
                if seen_clusters.insert(cluster) {
                    let byte_off = cluster as usize;
                    if byte_off < byte_to_char.len() {
                        let char_idx = byte_to_char[byte_off];
                        if char_idx < num_chars {
                            bases[char_idx] = true;
                        }
                    }
                }
            }
        }

        // If no cluster data was available, treat all characters as bases.
        if !had_clusters {
            for b in &mut bases {
                *b = true;
            }
        }

        bases
    }

    /// Determine safe-to-break points.
    ///
    /// A character position is safe to break before if reshaping the text
    /// from that point onward would produce the same glyphs. For fully
    /// accurate complex-script reshaping (e.g., Arabic initial/medial/final
    /// forms), HarfBuzz's `HB_GLYPH_FLAG_UNSAFE_TO_BREAK` flag would be
    /// needed, which is not exposed through SkShaper. This implementation
    /// uses whitespace boundaries and cluster boundaries as a reasonable
    /// approximation that works well for Latin, CJK, and most other scripts.
    fn compute_safe_breaks(runs: &[CollectedRun], chars: &[char], byte_to_char: &[usize]) -> Vec<bool> {
        let mut safe = vec![false; chars.len()];
        if safe.is_empty() {
            return safe;
        }
        // Always safe to break at start.
        safe[0] = true;

        // Safe at whitespace boundaries.
        for (i, &ch) in chars.iter().enumerate() {
            if ch.is_whitespace() {
                safe[i] = true;
                if i + 1 < chars.len() {
                    safe[i + 1] = true;
                }
            }
        }

        // Also safe at cluster boundaries within runs — where the cluster
        // value changes, glyphs can generally be reshaped independently.
        for run in runs {
            if run.clusters.len() > 1 {
                for i in 1..run.clusters.len() {
                    if run.clusters[i] != run.clusters[i - 1] {
                        let byte_off = run.clusters[i] as usize;
                        if byte_off < byte_to_char.len() {
                            let char_idx = byte_to_char[byte_off];
                            if char_idx < chars.len() {
                                safe[char_idx] = true;
                            }
                        }
                    }
                }
            }
        }

        safe
    }
}

impl SkRunHandler for ShapeCollector {
    fn begin_line(&mut self) {
        // Single-line shaping — nothing to do.
    }

    fn run_info(&mut self, info: &run_handler::RunInfo) {
        self.current_glyph_count = info.glyph_count;
        self.current_utf8_range = info.utf8_range.clone();
        self.current_advance = info.advance;
    }

    fn commit_run_info(&mut self) {
        // Allocate storage for the run.
        let n = self.current_glyph_count;
        self.current_glyphs.clear();
        self.current_glyphs.resize(n, 0);
        self.current_positions.clear();
        self.current_positions.resize(n, Point::default());
        self.current_offsets.clear();
        self.current_offsets.resize(n, Point::default());
        self.current_clusters.clear();
        self.current_clusters.resize(n, 0);
    }

    fn run_buffer(&mut self, _info: &run_handler::RunInfo) -> run_handler::Buffer<'_> {
        run_handler::Buffer {
            glyphs: &mut self.current_glyphs,
            positions: &mut self.current_positions,
            offsets: Some(&mut self.current_offsets),
            clusters: Some(&mut self.current_clusters),
            point: Point::default(),
        }
    }

    fn commit_run_buffer(&mut self, _info: &run_handler::RunInfo) {
        // Save the completed run.
        self.runs.push(CollectedRun {
            glyphs: self.current_glyphs.clone(),
            positions: self.current_positions.clone(),
            offsets: self.current_offsets.clone(),
            clusters: self.current_clusters.clone(),
            utf8_range: self.current_utf8_range.clone(),
            advance: self.current_advance,
        });
    }

    fn commit_line(&mut self) {
        // Single-line shaping — nothing to do.
    }
}

/// Text shaper — shapes text using HarfBuzz via Skia's SkShaper.
///
/// Mirrors Blink's `HarfBuzzShaper` (`platform/fonts/shaping/harfbuzz_shaper.h`).
pub struct TextShaper {
    shaper: Shaper,
}

impl TextShaper {
    /// Create a new TextShaper with the default Skia shaper (HarfBuzz backend).
    pub fn new() -> Self {
        Self {
            shaper: Shaper::new(None),
        }
    }

    /// Shape text using HarfBuzz via Skia's SkShaper.
    ///
    /// This is the main entry point for text shaping. Takes text and a Font,
    /// runs HarfBuzz shaping, and returns a ShapeResult with positioned glyphs.
    ///
    /// Blink: `HarfBuzzShaper::Shape` in `harfbuzz_shaper.cc`.
    pub fn shape(&self, text: &str, font: &Font, direction: TextDirection) -> ShapeResult {
        if text.is_empty() {
            return ShapeResult::empty(direction);
        }

        let font_data = match font.primary_font() {
            Some(fd) => Arc::clone(fd),
            None => return ShapeResult::empty(direction),
        };

        let sk_font = font_data.sk_font();
        let left_to_right = direction.is_ltr();

        let mut collector = ShapeCollector::new(Arc::clone(&font_data), direction);

        // Use f32::INFINITY for width to disable line wrapping — we shape
        // the entire string as a single line.
        self.shaper
            .shape(text, sk_font, left_to_right, f32::INFINITY, &mut collector);

        let mut result = collector.into_shape_result(text);

        // Apply letter spacing and word spacing from the font description.
        Self::apply_spacing(&mut result, font, text);

        result
    }

    /// Apply letter spacing and word spacing after shaping.
    ///
    /// Blink: `ShapeResult::ApplySpacing` in `shape_result.cc`.
    fn apply_spacing(result: &mut ShapeResult, font: &Font, text: &str) {
        let desc = font.description();
        let letter_spacing = desc.letter_spacing;
        let word_spacing = desc.word_spacing;

        if letter_spacing == 0.0 && word_spacing == 0.0 {
            return;
        }

        let chars: Vec<char> = text.chars().collect();
        let num_chars = chars.len();
        if num_chars == 0 {
            return;
        }

        // Build a map from glyph-run positions back to character indices.
        let mut extra_advance_per_char = vec![0.0f32; num_chars];

        for (char_idx, &ch) in chars.iter().enumerate() {
            // Letter spacing: applied to every character.
            extra_advance_per_char[char_idx] += letter_spacing;

            // Word spacing: applied only to space characters (U+0020).
            if ch == ' ' {
                extra_advance_per_char[char_idx] += word_spacing;
            }
        }

        // Distribute the extra advance to glyph runs.
        // For ligature or decomposition runs (non-1:1 glyph/character mapping),
        // accumulate spacing for ALL characters in the run rather than mapping
        // glyphs individually.
        let mut total_extra = 0.0f32;
        for run in &mut result.runs {
            let run_start = run.start_index;
            let run_chars = run.num_characters;

            if run.num_glyphs == run_chars {
                // 1:1 mapping — apply per glyph directly.
                for i in 0..run.num_glyphs {
                    let char_idx = run_start + i;
                    if char_idx < num_chars {
                        run.advances[i] += extra_advance_per_char[char_idx];
                        total_extra += extra_advance_per_char[char_idx];
                    }
                }
            } else {
                // Ligature or decomposition — accumulate ALL character spacing
                // and add to the last glyph of the run so the total advance is
                // correct.
                let mut run_extra = 0.0f32;
                for ci in run_start..run_start + run_chars {
                    if ci < num_chars {
                        run_extra += extra_advance_per_char[ci];
                    }
                }
                if run.num_glyphs > 0 {
                    run.advances[run.num_glyphs - 1] += run_extra;
                }
                total_extra += run_extra;
            }
        }

        result.width += total_extra;

        // Rebuild character data x_positions with spacing applied.
        if !result.character_data.is_empty() {
            let mut x = 0.0f32;
            for i in 0..num_chars {
                result.character_data[i].x_position = x;
                // Compute per-character advance from runs.
                let char_width = Self::char_advance_from_runs(&result.runs, i);
                x += char_width;
            }
        }
    }

    /// Compute the advance width for a specific character from the glyph runs.
    fn char_advance_from_runs(runs: &[ShapeResultRun], char_idx: usize) -> f32 {
        for run in runs {
            let run_start = run.start_index;
            let run_end = run.start_index + run.num_characters;
            if char_idx >= run_start && char_idx < run_end {
                let local_idx = char_idx - run_start;
                if run.num_glyphs == run.num_characters {
                    return run.advances[local_idx];
                } else {
                    // Non-1:1 mapping: distribute run width proportionally.
                    let total: f32 = run.advances.iter().sum();
                    return total / run.num_characters as f32;
                }
            }
        }
        0.0
    }
}

impl Default for TextShaper {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TextShaper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextShaper").finish()
    }
}
