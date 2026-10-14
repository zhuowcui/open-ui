//! Real Grid containing-block source windows through the public native Engine.
//! Original generated construction operations are preserved in the three copied builders.
//! All new guards are authored UNRUN; immutable Chromium remains the pixel oracle.
use openui_compositor::{SceneNodeIdentity, SceneSnapshot};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineError, EngineOptions, NodeHandle, RendererNodeState};
use openui_geometry::{Length, PhysicalOffset, RasterConfiguration, ViewportMetrics};
use openui_layout::Fragment;
use openui_style::*;

// This test-only construction facade follows the existing native FixtureEngine
// setter/create/append and capture-scrollbar policy. It exposes no mutable styles.
type NodeId = NodeHandle;
struct FixtureEngine {
    inner: Engine,
    authored_scrollbar_widths: std::collections::HashMap<NodeId, ScrollbarWidth>,
}
impl FixtureEngine {
    fn new(viewport: ViewportMetrics) -> Result<Self, EngineError> {
        let mut inner = Engine::new_with_font_collection_and_options(
            viewport,
            openui_text::FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: RasterConfiguration::deterministic_aliased(false),
            },
        )?;
        inner.set_renderer_style(
            inner.root(),
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        )?;
        Ok(Self {
            inner,
            authored_scrollbar_widths: std::collections::HashMap::new(),
        })
    }
    fn root(&self) -> NodeId {
        self.inner.root()
    }
    fn into_engine(self) -> Engine {
        self.inner
    }
    fn create_node(&mut self, tag: ElementTag) -> NodeId {
        let node = self.inner.create_element(tag).expect("fixture element");
        self.inner
            .set_renderer_style(
                node,
                RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
            )
            .expect("fixture capture scrollbar");
        node
    }
    fn append_child(&mut self, parent: NodeId, child: NodeId) {
        self.inner
            .append_child(parent, child)
            .expect("fixture append")
    }
    fn set_attribute(&mut self, node: NodeId, name: impl Into<String>, value: impl Into<String>) {
        self.inner
            .set_attribute(node, name, value)
            .expect("fixture attribute")
    }
    fn set_style(&mut self, node: NodeId, value: RendererStyleValue) {
        let affects = matches!(
            &value,
            RendererStyleValue::ScrollbarWidth(_)
                | RendererStyleValue::ScrollbarThumbColor(_)
                | RendererStyleValue::ScrollbarTrackColor(_)
        );
        if let RendererStyleValue::ScrollbarWidth(width) = &value {
            self.authored_scrollbar_widths.insert(node, *width);
        }
        self.inner
            .set_renderer_style(node, value)
            .expect("fixture style");
        if affects {
            let authored = self
                .authored_scrollbar_widths
                .get(&node)
                .copied()
                .unwrap_or(ScrollbarWidth::Auto);
            let computed = self.inner.computed_style(node).expect("fixture style read");
            let effective = if authored != ScrollbarWidth::Auto
                || computed.scrollbar_thumb_color.is_some()
                || computed.scrollbar_track_color.is_some()
            {
                authored
            } else {
                ScrollbarWidth::None
            };
            self.inner
                .set_renderer_style(node, RendererStyleValue::ScrollbarWidth(effective))
                .expect("fixture capture precedence");
        }
    }
    fn computed_style(&self, node: NodeId) -> &ComputedStyle {
        self.inner.computed_style(node).expect("fixture style read")
    }
}

