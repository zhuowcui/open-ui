//! Real Engine construction preserves the original Balance witness's style
//! operations. Authored declarations and Snapshot inputs must publish the same
//! once-owned rows. Fractional AA is compared between real routes; the ideal
//! solid-square oracle is used only at the two pinned integer raster scales.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineError, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_layout::Fragment;
use openui_style::*;

struct FixtureEngine {
    inner: Engine,
    authored: bool,
}
impl FixtureEngine {
    fn create_node(&mut self, tag: ElementTag) -> NodeHandle {
        let n = self.inner.create_element(tag).unwrap();
        self.set_style(n, RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None));
        n
    }
    fn set_style(&mut self, n: NodeHandle, value: RendererStyleValue) {
        if self.authored {
            self.inner
                .set_property(n, value.property(), StyleValue::Renderer(value))
                .unwrap();
        } else {
            self.inner.set_renderer_style(n, value).unwrap();
        }
    }
    fn append_child(&mut self, p: NodeHandle, c: NodeHandle) {
        self.inner.append_child(p, c).unwrap();
    }
    fn into_engine(self) -> Engine {
        self.inner
    }
}
fn base_doc(viewport: ViewportMetrics, authored: bool) -> (FixtureEngine, NodeHandle) {
    let mut doc = FixtureEngine {
        inner: Engine::new_with_font_collection(
            viewport,
            openui_text::FontCollection::deterministic_test(),
        )
        .unwrap(),
        authored,
    };
    let root = doc.inner.root();
    doc.set_style(
        root,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    );
    doc.set_style(root, RendererStyleValue::Display(Display::Block));
    doc.set_style(root, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(root, RendererStyleValue::OverflowY(Overflow::Hidden));
    doc.set_style(
        root,
        RendererStyleValue::BackgroundColor(Color::TRANSPARENT),
    );
    let body = doc.create_node(ElementTag::Div);
    doc.set_style(body, RendererStyleValue::Display(Display::FlowRoot));
    for value in [
        RendererStyleValue::PaddingTop(Length::px(20.0)),
        RendererStyleValue::PaddingRight(Length::px(20.0)),
        RendererStyleValue::PaddingBottom(Length::px(20.0)),
        RendererStyleValue::PaddingLeft(Length::px(20.0)),
        RendererStyleValue::FontFamily(FontFamilyList::single("DejaVu Sans")),
        RendererStyleValue::FontSize(16.0),
        RendererStyleValue::Color(Color::BLACK),
    ] {
        doc.set_style(body, value);
    }
    doc.append_child(root, body);
    (doc, body)
}
fn css_multicol_multicol_fill_balance_003(
    viewport: ViewportMetrics,
    authored: bool,
) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport, authored);
    doc.set_style(vp, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::PaddingTop(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::PaddingRight(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::PaddingBottom(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::PaddingLeft(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    let n1 = doc.create_node(ElementTag::Div);
    doc.set_style(n1, RendererStyleValue::Display(Display::Block));
    doc.set_style(n1, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n1, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n1, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(n1, RendererStyleValue::ColumnFill(ColumnFill::Auto));
    doc.set_style(n1, RendererStyleValue::ColumnGap(Some(Length::px(0.0))));
    doc.set_style(n1, RendererStyleValue::Height(Length::px(200.0)));
    doc.set_style(n1, RendererStyleValue::Width(Length::px(100.0)));
    doc.append_child(vp, n1);
    let n2 = doc.create_node(ElementTag::Div);
    doc.set_style(n2, RendererStyleValue::Display(Display::Block));
    doc.set_style(n2, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n2, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(n2, RendererStyleValue::ColumnGap(Some(Length::px(0.0))));
    doc.set_style(n2, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(n1, n2);
    let n3 = doc.create_node(ElementTag::Div);
    doc.set_style(n3, RendererStyleValue::Display(Display::Block));
    doc.set_style(n3, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n3, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n3, RendererStyleValue::Height(Length::px(600.0)));
    doc.append_child(n2, n3);
    let n4 = doc.create_node(ElementTag::Div);
    doc.set_style(n4, RendererStyleValue::Display(Display::Block));
    doc.set_style(n4, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n4, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(
        n4,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.append_child(n3, n4);
    let n5 = doc.create_node(ElementTag::Div);
    doc.set_style(n5, RendererStyleValue::Display(Display::Block));
    doc.set_style(n5, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n5, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(n5, RendererStyleValue::BackgroundColor(Color::WHITE));
    doc.append_child(n3, n5);
    let n6 = doc.create_node(ElementTag::Div);
    doc.set_style(n6, RendererStyleValue::Display(Display::Block));
    doc.set_style(n6, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n6, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(
        n6,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.append_child(n3, n6);
    let n7 = doc.create_node(ElementTag::Div);
    doc.set_style(n7, RendererStyleValue::Display(Display::Block));
    doc.set_style(n7, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n7, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n7, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(n7, RendererStyleValue::BackgroundColor(Color::WHITE));
    doc.append_child(n3, n7);
    let n8 = doc.create_node(ElementTag::Div);
    doc.set_style(n8, RendererStyleValue::Display(Display::Block));
    doc.set_style(n8, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n8, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n8, RendererStyleValue::Height(Length::px(200.0)));
    doc.set_style(
        n8,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.append_child(n3, n8);
    Ok(doc.into_engine())
}

fn identify(mut e: Engine) -> (Engine, NodeHandle, NodeHandle) {
    let body = e.children(e.root()).unwrap()[0];
    let outer = e.children(body).unwrap()[0];
    let inner = e.children(outer).unwrap()[0];
    let block = e.children(inner).unwrap()[0];
    e.set_attribute(inner, "id", "balanced-inner").unwrap();
    e.set_attribute(block, "id", "balanced-block").unwrap();
    (e, inner, block)
}
fn parts<'a>(s: &'a SceneSnapshot, e: &Engine, id: &str) -> Vec<&'a Fragment> {
    fn walk<'a>(f: &'a Fragment, e: &Engine, id: &str, v: &mut Vec<&'a Fragment>) {
        if f.owns_element_box()
            && e.native_document()
                .node(f.node_id)
                .attributes
                .get("id")
                .map(String::as_str)
                == Some(id)
        {
            v.push(f);
        }
        for c in &f.children {
            walk(c, e, id, v);
        }
    }
    let mut v = Vec::new();
    walk(s.fragments(), e, id, &mut v);
    v
}
fn raster(s: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(s).unwrap()
}
fn assert_complete_block_source(s: &SceneSnapshot, e: &Engine, total: f64) {
    let mut records: Vec<_> = parts(s, e, "balanced-block")
        .into_iter()
        .map(|f| f.box_geometry.unwrap())
        .collect();
    records.sort_by_key(|g| g.consumed_block_size.raw());
    let mut consumed = 0.0;
    for g in records {
        assert_eq!(
            g.consumed_block_size.to_f64(),
            consumed,
            "no duplicated or missing normal source"
        );
        assert!(g.block_size.to_f64() > 0.0);
        consumed += g.block_size.to_f64();
    }
    assert_eq!(consumed, total);
}
#[test]
fn original_balance_source_resumes_and_rebalances_in_a_new_outer_fragmentainer() {
    for (w, h, scale) in [
        (800.0, 600.0, 1.0),
        (375.0, 667.0, 2.0),
        (1280.0, 720.0, 1.25),
        (1920.0, 1080.0, 1.5),
        (800.0, 600.0, 1.75),
    ] {
        let viewport = ViewportMetrics::from_logical_size(w, h, scale).unwrap();
        let mut frames = Vec::new();
        for authored in [false, true] {
            let (mut e, _, _) =
                identify(css_multicol_multicol_fill_balance_003(viewport, authored).unwrap());
            let s = e.scene().unwrap();
            let mut heights: Vec<_> = parts(&s, &e, "balanced-inner")
                .into_iter()
                .map(|f| {
                    assert_eq!(f.size.width.to_f64(), 50.0);
                    f.size.height.to_f64()
                })
                .collect();
            heights.sort_by(f64::total_cmp);
            assert_eq!(
                heights,
                vec![100.0, 200.0],
                "actual balanced outer source rows @{scale}/{authored}"
            );
            assert_complete_block_source(&s, &e, 600.0);
            let frame = raster(&s);
            if scale == 1.0 || scale == 2.0 {
                let start = (20.0 * scale) as usize;
                let end = (120.0 * scale) as usize;
                for y in 0..frame.height as usize {
                    for x in 0..frame.width as usize {
                        let off = y * frame.stride as usize + x * 4;
                        let wanted = if x >= start && x < end && y >= start && y < end {
                            [0, 128, 0, 255]
                        } else {
                            [255, 255, 255, 255]
                        };
                        assert_eq!(
                            &frame.pixels[off..off + 4],
                            &wanted,
                            "pinned integer whole image {x},{y}/{scale}/{authored}"
                        );
                    }
                }
            }
            let generation = s.generation();
            let stats = e.stats();
            assert_eq!(e.scene().unwrap().generation(), generation);
            assert_eq!(e.stats(), stats);
            frames.push(frame);
        }
        assert_eq!(
            frames[0].pixels, frames[1].pixels,
            "shared authored/Snapshot replay @{scale}"
        );
    }
}
#[test]
fn real_normal_source_and_end_margin_resume_after_the_first_row() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let viewport = ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap();
        let (mut e, _, block) =
            identify(css_multicol_multicol_fill_balance_003(viewport, true).unwrap());
        e.set_property(
            block,
            StyleProperty::MarginTop,
            StyleValue::Renderer(RendererStyleValue::MarginTop(Length::px(20.0))),
        )
        .unwrap();
        e.set_property(
            block,
            StyleProperty::MarginBottom,
            StyleValue::Renderer(RendererStyleValue::MarginBottom(Length::px(40.0))),
        )
        .unwrap();
        let s = e.scene().unwrap();
        let mut heights: Vec<_> = parts(&s, &e, "balanced-inner")
            .into_iter()
            .map(|f| f.size.height.to_f64())
            .collect();
        heights.sort_by(f64::total_cmp);
        assert_eq!(heights, vec![130.0, 200.0]);
        assert_complete_block_source(&s, &e, 600.0);
        let stats = e.stats();
        let generation = s.generation();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
    }
}

fn authored(e: &mut Engine, n: NodeHandle, property: StyleProperty, literal: &str) {
    e.set_property(n, property, parse_literal(property, literal).unwrap())
        .unwrap();
}
#[test]
fn bordered_short_first_row_uses_fresh_content_capacity_and_each_actual_source_row() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let viewport = ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap();
        let (mut e, inner, block) =
            identify(css_multicol_multicol_fill_balance_003(viewport, true).unwrap());
        let body = e.children(e.root()).unwrap()[0];
        let outer = e.children(body).unwrap()[0];
        let prefix = e.create_element(ElementTag::Div).unwrap();
        authored(&mut e, prefix, StyleProperty::Height, "30px");
        e.insert_before(outer, prefix, inner).unwrap();
        authored(&mut e, inner, StyleProperty::BreakBefore, "avoid");
        authored(&mut e, inner, StyleProperty::Border, "1px solid black");
        authored(&mut e, inner, StyleProperty::BoxDecorationBreak, "clone");
        authored(&mut e, block, StyleProperty::Height, "1000px");
        let s = e.scene().unwrap();
        assert_eq!(
            parts(&s, &e, "balanced-inner").len(),
            3,
            "real three-source-row continuation @{scale}"
        );
        assert_complete_block_source(&s, &e, 1000.0);
        let generation = s.generation();
        let stats = e.stats();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
    }
}
#[test]
fn exhausted_fixed_ancestor_keeps_normal_paragraph_lines_in_the_next_row() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let viewport = ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap();
        let (mut e, inner, block) =
            identify(css_multicol_multicol_fill_balance_003(viewport, true).unwrap());
        authored(&mut e, inner, StyleProperty::ColumnFill, "auto");
        e.remove_children(block).unwrap();
        for (p, v) in [
            (StyleProperty::Height, "50px"),
            (StyleProperty::BackgroundColor, "transparent"),
            (StyleProperty::FontFamily, "Ahem"),
            (StyleProperty::FontSize, "20px"),
            (StyleProperty::LineHeight, "20px"),
            (StyleProperty::WhiteSpace, "pre"),
            (StyleProperty::Color, "green"),
        ] {
            authored(&mut e, block, p, v);
        }
        let text = e.create_text(vec!["X"; 15].join("\n")).unwrap();
        e.append_child(block, text).unwrap();
        let fixed = e.scene().unwrap();
        assert_eq!(
            parts(&fixed, &e, "balanced-inner").len(),
            2,
            "normal paragraph rows @{scale}"
        );
        let fixed_pixels = raster(&fixed).pixels;
        let generation = fixed.generation();
        let stats = e.stats();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
        // Removing the ancestor's short principal size does not create new
        // glyph source. Both routes must retain the same fifteen real lines.
        authored(&mut e, block, StyleProperty::Height, "auto");
        let normal = e.scene().unwrap();
        assert_eq!(raster(&normal).pixels, fixed_pixels);
        authored(&mut e, block, StyleProperty::Height, "50px");
        let restored = e.scene().unwrap();
        assert_eq!(raster(&restored).pixels, fixed_pixels);
    }
}
#[test]
fn repeated_empty_block_end_margins_form_one_collapsed_continuing_strut() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let viewport = ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap();
        let (mut e, inner, _) =
            identify(css_multicol_multicol_fill_balance_003(viewport, true).unwrap());
        e.remove_children(inner).unwrap();
        for _ in 0..2 {
            let empty = e.create_element(ElementTag::Div).unwrap();
            authored(&mut e, empty, StyleProperty::Height, "0px");
            authored(&mut e, empty, StyleProperty::MarginBottom, "600px");
            e.append_child(inner, empty).unwrap();
        }
        let s = e.scene().unwrap();
        let mut heights: Vec<_> = parts(&s, &e, "balanced-inner")
            .into_iter()
            .map(|f| f.size.height.to_f64())
            .collect();
        heights.sort_by(f64::total_cmp);
        assert_eq!(heights, vec![100.0, 200.0]);
        let stats = e.stats();
        let generation = s.generation();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
    }
}
