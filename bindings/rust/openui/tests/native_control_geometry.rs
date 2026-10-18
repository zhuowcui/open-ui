use openui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

fn control(document: &Document, tag: &str) -> Element {
    let element = Element::create(document, tag).unwrap();
    element.set_display(Display::Block).unwrap();
    element.set_position(Position::Absolute).unwrap();
    element.set_left(Length::px(20.0)).unwrap();
    element.set_top(Length::px(20.0)).unwrap();
    element
        .set_font_family(FontFamilyList::single("Ahem"))
        .unwrap();
    element.set_font_size(LengthValue::px(16.0)).unwrap();
    document.body().append_child(&element).unwrap();
    element
}

fn size(element: &Element) -> (f32, f32) {
    let bounds = element.bounding_rect().unwrap().unwrap();
    (bounds.width, bounds.height)
}

fn document(scale: f64) -> Document {
    Document::with_font_collection(
        ViewportMetrics::from_logical_size(640.0, 360.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap()
}

#[test]
fn native_size_attribute_alone_relayouts_after_rust_callback_and_removal() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = document(scale);
        let input = control(&document, "input");
        assert_eq!(size(&input), (320.0, 16.0));
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let weak = input.downgrade();
        document
            .body()
            .on("click", move |_| {
                weak.upgrade().unwrap().set_attribute("size", "7").unwrap();
                observed.set(observed.get() + 1);
            })
            .unwrap();
        document.body().click().unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(size(&input), (112.0, 16.0));
        input.remove_attribute("size").unwrap();
        assert_eq!(size(&input), (320.0, 16.0));
        for (attribute, width) in [
            (" \t+3suffix", 48.0),
            ("0", 320.0),
            ("-3", 320.0),
            ("word", 320.0),
            ("2147483648", 320.0),
        ] {
            input.set_attribute("size", attribute).unwrap();
            assert_eq!(
                size(&input),
                (width, 16.0),
                "size={attribute:?}, scale={scale}"
            );
        }
    }
}

#[test]
fn native_textarea_rows_cols_and_vertical_axis_mutations_relayout() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = document(scale);
        let textarea = control(&document, "textarea");
        assert_eq!(size(&textarea), (335.0, 32.0));
        textarea.set_attribute("cols", "9").unwrap();
        assert_eq!(size(&textarea), (159.0, 32.0));
        textarea.set_attribute("rows", "3").unwrap();
        assert_eq!(size(&textarea), (159.0, 48.0));
        textarea.set_writing_mode(WritingMode::VerticalRl).unwrap();
        assert_eq!(size(&textarea), (48.0, 159.0));
        textarea.remove_attribute("cols").unwrap();
        textarea.remove_attribute("rows").unwrap();
        assert_eq!(size(&textarea), (32.0, 335.0));
    }
}

#[test]
fn cloned_native_controls_recompute_inherited_font_geometry_independently() {
    let document = document(1.0);
    document
        .body()
        .set_font_size(LengthValue::px(16.0))
        .unwrap();
    let original = Element::create(&document, "input").unwrap();
    original.set_display(Display::Block).unwrap();
    original
        .set_font_family(FontFamilyList::single("Ahem"))
        .unwrap();
    document.body().append_child(&original).unwrap();
    original.set_attribute("size", "7").unwrap();
    assert_eq!(size(&original), (112.0, 16.0));
    let clone = original.clone_subtree().unwrap();
    document.body().append_child(&clone).unwrap();
    document
        .body()
        .set_font_size(LengthValue::px(20.0))
        .unwrap();
    assert_eq!(size(&original), (140.0, 20.0));
    assert_eq!(size(&clone), (140.0, 20.0));
    clone.set_attribute("size", "3").unwrap();
    assert_eq!(size(&clone), (60.0, 20.0));
    assert_eq!(size(&original), (140.0, 20.0));
}

#[test]
fn authored_native_control_dimensions_survive_intrinsic_attribute_changes() {
    let document = document(1.0);
    for tag in ["input", "textarea"] {
        let element = control(&document, tag);
        element.set_width(LengthValue::px(80.0)).unwrap();
        element.set_height(LengthValue::px(30.0)).unwrap();
        assert_eq!(size(&element), (80.0, 30.0));
        element.set_attribute("size", "7").unwrap();
        element.set_attribute("cols", "9").unwrap();
        element.set_attribute("rows", "3").unwrap();
        element.set_font_size(LengthValue::px(20.0)).unwrap();
        assert_eq!(size(&element), (80.0, 30.0));
    }
}

#[test]
fn native_textarea_resize_style_controls_grip_paint_and_round_trip() {
    let document = document(1.0);
    document.body().set_background_color(Color::WHITE).unwrap();
    let textarea = control(&document, "textarea");
    textarea.set_background_color(Color::GREEN).unwrap();
    let bounds = size(&textarea);
    let before = document.render_to_png_buffer().unwrap();
    assert_eq!(textarea.computed_style().unwrap().resize, Resize::None);
    textarea.set_resize(Resize::Both).unwrap();
    assert_eq!(textarea.computed_style().unwrap().resize, Resize::Both);
    assert_eq!(size(&textarea), bounds);
    assert_ne!(document.render_to_png_buffer().unwrap(), before);
    textarea.set_resize(Resize::None).unwrap();
    assert_eq!(document.render_to_png_buffer().unwrap(), before);
}
