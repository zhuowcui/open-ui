//! Semantic owned ancestry is checked before tree/focus/scene mutation.
use super::*;
use openui_style::{Color, Display, LengthValue};

fn engine(scale: f64) -> Engine {
    Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap()
}

fn node(engine: &mut Engine, parent: NodeHandle, tag: ElementTag) -> NodeHandle {
    let node = engine.create_native_element(tag).unwrap();
    engine.append_child(parent, node).unwrap();
    node
}

fn bindings(
    engine: &Engine,
) -> Vec<(
    NodeHandle,
    NodeHandle,
    ScrollButtonDirection,
    u64,
    ScrollButtonState,
)> {
    let mut result: Vec<_> = engine
        .scroll_buttons
        .bindings
        .values()
        .map(|b| (b.button, b.origin, b.direction, b.order, b.state))
        .collect();
    result.sort_by_key(|value| (value.0.index, value.0.generation));
    result
}

fn reject_without_mutation(
    engine: &mut Engine,
    parent: NodeHandle,
    origin: NodeHandle,
    before: NodeHandle,
    observed: &[NodeHandle],
) {
    let generation = engine.scene().unwrap().generation();
    engine.drain_scroll_events();
    engine.drain_animation_events();
    let tree: Vec<_> = observed
        .iter()
        .map(|h| (*h, engine.parent(*h).unwrap(), engine.children(*h).unwrap()))
        .collect();
    let styles: Vec<_> = observed
        .iter()
        .map(|h| {
            let s = engine.computed_style(*h).unwrap();
            (s.display, s.color, s.width)
        })
        .collect();
    let prior_bindings = bindings(engine);
    let stats = engine.stats();
    let dirty = engine.dirty_generations();
    let objects = engine.object_counts();
    let focused = engine.focused();
    let capture = engine.pointer_capture(7);
    assert_eq!(
        engine.validate_append_or_move_child(parent, origin),
        Err(EngineError::Cycle)
    );
    assert_eq!(
        engine.validate_insert_before(parent, origin, before),
        Err(EngineError::Cycle)
    );
    assert_eq!(
        engine.append_or_move_child(parent, origin),
        Err(EngineError::Cycle)
    );
    assert_eq!(
        engine.insert_before(parent, origin, before),
        Err(EngineError::Cycle)
    );
    let append_error = if engine.parent(origin).unwrap().is_none() {
        EngineError::Cycle
    } else {
        EngineError::AlreadyAttached
    };
    assert_eq!(engine.append_child(parent, origin), Err(append_error));
    for (i, (handle, parent, children)) in tree.into_iter().enumerate() {
        assert_eq!(engine.parent(handle).unwrap(), parent);
        assert_eq!(engine.children(handle).unwrap(), children);
        let s = engine.computed_style(handle).unwrap();
        assert_eq!((s.display, s.color, s.width), styles[i]);
    }
    assert_eq!(bindings(engine), prior_bindings);
    assert_eq!(engine.stats(), stats);
    assert_eq!(engine.dirty_generations(), dirty);
    assert_eq!(engine.object_counts(), objects);
    assert_eq!(engine.focused(), focused);
    assert_eq!(engine.pointer_capture(7), capture);
    assert!(engine.drain_scroll_events().is_empty());
    assert!(engine.drain_animation_events().is_empty());
    assert_eq!(engine.scene().unwrap().generation(), generation);
    assert_eq!(engine.stats(), stats);
}

