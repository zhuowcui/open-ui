//! ShapeResult — output of text shaping.
//!
//! Mirrors Blink's `ShapeResult` (`platform/fonts/shaping/shape_result.h`).
//! Contains glyph runs with IDs, positions, and per-character metadata
//! for cursor placement, hit testing, and line breaking.

use std::sync::Arc;

use skia_safe::{Point, TextBlob, TextBlobBuilder};

use crate::font::FontPlatformData;

/// Direction of text flow within a run or result.
///
/// Blink: `TextDirection` in `platform/text/text_direction.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
}

impl TextDirection {
    /// Whether this direction is left-to-right.
    #[inline]
    pub fn is_ltr(self) -> bool {
        self == TextDirection::Ltr
    }

    /// Whether this direction is right-to-left.
    #[inline]
    pub fn is_rtl(self) -> bool {
        self == TextDirection::Rtl
    }
}

/// Result of shaping a text range. Contains glyph IDs, positions, and metadata.
///
/// Blink: `ShapeResult` in `platform/fonts/shaping/shape_result.h`.
pub struct ShapeResult {
    /// Glyph runs (one per font/direction change).
    pub runs: Vec<ShapeResultRun>,
    /// Total advance width of all runs.
    pub width: f32,
    /// Number of characters in the original text.
    pub num_characters: usize,
    /// Direction of the text.
    pub direction: TextDirection,
    /// Per-character data for cursor positioning and line breaking.
    pub character_data: Vec<ShapeResultCharacterData>,
}

/// A contiguous run of glyphs using the same font.
///
/// Blink: `ShapeResult::RunInfo` in `platform/fonts/shaping/shape_result.h`.
pub struct ShapeResultRun {
    /// Font used for this run.
    pub font_data: Arc<FontPlatformData>,
    /// Glyph IDs from the font.
    pub glyphs: Vec<u16>,
    /// Advance width for each glyph.
    pub advances: Vec<f32>,
    /// X/Y offset for each glyph (for combining marks, kerning adjustments).
    pub offsets: Vec<(f32, f32)>,
    /// Per-glyph cluster mapping: character index (relative to run start) each glyph belongs to.
    pub clusters: Vec<usize>,
    /// Start character index in original text.
    pub start_index: usize,
    /// Number of characters covered by this run.
    pub num_characters: usize,
    /// Number of glyphs (may differ from num_characters due to ligatures/decomposition).
    pub num_glyphs: usize,
    /// Direction of this run.
    pub direction: TextDirection,
}

/// Per-character metadata for cursor positioning and line breaking.
///
/// Blink: character-index data within `ShapeResult` for offset-to-position mapping.
#[derive(Clone, Debug)]
pub struct ShapeResultCharacterData {
    /// Cumulative advance from the start of the ShapeResult to this character.
    pub x_position: f32,
    /// Whether this character starts a new grapheme cluster.
    pub is_cluster_base: bool,
    /// Whether it's safe to break the line before this character.
    pub safe_to_break_before: bool,
}

impl ShapeResult {
    /// Create an empty ShapeResult for zero-length text.
    pub fn empty(direction: TextDirection) -> Self {
        Self {
            runs: Vec::new(),
            width: 0.0,
            num_characters: 0,
            direction,
            character_data: Vec::new(),
        }
    }

    /// Total width of the shaped text.
    #[inline]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Number of glyphs across all runs.
    pub fn num_glyphs(&self) -> usize {
        self.runs.iter().map(|r| r.num_glyphs).sum()
    }

    /// Get the X position for a character offset (for cursor placement).
    ///
    /// Blink: `ShapeResult::XPositionForOffset`.
    pub fn x_position_for_offset(&self, offset: usize) -> f32 {
        if self.character_data.is_empty() {
            return 0.0;
        }
        if offset >= self.num_characters {
            return self.width;
        }
        self.character_data[offset].x_position
    }

    /// Get the character offset for an X position (for hit testing).
    ///
    /// Returns the offset of the character whose center is closest to `x`.
    /// Blink: `ShapeResult::OffsetForPosition`.
    pub fn offset_for_x_position(&self, x: f32) -> usize {
        if self.character_data.is_empty() || x <= 0.0 {
            return 0;
        }
        if x >= self.width {
            return self.num_characters;
        }

        // Binary search for the character whose range contains x.
        // Each character spans from character_data[i].x_position to the next
        // character's x_position (or width for the last character).
        for i in 0..self.num_characters {
            let char_start = self.character_data[i].x_position;
            let char_end = if i + 1 < self.num_characters {
                self.character_data[i + 1].x_position
            } else {
                self.width
            };
            let mid = (char_start + char_end) / 2.0;
            if x < mid {
                return i;
            }
        }
        self.num_characters
    }

