//! ShapeResult — output of text shaping.
//!
//! Mirrors Blink's `ShapeResult` (`platform/fonts/shaping/shape_result.h`).
//! Contains glyph runs with IDs, positions, and per-character metadata
//! for cursor placement, hit testing, and line breaking.

use std::sync::Arc;

use skia_safe::{
    font::Edging, utils::CustomTypefaceBuilder, FontHinting, FontMetrics, PathBuilder, Point,
    TextBlob, TextBlobBuilder,
};
use skrifa::{
    instance::Size,
    outline::{
        DrawSettings, Engine, GlyphStyles, HintingInstance, HintingOptions, OutlinePen, SmoothMode,
        Target,
    },
    FontRef, GlyphId as SkrifaGlyphId, MetadataProvider,
};

use crate::font::FontPlatformData;

/// Selects the outline rasterizer used when a shaped run becomes a Skia
/// text blob. Chromium's Linux controls and hinted author text use
/// Fontations-compatible paths; unhinted author strikes retain their resolved
/// Skia font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextRasterPolicy {
    #[default]
    Skia,
    ChromiumNativeControl,
    ChromiumEmbeddedDocument,
    ChromiumAuthorLcd,
    ChromiumAliased,
}

/// Match Chromium's retained 10px LCD mask in the one fixed-point phase cell
/// where direct Skia switches to the following cached mask early. Larger
/// strikes use Skia's native phase boundary.
fn chromium_lcd_raster_x(device_x: f32, font_size: f32) -> f32 {
    let phase = device_x.rem_euclid(1.0);
    if (font_size - 10.0).abs() < f32::EPSILON && (24.0 / 64.0..25.0 / 64.0).contains(&phase) {
        device_x - phase + 1.0 / 3.0
    } else {
        device_x
    }
}

