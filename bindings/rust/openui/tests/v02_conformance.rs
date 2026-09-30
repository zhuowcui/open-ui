use openui::prelude::*;
use openui::{AccessibilityAction, AccessibilityRelation, KeyEventType, MouseEventType};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn document() -> Document {
    Document::new(320, 240).unwrap()
}

fn child(document: &Document, tag: &str) -> Element {
    let element = Element::create(document, tag).unwrap();
    document.body().append_child(&element).unwrap();
    element
}

fn sized(element: &Element, width: f32, height: f32) {
    element.set_display(Display::Block).unwrap();
    element.set_width(LengthValue::px(width)).unwrap();
    element.set_height(LengthValue::px(height)).unwrap();
}

fn animation_options(duration_ms: f64) -> AnimationOptions {
    AnimationOptions {
        duration_ms,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    }
}

#[test]
fn static_headless_scene() {
    let document = document();
    let block = child(&document, "div");
    sized(&block, 80.0, 30.0);
    block.set_background_color(Color::RED).unwrap();
    let bitmap = document.render_to_bitmap().unwrap();
    assert_eq!((bitmap.width(), bitmap.height()), (320, 240));
}

#[test]
fn batched_tree_transaction() {
    let document = document();
    document
        .transaction(|document| {
            for value in 0..32 {
                child(document, "div").set_text(&value.to_string())?;
            }
            Ok(())
        })
        .unwrap();
    document.update_all().unwrap();
    assert!(document.body().first_child().unwrap().is_some());
}

#[test]
fn native_id_lookup_follows_attached_document_order() {
    let document = document();
    let first = child(&document, "div");
    first.set_id("container").unwrap();
    let nested = Element::create(&document, "button").unwrap();
    nested.set_id("action").unwrap();
    first.append_child(&nested).unwrap();
    let later = child(&document, "button");
    later.set_id("action").unwrap();

    let found = document.element_by_id("action").unwrap().unwrap();
    assert_eq!(
        found
            .parent()
            .unwrap()
            .unwrap()
            .get_attribute("id")
            .unwrap()
            .as_deref(),
        Some("container")
    );
    first.remove().unwrap();
    assert!(document.element_by_id("action").unwrap().is_some());
    later.remove().unwrap();
    assert!(document.element_by_id("action").unwrap().is_none());
}

#[test]
fn native_detach_preserves_subtree_handles_and_listeners_for_reattachment() {
    let document = document();
    let container = child(&document, "div");
    container.set_id("container").unwrap();
    let button = Element::create(&document, "button").unwrap();
    button.set_id("action").unwrap();
    container.append_child(&button).unwrap();
    let clicks = Rc::new(Cell::new(0));
    let seen = clicks.clone();
    button
        .on("click", move |_| seen.set(seen.get() + 1))
        .unwrap();
    button.focus().unwrap();
    assert!(document.focused_element().unwrap().is_some());

    container.detach().unwrap();
    assert!(container.parent().unwrap().is_none());
    assert!(document.element_by_id("action").unwrap().is_none());
    assert!(document.focused_element().unwrap().is_none());
    assert_eq!(
        button.parent().unwrap().unwrap().kind().unwrap(),
        ElementTag::Div
    );

    container.set_width(LengthValue::px(70.0)).unwrap();
    document.body().append_child(&container).unwrap();
    assert!(document.element_by_id("action").unwrap().is_some());
    button.click().unwrap();
    assert_eq!(clicks.get(), 1);
    container.remove().unwrap();
    assert!(button.kind().is_err());
}

#[test]
fn native_layout_read_then_id_style_mutation_updates_the_same_document() {
    let document = document();
    let target = child(&document, "div");
    target.set_id("target").unwrap();
    sized(&target, 80.0, 20.0);

    // A Chromium fixture can use a layout read before changing an element's
    // width. Applications perform both operations through retained Rust APIs.
    assert!(document.body().bounding_rect().unwrap().is_some());
    assert_eq!(target.bounding_rect().unwrap().unwrap().width, 80.0);
    let found = document.element_by_id("target").unwrap().unwrap();
    found.set_width(LengthValue::px(50.0)).unwrap();
    assert_eq!(target.bounding_rect().unwrap().unwrap().width, 50.0);
}

#[test]
fn native_geometry_includes_all_column_fragments() {
    let document = document();
    let columns = child(&document, "div");
    columns.set_display(Display::Block).unwrap();
    columns.set_width(LengthValue::px(300.0)).unwrap();
    columns.set_column_count(Some(3)).unwrap();
    columns.set_column_gap(LengthValue::px(24.0)).unwrap();
    let wrapper = Element::create(&document, "div").unwrap();
    wrapper.set_display(Display::Block).unwrap();
    wrapper.set_max_height(LengthValue::px(160.0)).unwrap();
    columns.append_child(&wrapper).unwrap();
    let target = Element::create(&document, "div").unwrap();
    sized(&target, 50.0, 200.0);
    target
        .set_border(Border {
            width: 3.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })
        .unwrap();
    wrapper.append_child(&target).unwrap();
    target.set_accessibility_label("column target").unwrap();

    let bounds = target.bounding_rect().unwrap().unwrap();
    assert_eq!(bounds.x, 0.0);
    assert_eq!(bounds.y, 0.0);
    assert_eq!(bounds.width, 272.0);
    assert_eq!(bounds.height, 68.671875);
    assert_eq!(target.width().unwrap(), 272.0);
    assert_eq!(target.height().unwrap(), 68.671875);
    let tree = document.accessibility_update().unwrap();
    let accessible = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("column target"))
        .unwrap();
    let accessible_bounds = accessible.1.bounds().unwrap();
    assert_eq!(
        (
            accessible_bounds.x0,
            accessible_bounds.y0,
            accessible_bounds.x1,
            accessible_bounds.y1
        ),
        (0.0, 0.0, 272.0, 68.671875)
    );
    let rects = target.client_rects().unwrap();
    assert_eq!(rects.len(), 3);
    assert_eq!(
        rects
            .iter()
            .map(|rect| (rect.x, rect.y, rect.width, rect.height))
            .collect::<Vec<_>>(),
        [
            (0.0, 0.0, 56.0, 68.671875),
            (108.0, 0.0, 56.0, 68.671875),
            (216.0, 0.0, 56.0, 68.65625)
        ]
    );
}

