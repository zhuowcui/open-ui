//! Pending offline lowering runs real typed Engine mutations, never scripts.
//! These source guards do not admit the corresponding Chromium pixel fixtures.
use openui_engine::{Engine, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    parse_literal, value_from_computed, Color, Display, Position, ScrollbarWidth, StyleProperty,
};

#[path = "../../pixel-compare/src/pending_native_mutation_v1.rs"]
mod replay;

fn engine(scale: f64) -> Engine {
    Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
    )
    .unwrap()
}

#[test]
fn prepend_after_layout_keeps_live_identity_order_and_stylesheet_child_declarations() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = engine(scale);
        let result = replay::replay_0(&mut engine).unwrap();
        let parent = result.handles["abscb"];
        let inserted = result.handles["binding:newDiv"];
        let children = engine.children(parent).unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0], inserted);
        assert_eq!(engine.parent(inserted).unwrap(), Some(parent));
        let style = engine.computed_style(inserted).unwrap();
        assert_eq!(style.width, Length::percent(100.0));
        assert_eq!(style.height.value(), 60.0);
        assert_eq!(style.background_color, Color::GREEN);
        assert_eq!(style.scrollbar_width, ScrollbarWidth::None);
        assert_eq!(result.barriers.len(), 1);
        assert_eq!(result.barriers[0].0, 0);
        assert_eq!(result.barriers[0].1.layouts, 1);
        engine.update().unwrap();
        assert_eq!(engine.stats().layouts, 2);
        let idle = engine.stats();
        engine.update().unwrap();
        assert_eq!(engine.stats(), idle);
    }
}

#[test]
fn empty_display_assignment_removes_source_and_preserves_other_live_declarations() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = engine(scale);
        let result = replay::replay_1(&mut engine).unwrap();
        let target = result.handles["target"];
        let style = engine.computed_style(target).unwrap();
        assert_eq!(style.display, Display::Block);
        assert_eq!(style.position, Position::Relative);
        assert!(!engine
            .remove_property(target, StyleProperty::Display)
            .unwrap());
        assert_eq!(result.barriers.len(), 1);
        assert_eq!(result.barriers[0].0, 1);
        let fixed = engine.children(target).unwrap()[1];
        assert_eq!(engine.client_rects(fixed).unwrap().len(), 1);
        assert_eq!(
            engine.computed_style(fixed).unwrap().background_color,
            Color::BLUE
        );
    }
}

#[test]
fn two_layout_reads_flush_initial_and_intermediate_transform_before_final_assignment() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = engine(scale);
        let result = replay::replay_2(&mut engine).unwrap();
        assert_eq!(
            result.barriers.iter().map(|b| b.0).collect::<Vec<_>>(),
            [0, 2]
        );
        assert_eq!(result.barriers[0].1.layouts, 1);
        assert!(result.barriers[1].1.paints > result.barriers[0].1.paints);
        let target = result.handles["target"];
        assert_eq!(
            value_from_computed(
                engine.computed_style(target).unwrap(),
                StyleProperty::Transform
            ),
            parse_literal(StyleProperty::Transform, "scale(4.0)").unwrap()
        );
        let container = result.handles["container"];
        assert_eq!(
            engine.computed_style(container).unwrap().scrollbar_width,
            ScrollbarWidth::None
        );
        // scale(4) keeps the 100x50 reference center (50,25): right=250,
        // bottom=125. LTR excludes negative overflow; client height is 150.
        assert_eq!(engine.scroll_width(container).unwrap(), 250);
        assert_eq!(engine.scroll_height(container).unwrap(), 150);
        let idle = engine.stats();
        engine.update().unwrap();
        assert_eq!(engine.stats(), idle);
    }
}
