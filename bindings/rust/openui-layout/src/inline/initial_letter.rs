//! CSS `initial-letter` property — drop caps and raised caps.
//!
//! CSS Inline Level 3 §5: `initial-letter` sizes the first letter of a
//! block container to span multiple lines, creating a drop-cap or raised-cap
//! effect. The property has two values: size (number of lines to span)
//! and sink (number of lines to drop below the first baseline).
//!
//! Blink: `initial_letter_utils.cc` in
//! `third_party/blink/renderer/core/layout/inline/`.
//!
//! ## Drop cap
//! `initial-letter: 3` — letter is 3 lines tall, sinks 3 lines (default).
//! Adjacent text flows around the letter's exclusion area.
//!
//! ## Raised cap
//! `initial-letter: 3 1` — letter is 3 lines tall, sits on the first
//! baseline (sink = 1). The letter protrudes above the block container.
//!
//! ## Sunken cap
//! `initial-letter: 3 2` — letter is 3 lines tall, sinks 2 lines.
//! Partially sunken into the paragraph.

use crate::inline::items_builder::style_to_font_description;
use openui_geometry::LayoutUnit;
use openui_style::{ComputedStyle, InitialLetter};
use openui_text::Font;

/// Computed layout metrics for an initial letter.
///
/// These values are derived from the `initial-letter` property and the
/// surrounding line metrics. They drive the sizing and positioning of
/// the letter and the text exclusion area around it.
#[derive(Debug, Clone, Copy)]
pub struct InitialLetterLayout {
    /// Total height of the initial letter (size × line_height).
    pub letter_height: LayoutUnit,
    /// Width of the initial letter (from glyph metrics or font-size ratio).
    pub letter_width: LayoutUnit,
    /// Block-offset of the top of the letter from the block container's
    /// content edge. For drop caps, this is 0. For sunken caps, this may
    /// be negative if the letter starts above the first line.
    pub block_offset: LayoutUnit,
    /// Number of adjacent text lines that must exclude the letter's area.
    /// This equals `ceil(effective_sink)`.
    pub exclusion_lines: u32,
    /// The font size computed to make the letter span the required height.
    pub computed_font_size: f32,
    /// Whether this is a raised cap (sink <= 1).
    pub is_raised: bool,
    /// Whether this is a drop cap (sink == size).
    pub is_drop_cap: bool,
}

/// Compute the layout metrics for an initial letter.
///
/// Given the `initial-letter` property values and the surrounding line
/// metrics, computes the size, position, and exclusion area.
///
/// # Parameters
/// - `initial_letter`: The parsed CSS `initial-letter` property.
/// - `line_height`: The computed line height of the surrounding text.
/// - `font_size`: The base font size of the surrounding text.
///
/// # Returns
/// Layout metrics for positioning the initial letter.
pub fn compute_initial_letter_layout(
    initial_letter: &InitialLetter,
    line_height: f32,
    font_size: f32,
) -> InitialLetterLayout {
    let size = initial_letter.size;
    let sink = initial_letter.effective_sink();

    // The letter height spans `size` lines
    let letter_height = size * line_height;

    // Compute the font size needed to achieve the target height.
    // The letter should fill the full height (size * line_height).
    // We scale the font size proportionally:
    // computed_font_size / base_font_size = letter_height / line_height
    // → computed_font_size = font_size * size
    let computed_font_size = font_size * size;

    // Query real font metrics at the computed size to derive the letter width.
    let mut letter_style = ComputedStyle::default();
    letter_style.font_size = computed_font_size;
    let font_desc = style_to_font_description(&letter_style);
    let font = Font::new(font_desc);
    let metrics = font.font_metrics().copied().unwrap_or_default();
    let letter_width = metrics.zero_width.max(metrics.cap_height * 0.7);

    // Block offset: how far the top of the letter is from the content edge.
    // For a drop cap (sink == size), the top aligns with the first line's top → 0.
    // For a raised cap (sink == 1), the letter protrudes above → negative offset.
    // For a sunken cap (1 < sink < size), partially above → computed offset.
    let block_offset = if sink >= size {
        0.0 // Full drop cap: top aligns with first line
    } else if sink <= 1.0 {
        -((size - 1.0) * line_height) // Raised cap: protrudes above
    } else {
        -((size - sink) * line_height) // Sunken: partially above
    };

    // Number of lines that need text exclusion
    let exclusion_lines = sink.ceil() as u32;

    let is_raised = sink <= 1.0;
    let is_drop_cap = (sink - size).abs() < 0.01;

    InitialLetterLayout {
        letter_height: LayoutUnit::from_f32(letter_height),
        letter_width: LayoutUnit::from_f32(letter_width),
        block_offset: LayoutUnit::from_f32(block_offset),
        exclusion_lines,
        computed_font_size,
        is_raised,
        is_drop_cap,
    }
}