#[test]
fn native_geometry_is_independent_of_pointer_and_visibility() {
    let document = document();
    let target = child(&document, "div");
    sized(&target, 80.0, 30.0);
    let initial = target.bounding_rect().unwrap().unwrap();
    target.set_pointer_events(PointerEvents::None).unwrap();
    assert_eq!(target.bounding_rect().unwrap(), Some(initial));
    assert_eq!(target.client_rects().unwrap(), [initial]);
    target.set_visibility(Visibility::Hidden).unwrap();
    assert_eq!(target.bounding_rect().unwrap(), Some(initial));
    target.set_display(Display::None).unwrap();
    assert!(target.bounding_rect().unwrap().is_none());
    assert!(target.client_rects().unwrap().is_empty());
}

#[test]
fn native_geometry_preserves_empty_and_singular_boxes() {
    let document = document();
    let target = child(&document, "div");
    target.set_id("collapsed").unwrap();
    sized(&target, 0.0, 30.0);
    let bounds = target.bounding_rect().unwrap().unwrap();
    assert_eq!((bounds.width, bounds.height), (0.0, 30.0));
    assert_eq!(target.client_rects().unwrap(), [bounds]);
    sized(&target, 80.0, 30.0);
    target
        .set_transform(TransformList(vec![TransformOperation::Scale(0.0, 1.0)]))
        .unwrap();
    let bounds = target.bounding_rect().unwrap().unwrap();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (40.0, 0.0, 0.0, 30.0)
    );
    assert_eq!(target.client_rects().unwrap(), [bounds]);
    assert!(!document
        .hit_test(40.0, 5.0)
        .unwrap()
        .is_some_and(|hit| { hit.get_attribute("id").unwrap().as_deref() == Some("collapsed") }));
}

#[test]
fn native_geometry_updates_after_scroll_transform_and_detachment() {
    let document = document();
    let scroller = child(&document, "div");
    sized(&scroller, 80.0, 50.0);
    scroller.set_overflow(Overflow::Hidden).unwrap();
    let target = Element::create(&document, "div").unwrap();
    sized(&target, 120.0, 100.0);
    scroller.append_child(&target).unwrap();
    let original = target.client_rects().unwrap();
    assert_eq!(
        (
            original[0].x,
            original[0].y,
            original[0].width,
            original[0].height
        ),
        (0.0, 0.0, 120.0, 100.0)
    );

    scroller.scroll_to(20.0, 30.0).unwrap();
    let scrolled = target.bounding_rect().unwrap().unwrap();
    assert_eq!(
        (scrolled.x, scrolled.y, scrolled.width, scrolled.height),
        (-20.0, -30.0, 120.0, 100.0)
    );
    target
        .set_transform(TransformList(vec![TransformOperation::Translate(
            LengthValue::px(15.0),
            LengthValue::px(7.0),
        )]))
        .unwrap();
    let transformed = target.bounding_rect().unwrap().unwrap();
    assert_eq!(
        (
            transformed.x,
            transformed.y,
            transformed.width,
            transformed.height
        ),
        (-5.0, -23.0, 120.0, 100.0)
    );
    assert_eq!((original[0].x, original[0].y), (0.0, 0.0));

    target.detach().unwrap();
    assert!(target.client_rects().unwrap().is_empty());
    assert!(target.bounding_rect().unwrap().is_none());
    scroller.append_child(&target).unwrap();
    assert_eq!(target.client_rects().unwrap(), [transformed]);
    target.remove().unwrap();
    assert!(target.client_rects().is_err());
}

#[test]
fn native_class_lookup_tracks_event_driven_updates_and_detachment() {
    let document = document();
    let first = child(&document, "div");
    first.set_id("first").unwrap();
    first.set_class(" active\tprimary ").unwrap();
    let second = child(&document, "div");
    second.set_id("second").unwrap();
    let button = child(&document, "button");
    let second_for_callback = second.clone();
    button
        .on("click", move |_| {
            assert!(second_for_callback.add_class("active").unwrap());
        })
        .unwrap();

    assert!(first.has_class("active").unwrap());
    assert!(!second.has_class("active").unwrap());
    button.click().unwrap();
    assert!(!second.add_class("active").unwrap());
    let ids: Vec<_> = document
        .elements_with_class("active")
        .unwrap()
        .into_iter()
        .map(|element| element.get_attribute("id").unwrap().unwrap())
        .collect();
    assert_eq!(ids, ["first", "second"]);

    assert!(second.remove_class("active").unwrap());
    assert!(!second.remove_class("active").unwrap());
    assert!(document.elements_with_class("active token").is_err());
    first.remove().unwrap();
    assert!(document.elements_with_class("active").unwrap().is_empty());
}

