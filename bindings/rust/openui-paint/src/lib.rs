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
mod source_layer_program;
mod surface_program;
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
    RecordedPaintCapture, RecordedPicture, SourceLayerPaintCapture, SourceRecordingUnavailable,
};
pub use source_layer_program::*;
pub use surface_program::*;

/// Installs Skia's CPU-specific raster stages, as Chromium does at startup
/// through `SkGraphics::Init`. Without it Skia keeps its baseline SSE
/// stages, while Chromium runs the AVX2 and FMA stages on a CPU that has
/// them, and their rounding differs. Painting and rasterizing call this
/// before drawing, and it is safe to call again.
pub fn initialize_skia() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(skia_safe::graphics::init);
}

// The unit tests draw through internal painters as well as `paint_fragment`,
// so this binary initializes Skia when it loads, before any test runs, and no
// result depends on test order.
#[cfg(test)]
#[used]
#[cfg_attr(target_os = "linux", link_section = ".init_array")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
static INITIALIZE_SKIA_FOR_TESTS: extern "C" fn() = {
    extern "C" fn initialize() {
        initialize_skia();
    }
    initialize
};
