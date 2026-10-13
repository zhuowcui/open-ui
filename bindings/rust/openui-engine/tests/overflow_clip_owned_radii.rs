//! Shared native radius operands. Native neighbor equality does not qualify
//! Chromium pixels or erase paint-versus-direct-fragment hit route differences.
use openui_compositor::{SceneBoxRole, SceneNodeIdentity, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, FontCollection, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_paint::OverflowClipContourPurpose;
use openui_style::{
    Border, BorderCollapse, BorderStyle, BoxSizing, Color, ColumnFill, Containment, CornerRadii,
    Display, Edges, LengthValue, Overflow, OverflowClipBox, Position, RendererStyleValue,
    ScrollbarWidth, StyleProperty, StyleValue, TransformList, TransformOperation, WritingMode,
};
const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn engine(scale: f64) -> Engine {
    let mut e = Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(240.0, 180.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    let root = e.root();
    e.set_property(root, StyleProperty::BackgroundColor, Color::WHITE.into())
        .unwrap();
    e
}
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
fn identity(n: NodeHandle) -> SceneNodeIdentity {
    SceneNodeIdentity {
        document: n.document_id(),
        index: n.index(),
        generation: n.generation(),
    }
}
fn clip<'a>(
    scene: &'a SceneSnapshot,
    n: NodeHandle,
    role: SceneBoxRole,
) -> &'a openui_compositor::SceneClipProjection {
    &scene
        .retained_scene()
        .unwrap()
        .clips()
        .iter()
        .find(|c| c.owner == identity(n) && c.role == role)
        .unwrap()
        .projections[0]
}
fn fixed_ellipse(e: &mut Engine, n: NodeHandle, radii: [(f32, f32); 4]) {
    set(
        e,
        n,
        StyleProperty::BorderRadiusPercent,
        StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent([(false, false); 4])),
    );
    for (p, v) in [
        (
            StyleProperty::BorderTopLeftRadius,
            RendererStyleValue::BorderTopLeftRadius(radii[0]),
        ),
        (
            StyleProperty::BorderTopRightRadius,
            RendererStyleValue::BorderTopRightRadius(radii[1]),
        ),
        (
            StyleProperty::BorderBottomRightRadius,
            RendererStyleValue::BorderBottomRightRadius(radii[2]),
        ),
        (
            StyleProperty::BorderBottomLeftRadius,
            RendererStyleValue::BorderBottomLeftRadius(radii[3]),
        ),
    ] {
        set(e, n, p, StyleValue::Renderer(v));
    }
}
fn pixels(scene: &SceneSnapshot) -> Vec<u8> {
    SoftwareCompositor::default().render(scene).unwrap().pixels
}
#[test]
fn calculated_radii_resolve_on_the_border_source_before_inner_contraction() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let port = box_(&mut e, root, ElementTag::Div, 100.0, 80.0);
        set(
            &mut e,
            port,
            StyleProperty::Border,
            Border {
                width: 10.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(
            &mut e,
            port,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(5.0)),
        );
        set(&mut e, port, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            port,
            StyleProperty::ScrollbarWidth,
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
        );
        let child = box_(&mut e, port, ElementTag::Div, 160.0, 160.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        set(
            &mut e,
            port,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::calc_percent_px(25.0, 5.0))),
        );
        let calculated = e.scene().unwrap();
        let projection = clip(&calculated, port, SceneBoxRole::Principal);
        assert_eq!(
            projection.border_radius_lengths,
            [Some((
                Length::calc_percent_px(25.0, 5.0),
                Length::calc_percent_px(25.0, 5.0)
            )); 4]
        );
        assert_eq!(
            projection.paint_border_radius_reference_box,
            (0.0, 0.0, 100.0, 80.0)
        );
        let rounded = projection
            .direct_hit_clip_geometry
            .rounded
            .as_ref()
            .unwrap();
        assert_eq!(
            rounded.purpose,
            OverflowClipContourPurpose::DirectFragmentHit
        );
        assert_eq!(rounded.rect, (10.0, 10.0, 80.0, 60.0));
        // Source outer 30x25, then the actual ten-pixel borders: 20x15.
        assert_eq!(rounded.radii, [(20.0, 15.0); 4]);
        let probes = [
            (16.0, 16.0),
            (25.0, 20.0),
            (50.0, 40.0),
            (84.0, 64.0),
            (95.0, 75.0),
        ];
        let hits: Vec<_> = probes
            .iter()
            .map(|&(x, y)| e.hit_test(x, y).unwrap())
            .collect();
        let image = pixels(&calculated);
        fixed_ellipse(&mut e, port, [(30.0, 25.0); 4]);
        let fixed = e.scene().unwrap();
        assert_eq!(image, pixels(&fixed), "scale={scale}");
        assert_eq!(
            hits,
            probes
                .iter()
                .map(|&(x, y)| e.hit_test(x, y).unwrap())
                .collect::<Vec<_>>()
        );
        let stats = e.stats();
        let generation = fixed.generation();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
        drop(e);
        assert_eq!(pixels(&calculated), image);
        assert_eq!(
            projection.border_radius_lengths[0].unwrap().0,
            Length::calc_percent_px(25.0, 5.0)
        );
    }
}
#[test]
fn pure_percent_and_font_lengths_keep_source_kinds_through_box_and_font_mutation() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let port = box_(&mut e, root, ElementTag::Div, 100.0, 80.0);
        set(&mut e, port, StyleProperty::Overflow, Overflow::Hidden);
        let child = box_(&mut e, port, ElementTag::Div, 100.0, 80.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        set(
            &mut e,
            port,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::percent(50.0))),
        );
        let percent = e.scene().unwrap();
        let source = clip(&percent, port, SceneBoxRole::Principal);
        assert_eq!(
            source.paint_clip_geometry.rounded.as_ref().unwrap().radii,
            [(50.0, 40.0); 4]
        );
        set(&mut e, port, StyleProperty::Width, LengthValue::px(80.0));
        let resized = e.scene().unwrap();
        assert_eq!(
            clip(&resized, port, SceneBoxRole::Principal)
                .paint_clip_geometry
                .rounded
                .as_ref()
                .unwrap()
                .radii,
            [(40.0, 40.0); 4]
        );
        set(&mut e, port, StyleProperty::FontSize, LengthValue::px(12.0));
        set(
            &mut e,
            port,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::Em(1.0))),
        );
        let font = e.scene().unwrap();
        assert_eq!(
            clip(&font, port, SceneBoxRole::Principal)
                .paint_clip_geometry
                .rounded
                .as_ref()
                .unwrap()
                .radii,
            [(12.0, 12.0); 4]
        );
        set(&mut e, port, StyleProperty::FontSize, LengthValue::px(18.0));
        let changed = e.scene().unwrap();
        assert_eq!(
            clip(&changed, port, SceneBoxRole::Principal)
                .paint_clip_geometry
                .rounded
                .as_ref()
                .unwrap()
                .radii,
            [(18.0, 18.0); 4]
        );
        fixed_ellipse(&mut e, port, [(18.0, 18.0); 4]);
        assert_eq!(pixels(&changed), pixels(&e.scene().unwrap()));
        assert_eq!(
            source.border_radius_lengths[0],
            Some((Length::percent(50.0), Length::percent(50.0)))
        );
    }
}
#[test]
fn fieldset_content_retains_sources_while_legend_keeps_its_own_hit_and_paint_scope() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let fieldset = box_(&mut e, root, ElementTag::Fieldset, 100.0, 100.0);
        set(
            &mut e,
            fieldset,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(0.0)),
        );
        set(
            &mut e,
            fieldset,
            StyleProperty::Border,
            Border {
                width: 4.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(&mut e, fieldset, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            fieldset,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::calc_percent_px(20.0, 4.0))),
        );
        let legend = box_(&mut e, fieldset, ElementTag::Legend, 40.0, 12.0);
        set(&mut e, legend, StyleProperty::BackgroundColor, Color::RED);
        let content = box_(&mut e, fieldset, ElementTag::Div, 180.0, 180.0);
        set(
            &mut e,
            content,
            StyleProperty::BackgroundColor,
            Color::GREEN,
        );
        let scene = e.scene().unwrap();
        let content_clip = clip(&scene, fieldset, SceneBoxRole::FieldsetContent);
        assert!(content_clip
            .border_radius_lengths
            .iter()
            .all(Option::is_some));
        assert!(content_clip.paint_clip_geometry.rounded.is_some());
        assert!(content_clip.direct_hit_clip_geometry.rounded.is_some());
        let legend_box = e.bounds(legend).unwrap().unwrap();
        let x = legend_box.x + legend_box.width * 0.5;
        let y = legend_box.y + legend_box.height * 0.5;
        assert_eq!(e.hit_test(x as f32, y as f32).unwrap(), Some(legend));
        e.scroll_to(fieldset, 0.0, 30.0).unwrap();
        assert_eq!(e.bounds(legend).unwrap().unwrap(), legend_box);
        assert_eq!(e.hit_test(x as f32, y as f32).unwrap(), Some(legend));
        // A uniform fixed source is equivalent on a square ordinary box; a
        // fieldset's delegated source basis remains owned instead of guessed.
        let ordinary = box_(&mut e, root, ElementTag::Div, 80.0, 80.0);
        set(
            &mut e,
            ordinary,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::px(20.0))),
        );
        set(&mut e, ordinary, StyleProperty::Overflow, Overflow::Hidden);
        assert!(clip(&e.scene().unwrap(), ordinary, SceneBoxRole::Principal)
            .paint_clip_geometry
            .rounded
            .is_some());
        drop(e);
        assert!(content_clip.border_radius_lengths[0]
            .unwrap()
            .0
            .is_calculated());
        assert!(!pixels(&scene).is_empty());
    }
}
#[test]
fn real_bordered_continuations_keep_paint_source_and_direct_fragment_operands_distinct() {
    for scale in SCALES {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let mut e = engine(scale);
            let root = e.root();
            let (column_width, column_height) = if mode == WritingMode::HorizontalTb {
                (200.0, 40.0)
            } else {
                (40.0, 200.0)
            };
            let columns = box_(&mut e, root, ElementTag::Div, column_width, column_height);
            set(
                &mut e,
                columns,
                StyleProperty::WritingMode,
                StyleValue::Renderer(RendererStyleValue::WritingMode(mode)),
            );
            set(&mut e, columns, StyleProperty::ColumnCount, 2_i32);
            set(
                &mut e,
                columns,
                StyleProperty::ColumnGap,
                LengthValue::px(0.0),
            );
            set(
                &mut e,
                columns,
                StyleProperty::ColumnFill,
                StyleValue::Renderer(RendererStyleValue::ColumnFill(ColumnFill::Auto)),
            );
            let continued = box_(&mut e, columns, ElementTag::Div, 100.0, 100.0);
            set(
                &mut e,
                continued,
                StyleProperty::Border,
                Border {
                    width: 10.0,
                    style: BorderStyle::Solid,
                    color: Color::BLACK,
                },
            );
            set(
                &mut e,
                continued,
                StyleProperty::BorderRadius,
                CornerRadii(Edges::all(LengthValue::percent(50.0))),
            );
            set(
                &mut e,
                continued,
                StyleProperty::Contain,
                StyleValue::Renderer(RendererStyleValue::Contain(Containment::PAINT)),
            );
            let child = box_(&mut e, continued, ElementTag::Div, 80.0, 80.0);
            set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
            let scene = e.scene().unwrap();
            let clips = &scene
                .retained_scene()
                .unwrap()
                .clips()
                .iter()
                .find(|c| c.owner == identity(continued) && c.role == SceneBoxRole::Principal)
                .unwrap()
                .projections;
            assert!(clips.len() >= 2);
            assert!(clips.iter().any(|p| p.paint_border_radius_reference_box.3
                > p.hit_border_radius_reference_box.3
                || p.paint_border_radius_reference_box.2 > p.hit_border_radius_reference_box.2));
            let middle = clips
                .iter()
                .find(|p| {
                    let zero = openui_geometry::LayoutUnit::zero();
                    (p.used_border.top == zero
                        || p.used_border.left == zero
                        || p.used_border.right == zero)
                        && p.hit_border_radius_reference_box.2 > 20.0
                        && p.hit_border_radius_reference_box.3 > 20.0
                })
                .unwrap();
            let direct = middle.direct_hit_clip_geometry.rounded.as_ref().unwrap();
            let basis = middle.hit_border_radius_reference_box;
            assert_eq!(direct.rect.0, (basis.0 + 10.0).round());
            assert_eq!(direct.rect.1, (basis.1 + 10.0).round());
            assert_eq!(
                direct.rect.2,
                ((basis.0 + basis.2 - 10.0).round() - (basis.0 + 10.0).round()).max(0.0)
            );
            assert_eq!(
                direct.rect.3,
                ((basis.1 + basis.3 - 10.0).round() - (basis.1 + 10.0).round()).max(0.0)
            );
            assert_eq!(direct.border_radius_reference_box, basis);
            assert_eq!(
                direct.purpose,
                OverflowClipContourPurpose::DirectFragmentHit
            );
            // A used block edge is sliced away; the direct .50 inner check still
            // contracts by all four full computed ten-pixel borders.
            assert!(direct.rect.1 > basis.1);
            let stats = e.stats();
            assert_eq!(e.scene().unwrap().generation(), scene.generation());
            assert_eq!(e.stats(), stats);
        }
    }
}
#[test]
fn mixed_css_axes_keep_paint_and_direct_child_hit_route_policies_explicit() {
    let mut e = engine(1.0);
    let root = e.root();
    let port = box_(&mut e, root, ElementTag::Div, 80.0, 80.0);
    set(&mut e, port, StyleProperty::OverflowX, Overflow::Clip);
    set(&mut e, port, StyleProperty::OverflowY, Overflow::Visible);
    set(
        &mut e,
        port,
        StyleProperty::BorderRadius,
        CornerRadii(Edges::all(LengthValue::px(30.0))),
    );
    let child = box_(&mut e, port, ElementTag::Div, 80.0, 140.0);
    set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
    let scene = e.scene().unwrap();
    let p = clip(&scene, port, SceneBoxRole::Principal);
    assert!(p.paint_clip_geometry.clips_x);
    assert!(!p.paint_clip_geometry.clips_y);
    assert!(p.paint_clip_geometry.rounded.is_none());
    assert!(p.direct_hit_clip_geometry.rounded.is_some());
    // This records distinct .50 source routes; actual browser DOM/AX dispatch
    // for the mixed-axis case remains a required separate oracle measurement.
}