    /// Check if it's safe to break before a character offset.
    ///
    /// Blink: `ShapeResult::SafeToBreakBefore`.
    pub fn safe_to_break_before(&self, offset: usize) -> bool {
        if offset == 0 {
            return true;
        }
        if offset >= self.num_characters {
            return true;
        }
        self.character_data[offset].safe_to_break_before
    }

    /// Width of a sub-range of characters.
    ///
    /// Blink: `ShapeResult::Width` with range parameters.
    pub fn width_for_range(&self, start: usize, end: usize) -> f32 {
        if start >= end || self.character_data.is_empty() {
            return 0.0;
        }
        let start = start.min(self.num_characters);
        let end = end.min(self.num_characters);
        if start >= end {
            return 0.0;
        }
        let start_x = if start == 0 {
            0.0
        } else {
            self.character_data[start].x_position
        };
        let end_x = if end >= self.num_characters {
            self.width
        } else {
            self.character_data[end].x_position
        };
        end_x - start_x
    }

    /// Get a sub-range of the shape result (for line breaking).
    ///
    /// Blink: `ShapeResult::SubRange`.
    pub fn sub_range(&self, start: usize, end: usize) -> ShapeResult {
        if start >= end || self.character_data.is_empty() {
            return ShapeResult::empty(self.direction);
        }
        let start = start.min(self.num_characters);
        let end = end.min(self.num_characters);
        if start >= end {
            return ShapeResult::empty(self.direction);
        }
        let sub_width = self.width_for_range(start, end);
        let start_x = if start > 0 {
            self.character_data[start].x_position
        } else {
            0.0
        };

        // Build sub-range character data, shifting x_positions to start at 0.
        let character_data: Vec<ShapeResultCharacterData> = (start..end)
            .map(|i| ShapeResultCharacterData {
                x_position: self.character_data[i].x_position - start_x,
                is_cluster_base: self.character_data[i].is_cluster_base,
                safe_to_break_before: if i == start {
                    true
                } else {
                    self.character_data[i].safe_to_break_before
                },
            })
            .collect();

        // Build sub-range runs by clipping to the [start, end) character range.
        let mut sub_runs = Vec::new();
        for run in &self.runs {
            let run_start = run.start_index;
            let run_end = run.start_index + run.num_characters;

            // Skip runs that don't overlap with [start, end).
            if run_end <= start || run_start >= end {
                continue;
            }

            let clip_start = start.max(run_start);
            let clip_end = end.min(run_end);

            // Find which glyphs correspond to the clipped character range.
            // Use cluster data stored in the run to map characters to glyphs.
            let (glyph_start, glyph_end) =
                Self::glyph_range_for_char_range(run, clip_start - run_start, clip_end - run_start);

            if glyph_start >= glyph_end {
                continue;
            }

            sub_runs.push(ShapeResultRun {
                font_data: Arc::clone(&run.font_data),
                glyphs: run.glyphs[glyph_start..glyph_end].to_vec(),
                advances: run.advances[glyph_start..glyph_end].to_vec(),
                offsets: run.offsets[glyph_start..glyph_end].to_vec(),
                clusters: if !run.clusters.is_empty() {
                    let char_offset = clip_start - run_start;
                    run.clusters[glyph_start..glyph_end]
                        .iter()
                        .map(|c| c.saturating_sub(char_offset))
                        .collect()
                } else {
                    Vec::new()
                },
                start_index: clip_start - start,
                num_characters: clip_end - clip_start,
                num_glyphs: glyph_end - glyph_start,
                direction: run.direction,
            });
        }

        ShapeResult {
            runs: sub_runs,
            width: sub_width,
            num_characters: end - start,
            direction: self.direction,
            character_data,
        }
    }

    /// Build a Skia TextBlob from this shape result for rendering.
    ///
    /// Returns `None` if the result has no glyphs.
    pub fn to_text_blob(&self) -> Option<TextBlob> {
        if self.runs.is_empty() || self.num_glyphs() == 0 {
            return None;
        }

        let mut builder = TextBlobBuilder::new();
        let mut run_x = 0.0f32;

        for run in &self.runs {
            if run.num_glyphs == 0 {
                continue;
            }
            let sk_font = run.font_data.sk_font();
            let (glyphs_out, positions_out) =
                builder.alloc_run_pos(sk_font, run.num_glyphs, None);
            glyphs_out.copy_from_slice(&run.glyphs);

            let mut x = run_x;
            for i in 0..run.num_glyphs {
                positions_out[i] = Point::new(x + run.offsets[i].0, run.offsets[i].1);
                x += run.advances[i];
            }
            run_x = x;
        }

        builder.make()
    }

