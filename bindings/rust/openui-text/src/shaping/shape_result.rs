//! ShapeResult — output of text shaping.
//!
//! Mirrors Blink's `ShapeResult` (`platform/fonts/shaping/shape_result.h`).
//! Contains glyph runs with IDs, positions, and per-character metadata
//! for cursor placement, hit testing, and line breaking.

use std::sync::Arc;

use openui_geometry::RasterPixelGeometry;
use skia_safe::{
    font::Edging, utils::CustomTypefaceBuilder, FontHinting, Matrix, PathBuilder, Point, TextBlob,
    TextBlobBuilder,
};
use skrifa::{
    instance::{LocationRef, Size},
    outline::{
        DrawSettings, Engine, GlyphStyles, HintingInstance, HintingOptions, OutlineGlyphFormat,
        OutlinePen, SmoothMode, Target,
    },
    FontRef, GlyphId as SkrifaGlyphId, MetadataProvider,
};

use crate::font::FontPlatformData;

/// Selects the outline rasterizer used when a shaped run becomes a Skia
/// text blob. Chromium's Linux native controls use Fontations-hinted paths,
/// while authored text continues through Skia's ordinary FreeType backend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextRasterPolicy {
    #[default]
    Skia,
    ChromiumNativeControl,
    ChromiumEmbeddedDocument,
    ChromiumAuthorLcd,
    ChromiumAliased,
}

struct SkiaPathPen(PathBuilder);

impl Default for SkiaPathPen {
    fn default() -> Self {
        Self(PathBuilder::new())
    }
}

impl OutlinePen for SkiaPathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x, -y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x, -y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to((cx0, -cy0), (x, -y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to((cx0, -cy0), (cx1, -cy1), (x, -y));
    }

    fn close(&mut self) {
        self.0.close();
    }
}

#[cfg(test)]
fn fontations_compatible_font(
    source_font: &skia_safe::Font,
    glyphs: &[u16],
    raster_policy: TextRasterPolicy,
) -> Option<skia_safe::Font> {
    fontations_compatible_font_with_geometry(
        source_font,
        glyphs,
        raster_policy,
        RasterPixelGeometry::RgbHorizontal,
    )
}

