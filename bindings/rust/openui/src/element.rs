//! Generation-checked element handles for the native retained engine.

use crate::document::{Document, DocumentInner};
use crate::events::{Event, Listener};
use crate::style::{Error, Rect};
use openui_dom::ElementTag;
use openui_engine::{
    AccessibilityAction, AccessibilityLive, AccessibilityRelation, AccessibilityRole, AnimationId,
    AnimationTimeline, NodeHandle, ScrollAnimationId, WeakNode,
};
use openui_style::{
    AnimationOptions, ComputedStyle, Display, Keyframes, PropertyKeyframes, Style, StyleProperty,
    StyleValue, TimelineAxis, TimelineRange,
};
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub struct Element {
    pub(crate) document: Document,
    pub(crate) handle: NodeHandle,
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("document", &self.handle.document_id())
            .field("index", &self.handle.index())
            .field("generation", &self.handle.generation())
            .finish()
    }
}

#[derive(Clone)]
pub struct WeakElement {
    document: Weak<DocumentInner>,
    handle: WeakNode,
}

impl WeakElement {
    pub fn upgrade(&self) -> Option<Element> {
        let inner = self.document.upgrade()?;
        let document = Document { inner };
        let handle = document
            .with_engine(|engine| self.handle.upgrade(engine))
            .ok()?
            .ok()?;
        Some(Element { document, handle })
    }
}

impl Element {
    pub fn create(document: &Document, tag: &str) -> Result<Self, Error> {
        let (tag_kind, display) = tag_definition(tag)?;
        let handle = document.with_engine_mut(|engine| engine.create_element(tag_kind))?;
        if let Some(display) = display {
            document.with_engine_mut(|engine| {
                engine.set_property(handle, StyleProperty::Display, display.into())
            })?;
        }
        Ok(Self::from_handle(document.clone(), handle))
    }

    pub(crate) fn from_handle(document: Document, handle: NodeHandle) -> Self {
        Self { document, handle }
    }

    pub fn downgrade(&self) -> WeakElement {
        WeakElement {
            document: Rc::downgrade(&self.document.inner),
            handle: self.handle.downgrade(),
        }
    }