    /// Find the glyph range within a run that covers a given character range.
    ///
    /// Uses the run's cluster data for precise mapping. Falls back to
    /// proportional mapping when cluster data is unavailable.
    pub fn glyph_range_for_char_range(
        run: &ShapeResultRun,
        char_start: usize,
        char_end: usize,
    ) -> (usize, usize) {
        if run.num_characters == 0 {
            return (0, 0);
        }

        // Use cluster data for precise glyph-to-character mapping.
        // Derive each glyph's covered character interval from successive
        // cluster boundaries. Include the glyph if its covered interval
        // overlaps with [char_start, char_end). This correctly handles
        // ligature glyphs whose cluster base is before char_start but
        // that cover characters within the range (Issue 5 fix).
        if !run.clusters.is_empty() {
            // Sort glyphs by cluster to determine each glyph's character coverage.
            let mut glyph_by_cluster: Vec<(usize, usize)> = run
                .clusters
                .iter()
                .enumerate()
                .map(|(gi, &c)| (c, gi))
                .collect();
            glyph_by_cluster.sort_by_key(|(c, _)| *c);

            let mut glyph_start = run.num_glyphs;
            let mut glyph_end = 0;
            for (idx, &(cluster, gi)) in glyph_by_cluster.iter().enumerate() {
                let next_cluster = if idx + 1 < glyph_by_cluster.len() {
                    glyph_by_cluster[idx + 1].0
                } else {
                    run.num_characters
                };
                // Glyph covers characters [cluster, next_cluster).
                // Include if it overlaps with [char_start, char_end).
                if cluster < char_end && next_cluster > char_start {
                    glyph_start = glyph_start.min(gi);
                    glyph_end = glyph_end.max(gi + 1);
                }
            }
            if glyph_start >= glyph_end {
                return (0, 0);
            }
            return (glyph_start, glyph_end);
        }

        // For 1:1 mapping (common in Latin text):
        if run.num_glyphs == run.num_characters {
            let gs = char_start.min(run.num_glyphs);
            let ge = char_end.min(run.num_glyphs);
            return (gs, ge);
        }

        // For non-1:1 mapping without cluster data, use proportional mapping.
        let ratio = run.num_glyphs as f32 / run.num_characters as f32;
        let gs = (char_start as f32 * ratio).round() as usize;
        let ge = (char_end as f32 * ratio).round() as usize;
        (gs.min(run.num_glyphs), ge.min(run.num_glyphs))
    }

    /// Apply justification by distributing extra width to space glyphs.
    ///
    /// Finds all space characters (U+0020) in the result and adds
    /// `extra_per_space` to their corresponding glyph advances, then
    /// shifts subsequent glyph positions so the total width is correct.
    /// The last `exclude_trailing` space characters are skipped so that
    /// trailing spaces (which hang in pre-wrap) are not expanded.
    ///
    /// Blink: `ShapeResult::ApplyExpansion`.
    pub fn apply_justification(&mut self, extra_per_space: f32, text: &str, exclude_trailing: usize) {
        if extra_per_space <= 0.0 || self.runs.is_empty() {
            return;
        }

        let chars: Vec<char> = text.chars().collect();

        // Pre-compute the set of character indices that are trailing spaces
        // (the last N spaces in LOGICAL order). This avoids depending on
        // glyph iteration order, which differs between LTR and RTL runs.
        let mut trailing_space_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        if exclude_trailing > 0 {
            let mut remaining = exclude_trailing;
            for (i, &ch) in chars.iter().enumerate().rev() {
                if remaining == 0 {
                    break;
                }
                if ch == ' ' {
                    trailing_space_indices.insert(i);
                    remaining -= 1;
                }
            }
        }

        let mut total_extra = 0.0f32;

        for run in &mut self.runs {
            let run_start = run.start_index;
            for gi in 0..run.num_glyphs {
                // Map glyph to character index using cluster data.
                let char_idx = if !run.clusters.is_empty() {
                    run_start + run.clusters[gi]
                } else if run.num_glyphs == run.num_characters {
                    run_start + gi
                } else {
                    continue;
                };

                if char_idx < chars.len() && chars[char_idx] == ' ' {
                    if !trailing_space_indices.contains(&char_idx) {
                        run.advances[gi] += extra_per_space;
                        total_extra += extra_per_space;
                    }
                }
            }
        }

        self.width += total_extra;

        // Rebuild character_data x_positions to reflect adjusted advances.
        if !self.character_data.is_empty() && !chars.is_empty() {
            let mut x = 0.0f32;
            for i in 0..self.num_characters.min(self.character_data.len()) {
                self.character_data[i].x_position = x;
                x += self.char_advance_for(i);
            }
        }
    }

