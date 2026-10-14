//! Descendant table backgrounds retain their owned overflow clip in both
//! background and atomic paint phases. Native equality is a phase neighbor;
//! only the transparent-cell corner/center rule has a saved stock .50 witness.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, FontCollection, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    BorderCollapse, BoxSizing, Color, CornerRadii, Display, Edges, LengthValue, Overflow, Position,
    RendererStyleValue, StyleProperty, StyleValue, TableLayout,
};

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: impl Into<StyleValue>) {
    e.set_property(n, p, v.into()).unwrap();
}
fn box_(e: &mut Engine, parent: NodeHandle, tag: ElementTag, w: f32, h: f32) -> NodeHandle {
    let n = e.create_native_element(tag).unwrap();
    set(e, n, StyleProperty::Display, Display::Block);
    set(e, n, StyleProperty::Width, LengthValue::px(w));
    set(e, n, StyleProperty::Height, LengthValue::px(h));
    set(
        e,
        n,
        StyleProperty::BoxSizing,
        StyleValue::Renderer(RendererStyleValue::BoxSizing(BoxSizing::BorderBox)),
    );
    e.append_child(parent, n).unwrap();
    n
}
fn table(scale: f64, collapse: BorderCollapse) -> (Engine, NodeHandle) {
    let mut e = Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(240.0, 180.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    let root = e.root();
    set(&mut e, root, StyleProperty::BackgroundColor, Color::WHITE);
    let table = box_(&mut e, root, ElementTag::Table, 100.0, 60.0);
    set(&mut e, table, StyleProperty::Display, Display::Table);
    set(
        &mut e,
        table,
        StyleProperty::BorderCollapse,
        StyleValue::Renderer(RendererStyleValue::BorderCollapse(collapse)),
    );
    set(
        &mut e,
        table,
        StyleProperty::TableLayout,
        StyleValue::Renderer(RendererStyleValue::TableLayout(TableLayout::Fixed)),
    );
    set(
        &mut e,
        table,
        StyleProperty::BorderSpacing,
        StyleValue::Renderer(RendererStyleValue::BorderSpacing((
            Length::px(0.0),
            Length::px(0.0),
        ))),
    );
    let row = box_(&mut e, table, ElementTag::TableRow, 100.0, 60.0);
    set(&mut e, row, StyleProperty::Display, Display::TableRow);
    let cell = box_(&mut e, row, ElementTag::TableCell, 100.0, 60.0);
    set(&mut e, cell, StyleProperty::Display, Display::TableCell);
    set(&mut e, cell, StyleProperty::Overflow, Overflow::Hidden);
    set(
        &mut e,
        cell,
        StyleProperty::BorderRadius,
        CornerRadii(Edges::all(LengthValue::px(20.0))),
    );
    (e, cell)
}
fn frame(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(scene).unwrap()
}
fn pixel(frame: &Frame, scale: f64, point: (f64, f64)) -> [u8; 4] {
    let x = (point.0 * scale).round() as usize;
    let y = (point.1 * scale).round() as usize;
    let start = y * frame.stride + x * 4;
    frame.pixels[start..start + 4].try_into().unwrap()
}
fn points(e: &mut Engine, cell: NodeHandle) -> ((f64, f64), (f64, f64)) {
    let bounds = e.bounds(cell).unwrap().unwrap();
    (
        (f64::from(bounds.x) + 1.0, f64::from(bounds.y) + 1.0),
        (
            f64::from(bounds.x) + f64::from(bounds.width) * 0.5,
            f64::from(bounds.y) + f64::from(bounds.height) * 0.5,
        ),
    )
}

#[test]
fn descendant_background_phase_matches_atomic_cell_and_retains_own_decoration() {
    for scale in SCALES {
        for collapse in [BorderCollapse::Separate, BorderCollapse::Collapse] {
            let (mut e, cell) = table(scale, collapse);
            let child = box_(&mut e, cell, ElementTag::Div, 100.0, 60.0);
            set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
            let background_phase = e.scene().unwrap();
            let (corner, center) = points(&mut e, cell);
            let original = frame(&background_phase);
            assert_eq!(
                pixel(&original, scale, corner),
                [255; 4],
                "{scale}/{collapse:?}"
            );
            assert_eq!(pixel(&original, scale, center), [0, 128, 0, 255]);
            assert_ne!(
                e.hit_test(corner.0 as f32, corner.1 as f32).unwrap(),
                Some(child)
            );
            assert_eq!(
                e.hit_test(center.0 as f32, center.1 as f32).unwrap(),
                Some(child)
            );

            // Position:relative creates the part's own atomic layer without
            // moving it. It must not change the final clipping/paint result.
            set(&mut e, cell, StyleProperty::Position, Position::Relative);
            let atomic = e.scene().unwrap();
            assert_eq!(
                frame(&atomic).pixels,
                original.pixels,
                "{scale}/{collapse:?}"
            );
            set(&mut e, cell, StyleProperty::Position, Position::Static);
            set(&mut e, cell, StyleProperty::BackgroundColor, Color::BLUE);
            let own_decoration = e.scene().unwrap();
            let decorated = frame(&own_decoration);
            // Compare actual phases, including the cell's own decoration,
            // rather than assuming collapsed and separate decorations match.
            set(&mut e, cell, StyleProperty::Position, Position::Relative);
            let own_atomic = e.scene().unwrap();
            assert_eq!(
                frame(&own_atomic).pixels,
                decorated.pixels,
                "{scale}/{collapse:?}"
            );
            assert_eq!(pixel(&decorated, scale, center), [0, 128, 0, 255]);
            set(&mut e, cell, StyleProperty::Position, Position::Static);
            set(
                &mut e,
                cell,
                StyleProperty::BackgroundColor,
                Color::TRANSPARENT,
            );
            set(
                &mut e,
                cell,
                StyleProperty::BorderRadius,
                CornerRadii(Edges::all(LengthValue::px(0.0))),
            );
            let rectangular = e.scene().unwrap();
            assert_eq!(pixel(&frame(&rectangular), scale, corner), [0, 128, 0, 255]);
            set(
                &mut e,
                cell,
                StyleProperty::BorderRadius,
                CornerRadii(Edges::all(LengthValue::px(20.0))),
            );
            let restored = e.scene().unwrap();
            assert_eq!(frame(&restored).pixels, original.pixels);
            let stats = e.stats();
            assert_eq!(e.scene().unwrap().generation(), restored.generation());
            assert_eq!(e.stats(), stats);
            drop(e);
            assert_eq!(frame(&background_phase).pixels, original.pixels);
        }
    }
}

#[test]
fn rounded_background_mask_stays_at_the_cell_while_real_contents_scroll() {
    for scale in SCALES {
        for collapse in [BorderCollapse::Separate, BorderCollapse::Collapse] {
            let (mut e, cell) = table(scale, collapse);
            let strip = box_(&mut e, cell, ElementTag::Div, 200.0, 60.0);
            set(&mut e, strip, StyleProperty::Display, Display::Flex);
            let red = box_(&mut e, strip, ElementTag::Div, 100.0, 60.0);
            set(&mut e, red, StyleProperty::BackgroundColor, Color::RED);
            let green = box_(&mut e, strip, ElementTag::Div, 100.0, 60.0);
            set(&mut e, green, StyleProperty::BackgroundColor, Color::GREEN);
            let initial = e.scene().unwrap();
            let (corner, center) = points(&mut e, cell);
            assert_eq!(pixel(&frame(&initial), scale, center), [255, 0, 0, 255]);
            e.scroll_to(cell, 80.0, 0.0).unwrap();
            assert_eq!(e.scroll_offset(cell).unwrap(), (80.0, 0.0));
            let scrolled = e.scene().unwrap();
            let expected = frame(&scrolled);
            assert_eq!(
                pixel(&expected, scale, corner),
                [255; 4],
                "{scale}/{collapse:?}"
            );
            assert_eq!(pixel(&expected, scale, center), [0, 128, 0, 255]);
            assert_ne!(
                e.hit_test(corner.0 as f32, corner.1 as f32).unwrap(),
                Some(green)
            );
            assert_eq!(
                e.hit_test(center.0 as f32, center.1 as f32).unwrap(),
                Some(green)
            );
            set(&mut e, cell, StyleProperty::Position, Position::Relative);
            let atomic = e.scene().unwrap();
            assert_eq!(
                frame(&atomic).pixels,
                expected.pixels,
                "{scale}/{collapse:?}"
            );
            let stats = e.stats();
            assert_eq!(e.scene().unwrap().generation(), atomic.generation());
            assert_eq!(e.stats(), stats);
            drop(e);
            assert_eq!(frame(&scrolled).pixels, expected.pixels);
        }
    }
}
