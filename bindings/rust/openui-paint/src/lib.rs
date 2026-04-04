//! Fragment-to-Skia paint pipeline — extracted from Blink's paint system.
//!
//! Source: core/paint/box_fragment_painter.cc, box_painter_base.cc,
//!         core/paint/text_painter.cc, core/paint/text_decoration_painter.cc
//!
//! This renders layout fragments to a Skia canvas using the exact same
//! draw calls, paint flags, and coordinate handling as Blink.

mod painter;
pub mod text_painter;
pub mod decoration_painter;
mod render;

pub use painter::paint_fragment;
pub use render::{render_to_png, render_to_surface};