fn fontations_compatible_font_with_geometry(
    source_font: &skia_safe::Font,
    glyphs: &[u16],
    raster_policy: TextRasterPolicy,
    pixel_geometry: RasterPixelGeometry,
) -> Option<skia_safe::Font> {
    if raster_policy == TextRasterPolicy::Skia {
        return None;
    }
    let (font_data, ttc_index) = source_font.typeface().to_font_data()?;
    let font_ref = FontRef::from_index(&font_data, ttc_index as u32).ok()?;
    let outlines = font_ref.outline_glyphs();
    let size = source_font.size();
    if !size.is_finite() || size <= 0.0 {
        return None;
    }
    let inverse_size = size.recip();
    let location = LocationRef::default();
    // Match SkTypeface_fontations.cpp's scaler settings. Text provenance
    // selects this outline engine; the configured font selects its hinting
    // and mask. A pre-hinted path must not silently force LCD or autohinting.
    let hinting_options = if outlines.require_interpreter() {
        Some(skrifa::outline::HintingMode::Strong.into())
    } else if source_font.hinting() == FontHinting::None {
        None
    } else if source_font.edging() == Edging::Alias {
        Some(skrifa::outline::HintingMode::Strong.into())
    } else {
        let slight = source_font.hinting() == FontHinting::Slight;
        let lcd = source_font.hinting() == FontHinting::Full
            && source_font.edging() == Edging::SubpixelAntiAlias;
        let mode = if slight {
            SmoothMode::Light
        } else if lcd {
            match pixel_geometry {
                RasterPixelGeometry::RgbVertical | RasterPixelGeometry::BgrVertical => {
                    SmoothMode::VerticalLcd
                }
                _ => SmoothMode::Lcd,
            }
        } else {
            SmoothMode::Normal
        };
        // Light fitting forces autohinting for TrueType. CFF keeps its
        // interpreter unless the caller explicitly requests autohinting.
        let engine = if source_font.is_force_auto_hinting()
            || (slight && outlines.format() == Some(OutlineGlyphFormat::Glyf))
        {
            Engine::Auto(Some(GlyphStyles::new(&outlines)))
        } else {
            Engine::AutoFallback
        };
        Some(HintingOptions {
            engine,
            target: Target::Smooth {
                mode,
                symmetric_rendering: true,
                preserve_linear_metrics: false,
            },
        })
    };
    let hinting = hinting_options
        .map(|options| HintingInstance::new(&outlines, Size::new(size), location, options))
        .transpose()
        .ok()?;

    let mut builder = CustomTypefaceBuilder::new();
    let (_, metrics) = source_font.metrics();
    builder.set_metrics(&metrics, inverse_size);
    for &glyph_id in glyphs {
        let glyph = outlines.get(SkrifaGlyphId::new(glyph_id as u32))?;
        let mut pen = SkiaPathPen::default();
        let settings = hinting.as_ref().map_or_else(
            || DrawSettings::unhinted(Size::new(size), location),
            |instance| DrawSettings::hinted(instance, false),
        );
        glyph.draw(settings, &mut pen).ok()?;
        // Retain the actual physical font size in Skia's strike descriptor.
        // The outline has already been fitted at that size; express it in
        // normalized font coordinates before Skia scales it once for replay.
        // A unit-size descriptor hides the effective size and transform from
        // Skia's ordinary mask selection, including its LCD coverage limit.
        let path = pen
            .0
            .detach()
            .make_transform(&Matrix::scale((inverse_size, inverse_size)));
        builder.set_glyph(glyph_id, 0.0, &path);
    }
    let typeface = builder.detach()?;
    let mut font = source_font.clone();
    font.set_typeface(typeface);
    // Outlines are already fitted at the physical strike size. Preserve
    // edging, subpixel, linear-metric and force-autohint flags from the
    // configured source font; only avoid applying hinting a second time.
    font.set_hinting(FontHinting::None);
    Some(font)
}

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
        self.to_text_blob_with_raster_policy(None, TextRasterPolicy::Skia)
    }

    /// Build a text blob retaining an absolute LCD origin for glyph placement.
    /// Logical positions and advances remain independent of raster settings.
    pub fn to_text_blob_with_lcd_origin(&self, device_origin_x: Option<f32>) -> Option<TextBlob> {
        self.to_text_blob_with_raster_policy(device_origin_x, TextRasterPolicy::Skia)
    }

    /// Build a text blob with an explicit platform raster policy.
    pub fn to_text_blob_with_raster_policy(
        &self,
        device_origin_x: Option<f32>,
        raster_policy: TextRasterPolicy,
    ) -> Option<TextBlob> {
        self.to_text_blob_with_raster_policy_at_scale(device_origin_x, raster_policy, 1.0)
    }

    /// Build a blob whose glyph strike and positions are expressed in
    /// physical pixels. Callers apply the inverse local scale while recording
    /// so layout origins and advances remain logical.
    pub fn to_text_blob_with_raster_policy_at_scale(
        &self,
        device_origin_x: Option<f32>,
        raster_policy: TextRasterPolicy,
        device_scale: f32,
    ) -> Option<TextBlob> {
        self.to_text_blob_with_raster_policy_and_geometry_at_scale(
            device_origin_x,
            raster_policy,
            device_scale,
            RasterPixelGeometry::RgbHorizontal,
        )
    }

    /// Build a physical strike using the caller's immutable LCD orientation.
    /// Existing blob builders retain their horizontal RGB default.
    pub fn to_text_blob_with_raster_policy_and_geometry_at_scale(
        &self,
        device_origin_x: Option<f32>,
        raster_policy: TextRasterPolicy,
        device_scale: f32,
        pixel_geometry: RasterPixelGeometry,
    ) -> Option<TextBlob> {
        if self.runs.is_empty() || self.num_glyphs() == 0 {
            return None;
        }

        let device_scale = if device_scale.is_finite() && device_scale > 0.0 {
            device_scale
        } else {
            1.0
        };

        let mut builder = TextBlobBuilder::new();
        let mut run_x = 0.0f32;

        for run in &self.runs {
            if run.num_glyphs == 0 {
                continue;
            }
            let source_font = run.font_data.sk_font();
            // Chromium builds hinted outlines at the physical strike size.
            // Scaling an outline already fitted at CSS size changes its ink
            // bounds. This applies to aliased and LCD outlines alike. Keep
            // the custom outline in normalized font units after fitting once; glyph
            // positions are converted to physical coordinates below.
            let mut physical_source_font;
            let compatible_source_font = if raster_policy != TextRasterPolicy::Skia
                && (device_scale - 1.0).abs() > f32::EPSILON
            {
                physical_source_font = source_font.clone();
                physical_source_font.set_size(source_font.size() * device_scale);
                &physical_source_font
            } else {
                source_font
            };
            let compatible_font = (raster_policy != TextRasterPolicy::Skia)
                .then(|| {
                    fontations_compatible_font_with_geometry(
                        compatible_source_font,
                        &run.glyphs,
                        raster_policy,
                        pixel_geometry,
                    )
                })
                .flatten();
            let sk_font = compatible_font.as_ref().unwrap_or(source_font);
            let mut physical_font;
            let compatible_outline_is_physical =
                compatible_font.is_some() && (device_scale - 1.0).abs() > f32::EPSILON;
            let raster_font =
                if (device_scale - 1.0).abs() > f32::EPSILON && !compatible_outline_is_physical {
                    physical_font = sk_font.clone();
                    physical_font.set_size(sk_font.size() * device_scale);
                    &physical_font
                } else {
                    // Keep the legacy unit-scale font object byte-for-byte
                    // unchanged. Calling set_size with its existing value still
                    // rebuilds Skia's strike descriptor and shifts aliased Ahem
                    // masks by one row in fragmented paint.
                    sk_font
                };
            let (glyphs_out, positions_out) =
                builder.alloc_run_pos(raster_font, run.num_glyphs, None);
            glyphs_out.copy_from_slice(&run.glyphs);

            let mut x = run_x;
            for i in 0..run.num_glyphs {
                let local_x = x + run.offsets[i].0;
                let raster_x = device_origin_x.map_or(local_x, |origin| {
                    let mut device_x = origin + local_x;
                    if raster_policy == TextRasterPolicy::ChromiumNativeControl {
                        // Retain the existing native-control positioning.
                        // Authored glyphs carry their finer shaped advances to
                        // Skia, which selects the final physical LCD phase.
                        device_x = (device_x * 64.0).round() / 64.0;
                    }
                    device_x - origin
                });
                positions_out[i] =
                    Point::new(raster_x * device_scale, run.offsets[i].1 * device_scale);
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
    ///
    /// When multiple glyphs share the same cluster value (combining marks,
    /// decompositions), they are grouped together. The character coverage
    /// of a cluster group is `[cluster_value, next_different_cluster_value)`.
    pub fn glyph_range_for_char_range(
        run: &ShapeResultRun,
        char_start: usize,
        char_end: usize,
    ) -> (usize, usize) {
        if run.num_characters == 0 {
            return (0, 0);
        }

        // Use cluster data for precise glyph-to-character mapping.
        // Group glyphs with the same cluster value together. The coverage
        // of a cluster group is [cluster_value, next_different_cluster_value).
        if !run.clusters.is_empty() {
            // Build sorted unique cluster values with their glyph indices.
            let mut glyph_by_cluster: Vec<(usize, usize)> = run
                .clusters
                .iter()
                .enumerate()
                .map(|(gi, &c)| (c, gi))
                .collect();
            glyph_by_cluster.sort_by_key(|(c, gi)| (*c, *gi));

            // Deduplicate cluster values to find distinct cluster boundaries.
            let mut unique_clusters: Vec<usize> =
                glyph_by_cluster.iter().map(|(c, _)| *c).collect();
            unique_clusters.dedup();

            let mut glyph_start = run.num_glyphs;
            let mut glyph_end = 0;
            for (_idx, &(cluster, gi)) in glyph_by_cluster.iter().enumerate() {
                // Find the next *different* cluster value.
                let unique_pos = unique_clusters.iter().position(|&c| c == cluster).unwrap();
                let next_cluster = if unique_pos + 1 < unique_clusters.len() {
                    unique_clusters[unique_pos + 1]
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
        // Use floor() for start and ceil() for end to guarantee at least one
        // glyph for any non-empty character range.
        let ratio = run.num_glyphs as f32 / run.num_characters as f32;
        let gs = (char_start as f32 * ratio).floor() as usize;
        let ge = (char_end as f32 * ratio).ceil() as usize;
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
    pub fn apply_justification(
        &mut self,
        extra_per_space: f32,
        text: &str,
        exclude_trailing: usize,
    ) {
        if extra_per_space <= 0.0 || self.runs.is_empty() {
            return;
        }

        let chars: Vec<char> = text.chars().collect();

        // Pre-compute the set of character indices that are trailing spaces
        // (the last N spaces in LOGICAL order). This avoids depending on
        // glyph iteration order, which differs between LTR and RTL runs.
        let mut trailing_space_indices: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
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
        for run in &mut self.runs {
            let run_start = run.start_index;
            if !run.clusters.is_empty() {
                // With cluster data: determine each glyph's character coverage
                // using unique cluster boundaries. For a ligature covering N
                // characters, add extra_per_gap * (N - 1) to the first glyph
                // in the cluster group (if it's not covering the last character).
                // Skip duplicate-cluster glyphs (don't add extra spacing).
                let mut sorted_pairs: Vec<(usize, usize)> = run
                    .clusters
                    .iter()
                    .enumerate()
                    .map(|(gi, &c)| (c, gi))
                    .collect();
                sorted_pairs.sort_by_key(|(c, gi)| (*c, *gi));

                let mut unique_clusters: Vec<usize> =
                    sorted_pairs.iter().map(|(c, _)| *c).collect();
                unique_clusters.dedup();

                // Track which cluster groups we've already processed.
                let mut processed_clusters = std::collections::HashSet::new();
                for &(cluster, gi) in &sorted_pairs {
                    if processed_clusters.contains(&cluster) {
                        // Duplicate-cluster glyph — skip.
                        continue;
                    }
                    processed_clusters.insert(cluster);

                    let abs_cluster = run_start + cluster;
                    let unique_pos = unique_clusters.iter().position(|&c| c == cluster).unwrap();
                    let next_cluster = if unique_pos + 1 < unique_clusters.len() {
                        unique_clusters[unique_pos + 1]
                    } else {
                        run.num_characters
                    };
                    let chars_covered = next_cluster - cluster;
                    let abs_end = run_start + next_cluster;

                    // Count gaps: each character boundary within/after this
                    // cluster contributes a gap, except the very last character
                    // in the entire result.
                    let mut gaps = 0usize;
                    // Internal gaps within a ligature cluster.
                    if chars_covered > 1 {
                        gaps += chars_covered - 1;
                    }
                    // Boundary gap after this cluster (if not the last char overall).
                    if abs_end <= total_chars - 1 {
                        gaps += 1;
                    } else if abs_end > total_chars - 1 && abs_cluster < total_chars - 1 {
                        // Cluster ends at last char but starts before — count
                        // only the internal gaps up to the second-to-last char.
                        let chars_before_last = (total_chars - 1).saturating_sub(abs_cluster);
                        gaps = chars_before_last;
                    }

                    if gaps > 0 {
                        let extra = extra_per_gap * gaps as f32;
                        run.advances[gi] += extra;
                        total_extra += extra;
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
                    // For RTL runs, glyphs are in visual order so glyph
                    // local_idx may map to a different character. Use cluster
                    // data when available to find the correct glyph.
                    if !run.clusters.is_empty() {
                        for gi in 0..run.num_glyphs {
                            if run.clusters[gi] == local_idx {
                                return run.advances[gi];
                            }
                        }
                    }
                    return run.advances[local_idx];
                } else {
                    // Non-1:1 mapping: use cluster data for precise advance.
                    // Issue 6 fix: distribute based on cluster boundaries
                    // rather than averaging uniformly.
                    if !run.clusters.is_empty() {
                        // Build unique cluster groups and sum advances of
                        // all glyphs sharing the same cluster value.
                        let mut glyph_by_cluster: Vec<(usize, usize)> = run
                            .clusters
                            .iter()
                            .enumerate()
                            .map(|(gi, &c)| (c, gi))
                            .collect();
                        glyph_by_cluster.sort_by_key(|(c, _)| *c);

                        let mut unique_clusters: Vec<usize> =
                            glyph_by_cluster.iter().map(|(c, _)| *c).collect();
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

#[cfg(test)]
mod tests {
    #[test]
    fn fontations_preserves_physical_strike_descriptor_for_real_fonts() {
        use crate::font::{Font, FontCollection, FontDescription};
        use crate::shaping::{TextDirection, TextRasterPolicy, TextShaper};
        use openui_style::FontFamilyList;

        let collection = FontCollection::deterministic_test();
        let shaper = TextShaper::new();
        for family in ["Ahem", "DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono"] {
            for size in [10.0, 12.0, 16.0, 20.0, 24.0] {
                let font = Font::new_in_collection(
                    FontDescription {
                        family: FontFamilyList::single(family),
                        size,
                        specified_size: size,
                        ..FontDescription::default()
                    },
                    std::sync::Arc::clone(&collection),
                );
                let shaped = shaper.shape("XX", &font, TextDirection::Ltr);
                let run = shaped.runs.first().unwrap();
                for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
                    let mut source = run.font_data.sk_font().clone();
                    source.set_size(size * scale);
                    source.set_edging(Edging::SubpixelAntiAlias);
                    source.set_subpixel(true);
                    source.set_linear_metrics(true);
                    let converted = super::fontations_compatible_font(
                        &source,
                        &run.glyphs,
                        TextRasterPolicy::ChromiumAuthorLcd,
                    )
                    .unwrap();
                    // Skia selects coverage from this descriptor together
                    // with the replay matrix. A unit-size custom font hides
                    // the effective strike size from that shared decision.
                    assert_eq!(
                            converted.size(),
                            source.size(),
                            "Fontations must preserve the physical strike descriptor: {family}, {size}, {scale}"
                        );
                    assert_eq!(converted.edging(), source.edging());
                    assert_eq!(converted.is_subpixel(), source.is_subpixel());
                    assert_eq!(converted.is_linear_metrics(), source.is_linear_metrics());
                }
            }
        }
    }
    use skia_safe::{font::Edging, FontHinting};

    #[test]
    fn authored_lcd_origin_retains_shaped_advance_precision() {
        use crate::font::{Font, FontCollection, FontDescription};
        use crate::shaping::{TextDirection, TextRasterPolicy, TextShaper};
        use openui_geometry::RasterConfiguration;
        use openui_style::FontFamilyList;

        let collection = FontCollection::deterministic_test();
        let shaper = TextShaper::new();
        for family in ["Ahem", "DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono"] {
            for size in [10.0, 12.0, 16.0, 20.0, 24.0] {
                let font = Font::new_in_collection(
                    FontDescription {
                        family: FontFamilyList::single(family),
                        size,
                        specified_size: size,
                        raster_configuration: RasterConfiguration::chromium_linux_fontations_lcd(),
                        ..FontDescription::default()
                    },
                    std::sync::Arc::clone(&collection),
                );
                let shaped = shaper.shape("XX", &font, TextDirection::Ltr);
                assert_eq!(shaped.num_glyphs(), 2);
                for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
                    // Adding the container's layout origin must not quantize
                    // advances within the run. Use the existing local builder
                    // as the reference for the same glyphs and physical strike.
                    let local = shaped
                        .to_text_blob_with_raster_policy_at_scale(
                            None,
                            TextRasterPolicy::ChromiumAuthorLcd,
                            scale,
                        )
                        .unwrap();
                    let positioned = shaped
                        .to_text_blob_with_raster_policy_at_scale(
                            Some(20.125),
                            TextRasterPolicy::ChromiumAuthorLcd,
                            scale,
                        )
                        .unwrap();
                    assert_eq!(
                        positioned.bounds(),
                        local.bounds(),
                        "authored glyph origin must retain shaped advance precision: {family}, size={size}, scale={scale}"
                    );
                }
            }
        }
    }

    #[test]
    fn fontations_preserves_requested_mask_and_position_settings() {
        use crate::font::{Font, FontCollection, FontDescription};
        use crate::shaping::{TextDirection, TextRasterPolicy, TextShaper};
        use openui_style::FontFamilyList;

        let font = Font::new_in_collection(
            FontDescription {
                family: FontFamilyList::single("DejaVu Serif"),
                size: 17.0,
                specified_size: 17.0,
                ..FontDescription::default()
            },
            FontCollection::deterministic_test(),
        );
        let shaped = TextShaper::new().shape("Xe", &font, TextDirection::Ltr);
        let run = shaped.runs.first().unwrap();
        for policy in [
            TextRasterPolicy::ChromiumAuthorLcd,
            TextRasterPolicy::ChromiumNativeControl,
            TextRasterPolicy::ChromiumEmbeddedDocument,
            TextRasterPolicy::ChromiumAliased,
        ] {
            for edging in [Edging::Alias, Edging::AntiAlias, Edging::SubpixelAntiAlias] {
                for subpixel in [false, true] {
                    for autohint in [false, true] {
                        let mut source = run.font_data.sk_font().clone();
                        source.set_edging(edging);
                        source.set_hinting(FontHinting::None);
                        source.set_subpixel(subpixel);
                        source.set_linear_metrics(subpixel);
                        source.set_force_auto_hinting(autohint);
                        let converted =
                            super::fontations_compatible_font(&source, &run.glyphs, policy)
                                .unwrap();
                        assert_eq!(converted.edging(), edging, "{policy:?}");
                        assert_eq!(converted.is_subpixel(), subpixel, "{policy:?}");
                        assert_eq!(converted.is_linear_metrics(), subpixel, "{policy:?}");
                        assert_eq!(converted.is_force_auto_hinting(), autohint, "{policy:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn fontations_lcd_hints_at_physical_size_before_replay() {
        use crate::font::{Font, FontCollection, FontDescription};
        use crate::shaping::{TextDirection, TextRasterPolicy, TextShaper};
        use openui_geometry::RasterConfiguration;
        use openui_style::FontFamilyList;

        let collection = FontCollection::deterministic_test();
        let shaper = TextShaper::new();
        let ink_bounds = |size, scale| {
            let description = FontDescription {
                family: FontFamilyList::single("Ahem"),
                size,
                specified_size: size,
                device_scale_factor: f64::from(scale),
                raster_configuration: RasterConfiguration::chromium_linux_lcd(),
                ..FontDescription::default()
            };
            let font = Font::new_in_collection(description, std::sync::Arc::clone(&collection));
            let result = shaper.shape("X", &font, TextDirection::Ltr);
            assert_eq!(result.num_glyphs(), 1);
            let blob = result
                .to_text_blob_with_raster_policy_at_scale(
                    Some(0.0),
                    TextRasterPolicy::ChromiumAuthorLcd,
                    scale,
                )
                .unwrap();
            *blob.bounds()
        };

        // A single glyph at the same physical font size has the same hinted
        // outline regardless of how the logical size and device scale divide
        // that size. Hinting at CSS size and then scaling the fitted outline
        // changes its ink bounds at fractional scales.
        for (size, scale) in [
            (16.0, 1.25),
            (16.0, 1.5),
            (16.0, 2.0),
            (16.0, 3.0),
            (10.0, 2.5),
        ] {
            assert_eq!(
                ink_bounds(size, scale),
                ink_bounds(size * scale, 1.0),
                "logical size {size}, device scale {scale}"
            );
        }
    }
}
