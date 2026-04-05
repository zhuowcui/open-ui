//! Writing-mode coordinate conversion — extracted from Blink's `WritingModeConverter`.
//!
//! Converts between logical (inline/block) and physical (x/y) coordinates
//! for any combination of writing-mode + direction.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/writing_mode_converter.h
//!         third_party/blink/renderer/platform/text/writing_direction_mode.h

use crate::{
    LogicalOffset, LogicalRect, LogicalSize, PhysicalOffset, PhysicalRect, PhysicalSize,
};

/// Captures the flags from `WritingMode + Direction` needed for coordinate
/// conversion, without depending on the enum crate.
///
/// Use `WritingDirectionMode::horizontal_ltr()` for the default, or construct
/// from booleans that match the `WritingMode` and `Direction` properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WritingDirectionMode {
    is_horizontal: bool,
    is_flipped_blocks: bool,
    is_flipped_lines: bool,
    is_rtl: bool,
}

impl WritingDirectionMode {
    /// Construct from the boolean properties of a writing mode and direction.
    ///
    /// - `is_horizontal`: true for `horizontal-tb`
    /// - `is_flipped_blocks`: true when block direction is reversed
    ///   (`vertical-rl`, `sideways-rl`)
    /// - `is_flipped_lines`: true when inline direction is reversed
    ///   (`sideways-lr`)
    /// - `is_rtl`: true for `direction: rtl`
    #[inline]
    pub const fn new(
        is_horizontal: bool,
        is_flipped_blocks: bool,
        is_flipped_lines: bool,
        is_rtl: bool,
    ) -> Self {
        Self { is_horizontal, is_flipped_blocks, is_flipped_lines, is_rtl }
    }

    /// Default: horizontal-tb + LTR.
    #[inline]
    pub const fn horizontal_ltr() -> Self {
        Self { is_horizontal: true, is_flipped_blocks: false, is_flipped_lines: false, is_rtl: false }
    }

    #[inline]
    pub const fn is_horizontal(self) -> bool { self.is_horizontal }
    #[inline]
    pub const fn is_flipped_blocks(self) -> bool { self.is_flipped_blocks }
    #[inline]
    pub const fn is_flipped_lines(self) -> bool { self.is_flipped_lines }
    #[inline]
    pub const fn is_rtl(self) -> bool { self.is_rtl }
}

impl Default for WritingDirectionMode {
    fn default() -> Self { Self::horizontal_ltr() }
}

// ─── WritingModeConverter ───────────────────────────────────────────────────

/// Converts between logical (inline/block) and physical (x/y) coordinates.
///
/// Mirrors Blink's `WritingModeConverter` from
/// `core/layout/geometry/writing_mode_converter.h`.
///
/// The converter needs an *outer size* — the physical dimensions of the
/// containing block — to resolve flipped-block and RTL offsets.
///
/// # Coordinate system conventions
///
/// | Writing Mode  | InlineStart | InlineEnd | BlockStart | BlockEnd |
/// |---------------|-------------|-----------|------------|----------|
/// | horizontal-tb | left        | right     | top        | bottom   |
/// | vertical-rl   | top         | bottom    | right      | left     |
/// | vertical-lr   | top         | bottom    | left       | right    |
/// | sideways-lr   | bottom      | top       | left       | right    |
///
/// RTL reverses inline-start ↔ inline-end.
#[derive(Debug, Clone, Copy)]
pub struct WritingModeConverter {
    wm: WritingDirectionMode,
    outer_size: PhysicalSize,
}

impl WritingModeConverter {
    #[inline]
    pub const fn new(wm: WritingDirectionMode, outer_size: PhysicalSize) -> Self {
        Self { wm, outer_size }
    }

    #[inline]
    pub const fn writing_direction(&self) -> WritingDirectionMode {
        self.wm
    }

    // ── Size conversions ────────────────────────────────────────────