#[test]
fn content_reference_clip_margin_keeps_the_direct_inner_border_check_separate() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let port = box_(&mut e, root, ElementTag::Div, 100.0, 100.0);
        set(
            &mut e,
            port,
            StyleProperty::Border,
            Border {
                width: 4.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(
            &mut e,
            port,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(8.0)),
        );
        set(&mut e, port, StyleProperty::Overflow, Overflow::Clip);
        set(
            &mut e,
            port,
            StyleProperty::OverflowClipBox,
            StyleValue::Renderer(RendererStyleValue::OverflowClipBox(
                OverflowClipBox::ContentBox,
            )),
        );
        set(&mut e, port, StyleProperty::OverflowClipMargin, 6.0_f32);
        set(
            &mut e,
            port,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::calc_percent_px(15.0, 5.0))),
        );
        let child = box_(&mut e, port, ElementTag::Div, 160.0, 160.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        let scene = e.scene().unwrap();
        let p = clip(&scene, port, SceneBoxRole::Principal);
        let paint = p.paint_clip_geometry.rounded.as_ref().unwrap();
        let hit = p.direct_hit_clip_geometry.rounded.as_ref().unwrap();
        assert_eq!(paint.rect, (6.0, 6.0, 88.0, 88.0));
        assert_eq!(paint.radii, [(14.0, 14.0); 4]);
        assert_eq!(hit.rect, (4.0, 4.0, 92.0, 92.0));
        assert_eq!(hit.radii, [(16.0, 16.0); 4]);
        let image = pixels(&scene);
        fixed_ellipse(&mut e, port, [(20.0, 20.0); 4]);
        assert_eq!(image, pixels(&e.scene().unwrap()));
    }
}
#[test]
fn fractional_layout_placement_transform_and_scroll_preserve_source_radius_neighbors() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let parent = box_(&mut e, root, ElementTag::Div, 150.0, 120.0);
        set(&mut e, parent, StyleProperty::Overflow, Overflow::Hidden);
        let port = box_(&mut e, parent, ElementTag::Div, 100.0, 80.0);
        set(&mut e, port, StyleProperty::Position, Position::Relative);
        set(&mut e, port, StyleProperty::Left, LengthValue::px(0.375));
        set(&mut e, port, StyleProperty::Top, LengthValue::px(0.625));
        set(
            &mut e,
            port,
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Translate(
                LengthValue::px(2.25),
                LengthValue::px(1.5),
            )]),
        );
        set(
            &mut e,
            port,
            StyleProperty::Border,
            Border {
                width: 3.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(&mut e, port, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            port,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::calc_percent_px(25.0, 5.0))),
        );
        let child = box_(&mut e, port, ElementTag::Div, 200.0, 140.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        e.scroll_to(port, 20.0, 10.0).unwrap();
        let scene = e.scene().unwrap();
        let p = clip(&scene, port, SceneBoxRole::Principal);
        assert_eq!(
            p.hit_border_radius_reference_box,
            (0.375, 0.625, 100.0, 80.0)
        );
        let hit = p.direct_hit_clip_geometry.rounded.as_ref().unwrap();
        assert_eq!(hit.rect.0, 3.0);
        assert_eq!(hit.rect.1, 4.0);
        let probes = [(4.0, 4.0), (15.0, 15.0), (45.0, 30.0), (95.0, 72.0)];
        let targets: Vec<_> = probes
            .iter()
            .map(|&(x, y)| e.hit_test(x, y).unwrap())
            .collect();
        let image = pixels(&scene);
        fixed_ellipse(&mut e, port, [(30.0, 25.0); 4]);
        assert_eq!(image, pixels(&e.scene().unwrap()));
        assert_eq!(
            targets,
            probes
                .iter()
                .map(|&(x, y)| e.hit_test(x, y).unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(e.scroll_offset(port).unwrap(), (20.0, 10.0));
    }
}
#[test]
fn internal_and_collapsed_tables_keep_ignored_radii_out_of_clip_geometry() {
    for collapse in [BorderCollapse::Separate, BorderCollapse::Collapse] {
        let mut e = engine(1.0);
        let root = e.root();
        let table = box_(&mut e, root, ElementTag::Table, 100.0, 60.0);
        set(&mut e, table, StyleProperty::Display, Display::Table);
        set(
            &mut e,
            table,
            StyleProperty::BorderCollapse,
            StyleValue::Renderer(RendererStyleValue::BorderCollapse(collapse)),
        );
        set(&mut e, table, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            table,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::calc_percent_px(20.0, 4.0))),
        );
        let row = box_(&mut e, table, ElementTag::TableRow, 100.0, 60.0);
        set(&mut e, row, StyleProperty::Display, Display::TableRow);
        set(
            &mut e,
            row,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::px(20.0))),
        );
        let cell = box_(&mut e, row, ElementTag::TableCell, 100.0, 60.0);
        set(&mut e, cell, StyleProperty::Display, Display::TableCell);
        set(&mut e, cell, StyleProperty::BackgroundColor, Color::GREEN);
        let scene = e.scene().unwrap();
        let table_clip = clip(&scene, table, SceneBoxRole::Principal);
        let row_clip = clip(&scene, row, SceneBoxRole::Principal);
        assert!(row_clip.paint_clip_geometry.rounded.is_none());
        assert!(row_clip.direct_hit_clip_geometry.rounded.is_none());
        if collapse == BorderCollapse::Collapse {
            assert!(table_clip.paint_clip_geometry.rounded.is_none());
            assert_eq!(table_clip.border_radius_lengths, [None; 4]);
        } else {
            assert!(table_clip.paint_clip_geometry.rounded.is_some());
            assert!(table_clip.border_radius_lengths.iter().all(Option::is_some));
        }
    }
}