#[test]
fn native_kind_lookup_tracks_tree_order_and_detachment() {
    let document = document();
    let first = child(&document, "div");
    first.set_id("first").unwrap();
    let nested = Element::create(&document, "textarea").unwrap();
    first.append_child(&nested).unwrap();
    let later = child(&document, "div");
    later.set_id("later").unwrap();
    let detached = Element::create(&document, "div").unwrap();
    let _text = first.create_text_child("not an element").unwrap();

    assert_eq!(nested.kind().unwrap(), ElementTag::TextArea);
    let ids: Vec<_> = document
        .elements_of_kind(ElementTag::Div)
        .unwrap()
        .into_iter()
        .map(|element| element.get_attribute("id").unwrap().unwrap())
        .collect();
    assert_eq!(ids, ["first", "later"]);
    assert!(document
        .elements_of_kind(ElementTag::Text)
        .unwrap()
        .is_empty());
    assert_eq!(
        document
            .elements_of_kind(ElementTag::TextArea)
            .unwrap()
            .len(),
        1
    );

    first.remove().unwrap();
    assert_eq!(
        document
            .elements_of_kind(ElementTag::TextArea)
            .unwrap()
            .len(),
        0
    );
    assert_eq!(document.elements_of_kind(ElementTag::Div).unwrap().len(), 1);
    assert!(nested.kind().is_err());
    assert_eq!(detached.kind().unwrap(), ElementTag::Div);
}

#[test]
fn native_text_nodes_attach_move_and_keep_element_traversal_typed() {
    let document = document();
    let container = child(&document, "div");
    let first = Element::create(&document, "span").unwrap();
    first.set_id("first").unwrap();
    container.append_child(&first).unwrap();
    let second = Element::create(&document, "span").unwrap();
    second.set_id("second").unwrap();
    container.append_child(&second).unwrap();

    let leading_text = document.create_text_node("before").unwrap();
    container.insert_text_before(&leading_text, &first).unwrap();
    let mut middle_text = document.create_text_node("middle").unwrap();
    container.insert_text_before(&middle_text, &second).unwrap();
    assert_eq!(leading_text.data().unwrap(), "before");
    assert_eq!(container.text_content().unwrap(), "beforemiddle");
    assert_eq!(
        container
            .first_child()
            .unwrap()
            .unwrap()
            .get_attribute("id")
            .unwrap()
            .as_deref(),
        Some("first")
    );
    assert_eq!(
        first
            .next_sibling()
            .unwrap()
            .unwrap()
            .get_attribute("id")
            .unwrap()
            .as_deref(),
        Some("second")
    );
    assert!(second.next_sibling().unwrap().is_none());

    let before = document.render_to_bitmap().unwrap();
    leading_text.set_data("a longer native text node").unwrap();
    assert_eq!(
        container.text_content().unwrap(),
        "a longer native text nodemiddle"
    );
    assert_ne!(
        before.pixels(),
        document.render_to_bitmap().unwrap().pixels()
    );

    let other = child(&document, "div");
    other.set_id("text-parent").unwrap();
    other.append_text_child(&middle_text).unwrap();
    assert_eq!(middle_text.data().unwrap(), "middle");
    assert_eq!(other.text_content().unwrap(), "middle");
    assert!(other.first_child().unwrap().is_none());
    assert_eq!(
        middle_text
            .parent()
            .unwrap()
            .unwrap()
            .get_attribute("id")
            .unwrap()
            .as_deref(),
        Some("text-parent")
    );
    middle_text.detach().unwrap();
    assert!(middle_text.parent().unwrap().is_none());
    assert_eq!(other.text_content().unwrap(), "");
    middle_text.set_data("reattached").unwrap();
    container.append_text_child(&middle_text).unwrap();
    assert_eq!(
        container.text_content().unwrap(),
        "a longer native text nodereattached"
    );
    let foreign_document = Document::new(320, 240).unwrap();
    let foreign = foreign_document.create_text_node("foreign").unwrap();
    assert!(container.append_text_child(&foreign).is_err());
}

#[test]
fn computed_style_is_an_owned_native_snapshot() {
    let document = document();
    let element = child(&document, "div");
    element.set_width(LengthValue::px(42.0)).unwrap();
    let original = element.computed_style().unwrap();

    element.set_width(LengthValue::px(84.0)).unwrap();
    let updated = element.computed_style().unwrap();
    assert_ne!(original.width, updated.width);
    assert_eq!(original.width, Length::px(42.0));
    assert_eq!(updated.width, Length::px(84.0));

    element.remove().unwrap();
    assert!(element.computed_style().is_err());
}

#[test]
fn removed_weak_handle_expires() {
    let document = document();
    let element = child(&document, "div");
    let weak = element.downgrade();
    element.remove().unwrap();
    assert!(weak.upgrade().is_none());
}

#[test]
fn cross_document_ownership_is_rejected() {
    let first = document();
    let second = document();
    let foreign = child(&second, "div");
    assert!(first.body().append_child(&foreign).is_err());
}

#[test]
fn large_list_updates_remain_mutable() {
    let document = document();
    let list = child(&document, "div");
    for value in 0..256 {
        let row = Element::create(&document, "span").unwrap();
        row.set_text(&format!("row {value}")).unwrap();
        list.append_child(&row).unwrap();
    }
    list.remove_all_children().unwrap();
    assert!(list.first_child().unwrap().is_none());
}

