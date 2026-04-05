//! WPT-equivalent tests for CSS Ruby Annotation Layout Module.
//!
//! Each test corresponds to behaviors verified by WPT css/css-ruby tests.
//! Categories: ruby-position, ruby-align, column-width, overhang,
//! writing-mode interaction, and edge cases.

use openui_layout::ruby::{clamp_overhang, compute_ruby_layout, max_ruby_overhang, RubyInfo, RubyLayout};
use openui_style::{RubyAlign, RubyPosition, WritingMode};

// ── Helpers ─────────────────────────────────────────────────────────────

/// Shorthand: compute layout with defaults for writing mode.
fn layout(
    base_w: f32,
    ann_w: f32,
    font: f32,
    align: RubyAlign,
    pos: RubyPosition,
) -> RubyLayout {
    compute_ruby_layout(base_w, ann_w, font, align, pos, WritingMode::HorizontalTb)
}

const LINE_HEIGHT_FACTOR: f32 = 1.2;

// ═══════════════════════════════════════════════════════════════════════
// § ruby_position — css/css-ruby/ruby-position-over-under
// ═══════════════════════════════════════════════════════════════════════
mod ruby_position {
    use super::*;

    /// ruby-position: over — annotation sits above the base in horizontal-tb,
    /// so annotation_block_offset is negative.
    #[test]
    fn over_produces_negative_block_offset() {
        let r = layout(100.0, 60.0, 12.0, RubyAlign::Center, RubyPosition::Over);
        assert!(
            r.annotation_block_offset < 0.0,
            "Over should place annotation above (negative offset), got {}",
            r.annotation_block_offset
        );
    }

    /// ruby-position: under — annotation sits below the base,
    /// so annotation_block_offset is non-negative.
    #[test]
    fn under_produces_non_negative_block_offset() {
        let r = layout(100.0, 60.0, 12.0, RubyAlign::Center, RubyPosition::Under);
        assert!(
            r.annotation_block_offset >= 0.0,
            "Under should place annotation below (non-negative offset), got {}",
            r.annotation_block_offset
        );
    }

    /// Over block offset magnitude equals annotation_size.
    #[test]
    fn over_block_offset_equals_neg_annotation_size() {
        let r = layout(80.0, 50.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over);
        let expected = -(10.0 * LINE_HEIGHT_FACTOR);
        assert!(
            (r.annotation_block_offset - expected).abs() < 1e-4,
            "Expected {} but got {}",
            expected,
            r.annotation_block_offset
        );
    }

    /// Under block offset is exactly zero.
    #[test]
    fn under_block_offset_is_zero() {
        let r = layout(80.0, 50.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Under);
        assert!(
            r.annotation_block_offset.abs() < 1e-4,
            "Under offset should be 0.0, got {}",
            r.annotation_block_offset
        );
    }

    /// Over with vertical-rl writing mode still has negative block offset.
    #[test]
    fn over_vertical_rl() {
        let r = compute_ruby_layout(
            60.0, 40.0, 14.0,
            RubyAlign::Center,
            RubyPosition::Over,
            WritingMode::VerticalRl,
        );
        assert!(r.annotation_block_offset < 0.0);
    }

    /// Under with vertical-lr writing mode still has non-negative block offset.
    #[test]
    fn under_vertical_lr() {
        let r = compute_ruby_layout(
            60.0, 40.0, 14.0,
            RubyAlign::Center,
            RubyPosition::Under,
            WritingMode::VerticalLr,
        );
        assert!(r.annotation_block_offset >= 0.0);
    }

