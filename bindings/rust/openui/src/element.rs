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

#[derive(Debug, Clone)]
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
    /// Create a native retained element. `foreignObject` creates an SVG
    /// viewport for native UI children; like `div`, it has container kind
    /// [`ElementTag::Div`]. No document scripts execute.
    pub fn create(document: &Document, tag: &str) -> Result<Self, Error> {
        if tag.eq_ignore_ascii_case("foreignobject") {
            return Self::create_svg_foreign_object(document);
        }
        let (tag_kind, display) = tag_definition(tag)?;
        let handle = document.with_engine_mut(|engine| engine.create_native_element(tag_kind))?;
        if let Some(display) = display {
            document.with_engine_mut(|engine| {
                engine.set_property(handle, StyleProperty::Display, display.into())
            })?;
        }
        Ok(Self::from_handle(document.clone(), handle))
    }

    /// Create an SVG viewport containing native retained UI elements.
    /// Width and height specify the viewport bounds, including decoration.
    /// Child mutation, events and ownership use the ordinary native API.
    /// The container's native kind is [`ElementTag::Div`].
    pub fn create_svg_foreign_object(document: &Document) -> Result<Self, Error> {
        let handle = document.with_engine_mut(|engine| engine.create_svg_foreign_object())?;
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

    /// Return this element's native kind. Several authored tag names can
    /// share a kind; for example, `div` and `main` are both [`ElementTag::Div`].
    pub fn kind(&self) -> Result<ElementTag, Error> {
        self.document
            .with_engine(|engine| engine.element_tag(self.handle))?
            .map_err(Into::into)
    }

    pub fn append_child(&self, child: &Element) -> Result<(), Error> {
        if !Rc::ptr_eq(&self.document.inner, &child.document.inner) {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        self.document
            .append_element_child(self.handle, child.handle)
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
        self.document
            .insert_element_before(self.handle, child.handle, before.handle)
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

    /// Detach this element while keeping its handle, descendants, state, and
    /// Rust event listeners. Attach it again with [`Self::append_child`] or
    /// [`Self::insert_before`]. Use [`Self::remove`] to destroy it instead.
    pub fn detach(&self) -> Result<(), Error> {
        self.document.detach_node(self.handle)
    }

    /// Return the first authored element child, skipping text nodes.
    pub fn first_child(&self) -> Result<Option<Element>, Error> {
        let child = self.document.with_engine(|engine| {
            for handle in engine.children(self.handle)? {
                if engine.is_authored_element(handle)? {
                    return Ok(Some(handle));
                }
            }
            Ok::<_, openui_engine::EngineError>(None)
        })??;
        Ok(child.map(|handle| Self::from_handle(self.document.clone(), handle)))
    }

    /// Return the next authored element sibling, skipping text nodes.
    pub fn next_sibling(&self) -> Result<Option<Element>, Error> {
        let next = self.document.with_engine(|engine| {
            let Some(parent) = engine.parent(self.handle)? else {
                return Ok(None);
            };
            let siblings = engine.children(parent)?;
            if let Some(index) = siblings.iter().position(|handle| *handle == self.handle) {
                for handle in siblings.into_iter().skip(index + 1) {
                    if engine.is_authored_element(handle)? {
                        return Ok(Some(handle));
                    }
                }
            }
            Ok::<_, openui_engine::EngineError>(None)
        })??;
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

    /// Attach or move an existing text node under this element.
    pub fn append_text_child(&self, child: &crate::TextNode) -> Result<(), Error> {
        if !Rc::ptr_eq(&self.document.inner, &child.document.inner) {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        self.document
            .append_element_child(self.handle, child.handle)
    }

    /// Insert or move a text node immediately before an element child.
    pub fn insert_text_before(
        &self,
        child: &crate::TextNode,
        before: &Element,
    ) -> Result<(), Error> {
        if !Rc::ptr_eq(&self.document.inner, &child.document.inner)
            || !Rc::ptr_eq(&self.document.inner, &before.document.inner)
        {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        self.document
            .insert_element_before(self.handle, child.handle, before.handle)
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
        self.document
            .set_element_attribute(self.handle, name, value)
    }

    pub fn remove_attribute(&self, name: &str) -> Result<bool, Error> {
        self.document.remove_element_attribute(self.handle, name)
    }

    /// Reflected own disabled state, without inheritance from an ancestor group.
    /// Use [`Self::is_effectively_disabled`] to determine interaction eligibility.
    pub fn is_own_disabled(&self) -> Result<bool, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.is_own_disabled(self.handle))??)
    }

    /// Whether this control is disabled by its own state or an ancestor group.
    /// This includes the first-legend exception and direct option-group rules.
    /// The query does not run layout or change reflected attributes.
    pub fn is_effectively_disabled(&self) -> Result<bool, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.is_effectively_disabled(self.handle))??)
    }

    /// Whether this node is attached to its retained document, without layout.
    pub fn is_connected(&self) -> Result<bool, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.is_connected(self.handle))??)
    }

    /// Compare live native node identity, including independently owned aliases.
    pub fn is_same_node(&self, other: &Element) -> Result<bool, Error> {
        self.document
            .with_engine(|engine| self.handle.downgrade().upgrade(engine))??;
        other
            .document
            .with_engine(|engine| other.handle.downgrade().upgrade(engine))??;
        Ok(
            Rc::ptr_eq(&self.document.inner.engine, &other.document.inner.engine)
                && self.handle == other.handle,
        )
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
        self.document
            .with_engine_mut(|engine| engine.set_text_content(self.handle, text))
    }

    /// Return this element's authored text and descendant text in tree order.
    pub fn text_content(&self) -> Result<String, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.text_content(self.handle))??)
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

    /// Owned border-box rectangles for every layout fragment, in logical
    /// viewport coordinates after scrolling and transforms. Includes empty,
    /// hidden, clipped, and pointer-ineligible boxes. Detached elements and
    /// `display: none` elements return an empty list. Reading flushes layout.
    pub fn client_rects(&self) -> Result<Vec<Rect>, Error> {
        self.document
            .with_engine_mut(|engine| engine.client_rects(self.handle))
    }

    /// Bounds of all nonempty layout fragments in logical viewport
    /// coordinates. Returns the final rectangle when every fragment is
    /// empty, and `None` when the element has no layout box.
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
            .with_engine_mut(|engine| {
                engine.update()?;
                engine.scroll_offset(self.handle)
            })?
            .0)
    }

    /// Owned client and content dimensions after resolving pending layout.
    pub fn scroll_metrics(&self) -> Result<Option<crate::ScrollMetrics>, Error> {
        self.document
            .with_engine_mut(|engine| engine.scroll_metrics(self.handle))
    }

    pub fn scroll_top(&self) -> Result<f64, Error> {
        Ok(self
            .document
            .with_engine_mut(|engine| {
                engine.update()?;
                engine.scroll_offset(self.handle)
            })?
            .1)
    }

    pub fn scroll_to(&self, x: f64, y: f64) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.scroll_to(self.handle, x, y))
    }

    pub fn scroll_by(&self, dx: f64, dy: f64) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.scroll_by(self.handle, dx, dy))
    }

    /// Reveal this element through enclosing scrollports and the native viewport.
    /// Logical alignment follows this element's writing mode and direction.
    pub fn scroll_into_view(&self, options: crate::ScrollIntoViewOptions) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.scroll_into_view(self.handle, options))
    }

    /// Reveal using the retained animation clock and CSS ease curve. Zero
    /// duration and reduced motion settle immediately. No app callback runs
    /// while the engine is borrowed.
    pub fn smooth_scroll_into_view(
        &self,
        options: crate::ScrollIntoViewOptions,
        duration_ms: f64,
    ) -> Result<Vec<crate::ScrollAnimationId>, Error> {
        self.document.with_engine_mut(|engine| {
            engine.smooth_scroll_into_view(self.handle, options, duration_ms)
        })
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
        self.document.set_control_value(self.handle, value)
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
        self.document.set_selection(self.handle, anchor, focus)
    }

    /// Retained native selection direction, or None for a non-text control.
    pub fn selection_direction(&self) -> Result<Option<crate::SelectionDirection>, Error> {
        self.document
            .with_engine(|engine| {
                engine.control_state(self.handle).map(|state| {
                    state
                        .filter(|state| {
                            matches!(
                                state.role,
                                openui_dom::FormControlRole::TextInput
                                    | openui_dom::FormControlRole::TextArea
                            )
                        })
                        .map(openui_engine::ControlState::selection_direction)
                })
            })?
            .map_err(Into::into)
    }
    /// Set a native range in UTF-8 byte units, retaining explicit direction.
    pub fn set_selection_range(
        &self,
        start: usize,
        end: usize,
        direction: crate::SelectionDirection,
    ) -> Result<(), Error> {
        self.document
            .set_selection_range(self.handle, start, end, direction)
    }
    /// Programmatically replace a UTF-8 range without input/change notifications.
    /// Selection notifications are delivered by the native task barrier/event loop.
    pub fn replace_control_range(
        &self,
        replacement: &str,
        start: usize,
        end: usize,
        mode: crate::RangeSelectionMode,
    ) -> Result<(), Error> {
        self.document
            .replace_control_range(self.handle, replacement, start, end, mode)
    }

    /// Run a native editing command on an input or textarea.
    ///
    /// Uses the same engine and event path as keyboard editing. Value changes
    /// emit `input` after the engine borrow is released, so Rust callbacks can
    /// read or change the document. Selection commands emit no `input` event.
    /// Read-only controls allow selection commands; disabled controls reject
    /// every command. Invalid or removed elements return an error.
    pub fn edit_text(&self, command: crate::EditCommand) -> Result<(), Error> {
        self.document.edit_control(self.handle, command)
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
        self.document.focus_element(self.handle)
    }

    pub fn blur(&self) -> Result<(), Error> {
        self.document.blur_element(self.handle)
    }

    pub fn has_focus(&self) -> Result<bool, Error> {
        self.document
            .with_engine(|engine| engine.focused() == Some(self.handle))
    }

    /// Dispatch a native click and run the element's default activation unless canceled.
    pub fn click(&self) -> Result<(), Error> {
        self.perform_accessibility_action(AccessibilityAction::Click)
    }

    /// Dispatch an owned, bubbling, cancelable left click event.
    ///
    /// Raw dispatch remains available on disabled controls. Checkable state
    /// changes before listeners; cancellation restores it. Returns false when
    /// a listener prevented the default action. Use Self::click for an
    /// ordinary simulated click that obeys disabled eligibility.
    pub fn dispatch_click_event(&self) -> Result<bool, Error> {
        let event = Event::pointer(
            "click",
            0,
            0.0,
            0.0,
            crate::MouseButton::Left,
            crate::Modifiers::NONE,
        );
        self.document.dispatch_click_event(self.handle, &event)
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
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "hr" => (T::Div, None),
        "li" => (T::Div, Some(Display::ListItem)),
        "span" | "a" | "label" | "strong" | "em" | "small" => (T::Span, None),
        "br" => (T::Break, None),
        "wbr" => (T::WordBreak, None),
        "ruby" => (T::Ruby, None),
        "rt" => (T::RubyText, None),
        "table" => (T::Table, None),
        "caption" => (T::TableCaption, None),
        "colgroup" => (T::TableColumnGroup, None),
        "col" => (T::TableColumn, None),
        "thead" => (T::TableHead, None),
        "tbody" => (T::TableBody, None),
        "tfoot" => (T::TableFoot, None),
        "tr" => (T::TableRow, None),
        "td" => (T::TableCell, None),
        "th" => (T::TableHeaderCell, None),
        "img" => (T::Image, None),
        "canvas" => (T::Canvas, None),
        "svg" => (T::Svg, None),
        "iframe" => (T::IFrame, None),
        "object" => (T::Object, None),
        "audio" => (T::Audio, None),
        "video" => (T::Video, None),
        "input" => (T::Input, None),
        "button" => (T::Button, None),
        "meter" => (T::Meter, None),
        "progress" => (T::Progress, None),
        "fieldset" => (T::Fieldset, None),
        "legend" => (T::Legend, None),
        "details" => (T::Details, None),
        "summary" => (T::Summary, None),
        "textarea" => (T::TextArea, None),
        "select" => (T::Select, None),
        "option" => (T::Option, None),
        "optgroup" => (T::OptGroup, None),
        "form" => (T::Form, None),
        "embed" => (T::Embed, None),
        _ => return Err(Error::UnknownTag(tag.to_owned())),
    };
    Ok(definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventPhase, Modifiers, MouseButton, MouseEventType};
    use openui_style::{AnimationOptions, FillMode, Keyframes, Overflow, OverflowClipBox};
    use std::cell::{Cell, RefCell};

    #[test]
    fn native_edit_commands_dispatch_after_borrows_and_preserve_graphemes() {
        use crate::{EditCommand, TextDirection, TextUnit};

        let document = Document::new(200, 100).unwrap();
        let input = Element::create(&document, "input").unwrap();
        let button = Element::create(&document, "button").unwrap();
        let label = Element::create(&document, "div").unwrap();
        document.body().append_child(&input).unwrap();
        document.body().append_child(&button).unwrap();
        document.body().append_child(&label).unwrap();
        input.set_control_value("á👩‍💻z").unwrap();
        let owned_value = input.control_value().unwrap();
        let observed = Rc::new(RefCell::new(Vec::new()));
        let callbacks = observed.clone();
        let weak_label = label.downgrade();
        input
            .on("input", move |event| {
                let target = event.target().unwrap();
                let value = target.control_value().unwrap().unwrap();
                weak_label.upgrade().unwrap().set_text(&value).unwrap();
                callbacks.borrow_mut().push(value);
            })
            .unwrap();
        let weak_input = input.downgrade();
        button
            .on("click", move |_| {
                weak_input
                    .upgrade()
                    .unwrap()
                    .edit_text(EditCommand::Delete {
                        direction: TextDirection::Backward,
                        unit: TextUnit::Grapheme,
                    })
                    .unwrap();
            })
            .unwrap();
        button.click().unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("á👩‍💻"));
        input.edit_text(EditCommand::Undo).unwrap();
        input.edit_text(EditCommand::Redo).unwrap();
        assert_eq!(&*observed.borrow(), &["á👩‍💻", "á👩‍💻z", "á👩‍💻"]);
        assert_eq!(label.text_content().unwrap(), "á👩‍💻");
        assert_eq!(owned_value.as_deref(), Some("á👩‍💻z"));
        input.edit_text(EditCommand::SelectAll).unwrap();
        assert_eq!(input.selection().unwrap(), Some((0, "á👩‍💻".len())));
        input.set_selection(0, 0).unwrap();
        input
            .edit_text(EditCommand::Move {
                direction: TextDirection::Forward,
                unit: TextUnit::Grapheme,
                extend: true,
            })
            .unwrap();
        assert_eq!(input.selection().unwrap(), Some((0, "á".len())));
        assert_eq!(observed.borrow().len(), 3);
        let weak = input.downgrade();
        drop(input);
        drop(button);
        drop(label);
        drop(document);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn native_edit_commands_enforce_readonly_disabled_and_handle_lifetimes() {
        use crate::{EditCommand, TextDirection, TextUnit};

        let document = Document::new(100, 100).unwrap();
        let input = Element::create(&document, "input").unwrap();
        document.body().append_child(&input).unwrap();
        input.set_control_value("é👍z").unwrap();
        input.set_attribute("readonly", "").unwrap();
        input.edit_text(EditCommand::SelectAll).unwrap();
        assert_eq!(input.selection().unwrap(), Some((0, "é👍z".len())));
        let delete = EditCommand::Delete {
            direction: TextDirection::Backward,
            unit: TextUnit::Grapheme,
        };
        assert!(matches!(
            input.edit_text(delete),
            Err(Error::Engine(openui_engine::EngineError::NotEditable))
        ));
        assert_eq!(input.control_value().unwrap().as_deref(), Some("é👍z"));
        input.set_attribute("disabled", "").unwrap();
        assert!(input.edit_text(EditCommand::SelectAll).is_err());
        let ordinary = Element::create(&document, "div").unwrap();
        assert!(ordinary.edit_text(delete).is_err());
        input.remove().unwrap();
        assert!(input.edit_text(delete).is_err());
    }

    #[test]
    fn native_edit_commands_and_keyboard_defaults_share_control_state_and_events() {
        use crate::{EditCommand, KeyEventType, TextDirection, TextUnit};

        let document = Document::new(200, 100).unwrap();
        for tag in ["input", "textarea"] {
            let direct = Element::create(&document, tag).unwrap();
            let keyboard = Element::create(&document, tag).unwrap();
            for element in [&direct, &keyboard] {
                document.body().append_child(element).unwrap();
                element.set_control_value("á👩‍💻z").unwrap();
            }
            let direct_events = Rc::new(Cell::new(0));
            let keyboard_events = Rc::new(Cell::new(0));
            for (element, counter) in [(&direct, &direct_events), (&keyboard, &keyboard_events)] {
                let counter = counter.clone();
                element
                    .on("input", move |_| counter.set(counter.get() + 1))
                    .unwrap();
            }
            keyboard.focus().unwrap();
            for (command, code, key, modifiers) in [
                (
                    EditCommand::Delete {
                        direction: TextDirection::Backward,
                        unit: TextUnit::Grapheme,
                    },
                    8,
                    "Backspace",
                    Modifiers::NONE,
                ),
                (EditCommand::Undo, 90, "z", Modifiers::CTRL),
                (EditCommand::Redo, 89, "y", Modifiers::CTRL),
                (EditCommand::SelectAll, 65, "a", Modifiers::CTRL),
            ] {
                direct.edit_text(command).unwrap();
                document
                    .dispatch_key_event(KeyEventType::Down, code, Some(key), modifiers)
                    .unwrap();
                assert_eq!(
                    direct.control_value().unwrap(),
                    keyboard.control_value().unwrap()
                );
                assert_eq!(direct.selection().unwrap(), keyboard.selection().unwrap());
                assert_eq!(direct_events.get(), keyboard_events.get());
            }
            assert_eq!(direct_events.get(), 3);
        }
    }

    #[test]
    fn native_replaced_elements_use_chromium_host_clip_defaults() {
        let document = Document::new(100, 100).unwrap();
        for tag in ["img", "canvas", "video", "iframe", "embed"] {
            let element = Element::create(&document, tag).unwrap();
            let style = element.computed_style().unwrap();
            assert_eq!(style.overflow_x, Overflow::Clip, "{tag}");
            assert_eq!(style.overflow_y, Overflow::Clip, "{tag}");
            assert_eq!(
                style.overflow_clip_box,
                OverflowClipBox::ContentBox,
                "{tag}"
            );
        }

        let image = Element::create(&document, "img").unwrap();
        image.set_overflow_x(Overflow::Scroll).unwrap();
        assert_eq!(image.computed_style().unwrap().overflow_x, Overflow::Scroll);
        assert_eq!(image.computed_style().unwrap().overflow_y, Overflow::Clip);
        // Object fallback children currently use a native block-flow path;
        // its host clip needs separate qualification against Chromium.
        assert_eq!(
            Element::create(&document, "object")
                .unwrap()
                .computed_style()
                .unwrap()
                .overflow_x,
            Overflow::Visible
        );
        assert_eq!(
            Element::create(&document, "div")
                .unwrap()
                .computed_style()
                .unwrap()
                .overflow_x,
            Overflow::Visible
        );
    }

    #[test]
    fn native_svg_viewport_bounds_survive_decoration_mutation_and_cloning() {
        let document = Document::new(100, 100).unwrap();
        let svg = Element::create(&document, "svg").unwrap();
        svg.set_width(crate::typed_style::LengthValue::px(80.0))
            .unwrap();
        svg.set_height(crate::typed_style::LengthValue::px(60.0))
            .unwrap();
        document.body().append_child(&svg).unwrap();
        let foreign = Element::create(&document, "foreignObject").unwrap();
        foreign
            .set_width(crate::typed_style::LengthValue::px(1.0))
            .unwrap();
        foreign
            .set_height(crate::typed_style::LengthValue::px(1.0))
            .unwrap();
        svg.append_child(&foreign).unwrap();
        let bounds = foreign.bounding_rect().unwrap().unwrap();
        foreign.set_border_left_width(3).unwrap();
        foreign
            .set_border_left_style(openui_style::BorderStyle::Double)
            .unwrap();
        foreign
            .set_padding(crate::typed_style::Edges::all(
                crate::typed_style::LengthValue::px(2.0),
            ))
            .unwrap();
        assert_eq!(foreign.bounding_rect().unwrap().unwrap(), bounds);
        foreign
            .set_box_sizing(openui_style::BoxSizing::BorderBox)
            .unwrap();
        assert_eq!(foreign.bounding_rect().unwrap().unwrap(), bounds);
        let clone = foreign.clone_subtree().unwrap();
        foreign.detach().unwrap();
        svg.append_child(&clone).unwrap();
        assert_eq!(clone.bounding_rect().unwrap().unwrap(), bounds);
        clone
            .set_width(crate::typed_style::LengthValue::px(8.0))
            .unwrap();
        let changed = clone.bounding_rect().unwrap().unwrap();
        assert_eq!((changed.width, changed.height), (8.0, 1.0));
    }

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