fn base_doc(viewport_metrics: ViewportMetrics) -> (FixtureEngine, NodeId) {
    let mut doc = FixtureEngine::new(viewport_metrics).expect("fixture engine");
    let viewport = doc.root();
    // Viewport: no margin/padding, just a container matching screen dimensions
    doc.set_style(viewport, RendererStyleValue::Display(Display::Block));
    // BODY_STYLE also sets `html { overflow: hidden; }`; the body itself
    // remains visible below so its background and overflowing ink can propagate.
    doc.set_style(viewport, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(viewport, RendererStyleValue::OverflowY(Overflow::Hidden));
    // The raster surface itself supplies the initial white canvas. Keep the
    // synthetic viewport transparent so an explicitly emitted legacy `html`
    // background can be distinguished from that initial canvas color.
    doc.set_style(
        viewport,
        RendererStyleValue::BackgroundColor(Color::TRANSPARENT),
    );

    // Body: child of viewport, carries the default body padding.
    // Use display:flow-root (not overflow:hidden) to establish a BFC.
    // Per CSS spec, <body>'s overflow is propagated to the viewport, so
    // the body itself has overflow:visible — it must NOT clip content.
    // The viewport/canvas clips at 800×600 like Chrome's viewport does.
    let body = doc.create_node(ElementTag::Div);
    doc.set_style(body, RendererStyleValue::Display(Display::FlowRoot));
    doc.set_style(body, RendererStyleValue::PaddingTop(Length::px(20.0)));
    doc.set_style(body, RendererStyleValue::PaddingRight(Length::px(20.0)));
    doc.set_style(body, RendererStyleValue::PaddingBottom(Length::px(20.0)));
    doc.set_style(body, RendererStyleValue::PaddingLeft(Length::px(20.0)));
    doc.set_style(
        body,
        RendererStyleValue::FontFamily(FontFamilyList::single("DejaVu Sans")),
    );
    doc.set_style(body, RendererStyleValue::FontSize(16.0));
    doc.set_style(body, RendererStyleValue::Color(Color::BLACK));
    doc.append_child(viewport, body);
    (doc, body)
}

fn css_break_grid_grid_item_009(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n1, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::ColumnFill(ColumnFill::Auto));
    doc.set_style(n1, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::RowGap(Some(Length::px(0.0))));
    doc.set_style(n1, RendererStyleValue::ColumnGap(Some(Length::px(0.0))));
    doc.set_style(n1, RendererStyleValue::ColumnWidth(None));
    doc.set_style(n1, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        n1,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(vp, n1);
    doc.set_attribute(
        n1,
        "style",
        "width: 100px; column-fill: auto; height: 100px; gap: 0; columns: 2;",
    );
    let n2 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n2, RendererStyleValue::ListStyleType(ListStyleType::None));
    doc.set_style(n2, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n2, RendererStyleValue::Display(Display::Grid));
    doc.set_style(n2, RendererStyleValue::Position(Position::Relative));
    doc.set_style(
        n2,
        RendererStyleValue::GridTemplateRows(GridTrackList::Tracks(vec![
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Auto)),
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Length(
                Length::px(30.0),
            ))),
        ])),
    );
    doc.set_style(
        n2,
        RendererStyleValue::GridTemplateColumns(GridTrackList::Tracks(vec![
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Auto)),
        ])),
    );
    doc.set_style(n2, RendererStyleValue::BackgroundColor(Color::RED));
    doc.set_style(n2, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n1, n2);
    doc.set_attribute(
        n2,
        "style",
        "display: grid; position: relative; grid-template: auto 30px / auto; background: red;",
    );
    let n3 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(
        n3,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n3, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n3);
    doc.set_attribute(n3, "style", "background: green;");
    let n4 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n4, RendererStyleValue::Height(Length::px(70.0)));
    doc.set_style(n4, RendererStyleValue::BreakInside(BreakInside::Avoid));
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n4);
    doc.set_attribute(n4, "style", "height: 70px; break-inside: avoid;");
    let n5 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n5, RendererStyleValue::Height(Length::px(70.0)));
    doc.set_style(n5, RendererStyleValue::BreakInside(BreakInside::Avoid));
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n5);
    doc.set_attribute(n5, "style", "height: 70px; break-inside: avoid;");
    let n6 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n6, RendererStyleValue::Top(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::Left(Length::px(0.0)));
    doc.set_style(n6, RendererStyleValue::Width(Length::px(50.0)));
    doc.set_style(n6, RendererStyleValue::Height(Length::px(30.0)));
    {
        let mut value = doc.computed_style(n6).grid_row.clone();
        value.start = GridLine::Line {
            index: 2,
            name: None,
        };
        doc.set_style(n6, RendererStyleValue::GridRow(value));
    }
    {
        let mut value = doc.computed_style(n6).grid_row.clone();
        value.end = GridLine::Line {
            index: 2,
            name: None,
        };
        doc.set_style(n6, RendererStyleValue::GridRow(value));
    }
    doc.set_style(
        n6,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n6, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n6);
    doc.set_attribute(n6, "<", "");
    doc.set_attribute(n6, "div", "");
    doc.set_attribute(n6, "style", "position: absolute; top: 0; left: 0; width: 50px; height: 30px; grid-row-start: 2; grid-row-end: 2; background: green;");
    Ok(doc.into_engine())
}