#[test]
fn negative_half_placement_and_thin_owned_boxes_use_layout_unit_contour_snapping() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let port = box_(&mut e, root, ElementTag::Div, 80.0, 80.0);
        set(&mut e, port, StyleProperty::Position, Position::Relative);
        set(&mut e, port, StyleProperty::Left, LengthValue::px(-10.5));
        set(&mut e, port, StyleProperty::Top, LengthValue::px(-10.5));
        set(
            &mut e,
            port,
            StyleProperty::Border,
            Border {
                width: 10.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(&mut e, port, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            port,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::calc_percent_px(25.0, 5.0))),
        );
        let child = box_(&mut e, port, ElementTag::Div, 100.0, 100.0);
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        let scene = e.scene().unwrap();
        let p = clip(&scene, port, SceneBoxRole::Principal);
        assert_eq!(
            p.hit_border_radius_reference_box,
            (-10.5, -10.5, 80.0, 80.0)
        );
        let direct = p.direct_hit_clip_geometry.rounded.as_ref().unwrap();
        assert_eq!(direct.rect, (0.0, 0.0, 60.0, 60.0));
        assert_eq!(direct.radii, [(15.0, 15.0); 4]);
        assert_ne!(e.hit_test(1.0, 1.0).unwrap(), Some(child));
        assert_eq!(e.hit_test(25.0, 20.0).unwrap(), Some(child));
        let image = pixels(&scene);
        fixed_ellipse(&mut e, port, [(25.0, 25.0); 4]);
        assert_eq!(image, pixels(&e.scene().unwrap()));

        let thin = box_(&mut e, root, ElementTag::Div, 100.0, 0.125);
        set(&mut e, thin, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            thin,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::percent(50.0))),
        );
        let thin_child = box_(&mut e, thin, ElementTag::Div, 100.0, 10.0);
        set(
            &mut e,
            thin_child,
            StyleProperty::BackgroundColor,
            Color::RED,
        );
        let thin_scene = e.scene().unwrap();
        let p = clip(&thin_scene, thin, SceneBoxRole::Principal);
        assert_eq!(p.hit_border_radius_reference_box.3, 0.125);
        assert_eq!(
            p.direct_hit_clip_geometry.rounded.as_ref().unwrap().rect.3,
            0.0
        );
        let bounds = e.bounds(thin).unwrap().unwrap();
        assert_ne!(e.hit_test(50.0, bounds.y).unwrap(), Some(thin_child));
    }
}
