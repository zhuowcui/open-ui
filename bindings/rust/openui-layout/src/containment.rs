//! Shared CSS containment sizing helpers.

use openui_geometry::LayoutUnit;
use openui_style::{ComputedStyle, ContainIntrinsicLength};

use crate::length_resolver::resolve_length;

fn resolve_fallback(value: &ContainIntrinsicLength) -> LayoutUnit {
    value
        .fallback
        .as_ref()
        .map_or(LayoutUnit::zero(), |length| {
            resolve_length(
                length,
                openui_geometry::INDEFINITE_SIZE,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            )
            .clamp_negative_to_zero()
        })
}

pub(crate) fn physical_width_fallback(style: &ComputedStyle) -> LayoutUnit {
    resolve_fallback(&style.contain_intrinsic_width)
}

pub(crate) fn physical_height_fallback(style: &ComputedStyle) -> LayoutUnit {
    resolve_fallback(&style.contain_intrinsic_height)
}

pub(crate) fn physical_width_is_contained(style: &ComputedStyle) -> bool {
    style.has_block_size_containment()
        || (style.has_inline_size_containment() && style.writing_mode.is_horizontal())
}

pub(crate) fn physical_height_is_contained(style: &ComputedStyle) -> bool {
    style.has_block_size_containment()
        || (style.has_inline_size_containment() && !style.writing_mode.is_horizontal())
}

pub(crate) fn logical_block_fallback(style: &ComputedStyle) -> Option<LayoutUnit> {
    if style.has_block_size_containment() {
        Some(if style.writing_mode.is_horizontal() {
            physical_height_fallback(style)
        } else {
            physical_width_fallback(style)
        })
    } else {
        None
    }
}

pub(crate) fn logical_inline_fallback(style: &ComputedStyle) -> Option<LayoutUnit> {
    if style.has_inline_size_containment() {
        Some(if style.writing_mode.is_horizontal() {
            physical_width_fallback(style)
        } else {
            physical_height_fallback(style)
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::Length;
    use openui_style::{ContainerType, Containment, WritingMode};

    #[test]
    fn size_containment_uses_physical_fallbacks() {
        let mut style = ComputedStyle::initial();
        style.contain = Containment::SIZE;
        style.contain_intrinsic_width = ContainIntrinsicLength::length(Length::px(111.0));
        style.contain_intrinsic_height = ContainIntrinsicLength::length(Length::px(222.0));
        assert_eq!(
            logical_inline_fallback(&style),
            Some(LayoutUnit::from_i32(111))
        );
        assert_eq!(
            logical_block_fallback(&style),
            Some(LayoutUnit::from_i32(222))
        );

        style.writing_mode = WritingMode::VerticalRl;
        assert_eq!(
            logical_inline_fallback(&style),
            Some(LayoutUnit::from_i32(222))
        );
        assert_eq!(
            logical_block_fallback(&style),
            Some(LayoutUnit::from_i32(111))
        );
    }

    #[test]
    fn inline_size_container_substitutes_only_its_inline_axis() {
        let mut style = ComputedStyle::initial();
        style.container_type = ContainerType::InlineSize;
        style.contain_intrinsic_width = ContainIntrinsicLength::length(Length::px(42.0));
        assert_eq!(
            logical_inline_fallback(&style),
            Some(LayoutUnit::from_i32(42))
        );
        assert_eq!(logical_block_fallback(&style), None);
    }
}