#[test]
fn button_pointer_activation_dispatches_click() {
    let document = document();
    let button = child(&document, "button");
    sized(&button, 80.0, 40.0);
    let clicked = Rc::new(Cell::new(false));
    let observed = clicked.clone();
    button.on("click", move |_| observed.set(true)).unwrap();
    document.update_all().unwrap();
    document
        .dispatch_mouse_event(
            MouseEventType::Down,
            10.0,
            10.0,
            MouseButton::Left,
            Modifiers::NONE,
        )
        .unwrap();
    document
        .dispatch_mouse_event(
            MouseEventType::Up,
            10.0,
            10.0,
            MouseButton::Left,
            Modifiers::NONE,
        )
        .unwrap();
    assert!(clicked.get());
}

#[test]
fn checkbox_keyboard_activation_toggles() {
    let document = document();
    let checkbox = child(&document, "input");
    checkbox.set_attribute("type", "checkbox").unwrap();
    checkbox.focus().unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 32, Some(" "), Modifiers::NONE)
        .unwrap();
    assert!(checkbox.is_checked().unwrap());
}

#[test]
fn named_radio_group_is_exclusive() {
    let document = document();
    let first = child(&document, "input");
    let second = child(&document, "input");
    for radio in [&first, &second] {
        radio.set_attribute("type", "radio").unwrap();
        radio.set_attribute("name", "choice").unwrap();
    }
    first.set_checked(true).unwrap();
    second.set_checked(true).unwrap();
    assert!(!first.is_checked().unwrap());
    assert!(second.is_checked().unwrap());
}

#[test]
fn unicode_grapheme_editing() {
    let document = document();
    let input = child(&document, "input");
    input.set_control_value("a👨‍👩‍👧‍👦").unwrap();
    input.focus().unwrap();
    input.set_selection("a👨‍👩‍👧‍👦".len(), "a👨‍👩‍👧‍👦".len()).unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 8, Some("Backspace"), Modifiers::NONE)
        .unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("a"));
}

#[test]
fn textarea_accepts_multiline_text() {
    let document = document();
    let textarea = child(&document, "textarea");
    textarea.focus().unwrap();
    document.dispatch_text_input("one\ntwo").unwrap();
    assert_eq!(
        textarea.control_value().unwrap().as_deref(),
        Some("one\ntwo")
    );

    textarea.set_control_value("é👍z").unwrap();
    textarea.set_selection(2, 6).unwrap();
    let edits = Rc::new(RefCell::new(Vec::new()));
    for event_type in ["beforeinput", "input"] {
        let edits = edits.clone();
        textarea
            .on(event_type, move |event| {
                edits
                    .borrow_mut()
                    .push((event.event_type.clone(), event.key_text.clone()));
            })
            .unwrap();
    }
    let clicks = Rc::new(Cell::new(0));
    let seen_clicks = clicks.clone();
    textarea
        .on("click", move |_| seen_clicks.set(seen_clicks.get() + 1))
        .unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 13, Some("Enter"), Modifiers::NONE)
        .unwrap();
    assert_eq!(textarea.control_value().unwrap().as_deref(), Some("é\nz"));
    assert_eq!(textarea.selection().unwrap(), Some((3, 3)));
    assert_eq!(
        &*edits.borrow(),
        &[
            ("beforeinput".to_owned(), "\n".to_owned()),
            ("input".to_owned(), "\n".to_owned())
        ]
    );
    assert_eq!(clicks.get(), 0);

    document
        .dispatch_key_event(KeyEventType::Down, 90, Some("z"), Modifiers::CTRL)
        .unwrap();
    assert_eq!(textarea.control_value().unwrap().as_deref(), Some("é👍z"));
    textarea
        .on("keydown", |event| event.prevent_default())
        .unwrap();
    let event_count = edits.borrow().len();
    document
        .dispatch_key_event(KeyEventType::Down, 13, Some("Enter"), Modifiers::NONE)
        .unwrap();
    assert_eq!(textarea.control_value().unwrap().as_deref(), Some("é👍z"));
    assert_eq!(edits.borrow().len(), event_count);
    textarea.remove_event("keydown").unwrap();
    textarea
        .on("beforeinput", |event| event.prevent_default())
        .unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 13, Some("Enter"), Modifiers::NONE)
        .unwrap();
    assert_eq!(textarea.control_value().unwrap().as_deref(), Some("é👍z"));
    assert_eq!(edits.borrow().len(), event_count + 1);
    textarea.remove_event("beforeinput").unwrap();

    let input = child(&document, "input");
    let callback_input = input.clone();
    textarea
        .on("keydown", move |_| callback_input.focus().unwrap())
        .unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 13, Some("Enter"), Modifiers::NONE)
        .unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some(""));
    assert_eq!(textarea.control_value().unwrap().as_deref(), Some("é👍z"));
    textarea.remove_event("keydown").unwrap();
    input.focus().unwrap();
    let seen_clicks = clicks.clone();
    input
        .on("click", move |_| seen_clicks.set(seen_clicks.get() + 1))
        .unwrap();
    for (code, text) in [(13, "Enter"), (32, " ")] {
        document
            .dispatch_key_event(KeyEventType::Down, code, Some(text), Modifiers::NONE)
            .unwrap();
    }
    assert_eq!(input.control_value().unwrap().as_deref(), Some(""));
    assert_eq!(clicks.get(), 0);
}

