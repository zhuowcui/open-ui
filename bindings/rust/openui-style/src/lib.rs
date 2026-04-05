//! Typed CSS style properties — extracted from Blink's `ComputedStyle`.
//!
//! No CSS parsing. Properties are set programmatically via typed Rust API.
//! Every enum, every initial value, every type is extracted character-by-character
//! from Blink's generated `computed_style_base.h` and `computed_style_constants.h`.

mod color;
mod computed;
mod enums;
mod font_types;

pub use color::{Color, StyleColor};
pub use computed::{ComputedStyle, AspectRatio};
pub use enums::*;
pub use font_types::*;