/// Compute the exclusion rectangle for an initial letter.
///
/// The exclusion area is the region where adjacent text lines must not
/// intrude. It occupies the full width of the letter (plus any specified
/// margin) and extends vertically for `exclusion_lines` worth of line
/// heights.
///
/// # Parameters
/// - `layout`: The computed initial letter layout metrics.
/// - `margin_inline_end`: Extra space between the letter and adjacent text.
///
/// # Returns
/// `(inline_start, inline_end, block_start, block_end)` in LayoutUnit.
pub fn compute_exclusion_rect(
    layout: &InitialLetterLayout,
    margin_inline_end: LayoutUnit,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let inline_start = LayoutUnit::from_i32(0);
    let inline_end = layout.letter_width + margin_inline_end;
    let block_start = layout.block_offset;
    let block_end = layout.block_offset + layout.letter_height;

    (inline_start, inline_end, block_start, block_end)
}

/// Determine if a line at the given block offset is within the exclusion
/// zone of the initial letter.
///
/// # Parameters
/// - `line_block_offset`: The block-start position of the line.
/// - `line_height`: Height of the line.
/// - `exclusion_block_start`: Block-start of the exclusion area.
/// - `exclusion_block_end`: Block-end of the exclusion area.
///
/// # Returns
/// `true` if the line overlaps with the exclusion zone.
pub fn line_in_exclusion_zone(
    line_block_offset: LayoutUnit,
    line_height: LayoutUnit,
    exclusion_block_start: LayoutUnit,
    exclusion_block_end: LayoutUnit,
) -> bool {
    let line_end = line_block_offset + line_height;
    // Lines overlap if neither is entirely before or after the other
    line_block_offset < exclusion_block_end && line_end > exclusion_block_start
}

/// Compute the available inline width for a line that is within the
/// exclusion zone of an initial letter.
///
/// The line's available width is reduced by the letter's inline extent
/// plus any margin.
///
/// # Parameters
/// - `total_available`: The full available inline width of the container.
/// - `letter_inline_extent`: The inline extent of the initial letter
///   (width + margin-inline-end).
///
/// # Returns
/// The reduced available width for the line.
pub fn available_width_with_exclusion(
    total_available: LayoutUnit,
    letter_inline_extent: LayoutUnit,
) -> LayoutUnit {
    let width = total_available - letter_inline_extent;
    if width < LayoutUnit::from_i32(0) {
        LayoutUnit::from_i32(0)
    } else {
        width
    }
}

/// Style adjustments for the initial letter.
///
/// The initial letter typically needs:
/// - Enlarged font-size to span the required number of lines
/// - Optionally float: left (for traditional drop-cap layout)
/// - margin-inline-end for spacing from adjacent text
///
/// This creates a modified style suitable for laying out the initial letter.
pub fn create_initial_letter_style(
    base_style: &ComputedStyle,
    layout: &InitialLetterLayout,
) -> ComputedStyle {
    let mut style = base_style.clone();
    style.font_size = layout.computed_font_size;
    style
}