fn css_break_grid_grid_item_oof_005(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n1, RendererStyleValue::ColumnWidth(None));
    doc.set_style(n1, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(n1, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::ColumnGap(Some(Length::px(0.0))));
    doc.set_style(n1, RendererStyleValue::ColumnFill(ColumnFill::Auto));
    doc.set_style(n1, RendererStyleValue::BackgroundColor(Color::RED));
    doc.set_style(n1, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n1,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(vp, n1);
    doc.set_attribute(n1, "style", "columns: 2; height: 100px; width: 100px; column-gap: 0; column-fill: auto; background: red;");
    let n2 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n2, RendererStyleValue::ListStyleType(ListStyleType::None));
    doc.set_style(n2, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n2, RendererStyleValue::Position(Position::Relative));
    doc.set_style(n2, RendererStyleValue::Display(Display::Grid));
    doc.set_style(
        n2,
        RendererStyleValue::GridTemplateRows(GridTrackList::Tracks(vec![
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Length(
                Length::px(50.0),
            ))),
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Auto)),
        ])),
    );
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n1, n2);
    doc.set_attribute(
        n2,
        "style",
        "position: relative; display: grid; grid-template-rows: 50px auto;",
    );
    let n3 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n3, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(
        n3,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n3, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(n3, RendererStyleValue::Width(Length::px(50.0)));
    doc.set_style(n3, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n3);
    doc.set_attribute(
        n3,
        "style",
        "position: absolute; background: green; width: 50px; height: 100px;",
    );
    let n4 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    {
        let mut value = doc.computed_style(n4).grid_row.clone();
        value.start = GridLine::Line {
            index: 2,
            name: None,
        };
        doc.set_style(n4, RendererStyleValue::GridRow(value));
    }
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n4);
    doc.set_attribute(n4, "style", "grid-row-start: 2;");
    let n5 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n5, RendererStyleValue::BreakInside(BreakInside::Avoid));
    doc.set_style(n5, RendererStyleValue::Height(Length::px(40.0)));
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n4, n5);
    doc.set_attribute(n5, "style", "break-inside: avoid; height: 40px;");
    let n6 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n6, RendererStyleValue::BreakInside(BreakInside::Avoid));
    doc.set_style(n6, RendererStyleValue::Height(Length::px(40.0)));
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n4, n6);
    doc.set_attribute(n6, "style", "break-inside: avoid; height: 40px;");
    let n7 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n7,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n7, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    {
        let mut value = doc.computed_style(n7).grid_row.clone();
        value.start = GridLine::Line {
            index: 2,
            name: None,
        };
        doc.set_style(n7, RendererStyleValue::GridRow(value));
    }
    doc.set_style(n7, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(n7, RendererStyleValue::Width(Length::px(50.0)));
    doc.set_style(n7, RendererStyleValue::Height(Length::px(40.0)));
    doc.set_style(
        n7,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n7, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n7,
        RendererStyleValue::AlignSelf(ItemAlignment::new(ItemPosition::End)),
    );
    doc.set_style(
        n7,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n7);
    doc.set_attribute(n7, "style", "grid-row-start: 2; position: absolute; width: 50px; height: 40px; background: green; align-self: end;");
    let n8 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n8,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n8, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n8, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(
        n8,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n8, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(n8, RendererStyleValue::Width(Length::px(50.0)));
    doc.set_style(n8, RendererStyleValue::Height(Length::px(60.0)));
    doc.set_style(n8, RendererStyleValue::Bottom(Length::px(-60.0)));
    doc.set_style(
        n8,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n8);
    doc.set_attribute(
        n8,
        "style",
        "position: absolute; background: green; width: 50px; height: 60px; bottom: -60px;",
    );
    Ok(doc.into_engine())
}

fn css_break_grid_grid_item_oof_007(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n1, RendererStyleValue::ColumnWidth(None));
    doc.set_style(n1, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(n1, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(n1, RendererStyleValue::ColumnGap(Some(Length::px(0.0))));
    doc.set_style(n1, RendererStyleValue::ColumnFill(ColumnFill::Auto));
    doc.set_style(n1, RendererStyleValue::BackgroundColor(Color::RED));
    doc.set_style(n1, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n1,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(vp, n1);
    doc.set_attribute(n1, "style", "columns: 2; height: 100px; width: 100px; column-gap: 0; column-fill: auto; background: red;");
    let n2 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n2, RendererStyleValue::ListStyleType(ListStyleType::None));
    doc.set_style(n2, RendererStyleValue::MarginTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::MarginLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingTop(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingRight(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingBottom(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::PaddingLeft(Length::px(0.0)));
    doc.set_style(n2, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(n2, RendererStyleValue::Position(Position::Relative));
    doc.set_style(n2, RendererStyleValue::Display(Display::Grid));
    doc.set_style(
        n2,
        RendererStyleValue::GridTemplateRows(GridTrackList::Tracks(vec![
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Length(
                Length::px(50.0),
            ))),
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Auto)),
            GridTrackComponent::Track(GridTrackSize::Breadth(GridTrackBreadth::Length(
                Length::px(50.0),
            ))),
        ])),
    );
    doc.set_style(
        n2,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n1, n2);
    doc.set_attribute(
        n2,
        "style",
        "position: relative; display: grid; grid-template-rows: 50px auto 50px;",
    );
    let n3 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(
        n3,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n3, RendererStyleValue::BackgroundLinearGradient(None));
    doc.set_style(
        n3,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n3);
    doc.set_attribute(n3, "style", "background: green;");
    let n4 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n4, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(
        n4,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n4, RendererStyleValue::BackgroundLinearGradient(None));
    {
        let mut value = doc.computed_style(n4).grid_row.clone();
        value.start = GridLine::Line {
            index: 2,
            name: None,
        };
        doc.set_style(n4, RendererStyleValue::GridRow(value));
    }
    doc.set_style(n4, RendererStyleValue::Top(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::Right(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::Bottom(Length::px(0.0)));
    doc.set_style(n4, RendererStyleValue::Left(Length::px(0.0)));
    doc.set_style(
        n4,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n4);
    doc.set_attribute(
        n4,
        "style",
        "position: absolute; background: green; grid-row-start: 2; inset: 0;",
    );
    let n5 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(n5, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(
        n5,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.set_style(n5, RendererStyleValue::BackgroundLinearGradient(None));
    {
        let mut value = doc.computed_style(n5).grid_row.clone();
        value.start = GridLine::Line {
            index: 3,
            name: None,
        };
        doc.set_style(n5, RendererStyleValue::GridRow(value));
    }
    doc.set_style(n5, RendererStyleValue::Top(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::Right(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::Bottom(Length::px(0.0)));
    doc.set_style(n5, RendererStyleValue::Left(Length::px(0.0)));
    doc.set_style(
        n5,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n3, n5);
    doc.set_attribute(
        n5,
        "style",
        "position: absolute; background: green; grid-row-start: 3; inset: 0;",
    );
    let n6 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
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
    doc.set_style(
        n6,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n2, n6);
    let n7 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n7,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n7, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n7, RendererStyleValue::BreakInside(BreakInside::Avoid));
    doc.set_style(n7, RendererStyleValue::Height(Length::px(20.0)));
    doc.set_style(
        n7,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n6, n7);
    doc.set_attribute(n7, "style", "break-inside: avoid; height: 20px;");
    let n8 = doc.create_node(ElementTag::Div);
    doc.set_style(
        n8,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.set_style(n8, RendererStyleValue::ListStyleType(ListStyleType::None));
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
    doc.set_style(n8, RendererStyleValue::BreakInside(BreakInside::Avoid));
    doc.set_style(n8, RendererStyleValue::Height(Length::px(50.0)));
    doc.set_style(
        n8,
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("Droid Sans Fallback".to_string()),
                FontFamily::Named("Noto Sans Devanagari".to_string()),
                FontFamily::Named("Noto Color Emoji".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
    );
    doc.append_child(n6, n8);
    doc.set_attribute(n8, "style", "break-inside: avoid; height: 50px;");
    Ok(doc.into_engine())
}

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn nodes(e: &Engine) -> (NodeHandle, Vec<NodeHandle>) {
    let body = e.children(e.root()).unwrap()[0];
    let outer = e.children(body).unwrap()[0];
    let grid = e.children(outer).unwrap()[0];
    (grid, e.children(grid).unwrap())
}

fn projected<'a>(
    scene: &'a SceneSnapshot,
    node: NodeHandle,
) -> Vec<(Vec<usize>, &'a Fragment, PhysicalOffset)> {
    let owner = SceneNodeIdentity {
        document: node.document_id(),
        index: node.index(),
        generation: node.generation(),
    };
    let mut result = Vec::new();
    for layer in scene
        .retained_scene()
        .unwrap()
        .layers()
        .iter()
        .filter(|layer| layer.owner == owner)
    {
        for projection in layer.projections.iter() {
            let mut f = scene.fragments().as_ref();
            let mut absolute = f.offset;
            for &index in projection.fragment_path.iter() {
                f = &f.children[index];
                absolute.left += f.offset.left;
                absolute.top += f.offset.top;
            }
            result.push((projection.fragment_path.to_vec(), f, absolute));
        }
    }
    result
}

fn assert_owned_grid_resume(
    e: &mut Engine,
    grid: NodeHandle,
    oof: NodeHandle,
    starts: &[f64],
    final_top: f64,
) {
    let scene = e.scene().unwrap();
    let grid_parts = projected(&scene, grid);
    let native_grid = grid_parts[0].1.node_id;
    let mut observed: Vec<f64> = grid_parts
        .iter()
        .map(|(_, f, _)| f.decoration_slice.unwrap().source_block_offset.to_f64())
        .collect();
    observed.sort_by(f64::total_cmp);
    assert_eq!(
        observed, starts,
        "actual normal Grid consumed source intervals"
    );
    let parts = projected(&scene, oof);
    assert_eq!(
        parts.len(),
        2,
        "OOF resumes with both real containing-block pieces"
    );
    let mut last_top = None;
    for (path, f, absolute) in parts {
        let mut parent = scene.fragments().as_ref();
        let mut belongs_to_grid = false;
        for &index in path.iter().take(path.len() - 1) {
            parent = &parent.children[index];
            belongs_to_grid |= parent.node_id == native_grid;
        }
        assert!(
            belongs_to_grid,
            "represented OOF must retain its Grid source owner"
        );
        assert!(
            f.decoration_slice.is_none(),
            "no independent nominal-column source slicing"
        );
        assert!(f
            .positioned_fragmentation
            .unwrap()
            .fragmentainer_index
            .is_none());
        if absolute.left.to_f64() == 70.0 {
            last_top = Some(absolute.top.to_f64());
        }
    }
    assert_eq!(
        last_top,
        Some(final_top),
        "actual later CB source origin, not nominal capacity"
    );
    let generation = scene.generation();
    let stats = e.stats();
    let dirty = e.dirty_generations();
    assert_eq!(e.scene().unwrap().generation(), generation);
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
}

#[test]
fn real_grid_avoid_breaks_keep_grid_area_positioned_source_at_actual_consumed_origins() {
    for scale in SCALES {
        let vp = ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap();
        let mut e = css_break_grid_grid_item_009(vp).unwrap();
        let (grid, children) = nodes(&e);
        assert_owned_grid_resume(&mut e, grid, children[1], &[0.0, 70.0], 90.0);
        let mut e = css_break_grid_grid_item_oof_005(vp).unwrap();
        let (grid, children) = nodes(&e);
        assert_owned_grid_resume(&mut e, grid, children[3], &[0.0, 90.0], 60.0);
        let mut e = css_break_grid_grid_item_oof_007(vp).unwrap();
        let (grid, children) = nodes(&e);
        let oof = e.children(children[0]).unwrap()[1];
        assert_owned_grid_resume(&mut e, grid, oof, &[0.0, 70.0], 70.0);
    }
}

#[test]
fn genuine_positioned_source_beyond_real_grid_windows_keeps_independent_continuation() {
    for scale in SCALES {
        let mut e = css_break_grid_grid_item_009(
            ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
        )
        .unwrap();
        let (_, children) = nodes(&e);
        let oof = children[1];
        e.set_renderer_style(oof, RendererStyleValue::Height(Length::px(350.0)))
            .unwrap();
        let scene = e.scene().unwrap();
        let mut intervals: Vec<(f64, f64)> = projected(&scene, oof)
            .into_iter()
            .map(|(_, f, _)| {
                let slice = f
                    .decoration_slice
                    .expect("actual beyond-window OOF source slices");
                assert_eq!(slice.source_block_size.to_f64(), 350.0);
                assert!(f
                    .positioned_fragmentation
                    .unwrap()
                    .fragmentainer_index
                    .is_some());
                (slice.source_block_offset.to_f64(), f.size.height.to_f64())
            })
            .collect();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut end = 0.0;
        for (start, height) in intervals {
            assert_eq!(start, end);
            assert!(height > 0.0);
            end += height;
        }
        assert_eq!(
            end, 350.0,
            "whole genuinely unrepresented OOF source remains owned once"
        );
    }
}