#[cfg(test)]
mod native_node_query_guards {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn connectivity_checks_the_whole_ancestor_chain_without_layout_or_focus_work() {
        let document = Document::new(160, 100).unwrap();
        let root = document.body();
        let parent = Element::create(&document, "div").unwrap();
        let child = Element::create(&document, "input").unwrap();
        parent.append_child(&child).unwrap();
        assert!(root.is_connected().unwrap());
        assert!(!parent.is_connected().unwrap());
        assert!(!child.is_connected().unwrap());
        root.append_child(&parent).unwrap();
        child.focus().unwrap();
        document
            .with_engine_mut(|engine| {
                engine.scene()?;
                // Leave a pending Engine-only mutation. Pure node queries must
                // neither run layout nor settle focus on behalf of the app.
                engine.set_attribute(child.handle, "disabled", "")
            })
            .unwrap();
        let before = document
            .with_engine(|engine| (engine.stats(), engine.focused()))
            .unwrap();
        assert!(parent.is_connected().unwrap());
        assert!(child.is_connected().unwrap());
        assert!(child.is_same_node(&child.clone()).unwrap());
        assert_eq!(
            before,
            document
                .with_engine(|engine| (engine.stats(), engine.focused()))
                .unwrap()
        );
        parent.detach().unwrap();
        assert!(!parent.is_connected().unwrap());
        assert!(!child.is_connected().unwrap());
        root.append_child(&parent).unwrap();
        assert!(child.is_connected().unwrap());
    }