#[test]
fn read_only_text_controls_keep_selection_and_reject_native_edits() {
    for tag in ["input", "textarea"] {
        let document = document();
        let target = child(&document, tag);
        target.set_control_value("original").unwrap();
        target.set_selection(0, 8).unwrap();
        target.set_attribute("readonly", "").unwrap();
        target.set_accessibility_label("readonly target").unwrap();
        target.focus().unwrap();
        let input_events = Rc::new(RefCell::new(Vec::new()));
        for event_type in ["beforeinput", "input"] {
            let seen = input_events.clone();
            target
                .on(event_type, move |event| {
                    seen.borrow_mut()
                        .push((event.event_type.clone(), event.key_text.clone()))
                })
                .unwrap();
        }
        document.dispatch_text_input("replacement").unwrap();
        document
            .dispatch_key_event(KeyEventType::Char, 0, Some("typed"), Modifiers::NONE)
            .unwrap();
        document
            .dispatch_key_event(KeyEventType::Down, 8, Some("Backspace"), Modifiers::NONE)
            .unwrap();
        document
            .dispatch_key_event(KeyEventType::Down, 13, Some("Enter"), Modifiers::NONE)
            .unwrap();
        document.dispatch_composition_start().unwrap();
        document.dispatch_composition_update("preview").unwrap();
        document.dispatch_composition_end("committed").unwrap();
        assert_eq!(target.control_value().unwrap().as_deref(), Some("original"));
        assert!(
            input_events.borrow().is_empty(),
            "{tag}: {:?}",
            input_events.borrow()
        );
        assert!(target
            .perform_accessibility_action(AccessibilityAction::SetValue("blocked".into()))
            .is_err());
        assert!(target
            .perform_accessibility_action(AccessibilityAction::ReplaceSelectedText(
                "blocked".into()
            ))
            .is_err());
        assert_eq!(target.control_value().unwrap().as_deref(), Some("original"));
        let tree = document.accessibility_update().unwrap();
        let node = &tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("readonly target"))
            .unwrap()
            .1;
        assert!(node.is_read_only());
        assert!(!node.supports_action(openui::AccessibilityPlatformAction::SetValue));
        assert!(!node.supports_action(openui::AccessibilityPlatformAction::ReplaceSelectedText));
        assert!(node.supports_action(openui::AccessibilityPlatformAction::SetTextSelection));
        document
            .dispatch_key_event(KeyEventType::Down, 65, Some("a"), Modifiers::CTRL)
            .unwrap();
        assert_eq!(target.selection().unwrap(), Some((0, 8)));

        // Explicit native application writes remain available in read-only mode.
        target.set_control_value("application").unwrap();
        assert_eq!(
            target.control_value().unwrap().as_deref(),
            Some("application")
        );
        target.set_selection(11, 11).unwrap();
        target.remove_attribute("readonly").unwrap();
        document
            .dispatch_key_event(KeyEventType::Char, 0, Some("!"), Modifiers::NONE)
            .unwrap();
        assert_eq!(
            target.control_value().unwrap().as_deref(),
            Some("application!")
        );
        assert_eq!(input_events.borrow().len(), 2);
    }
}

#[test]
fn ime_composition_commits_once() {
    let document = document();
    let input = child(&document, "input");
    input.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("かな").unwrap();
    document.dispatch_composition_end("かな").unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("かな"));
}

#[test]
fn ime_commits_final_text_after_platform_clears_preview_as_one_undo_step() {
    let document = document();
    let input = child(&document, "input");
    input.set_control_value("leftかなright").unwrap();
    input.set_selection(4, 10).unwrap();
    input.focus().unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    for name in ["beforeinput", "compositionend", "input"] {
        let observed = events.clone();
        let retained = input.clone();
        input
            .on(name, move |event| {
                // Native callbacks can inspect and mutate this same document.
                retained.set_attribute("data-last-event", name).unwrap();
                observed.borrow_mut().push((
                    event.event_type.clone(),
                    event.key_text.clone(),
                    retained.control_value().unwrap().unwrap(),
                ));
            })
            .unwrap();
    }
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("ka").unwrap();
    document.dispatch_composition_update("kanji").unwrap();
    // Winit sends an empty preview immediately before the final commit.
    document.dispatch_composition_update("").unwrap();
    document.dispatch_composition_end("漢字").unwrap();
    assert_eq!(
        input.control_value().unwrap().as_deref(),
        Some("left漢字right")
    );
    assert_eq!(
        events.borrow().as_slice(),
        [
            ("beforeinput".into(), "漢字".into(), "leftright".into()),
            (
                "compositionend".into(),
                "漢字".into(),
                "left漢字right".into()
            ),
            ("input".into(), "漢字".into(), "left漢字right".into()),
        ]
    );
    assert_eq!(input.selection().unwrap(), Some((10, 10)));
    assert!(input
        .get_attribute("data-oui-composition-start")
        .unwrap()
        .is_none());
    document
        .dispatch_key_event(KeyEventType::Down, 90, Some("z"), Modifiers::CTRL)
        .unwrap();
    assert_eq!(
        input.control_value().unwrap().as_deref(),
        Some("leftかなright")
    );
    document
        .dispatch_key_event(
            KeyEventType::Down,
            90,
            Some("z"),
            Modifiers::CTRL | Modifiers::SHIFT,
        )
        .unwrap();
    assert_eq!(
        input.control_value().unwrap().as_deref(),
        Some("left漢字right")
    );
    for name in ["beforeinput", "compositionend", "input"] {
        input.remove_event(name).unwrap();
    }
}