    pub fn append_child(&self, child: &Element) -> Result<(), Error> {
        if !Rc::ptr_eq(&self.document.inner, &child.document.inner) {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        self.document
            .with_engine_mut(|engine| engine.append_or_move_child(self.handle, child.handle))
    }

    /// Copy this element and its authored descendants into a detached tree.
    /// The returned tree can be attached with [`Self::append_child`] or
    /// [`Self::insert_before`]. Event listeners and running animations stay on
    /// the original elements.
    pub fn clone_subtree(&self) -> Result<Element, Error> {
        let handle = self
            .document
            .with_engine_mut(|engine| engine.clone_subtree(self.handle))?;
        Ok(Self::from_handle(self.document.clone(), handle))
    }

    pub fn insert_before(&self, child: &Element, before: &Element) -> Result<(), Error> {
        if !Rc::ptr_eq(&self.document.inner, &child.document.inner)
            || !Rc::ptr_eq(&self.document.inner, &before.document.inner)
        {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        self.document.with_engine_mut(|engine| {
            engine.insert_before(self.handle, child.handle, before.handle)
        })
    }

    pub fn remove_child(&self, child: &Element) -> Result<(), Error> {
        if !Rc::ptr_eq(&self.document.inner, &child.document.inner) {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        let parent = self
            .document
            .with_engine(|engine| engine.parent(child.handle))??;
        if parent != Some(self.handle) {
            return Err(openui_engine::EngineError::InvalidSibling.into());
        }
        self.document.remove_node(child.handle)
    }

    pub fn remove(&self) -> Result<(), Error> {
        self.document.remove_node(self.handle)
    }

    pub fn first_child(&self) -> Result<Option<Element>, Error> {
        let child = self
            .document
            .with_engine(|engine| engine.children(self.handle))??
            .into_iter()
            .next();
        Ok(child.map(|handle| Self::from_handle(self.document.clone(), handle)))
    }

    pub fn next_sibling(&self) -> Result<Option<Element>, Error> {
        let parent = self
            .document
            .with_engine(|engine| engine.parent(self.handle))??;
        let Some(parent) = parent else {
            return Ok(None);
        };
        let siblings = self
            .document
            .with_engine(|engine| engine.children(parent))??;
        let next = siblings
            .iter()
            .position(|handle| *handle == self.handle)
            .and_then(|index| siblings.get(index + 1))
            .copied();
        Ok(next.map(|handle| Self::from_handle(self.document.clone(), handle)))
    }

    pub fn parent(&self) -> Result<Option<Element>, Error> {
        let parent = self
            .document
            .with_engine(|engine| engine.parent(self.handle))??;
        Ok(parent.map(|handle| Self::from_handle(self.document.clone(), handle)))
    }

    pub fn append_text_node(&self, text: &str) -> Result<(), Error> {
        let handle = self
            .document
            .with_engine_mut(|engine| engine.create_text(text))?;
        self.document
            .with_engine_mut(|engine| engine.append_child(self.handle, handle))
    }

    pub fn create_text_child(&self, text: &str) -> Result<crate::TextNode, Error> {
        let handle = self
            .document
            .with_engine_mut(|engine| engine.create_text(text))?;
        self.document
            .with_engine_mut(|engine| engine.append_child(self.handle, handle))?;
        Ok(crate::TextNode::from_handle(
            self.document.clone(),
            handle,
            true,
        ))
    }

    pub fn set_property(&self, property: StyleProperty, value: StyleValue) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_property(self.handle, property, value))
    }

    /// Return an owned snapshot of this element's resolved style.
    /// Subsequent mutations do not change the returned value.
    pub fn computed_style(&self) -> Result<ComputedStyle, Error> {
        self.document
            .with_engine(|engine| engine.computed_style(self.handle).cloned())?
            .map_err(Into::into)
    }

    pub fn set_language(&self, language: &openui_style::LanguageTag) -> Result<(), Error> {
        self.set_attribute("lang", language.as_str())
    }

    pub fn apply_style(&self, style: &Style) -> Result<(), Error> {
        self.document.transaction(|_| {
            for declaration in style.declarations() {
                self.set_property(declaration.property, declaration.value.clone())?;
            }
            Ok(())
        })
    }

    /// Atomically replace a supported pseudo-element declaration list.
    pub fn set_pseudo_style(
        &self,
        target: openui_style::PseudoStyleTarget,
        style: &Style,
    ) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_pseudo_style(self.handle, target, style))
    }

    pub fn animate<T>(
        &self,
        property: StyleProperty,
        keyframes: Keyframes<T>,
        options: AnimationOptions,
    ) -> Result<AnimationId, Error>
    where
        T: Into<StyleValue>,
    {
        let keyframes = PropertyKeyframes::typed(property, keyframes)?;
        self.document.with_engine_mut(|engine| {
            engine.animate(self.handle, keyframes, options, AnimationTimeline::Document)
        })
    }

    pub fn animate_on_scroll<T>(
        &self,
        property: StyleProperty,
        keyframes: Keyframes<T>,
        options: AnimationOptions,
        source: &Element,
        axis: TimelineAxis,
        range: TimelineRange,
    ) -> Result<AnimationId, Error>
    where
        T: Into<StyleValue>,
    {
        self.ensure_same_document(source)?;
        let keyframes = PropertyKeyframes::typed(property, keyframes)?;
        self.document.with_engine_mut(|engine| {
            engine.animate(
                self.handle,
                keyframes,
                options,
                AnimationTimeline::Scroll {
                    source: source.handle,
                    axis,
                    range,
                },
            )
        })
    }

    pub fn animate_on_view<T>(
        &self,
        property: StyleProperty,
        keyframes: Keyframes<T>,
        options: AnimationOptions,
        subject: &Element,
        axis: TimelineAxis,
        range: TimelineRange,
    ) -> Result<AnimationId, Error>
    where
        T: Into<StyleValue>,
    {
        self.ensure_same_document(subject)?;
        let keyframes = PropertyKeyframes::typed(property, keyframes)?;
        self.document.with_engine_mut(|engine| {
            engine.animate(
                self.handle,
                keyframes,
                options,
                AnimationTimeline::View {
                    subject: subject.handle,
                    axis,
                    range,
                },
            )
        })
    }

    pub fn transition<T>(
        &self,
        property: StyleProperty,
        to: T,
        options: AnimationOptions,
    ) -> Result<AnimationId, Error>
    where
        T: Into<StyleValue>,
    {
        self.document
            .with_engine_mut(|engine| engine.transition(self.handle, property, to, options))
    }

    pub fn smooth_scroll_to(
        &self,
        x: f64,
        y: f64,
        duration_ms: f64,
        easing: openui_style::Easing,
    ) -> Result<ScrollAnimationId, Error> {
        self.document.with_engine_mut(|engine| {
            engine.smooth_scroll_to(self.handle, x, y, duration_ms, easing)
        })
    }

    pub fn settle_scroll_snap(
        &self,
        snap_points_x: &[f64],
        snap_points_y: &[f64],
        duration_ms: f64,
        easing: openui_style::Easing,
    ) -> Result<Option<ScrollAnimationId>, Error> {
        self.document.with_engine_mut(|engine| {
            engine.settle_scroll_snap(
                self.handle,
                snap_points_x,
                snap_points_y,
                duration_ms,
                easing,
            )
        })
    }

    fn ensure_same_document(&self, other: &Element) -> Result<(), Error> {
        if Rc::ptr_eq(&self.document.inner, &other.document.inner) {
            Ok(())
        } else {
            Err(openui_engine::EngineError::WrongDocument.into())
        }
    }

    pub fn set_attribute(&self, name: &str, value: &str) -> Result<(), Error> {
        self.document.with_engine_mut(|engine| {
            engine.set_attribute(self.handle, name.to_owned(), value.to_owned())
        })
    }

    pub fn remove_attribute(&self, name: &str) -> Result<bool, Error> {
        self.document
            .with_engine_mut(|engine| engine.remove_attribute(self.handle, name))
    }

    pub fn get_attribute(&self, name: &str) -> Result<Option<String>, Error> {
        self.document
            .with_engine(|engine| {
                engine
                    .attribute(self.handle, name)
                    .map(|value| value.map(str::to_owned))
            })?
            .map_err(Into::into)
    }

    pub fn set_id(&self, id: &str) -> Result<(), Error> {
        self.set_attribute("id", id)
    }

    pub fn set_class(&self, classes: &str) -> Result<(), Error> {
        self.set_attribute("class", classes)
    }

    /// Check whether this element has a class token.
    pub fn has_class(&self, class: &str) -> Result<bool, Error> {
        validate_class_token(class)?;
        Ok(self
            .get_attribute("class")?
            .is_some_and(|classes| class_tokens(&classes).any(|token| token == class)))
    }

    /// Add a class token, returning whether the attribute changed.
    pub fn add_class(&self, class: &str) -> Result<bool, Error> {
        validate_class_token(class)?;
        let classes = self.get_attribute("class")?.unwrap_or_default();
        let mut tokens: Vec<_> = class_tokens(&classes).collect();
        if tokens.contains(&class) {
            return Ok(false);
        }
        tokens.push(class);
        self.set_class(&tokens.join(" "))?;
        Ok(true)
    }

    /// Remove a class token, returning whether the attribute changed.
    pub fn remove_class(&self, class: &str) -> Result<bool, Error> {
        validate_class_token(class)?;
        let Some(classes) = self.get_attribute("class")? else {
            return Ok(false);
        };
        let tokens: Vec<_> = class_tokens(&classes).collect();
        if !tokens.contains(&class) {
            return Ok(false);
        }
        self.set_class(
            &tokens
                .into_iter()
                .filter(|token| *token != class)
                .collect::<Vec<_>>()
                .join(" "),
        )?;
        Ok(true)
    }

    pub fn set_text(&self, text: &str) -> Result<(), Error> {
        self.remove_all_children()?;
        self.append_text_node(text)
    }

    pub fn set_image_resource(
        &self,
        resource: openui_style::ImageResourceId,
        intrinsic_size: Option<(f32, f32)>,
    ) -> Result<(), Error> {
        self.document.with_engine_mut(|engine| {
            engine.set_image_resource(self.handle, resource, intrinsic_size)
        })
    }

    pub fn set_accessibility_role(&self, role: AccessibilityRole) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_accessibility_role(self.handle, role))
    }

    pub fn set_accessibility_label(&self, label: &str) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_accessibility_label(self.handle, label))
    }

    pub fn set_accessibility_description(&self, description: &str) -> Result<(), Error> {
        self.document.with_engine_mut(|engine| {
            engine.set_accessibility_description(self.handle, description)
        })
    }

    pub fn set_accessibility_value(&self, value: &str) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_accessibility_value(self.handle, value))
    }

    pub fn set_accessibility_live(&self, live: AccessibilityLive) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_accessibility_live(self.handle, live))
    }

    pub fn set_accessibility_hidden(&self, hidden: bool) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_accessibility_hidden(self.handle, hidden))
    }

    pub fn set_accessibility_relation(
        &self,
        relation: AccessibilityRelation,
        targets: &[Element],
    ) -> Result<(), Error> {
        if targets
            .iter()
            .any(|target| !Rc::ptr_eq(&self.document.inner, &target.document.inner))
        {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        let handles: Vec<_> = targets.iter().map(|target| target.handle).collect();
        self.document.with_engine_mut(|engine| {
            engine.set_accessibility_relation(self.handle, relation, &handles)
        })
    }

    pub fn perform_accessibility_action(&self, action: AccessibilityAction) -> Result<(), Error> {
        self.document
            .perform_accessibility_action(self.handle, action)
    }

    pub fn bounding_rect(&self) -> Result<Option<Rect>, Error> {
        self.document
            .with_engine_mut(|engine| engine.bounds(self.handle))
    }

    pub fn width(&self) -> Result<f32, Error> {
        Ok(self.bounding_rect()?.map_or(0.0, |rect| rect.width))
    }

    pub fn height(&self) -> Result<f32, Error> {
        Ok(self.bounding_rect()?.map_or(0.0, |rect| rect.height))
    }

    pub fn scroll_left(&self) -> Result<f64, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.scroll_offset(self.handle))??
            .0)
    }

    pub fn scroll_top(&self) -> Result<f64, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.scroll_offset(self.handle))??
            .1)
    }

    pub fn scroll_to(&self, x: f64, y: f64) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.scroll_to(self.handle, x, y))
    }

    pub fn scroll_by(&self, dx: f64, dy: f64) -> Result<(), Error> {
        let (x, y) = self
            .document
            .with_engine(|engine| engine.scroll_offset(self.handle))??;
        self.scroll_to(x + dx, y + dy)
    }

    pub fn control_value(&self) -> Result<Option<String>, Error> {
        self.document
            .with_engine(|engine| {
                engine
                    .control_state(self.handle)
                    .map(|state| state.map(|state| state.value.clone()))
            })?
            .map_err(Into::into)
    }

    pub fn control_display_value(&self) -> Result<Option<String>, Error> {
        self.document
            .with_engine(|engine| {
                engine
                    .control_state(self.handle)
                    .map(|state| state.map(openui_engine::ControlState::display_value))
            })?
            .map_err(Into::into)
    }

    pub fn set_control_value(&self, value: &str) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_control_value(self.handle, value))
    }

    pub fn selection(&self) -> Result<Option<(usize, usize)>, Error> {
        self.document
            .with_engine(|engine| {
                engine
                    .control_state(self.handle)
                    .map(|state| state.map(openui_engine::ControlState::selection))
            })?
            .map_err(Into::into)
    }

    pub fn set_selection(&self, anchor: usize, focus: usize) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_selection(self.handle, anchor, focus))
    }

    pub fn is_checked(&self) -> Result<bool, Error> {
        self.control_flag(|state| state.checked)
    }

    pub fn is_selected(&self) -> Result<bool, Error> {
        self.control_flag(|state| state.selected)
    }

    pub fn is_open(&self) -> Result<bool, Error> {
        if self
            .document
            .with_engine(|engine| engine.element_tag(self.handle))??
            == ElementTag::Details
        {
            return Ok(self.get_attribute("open")?.is_some());
        }
        self.control_flag(|state| state.open)
    }

    /// Set whether a native details element is expanded.
    pub fn set_open(&self, open: bool) -> Result<(), Error> {
        if self
            .document
            .with_engine(|engine| engine.element_tag(self.handle))??
            != ElementTag::Details
        {
            return Err(Error::InvalidArgument(
                "set_open requires a details element",
            ));
        }
        if open {
            self.set_attribute("open", "")
        } else {
            self.remove_attribute("open").map(|_| ())
        }
    }

    pub fn is_indeterminate(&self) -> Result<bool, Error> {
        self.control_flag(|state| state.indeterminate)
    }

    pub fn set_checked(&self, checked: bool) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_checked(self.handle, checked))
    }

    pub fn set_indeterminate(&self, indeterminate: bool) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_indeterminate(self.handle, indeterminate))
    }

    pub fn is_hovered(&self) -> Result<bool, Error> {
        self.document
            .with_engine(|engine| engine.is_hovered(self.handle))?
            .map_err(Into::into)
    }

    pub fn is_active(&self) -> Result<bool, Error> {
        self.document
            .with_engine(|engine| engine.is_active(self.handle))?
            .map_err(Into::into)
    }

    pub fn focus(&self) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.focus(self.handle))
    }

    pub fn blur(&self) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.blur(self.handle))
    }

    pub fn has_focus(&self) -> Result<bool, Error> {
        self.document
            .with_engine(|engine| engine.focused() == Some(self.handle))
    }

    /// Dispatch a native click and run the element's default activation unless canceled.
    pub fn click(&self) -> Result<(), Error> {
        self.perform_accessibility_action(AccessibilityAction::Click)
    }

    pub fn set_pointer_capture(&self, pointer_id: u64) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_pointer_capture(pointer_id, self.handle))
    }

    pub fn release_pointer_capture(&self, pointer_id: u64) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.release_pointer_capture(pointer_id, self.handle))
    }

    pub fn on<F>(&self, event_type: &str, callback: F) -> Result<(), Error>
    where
        F: Fn(&Event) + 'static,
    {
        self.add_listener(event_type, false, callback)
    }

    pub fn on_capture<F>(&self, event_type: &str, callback: F) -> Result<(), Error>
    where
        F: Fn(&Event) + 'static,
    {
        self.add_listener(event_type, true, callback)
    }

    pub fn remove_event(&self, event_type: &str) -> Result<(), Error> {
        self.document.remove_listeners(self.handle, event_type)
    }

    pub fn remove_all_children(&self) -> Result<(), Error> {
        let children = self
            .document
            .with_engine(|engine| engine.children(self.handle))??;
        for child in children {
            self.document.remove_node(child)?;
        }
        Ok(())
    }

    fn add_listener<F>(&self, event_type: &str, capture: bool, callback: F) -> Result<(), Error>
    where
        F: Fn(&Event) + 'static,
    {
        self.document.add_listener(
            self.handle,
            event_type,
            Listener {
                capture,
                callback: Rc::new(callback),
            },
        )
    }

    fn control_flag(
        &self,
        get: impl FnOnce(&openui_engine::ControlState) -> bool,
    ) -> Result<bool, Error> {
        self.document
            .with_engine(|engine| {
                engine
                    .control_state(self.handle)
                    .map(|state| state.is_some_and(get))
            })?
            .map_err(Into::into)
    }
}

