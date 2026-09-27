use openui::prelude::*;
use openui::{AccessibilityAction, AccessibilityRelation, KeyEventType, MouseEventType};
use std::cell::Cell;
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