    /// Apply inter-character justification by distributing extra space
    /// between all character boundaries (not just spaces).
    ///
    /// `extra_per_gap` is added to each glyph's advance. For a run with
    /// N characters, there are N-1 internal gaps. Each glyph that starts
    /// a character gets `extra_per_gap` added except the last character
    /// in the entire result.
    ///
    /// Blink: `ShapeResult::ApplyExpansion` with inter-character mode.
    pub fn apply_inter_character_justification(&mut self, extra_per_gap: f32) {
        if extra_per_gap <= 0.0 || self.runs.is_empty() || self.num_characters <= 1 {
            return;
        }

        let mut total_extra = 0.0f32;
        let total_chars = self.num_characters;
        // We need to expand gaps between every adjacent pair of characters
        // across the entire result. That's (total_chars - 1) gaps total.
        // Each glyph gets extra_per_gap added to its advance for every
        // character it covers, except the very last character overall.
        for run in &mut self.runs {
            let run_start = run.start_index;
            if !run.clusters.is_empty() {
                // With cluster data: add extra_per_gap to each glyph,
                // except for the glyph that covers the last character.
                for gi in 0..run.num_glyphs {
                    let char_idx = run_start + run.clusters[gi];
                    if char_idx < total_chars - 1 {
                        run.advances[gi] += extra_per_gap;
                        total_extra += extra_per_gap;
                    }
                }
            } else if run.num_glyphs == run.num_characters {
                // 1:1 mapping
                for gi in 0..run.num_glyphs {
                    let char_idx = run_start + gi;
                    if char_idx < total_chars - 1 {
                        run.advances[gi] += extra_per_gap;
                        total_extra += extra_per_gap;
                    }
                }
            } else {
                // Non-1:1 without clusters: distribute proportionally.
                // Add extra to all glyphs except last.
                let gaps_in_run = if run_start + run.num_characters >= total_chars {
                    run.num_characters.saturating_sub(1)
                } else {
                    run.num_characters
                };
                if gaps_in_run > 0 && run.num_glyphs > 0 {
                    let per_glyph = (extra_per_gap * gaps_in_run as f32) / run.num_glyphs as f32;
                    for gi in 0..run.num_glyphs {
                        run.advances[gi] += per_glyph;
                        total_extra += per_glyph;
                    }
                }
            }
        }

        self.width += total_extra;

        // Rebuild character_data x_positions.
        if !self.character_data.is_empty() {
            let mut x = 0.0f32;
            for i in 0..self.num_characters.min(self.character_data.len()) {
                self.character_data[i].x_position = x;
                x += self.char_advance_for(i);
            }
        }
    }

    /// Compute the advance width for a specific character from the glyph runs.
    fn char_advance_for(&self, char_idx: usize) -> f32 {
        for run in &self.runs {
            let run_start = run.start_index;
            let run_end = run.start_index + run.num_characters;
            if char_idx >= run_start && char_idx < run_end {
                let local_idx = char_idx - run_start;
                if run.num_glyphs == run.num_characters {
                    return run.advances[local_idx];
                } else {
                    // Non-1:1 mapping: use cluster data for precise advance.
                    // Issue 6 fix: distribute based on cluster boundaries
                    // rather than averaging uniformly.
                    if !run.clusters.is_empty() {
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
                                run.num_characters
                            };
                            if local_idx >= cluster && local_idx < next_cluster {
                                let chars_in_cluster = next_cluster - cluster;
                                return run.advances[gi] / chars_in_cluster as f32;
                            }
                        }
                    }
                    let total: f32 = run.advances.iter().sum();
                    return total / run.num_characters.max(1) as f32;
                }
            }
        }
        0.0
    }
}
impl std::fmt::Debug for ShapeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShapeResult")
            .field("width", &self.width)
            .field("num_characters", &self.num_characters)
            .field("num_glyphs", &self.num_glyphs())
            .field("runs", &self.runs.len())
            .field("direction", &self.direction)
            .finish()
    }
}

impl std::fmt::Debug for ShapeResultRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShapeResultRun")
            .field("num_glyphs", &self.num_glyphs)
            .field("num_characters", &self.num_characters)
            .field("start_index", &self.start_index)
            .field("direction", &self.direction)
            .finish()
    }
}