pub(crate) fn class_tokens(classes: &str) -> impl Iterator<Item = &str> {
    classes
        .split(|ch| matches!(ch, ' ' | '\t' | '\n' | '\x0c' | '\r'))
        .filter(|token| !token.is_empty())
}

pub(crate) fn validate_class_token(class: &str) -> Result<(), Error> {
    if class.is_empty()
        || class
            .chars()
            .any(|ch| matches!(ch, ' ' | '\t' | '\n' | '\x0c' | '\r'))
    {
        Err(Error::InvalidArgument("class must be one nonempty token"))
    } else {
        Ok(())
    }
}

fn tag_definition(tag: &str) -> Result<(ElementTag, Option<Display>), Error> {
    use ElementTag as T;
    let normalized = tag.to_ascii_lowercase();
    let definition = match normalized.as_str() {
        "div" | "main" | "nav" | "header" | "footer" | "section" | "article" | "aside" | "p"
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "hr" => {
            (T::Div, Some(Display::Block))
        }
        "li" => (T::Div, Some(Display::ListItem)),
        "span" | "a" | "label" | "strong" | "em" | "small" => (T::Span, Some(Display::Inline)),
        "br" => (T::Break, None),
        "wbr" => (T::WordBreak, None),
        "ruby" => (T::Ruby, Some(Display::Inline)),
        "rt" => (T::RubyText, Some(Display::Inline)),
        "table" => (T::Table, Some(Display::Table)),
        "caption" => (T::TableCaption, Some(Display::TableCaption)),
        "colgroup" => (T::TableColumnGroup, Some(Display::TableColumnGroup)),
        "col" => (T::TableColumn, Some(Display::TableColumn)),
        "thead" => (T::TableHead, Some(Display::TableHeaderGroup)),
        "tbody" => (T::TableBody, Some(Display::TableRowGroup)),
        "tfoot" => (T::TableFoot, Some(Display::TableFooterGroup)),
        "tr" => (T::TableRow, Some(Display::TableRow)),
        "td" => (T::TableCell, Some(Display::TableCell)),
        "th" => (T::TableHeaderCell, Some(Display::TableCell)),
        "img" => (T::Image, Some(Display::InlineBlock)),
        "canvas" => (T::Canvas, Some(Display::InlineBlock)),
        "svg" => (T::Svg, Some(Display::InlineBlock)),
        "iframe" => (T::IFrame, Some(Display::InlineBlock)),
        "object" => (T::Object, Some(Display::InlineBlock)),
        "audio" => (T::Audio, Some(Display::InlineBlock)),
        "video" => (T::Video, Some(Display::InlineBlock)),
        "input" => (T::Input, Some(Display::InlineBlock)),
        "button" => (T::Button, Some(Display::InlineBlock)),
        "meter" => (T::Meter, Some(Display::InlineBlock)),
        "progress" => (T::Progress, Some(Display::InlineBlock)),
        "fieldset" => (T::Fieldset, Some(Display::Block)),
        "legend" => (T::Legend, Some(Display::Block)),
        "details" => (T::Details, Some(Display::Block)),
        "summary" => (T::Summary, Some(Display::Block)),
        "textarea" => (T::TextArea, Some(Display::InlineBlock)),
        "select" => (T::Select, Some(Display::InlineBlock)),
        "option" => (T::Option, Some(Display::Block)),
        "optgroup" => (T::OptGroup, Some(Display::Block)),
        "form" => (T::Form, Some(Display::Block)),
        "embed" => (T::Embed, Some(Display::InlineBlock)),
        _ => return Err(Error::UnknownTag(tag.to_owned())),
    };
    Ok(definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventPhase, Modifiers, MouseButton, MouseEventType};
    use openui_style::{AnimationOptions, FillMode, Keyframes};
    use std::cell::{Cell, RefCell};

    #[test]
    fn typed_animation_uses_manual_clock_and_dispatches_events() {
        let document = Document::new(100, 100).unwrap();
        let element = Element::create(&document, "div").unwrap();
        document.body().append_child(&element).unwrap();
        let ended = Rc::new(Cell::new(false));
        let observed = ended.clone();
        element
            .on("animationend", move |_| observed.set(true))
            .unwrap();
        let animation = element
            .animate(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
            )
            .unwrap();
        document.advance_time(50.0).unwrap();
        assert_eq!(
            document.animation_state(animation).unwrap().current_time_ms,
            50.0
        );
        document.advance_time(100.0).unwrap();
        assert!(ended.get());
        assert_eq!(document.drain_animation_events().unwrap().len(), 2);
    }

    #[test]
    fn stale_weak_and_cross_document_handles_are_safe() {
        let a = Document::new(100, 100).unwrap();
        let b = Document::new(100, 100).unwrap();
        let parent = a.body();
        let child = Element::create(&a, "div").unwrap();
        parent.append_child(&child).unwrap();
        let weak = child.downgrade();
        child.remove().unwrap();
        assert!(weak.upgrade().is_none());

        let foreign = Element::create(&b, "div").unwrap();
        assert!(parent.append_child(&foreign).is_err());
    }

    #[test]
    fn cloned_subtree_keeps_native_content_but_has_independent_handles() {
        let document = Document::new(100, 100).unwrap();
        let original = Element::create(&document, "div").unwrap();
        original.set_id("original").unwrap();
        original
            .set_property(
                StyleProperty::Width,
                openui_style::LengthValue::px(42.0).into(),
            )
            .unwrap();
        let input = Element::create(&document, "input").unwrap();
        input.set_attribute("type", "text").unwrap();
        input.set_control_value("edited").unwrap();
        original.append_child(&input).unwrap();
        document.body().append_child(&original).unwrap();

        let clone = original.clone_subtree().unwrap();
        assert!(clone.parent().unwrap().is_none());
        assert_eq!(
            clone.get_attribute("id").unwrap().as_deref(),
            Some("original")
        );
        let cloned_input = clone.first_child().unwrap().unwrap();
        assert_eq!(
            cloned_input.control_value().unwrap().as_deref(),
            Some("edited")
        );
        assert_eq!(
            document
                .with_engine(|engine| engine.computed_style(clone.handle).unwrap().width)
                .unwrap(),
            document
                .with_engine(|engine| engine.computed_style(original.handle).unwrap().width)
                .unwrap()
        );

        cloned_input.set_control_value("copy").unwrap();
        clone
            .set_property(
                StyleProperty::Width,
                openui_style::LengthValue::px(24.0).into(),
            )
            .unwrap();
        clone.set_id("copy").unwrap();
        document.body().append_child(&clone).unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("edited"));
        assert_ne!(
            document
                .with_engine(|engine| engine.computed_style(clone.handle).unwrap().width)
                .unwrap(),
            document
                .with_engine(|engine| engine.computed_style(original.handle).unwrap().width)
                .unwrap()
        );
        assert_eq!(
            original.get_attribute("id").unwrap().as_deref(),
            Some("original")
        );
        assert!(document.element_by_id("copy").unwrap().is_some());
        original.remove().unwrap();
        assert!(original.clone_subtree().is_err());
        assert_eq!(
            cloned_input.control_value().unwrap().as_deref(),
            Some("copy")
        );
    }

    #[test]
    fn events_capture_target_and_bubble_without_reentrant_borrows() {
        let document = Document::new(100, 100).unwrap();
        let root = document.body();
        let button = Element::create(&document, "button").unwrap();
        root.append_child(&button).unwrap();
        button
            .set_property(
                StyleProperty::Width,
                openui_style::LengthValue::px(40.0).into(),
            )
            .unwrap();
        button
            .set_property(
                StyleProperty::Height,
                openui_style::LengthValue::px(40.0).into(),
            )
            .unwrap();

        let trace = Rc::new(RefCell::new(Vec::new()));
        let capture_trace = trace.clone();
        root.on_capture("click", move |event| {
            capture_trace.borrow_mut().push(event.phase().unwrap());
        })
        .unwrap();
        let target_trace = trace.clone();
        let button_for_callback = button.clone();
        button
            .on("click", move |event| {
                target_trace.borrow_mut().push(event.phase().unwrap());
                button_for_callback
                    .set_attribute("data-clicked", "true")
                    .unwrap();
            })
            .unwrap();
        let bubble_trace = trace.clone();
        root.on("click", move |event| {
            bubble_trace.borrow_mut().push(event.phase().unwrap());
        })
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
        assert_eq!(
            *trace.borrow(),
            vec![EventPhase::Capture, EventPhase::Target, EventPhase::Bubble]
        );
        assert_eq!(
            button.get_attribute("data-clicked").unwrap().as_deref(),
            Some("true")
        );
    }
}