    /// Convert a logical size to a physical size.
    ///
    /// - horizontal: (inline, block) → (width=inline, height=block)
    /// - vertical:   (inline, block) → (width=block, height=inline)
    #[inline]
    pub fn to_physical_size(&self, logical: LogicalSize) -> PhysicalSize {
        if self.wm.is_horizontal {
            PhysicalSize::new(logical.inline_size, logical.block_size)
        } else {
            PhysicalSize::new(logical.block_size, logical.inline_size)
        }
    }

    /// Convert a physical size to a logical size.
    ///
    /// - horizontal: (width, height) → (inline=width, block=height)
    /// - vertical:   (width, height) → (inline=height, block=width)
    #[inline]
    pub fn to_logical_size(&self, physical: PhysicalSize) -> LogicalSize {
        if self.wm.is_horizontal {
            LogicalSize::new(physical.width, physical.height)
        } else {
            LogicalSize::new(physical.height, physical.width)
        }
    }

    // ── Offset conversions ──────────────────────────────────────────

    /// Convert a logical offset to a physical offset.
    ///
    /// Requires the inner (child) physical size because in flipped-block
    /// or RTL modes the physical origin differs from the logical origin.
    ///
    /// Blink: `WritingModeConverter::ToPhysical(LogicalOffset, PhysicalSize)`.
    pub fn to_physical_offset(
        &self,
        logical: LogicalOffset,
        inner_size: PhysicalSize,
    ) -> PhysicalOffset {
        if self.wm.is_horizontal {
            self.horizontal_to_physical_offset(logical, inner_size)
        } else {
            self.vertical_to_physical_offset(logical, inner_size)
        }
    }

    fn horizontal_to_physical_offset(
        &self,
        logical: LogicalOffset,
        inner_size: PhysicalSize,
    ) -> PhysicalOffset {
        // horizontal-tb: inline → left, block → top
        let left = if self.wm.is_rtl {
            // RTL: inline-start is right edge
            self.outer_size.width - logical.inline_offset - inner_size.width
        } else {
            logical.inline_offset
        };
        PhysicalOffset::new(left, logical.block_offset)
    }

    fn vertical_to_physical_offset(
        &self,
        logical: LogicalOffset,
        inner_size: PhysicalSize,
    ) -> PhysicalOffset {
        // vertical: inline → top/bottom, block → left/right
        let top = if self.wm.is_flipped_lines {
            // sideways-lr: inline runs bottom-to-top
            if self.wm.is_rtl {
                logical.inline_offset
            } else {
                self.outer_size.height - logical.inline_offset - inner_size.height
            }
        } else {
            // vertical-rl/lr: inline runs top-to-bottom
            if self.wm.is_rtl {
                self.outer_size.height - logical.inline_offset - inner_size.height
            } else {
                logical.inline_offset
            }
        };
        let left = if self.wm.is_flipped_blocks {
            // vertical-rl: block-start is right edge
            self.outer_size.width - logical.block_offset - inner_size.width
        } else {
            logical.block_offset
        };
        PhysicalOffset::new(left, top)
    }

    /// Convert a physical offset to a logical offset.
    ///
    /// Requires the inner (child) physical size for the same reason as
    /// `to_physical_offset`.
    ///
    /// Blink: `WritingModeConverter::ToLogical(PhysicalOffset, PhysicalSize)`.
    pub fn to_logical_offset(
        &self,
        physical: PhysicalOffset,
        inner_size: PhysicalSize,
    ) -> LogicalOffset {
        if self.wm.is_horizontal {
            self.horizontal_to_logical_offset(physical, inner_size)
        } else {
            self.vertical_to_logical_offset(physical, inner_size)
        }
    }

    fn horizontal_to_logical_offset(
        &self,
        physical: PhysicalOffset,
        inner_size: PhysicalSize,
    ) -> LogicalOffset {
        let inline_offset = if self.wm.is_rtl {
            self.outer_size.width - physical.left - inner_size.width
        } else {
            physical.left
        };
        LogicalOffset::new(inline_offset, physical.top)
    }

