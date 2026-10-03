use openui_dom::{Document, ElementTag};
use openui_geometry::ViewportMetrics;
use openui_paint::{rasterize_picture, record_document, RecordedPicture};
use openui_style::{Overflow, StyleProperty};
use std::sync::Arc;

fn recording(scrolling: bool) -> RecordedPicture {
    let mut document = Document::new();
    let collection = Arc::downgrade(document.font_collection());
    let root = document.root();
    if scrolling {
        document
            .apply_style_property(
                root,
                StyleProperty::OverflowY,
                &Overflow::Scroll.into(),
                (120.0, 48.0),
            )
            .unwrap();
    }
    let text = document.create_node(ElementTag::Text);
    document.node_mut(text).text = Some("Native text remains owned by the recording".into());
    document.append_child(root, text);
    let viewport = ViewportMetrics::from_logical_size(120.0, 48.0, 1.25).unwrap();
    let (fragment, picture) = record_document(&document, viewport).unwrap();
    drop(fragment);
    drop(document);
    // A recording keeps its cache lifetime, not mutable registry/DOM state.
    assert!(collection.upgrade().is_none());
    picture
}

fn pixels(picture: &RecordedPicture) -> Vec<u8> {
    let mut surface = rasterize_picture(picture).unwrap();
    let image = surface.image_snapshot();
    let info = image
        .image_info()
        .with_color_type(skia_safe::ColorType::RGBA8888);
    let mut pixels = vec![0; info.compute_min_byte_size()];
    assert!(image.read_pixels(
        &info,
        &mut pixels,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    ));
    pixels
}

// Isolated process: the shared unit-test harness has unrelated live documents.
#[test]
fn retained_recordings_own_font_lifetime_through_replay_and_layer_clones() {
    assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);
    let picture = recording(false);
    let expected = pixels(&picture);
    let used = skia_safe::graphics::font_cache_count_used();
    assert!(used > 0);
    let clone = picture.clone();
    drop(picture);
    assert_eq!(skia_safe::graphics::font_cache_count_used(), used);
    let replayed = std::thread::spawn(move || {
        let actual = pixels(&clone);
        assert!(skia_safe::graphics::font_cache_count_used() > 0);
        actual
    })
    .join()
    .unwrap();
    assert_eq!(replayed, expected);
    assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);

    let scrolling = recording(true);
    let layer = scrolling
        .viewport_content_layer()
        .expect("scrolling viewport retains a content layer")
        .clone();
    drop(scrolling);
    assert!(skia_safe::graphics::font_cache_count_used() > 0);
    // Dropping unrelated recordings must not evict a surviving layer's cache.
    let temporary = recording(false);
    drop(temporary);
    assert!(skia_safe::graphics::font_cache_count_used() > 0);
    std::thread::spawn(move || {
        let mut surface = skia_safe::surfaces::raster_n32_premul((150, 60)).unwrap();
        surface.canvas().draw_picture(&layer.picture, None, None);
        assert!(skia_safe::graphics::font_cache_count_used() > 0);
        drop(layer);
    })
    .join()
    .unwrap();
    assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);
}