    #[test]
    fn native_node_identity_distinguishes_aliases_siblings_documents_and_stale_nodes() {
        let document = Document::new(64, 64).unwrap();
        let a = Element::create(&document, "div").unwrap();
        let b = Element::create(&document, "div").unwrap();
        let other = Document::new(64, 64).unwrap();
        assert!(a.is_same_node(&a.clone()).unwrap());
        assert!(!a.is_same_node(&b).unwrap());
        assert!(!document.body().is_same_node(&other.body()).unwrap());
        a.remove().unwrap();
        let replacement = Element::create(&document, "div").unwrap();
        for result in [
            a.is_connected(),
            a.is_same_node(&replacement),
            replacement.is_same_node(&a),
        ] {
            assert!(matches!(
                result,
                Err(Error::Engine(openui_engine::EngineError::StaleHandle))
            ));
        }
    }

    #[test]
    fn native_node_queries_reject_conflicting_engine_borrows() {
        let document = Document::new(64, 64).unwrap();
        let root = document.body();
        let held = document.inner.engine.borrow_mut();
        assert!(matches!(root.is_connected(), Err(Error::ReentrantMutation)));
        assert!(matches!(
            root.is_same_node(&root),
            Err(Error::ReentrantMutation)
        ));
        drop(held);
        assert!(root.is_connected().unwrap());
    }

    #[cfg(feature = "ffi-integration")]
    #[test]
    fn native_node_identity_survives_independent_facade_wrappers_of_one_engine() {
        let engine = Rc::new(RefCell::new(
            openui_engine::Engine::new(
                crate::ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap(),
            )
            .unwrap(),
        ));
        let first = Document::from_shared_engine(engine.clone());
        let second = Document::from_shared_engine(engine);
        assert!(first.body().is_same_node(&second.body()).unwrap());
    }
}