    fn vertical_to_logical_offset(
        &self,
        physical: PhysicalOffset,
        inner_size: PhysicalSize,
    ) -> LogicalOffset {
        let inline_offset = if self.wm.is_flipped_lines {
            // sideways-lr: inline runs bottom-to-top
            if self.wm.is_rtl {
                physical.top
            } else {
                self.outer_size.height - physical.top - inner_size.height
            }
        } else {
            if self.wm.is_rtl {
                self.outer_size.height - physical.top - inner_size.height
            } else {
                physical.top
            }
        };
        let block_offset = if self.wm.is_flipped_blocks {
            self.outer_size.width - physical.left - inner_size.width
        } else {
            physical.left
        };
        LogicalOffset::new(inline_offset, block_offset)
    }

    // ── Rect conversions ────────────────────────────────────────────

    /// Convert a logical rect to a physical rect.
    ///
    /// Blink: `WritingModeConverter::ToPhysical(LogicalRect)`.
    pub fn to_physical_rect(&self, logical: LogicalRect) -> PhysicalRect {
        let size = self.to_physical_size(logical.size);
        let offset = self.to_physical_offset(logical.offset, size);
        PhysicalRect::new(offset, size)
    }

    /// Convert a physical rect to a logical rect.
    ///
    /// Blink: `WritingModeConverter::ToLogical(PhysicalRect)`.
    pub fn to_logical_rect(&self, physical: PhysicalRect) -> LogicalRect {
        let offset = self.to_logical_offset(physical.offset, physical.size);
        let size = self.to_logical_size(physical.size);
        LogicalRect::new(offset, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutUnit;

    fn lu(px: i32) -> LayoutUnit { LayoutUnit::from_i32(px) }

    // Shorthand constructors for writing direction modes.
    fn htb_ltr() -> WritingDirectionMode {
        WritingDirectionMode::new(true, false, false, false)
    }
    fn htb_rtl() -> WritingDirectionMode {
        WritingDirectionMode::new(true, false, false, true)
    }
    fn vrl_ltr() -> WritingDirectionMode {
        WritingDirectionMode::new(false, true, false, false)
    }
    fn vrl_rtl() -> WritingDirectionMode {
        WritingDirectionMode::new(false, true, false, true)
    }
    fn vlr_ltr() -> WritingDirectionMode {
        WritingDirectionMode::new(false, false, false, false)
    }
    fn vlr_rtl() -> WritingDirectionMode {
        WritingDirectionMode::new(false, false, false, true)
    }
    fn srl_ltr() -> WritingDirectionMode {
        // sideways-rl: same as vertical-rl (flipped blocks, no flipped lines)
        WritingDirectionMode::new(false, true, false, false)
    }
    fn slr_ltr() -> WritingDirectionMode {
        // sideways-lr: flipped lines (bottom-to-top inline), not flipped blocks
        WritingDirectionMode::new(false, false, true, false)
    }
    fn slr_rtl() -> WritingDirectionMode {
        WritingDirectionMode::new(false, false, true, true)
    }

    // ── WritingDirectionMode tests ──────────────────────────────────

    #[test]
    fn default_is_horizontal_ltr() {
        let wm = WritingDirectionMode::default();
        assert!(wm.is_horizontal());
        assert!(!wm.is_flipped_blocks());
        assert!(!wm.is_flipped_lines());
        assert!(!wm.is_rtl());
    }

    // ── Size conversion tests ───────────────────────────────────────

    #[test]
    fn size_horizontal_tb() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(htb_ltr(), outer);
        let logical = LogicalSize::new(lu(200), lu(100));
        let physical = conv.to_physical_size(logical);
        assert_eq!(physical.width, lu(200));
        assert_eq!(physical.height, lu(100));
    }

    #[test]
    fn size_vertical_rl() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vrl_ltr(), outer);
        let logical = LogicalSize::new(lu(200), lu(100));
        let physical = conv.to_physical_size(logical);
        // vertical: inline→height, block→width
        assert_eq!(physical.width, lu(100));
        assert_eq!(physical.height, lu(200));
    }

    #[test]
    fn size_vertical_lr() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vlr_ltr(), outer);
        let logical = LogicalSize::new(lu(300), lu(50));
        let physical = conv.to_physical_size(logical);
        assert_eq!(physical.width, lu(50));
        assert_eq!(physical.height, lu(300));
    }

    #[test]
    fn size_round_trip_horizontal() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(htb_ltr(), outer);
        let original = LogicalSize::new(lu(123), lu(456));
        let physical = conv.to_physical_size(original);
        let back = conv.to_logical_size(physical);
        assert_eq!(back, original);
    }

    #[test]
    fn size_round_trip_vertical_rl() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vrl_ltr(), outer);
        let original = LogicalSize::new(lu(123), lu(456));
        let physical = conv.to_physical_size(original);
        let back = conv.to_logical_size(physical);
        assert_eq!(back, original);
    }

    #[test]
    fn size_round_trip_vertical_lr() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vlr_ltr(), outer);
        let original = LogicalSize::new(lu(77), lu(33));
        let physical = conv.to_physical_size(original);
        let back = conv.to_logical_size(physical);
        assert_eq!(back, original);
    }

    // ── Offset conversion tests ─────────────────────────────────────

    #[test]
    fn offset_htb_ltr() {
        // horizontal-tb + LTR: inline→left, block→top (identity)
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(htb_ltr(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        assert_eq!(physical.left, lu(10));
        assert_eq!(physical.top, lu(20));
    }

    #[test]
    fn offset_htb_rtl() {
        // horizontal-tb + RTL: inline-start is right edge
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(htb_rtl(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        // left = 800 - 10 - 100 = 690
        assert_eq!(physical.left, lu(690));
        assert_eq!(physical.top, lu(20));
    }

    #[test]
    fn offset_vrl_ltr() {
        // vertical-rl + LTR: inline→top, block→right-to-left
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vrl_ltr(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        // top = 10 (inline, LTR, top-to-bottom)
        assert_eq!(physical.top, lu(10));
        // left = 800 - 20 - 100 = 680 (flipped blocks)
        assert_eq!(physical.left, lu(680));
    }

    #[test]
    fn offset_vrl_rtl() {
        // vertical-rl + RTL: inline runs bottom-to-top
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vrl_rtl(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        // top = 600 - 10 - 50 = 540 (RTL flips inline in vertical)
        assert_eq!(physical.top, lu(540));
        // left = 800 - 20 - 100 = 680 (flipped blocks)
        assert_eq!(physical.left, lu(680));
    }

    #[test]
    fn offset_vlr_ltr() {
        // vertical-lr + LTR: inline→top, block→left
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vlr_ltr(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        assert_eq!(physical.top, lu(10));
        assert_eq!(physical.left, lu(20));
    }

    #[test]
    fn offset_vlr_rtl() {
        // vertical-lr + RTL: inline runs bottom-to-top
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vlr_rtl(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        // top = 600 - 10 - 50 = 540
        assert_eq!(physical.top, lu(540));
        assert_eq!(physical.left, lu(20));
    }

    #[test]
    fn offset_slr_ltr() {
        // sideways-lr + LTR: inline runs bottom-to-top (flipped lines)
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(slr_ltr(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        // top = 600 - 10 - 50 = 540 (flipped lines, LTR)
        assert_eq!(physical.top, lu(540));
        // left = 20 (not flipped blocks)
        assert_eq!(physical.left, lu(20));
    }

    #[test]
    fn offset_slr_rtl() {
        // sideways-lr + RTL: inline top-to-bottom (flipped-lines + RTL cancel)
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(slr_rtl(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let logical = LogicalOffset::new(lu(10), lu(20));
        let physical = conv.to_physical_offset(logical, inner);
        // top = 10 (flipped lines + RTL → normal direction)
        assert_eq!(physical.top, lu(10));
        assert_eq!(physical.left, lu(20));
    }

    // ── Offset round-trip tests ─────────────────────────────────────

    fn assert_offset_round_trip(wm: WritingDirectionMode, outer: PhysicalSize, inner: PhysicalSize) {
        let conv = WritingModeConverter::new(wm, outer);
        let original = LogicalOffset::new(lu(17), lu(29));
        let physical = conv.to_physical_offset(original, inner);
        let back = conv.to_logical_offset(physical, inner);
        assert_eq!(back, original, "round-trip failed for {:?}", wm);
    }

    #[test]
    fn offset_round_trip_htb_ltr() {
        assert_offset_round_trip(htb_ltr(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_htb_rtl() {
        assert_offset_round_trip(htb_rtl(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_vrl_ltr() {
        assert_offset_round_trip(vrl_ltr(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_vrl_rtl() {
        assert_offset_round_trip(vrl_rtl(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_vlr_ltr() {
        assert_offset_round_trip(vlr_ltr(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_vlr_rtl() {
        assert_offset_round_trip(vlr_rtl(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_slr_ltr() {
        assert_offset_round_trip(slr_ltr(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_slr_rtl() {
        assert_offset_round_trip(slr_rtl(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    #[test]
    fn offset_round_trip_srl_ltr() {
        assert_offset_round_trip(srl_ltr(), PhysicalSize::new(lu(800), lu(600)), PhysicalSize::new(lu(100), lu(50)));
    }

    // ── Rect conversion tests ───────────────────────────────────────

    #[test]
    fn rect_htb_ltr() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(htb_ltr(), outer);
        let logical = LogicalRect::from_position_and_size(lu(10), lu(20), lu(100), lu(50));
        let physical = conv.to_physical_rect(logical);
        assert_eq!(physical.offset.left, lu(10));
        assert_eq!(physical.offset.top, lu(20));
        assert_eq!(physical.size.width, lu(100));
        assert_eq!(physical.size.height, lu(50));
    }

    #[test]
    fn rect_htb_rtl() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(htb_rtl(), outer);
        let logical = LogicalRect::from_position_and_size(lu(10), lu(20), lu(100), lu(50));
        let physical = conv.to_physical_rect(logical);
        assert_eq!(physical.offset.left, lu(690)); // 800 - 10 - 100
        assert_eq!(physical.offset.top, lu(20));
        assert_eq!(physical.size.width, lu(100));
        assert_eq!(physical.size.height, lu(50));
    }

    #[test]
    fn rect_vrl_ltr() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vrl_ltr(), outer);
        let logical = LogicalRect::from_position_and_size(lu(10), lu(20), lu(300), lu(100));
        let physical = conv.to_physical_rect(logical);
        // size: width=block=100, height=inline=300
        assert_eq!(physical.size.width, lu(100));
        assert_eq!(physical.size.height, lu(300));
        // offset: top=10, left=800-20-100=680
        assert_eq!(physical.offset.top, lu(10));
        assert_eq!(physical.offset.left, lu(680));
    }

    #[test]
    fn rect_vlr_ltr() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vlr_ltr(), outer);
        let logical = LogicalRect::from_position_and_size(lu(10), lu(20), lu(300), lu(100));
        let physical = conv.to_physical_rect(logical);
        assert_eq!(physical.size.width, lu(100));
        assert_eq!(physical.size.height, lu(300));
        assert_eq!(physical.offset.top, lu(10));
        assert_eq!(physical.offset.left, lu(20));
    }

    fn assert_rect_round_trip(wm: WritingDirectionMode, outer: PhysicalSize) {
        let conv = WritingModeConverter::new(wm, outer);
        let original = LogicalRect::from_position_and_size(lu(11), lu(22), lu(100), lu(50));
        let physical = conv.to_physical_rect(original);
        let back = conv.to_logical_rect(physical);
        assert_eq!(back, original, "rect round-trip failed for {:?}", wm);
    }

    #[test]
    fn rect_round_trip_htb_ltr() {
        assert_rect_round_trip(htb_ltr(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_htb_rtl() {
        assert_rect_round_trip(htb_rtl(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_vrl_ltr() {
        assert_rect_round_trip(vrl_ltr(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_vrl_rtl() {
        assert_rect_round_trip(vrl_rtl(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_vlr_ltr() {
        assert_rect_round_trip(vlr_ltr(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_vlr_rtl() {
        assert_rect_round_trip(vlr_rtl(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_slr_ltr() {
        assert_rect_round_trip(slr_ltr(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_slr_rtl() {
        assert_rect_round_trip(slr_rtl(), PhysicalSize::new(lu(800), lu(600)));
    }

    #[test]
    fn rect_round_trip_srl_ltr() {
        assert_rect_round_trip(srl_ltr(), PhysicalSize::new(lu(800), lu(600)));
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn zero_size_offset_round_trip() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let inner = PhysicalSize::zero();
        for wm in [htb_ltr(), htb_rtl(), vrl_ltr(), vrl_rtl(), vlr_ltr(), vlr_rtl(), slr_ltr(), slr_rtl()] {
            let conv = WritingModeConverter::new(wm, outer);
            let original = LogicalOffset::new(lu(50), lu(30));
            let physical = conv.to_physical_offset(original, inner);
            let back = conv.to_logical_offset(physical, inner);
            assert_eq!(back, original, "zero inner size round-trip failed for {:?}", wm);
        }
    }

    #[test]
    fn zero_offset_round_trip() {
        let outer = PhysicalSize::new(lu(800), lu(600));
        let inner = PhysicalSize::new(lu(100), lu(50));
        for wm in [htb_ltr(), htb_rtl(), vrl_ltr(), vrl_rtl(), vlr_ltr(), vlr_rtl(), slr_ltr(), slr_rtl()] {
            let conv = WritingModeConverter::new(wm, outer);
            let original = LogicalOffset::zero();
            let physical = conv.to_physical_offset(original, inner);
            let back = conv.to_logical_offset(physical, inner);
            assert_eq!(back, original, "zero offset round-trip failed for {:?}", wm);
        }
    }

    #[test]
    fn full_size_child_offset_round_trip() {
        // Child fills the entire container
        let outer = PhysicalSize::new(lu(800), lu(600));
        for wm in [htb_ltr(), htb_rtl(), vrl_ltr(), vrl_rtl(), vlr_ltr(), vlr_rtl(), slr_ltr(), slr_rtl()] {
            let conv = WritingModeConverter::new(wm, outer);
            let inner_physical = outer;
            let original = LogicalOffset::zero();
            let physical = conv.to_physical_offset(original, inner_physical);
            let back = conv.to_logical_offset(physical, inner_physical);
            assert_eq!(back, original, "full-size child round-trip failed for {:?}", wm);
        }
    }

    #[test]
    fn physical_to_logical_offset_vrl_ltr_specific() {
        // Verify the formula from the task description:
        // vertical-rl + LTR: inline_offset = top, block_offset = outer_width - left - inner_width
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vrl_ltr(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let physical = PhysicalOffset::new(lu(200), lu(30));
        let logical = conv.to_logical_offset(physical, inner);
        assert_eq!(logical.inline_offset, lu(30));          // top
        assert_eq!(logical.block_offset, lu(500));           // 800 - 200 - 100
    }

    #[test]
    fn physical_to_logical_offset_vlr_ltr_specific() {
        // Verify: vertical-lr + LTR: inline_offset = top, block_offset = left
        let outer = PhysicalSize::new(lu(800), lu(600));
        let conv = WritingModeConverter::new(vlr_ltr(), outer);
        let inner = PhysicalSize::new(lu(100), lu(50));
        let physical = PhysicalOffset::new(lu(200), lu(30));
        let logical = conv.to_logical_offset(physical, inner);
        assert_eq!(logical.inline_offset, lu(30));          // top
        assert_eq!(logical.block_offset, lu(200));           // left
    }
}
