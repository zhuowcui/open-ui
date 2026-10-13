//! Fragment-to-Skia paint pipeline — extracted from Blink's paint system.
//!
//! Source: core/paint/box_fragment_painter.cc, box_painter_base.cc,
//!         core/paint/text_painter.cc, core/paint/text_decoration_painter.cc
//!
//! This renders layout fragments to a Skia canvas using the exact same
//! draw calls, paint flags, and coordinate handling as Blink.

mod collapsed_table_painter;
mod content_layer;
pub mod decoration_painter;
pub mod emphasis_painter;
pub mod image_resource;
mod layered_program;
pub mod paint_order;
mod paint_program;
mod paint_record;
mod painter;
mod render;
pub mod text_painter;

pub use content_layer::RecordedContentLayer;
pub use layered_program::*;
pub use paint_program::{
    IntrinsicPictureStatus, PaintCaptureIncompleteReason, PaintCaptureStatus, PaintLayoutScopeRole,
    PaintOwnerPhase, PaintProgramChunk, PaintProgramOwner, PaintShaderDependency,
    RecordedPaintProgram,
};
pub use painter::paint_fragment;
pub use painter::{
    compute_clip_rect, compute_overflow_clip_geometry, has_overflow_border_radius,
    overflow_border_radius_reference_box, OverflowClipContour, OverflowClipContourPurpose,
    OverflowClipGeometry,
};
pub use render::{
    raster_surface_properties, rasterize_picture, record_document, record_fragment,
    record_fragment_with_program, render_to_png, render_to_surface, LayeredPaintCapture,
    RecordedPaintCapture, RecordedPicture,
};