#[test]
fn origin_cannot_move_under_its_owned_button_or_descendant_in_any_presentation_state() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for presentation in [0, 1, 2] {
            let mut e = engine(scale);
            let root = e.root();
            let origin = node(&mut e, root, ElementTag::Div);
            e.set_property(origin, StyleProperty::Width, LengthValue::px(90.0).into())
                .unwrap();
            e.set_property(origin, StyleProperty::Color, Color::RED.into())
                .unwrap();
            let button = e
                .create_scroll_button(origin, ScrollButtonDirection::Down)
                .unwrap();
            let descendant = node(&mut e, button, ElementTag::Span);
            let marker = node(&mut e, descendant, ElementTag::Span);
            let focus = node(&mut e, root, ElementTag::Button);
            e.focus(focus).unwrap();
            e.set_pointer_capture(7, focus).unwrap();
            match presentation {
                1 => e
                    .set_property(origin, StyleProperty::Display, Display::None.into())
                    .unwrap(),
                2 => e.detach(origin).unwrap(),
                _ => {}
            }
            e.scene().unwrap();
            if presentation != 0 {
                assert_eq!(e.parent(button).unwrap(), None);
            }
            let observed = [root, origin, button, descendant, marker, focus];
            reject_without_mutation(&mut e, button, origin, descendant, &observed);
            reject_without_mutation(&mut e, descendant, origin, marker, &observed);
            assert_eq!(
                e.scroll_button(origin, ScrollButtonDirection::Down)
                    .unwrap(),
                Some(button)
            );
        }
    }
}

#[test]
fn nested_origins_do_not_allow_indirect_owned_cycles() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut e = engine(scale);
        let root = e.root();
        let outer = node(&mut e, root, ElementTag::Div);
        let outer_button = e
            .create_scroll_button(outer, ScrollButtonDirection::Right)
            .unwrap();
        let outer_child = node(&mut e, outer_button, ElementTag::Span);
        let inner = node(&mut e, outer_child, ElementTag::Div);
        let inner_button = e
            .create_scroll_button(inner, ScrollButtonDirection::Down)
            .unwrap();
        let inner_child = node(&mut e, inner_button, ElementTag::Span);
        let marker = node(&mut e, inner_child, ElementTag::Span);
        reject_without_mutation(
            &mut e,
            inner_child,
            outer,
            marker,
            &[
                root,
                outer,
                outer_button,
                outer_child,
                inner,
                inner_button,
                inner_child,
                marker,
            ],
        );
    }
}

#[test]
fn shared_physical_and_owned_ancestry_allows_valid_diamonds_and_unrelated_moves() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut e = engine(scale);
        let root = e.root();
        let origin = node(&mut e, root, ElementTag::Div);
        let button = e
            .create_scroll_button(origin, ScrollButtonDirection::Down)
            .unwrap();
        let holder = node(&mut e, button, ElementTag::Span);
        let candidate = e.create_native_element(ElementTag::Div).unwrap();
        assert_eq!(e.validate_append_or_move_child(holder, candidate), Ok(()));
        e.append_child(holder, candidate).unwrap();
        let other = node(&mut e, root, ElementTag::Div);
        e.append_or_move_child(other, candidate).unwrap();
        let marker = node(&mut e, holder, ElementTag::Span);
        e.insert_before(holder, candidate, marker).unwrap();
        assert_eq!(e.children(holder).unwrap(), vec![candidate, marker]);
        e.insert_before(holder, candidate, candidate).unwrap();
        assert_eq!(e.children(holder).unwrap(), vec![candidate, marker]);
        // A root-owned button has two identical root routes, not a cycle.
        let root_button = e
            .create_scroll_button(root, ScrollButtonDirection::Right)
            .unwrap();
        let root_child = e.create_native_element(ElementTag::Span).unwrap();
        e.append_child(root_button, root_child).unwrap();
        assert_eq!(e.parent(root_child).unwrap(), Some(root_button));
        assert_eq!(
            e.scroll_button(origin, ScrollButtonDirection::Down)
                .unwrap(),
            Some(button)
        );
    }
}