#[test]
fn ime_cancel_restores_selection_pixels_and_existing_redo() {
    let document = document();
    let input = child(&document, "input");
    input.set_control_value("original").unwrap();
    input.focus().unwrap();
    document.dispatch_text_input("!").unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 90, Some("z"), Modifiers::CTRL)
        .unwrap();
    input.set_selection(6, 2).unwrap();
    let before = document.render_to_bitmap().unwrap();
    let inputs = Rc::new(Cell::new(0));
    let observed = inputs.clone();
    input
        .on("input", move |_| observed.set(observed.get() + 1))
        .unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("仮").unwrap();
    document.dispatch_composition_update("preview").unwrap();
    document.dispatch_composition_cancel().unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("original"));
    assert_eq!(input.selection().unwrap(), Some((2, 6)));
    assert_eq!(inputs.get(), 0);
    assert!(input
        .get_attribute("data-oui-composition-start")
        .unwrap()
        .is_none());
    assert_eq!(
        before.pixels(),
        document.render_to_bitmap().unwrap().pixels()
    );
    document
        .dispatch_key_event(
            KeyEventType::Down,
            90,
            Some("z"),
            Modifiers::CTRL | Modifiers::SHIFT,
        )
        .unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("original!"));
}

#[test]
fn ime_ignores_noneditable_controls_and_cancels_when_disabled() {
    let document = document();
    let button = child(&document, "button");
    button.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    document.dispatch_composition_end("committed").unwrap();
    let input = child(&document, "input");
    input.set_control_value("kept").unwrap();
    input.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    input.set_attribute("disabled", "").unwrap();
    document.dispatch_composition_end("committed").unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("kept"));
}

#[test]
fn ime_commit_can_be_canceled_by_a_native_beforeinput_callback() {
    let document = document();
    let input = child(&document, "input");
    input.set_control_value("kept").unwrap();
    input.set_selection(0, 4).unwrap();
    input.focus().unwrap();
    input
        .on("beforeinput", |event| event.prevent_default())
        .unwrap();
    let inputs = Rc::new(Cell::new(0));
    let observed = inputs.clone();
    input
        .on("input", move |_| observed.set(observed.get() + 1))
        .unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    document.dispatch_composition_end("committed").unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("kept"));
    assert_eq!(input.selection().unwrap(), Some((0, 4)));
    assert_eq!(inputs.get(), 0);

    input.remove_event("beforeinput").unwrap();
    let retained = document.clone();
    input
        .on("compositionstart", move |event| {
            retained
                .dispatch_composition_update("reentrant preview")
                .unwrap();
            event.prevent_default();
        })
        .unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_end("ignored commit").unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("kept"));
    assert_eq!(inputs.get(), 0);
    input.remove_event("compositionstart").unwrap();
}

#[test]
fn ime_callbacks_cannot_redirect_an_edit_to_a_different_focused_control() {
    let document = document();
    let first = child(&document, "input");
    let second = child(&document, "input");
    first.set_control_value("first").unwrap();
    second.set_control_value("second").unwrap();
    first.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    let next = second.clone();
    first
        .on("beforeinput", move |_| next.focus().unwrap())
        .unwrap();
    document.dispatch_composition_end("committed").unwrap();
    document
        .dispatch_composition_update("late preview")
        .unwrap();
    document.dispatch_composition_end("late commit").unwrap();
    assert_eq!(first.control_value().unwrap().as_deref(), Some("first"));
    assert_eq!(second.control_value().unwrap().as_deref(), Some("second"));
    first.remove_event("beforeinput").unwrap();
}

#[test]
fn ime_callback_restarting_the_same_control_keeps_the_new_edit() {
    let document = document();
    let input = child(&document, "input");
    input.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("old preview").unwrap();
    let restarted = Rc::new(Cell::new(false));
    let once = restarted.clone();
    let retained = document.clone();
    input
        .on("beforeinput", move |_| {
            if !once.replace(true) {
                retained.dispatch_composition_start().unwrap();
                retained.dispatch_composition_update("new preview").unwrap();
            }
        })
        .unwrap();
    document.dispatch_composition_end("old commit").unwrap();
    assert!(restarted.get());
    assert_eq!(
        input.control_value().unwrap().as_deref(),
        Some("new preview")
    );
    document.dispatch_composition_end("new commit").unwrap();
    assert_eq!(
        input.control_value().unwrap().as_deref(),
        Some("new commit")
    );
    input.remove_event("beforeinput").unwrap();
}

#[test]
fn ime_callbacks_may_remove_the_target_without_terminating_input_dispatch() {
    let document = document();
    let input = child(&document, "input");
    input.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    document.dispatch_composition_update("preview").unwrap();
    let removed = input.clone();
    input
        .on("beforeinput", move |_| removed.remove().unwrap())
        .unwrap();
    document.dispatch_composition_end("committed").unwrap();
    assert!(document.focused_element().unwrap().is_none());

    let input = child(&document, "input");
    input.focus().unwrap();
    document.dispatch_composition_start().unwrap();
    let removed = input.clone();
    input
        .on("compositionend", move |_| removed.remove().unwrap())
        .unwrap();
    document.dispatch_composition_end("committed").unwrap();
    assert!(document.focused_element().unwrap().is_none());
}

