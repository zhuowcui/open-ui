//! Normalized input routing and retained form-control state.

use crate::{Engine, EngineError, NodeHandle};
use openui_dom::{ElementTag, FormControlRole};
use std::cmp::Ordering;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteStep {
    pub node: NodeHandle,
    pub phase: EventPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRoute {
    pub target: NodeHandle,
    pub steps: Vec<RouteStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEventKind {
    Down,
    Up,
    Move,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointerUpdate {
    pub target: Option<NodeHandle>,
    /// Nodes left by this pointer, from the former target toward the root.
    pub left: Vec<NodeHandle>,
    /// Nodes entered by this pointer, from the root toward the new target.
    pub entered: Vec<NodeHandle>,
    /// Candidate click target. The caller applies activation only when neither
    /// pointer-up nor click was canceled by an event listener.
    pub activation: Option<NodeHandle>,
    /// Deferred range-control default action. The frontend applies this only
    /// after listeners have had an opportunity to prevent the pointer event.
    pub range_value: Option<(NodeHandle, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusOrigin {
    Pointer,
    Keyboard,
    /// Legacy name for focus requested by native code. No script is executed.
    Script,
    Accessibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Backward,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextUnit {
    Grapheme,
    Word,
    Line,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditCommand {
    Move {
        direction: TextDirection,
        unit: TextUnit,
        extend: bool,
    },
    Delete {
        direction: TextDirection,
        unit: TextUnit,
    },
    SelectAll,
    Undo,
    Redo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAdjustment {
    Previous,
    Next,
    PageBackward,
    PageForward,
    Minimum,
    Maximum,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlState {
    pub role: FormControlRole,
    pub disabled: bool,
    pub checked: bool,
    pub selected: bool,
    pub open: bool,
    pub indeterminate: bool,
    pub value: String,
    pub placeholder: String,
    pub selection_anchor: usize,
    pub selection_focus: usize,
    pub composition: Option<(usize, usize)>,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub password: bool,
    pub(crate) native_intrinsic_sizing: bool,
    composition_original: Option<(String, usize, usize)>,
    history: Vec<String>,
    future: Vec<String>,
}

impl ControlState {
    pub(crate) fn for_tag(tag: ElementTag) -> Option<Self> {
        let role = match tag {
            ElementTag::Button => FormControlRole::Button,
            ElementTag::Input => FormControlRole::TextInput,
            ElementTag::TextArea => FormControlRole::TextArea,
            ElementTag::Select => FormControlRole::Select,
            ElementTag::Option => FormControlRole::Option,
            ElementTag::OptGroup => FormControlRole::OptGroup,
            ElementTag::Meter => FormControlRole::Meter,
            ElementTag::Progress => FormControlRole::Progress,
            _ => return None,
        };
        Some(Self {
            role,
            disabled: false,
            checked: false,
            selected: false,
            open: false,
            indeterminate: false,
            value: String::new(),
            placeholder: String::new(),
            selection_anchor: 0,
            selection_focus: 0,
            composition: None,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            password: false,
            native_intrinsic_sizing: matches!(
                role,
                FormControlRole::TextInput | FormControlRole::TextArea
            ),
            composition_original: None,
            history: Vec::new(),
            future: Vec::new(),
        })
    }

    pub fn selection(&self) -> (usize, usize) {
        ordered(self.selection_anchor, self.selection_focus)
    }

    pub fn display_value(&self) -> String {
        if self.password {
            "•".repeat(self.value.graphemes(true).count())
        } else {
            self.value.clone()
        }
    }

    fn clamp_selection(&mut self) {
        self.selection_anchor = previous_boundary(&self.value, self.selection_anchor);
        self.selection_focus = previous_boundary(&self.value, self.selection_focus);
    }

    fn checkpoint(&mut self) {
        if self.history.last() != Some(&self.value) {
            self.history.push(self.value.clone());
        }
        self.future.clear();
    }

    fn replace_selection(&mut self, replacement: &str) {
        self.finish_composition();
        let (start, end) = self.selection();
        self.checkpoint();
        self.value.replace_range(start..end, replacement);
        let caret = start + replacement.len();
        self.selection_anchor = caret;
        self.selection_focus = caret;
        self.composition = None;
    }

    fn update_composition(&mut self, replacement: &str) {
        if self.composition_original.is_none() {
            self.composition_original = Some((
                self.value.clone(),
                self.selection_anchor,
                self.selection_focus,
            ));
        }
        let (start, end) = self.composition.unwrap_or_else(|| self.selection());
        self.value.replace_range(start..end, replacement);
        let caret = start + replacement.len();
        self.selection_anchor = caret;
        self.selection_focus = caret;
        self.composition = Some((start, caret));
    }

    pub(crate) fn finish_composition(&mut self) {
        if let Some((original, _, _)) = self.composition_original.take() {
            if original != self.value {
                if self.history.last() != Some(&original) {
                    self.history.push(original);
                }
                self.future.clear();
            }
        }
        self.composition = None;
    }

    fn cancel_composition(&mut self) {
        if let Some((value, anchor, focus)) = self.composition_original.take() {
            self.value = value;
            self.selection_anchor = anchor;
            self.selection_focus = focus;
        }
        self.composition = None;
    }

    pub(crate) fn clear_composition(&mut self) {
        self.composition = None;
        self.composition_original = None;
    }

    fn move_selection(&mut self, direction: TextDirection, unit: TextUnit, extend: bool) {
        self.finish_composition();
        let (_, end) = self.selection();
        let caret = if !extend && self.selection_anchor != self.selection_focus {
            match direction {
                TextDirection::Backward => self.selection().0,
                TextDirection::Forward => end,
            }
        } else {
            move_boundary(&self.value, self.selection_focus, direction, unit)
        };
        if !extend {
            self.selection_anchor = caret;
        }
        self.selection_focus = caret;
    }

    fn delete(&mut self, direction: TextDirection, unit: TextUnit) {
        if self.selection_anchor != self.selection_focus {
            self.replace_selection("");
            return;
        }
        let caret = self.selection_focus;
        let other = move_boundary(&self.value, caret, direction, unit);
        if caret == other {
            return;
        }
        self.selection_anchor = other;
        self.replace_selection("");
    }

    fn undo(&mut self) {
        self.finish_composition();
        let Some(previous) = self.history.pop() else {
            return;
        };
        self.future
            .push(std::mem::replace(&mut self.value, previous));
        self.selection_anchor = self.value.len();
        self.selection_focus = self.value.len();
        self.composition = None;
    }

    fn redo(&mut self) {
        self.finish_composition();
        let Some(next) = self.future.pop() else {
            return;
        };
        self.history.push(std::mem::replace(&mut self.value, next));
        self.selection_anchor = self.value.len();
        self.selection_focus = self.value.len();
        self.composition = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationResult {
    pub changed: Vec<NodeHandle>,
}

impl Engine {
    pub fn event_route(&self, target: NodeHandle) -> Result<EventRoute, EngineError> {
        self.resolve(target)?;
        let mut path = vec![target];
        let mut current = target;
        while let Some(parent) = self.parent(current)? {
            path.push(parent);
            current = parent;
        }
        let mut steps = Vec::with_capacity(path.len() * 2);
        for node in path.iter().skip(1).rev() {
            steps.push(RouteStep {
                node: *node,
                phase: EventPhase::Capture,
            });
        }
        steps.push(RouteStep {
            node: target,
            phase: EventPhase::Target,
        });
        for node in path.iter().skip(1) {
            steps.push(RouteStep {
                node: *node,
                phase: EventPhase::Bubble,
            });
        }
        Ok(EventRoute { target, steps })
    }

    pub fn pointer_event(
        &mut self,
        pointer_id: u64,
        kind: PointerEventKind,
        x: f32,
        y: f32,
    ) -> Result<PointerUpdate, EngineError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::InvalidInput(
                "pointer coordinates must be finite",
            ));
        }
        let target = if let Some(captured) = self.pointer_capture(pointer_id) {
            Some(captured)
        } else {
            self.hit_test(x, y)?
        };
        let new_path = target
            .map(|node| self.path_to_root(node))
            .transpose()?
            .unwrap_or_default();
        let old_path = self
            .hover_paths
            .get(&pointer_id)
            .cloned()
            .unwrap_or_default();
        let common = common_root_suffix(&old_path, &new_path);
        let left = old_path[..old_path.len().saturating_sub(common)].to_vec();
        let mut entered = new_path[..new_path.len().saturating_sub(common)].to_vec();
        entered.reverse();
        if kind == PointerEventKind::Cancel {
            self.hover_paths.remove(&pointer_id);
        } else {
            self.hover_paths.insert(pointer_id, new_path);
        }

        let range_target = match kind {
            PointerEventKind::Down => target,
            PointerEventKind::Move | PointerEventKind::Up => {
                self.active_pointers.get(&pointer_id).copied()
            }
            PointerEventKind::Cancel => None,
        };
        let range_value = if let Some(range) = range_target {
            let is_range = self
                .controls
                .get(&range.index)
                .is_some_and(|control| control.role == FormControlRole::Range && !control.disabled);
            let bounds = is_range.then(|| self.bounds(range)).transpose()?.flatten();
            bounds
                .filter(|bounds| bounds.width > 0.0)
                .map(|bounds| (range, ((x - bounds.x) / bounds.width).clamp(0.0, 1.0)))
        } else {
            None
        };
        let activation = match kind {
            PointerEventKind::Down => {
                if let Some(target) = target.filter(|node| !self.is_disabled(*node)) {
                    self.active_pointers.insert(pointer_id, target);
                }
                None
            }
            PointerEventKind::Up => self
                .active_pointers
                .remove(&pointer_id)
                .filter(|pressed| Some(*pressed) == target && !self.is_disabled(*pressed)),
            PointerEventKind::Cancel => {
                self.active_pointers.remove(&pointer_id);
                self.pointer_capture.remove(&pointer_id);
                None
            }
            PointerEventKind::Move => None,
        };
        Ok(PointerUpdate {
            target,
            left,
            entered,
            activation,
            range_value,
        })
    }

    /// Validate whether native focus can be requested without changing state.
    pub fn can_focus(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        self.resolve(handle)?;
        Ok(self.is_focusable(handle))
    }

    pub fn focus_with_origin(
        &mut self,
        handle: NodeHandle,
        origin: FocusOrigin,
    ) -> Result<Option<NodeHandle>, EngineError> {
        if !self.can_focus(handle)? {
            return Err(EngineError::NotFocusable);
        }
        let previous = self.focused;
        if previous != Some(handle) {
            if let Some(previous) = previous {
                if let Ok(node) = self.resolve(previous) {
                    self.document
                        .node_mut(node)
                        .attributes
                        .remove("data-oui-focused");
                }
            }
            self.focused = Some(handle);
            self.focus_visible = !matches!(origin, FocusOrigin::Pointer);
            if let Ok(node) = self.resolve(handle) {
                self.document.set_attribute(node, "data-oui-focused", "");
            }
            self.dirty.paint = true;
            self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        }
        Ok(previous)
    }

    pub fn focus_visible(&self) -> bool {
        self.focused.is_some() && self.focus_visible
    }

    pub fn advance_focus(&mut self, direction: i32) -> Result<Option<NodeHandle>, EngineError> {
        let next = self.next_focus_target(direction)?;
        if let Some(next) = next {
            self.focus_with_origin(next, FocusOrigin::Keyboard)?;
        }
        Ok(next)
    }

    /// Choose a sequential focus target without mutating focus. Native frontends
    /// release their engine borrow before delivering the intervening blur event.
    pub fn next_focus_target(&self, direction: i32) -> Result<Option<NodeHandle>, EngineError> {
        if direction != -1 && direction != 1 {
            return Err(EngineError::InvalidInput("focus direction must be -1 or 1"));
        }
        let candidates = self.focus_candidates()?;
        if candidates.is_empty() {
            return Ok(None);
        }
        let current = self
            .focused
            .and_then(|focused| candidates.iter().position(|node| *node == focused));
        if current.is_none() {
            if let Some(focused) = self.focused {
                let eligible: std::collections::HashSet<_> = candidates.iter().copied().collect();
                let mut document_order = Vec::new();
                self.collect_focus_document_order(
                    self.modal_root.unwrap_or_else(|| self.root()),
                    &mut document_order,
                )?;
                if let Some(anchor) = document_order.iter().position(|node| *node == focused) {
                    let next = if direction == 1 {
                        document_order[anchor + 1..]
                            .iter()
                            .find(|node| eligible.contains(*node))
                    } else {
                        document_order[..anchor]
                            .iter()
                            .rev()
                            .find(|node| eligible.contains(*node))
                    };
                    if let Some(next) = next {
                        return Ok(Some(*next));
                    }
                }
            }
        }
        let index = match (current, direction) {
            (Some(index), 1) => (index + 1) % candidates.len(),
            (Some(index), -1) => (index + candidates.len() - 1) % candidates.len(),
            (None, 1) => 0,
            (None, -1) => candidates.len() - 1,
            _ => unreachable!(),
        };
        let next = candidates[index];
        Ok(Some(next))
    }

    pub fn set_modal_root(&mut self, root: Option<NodeHandle>) -> Result<(), EngineError> {
        let next = self.prepare_modal_focus(root)?;
        if self.focused != next {
            if let Some(previous) = self.focused {
                self.blur(previous)?;
            }
            if let Some(next) = next {
                self.focus_with_origin(next, FocusOrigin::Keyboard)?;
            }
        }
        Ok(())
    }

    pub fn modal_root(&self) -> Option<NodeHandle> {
        self.modal_root
    }

    /// Change modal containment and choose its focus target without moving
    /// focus. Frontends deliver composition/blur callbacks before that move.
    pub fn prepare_modal_focus(
        &mut self,
        root: Option<NodeHandle>,
    ) -> Result<Option<NodeHandle>, EngineError> {
        if let Some(root) = root {
            self.resolve(root)?;
        }
        if self.modal_root != root {
            self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        }
        let next = match root {
            Some(root) => {
                if self.modal_root != Some(root) {
                    self.focus_before_modal = self.focused;
                }
                self.modal_root = Some(root);
                if self
                    .focused
                    .is_none_or(|focused| !self.is_inside_modal(focused))
                {
                    self.focus_candidates()?.into_iter().next()
                } else {
                    self.focused
                }
            }
            None => {
                if self.modal_root.is_none() {
                    return Ok(self.focused);
                }
                self.modal_root = None;
                self.focus_before_modal
                    .take()
                    .filter(|node| self.is_focusable(*node))
            }
        };
        Ok(next)
    }

    pub fn control_state(&self, handle: NodeHandle) -> Result<Option<&ControlState>, EngineError> {
        self.resolve(handle)?;
        Ok(self.controls.get(&handle.index))
    }

    pub fn set_control_value(
        &mut self,
        handle: NodeHandle,
        value: impl Into<String>,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let value = value.into();
        let control = self
            .controls
            .get_mut(&handle.index)
            .ok_or(EngineError::NotAControl)?;
        control.value = value.clone();
        control.selection_anchor = value.len();
        control.selection_focus = value.len();
        control.clear_composition();
        control.history.clear();
        control.future.clear();
        self.document.set_attribute(node, "value", value);
        self.sync_selection_attributes(handle);
        self.mark_dirty(openui_style::InvalidationClass::Intrinsic);
        Ok(())
    }

    pub fn set_selection(
        &mut self,
        handle: NodeHandle,
        anchor: usize,
        focus: usize,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        let control = self
            .controls
            .get_mut(&handle.index)
            .ok_or(EngineError::NotEditable)?;
        if !is_editable_role(control.role) {
            return Err(EngineError::NotEditable);
        }
        if anchor > control.value.len() || focus > control.value.len() {
            return Err(EngineError::InvalidSelection);
        }
        control.finish_composition();
        control.selection_anchor = anchor;
        control.selection_focus = focus;
        control.clamp_selection();
        self.sync_selection_attributes(handle);
        self.mark_dirty(openui_style::InvalidationClass::Paint);
        Ok(())
    }

    /// Whether user text input may modify this control. Native application
    /// value setters and selection queries remain available for read-only controls.
    pub fn can_edit_text(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        let node = self.resolve(handle)?;
        Ok(self.controls.get(&handle.index).is_some_and(|control| {
            !control.disabled
                && is_editable_role(control.role)
                && self.document.attribute(node, "readonly").is_none()
        }))
    }

    pub fn insert_text(&mut self, handle: NodeHandle, text: &str) -> Result<(), EngineError> {
        if !self.can_edit_text(handle)? {
            return Err(EngineError::NotEditable);
        }
        let value = {
            let control = self
                .controls
                .get_mut(&handle.index)
                .ok_or(EngineError::NotEditable)?;
            control.replace_selection(text);
            control.value.clone()
        };
        let node = self.resolve(handle)?;
        self.document.set_attribute(node, "value", value);
        self.sync_selection_attributes(handle);
        self.mark_dirty(openui_style::InvalidationClass::Intrinsic);
        Ok(())
    }

    pub fn edit_text(
        &mut self,
        handle: NodeHandle,
        command: EditCommand,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if !matches!(command, EditCommand::Move { .. } | EditCommand::SelectAll)
            && !self.can_edit_text(handle)?
        {
            return Err(EngineError::NotEditable);
        }
        let (value, changed) = {
            let control = self
                .controls
                .get_mut(&handle.index)
                .ok_or(EngineError::NotEditable)?;
            if control.disabled || !is_editable_role(control.role) {
                return Err(EngineError::NotEditable);
            }
            control.finish_composition();
            let before = control.value.clone();
            match command {
                EditCommand::Move {
                    direction,
                    unit,
                    extend,
                } => control.move_selection(direction, unit, extend),
                EditCommand::Delete { direction, unit } => control.delete(direction, unit),
                EditCommand::SelectAll => {
                    control.selection_anchor = 0;
                    control.selection_focus = control.value.len();
                }
                EditCommand::Undo => control.undo(),
                EditCommand::Redo => control.redo(),
            }
            (control.value.clone(), before != control.value)
        };
        if changed {
            let node = self.resolve(handle)?;
            self.document.set_attribute(node, "value", value);
            self.mark_dirty(openui_style::InvalidationClass::Intrinsic);
        } else {
            self.mark_dirty(openui_style::InvalidationClass::Paint);
        }
        self.sync_selection_attributes(handle);
        Ok(())
    }

    pub fn update_composition(
        &mut self,
        handle: NodeHandle,
        text: &str,
    ) -> Result<(), EngineError> {
        if !self.can_edit_text(handle)? {
            return Err(EngineError::NotEditable);
        }
        let value = {
            let control = self
                .controls
                .get_mut(&handle.index)
                .ok_or(EngineError::NotEditable)?;
            control.update_composition(text);
            control.value.clone()
        };
        let node = self.resolve(handle)?;
        self.document.set_attribute(node, "value", value);
        self.sync_selection_attributes(handle);
        self.mark_dirty(openui_style::InvalidationClass::Intrinsic);
        Ok(())
    }

    pub fn finish_composition(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        self.resolve(handle)?;
        let control = self
            .controls
            .get_mut(&handle.index)
            .ok_or(EngineError::NotEditable)?;
        control.finish_composition();
        self.sync_selection_attributes(handle);
        self.mark_dirty(openui_style::InvalidationClass::Paint);
        Ok(())
    }

    /// Replace the preedit range with the final text as one undoable edit.
    pub fn commit_composition(
        &mut self,
        handle: NodeHandle,
        text: &str,
    ) -> Result<(), EngineError> {
        self.update_composition(handle, text)?;
        self.finish_composition(handle)
    }

    /// Restore the value and selection from before the first preedit update.
    pub fn cancel_composition(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let control = self
            .controls
            .get_mut(&handle.index)
            .ok_or(EngineError::NotEditable)?;
        let before = control.value.clone();
        control.cancel_composition();
        let changed = before != control.value;
        let value = control.value.clone();
        self.document.set_attribute(node, "value", value);
        self.sync_selection_attributes(handle);
        self.mark_dirty(if changed {
            openui_style::InvalidationClass::Intrinsic
        } else {
            openui_style::InvalidationClass::Paint
        });
        Ok(())
    }

    pub fn activate(&mut self, handle: NodeHandle) -> Result<ActivationResult, EngineError> {
        let node = self.resolve(handle)?;
        if self.is_disabled(handle) {
            return Ok(ActivationResult { changed: vec![] });
        }
        let Some(role) = self.controls.get(&handle.index).map(|state| state.role) else {
            if self.document.node(node).tag == ElementTag::Summary {
                if let Some(details) = self.parent(handle)? {
                    if self.element_tag(details)? == ElementTag::Details {
                        let open = self.attribute(details, "open")?.is_none();
                        if open {
                            self.set_attribute(details, "open", "")?;
                        } else {
                            self.remove_attribute(details, "open")?;
                        }
                        return Ok(ActivationResult {
                            changed: vec![details],
                        });
                    }
                }
            }
            return Ok(ActivationResult { changed: vec![] });
        };

        let mut changed = Vec::new();
        match role {
            FormControlRole::Checkbox => {
                let checked = !self.controls[&handle.index].checked;
                self.set_checked_internal(handle, checked);
                changed.push(handle);
            }
            FormControlRole::Radio => {
                let name = self
                    .attribute(handle, "name")?
                    .unwrap_or_default()
                    .to_owned();
                let handles: Vec<_> = self
                    .slots
                    .iter()
                    .enumerate()
                    .filter_map(|(index, slot)| {
                        slot.node.map(|_| self.handle_for_slot(index as u32))
                    })
                    .filter(|other| {
                        self.controls
                            .get(&other.index)
                            .is_some_and(|state| state.role == FormControlRole::Radio)
                            && (name.is_empty() && *other == handle
                                || !name.is_empty()
                                    && self
                                        .attribute(*other, "name")
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default()
                                        == name)
                    })
                    .collect();
                for other in handles {
                    let checked = other == handle;
                    if self.controls[&other.index].checked != checked {
                        self.set_checked_internal(other, checked);
                        changed.push(other);
                    }
                }
            }
            FormControlRole::Select => {
                let open = {
                    let control = self.controls.get_mut(&handle.index).expect("role exists");
                    control.open = !control.open;
                    control.open
                };
                self.sync_bool_attribute(handle, "open", open);
                changed.push(handle);
            }
            FormControlRole::Option => {
                let mut ancestor = self.parent(handle)?;
                while let Some(node) = ancestor {
                    if self
                        .controls
                        .get(&node.index)
                        .is_some_and(|state| state.role == FormControlRole::Select)
                    {
                        let value = self
                            .attribute(handle, "value")?
                            .map(str::to_owned)
                            .unwrap_or_default();
                        self.set_control_value(node, value)?;
                        if let Some(select) = self.controls.get_mut(&node.index) {
                            select.open = false;
                        }
                        self.sync_bool_attribute(node, "open", false);
                        changed.extend([handle, node]);
                        break;
                    }
                    ancestor = self.parent(node)?;
                }
            }
            _ => {}
        }
        if !changed.is_empty() {
            self.mark_dirty(openui_style::InvalidationClass::Paint);
        }
        Ok(ActivationResult { changed })
    }

    pub fn adjust_control(
        &mut self,
        handle: NodeHandle,
        adjustment: ControlAdjustment,
    ) -> Result<ActivationResult, EngineError> {
        self.resolve(handle)?;
        let state = self
            .controls
            .get(&handle.index)
            .cloned()
            .ok_or(EngineError::NotAControl)?;
        if state.disabled {
            return Ok(ActivationResult { changed: vec![] });
        }
        match state.role {
            FormControlRole::Range => {
                let current = finite_number(&state.value).unwrap_or(state.min);
                let delta = match adjustment {
                    ControlAdjustment::Previous => -state.step,
                    ControlAdjustment::Next => state.step,
                    ControlAdjustment::PageBackward => -state.step * 10.0,
                    ControlAdjustment::PageForward => state.step * 10.0,
                    ControlAdjustment::Minimum => state.min - current,
                    ControlAdjustment::Maximum => state.max - current,
                };
                let next =
                    (current + delta).clamp(state.min.min(state.max), state.max.max(state.min));
                if next == current {
                    return Ok(ActivationResult { changed: vec![] });
                }
                self.set_control_value(handle, format_control_number(next))?;
                Ok(ActivationResult {
                    changed: vec![handle],
                })
            }
            FormControlRole::Radio => {
                let name = self
                    .attribute(handle, "name")?
                    .unwrap_or_default()
                    .to_owned();
                if name.is_empty() {
                    return Ok(ActivationResult { changed: vec![] });
                }
                let radios: Vec<_> = self
                    .slots
                    .iter()
                    .enumerate()
                    .filter_map(|(index, slot)| {
                        slot.node.map(|_| self.handle_for_slot(index as u32))
                    })
                    .filter(|node| {
                        self.controls.get(&node.index).is_some_and(|control| {
                            control.role == FormControlRole::Radio && !control.disabled
                        }) && self.attribute(*node, "name").ok().flatten() == Some(name.as_str())
                    })
                    .collect();
                let Some(index) = radios.iter().position(|node| *node == handle) else {
                    return Ok(ActivationResult { changed: vec![] });
                };
                let target = match adjustment {
                    ControlAdjustment::Previous | ControlAdjustment::PageBackward => {
                        radios[(index + radios.len() - 1) % radios.len()]
                    }
                    ControlAdjustment::Next | ControlAdjustment::PageForward => {
                        radios[(index + 1) % radios.len()]
                    }
                    ControlAdjustment::Minimum => radios[0],
                    ControlAdjustment::Maximum => radios[radios.len() - 1],
                };
                self.activate(target)
            }
            FormControlRole::Select => {
                let mut options = Vec::new();
                self.collect_options(handle, &mut options)?;
                options.retain(|option| !self.is_disabled(*option));
                if options.is_empty() {
                    return Ok(ActivationResult { changed: vec![] });
                }
                let current = options
                    .iter()
                    .position(|option| self.controls[&option.index].selected)
                    .unwrap_or(0);
                let target = match adjustment {
                    ControlAdjustment::Previous | ControlAdjustment::PageBackward => {
                        current.saturating_sub(1)
                    }
                    ControlAdjustment::Next | ControlAdjustment::PageForward => {
                        (current + 1).min(options.len() - 1)
                    }
                    ControlAdjustment::Minimum => 0,
                    ControlAdjustment::Maximum => options.len() - 1,
                };
                let selected = options[target];
                let mut changed = Vec::new();
                for option in options {
                    let is_selected = option == selected;
                    if self.controls[&option.index].selected != is_selected {
                        if let Some(control) = self.controls.get_mut(&option.index) {
                            control.selected = is_selected;
                        }
                        self.sync_bool_attribute(option, "selected", is_selected);
                        changed.push(option);
                    }
                }
                let value = self
                    .attribute(selected, "value")?
                    .unwrap_or_default()
                    .to_owned();
                self.set_control_value(handle, value)?;
                changed.push(handle);
                Ok(ActivationResult { changed })
            }
            _ => Ok(ActivationResult { changed: vec![] }),
        }
    }

    /// Set a range control from a normalized track position. The value is
    /// clamped and snapped to the control's typed min/max/step state.
    pub fn set_range_fraction(
        &mut self,
        handle: NodeHandle,
        fraction: f32,
    ) -> Result<bool, EngineError> {
        self.resolve(handle)?;
        if !fraction.is_finite() {
            return Err(EngineError::InvalidInput("range fraction must be finite"));
        }
        let state = self
            .controls
            .get(&handle.index)
            .cloned()
            .ok_or(EngineError::NotAControl)?;
        if state.role != FormControlRole::Range || state.disabled {
            return Ok(false);
        }
        let low = state.min.min(state.max);
        let high = state.min.max(state.max);
        let raw = state.min + (state.max - state.min) * fraction.clamp(0.0, 1.0) as f64;
        let snapped = if state.step > 0.0 && state.step.is_finite() {
            (state.min + ((raw - state.min) / state.step).round() * state.step).clamp(low, high)
        } else {
            raw.clamp(low, high)
        };
        let current = finite_number(&state.value).unwrap_or(state.min);
        if (snapped - current).abs() <= f64::EPSILON {
            return Ok(false);
        }
        self.set_control_value(handle, format_control_number(snapped))?;
        Ok(true)
    }

    pub fn set_checked(&mut self, handle: NodeHandle, checked: bool) -> Result<(), EngineError> {
        self.resolve(handle)?;
        let role = self
            .controls
            .get(&handle.index)
            .map(|control| control.role)
            .ok_or(EngineError::NotAControl)?;
        if !matches!(role, FormControlRole::Checkbox | FormControlRole::Radio) {
            return Err(EngineError::NotAControl);
        }
        if role == FormControlRole::Radio && checked {
            self.activate(handle)?;
        } else {
            self.set_checked_internal(handle, checked);
            self.mark_dirty(openui_style::InvalidationClass::Paint);
        }
        Ok(())
    }

    pub fn set_indeterminate(
        &mut self,
        handle: NodeHandle,
        indeterminate: bool,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        let control = self
            .controls
            .get_mut(&handle.index)
            .filter(|control| control.role == FormControlRole::Checkbox)
            .ok_or(EngineError::NotAControl)?;
        control.indeterminate = indeterminate;
        self.sync_bool_attribute(handle, "indeterminate", indeterminate);
        self.mark_dirty(openui_style::InvalidationClass::Paint);
        Ok(())
    }

    pub fn is_hovered(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        self.resolve(handle)?;
        Ok(self.hover_paths.values().any(|path| path.contains(&handle)))
    }

    pub fn is_active(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        self.resolve(handle)?;
        Ok(self
            .active_pointers
            .values()
            .any(|active| *active == handle))
    }

    pub(crate) fn initialize_control(&mut self, handle: NodeHandle, tag: ElementTag) {
        if let Some(state) = ControlState::for_tag(tag) {
            let node = self.slots[handle.index as usize]
                .node
                .expect("new handle has native node");
            self.document.node_mut(node).form_control = Some(state.role);
            self.controls.insert(handle.index, state);
        }
    }

    pub(crate) fn sync_control_attribute(&mut self, handle: NodeHandle, name: &str, value: &str) {
        let is_input = self.element_tag(handle) == Ok(ElementTag::Input);
        let Some(control) = self.controls.get_mut(&handle.index) else {
            return;
        };
        match name {
            "disabled" => control.disabled = true,
            "checked" => control.checked = true,
            "selected" => control.selected = true,
            "open" => control.open = true,
            "indeterminate" => control.indeterminate = true,
            "value" => {
                control.clear_composition();
                control.value = value.to_owned();
                control.selection_anchor = value.len();
                control.selection_focus = value.len();
            }
            "placeholder" => control.placeholder = value.to_owned(),
            "min" => control.min = finite_number(value).unwrap_or(control.min),
            "max" => control.max = finite_number(value).unwrap_or(control.max),
            "step" => {
                control.step = finite_number(value)
                    .filter(|value| *value > 0.0)
                    .unwrap_or(1.0)
            }
            "type" if is_input => {
                control.role = match value.to_ascii_lowercase().as_str() {
                    "checkbox" => FormControlRole::Checkbox,
                    "radio" => FormControlRole::Radio,
                    "range" => FormControlRole::Range,
                    "color" => FormControlRole::ColorInput,
                    "date" => FormControlRole::DateInput,
                    "file" => FormControlRole::FileInput,
                    "button" | "submit" | "reset" => FormControlRole::Button,
                    _ => FormControlRole::TextInput,
                };
                if control.role == FormControlRole::Range && control.value.is_empty() {
                    control.value =
                        format_control_number(control.min + (control.max - control.min) / 2.0);
                }
                control.password = value.eq_ignore_ascii_case("password");
                if let Some(node) = self.slots[handle.index as usize].node {
                    self.document.node_mut(node).form_control = Some(control.role);
                }
            }
            _ => {}
        }
        if let Some(node) = self.slots[handle.index as usize].node {
            self.document.node_mut(node).form_control_disabled = control.disabled;
        }
    }

    pub(crate) fn remove_control_attribute(&mut self, handle: NodeHandle, name: &str) {
        let is_input = self.element_tag(handle) == Ok(ElementTag::Input);
        let Some(control) = self.controls.get_mut(&handle.index) else {
            return;
        };
        match name {
            "disabled" => control.disabled = false,
            "checked" => control.checked = false,
            "selected" => control.selected = false,
            "open" => control.open = false,
            "indeterminate" => control.indeterminate = false,
            "placeholder" => control.placeholder.clear(),
            "type" if is_input => {
                control.role = FormControlRole::TextInput;
                control.password = false;
            }
            _ => {}
        }
        if let Some(node) = self.slots[handle.index as usize].node {
            self.document.node_mut(node).form_control = Some(control.role);
            self.document.node_mut(node).form_control_disabled = control.disabled;
        }
    }

    fn path_to_root(&self, target: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let mut result = vec![target];
        let mut current = target;
        while let Some(parent) = self.parent(current)? {
            result.push(parent);
            current = parent;
        }
        Ok(result)
    }

    fn is_disabled(&self, handle: NodeHandle) -> bool {
        self.controls
            .get(&handle.index)
            .is_some_and(|control| control.disabled)
    }

    pub(crate) fn is_focusable(&self, handle: NodeHandle) -> bool {
        if self.resolve(handle).is_err()
            || self.is_disabled(handle)
            || !self.is_inside_modal(handle)
        {
            return false;
        }
        let tabindex = self
            .attribute(handle, "tabindex")
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i32>().ok());
        tabindex.is_some()
            || self.controls.contains_key(&handle.index)
            || self.element_tag(handle) == Ok(ElementTag::Summary)
    }

    fn is_inside_modal(&self, handle: NodeHandle) -> bool {
        let Some(modal) = self.modal_root else {
            return true;
        };
        let mut current = Some(handle);
        while let Some(node) = current {
            if node == modal {
                return true;
            }
            current = self.parent(node).ok().flatten();
        }
        false
    }

    fn focus_candidates(&self) -> Result<Vec<NodeHandle>, EngineError> {
        let root = self.modal_root.unwrap_or_else(|| self.root());
        let mut candidates = Vec::new();
        let mut order = 0usize;
        self.collect_focus_candidates(root, &mut order, &mut candidates)?;
        candidates.sort_by(|left, right| match (left.0, right.0) {
            (Some(a), Some(b)) if a > 0 && b > 0 => a.cmp(&b).then(left.1.cmp(&right.1)),
            (Some(a), _) if a > 0 => Ordering::Less,
            (_, Some(b)) if b > 0 => Ordering::Greater,
            _ => left.1.cmp(&right.1),
        });
        Ok(candidates.into_iter().map(|(_, _, node)| node).collect())
    }

    fn collect_focus_candidates(
        &self,
        node: NodeHandle,
        order: &mut usize,
        result: &mut Vec<(Option<i32>, usize, NodeHandle)>,
    ) -> Result<(), EngineError> {
        if self.is_focusable(node) {
            let tabindex = self
                .attribute(node, "tabindex")?
                .and_then(|value| value.parse::<i32>().ok());
            if !tabindex.is_some_and(|value| value < 0) {
                result.push((tabindex, *order, node));
            }
        }
        *order += 1;
        for child in self.children(node)? {
            self.collect_focus_candidates(child, order, result)?;
        }
        Ok(())
    }

    fn collect_focus_document_order(
        &self,
        node: NodeHandle,
        result: &mut Vec<NodeHandle>,
    ) -> Result<(), EngineError> {
        result.push(node);
        for child in self.children(node)? {
            self.collect_focus_document_order(child, result)?;
        }
        Ok(())
    }

    fn set_checked_internal(&mut self, handle: NodeHandle, checked: bool) {
        if let Some(control) = self.controls.get_mut(&handle.index) {
            control.checked = checked;
        }
        self.sync_bool_attribute(handle, "checked", checked);
    }

    fn collect_options(
        &self,
        node: NodeHandle,
        result: &mut Vec<NodeHandle>,
    ) -> Result<(), EngineError> {
        for child in self.children(node)? {
            if self
                .controls
                .get(&child.index)
                .is_some_and(|control| control.role == FormControlRole::Option)
            {
                result.push(child);
            }
            self.collect_options(child, result)?;
        }
        Ok(())
    }

    pub(crate) fn sync_bool_attribute(&mut self, handle: NodeHandle, name: &str, value: bool) {
        if let Ok(node) = self.resolve(handle) {
            if value {
                self.document.set_attribute(node, name, "");
            } else {
                self.document.node_mut(node).attributes.remove(name);
            }
        }
    }

    fn sync_selection_attributes(&mut self, handle: NodeHandle) {
        let Some(control) = self.controls.get(&handle.index) else {
            return;
        };
        let (anchor, focus, composition) = (
            control.selection_anchor,
            control.selection_focus,
            control.composition,
        );
        let Ok(node) = self.resolve(handle) else {
            return;
        };
        self.document
            .set_attribute(node, "data-oui-selection-anchor", anchor.to_string());
        self.document
            .set_attribute(node, "data-oui-selection-focus", focus.to_string());
        if let Some((start, end)) = composition {
            self.document
                .set_attribute(node, "data-oui-composition-start", start.to_string());
            self.document
                .set_attribute(node, "data-oui-composition-end", end.to_string());
        } else {
            self.document
                .node_mut(node)
                .attributes
                .remove("data-oui-composition-start");
            self.document
                .node_mut(node)
                .attributes
                .remove("data-oui-composition-end");
        }
    }
}

fn is_editable_role(role: FormControlRole) -> bool {
    matches!(role, FormControlRole::TextInput | FormControlRole::TextArea)
}

fn finite_number(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

fn format_control_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn previous_boundary(value: &str, offset: usize) -> usize {
    let offset = offset.min(value.len());
    value
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}

fn move_boundary(value: &str, offset: usize, direction: TextDirection, unit: TextUnit) -> usize {
    match unit {
        TextUnit::Document => match direction {
            TextDirection::Backward => 0,
            TextDirection::Forward => value.len(),
        },
        TextUnit::Line => {
            let offset = previous_boundary(value, offset);
            match direction {
                TextDirection::Backward => value[..offset].rfind('\n').map_or(0, |index| index + 1),
                TextDirection::Forward => value[offset..]
                    .find('\n')
                    .map_or(value.len(), |index| offset + index),
            }
        }
        TextUnit::Grapheme => {
            let boundaries: Vec<_> = value
                .grapheme_indices(true)
                .map(|(index, _)| index)
                .chain(std::iter::once(value.len()))
                .collect();
            match direction {
                TextDirection::Backward => boundaries
                    .into_iter()
                    .take_while(|index| *index < offset)
                    .last()
                    .unwrap_or(0),
                TextDirection::Forward => boundaries
                    .into_iter()
                    .find(|index| *index > offset)
                    .unwrap_or(value.len()),
            }
        }
        TextUnit::Word => {
            let mut boundaries = vec![0, value.len()];
            boundaries.extend(
                value
                    .split_word_bound_indices()
                    .flat_map(|(index, word)| [index, index + word.len()]),
            );
            boundaries.sort_unstable();
            boundaries.dedup();
            match direction {
                TextDirection::Backward => boundaries
                    .into_iter()
                    .take_while(|index| *index < offset)
                    .last()
                    .unwrap_or(0),
                TextDirection::Forward => boundaries
                    .into_iter()
                    .find(|index| *index > offset)
                    .unwrap_or(value.len()),
            }
        }
    }
}

fn common_root_suffix(a: &[NodeHandle], b: &[NodeHandle]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(a, b)| a == b)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::{Display, LengthValue, StyleProperty};

    fn sized_control(engine: &mut Engine, tag: ElementTag) -> NodeHandle {
        let node = engine.create_element(tag).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(40.0).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::px(40.0).into())
            .unwrap();
        node
    }

    #[test]
    fn event_route_is_capture_target_bubble() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap())
                .unwrap();
        let parent = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), parent).unwrap();
        let child = engine.create_element(ElementTag::Button).unwrap();
        engine.append_child(parent, child).unwrap();
        let route = engine.event_route(child).unwrap();
        assert_eq!(route.target, child);
        assert_eq!(route.steps.first().unwrap().phase, EventPhase::Capture);
        assert_eq!(
            route
                .steps
                .iter()
                .filter(|step| step.phase == EventPhase::Target)
                .count(),
            1
        );
        assert_eq!(route.steps.last().unwrap().phase, EventPhase::Bubble);
    }

    #[test]
    fn pointer_state_synthesizes_enter_leave_and_activation() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap())
                .unwrap();
        let button = sized_control(&mut engine, ElementTag::Button);
        let moved = engine
            .pointer_event(7, PointerEventKind::Move, 10.0, 10.0)
            .unwrap();
        assert_eq!(moved.target, Some(button));
        assert!(moved.entered.contains(&button));
        engine
            .pointer_event(7, PointerEventKind::Down, 10.0, 10.0)
            .unwrap();
        assert_eq!(
            engine
                .pointer_event(7, PointerEventKind::Up, 10.0, 10.0)
                .unwrap()
                .activation,
            Some(button)
        );
    }

    #[test]
    fn text_editing_uses_grapheme_boundaries_and_supports_undo() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap())
                .unwrap();
        let input = sized_control(&mut engine, ElementTag::Input);
        engine.set_control_value(input, "á👩‍💻z").unwrap();
        engine
            .edit_text(
                input,
                EditCommand::Delete {
                    direction: TextDirection::Backward,
                    unit: TextUnit::Grapheme,
                },
            )
            .unwrap();
        assert_eq!(engine.control_state(input).unwrap().unwrap().value, "á👩‍💻");
        engine.edit_text(input, EditCommand::Undo).unwrap();
        assert_eq!(engine.control_state(input).unwrap().unwrap().value, "á👩‍💻z");
    }

    #[test]
    fn checkbox_and_named_radio_activation_are_deterministic() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap())
                .unwrap();
        let checkbox = sized_control(&mut engine, ElementTag::Input);
        engine.set_attribute(checkbox, "type", "checkbox").unwrap();
        engine.activate(checkbox).unwrap();
        assert!(engine.control_state(checkbox).unwrap().unwrap().checked);

        let a = sized_control(&mut engine, ElementTag::Input);
        let b = sized_control(&mut engine, ElementTag::Input);
        for radio in [a, b] {
            engine.set_attribute(radio, "type", "radio").unwrap();
            engine.set_attribute(radio, "name", "group").unwrap();
        }
        engine.activate(a).unwrap();
        engine.activate(b).unwrap();
        assert!(!engine.control_state(a).unwrap().unwrap().checked);
        assert!(engine.control_state(b).unwrap().unwrap().checked);
    }

    #[test]
    fn input_button_type_attributes_preserve_button_role() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap())
                .unwrap();
        for input_type in ["button", "submit", "reset"] {
            let input = sized_control(&mut engine, ElementTag::Input);
            engine.set_attribute(input, "type", input_type).unwrap();
            assert_eq!(
                engine.control_state(input).unwrap().unwrap().role,
                FormControlRole::Button
            );
        }
    }
}