#[test]
fn ordinary_cycles_stale_generations_and_wrong_documents_still_fail_before_work() {
    let mut e = engine(1.0);
    let root = e.root();
    let parent = node(&mut e, root, ElementTag::Div);
    let child = node(&mut e, parent, ElementTag::Div);
    let marker = node(&mut e, child, ElementTag::Span);
    reject_without_mutation(
        &mut e,
        child,
        parent,
        marker,
        &[root, parent, child, marker],
    );
    let removed = e.create_native_element(ElementTag::Div).unwrap();
    e.remove(removed).unwrap();
    let replacement = e.create_native_element(ElementTag::Div).unwrap();
    assert_eq!(replacement.index(), removed.index());
    e.scene().unwrap();
    let stats = e.stats();
    let dirty = e.dirty_generations();
    assert_eq!(
        e.validate_append_or_move_child(removed, replacement),
        Err(EngineError::StaleHandle)
    );
    assert_eq!(
        e.append_child(removed, replacement),
        Err(EngineError::StaleHandle)
    );
    assert_eq!(
        e.insert_before(child, removed, marker),
        Err(EngineError::StaleHandle)
    );
    let other = engine(1.0);
    assert_eq!(
        e.validate_append_or_move_child(other.root(), replacement),
        Err(EngineError::WrongDocument)
    );
    assert_eq!(
        e.append_child(other.root(), replacement),
        Err(EngineError::WrongDocument)
    );
    assert_eq!(e.stats(), stats);
    assert_eq!(e.dirty_generations(), dirty);
    assert_eq!(e.parent(replacement).unwrap(), None);
}

#[test]
fn corrupted_origin_generation_registry_or_ancestor_cycles_fail_closed_read_only() {
    let mut e = engine(1.0);
    let root = e.root();
    let origin = node(&mut e, root, ElementTag::Div);
    let button = e
        .create_scroll_button(origin, ScrollButtonDirection::Down)
        .unwrap();
    let holder = node(&mut e, button, ElementTag::Span);
    let candidate = e.create_native_element(ElementTag::Div).unwrap();
    let removed = e.create_native_element(ElementTag::Div).unwrap();
    e.remove(removed).unwrap();
    let original = e.scroll_buttons.bindings[&button];
    let dirty = e.dirty_generations();
    let stats = e.stats();
    e.scroll_buttons.bindings.get_mut(&button).unwrap().origin = removed;
    assert_eq!(
        e.validate_append_or_move_child(holder, candidate),
        Err(EngineError::StaleHandle)
    );
    e.scroll_buttons.bindings.insert(button, original);
    e.scroll_buttons.bindings.get_mut(&button).unwrap().button = origin;
    assert!(matches!(
        e.validate_append_or_move_child(holder, candidate),
        Err(EngineError::InvalidInput(_))
    ));
    e.scroll_buttons.bindings.insert(button, original);
    e.scroll_buttons
        .directions
        .remove(&(origin, original.direction));
    assert!(matches!(
        e.validate_append_or_move_child(holder, candidate),
        Err(EngineError::InvalidInput(_))
    ));
    e.scroll_buttons
        .directions
        .insert((origin, original.direction), button);
    // A malformed live ownership back-edge is detected, not silently deduped.
    e.scroll_buttons.bindings.get_mut(&button).unwrap().origin = holder;
    e.scroll_buttons
        .directions
        .remove(&(origin, original.direction));
    e.scroll_buttons
        .directions
        .insert((holder, original.direction), button);
    assert_eq!(
        e.validate_append_or_move_child(holder, candidate),
        Err(EngineError::Cycle)
    );
    e.scroll_buttons.bindings.insert(button, original);
    e.scroll_buttons
        .directions
        .remove(&(holder, original.direction));
    e.scroll_buttons
        .directions
        .insert((origin, original.direction), button);
    let holder_node = e.resolve(holder).unwrap();
    let physical_parent = e.document.node(holder_node).parent;
    e.document.node_mut(holder_node).parent = holder_node;
    assert_eq!(
        e.validate_append_or_move_child(holder, candidate),
        Err(EngineError::Cycle)
    );
    e.document.node_mut(holder_node).parent = physical_parent;
    assert_eq!(e.validate_append_or_move_child(holder, candidate), Ok(()));
    assert_eq!(e.dirty_generations(), dirty);
    assert_eq!(e.stats(), stats);
    assert_eq!(e.parent(candidate).unwrap(), None);
}