#[test]
fn clipboard_copy_and_paste() {
    let document = document();
    let input = child(&document, "input");
    input.set_control_value("copy").unwrap();
    input.set_selection(0, 4).unwrap();
    input.focus().unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 0, Some("c"), Modifiers::CTRL)
        .unwrap();
    assert_eq!(document.clipboard_text().unwrap(), "copy");
    input.set_selection(4, 4).unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 0, Some("v"), Modifiers::CTRL)
        .unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("copycopy"));
}

#[test]
fn editing_undo_and_redo() {
    let document = document();
    let input = child(&document, "input");
    input.focus().unwrap();
    document.dispatch_text_input("x").unwrap();
    document
        .dispatch_key_event(KeyEventType::Down, 0, Some("z"), Modifiers::CTRL)
        .unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some(""));
    document
        .dispatch_key_event(KeyEventType::Down, 0, Some("y"), Modifiers::CTRL)
        .unwrap();
    assert_eq!(input.control_value().unwrap().as_deref(), Some("x"));
}

#[test]
fn tab_focus_order_skips_disabled() {
    let document = document();
    let disabled = child(&document, "button");
    disabled.set_attribute("disabled", "").unwrap();
    let enabled = child(&document, "button");
    document.advance_focus(1).unwrap();
    assert!(enabled.has_focus().unwrap());
}

#[test]
fn modal_focus_is_contained_and_restored() {
    let document = document();
    let outside = child(&document, "button");
    let modal = child(&document, "div");
    let inside = Element::create(&document, "button").unwrap();
    modal.append_child(&inside).unwrap();
    outside.focus().unwrap();
    document.set_modal_root(Some(&modal)).unwrap();
    document.advance_focus(1).unwrap();
    assert!(inside.has_focus().unwrap());
    document.set_modal_root(None).unwrap();
    assert!(outside.has_focus().unwrap());
}

#[test]
fn pointer_capture_lifecycle_is_validated() {
    let document = document();
    let element = child(&document, "div");
    element.set_pointer_capture(7).unwrap();
    element.release_pointer_capture(7).unwrap();
}

#[test]
fn hover_and_active_state_follow_pointer() {
    let document = document();
    let element = child(&document, "button");
    sized(&element, 80.0, 40.0);
    document.update_all().unwrap();
    document
        .dispatch_mouse_event(
            MouseEventType::Move,
            10.0,
            10.0,
            MouseButton::Left,
            Modifiers::NONE,
        )
        .unwrap();
    assert!(element.is_hovered().unwrap());
    document
        .dispatch_mouse_event(
            MouseEventType::Down,
            10.0,
            10.0,
            MouseButton::Left,
            Modifiers::NONE,
        )
        .unwrap();
    assert!(element.is_active().unwrap());
}

#[test]
fn nested_scroll_container_consumes_wheel() {
    let document = document();
    let scroller = child(&document, "div");
    sized(&scroller, 100.0, 80.0);
    scroller.set_overflow(Overflow::Scroll).unwrap();
    document.update_all().unwrap();
    document
        .dispatch_wheel_event(10.0, 10.0, 4.0, 12.0, Modifiers::NONE)
        .unwrap();
    assert_eq!(
        (
            scroller.scroll_left().unwrap(),
            scroller.scroll_top().unwrap()
        ),
        (4.0, 12.0)
    );
}

#[test]
fn smooth_scroll_uses_manual_clock() {
    let document = document();
    let scroller = child(&document, "div");
    scroller
        .smooth_scroll_to(100.0, 50.0, 100.0, Easing::Linear)
        .unwrap();
    document.advance_time(50.0).unwrap();
    assert_eq!(
        (
            scroller.scroll_left().unwrap(),
            scroller.scroll_top().unwrap()
        ),
        (50.0, 25.0)
    );
}

#[test]
fn scroll_snap_selects_nearest_point() {
    let document = document();
    let scroller = child(&document, "div");
    scroller.scroll_to(70.0, 0.0).unwrap();
    scroller
        .settle_scroll_snap(&[0.0, 100.0], &[], 100.0, Easing::Linear)
        .unwrap();
    document.advance_time(100.0).unwrap();
    assert_eq!(scroller.scroll_left().unwrap(), 100.0);
}

#[test]
fn typed_transition_retains_target() {
    let document = document();
    let element = child(&document, "div");
    element.set_opacity(0.0).unwrap();
    element
        .transition(StyleProperty::Opacity, 1.0_f32, animation_options(100.0))
        .unwrap();
    document.advance_time(100.0).unwrap();
    assert!(!document.is_animating().unwrap());
}

#[test]
fn keyframe_animation_dispatches_end() {
    let document = document();
    let element = child(&document, "div");
    let ended = Rc::new(Cell::new(false));
    let observed = ended.clone();
    element
        .on("animationend", move |_| observed.set(true))
        .unwrap();
    element
        .animate(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            animation_options(20.0),
        )
        .unwrap();
    document.advance_time(20.0).unwrap();
    assert!(ended.get());
}

#[test]
fn animation_cancel_restores_underlying_value() {
    let document = document();
    let element = child(&document, "div");
    element.set_opacity(0.75).unwrap();
    let animation = element
        .animate(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            animation_options(100.0),
        )
        .unwrap();
    document.advance_time(50.0).unwrap();
    document.cancel_animation(animation).unwrap();
    assert_eq!(
        document
            .drain_animation_events()
            .unwrap()
            .last()
            .unwrap()
            .kind,
        openui::AnimationEventKind::Cancel
    );
}

