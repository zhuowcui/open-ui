//! Shared column paint phases against complete measured Chromium captures.
//! The original style statements are copied unchanged; only the construction
//! facade adds an explicit qualification profile and a second authored route.
//! The paired expanded census of 23 October 2026 measures both tests exact
//! at every profile (`docs/progress/column-paint-phases-2026-10-23.md`).
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineError, EngineOptions, NodeHandle, ViewportMetrics};
use openui_geometry::{Length, RasterConfiguration};
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
        inner: Engine::new_with_font_collection_and_options(
            viewport,
            openui_text::FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: RasterConfiguration::deterministic_aliased(false),
                ..EngineOptions::default()
            },
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

fn css_break_out_of_flow_in_multicolumn_018(
    viewport: ViewportMetrics,
    authored: bool,
) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport, authored);
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(vp, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(vp, RendererStyleValue::PaddingTop(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::PaddingRight(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::PaddingBottom(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::PaddingLeft(Length::px(20.0)));
    doc.set_style(vp, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(vp, RendererStyleValue::ListStyleType(ListStyleType::None));
    let n1 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n1,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n1, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n1, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(
        n1,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(vp, n1);
    let n2 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n2, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n2, RendererStyleValue::ColumnFill(ColumnFill::Auto));
    doc.set_style(n2, RendererStyleValue::ColumnGap(Some(Length::px(0.0))));
    doc.set_style(n2, RendererStyleValue::Width(Length::px(50.0)));
    doc.set_style(n2, RendererStyleValue::BackgroundColor(Color::RED));
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n1, n2);
    let n3 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n3, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n3, RendererStyleValue::Position(Position::Relative));
    doc.set_style(n3, RendererStyleValue::Height(Length::px(360.0)));
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n3);
    let n4 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n4, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n4, RendererStyleValue::Height(Length::px(180.0)));
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n4);
    let n5 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n5, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n5, RendererStyleValue::ColumnSpan(ColumnSpan::All));
    doc.set_style(n5, RendererStyleValue::Height(Length::px(20.0)));
    doc.set_style(
        n5,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n5);
    let n6 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n6, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n6, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(n6, RendererStyleValue::Bottom(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::Top(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::Width(Length::px(25.0)));
    doc.set_style(
        n6,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n6);
    Ok(doc.into_engine())
}

fn raster(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(scene).unwrap()
}

type OracleCase = (
    f64,
    f64,
    f64,
    usize,
    usize,
    (usize, usize, usize, usize),
    &'static [u8],
);

fn assert_complete_chromium_frames(
    builder: fn(ViewportMetrics, bool) -> Result<Engine, EngineError>,
    cases: &[OracleCase],
) {
    for &(width, height, scale, physical_width, physical_height, crop, rgba) in cases {
        let viewport = ViewportMetrics::from_logical_size(width, height, scale).unwrap();
        let (left, top, crop_width, crop_height) = crop;
        assert!(left + crop_width <= physical_width && top + crop_height <= physical_height);
        assert_eq!(rgba.len(), crop_width * crop_height * 4);
        // Every source capture has a separately verified white complement.
        let mut expected = vec![255_u8; physical_width * physical_height * 4];
        for row in 0..crop_height {
            let from = row * crop_width * 4;
            let to = ((top + row) * physical_width + left) * 4;
            expected[to..to + crop_width * 4].copy_from_slice(&rgba[from..from + crop_width * 4]);
        }
        for authored in [false, true] {
            let mut previous_repeat: Option<Vec<u8>> = None;
            for repeat in 0..2 {
                let mut engine = builder(viewport, authored).unwrap();
                assert!(!engine.layered_paint_capture_enabled());
                let ordinary = engine.scene().unwrap();
                let frame = raster(&ordinary);
                assert_eq!(
                    (frame.width as usize, frame.height as usize, frame.stride),
                    (physical_width, physical_height, physical_width * 4)
                );
                assert_eq!(frame.pixels.len(), expected.len());
                let differing = frame
                    .pixels
                    .chunks_exact(4)
                    .zip(expected.chunks_exact(4))
                    .filter(|(actual, expected)| actual != expected)
                    .count();
                assert_eq!(differing, 0,
                    "complete measured Chromium frame @{scale}, authored={authored}, repeat={repeat}");
                if let Some(previous) = &previous_repeat {
                    assert!(
                        previous == &frame.pixels,
                        "independent native repeat changed the complete frame"
                    );
                }
                previous_repeat = Some(frame.pixels.clone());
                engine.set_layered_paint_capture_enabled(true);
                let captured = engine.scene().unwrap();
                assert!(captured.layered_paint_capture().is_some());
                let instrumented = raster(&captured);
                assert_eq!(
                    (frame.width, frame.height, frame.stride),
                    (instrumented.width, instrumented.height, instrumented.stride)
                );
                assert!(
                    frame.pixels == instrumented.pixels,
                    "same-walk capture changed the complete canonical frame @{scale}"
                );
                let generation = captured.generation();
                let stats = engine.stats();
                assert_eq!(engine.scene().unwrap().generation(), generation);
                assert_eq!(
                    engine.stats(),
                    stats,
                    "unchanged frame performed extra work"
                );
            }
        }
    }
}

#[test]
fn normal_column_background_phases_match_complete_chromium_frames() {
    let cases: &[OracleCase] = &[
        (
            800.0,
            600.0,
            1.0,
            800,
            600,
            (20, 20, 100, 100),
            include_bytes!("column_paint_phase_oracles/balance-003-legacy-800x600-1.rgba"),
        ),
        (
            375.0,
            667.0,
            2.0,
            750,
            1334,
            (40, 40, 200, 200),
            include_bytes!("column_paint_phase_oracles/balance-003-mobile-375x667-2.rgba"),
        ),
        (
            1280.0,
            720.0,
            1.25,
            1600,
            900,
            (25, 25, 125, 250),
            include_bytes!("column_paint_phase_oracles/balance-003-desktop-1280x720-1.25.rgba"),
        ),
        (
            1920.0,
            1080.0,
            1.5,
            2880,
            1620,
            (30, 30, 150, 300),
            include_bytes!("column_paint_phase_oracles/balance-003-desktop-1920x1080-1.5.rgba"),
        ),
    ];
    assert_complete_chromium_frames(css_multicol_multicol_fill_balance_003, cases);
}

#[test]
fn positioned_column_phase_and_extracted_spanner_match_complete_chromium_frames() {
    let cases: &[OracleCase] = &[
        (
            800.0,
            600.0,
            1.0,
            800,
            600,
            (20, 20, 100, 100),
            include_bytes!("column_paint_phase_oracles/oof-018-legacy-800x600-1.rgba"),
        ),
        (
            375.0,
            667.0,
            2.0,
            750,
            1334,
            (40, 40, 200, 200),
            include_bytes!("column_paint_phase_oracles/oof-018-mobile-375x667-2.rgba"),
        ),
        (
            1280.0,
            720.0,
            1.25,
            1600,
            900,
            (25, 25, 125, 125),
            include_bytes!("column_paint_phase_oracles/oof-018-desktop-1280x720-1.25.rgba"),
        ),
        (
            1920.0,
            1080.0,
            1.5,
            2880,
            1620,
            (30, 30, 150, 150),
            include_bytes!("column_paint_phase_oracles/oof-018-desktop-1920x1080-1.5.rgba"),
        ),
    ];
    assert_complete_chromium_frames(css_break_out_of_flow_in_multicolumn_018, cases);
}

#[test]
fn positioned_source_order_tracks_insert_and_reparent_without_reallocating_handles() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for authored in [false, true] {
            let viewport = ViewportMetrics::from_logical_size(240.0, 180.0, scale).unwrap();
            let (mut doc, body) = base_doc(viewport, authored);
            let outer = doc.create_node(ElementTag::Div);
            for value in [
                RendererStyleValue::Display(Display::Block),
                RendererStyleValue::Width(Length::px(100.0)),
                RendererStyleValue::Height(Length::px(100.0)),
                RendererStyleValue::ColumnCount(Some(2)),
                RendererStyleValue::ColumnFill(ColumnFill::Auto),
                RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
            ] {
                doc.set_style(outer, value);
            }
            doc.append_child(body, outer);
            let owner = doc.create_node(ElementTag::Div);
            for value in [
                RendererStyleValue::Display(Display::Block),
                RendererStyleValue::Position(Position::Relative),
                RendererStyleValue::Height(Length::px(200.0)),
            ] {
                doc.set_style(owner, value);
            }
            doc.append_child(outer, owner);
            let green = doc.create_node(ElementTag::Div);
            let red = doc.create_node(ElementTag::Div);
            for (node, color) in [
                (green, Color::from_rgba8(0, 128, 0, 255)),
                (red, Color::RED),
            ] {
                for value in [
                    RendererStyleValue::Display(Display::Block),
                    RendererStyleValue::Position(Position::Absolute),
                    RendererStyleValue::Top(Length::px(0.0)),
                    RendererStyleValue::Left(Length::px(0.0)),
                    RendererStyleValue::Width(Length::px(40.0)),
                    RendererStyleValue::Height(Length::px(60.0)),
                    RendererStyleValue::BackgroundColor(color),
                ] {
                    doc.set_style(node, value);
                }
                doc.append_child(owner, node);
            }
            let parking = doc.create_node(ElementTag::Div);
            doc.set_style(parking, RendererStyleValue::Display(Display::None));
            doc.append_child(body, parking);
            let mut engine = doc.into_engine();
            let mut initial: Option<Vec<u8>> = None;
            for state in 0..5 {
                match state {
                    0 => {}
                    1 => engine.insert_before(owner, red, green).unwrap(),
                    2 => engine.append_or_move_child(parking, red).unwrap(),
                    3 => engine.append_or_move_child(owner, red).unwrap(),
                    4 => engine.insert_before(owner, red, green).unwrap(),
                    _ => unreachable!(),
                }
                assert_eq!(engine.parent(green).unwrap(), Some(owner));
                let red_parent = if state == 2 { parking } else { owner };
                assert_eq!(engine.parent(red).unwrap(), Some(red_parent));
                assert!(engine.children(owner).unwrap().contains(&green));
                assert!(engine.children(red_parent).unwrap().contains(&red));
                engine.set_layered_paint_capture_enabled(false);
                let ordinary = engine.scene().unwrap();
                let frame = raster(&ordinary);
                // The point is strictly inside both opaque boxes and the
                // first retained column, so the last source sibling owns it.
                let x = (30.0 * scale).floor() as usize;
                let y = (30.0 * scale).floor() as usize;
                let at = y * frame.stride + x * 4;
                let expected = if state == 0 || state == 3 {
                    [255, 0, 0, 255]
                } else {
                    [0, 128, 0, 255]
                };
                assert_eq!(
                    &frame.pixels[at..at + 4],
                    &expected,
                    "current source order state={state} @{scale}, authored={authored}"
                );
                if state == 0 {
                    initial = Some(frame.pixels.clone());
                }
                if state == 3 {
                    assert!(
                        initial.as_ref().unwrap() == &frame.pixels,
                        "restoring the original tree order changed the complete frame"
                    );
                }
                engine.set_layered_paint_capture_enabled(true);
                let captured = engine.scene().unwrap();
                assert!(captured.layered_paint_capture().is_some());
                let actual = raster(&captured);
                assert!(
                    frame.pixels == actual.pixels,
                    "same-walk capture changed mutation output"
                );
                let stats = engine.stats();
                let generation = captured.generation();
                assert_eq!(engine.scene().unwrap().generation(), generation);
                assert_eq!(engine.stats(), stats);
            }
        }
    }
}
