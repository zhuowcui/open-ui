//! Source-backed pure raster geometry; actual integration/pixels are separate.
use openui_paint::{opaque_raster_clear_rects, SourceLayerError};
use skia_safe::{IRect, Rect};

#[test]
fn opaque_raster_integer_translation_scale_and_external_texels_are_separate() {
    let full = IRect::from_wh(4, 3);
    let exact =
        opaque_raster_clear_rects((4.0, 3.0), (1.0, 1.0), (0.0, 0.0), full, (0, 0)).unwrap();
    assert!(!exact.clear_required);
    assert_eq!(exact.outer, Rect::from_xywh(0.0, 0.0, 4.0, 3.0));
    for scale in [(1.25, 1.0), (1.0, 1.5), (1.75, 2.0)] {
        let r = opaque_raster_clear_rects((4.0, 3.0), scale, (0.25, 0.5), full, (2, 1)).unwrap();
        assert!(r.clear_required);
        assert_eq!(
            r.outer,
            Rect::from_ltrb(
                -2.0,
                -1.0,
                2.0 + if scale.0 != 1.0 { 1.0 } else { 0.0 },
                2.0 + if scale.1 != 1.0 { 1.0 } else { 0.0 }
            )
        );
        assert_eq!(
            r.inner,
            Rect::from_ltrb(
                -1.0,
                0.0,
                2.0 - if scale.0 != 1.0 { 1.0 } else { 0.0 },
                2.0 - if scale.1 != 1.0 { 1.0 } else { 0.0 }
            )
        );
        assert_eq!(r.visible, Rect::from_ltrb(-2.0, -1.0, 2.0, 2.0));
    }
}
#[test]
fn partial_playback_and_tiny_content_follow_clear_for_opaque_raster() {
    let inner = opaque_raster_clear_rects(
        (20.0, 20.0),
        (1.25, 1.5),
        (0.0, 0.0),
        IRect::from_xywh(3, 3, 4, 4),
        (0, 0),
    )
    .unwrap();
    assert!(!inner.clear_required);
    let edge = opaque_raster_clear_rects(
        (20.0, 20.0),
        (1.25, 1.5),
        (0.0, 0.0),
        IRect::from_xywh(18, 18, 2, 2),
        (0, 0),
    )
    .unwrap();
    assert!(edge.clear_required);
    assert_eq!(edge.outer, Rect::from_ltrb(18.0, 18.0, 21.0, 21.0));
    assert_eq!(edge.visible, Rect::from_ltrb(18.0, 18.0, 20.0, 20.0));
    let tiny = opaque_raster_clear_rects(
        (1.0, 1.0),
        (1.25, 1.5),
        (0.25, 0.5),
        IRect::from_wh(1, 1),
        (0, 0),
    )
    .unwrap();
    assert!(tiny.clear_required);
    assert!(tiny.inner.is_empty());
    for (size, scale, offset) in [
        ((0.0, 1.0), (1.0, 1.0), (0.0, 0.0)),
        ((1.5, 2.0), (1.0, 1.0), (0.0, 0.0)),
        ((2.0, 2.0), (f32::NAN, 1.0), (0.0, 0.0)),
        ((2.0, 2.0), (1.0, 1.0), (1.0, 0.0)),
    ] {
        assert_eq!(
            opaque_raster_clear_rects(size, scale, offset, IRect::from_wh(2, 2), (0, 0)),
            Err(SourceLayerError::InvalidBounds)
        );
    }
}