#[test]
fn reduced_motion_finishes_animation() {
    let document = document();
    document.set_prefers_reduced_motion(true).unwrap();
    let element = child(&document, "div");
    element
        .animate(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            animation_options(1000.0),
        )
        .unwrap();
    assert!(!document.is_animating().unwrap());
}

#[test]
fn scroll_timeline_samples_from_offset() {
    let document = document();
    let scroller = child(&document, "div");
    let target = child(&document, "div");
    target
        .animate_on_scroll(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            animation_options(100.0),
            &scroller,
            TimelineAxis::Y,
            TimelineRange::new(0.0, 100.0).unwrap(),
        )
        .unwrap();
    scroller.scroll_to(0.0, 50.0).unwrap();
    document.advance_time(1.0).unwrap();
    assert!(document.is_animating().unwrap());
}

#[test]
fn view_timeline_uses_retained_geometry() {
    let document = document();
    let target = child(&document, "div");
    sized(&target, 30.0, 30.0);
    document.update_all().unwrap();
    target
        .animate_on_view(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            animation_options(100.0),
            &target,
            TimelineAxis::Y,
            TimelineRange::new(0.0, 240.0).unwrap(),
        )
        .unwrap();
}

#[test]
fn accessibility_tree_has_stable_nodes() {
    let document = document();
    let button = child(&document, "button");
    button.set_accessibility_label("Save").unwrap();
    let initial = document.accessibility_update().unwrap();
    assert!(initial.tree.is_some());
    assert!(!initial.nodes.is_empty());
    assert!(document.accessibility_update().unwrap().nodes.is_empty());
}

#[test]
fn accessibility_action_uses_control_pipeline() {
    let document = document();
    let checkbox = child(&document, "input");
    checkbox.set_attribute("type", "checkbox").unwrap();
    checkbox
        .perform_accessibility_action(AccessibilityAction::Click)
        .unwrap();
    assert!(checkbox.is_checked().unwrap());
}

#[test]
fn accessibility_relations_reject_cross_document_targets() {
    let first = document();
    let second = document();
    let control = child(&first, "input");
    let foreign_label = child(&second, "label");
    assert!(control
        .set_accessibility_relation(AccessibilityRelation::LabelledBy, &[foreign_label])
        .is_err());
}

#[test]
fn responsive_viewport_reflows_geometry() {
    let document = document();
    let element = child(&document, "div");
    element.set_display(Display::Block).unwrap();
    element.set_width(LengthValue::percent(100.0)).unwrap();
    element.set_height(LengthValue::px(10.0)).unwrap();
    document.update_all().unwrap();
    let before = element.width().unwrap();
    document
        .set_viewport(openui::ViewportMetrics::from_logical_size(640.0, 240.0, 1.0).unwrap())
        .unwrap();
    document.update_all().unwrap();
    let after = element.width().unwrap();
    assert!(
        after > before,
        "viewport resize left width at {after} (before {before})"
    );
}

#[test]
fn synchronous_resource_provider_is_deterministic() {
    let document = document();
    document
        .set_resource_provider(|source| (source == "asset").then(|| vec![1, 2, 3]))
        .unwrap();
    assert!(document
        .load_image_resource("asset", "application/octet-stream", "sha256:test")
        .unwrap()
        .is_some());
    assert!(document
        .load_image_resource("missing", "application/octet-stream", "sha256:none")
        .unwrap()
        .is_none());
}

#[test]
fn repeated_headless_frames_are_byte_identical() {
    let document = document();
    let element = child(&document, "div");
    sized(&element, 60.0, 20.0);
    element.set_background_color(Color::BLUE).unwrap();
    let first = document.render_to_bitmap().unwrap();
    let second = document.render_to_bitmap().unwrap();
    assert_eq!(first, second);
}

#[test]
fn flex_dashboard_layout_has_geometry() {
    let document = document();
    let dashboard = child(&document, "div");
    dashboard.set_display(Display::Flex).unwrap();
    dashboard.set_width(LengthValue::px(300.0)).unwrap();
    for _ in 0..3 {
        let card = Element::create(&document, "div").unwrap();
        sized(&card, 80.0, 40.0);
        dashboard.append_child(&card).unwrap();
    }
    document.update_all().unwrap();
    assert_eq!(dashboard.width().unwrap(), 300.0);
}

#[test]
fn table_application_fixture_lays_out() {
    let document = document();
    let table = child(&document, "table");
    let row = Element::create(&document, "tr").unwrap();
    let cell = Element::create(&document, "td").unwrap();
    cell.set_text("value").unwrap();
    row.append_child(&cell).unwrap();
    table.append_child(&row).unwrap();
    document.update_all().unwrap();
    assert!(table.bounding_rect().unwrap().is_some());
}

#[test]
fn emoji_and_bidi_text_render_without_loss() {
    let document = document();
    let text = child(&document, "div");
    text.set_text("שלום 👩🏽‍💻 مرحبا").unwrap();
    assert!(!document.render_to_png_buffer().unwrap().is_empty());
}

#[test]
fn independent_documents_keep_focus_isolated() {
    let first = document();
    let second = document();
    let first_button = child(&first, "button");
    let second_button = child(&second, "button");
    first_button.focus().unwrap();
    assert!(first_button.has_focus().unwrap());
    assert!(!second_button.has_focus().unwrap());
}
