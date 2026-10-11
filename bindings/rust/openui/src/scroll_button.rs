//! Public native controls owned by a retained scroll container.

use crate::{AccessibilityAction, Element, Error, Rect, Style, StyleProperty, StyleValue};
pub use openui_engine::{
    ScrollButtonAvailability, ScrollButtonDirection, ScrollButtonState, ScrollSnapStrictness,
};

/// A native scroll button with origin-owned lifetime and shared Engine defaults.
///
/// Pointer and accessibility clicks use the same cancelable event/default path.
/// Enter and Space activate on keydown. Application event targets and the
/// public focused element are the originating scroller. Register callbacks on
/// [`Self::origin`]; no document scripts or synthetic application handler are
/// needed to implement the control.
#[derive(Debug, Clone)]
pub struct ScrollButton {
    element: Element,
}

impl Element {
    /// Create a scroll button for this container. Direction is unique per
    /// origin. The button moves, detaches and is removed with this element.
    pub fn create_scroll_button(
        &self,
        direction: ScrollButtonDirection,
    ) -> Result<ScrollButton, Error> {
        let handle = self
            .document
            .with_engine_mut(|engine| engine.create_scroll_button(self.handle, direction))?;
        Ok(ScrollButton {
            element: Element::from_handle(self.document.clone(), handle),
        })
    }

    pub fn scroll_button(
        &self,
        direction: ScrollButtonDirection,
    ) -> Result<Option<ScrollButton>, Error> {
        let handle = self
            .document
            .with_engine(|engine| engine.scroll_button(self.handle, direction))??;
        Ok(handle.map(|handle| ScrollButton {
            element: Element::from_handle(self.document.clone(), handle),
        }))
    }

    /// Native snap strictness supplements the shared axis/align style API.
    /// Unsupported selection remains explicit in a button's state snapshot.
    pub fn set_scroll_snap_strictness(
        &self,
        strictness: ScrollSnapStrictness,
    ) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_scroll_snap_strictness(self.handle, strictness))
    }
}

impl ScrollButton {
    pub fn origin(&self) -> Result<Element, Error> {
        let state = self.state()?;
        Ok(Element::from_handle(
            self.element.document.clone(),
            state.origin,
        ))
    }

    pub fn state(&self) -> Result<ScrollButtonState, Error> {
        self.element
            .document
            .with_engine_mut(|engine| engine.scroll_button_state(self.element.handle))
    }

    pub fn direction(&self) -> Result<ScrollButtonDirection, Error> {
        Ok(self.state()?.direction)
    }
    pub fn is_enabled(&self) -> Result<bool, Error> {
        Ok(self.state()?.enabled)
    }

    /// Update direction without changing ownership, style or handle identity.
    /// A collision with another owned direction leaves both buttons intact.
    pub fn set_direction(&self, direction: ScrollButtonDirection) -> Result<(), Error> {
        self.element.document.with_engine_mut(|engine| {
            engine.set_scroll_button_direction(self.element.handle, direction)
        })
    }

    pub fn set_text(&self, text: &str) -> Result<(), Error> {
        self.element.set_text(text)
    }
    pub fn set_property(&self, property: StyleProperty, value: StyleValue) -> Result<(), Error> {
        self.element.set_property(property, value)
    }
    pub fn apply_style(&self, style: &Style) -> Result<(), Error> {
        self.element.apply_style(style)
    }
    pub fn set_native_appearance(&self, enabled: bool) -> Result<(), Error> {
        self.element.set_native_appearance(enabled)
    }
    pub fn computed_style(&self) -> Result<crate::ComputedStyle, Error> {
        self.element.computed_style()
    }
    pub fn bounding_rect(&self) -> Result<Option<Rect>, Error> {
        self.element.bounding_rect()
    }
    pub fn client_rects(&self) -> Result<Vec<Rect>, Error> {
        self.element.client_rects()
    }

    /// Author suppression is independent of boundary-derived availability.
    pub fn set_disabled(&self, disabled: bool) -> Result<(), Error> {
        if disabled {
            self.element.set_attribute("disabled", "")
        } else {
            self.element.remove_attribute("disabled").map(|_| ())
        }
    }

    pub fn set_accessibility_label(&self, label: &str) -> Result<(), Error> {
        self.element.set_attribute("aria-label", label)
    }

    pub fn accessibility_node_id(&self) -> Result<crate::AccessibilityNodeId, Error> {
        self.element
            .document
            .with_engine(|engine| engine.accessibility_node_id(self.element.handle))?
            .map_err(Into::into)
    }

    pub fn focus(&self) -> Result<(), Error> {
        self.element.focus()
    }
    pub fn blur(&self) -> Result<(), Error> {
        self.element.blur()
    }
    pub fn has_focus(&self) -> Result<bool, Error> {
        self.element
            .document
            .with_engine(|engine| engine.focused() == Some(self.element.handle))
    }

    /// Run an ordinary cancelable native click through retained default action.
    pub fn activate(&self) -> Result<(), Error> {
        self.element
            .perform_accessibility_action(AccessibilityAction::Click)
    }

    pub fn perform_accessibility_action(&self, action: AccessibilityAction) -> Result<(), Error> {
        self.element.perform_accessibility_action(action)
    }

    /// Remove the owned control and invalidate its generation-checked handle.
    pub fn remove(&self) -> Result<(), Error> {
        self.element.remove()
    }
}