    /// Annotation size is font_size * 1.2 regardless of position.
    #[test]
    fn annotation_size_independent_of_position() {
        let over = layout(50.0, 30.0, 16.0, RubyAlign::Center, RubyPosition::Over);
        let under = layout(50.0, 30.0, 16.0, RubyAlign::Center, RubyPosition::Under);
        assert!(
            (over.annotation_size - under.annotation_size).abs() < 1e-4,
            "annotation_size should be the same regardless of position"
        );
        let expected = 16.0 * LINE_HEIGHT_FACTOR;
        assert!((over.annotation_size - expected).abs() < 1e-4);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// § ruby_align — css/css-ruby/ruby-align
// ═══════════════════════════════════════════════════════════════════════
mod ruby_align {
    use super::*;

    /// SpaceAround with annotation narrower than base: annotation is centered.
    #[test]
    fn space_around_centers_narrow_annotation() {
        let r = layout(100.0, 60.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over);
        // Annotation should be centered: offset = (100 - 60) / 2 = 20
        assert!((r.annotation_offset - 20.0).abs() < 1e-4);
        // Base should also be centered (but it's the wider one so offset ≈ 0)
        assert!(r.base_offset.abs() < 1e-4);
    }

    /// Center alignment: annotation narrower than base is centered.
    #[test]
    fn center_centers_narrow_annotation() {
        let r = layout(100.0, 40.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!((r.annotation_offset - 30.0).abs() < 1e-4);
        assert!(r.base_offset.abs() < 1e-4);
    }

    /// SpaceBetween: both offsets are zero (justification happens later).
    #[test]
    fn space_between_offsets_are_zero() {
        let r = layout(100.0, 60.0, 10.0, RubyAlign::SpaceBetween, RubyPosition::Over);
        assert!(r.base_offset.abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// Start alignment: both offsets are zero.
    #[test]
    fn start_offsets_are_zero() {
        let r = layout(100.0, 60.0, 10.0, RubyAlign::Start, RubyPosition::Over);
        assert!(r.base_offset.abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// When annotation is wider than base, Center centers the base.
    #[test]
    fn center_wider_annotation_centers_base() {
        let r = layout(40.0, 100.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        // base offset = (100 - 40) / 2 = 30
        assert!((r.base_offset - 30.0).abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// SpaceAround with wider annotation: base is centered.
    #[test]
    fn space_around_wider_annotation_centers_base() {
        let r = layout(40.0, 100.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over);
        assert!((r.base_offset - 30.0).abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// SpaceBetween when annotation is wider: offsets still zero.
    #[test]
    fn space_between_wider_annotation_offsets_zero() {
        let r = layout(40.0, 100.0, 10.0, RubyAlign::SpaceBetween, RubyPosition::Over);
        assert!(r.base_offset.abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// Start when annotation is wider: offsets still zero.
    #[test]
    fn start_wider_annotation_offsets_zero() {
        let r = layout(40.0, 100.0, 10.0, RubyAlign::Start, RubyPosition::Over);
        assert!(r.base_offset.abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// Equal widths: all alignments produce zero offsets.
    #[test]
    fn equal_widths_all_alignments_zero_offsets() {
        for align in [
            RubyAlign::Center,
            RubyAlign::SpaceAround,
            RubyAlign::SpaceBetween,
            RubyAlign::Start,
        ] {
            let r = layout(80.0, 80.0, 12.0, align, RubyPosition::Over);
            assert!(
                r.base_offset.abs() < 1e-4 && r.annotation_offset.abs() < 1e-4,
                "Alignment {:?} with equal widths should give zero offsets, got base={} ann={}",
                align,
                r.base_offset,
                r.annotation_offset
            );
        }
    }

    /// Center and SpaceAround produce identical offsets (per spec §4.1).
    #[test]
    fn center_and_space_around_are_equivalent() {
        let c = layout(100.0, 60.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        let sa = layout(100.0, 60.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over);
        assert!((c.base_offset - sa.base_offset).abs() < 1e-4);
        assert!((c.annotation_offset - sa.annotation_offset).abs() < 1e-4);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// § column_width — css/css-ruby/ruby-column-width
// ═══════════════════════════════════════════════════════════════════════
mod column_width {
    use super::*;

    /// Column width is at least as wide as both base and annotation.
    #[test]
    fn column_width_gte_both() {
        let r = layout(120.0, 80.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!(r.column_width >= r.base_width);
        assert!(r.column_width >= r.annotation_width);
    }

    /// When base is wider, column_width == base_width.
    #[test]
    fn base_wider() {
        let r = layout(150.0, 60.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!((r.column_width - 150.0).abs() < 1e-4);
    }

    /// When annotation is wider, column_width == annotation_width.
    #[test]
    fn annotation_wider() {
        let r = layout(60.0, 150.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!((r.column_width - 150.0).abs() < 1e-4);
    }

    /// Equal widths: column_width matches.
    #[test]
    fn equal_widths() {
        let r = layout(100.0, 100.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!((r.column_width - 100.0).abs() < 1e-4);
    }

    /// Column width does not depend on alignment mode.
    #[test]
    fn column_width_independent_of_alignment() {
        let c = layout(100.0, 60.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        let s = layout(100.0, 60.0, 10.0, RubyAlign::Start, RubyPosition::Over);
        let sb = layout(100.0, 60.0, 10.0, RubyAlign::SpaceBetween, RubyPosition::Over);
        let sa = layout(100.0, 60.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over);
        assert!((c.column_width - s.column_width).abs() < 1e-4);
        assert!((c.column_width - sb.column_width).abs() < 1e-4);
        assert!((c.column_width - sa.column_width).abs() < 1e-4);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// § overhang — css/css-ruby/ruby-overhang
// ═══════════════════════════════════════════════════════════════════════
mod overhang {
    use super::*;

    /// max_ruby_overhang is half the font size.
    #[test]
    fn max_overhang_is_half_font_size() {
        assert!((max_ruby_overhang(16.0) - 8.0).abs() < 1e-4);
        assert!((max_ruby_overhang(10.0) - 5.0).abs() < 1e-4);
    }

    /// max_ruby_overhang scales linearly with font size.
    #[test]
    fn max_overhang_scales_linearly() {
        let a = max_ruby_overhang(12.0);
        let b = max_ruby_overhang(24.0);
        assert!((b - 2.0 * a).abs() < 1e-4);
    }

    /// max_ruby_overhang with zero font size is zero.
    #[test]
    fn max_overhang_zero_font() {
        assert!(max_ruby_overhang(0.0).abs() < 1e-4);
    }

    /// clamp_overhang with plenty of space returns up to max per side.
    #[test]
    fn clamp_with_plenty_of_space() {
        // annotation 80, base 40 → excess 40, half_excess = 20
        // max_per_side = 16/2 = 8, available 100 each side
        // clamped to min(20, 8, 100) = 8 each side
        let (before, after) = clamp_overhang(80.0, 40.0, 16.0, 100.0, 100.0);
        assert!((before - 8.0).abs() < 1e-4);
        assert!((after - 8.0).abs() < 1e-4);
    }

    /// clamp_overhang with no available space returns zero.
    #[test]
    fn clamp_with_no_space() {
        let (before, after) = clamp_overhang(80.0, 40.0, 16.0, 0.0, 0.0);
        assert!(before.abs() < 1e-4);
        assert!(after.abs() < 1e-4);
    }

    /// clamp_overhang with partial available space.
    #[test]
    fn clamp_with_partial_space() {
        // excess 40, half = 20, max_per_side = 8
        // available_before = 5 (less than max 8), available_after = 100
        let (before, after) = clamp_overhang(80.0, 40.0, 16.0, 5.0, 100.0);
        assert!((before - 5.0).abs() < 1e-4);
        assert!((after - 8.0).abs() < 1e-4);
    }

    /// When annotation fits within base, no overhang needed.
    #[test]
    fn no_overhang_when_annotation_fits() {
        let (before, after) = clamp_overhang(40.0, 100.0, 16.0, 50.0, 50.0);
        assert!(before.abs() < 1e-4);
        assert!(after.abs() < 1e-4);
    }

    /// clamp_overhang result is always non-negative.
    #[test]
    fn clamp_overhang_non_negative() {
        // Even with negative available values, result should be >= 0
        let (before, after) = clamp_overhang(80.0, 40.0, 16.0, -10.0, -5.0);
        assert!(before >= 0.0);
        assert!(after >= 0.0);
    }

    /// Asymmetric available space produces asymmetric overhang.
    #[test]
    fn asymmetric_available_space() {
        // half_excess = 10, max = 8, avail_before = 3, avail_after = 8
        let (before, after) = clamp_overhang(60.0, 40.0, 16.0, 3.0, 100.0);
        assert!((before - 3.0).abs() < 1e-4);
        assert!((after - 8.0).abs() < 1e-4);
        assert!(before < after);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// § writing_mode_interaction — css/css-ruby/ruby-writing-mode
// ═══════════════════════════════════════════════════════════════════════
mod writing_mode_interaction {
    use super::*;

    /// Horizontal-tb produces consistent layout.
    #[test]
    fn horizontal_tb_layout() {
        let r = compute_ruby_layout(
            80.0, 50.0, 12.0,
            RubyAlign::Center,
            RubyPosition::Over,
            WritingMode::HorizontalTb,
        );
        assert!(r.column_width >= 80.0);
        assert!(r.annotation_block_offset < 0.0);
    }

    /// Vertical-rl produces same column arithmetic as horizontal.
    /// Note: compute_ruby_layout currently treats all writing modes the same
    /// (horizontal-only implementation). These tests verify API compatibility.
    #[test]
    fn vertical_rl_same_column_width() {
        let h = compute_ruby_layout(
            80.0, 50.0, 12.0,
            RubyAlign::Center,
            RubyPosition::Over,
            WritingMode::HorizontalTb,
        );
        let v = compute_ruby_layout(
            80.0, 50.0, 12.0,
            RubyAlign::Center,
            RubyPosition::Over,
            WritingMode::VerticalRl,
        );
        assert!((h.column_width - v.column_width).abs() < 1e-4);
        assert!((h.base_offset - v.base_offset).abs() < 1e-4);
        assert!((h.annotation_offset - v.annotation_offset).abs() < 1e-4);
    }

    /// Vertical-lr layout.
    #[test]
    fn vertical_lr_layout() {
        let r = compute_ruby_layout(
            60.0, 90.0, 14.0,
            RubyAlign::SpaceAround,
            RubyPosition::Under,
            WritingMode::VerticalLr,
        );
        assert!((r.column_width - 90.0).abs() < 1e-4);
        assert!(r.annotation_block_offset >= 0.0);
    }

    /// Sideways-rl: API accepts writing mode (behavior is horizontal-only for now).
    #[test]
    fn sideways_rl_column_width() {
        let r = compute_ruby_layout(
            70.0, 110.0, 10.0,
            RubyAlign::Center,
            RubyPosition::Over,
            WritingMode::SidewaysRl,
        );
        assert!((r.column_width - 110.0).abs() < 1e-4);
    }

    /// Sideways-lr: API accepts writing mode (behavior is horizontal-only for now).
    #[test]
    fn sideways_lr_column_width() {
        let r = compute_ruby_layout(
            110.0, 70.0, 10.0,
            RubyAlign::Start,
            RubyPosition::Under,
            WritingMode::SidewaysLr,
        );
        assert!((r.column_width - 110.0).abs() < 1e-4);
        assert!(r.annotation_block_offset >= 0.0);
    }

    /// Writing mode does not affect annotation_size (intentionally horizontal-only for now).
    #[test]
    fn annotation_size_constant_across_writing_modes() {
        let modes = [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
            WritingMode::SidewaysRl,
            WritingMode::SidewaysLr,
        ];
        let sizes: Vec<f32> = modes
            .iter()
            .map(|&wm| {
                compute_ruby_layout(50.0, 30.0, 14.0, RubyAlign::Center, RubyPosition::Over, wm)
                    .annotation_size
            })
            .collect();
        for s in &sizes {
            assert!((*s - sizes[0]).abs() < 1e-4);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// § edge_cases — css/css-ruby/ruby-edge-cases
// ═══════════════════════════════════════════════════════════════════════
mod edge_cases {
    use super::*;

    /// Zero-width base: column_width == annotation_width.
    #[test]
    fn zero_width_base() {
        let r = layout(0.0, 60.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!((r.column_width - 60.0).abs() < 1e-4);
        assert!((r.base_width).abs() < 1e-4);
    }

    /// Zero-width annotation: column_width == base_width.
    #[test]
    fn zero_width_annotation() {
        let r = layout(80.0, 0.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!((r.column_width - 80.0).abs() < 1e-4);
        assert!((r.annotation_width).abs() < 1e-4);
    }

    /// Very large font size annotation.
    #[test]
    fn very_large_font_size() {
        let r = layout(50.0, 30.0, 1000.0, RubyAlign::Center, RubyPosition::Over);
        let expected_size = 1000.0 * LINE_HEIGHT_FACTOR;
        assert!((r.annotation_size - expected_size).abs() < 1e-2);
        assert!(r.annotation_block_offset < -1000.0);
    }

    /// Both widths zero: column is zero-width.
    #[test]
    fn both_zero_widths() {
        let r = layout(0.0, 0.0, 10.0, RubyAlign::Center, RubyPosition::Over);
        assert!(r.column_width.abs() < 1e-4);
        assert!(r.base_offset.abs() < 1e-4);
        assert!(r.annotation_offset.abs() < 1e-4);
    }

    /// Negative inputs are clamped to zero.
    #[test]
    fn negative_inputs_clamped() {
        let r = layout(-50.0, -30.0, -10.0, RubyAlign::Center, RubyPosition::Over);
        assert!(r.base_width >= 0.0);
        assert!(r.annotation_width >= 0.0);
        assert!(r.column_width >= 0.0);
        assert!(r.annotation_size >= 0.0);
    }

    /// RubyInfo can be constructed with all fields.
    #[test]
    fn ruby_info_construction() {
        let info = RubyInfo {
            annotation_text: "きょう".to_string(),
            annotation_font_size: 7.0,
            position: RubyPosition::Over,
            align: RubyAlign::SpaceAround,
        };
        assert_eq!(info.annotation_font_size, 7.0);
        assert_eq!(info.position, RubyPosition::Over);
        assert_eq!(info.align, RubyAlign::SpaceAround);

        // Use the info to drive a layout computation.
        let r = compute_ruby_layout(
            50.0,
            30.0,
            info.annotation_font_size,
            info.align,
            info.position,
            WritingMode::HorizontalTb,
        );
        assert!(r.column_width >= 50.0);
    }

    /// Fractional pixel values are handled without panicking.
    #[test]
    fn fractional_values() {
        let r = layout(33.33, 66.67, 8.5, RubyAlign::Center, RubyPosition::Over);
        assert!((r.column_width - 66.67).abs() < 1e-2);
        assert!(r.annotation_size > 0.0);
    }

    /// max_ruby_overhang with negative font size is clamped to zero.
    #[test]
    fn max_overhang_negative_font() {
        assert!(max_ruby_overhang(-10.0).abs() < 1e-4);
    }
}