fn chromium_lcd_strike_y_offset(
    device_scale: f32,
    edging: Edging,
    hinting: FontHinting,
    font_size: f32,
    family: &str,
) -> f32 {
    // Chromium's retained 10px no-hint LCD strike keeps its origin one
    // logical pixel above Skia's direct scaled replay. Ahem is a synthetic
    // geometric face and does not use this outline-strike origin.
    if (device_scale - 1.0).abs() > f32::EPSILON
        && edging == Edging::SubpixelAntiAlias
        && hinting == FontHinting::None
        && (font_size - 10.0).abs() < f32::EPSILON
        && !family.eq_ignore_ascii_case("Ahem")
    {
        1.0
    } else {
        0.0
    }
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

fn fontations_compatible_font(
    source_font: &skia_safe::Font,
    glyphs: &[u16],
    raster_policy: TextRasterPolicy,
) -> Option<skia_safe::Font> {
    let (font_data, ttc_index) = source_font.typeface().to_font_data()?;
    let font_ref = FontRef::from_index(&font_data, ttc_index as u32).ok()?;
    let outlines = font_ref.outline_glyphs();
    let size = source_font.size();
    // The resolved typeface includes authored variation settings and derived
    // weight, width, slant and optical size. Reading its original font bytes
    // alone loses that instance and paints the default glyph outline.
    let coordinates = source_font.typeface().variation_design_position()?;
    let normalized_location = font_ref
        .axes()
        .location(coordinates.iter().map(|coordinate| {
            (
                skrifa::Tag::new(&(*coordinate.axis).to_be_bytes()),
                coordinate.value,
            )
        }));
    let location = skrifa::instance::LocationRef::from(&normalized_location);
    let hinting = match raster_policy {
        TextRasterPolicy::ChromiumNativeControl
        | TextRasterPolicy::ChromiumAuthorLcd
        | TextRasterPolicy::ChromiumEmbeddedDocument => {
            let glyph_styles = GlyphStyles::new(&outlines);
            HintingInstance::new(
                &outlines,
                Size::new(size),
                location,
                HintingOptions {
                    engine: Engine::Auto(Some(glyph_styles)),
                    target: Target::Smooth {
                        mode: SmoothMode::Light,
                        symmetric_rendering: true,
                        preserve_linear_metrics: false,
                    },
                },
            )
            .ok()?
        }
        // An explicitly unhinted resolved strike must keep its original
        // outline. Monochrome coverage alone does not enable outline fitting.
        TextRasterPolicy::ChromiumAliased if source_font.hinting() == FontHinting::None => {
            return None;
        }
        TextRasterPolicy::ChromiumAliased => HintingInstance::new(
            &outlines,
            Size::new(size),
            location,
            skrifa::outline::HintingMode::Strong,
        )
        .ok()?,
        TextRasterPolicy::Skia => return None,
    };

    let mut builder = CustomTypefaceBuilder::new();
    builder.set_metrics(&FontMetrics::default(), 1.0);
    for &glyph_id in glyphs {
        let glyph = outlines.get(SkrifaGlyphId::new(glyph_id as u32))?;
        let mut pen = SkiaPathPen::default();
        glyph
            .draw(DrawSettings::hinted(&hinting, false), &mut pen)
            .ok()?;
        let path = pen.0.detach();
        builder.set_glyph(glyph_id, 0.0, &path);
    }
    let typeface = builder.detach()?;
    let mut font = source_font.with_size(1.0)?;
    font.set_typeface(typeface);
    font.set_hinting(FontHinting::None);
    let native_control = matches!(
        raster_policy,
        TextRasterPolicy::ChromiumNativeControl | TextRasterPolicy::ChromiumAuthorLcd
    );
    let embedded_document = raster_policy == TextRasterPolicy::ChromiumEmbeddedDocument;
    // Chromium disables subpixel positioning for monochrome strikes. Retain
    // it for the antialiased policies, whose masks depend on the glyph phase.
    font.set_subpixel(raster_policy != TextRasterPolicy::ChromiumAliased);
    font.set_linear_metrics(native_control);
    font.set_edging(if native_control {
        Edging::SubpixelAntiAlias
    } else if embedded_document {
        Edging::AntiAlias
    } else {
        Edging::Alias
    });
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

    /// Build a text blob using Chromium's Linux 10px LCD phase boundary.
    ///
    /// Skia's direct FreeType path changes its cached RGB mask at 24/64 px,
    /// while the pinned Chromium FreeType display-list path retains the prior
    /// phase until 25/64 px. Positions in that single fixed-point cell are
    /// represented at the middle third without changing logical advances or
    /// fragment geometry.
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
            // Chromium asks FreeType/fontations for the device-size aliased
            // strike. Building the compatible outline at the CSS size and
            // scaling its already grid-fitted path widened every rotated Ahem
            // glyph by another device pixel (and two pixels at 1.5x/2x).
            // Hint at the physical size once and keep that custom outline at
            // its unit font size; glyph positions are converted below.
            let mut physical_source_font;
            let compatible_source_font = if raster_policy == TextRasterPolicy::ChromiumAliased
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
                    fontations_compatible_font(compatible_source_font, &run.glyphs, raster_policy)
                })
                .flatten();
            let sk_font = compatible_font.as_ref().unwrap_or(source_font);
            let mut physical_font;
            let compatible_outline_is_physical = compatible_font.is_some()
                && raster_policy == TextRasterPolicy::ChromiumAliased
                && (device_scale - 1.0).abs() > f32::EPSILON;
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
                    if matches!(
                        raster_policy,
                        TextRasterPolicy::ChromiumNativeControl
                            | TextRasterPolicy::ChromiumAuthorLcd
                    ) {
                        // Blink hands native-control glyph origins to Skia in
                        // LayoutUnit coordinates. Preserve that 1/64-device-
                        // pixel boundary before Skia selects its LCD phase.
                        device_x = (device_x * 64.0).round() / 64.0;
                    }
                    chromium_lcd_raster_x(device_x, source_font.size()) - origin
                });
                positions_out[i] = Point::new(
                    raster_x * device_scale,
                    (run.offsets[i].1
                        - chromium_lcd_strike_y_offset(
                            device_scale,
                            source_font.edging(),
                            source_font.hinting(),
                            source_font.size(),
                            &source_font.typeface().family_name(),
                        ))
                        * device_scale,
                );
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
    use super::{chromium_lcd_raster_x, chromium_lcd_strike_y_offset};
    use skia_safe::{font::Edging, FontHinting};

    #[test]
    fn fontations_keeps_the_resolved_variable_font_instance() {
        use skia_safe::font_arguments::{variation_position::Coordinate, VariationPosition};
        use skia_safe::{Font, FontArguments, FontMgr};

        // This independent WPT font defines the varied A as the upper-half
        // block and its default A as the lower-half block.
        let bytes = include_bytes!("../../tests/data/variabletest_box.ttf");
        let face = FontMgr::default().new_from_data(bytes, None).unwrap();
        let a = face.unichar_to_glyph('A' as i32);
        let upper = face.unichar_to_glyph('\u{2580}' as i32);
        let lower = face.unichar_to_glyph('\u{2584}' as i32);
        assert!(a != 0 && upper != 0 && lower != 0);
        let coordinates = [Coordinate {
            axis: u32::from_be_bytes(*b"UPWD").into(),
            value: 350.0,
        }];
        let varied_face = face
            .clone_with_arguments(&FontArguments::new().set_variation_design_position(
                VariationPosition {
                    coordinates: &coordinates,
                },
            ))
            .unwrap();
        for size in [20.0, 40.0, 64.0, 200.0] {
            let reference = Font::from_typeface(&face, size);
            let varied = Font::from_typeface(&varied_face, size);
            for policy in [
                super::TextRasterPolicy::ChromiumNativeControl,
                super::TextRasterPolicy::ChromiumEmbeddedDocument,
                super::TextRasterPolicy::ChromiumAuthorLcd,
                super::TextRasterPolicy::ChromiumAliased,
            ] {
                let expected =
                    super::fontations_compatible_font(&reference, &[a, upper, lower], policy)
                        .unwrap();
                let actual = super::fontations_compatible_font(&varied, &[a], policy).unwrap();
                let render_ink = |font: &Font, glyph| {
                    let dimension = size.ceil() as i32 + 8;
                    let mut surface =
                        skia_safe::surfaces::raster_n32_premul((dimension, dimension)).unwrap();
                    let canvas = surface.canvas();
                    canvas.clear(skia_safe::Color::WHITE);
                    canvas.translate((4.0, size + 4.0));
                    let mut paint = skia_safe::Paint::default();
                    paint
                        .set_color(skia_safe::Color::BLACK)
                        .set_anti_alias(true);
                    canvas.draw_path(&font.get_path(glyph).unwrap(), &paint);
                    surface
                        .image_snapshot()
                        .encode(None, skia_safe::EncodedImageFormat::PNG, None)
                        .unwrap()
                        .as_bytes()
                        .to_vec()
                };
                // Equal path bounds include degenerate contours and cannot prove
                // that the variation was applied. Compare the same glyph
                // before and after mutation; different characters can use
                // different automatic hinting styles.
                let upper_ink = render_ink(&expected, upper);
                let lower_ink = render_ink(&expected, lower);
                assert!(
                    upper_ink != lower_ink,
                    "independent reference glyphs must have different ink"
                );
                assert!(
                    render_ink(&actual, a) != render_ink(&expected, a),
                    "the resolved variable instance must reach outline painting: {size}, {policy:?}"
                );
            }
        }
    }

    #[test]
    fn chromium_lcd_phase_retains_only_the_24_to_25_sixty_fourths_cell() {
        let retained = chromium_lcd_raster_x(269.384_77, 10.0);
        assert!((retained - (269.0 + 1.0 / 3.0)).abs() < 0.000_01);
        assert_eq!(chromium_lcd_raster_x(269.390_625, 10.0), 269.390_625);
        assert_eq!(chromium_lcd_raster_x(269.374_97, 10.0), 269.374_97);
        assert_eq!(chromium_lcd_raster_x(269.384_77, 20.0), 269.384_77);
    }

    #[test]
    fn chromium_lcd_strike_y_offset_is_limited_to_scaled_10px_outline_faces() {
        assert_eq!(
            chromium_lcd_strike_y_offset(
                1.25,
                Edging::SubpixelAntiAlias,
                FontHinting::None,
                10.0,
                "DejaVu Sans Mono",
            ),
            1.0
        );
        for (scale, edging, hinting, size, family) in [
            (
                1.0,
                Edging::SubpixelAntiAlias,
                FontHinting::None,
                10.0,
                "DejaVu Sans Mono",
            ),
            (
                1.25,
                Edging::AntiAlias,
                FontHinting::None,
                10.0,
                "DejaVu Sans Mono",
            ),
            (
                1.25,
                Edging::SubpixelAntiAlias,
                FontHinting::Slight,
                10.0,
                "DejaVu Sans Mono",
            ),
            (
                1.25,
                Edging::SubpixelAntiAlias,
                FontHinting::None,
                16.0,
                "DejaVu Sans",
            ),
            (
                1.25,
                Edging::SubpixelAntiAlias,
                FontHinting::None,
                10.0,
                "aHeM",
            ),
        ] {
            assert_eq!(
                chromium_lcd_strike_y_offset(scale, edging, hinting, size, family),
                0.0
            );
        }
    }
}
