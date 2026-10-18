use openui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
fn child(parent: &Element, d: &Document, tag: &str) -> Element {
    let e = Element::create(d, tag).unwrap();
    parent.append_child(&e).unwrap();
    e
}
#[test]
fn groups_follow_dom_legend_order_and_do_not_rewrite_own_state() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let first = child(&f, &d, "legend");
    let a = child(&first, &d, "input");
    let second = child(&f, &d, "legend");
    let b = child(&second, &d, "input");
    let normal = child(&f, &d, "input");
    f.set_attribute("disabled", "").unwrap();
    first
        .set_property(StyleProperty::Display, Display::None.into())
        .unwrap();
    assert!(!a.is_effectively_disabled().unwrap());
    assert!(b.is_effectively_disabled().unwrap());
    assert!(normal.is_effectively_disabled().unwrap());
    assert!(!normal.is_own_disabled().unwrap());
    assert_eq!(normal.get_attribute("disabled").unwrap(), None);
    f.insert_before(&second, &first).unwrap();
    assert!(a.is_effectively_disabled().unwrap());
    assert!(!b.is_effectively_disabled().unwrap());
    f.remove_attribute("disabled").unwrap();
    assert!(!a.is_effectively_disabled().unwrap());
    assert!(!normal.is_effectively_disabled().unwrap());
    d.body().set_attribute("disabled", "").unwrap();
    assert!(!normal.is_effectively_disabled().unwrap());
}
#[test]
fn nested_group_exception_still_respects_outer_groups() {
    let d = Document::new(240, 160).unwrap();
    let outer = child(&d.body(), &d, "fieldset");
    let first = child(&outer, &d, "legend");
    let nested = child(&outer, &d, "fieldset");
    let legend = child(&nested, &d, "legend");
    let input = child(&legend, &d, "textarea");
    nested.set_attribute("disabled", "").unwrap();
    assert!(!input.is_effectively_disabled().unwrap());
    outer.set_attribute("disabled", "").unwrap();
    assert!(input.is_effectively_disabled().unwrap());
    first.append_child(&nested).unwrap();
    assert!(!input.is_effectively_disabled().unwrap());
    input.set_attribute("disabled", "").unwrap();
    assert!(input.is_effectively_disabled().unwrap());
    assert!(input.is_own_disabled().unwrap());
    input.remove_attribute("disabled").unwrap();
    assert!(!input.is_effectively_disabled().unwrap());
}
#[test]
fn options_inherit_only_their_direct_optgroup() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let select = child(&f, &d, "select");
    let group = child(&select, &d, "optgroup");
    let option = child(&group, &d, "option");
    f.set_attribute("disabled", "").unwrap();
    select.set_attribute("disabled", "").unwrap();
    assert!(!option.is_effectively_disabled().unwrap());
    assert!(!group.is_effectively_disabled().unwrap());
    group.set_attribute("disabled", "").unwrap();
    assert!(option.is_effectively_disabled().unwrap());
    assert!(!option.is_own_disabled().unwrap());
    let wrapper = child(&group, &d, "span");
    wrapper.append_child(&option).unwrap();
    assert!(!option.is_effectively_disabled().unwrap());
    option.set_attribute("disabled", "").unwrap();
    assert!(option.is_effectively_disabled().unwrap());
    option.remove_attribute("disabled").unwrap();
    assert!(!option.is_effectively_disabled().unwrap());
}
#[test]
fn group_disable_commits_then_blurs_after_the_engine_borrow() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let input = child(&f, &d, "input");
    input.set_control_value("abc").unwrap();
    input.focus().unwrap();
    d.dispatch_text_input("X").unwrap();
    let rows = Rc::new(RefCell::new(Vec::new()));
    for kind in ["change", "blur", "focusout"] {
        let input = input.downgrade();
        let rows = rows.clone();
        input
            .upgrade()
            .unwrap()
            .on(kind, move |event| {
                let e = input.upgrade().unwrap();
                assert!(!e.has_focus().unwrap());
                assert!(e.is_connected().unwrap());
                assert!(e.is_effectively_disabled().unwrap());
                assert!(!e.is_own_disabled().unwrap());
                assert!(!event.cancelable());
                rows.borrow_mut().push(event.event_type.clone());
            })
            .unwrap();
    }
    f.set_attribute("disabled", "").unwrap();
    assert_eq!(&*rows.borrow(), &["change", "blur", "focusout"]);
    assert!(!input.has_focus().unwrap());
    assert_eq!(input.control_value().unwrap().as_deref(), Some("abcX"));
}
#[test]
fn disabling_group_preserves_blur_callback_reenable_and_redirect() {
    for redirect in [false, true] {
        let d = Document::new(240, 160).unwrap();
        let f = child(&d.body(), &d, "fieldset");
        let input = child(&f, &d, "input");
        let outside = child(&d.body(), &d, "input");
        let group = f.downgrade();
        let next = outside.downgrade();
        input
            .on("blur", move |_| {
                group
                    .upgrade()
                    .unwrap()
                    .remove_attribute("disabled")
                    .unwrap();
                if redirect {
                    next.upgrade().unwrap().focus().unwrap();
                }
            })
            .unwrap();
        input.focus().unwrap();
        f.set_attribute("disabled", "").unwrap();
        assert!(!input.has_focus().unwrap());
        assert_eq!(outside.has_focus().unwrap(), redirect);
        assert!(!input.is_effectively_disabled().unwrap());
    }
}
#[test]
fn retained_moves_blur_while_attached_and_invalid_moves_keep_focus() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let input = child(&f, &d, "input");
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let weak = input.downgrade();
    input
        .on("blur", move |_| {
            let e = weak.upgrade().unwrap();
            assert!(e.is_connected().unwrap());
            assert_eq!(
                e.parent().unwrap().unwrap().kind().unwrap(),
                ElementTag::Fieldset
            );
            observed.borrow_mut().push("blur");
        })
        .unwrap();
    input.focus().unwrap();
    assert!(input.append_child(&f).is_err());
    assert!(d.body().detach().is_err());
    assert!(input.has_focus().unwrap());
    assert!(trace.borrow().is_empty());
    f.insert_before(&input, &input).unwrap();
    assert!(!input.has_focus().unwrap());
    assert_eq!(&*trace.borrow(), &["blur"]);
    input.focus().unwrap();
    f.detach().unwrap();
    assert!(!input.has_focus().unwrap());
    assert!(!input.is_connected().unwrap());
    assert_eq!(&*trace.borrow(), &["blur", "blur"]);
    d.body().append_child(&f).unwrap();
    assert!(input.is_connected().unwrap());
    assert!(!input.has_focus().unwrap());
    assert_eq!(input.get_attribute("data-oui-focused").unwrap(), None);
}
#[test]
fn disabled_groups_suppress_user_edits_and_clicks_but_allow_application_writes() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let input = child(&f, &d, "input");
    let checkbox = child(&f, &d, "input");
    checkbox.set_attribute("type", "checkbox").unwrap();
    input.set_control_value("abc").unwrap();
    let clicks = Rc::new(RefCell::new(0));
    let count = clicks.clone();
    checkbox
        .on("click", move |_| *count.borrow_mut() += 1)
        .unwrap();
    f.set_attribute("disabled", "").unwrap();
    assert!(input.focus().is_err());
    for command in [
        EditCommand::SelectAll,
        EditCommand::Delete {
            direction: TextDirection::Backward,
            unit: TextUnit::Grapheme,
        },
        EditCommand::Undo,
    ] {
        assert!(matches!(
            input.edit_text(command),
            Err(Error::Engine(openui_engine::EngineError::NotEditable))
        ));
    }
    checkbox.click().unwrap();
    assert!(!checkbox.is_checked().unwrap());
    assert_eq!(*clicks.borrow(), 0);
    input.set_control_value("application").unwrap();
    input
        .set_selection_range(1, 4, SelectionDirection::Backward)
        .unwrap();
    assert_eq!(
        input.control_value().unwrap().as_deref(),
        Some("application")
    );
    assert_eq!(input.selection().unwrap(), Some((1, 4)));
    f.remove_attribute("disabled").unwrap();
    checkbox.click().unwrap();
    assert!(checkbox.is_checked().unwrap());
    assert_eq!(*clicks.borrow(), 1);
}
#[test]
fn hidden_first_legend_exempts_disabled_state_without_becoming_focusable() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let first = child(&f, &d, "legend");
    let input = child(&first, &d, "input");
    first
        .set_property(StyleProperty::Display, Display::None.into())
        .unwrap();
    assert!(matches!(
        input.focus(),
        Err(Error::Engine(openui_engine::EngineError::NotFocusable))
    ));
    f.set_attribute("disabled", "").unwrap();
    assert!(!input.is_effectively_disabled().unwrap());
    first
        .set_property(StyleProperty::Display, Display::Block.into())
        .unwrap();
    input.focus().unwrap();
    assert!(input.has_focus().unwrap());
}
#[test]
fn disabled_accessibility_state_and_actions_change_without_touching_values() {
    let d = Document::new(240, 160).unwrap();
    let f = child(&d.body(), &d, "fieldset");
    let input = child(&f, &d, "input");
    let checkbox = child(&f, &d, "input");
    checkbox.set_attribute("type", "checkbox").unwrap();
    input.set_accessibility_label("Editor").unwrap();
    checkbox.set_accessibility_label("Toggle").unwrap();
    input.set_control_value("abc").unwrap();
    let before = d.accessibility_update().unwrap();
    assert!(before
        .nodes
        .iter()
        .any(|(_, n)| n.label() == Some("Editor")));
    f.set_attribute("disabled", "").unwrap();
    let update = d.accessibility_update().unwrap();
    let editor = &update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Editor"))
        .unwrap()
        .1;
    assert!(editor.is_disabled());
    assert_eq!(editor.value(), Some("abc"));
    assert!(!editor.supports_action(openui::AccessibilityPlatformAction::SetValue));
    assert!(!editor.supports_action(openui::AccessibilityPlatformAction::SetTextSelection));
    let toggle = &update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Toggle"))
        .unwrap()
        .1;
    assert!(toggle.is_disabled());
    assert!(!toggle.supports_action(openui::AccessibilityPlatformAction::Click));
    assert!(input
        .perform_accessibility_action(AccessibilityAction::SetValue("bad".into()))
        .is_err());
    checkbox
        .perform_accessibility_action(AccessibilityAction::Click)
        .unwrap();
    assert!(!checkbox.is_checked().unwrap());
    assert_eq!(input.control_value().unwrap().as_deref(), Some("abc"));
    f.remove_attribute("disabled").unwrap();
    let update = d.accessibility_update().unwrap();
    let editor = &update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Editor"))
        .unwrap()
        .1;
    assert!(!editor.is_disabled());
    assert!(editor.supports_action(openui::AccessibilityPlatformAction::SetValue));
}
#[test]
fn stale_and_foreign_handles_do_not_read_the_effective_tree() {
    let d = Document::new(240, 160).unwrap();
    let e = child(&d.body(), &d, "input");
    let other = Document::new(240, 160).unwrap();
    e.focus().unwrap();
    assert!(other.body().append_child(&e).is_err());
    assert!(e.has_focus().unwrap());
    e.remove().unwrap();
    assert!(matches!(
        e.is_effectively_disabled(),
        Err(Error::Engine(openui_engine::EngineError::StaleHandle))
    ));
    assert!(matches!(
        e.is_own_disabled(),
        Err(Error::Engine(openui_engine::EngineError::StaleHandle))
    ));
}
