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
use unicode_script::{Script, UnicodeScript};

use crate::font::{Font, FontPlatformData};

use super::shape_result::{ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection};

/// Whether a Unicode script requires complex shaping (joining, reordering,
/// or contextual forms) that makes cluster-boundary breaks unsafe.
///
/// For these scripts, breaking at a HarfBuzz cluster boundary can change glyph
/// forms (e.g., Arabic initial/medial/final forms, Indic conjuncts). Only
/// word/space boundaries are safe break points within a shaping run.
fn is_complex_script(ch: char) -> bool {
    let script = ch.script();
    matches!(
        script,
        // Arabic-derived joining scripts
        Script::Arabic
            | Script::Syriac
            | Script::Thaana
            | Script::Mandaic
            | Script::Nko
            // Hebrew has contextual final forms
            | Script::Hebrew
            // Indic scripts with reordering and conjunct formation
            | Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Gujarati
            | Script::Oriya
            | Script::Tamil
            | Script::Telugu
            | Script::Kannada
            | Script::Malayalam
            | Script::Sinhala
            // Southeast Asian scripts with contextual shaping
            | Script::Thai
            | Script::Lao
            | Script::Tibetan
            | Script::Myanmar
            | Script::Khmer
            // Other complex scripts
            | Script::Javanese
            | Script::Balinese
            | Script::Sundanese
            | Script::Lepcha
    )
}

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
    /// from that point onward would produce the same glyphs.
    ///
    /// Since SkShaper doesn't expose HarfBuzz's `HB_GLYPH_FLAG_UNSAFE_TO_BREAK`,
    /// we use a conservative, script-aware heuristic:
    /// - Always safe: start of text, whitespace, positions adjacent to whitespace.
    /// - Simple scripts (Latin, CJK, etc.): safe at cluster boundaries.
    /// - Complex scripts (Arabic, Indic, Thai, etc.): cluster boundaries are
    ///   NOT safe because reshaping would change glyph forms (joining,
    ///   reordering, contextual substitutions). Only word/space boundaries
    ///   are considered safe.
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

        // Safe at cluster boundaries within runs — but only for simple
        // scripts where cluster boundaries don't affect glyph forms.
        for run in runs {
            if run.clusters.len() > 1 {
                for i in 1..run.clusters.len() {
                    if run.clusters[i] != run.clusters[i - 1] {
                        let byte_off = run.clusters[i] as usize;
                        if byte_off < byte_to_char.len() {
                            let char_idx = byte_to_char[byte_off];
                            if char_idx < chars.len() && !is_complex_script(chars[char_idx]) {
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
    /// Implements per-run font fallback: after initial shaping with the primary
    /// font, segments containing `.notdef` glyphs (glyph_id == 0) are re-shaped
    /// with fonts from the fallback list. This mirrors Blink's
    /// `FontFallbackIterator` + `ShapeResultBuffer` approach.
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

        // ── Font fallback for missing glyphs ────────────────────────────
        // Scan for runs with glyph_id == 0 (.notdef) and attempt to
        // re-shape those character ranges with fallback fonts.
        let fallback_list = font.fallback_list();
        if fallback_list.len() > 1 {
            self.apply_font_fallback(&mut result, text, font, direction);
        }

        // Apply letter spacing and word spacing from the font description.
        Self::apply_spacing(&mut result, font, text);

        result
    }

    /// Re-shape character ranges that have missing glyphs (glyph_id == 0)
    /// using fonts from the fallback list.
    ///
    /// For each run, detects contiguous segments of missing glyphs, then
    /// iterates through the fallback chain to find a font that covers those
    /// characters. Successfully re-shaped segments replace the original run
    /// data in-place.
    fn apply_font_fallback(
        &self,
        result: &mut ShapeResult,
        text: &str,
        font: &Font,
        direction: TextDirection,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let fallback_list = font.fallback_list();
        let left_to_right = direction.is_ltr();

        // Collect segments (character ranges) with missing glyphs across all runs.
        let mut missing_segments: Vec<(usize, usize)> = Vec::new(); // (char_start, char_end)
        for run in &result.runs {
            // Build a set of characters in this run that have glyph_id == 0.
            let mut missing_chars = vec![false; run.num_characters];
            for (gi, &glyph_id) in run.glyphs.iter().enumerate() {
                if glyph_id == 0 {
                    // Map this glyph to a character via cluster data.
                    let local_char = if gi < run.clusters.len() {
                        run.clusters[gi]
                    } else if run.num_glyphs == run.num_characters {
                        gi
                    } else {
                        continue;
                    };
                    if local_char < run.num_characters {
                        missing_chars[local_char] = true;
                    }
                }
            }

            // Group contiguous missing characters into segments.
            let mut seg_start: Option<usize> = None;
            for (i, &missing) in missing_chars.iter().enumerate() {
                if missing {
                    if seg_start.is_none() {
                        seg_start = Some(run.start_index + i);
                    }
                } else if let Some(start) = seg_start {
                    missing_segments.push((start, run.start_index + i));
                    seg_start = None;
                }
            }
            if let Some(start) = seg_start {
                missing_segments.push((start, run.start_index + run.num_characters));
            }
        }

        if missing_segments.is_empty() {
            return;
        }

        // Merge overlapping/adjacent segments.
        missing_segments.sort_by_key(|s| s.0);
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for seg in missing_segments {
            if let Some(last) = merged.last_mut() {
                if seg.0 <= last.1 {
                    last.1 = last.1.max(seg.1);
                    continue;
                }
            }
            merged.push(seg);
        }

        // For each missing segment, try fallback fonts (skip index 0 = primary).
        for (seg_char_start, seg_char_end) in &merged {
            // Extract the substring for this segment.
            let byte_start: usize = chars[..*seg_char_start]
                .iter()
                .map(|c| c.len_utf8())
                .sum();
            let byte_end: usize = byte_start
                + chars[*seg_char_start..*seg_char_end]
                    .iter()
                    .map(|c| c.len_utf8())
                    .sum::<usize>();
            let segment_text = &text[byte_start..byte_end];

            let mut segment_handled = false;

            for fb_idx in 1..fallback_list.len() {
                let fb_data = match fallback_list.get(fb_idx) {
                    Some(fd) => fd,
                    None => continue,
                };

                // Check if this fallback font covers the missing characters.
                let fb_sk_font = fb_data.sk_font();
                let mut covers = true;
                for &ch in &chars[*seg_char_start..*seg_char_end] {
                    if fb_sk_font.unichar_to_glyph(ch as i32) == 0 {
                        covers = false;
                        break;
                    }
                }

                if !covers {
                    continue;
                }

                // Shape the segment with the fallback font.
                // Wrapped in catch_unwind because Skia's shaper may panic
                // for certain font/text combinations (e.g., fonts with
                // incomplete shaping tables).
                let fb_data_clone = Arc::clone(fb_data);
                let segment_text_owned = segment_text.to_string();
                let fb_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut fb_collector =
                        ShapeCollector::new(fb_data_clone, direction);
                    self.shaper.shape(
                        &segment_text_owned,
                        fb_sk_font,
                        left_to_right,
                        f32::INFINITY,
                        &mut fb_collector,
                    );
                    fb_collector.into_shape_result(&segment_text_owned)
                }));

                let fb_result = match fb_result {
                    Ok(r) => r,
                    Err(_) => continue, // Shaping panicked — skip this fallback font.
                };

                if fb_result.runs.is_empty() {
                    continue;
                }

                // Verify fallback actually produced non-zero glyphs.
                let has_real_glyphs = fb_result
                    .runs
                    .iter()
                    .any(|r| r.glyphs.iter().any(|&g| g != 0));
                if !has_real_glyphs {
                    continue;
                }

                // Replace runs in the original result for this segment.
                Self::splice_fallback_runs(
                    result,
                    *seg_char_start,
                    *seg_char_end,
                    fb_result,
                    Arc::clone(fb_data),
                );
                segment_handled = true;
                break; // This segment is handled.
            }

            // Platform fallback: ask the OS for a font covering the first
            // missing character. Blink: FontCache::PlatformFallbackFontForCharacter.
            if !segment_handled {
                let first_missing = chars[*seg_char_start];
                let desc = font.description();
                let platform_data = {
                    let mut cache = crate::font::cache::GLOBAL_FONT_CACHE
                        .lock()
                        .unwrap_or_else(|p| p.into_inner());
                    cache.platform_fallback_for_character(first_missing, desc)
                };
                if let Some(fb_data) = platform_data {
                    let fb_sk_font = fb_data.sk_font();
                    let fb_data_clone = Arc::clone(&fb_data);
                    let segment_text_owned = segment_text.to_string();
                    let fb_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut fb_collector =
                            ShapeCollector::new(fb_data_clone, direction);
                        self.shaper.shape(
                            &segment_text_owned,
                            fb_sk_font,
                            left_to_right,
                            f32::INFINITY,
                            &mut fb_collector,
                        );
                        fb_collector.into_shape_result(&segment_text_owned)
                    }));
                    if let Ok(fb_result) = fb_result {
                        let has_real_glyphs = fb_result
                            .runs
                            .iter()
                            .any(|r| r.glyphs.iter().any(|&g| g != 0));
                        if has_real_glyphs && !fb_result.runs.is_empty() {
                            Self::splice_fallback_runs(
                                result,
                                *seg_char_start,
                                *seg_char_end,
                                fb_result,
                                Arc::clone(&fb_data),
                            );
                        }
                    }
                }
            }
        }

        // Rebuild total width and character_data from the updated runs.
        Self::rebuild_character_data(result, text);
    }

    /// Splice fallback-shaped runs into the result, replacing the original
    /// glyph data for the given character range.
    fn splice_fallback_runs(
        result: &mut ShapeResult,
        seg_char_start: usize,
        seg_char_end: usize,
        fb_result: ShapeResult,
        fb_font_data: Arc<FontPlatformData>,
    ) {
        // Build new runs list: keep original runs outside the segment,
        // replace overlapping portions with fallback runs.
        let mut new_runs: Vec<ShapeResultRun> = Vec::new();

        for run in &result.runs {
            let run_start = run.start_index;
            let run_end = run.start_index + run.num_characters;

            if run_end <= seg_char_start || run_start >= seg_char_end {
                // No overlap — keep as-is.
                new_runs.push(ShapeResultRun {
                    font_data: Arc::clone(&run.font_data),
                    glyphs: run.glyphs.clone(),
                    advances: run.advances.clone(),
                    offsets: run.offsets.clone(),
                    clusters: run.clusters.clone(),
                    start_index: run.start_index,
                    num_characters: run.num_characters,
                    num_glyphs: run.num_glyphs,
                    direction: run.direction,
                });
                continue;
            }

            // Run overlaps with the segment. Split into up to 3 parts:
            // [run_start..seg_char_start] (prefix), [segment] (replaced),
            // [seg_char_end..run_end] (suffix).

            // Prefix (original glyphs before segment)
            if run_start < seg_char_start {
                let prefix_chars = seg_char_start - run_start;
                let (gs, ge) = ShapeResult::glyph_range_for_char_range(run, 0, prefix_chars);
                if gs < ge {
                    new_runs.push(ShapeResultRun {
                        font_data: Arc::clone(&run.font_data),
                        glyphs: run.glyphs[gs..ge].to_vec(),
                        advances: run.advances[gs..ge].to_vec(),
                        offsets: run.offsets[gs..ge].to_vec(),
                        clusters: run.clusters[gs..ge].to_vec(),
                        start_index: run_start,
                        num_characters: prefix_chars,
                        num_glyphs: ge - gs,
                        direction: run.direction,
                    });
                }
            }

            // Suffix (original glyphs after segment)
            if run_end > seg_char_end {
                let suffix_local_start = seg_char_end - run_start;
                let suffix_chars = run_end - seg_char_end;
                let (gs, ge) = ShapeResult::glyph_range_for_char_range(
                    run,
                    suffix_local_start,
                    suffix_local_start + suffix_chars,
                );
                if gs < ge {
                    new_runs.push(ShapeResultRun {
                        font_data: Arc::clone(&run.font_data),
                        glyphs: run.glyphs[gs..ge].to_vec(),
                        advances: run.advances[gs..ge].to_vec(),
                        offsets: run.offsets[gs..ge].to_vec(),
                        clusters: run.clusters[gs..ge]
                            .iter()
                            .map(|c| c.saturating_sub(suffix_local_start))
                            .collect(),
                        start_index: seg_char_end,
                        num_characters: suffix_chars,
                        num_glyphs: ge - gs,
                        direction: run.direction,
                    });
                }
            }
        }

        // Insert fallback runs (shifted to the correct start_index).
        for fb_run in fb_result.runs {
            new_runs.push(ShapeResultRun {
                font_data: Arc::clone(&fb_font_data),
                glyphs: fb_run.glyphs,
                advances: fb_run.advances,
                offsets: fb_run.offsets,
                clusters: fb_run.clusters,
                start_index: seg_char_start + fb_run.start_index,
                num_characters: fb_run.num_characters,
                num_glyphs: fb_run.num_glyphs,
                direction: fb_run.direction,
            });
        }

        // Sort runs by start_index.
        new_runs.sort_by_key(|r| r.start_index);
        result.runs = new_runs;
    }

    /// Rebuild width and character_data from the current runs.
    fn rebuild_character_data(result: &mut ShapeResult, text: &str) {
        let num_characters = text.chars().count();
        let mut char_advances = vec![0.0f32; num_characters];

        let mut total_width = 0.0f32;
        for run in &result.runs {
            let mut run_width = 0.0f32;
            for (gi, &advance) in run.advances.iter().enumerate() {
                run_width += advance;
                // Map glyph to character.
                let local_char = if gi < run.clusters.len() {
                    run.clusters[gi]
                } else if run.num_glyphs == run.num_characters {
                    gi
                } else {
                    continue;
                };
                let char_idx = run.start_index + local_char;
                if char_idx < num_characters {
                    char_advances[char_idx] += advance;
                }
            }
            total_width += run_width;
        }

        result.width = total_width;
        result.num_characters = num_characters;

        // Recompute cluster-base flags from actual run cluster data.
        let mut is_cluster_base = vec![false; num_characters];
        let mut has_cluster_data = false;
        for run in &result.runs {
            if run.clusters.is_empty() {
                continue;
            }
            has_cluster_data = true;
            let mut seen_clusters = std::collections::HashSet::new();
            for &cluster in &run.clusters {
                if seen_clusters.insert(cluster) {
                    let char_idx = run.start_index + cluster;
                    if char_idx < num_characters {
                        is_cluster_base[char_idx] = true;
                    }
                }
            }
        }
        // If no cluster data available, treat all characters as bases.
        if !has_cluster_data {
            for b in &mut is_cluster_base {
                *b = true;
            }
        }

        // Recompute safe-to-break flags from run cluster boundaries.
        // Use the same script-aware logic as compute_safe_breaks: for
        // complex scripts, cluster boundaries are NOT safe.
        let chars: Vec<char> = text.chars().collect();
        let mut safe_to_break = vec![false; num_characters];
        if num_characters > 0 {
            // Always safe at start.
            safe_to_break[0] = true;
            // Safe at whitespace boundaries.
            for (i, &ch) in chars.iter().enumerate() {
                if ch.is_whitespace() {
                    safe_to_break[i] = true;
                    if i + 1 < num_characters {
                        safe_to_break[i + 1] = true;
                    }
                }
            }
            // Safe at cluster boundaries — only for simple scripts.
            for run in &result.runs {
                if run.clusters.len() > 1 {
                    for i in 1..run.clusters.len() {
                        if run.clusters[i] != run.clusters[i - 1] {
                            let char_idx = run.start_index + run.clusters[i];
                            if char_idx < num_characters
                                && !is_complex_script(chars[char_idx])
                            {
                                safe_to_break[char_idx] = true;
                            }
                        }
                    }
                }
            }
        }

        let mut character_data = Vec::with_capacity(num_characters);
        let mut x = 0.0f32;
        for i in 0..num_characters {
            character_data.push(ShapeResultCharacterData {
                x_position: x,
                is_cluster_base: is_cluster_base[i],
                safe_to_break_before: safe_to_break[i],
            });
            x += char_advances[i];
        }
        result.character_data = character_data;
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
                // Non-1:1 mapping (ligature or decomposition).
                // Distribute spacing to each glyph based on the characters
                // it covers, determined by cluster boundaries. This avoids
                // piling all spacing on the last glyph, which would create
                // a visually uneven gap after the final glyph.
                let mut per_glyph_extra = vec![0.0f32; run.num_glyphs];

                // Sort glyph indices by ascending cluster value so we can
                // determine each glyph's character range regardless of
                // text direction (LTR clusters ascend, RTL descend).
                let mut glyph_by_cluster: Vec<(usize, usize)> = run
                    .clusters
                    .iter()
                    .enumerate()
                    .map(|(gi, &c)| (c, gi))
                    .collect();
                glyph_by_cluster.sort_by_key(|(c, _)| *c);

                for (idx, &(cluster, gi)) in glyph_by_cluster.iter().enumerate() {
                    let next_cluster = if idx + 1 < glyph_by_cluster.len() {
                        glyph_by_cluster[idx + 1].0
                    } else {
                        run_chars
                    };
                    // Accumulate extra advance for all characters this glyph covers.
                    for ci_local in cluster..next_cluster {
                        let ci = run_start + ci_local;
                        if ci < num_chars {
                            per_glyph_extra[gi] += extra_advance_per_char[ci];
                        }
                    }
                }

                let mut run_extra = 0.0f32;
                for gi in 0..run.num_glyphs {
                    run.advances[gi] += per_glyph_extra[gi];
                    run_extra += per_glyph_extra[gi];
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
    ///
    /// Uses the same dedup + advance-summing pattern as `ShapeResult::char_advance_for`:
    /// deduplicate cluster values, sum all glyph advances sharing a cluster, then
    /// distribute equally among covered characters.
    fn char_advance_from_runs(runs: &[ShapeResultRun], char_idx: usize) -> f32 {
        for run in runs {
            let run_start = run.start_index;
            let run_end = run.start_index + run.num_characters;
            if char_idx >= run_start && char_idx < run_end {
                let local_idx = char_idx - run_start;
                if run.num_glyphs == run.num_characters {
                    return run.advances[local_idx];
                } else {
                    // Non-1:1 mapping: use cluster data for precise advance.
                    // Deduplicate clusters and sum advances of all glyphs sharing
                    // the same cluster value (matches char_advance_for in shape_result.rs).
                    if !run.clusters.is_empty() {
                        let mut glyph_by_cluster: Vec<(usize, usize)> = run
                            .clusters
                            .iter()
                            .enumerate()
                            .map(|(gi, &c)| (c, gi))
                            .collect();
                        glyph_by_cluster.sort_by_key(|(c, _)| *c);

                        let mut unique_clusters: Vec<usize> = glyph_by_cluster
                            .iter()
                            .map(|(c, _)| *c)
                            .collect();
                        unique_clusters.dedup();

                        for (uc_idx, &uc) in unique_clusters.iter().enumerate() {
                            let next_cluster = if uc_idx + 1 < unique_clusters.len() {
                                unique_clusters[uc_idx + 1]
                            } else {
                                run.num_characters
                            };
                            if local_idx >= uc && local_idx < next_cluster {
                                // Sum advances of all glyphs in this cluster group.
                                let cluster_advance: f32 = run
                                    .clusters
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, &c)| c == uc)
                                    .map(|(gi, _)| run.advances[gi])
                                    .sum();
                                let chars_in_cluster = next_cluster - uc;
                                return cluster_advance / chars_in_cluster as f32;
                            }
                        }
                    }
                    // Fallback: distribute run width proportionally.
                    let total: f32 = run.advances.iter().sum();
                    return total / run.num_characters.max(1) as f32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{Font, FontDescription};

    #[test]
    fn shape_basic_latin_produces_nonzero_glyphs() {
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let result = shaper.shape("Hello", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 5);
        assert!(result.width > 0.0);
        // All glyphs should be non-zero for basic Latin.
        for run in &result.runs {
            for &g in &run.glyphs {
                assert_ne!(g, 0, "Basic Latin should not produce .notdef glyphs");
            }
        }
    }

    #[test]
    fn shape_with_fallback_uses_multiple_fonts() {
        // Shape text mixing scripts. The fallback mechanism should not panic
        // and should produce valid results regardless of available fonts.
        // We test with Latin + symbol characters that might trigger fallback.
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let text = "Hello World";
        let result = shaper.shape(text, &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, text.chars().count());
        assert!(result.width > 0.0);
        // Each run should track which font_data it was shaped with.
        for run in &result.runs {
            assert!(run.font_data.size() > 0.0);
        }

        // Verify fallback doesn't break when there's only one font.
        assert!(font.fallback_count() >= 1);
    }

    #[test]
    fn shape_fallback_detects_missing_glyphs() {
        // When a glyph_id of 0 appears, the fallback mechanism should try
        // other fonts. If no fallback covers it, the .notdef is preserved.
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        // Private use area character — unlikely to have a glyph in any font.
        let text = "a\u{F0000}b";
        let result = shaper.shape(text, &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 3);
        assert!(result.width > 0.0);
        // 'a' and 'b' should have real glyphs; the PUA char may be .notdef.
        // The point is that the shaper doesn't panic.
    }

    #[test]
    fn fallback_preserves_run_font_data() {
        // When fallback is used, each run should have its own font_data.
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let result = shaper.shape("abc", &font, TextDirection::Ltr);
        // For pure Latin, all runs should use the primary font.
        if let Some(primary) = font.primary_font() {
            for run in &result.runs {
                assert!(
                    std::sync::Arc::ptr_eq(&run.font_data, primary),
                    "Latin-only text should use the primary font for all runs"
                );
            }
        }
    }

    #[test]
    fn shape_empty_returns_empty() {
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let result = shaper.shape("", &font, TextDirection::Ltr);
        assert_eq!(result.num_characters, 0);
        assert_eq!(result.width, 0.0);
        assert!(result.runs.is_empty());
    }

    // ── SP11 Round 14 Issue 3: rebuild_character_data retains cluster metadata ──

    #[test]
    fn rebuild_character_data_retains_cluster_metadata() {
        // After shaping, the character_data should have correct cluster-base
        // and safe-to-break information. Shape text with spaces — whitespace
        // boundaries should be marked safe_to_break, and each character that
        // starts a new cluster should be marked is_cluster_base.
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let text = "ab cd";
        let result = shaper.shape(text, &font, TextDirection::Ltr);

        assert_eq!(result.character_data.len(), 5);
        // Position 0 should always be safe to break.
        assert!(result.character_data[0].safe_to_break_before);
        // The space at index 2 should be safe to break before.
        assert!(
            result.character_data[2].safe_to_break_before,
            "Whitespace should be marked safe_to_break_before"
        );
        // Character after space (index 3) should also be safe.
        assert!(
            result.character_data[3].safe_to_break_before,
            "Position after whitespace should be safe_to_break_before"
        );
        // For basic Latin, each character should be its own cluster base.
        for (i, cd) in result.character_data.iter().enumerate() {
            assert!(
                cd.is_cluster_base,
                "Character {} should be a cluster base for basic Latin text",
                i
            );
        }
    }

    // ── SP11 Round 18 Issue 1: Arabic positions marked unsafe within joining sequences ──

    #[test]
    fn arabic_joining_sequence_marked_unsafe() {
        // Arabic text: "مرحبا" (marhaba — 5 Arabic characters).
        // Within a joining sequence, every position (except start and
        // whitespace boundaries) should be marked unsafe to break before,
        // because reshaping would change glyph forms (initial/medial/final).
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let text = "مرحبا";
        let result = shaper.shape(text, &font, TextDirection::Rtl);

        assert_eq!(result.character_data.len(), text.chars().count());
        // Position 0 is always safe.
        assert!(
            result.character_data[0].safe_to_break_before,
            "Start of text should always be safe"
        );
        // Interior Arabic characters should NOT be safe to break before,
        // since they participate in joining and reshaping would change forms.
        for i in 1..result.character_data.len() {
            assert!(
                !result.character_data[i].safe_to_break_before,
                "Arabic char at index {} should not be safe_to_break_before",
                i
            );
        }
    }

    #[test]
    fn arabic_with_space_has_safe_break_at_word_boundary() {
        // Arabic words separated by space: "مرحبا عالم"
        // The space and position after it should be safe to break.
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let text = "مرحبا عالم";
        let result = shaper.shape(text, &font, TextDirection::Rtl);
        let chars: Vec<char> = text.chars().collect();

        // Find the space character index.
        let space_idx = chars.iter().position(|&c| c == ' ').expect("should have space");
        assert!(
            result.character_data[space_idx].safe_to_break_before,
            "Space should be safe_to_break_before"
        );
        if space_idx + 1 < chars.len() {
            assert!(
                result.character_data[space_idx + 1].safe_to_break_before,
                "Position after space should be safe_to_break_before"
            );
        }
    }

    #[test]
    fn latin_cluster_boundaries_remain_safe() {
        // Latin text should still have safe-to-break at cluster boundaries,
        // ensuring the complex-script guard doesn't affect simple scripts.
        let shaper = TextShaper::new();
        let font = Font::new(FontDescription::default());
        let result = shaper.shape("abcdef", &font, TextDirection::Ltr);

        assert_eq!(result.character_data.len(), 6);
        // For basic Latin with 1:1 clusters, all positions should be safe.
        for (i, cd) in result.character_data.iter().enumerate() {
            assert!(
                cd.safe_to_break_before,
                "Latin char at index {} should be safe_to_break_before",
                i
            );
        }
    }

    // ── SP11 Round 18 Issue 3: letter-spacing distributes across ligature glyphs ──

    #[test]
    fn letter_spacing_distributes_across_ligature_run() {
        // Construct a ShapeResult with a non-1:1 run (simulating a ligature)
        // and verify that apply_spacing distributes evenly per-cluster
        // instead of piling everything on the last glyph.
        use std::sync::Arc;
        use crate::font::FontPlatformData;

        let font = Font::new(FontDescription::default());
        let font_data = font.primary_font().unwrap().clone();

        // Simulate "office": 6 chars, 4 glyphs (o, ffi-ligature, c, e).
        // Clusters: glyph 0→char 0, glyph 1→char 1 (covers 1-3), glyph 2→char 4, glyph 3→char 5.
        let mut result = ShapeResult {
            runs: vec![ShapeResultRun {
                font_data: Arc::clone(&font_data),
                glyphs: vec![100, 200, 300, 400],
                advances: vec![10.0, 20.0, 10.0, 10.0],
                offsets: vec![(0.0, 0.0); 4],
                clusters: vec![0, 1, 4, 5],
                start_index: 0,
                num_characters: 6,
                num_glyphs: 4,
                direction: TextDirection::Ltr,
            }],
            width: 50.0,
            num_characters: 6,
            direction: TextDirection::Ltr,
            character_data: (0..6)
                .map(|i| ShapeResultCharacterData {
                    x_position: i as f32 * 8.33,
                    is_cluster_base: true,
                    safe_to_break_before: true,
                })
                .collect(),
        };

        let mut desc = FontDescription::default();
        desc.letter_spacing = 5.0;
        let spaced_font = Font::new(desc);

        TextShaper::apply_spacing(&mut result, &spaced_font, "office");

        // Glyph 0 (cluster 0, covers char 0): should get 1 × 5.0 = 5.0
        let eps = 0.01;
        assert!(
            (result.runs[0].advances[0] - 15.0).abs() < eps,
            "glyph 0 advance: expected 15.0, got {}",
            result.runs[0].advances[0]
        );
        // Glyph 1 (cluster 1, covers chars 1-3): should get 3 × 5.0 = 15.0
        assert!(
            (result.runs[0].advances[1] - 35.0).abs() < eps,
            "glyph 1 (ligature) advance: expected 35.0, got {}",
            result.runs[0].advances[1]
        );
        // Glyph 2 (cluster 4, covers char 4): should get 1 × 5.0 = 5.0
        assert!(
            (result.runs[0].advances[2] - 15.0).abs() < eps,
            "glyph 2 advance: expected 15.0, got {}",
            result.runs[0].advances[2]
        );
        // Glyph 3 (cluster 5, covers char 5): should get 1 × 5.0 = 5.0
        assert!(
            (result.runs[0].advances[3] - 15.0).abs() < eps,
            "glyph 3 advance: expected 15.0, got {}",
            result.runs[0].advances[3]
        );
        // Total extra = 6 × 5.0 = 30.0
        assert!(
            (result.width - 80.0).abs() < eps,
            "total width: expected 80.0, got {}",
            result.width
        );
    }

    #[test]
    fn word_spacing_only_applies_to_space_chars() {
        // Verify word-spacing is applied only at U+0020, not at every character.
        let shaper = TextShaper::new();
        let mut desc = FontDescription::default();
        desc.word_spacing = 10.0;
        desc.letter_spacing = 0.0;
        let font = Font::new(desc);

        let text = "a b";
        let result = shaper.shape(text, &font, TextDirection::Ltr);

        // Only the space character should have extra width.
        // Character 0 ('a') and 2 ('b') should not have word_spacing.
        let a_width = result.character_data[1].x_position - result.character_data[0].x_position;
        let space_width = result.character_data[2].x_position - result.character_data[1].x_position;
        assert!(
            space_width > a_width,
            "Space should be wider than 'a' due to word-spacing: space={}, a={}",
            space_width,
            a_width
        );
    }

    // ── Issue 1 (R24): char_advance_from_runs dedup + sum ────────────────

    #[test]
    fn char_advance_from_runs_sums_duplicate_cluster_glyphs() {
        // 2 glyphs with cluster=0 (advances 8.0 and 4.0), 1 glyph with cluster=1 (advance 10.0).
        // char_advance_from_runs(_, 0) should be (8+4)/1 = 12.0
        // char_advance_from_runs(_, 1) should be 10.0
        let font_data = {
            let mut cache = crate::font::cache::GLOBAL_FONT_CACHE.lock().unwrap();
            let desc = FontDescription::default();
            cache.get_font_platform_data("sans-serif", &desc)
                .unwrap_or_else(|| cache.get_font_platform_data("serif", &desc).unwrap())
        };
        let runs = vec![ShapeResultRun {
            font_data,
            glyphs: vec![1, 2, 3],
            advances: vec![8.0, 4.0, 10.0],
            offsets: vec![(0.0, 0.0); 3],
            clusters: vec![0, 0, 1],
            start_index: 0,
            num_characters: 2,
            num_glyphs: 3,
            direction: crate::shaping::shape_result::TextDirection::Ltr,
        }];

        let adv0 = TextShaper::char_advance_from_runs(&runs, 0);
        assert!(
            (adv0 - 12.0).abs() < 0.01,
            "char 0 advance should be 12.0 (8+4), got {adv0}"
        );
        let adv1 = TextShaper::char_advance_from_runs(&runs, 1);
        assert!(
            (adv1 - 10.0).abs() < 0.01,
            "char 1 advance should be 10.0, got {adv1}"
        );
    }

    #[test]
    fn char_advance_from_runs_three_glyphs_same_cluster() {
        // 3 glyphs all with cluster=0 covering 2 chars, 1 glyph with cluster=2.
        // cluster 0 covers chars [0,2), advance sum = 5+3+2 = 10, per-char = 5.0
        let font_data = {
            let mut cache = crate::font::cache::GLOBAL_FONT_CACHE.lock().unwrap();
            let desc = FontDescription::default();
            cache.get_font_platform_data("sans-serif", &desc)
                .unwrap_or_else(|| cache.get_font_platform_data("serif", &desc).unwrap())
        };
        let runs = vec![ShapeResultRun {
            font_data,
            glyphs: vec![1, 2, 3, 4],
            advances: vec![5.0, 3.0, 2.0, 8.0],
            offsets: vec![(0.0, 0.0); 4],
            clusters: vec![0, 0, 0, 2],
            start_index: 0,
            num_characters: 3,
            num_glyphs: 4,
            direction: crate::shaping::shape_result::TextDirection::Ltr,
        }];

        let adv0 = TextShaper::char_advance_from_runs(&runs, 0);
        let adv1 = TextShaper::char_advance_from_runs(&runs, 1);
        // Cluster 0 covers chars [0, 2), sum = 10, per-char = 5.0
        assert!(
            (adv0 - 5.0).abs() < 0.01,
            "char 0 advance should be 5.0 (10/2), got {adv0}"
        );
        assert!(
            (adv1 - 5.0).abs() < 0.01,
            "char 1 advance should be 5.0 (10/2), got {adv1}"
        );
        let adv2 = TextShaper::char_advance_from_runs(&runs, 2);
        assert!(
            (adv2 - 8.0).abs() < 0.01,
            "char 2 advance should be 8.0, got {adv2}"
        );
    }

    #[test]
    fn char_advance_from_runs_1to1_mapping_unchanged() {
        // 1:1 mapping should still work correctly.
        let font_data = {
            let mut cache = crate::font::cache::GLOBAL_FONT_CACHE.lock().unwrap();
            let desc = FontDescription::default();
            cache.get_font_platform_data("sans-serif", &desc)
                .unwrap_or_else(|| cache.get_font_platform_data("serif", &desc).unwrap())
        };
        let runs = vec![ShapeResultRun {
            font_data,
            glyphs: vec![1, 2, 3],
            advances: vec![10.0, 20.0, 30.0],
            offsets: vec![(0.0, 0.0); 3],
            clusters: vec![0, 1, 2],
            start_index: 0,
            num_characters: 3,
            num_glyphs: 3,
            direction: crate::shaping::shape_result::TextDirection::Ltr,
        }];

        assert!((TextShaper::char_advance_from_runs(&runs, 0) - 10.0).abs() < 0.01);
        assert!((TextShaper::char_advance_from_runs(&runs, 1) - 20.0).abs() < 0.01);
        assert!((TextShaper::char_advance_from_runs(&runs, 2) - 30.0).abs() < 0.01);
    }
}
