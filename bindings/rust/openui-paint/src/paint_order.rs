//! Ordinary child phases shared by fragment painting and input ordering.

use openui_style::{ComputedStyle, Float, Position, Transform2D};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InFlowPaintPhase {
    BlockBackground,
    Float,
    Atomic,
    StackingContext,
}

pub fn in_flow_paint_phase(style: &ComputedStyle) -> InFlowPaintPhase {
    if style.position != Position::Static
        || style.opacity < 1.0
        || style.transform != Transform2D::IDENTITY
        || style.filter_blur > 0.0
        || style.filter_grayscale > 0.0
        || style.clip_path_inset.is_some()
        || !style.mask_layers.is_empty()
        || style.has_paint_containment()
    {
        InFlowPaintPhase::StackingContext
    } else if style.float != Float::None {
        InFlowPaintPhase::Float
    } else if style.display.is_inline_level() || style.display.is_flex() || style.display.is_grid()
    {
        InFlowPaintPhase::Atomic
    } else {
        InFlowPaintPhase::BlockBackground
    }
}
