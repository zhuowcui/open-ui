//! Typed CSS style properties — extracted from Blink's `ComputedStyle`.
//!
//! Properties are set programmatically through typed Rust APIs. Native literal
//! helpers admit documented value subsets; they do not parse stylesheets.
//! Every enum, every initial value, every type is extracted character-by-character
//! from Blink's generated `computed_style_base.h` and `computed_style_constants.h`.

mod animation;
mod animation_value;
mod color;
mod computed;
mod enums;
mod font_types;
mod layout_systems;
mod property;
mod transform_composition;
mod transform_literal;
mod typography;

pub use animation::*;
pub use animation_value::*;
pub use color::{Color, StyleColor};
pub use computed::{
    AspectRatio, BackgroundLayer, BackgroundPosition, BackgroundRepeat, BackgroundSize,
    BlockEllipsis, BorderImage, BorderImageLength, BorderImageRepeat, BoxShadow, ComputedStyle,
    ComputedStyleBuilder, ComputedStyleFields, ConicGradient, CounterOperation, CounterStyle,
    CssImage, CssLinearGradient, GeneratedContentItem, GradientColorSpace, GradientStop,
    GradientStopPosition, ImageResourceId, LineClamp, LinearGradient, LinearGradientStop,
    PositionArea, QuotePair, RadialGradient, RadialGradientShape, RadialGradientSize,
    WebkitBoxOrient,
};
pub use enums::*;
pub use font_types::*;
pub use layout_systems::*;
pub use openui_geometry::Length;
pub use property::*;
pub use transform_composition::*;
pub use typography::*;