/// Validate initial-letter property values.
///
/// CSS Inline Level 3 §5.1:
/// - size must be >= 1
/// - sink, if specified, must be >= 1
/// - normal → None (no initial letter)
pub fn validate_initial_letter(size: f32, sink: Option<f32>) -> Option<InitialLetter> {
    if size < 1.0 {
        return None;
    }
    if let Some(s) = sink {
        if s < 1.0 {
            return None;
        }
    }
    Some(InitialLetter { size, sink })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: f32) -> LayoutUnit {
        LayoutUnit::from_f32(v)
    }

    #[test]
    fn drop_cap_3_lines() {
        let il = InitialLetter {
            size: 3.0,
            sink: None,
        };
        let layout = compute_initial_letter_layout(&il, 20.0, 16.0);

        assert_eq!(layout.letter_height, lu(60.0)); // 3 × 20
        assert_eq!(layout.computed_font_size, 48.0); // 16 × 3
        assert_eq!(layout.block_offset, lu(0.0)); // Full drop cap
        assert_eq!(layout.exclusion_lines, 3);
        assert!(layout.is_drop_cap);
        assert!(!layout.is_raised);
    }

    #[test]
    fn raised_cap_3_lines() {
        let il = InitialLetter {
            size: 3.0,
            sink: Some(1.0),
        };
        let layout = compute_initial_letter_layout(&il, 20.0, 16.0);

        assert_eq!(layout.letter_height, lu(60.0));
        assert_eq!(layout.block_offset, lu(-40.0)); // Protrudes 2 lines above
        assert_eq!(layout.exclusion_lines, 1);
        assert!(layout.is_raised);
        assert!(!layout.is_drop_cap);
    }

    #[test]
    fn sunken_cap_3_size_2_sink() {
        let il = InitialLetter {
            size: 3.0,
            sink: Some(2.0),
        };
        let layout = compute_initial_letter_layout(&il, 20.0, 16.0);

        assert_eq!(layout.letter_height, lu(60.0));
        assert_eq!(layout.block_offset, lu(-20.0)); // 1 line above
        assert_eq!(layout.exclusion_lines, 2);
        assert!(!layout.is_raised);
        assert!(!layout.is_drop_cap);
    }

    #[test]
    fn exclusion_rect_basic() {
        let il = InitialLetter {
            size: 3.0,
            sink: None,
        };
        let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
        let margin = lu(4.0);

        let (start, end, bstart, bend) = compute_exclusion_rect(&layout, margin);

        assert_eq!(start, lu(0.0));
        assert_eq!(bstart, lu(0.0));
        assert_eq!(bend, lu(60.0));
        assert!(end > lu(0.0)); // letter_width + margin
    }

    #[test]
    fn line_exclusion_detection() {
        // Exclusion zone: block 0..60
        assert!(line_in_exclusion_zone(lu(0.0), lu(20.0), lu(0.0), lu(60.0)));
        assert!(line_in_exclusion_zone(
            lu(20.0),
            lu(20.0),
            lu(0.0),
            lu(60.0)
        ));
        assert!(line_in_exclusion_zone(
            lu(40.0),
            lu(20.0),
            lu(0.0),
            lu(60.0)
        ));
        assert!(!line_in_exclusion_zone(
            lu(60.0),
            lu(20.0),
            lu(0.0),
            lu(60.0)
        ));
        assert!(!line_in_exclusion_zone(
            lu(80.0),
            lu(20.0),
            lu(0.0),
            lu(60.0)
        ));
    }

    #[test]
    fn available_width_reduced() {
        let total = lu(300.0);
        let letter = lu(50.0);
        let avail = available_width_with_exclusion(total, letter);
        assert_eq!(avail, lu(250.0));
    }

    #[test]
    fn available_width_clamped_to_zero() {
        let total = lu(30.0);
        let letter = lu(50.0);
        let avail = available_width_with_exclusion(total, letter);
        assert_eq!(avail, lu(0.0));
    }

    #[test]
    fn validate_normal_values() {
        assert!(validate_initial_letter(3.0, None).is_some());
        assert!(validate_initial_letter(3.0, Some(2.0)).is_some());
        assert!(validate_initial_letter(1.0, Some(1.0)).is_some());
    }

    #[test]
    fn validate_rejects_invalid() {
        assert!(validate_initial_letter(0.5, None).is_none());
        assert!(validate_initial_letter(3.0, Some(0.5)).is_none());
    }

    #[test]
    fn initial_letter_effective_sink() {
        let il = InitialLetter {
            size: 3.0,
            sink: None,
        };
        assert_eq!(il.effective_sink(), 3.0);

        let il2 = InitialLetter {
            size: 3.0,
            sink: Some(1.0),
        };
        assert_eq!(il2.effective_sink(), 1.0);
    }

    #[test]
    fn initial_letter_compute_height() {
        let il = InitialLetter {
            size: 3.0,
            sink: None,
        };
        assert_eq!(il.compute_height(20.0), 60.0);
    }

    #[test]
    fn initial_letter_sink_offset() {
        let il = InitialLetter {
            size: 3.0,
            sink: None,
        };
        assert_eq!(il.compute_sink_offset(20.0), 40.0); // (3-1) × 20

        let il2 = InitialLetter {
            size: 3.0,
            sink: Some(1.0),
        };
        assert_eq!(il2.compute_sink_offset(20.0), 0.0); // raised cap
    }

    #[test]
    fn create_initial_letter_style_sets_font_size() {
        let base = ComputedStyle::initial();
        let il = InitialLetter {
            size: 3.0,
            sink: None,
        };
        let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
        let style = create_initial_letter_style(&base, &layout);
        assert_eq!(style.font_size, 48.0);
    }

    #[test]
    fn two_line_drop_cap() {
        let il = InitialLetter {
            size: 2.0,
            sink: None,
        };
        let layout = compute_initial_letter_layout(&il, 24.0, 16.0);

        assert_eq!(layout.letter_height, lu(48.0));
        assert_eq!(layout.computed_font_size, 32.0);
        assert_eq!(layout.exclusion_lines, 2);
        assert!(layout.is_drop_cap);
    }

    #[test]
    fn exclusion_for_raised_cap_only_first_line() {
        let il = InitialLetter {
            size: 4.0,
            sink: Some(1.0),
        };
        let layout = compute_initial_letter_layout(&il, 20.0, 16.0);

        // Only the first line should be excluded
        assert_eq!(layout.exclusion_lines, 1);

        // But the letter protrudes 3 lines above
        assert_eq!(layout.block_offset, lu(-60.0));
    }
}
