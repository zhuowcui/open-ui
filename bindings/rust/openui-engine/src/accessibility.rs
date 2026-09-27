//! Retained semantic tree and AccessKit translation.

use crate::{ActivationResult, ControlAdjustment, Engine, EngineError, FocusOrigin, NodeHandle};
use accesskit::{
    Action, ActionData, ActionRequest, Live, Node, NodeId, Rect, Role, TextPosition, TextSelection,
    Toggled, TreeId, TreeInfo, TreeUpdate,
};
use openui_dom::{ElementTag, FormControlRole, NodeId as DomNodeId};
use std::collections::HashMap;

pub use accesskit::{
    Action as AccessibilityPlatformAction, ActionData as AccessibilityActionData,
    ActionRequest as AccessibilityActionRequest, Live as AccessibilityLive,
    Node as AccessibilityNode, NodeId as AccessibilityNodeId, Role as AccessibilityRole,
    Toggled as AccessibilityToggled, TreeUpdate as AccessibilityTreeUpdate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityRelation {
    LabelledBy,
    DescribedBy,
    Controls,
    Details,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessibilityAction {
    Click,
    Focus,
    Blur,
    Increment,
    Decrement,
    Expand,
    Collapse,
    SetValue(String),
    ReplaceSelectedText(String),
    SetTextSelection { anchor: usize, focus: usize },
    ScrollIntoView,
    ScrollBy { delta_x: f64, delta_y: f64 },
    SetScrollOffset { x: f64, y: f64 },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SemanticProperties {
    role: Option<Role>,
    label: Option<String>,
    description: Option<String>,
    value: Option<String>,
    live: Option<Live>,
    hidden: Option<bool>,
    labelled_by: Vec<NodeHandle>,
    described_by: Vec<NodeHandle>,
    controls: Vec<NodeHandle>,
    details: Vec<NodeHandle>,
}

impl SemanticProperties {
    pub(crate) fn remap_cloned_relations(&mut self, clones: &HashMap<NodeHandle, NodeHandle>) {
        for targets in [
            &mut self.labelled_by,
            &mut self.described_by,
            &mut self.controls,
            &mut self.details,
        ] {
            for target in targets {
                if let Some(clone) = clones.get(target) {
                    *target = *clone;
                }
            }
        }
    }
}

impl Engine {
    pub fn accessibility_node_id(&self, handle: NodeHandle) -> Result<NodeId, EngineError> {
        self.resolve(handle)?;
        Ok(node_id(handle))
    }

    pub fn set_accessibility_role(
        &mut self,
        handle: NodeHandle,
        role: Role,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.semantics.entry(handle.index).or_default().role = Some(role);
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn clear_accessibility_role(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if let Some(semantic) = self.semantics.get_mut(&handle.index) {
            semantic.role = None;
        }
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn set_accessibility_label(
        &mut self,
        handle: NodeHandle,
        label: impl Into<String>,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.semantics.entry(handle.index).or_default().label = Some(label.into());
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn set_accessibility_description(
        &mut self,
        handle: NodeHandle,
        description: impl Into<String>,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.semantics.entry(handle.index).or_default().description = Some(description.into());
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn set_accessibility_value(
        &mut self,
        handle: NodeHandle,
        value: impl Into<String>,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.semantics.entry(handle.index).or_default().value = Some(value.into());
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn set_accessibility_live(
        &mut self,
        handle: NodeHandle,
        live: Live,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.semantics.entry(handle.index).or_default().live = Some(live);
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn set_accessibility_hidden(
        &mut self,
        handle: NodeHandle,
        hidden: bool,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.semantics.entry(handle.index).or_default().hidden = Some(hidden);
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn set_accessibility_relation(
        &mut self,
        handle: NodeHandle,
        relation: AccessibilityRelation,
        targets: &[NodeHandle],
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        for target in targets {
            self.resolve(*target)?;
        }
        let semantics = self.semantics.entry(handle.index).or_default();
        let destination = match relation {
            AccessibilityRelation::LabelledBy => &mut semantics.labelled_by,
            AccessibilityRelation::DescribedBy => &mut semantics.described_by,
            AccessibilityRelation::Controls => &mut semantics.controls,
            AccessibilityRelation::Details => &mut semantics.details,
        };
        destination.clear();
        destination.extend_from_slice(targets);
        self.mark_dirty(openui_style::InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn prefers_reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    pub fn set_prefers_reduced_motion(&mut self, reduced: bool) {
        if self.reduced_motion != reduced {
            self.reduced_motion = reduced;
            self.mark_dirty(openui_style::InvalidationClass::Accessibility);
            let _ = self.sample_animations();
        }
    }

    /// Return a full initial AccessKit tree or the smallest node replacement
    /// set that brings the previously returned tree up to date.
    pub fn accessibility_update(&mut self) -> Result<TreeUpdate, EngineError> {
        let semantic_dirty = self.dirty.accessibility || !self.accessibility_initialized;
        self.update()?;
        let current = if semantic_dirty {
            self.build_accessibility_nodes()?
        } else {
            self.accessibility_nodes.clone()
        };
        let full = !self.accessibility_initialized;
        let mut nodes = Vec::new();
        if full {
            nodes.extend(current.iter().map(|(id, node)| (*id, node.clone())));
        } else if semantic_dirty {
            nodes.extend(
                current
                    .iter()
                    .filter(|(id, node)| self.accessibility_nodes.get(id) != Some(*node))
                    .map(|(id, node)| (*id, node.clone())),
            );
        }
        nodes.sort_by_key(|(id, _)| id.0);
        self.accessibility_nodes = current;
        self.accessibility_initialized = true;
        self.dirty.accessibility = false;
        let root = node_id(self.root());
        Ok(TreeUpdate {
            nodes,
            tree: full.then(|| {
                let mut tree = TreeInfo::new(root);
                tree.toolkit_name = Some("Open UI".into());
                tree.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
                tree
            }),
            tree_id: TreeId::ROOT,
            focus: self.focused.map(node_id).unwrap_or(root),
        })
    }

    /// Return a complete owned semantic tree without consuming the incremental
    /// AccessKit update stream. Callers can compare successive snapshots.
    pub fn accessibility_snapshot(&mut self) -> Result<Vec<(NodeId, Node)>, EngineError> {
        self.update()?;
        let mut nodes: Vec<_> = self.build_accessibility_nodes()?.into_iter().collect();
        nodes.sort_by_key(|(id, _)| id.0);
        Ok(nodes)
    }

    pub fn perform_accessibility_action(
        &mut self,
        handle: NodeHandle,
        action: AccessibilityAction,
    ) -> Result<ActivationResult, EngineError> {
        self.resolve(handle)?;
        match action {
            AccessibilityAction::Click => self.activate(handle),
            AccessibilityAction::Focus => {
                self.focus_with_origin(handle, FocusOrigin::Accessibility)?;
                Ok(ActivationResult { changed: vec![] })
            }
            AccessibilityAction::Blur => {
                self.blur(handle)?;
                Ok(ActivationResult { changed: vec![] })
            }
            AccessibilityAction::Increment => self.adjust_control(handle, ControlAdjustment::Next),
            AccessibilityAction::Decrement => {
                self.adjust_control(handle, ControlAdjustment::Previous)
            }
            AccessibilityAction::Expand => {
                let expanded = true;
                if self.element_tag(handle)? == ElementTag::Details {
                    self.set_attribute(handle, "open", "")?;
                } else {
                    self.sync_bool_attribute(handle, "open", expanded);
                }
                if let Some(control) = self.controls.get_mut(&handle.index) {
                    control.open = expanded;
                }
                self.mark_dirty(openui_style::InvalidationClass::Accessibility);
                Ok(ActivationResult {
                    changed: vec![handle],
                })
            }
            AccessibilityAction::Collapse => {
                let expanded = false;
                if self.element_tag(handle)? == ElementTag::Details {
                    self.remove_attribute(handle, "open")?;
                } else {
                    self.sync_bool_attribute(handle, "open", expanded);
                }
                if let Some(control) = self.controls.get_mut(&handle.index) {
                    control.open = expanded;
                }
                self.mark_dirty(openui_style::InvalidationClass::Accessibility);
                Ok(ActivationResult {
                    changed: vec![handle],
                })
            }
            AccessibilityAction::SetValue(value) => {
                self.set_control_value(handle, value)?;
                Ok(ActivationResult {
                    changed: vec![handle],
                })
            }
            AccessibilityAction::ReplaceSelectedText(value) => {
                self.insert_text(handle, &value)?;
                Ok(ActivationResult {
                    changed: vec![handle],
                })
            }
            AccessibilityAction::SetTextSelection { anchor, focus } => {
                self.set_selection(handle, anchor, focus)?;
                Ok(ActivationResult { changed: vec![] })
            }
            AccessibilityAction::ScrollIntoView => {
                self.scroll_into_view(handle)?;
                Ok(ActivationResult { changed: vec![] })
            }
            AccessibilityAction::ScrollBy { delta_x, delta_y } => {
                let (x, y) = self.scroll_offset(handle)?;
                self.scroll_to(handle, x + delta_x, y + delta_y)?;
                Ok(ActivationResult { changed: vec![] })
            }
            AccessibilityAction::SetScrollOffset { x, y } => {
                self.scroll_to(handle, x, y)?;
                Ok(ActivationResult { changed: vec![] })
            }
        }
    }

    /// Validate and lower a platform AccessKit request to the engine's typed
    /// action vocabulary. Text offsets are converted from characters to UTF-8
    /// byte boundaries before entering the editing pipeline.
    pub fn accessibility_action_from_request(
        &self,
        request: &ActionRequest,
    ) -> Result<(NodeHandle, AccessibilityAction), EngineError> {
        if request.target_tree != TreeId::ROOT {
            return Err(EngineError::InvalidInput(
                "accessibility action targets a foreign tree",
            ));
        }
        let handle = self.handle_from_accessibility_id(request.target_node)?;
        let action = match request.action {
            Action::Click => AccessibilityAction::Click,
            Action::Focus => AccessibilityAction::Focus,
            Action::Blur => AccessibilityAction::Blur,
            Action::Collapse => AccessibilityAction::Collapse,
            Action::Expand => AccessibilityAction::Expand,
            Action::Decrement => AccessibilityAction::Decrement,
            Action::Increment => AccessibilityAction::Increment,
            Action::ReplaceSelectedText => match request.data.as_ref() {
                Some(ActionData::Value(value)) => {
                    AccessibilityAction::ReplaceSelectedText(value.to_string())
                }
                _ => return Err(EngineError::InvalidInput("accessibility action needs text")),
            },
            Action::ScrollIntoView => AccessibilityAction::ScrollIntoView,
            Action::ScrollDown => AccessibilityAction::ScrollBy {
                delta_x: 0.0,
                delta_y: 40.0,
            },
            Action::ScrollUp => AccessibilityAction::ScrollBy {
                delta_x: 0.0,
                delta_y: -40.0,
            },
            Action::ScrollLeft => AccessibilityAction::ScrollBy {
                delta_x: -40.0,
                delta_y: 0.0,
            },
            Action::ScrollRight => AccessibilityAction::ScrollBy {
                delta_x: 40.0,
                delta_y: 0.0,
            },
            Action::SetScrollOffset => match request.data.as_ref() {
                Some(ActionData::SetScrollOffset(point)) => AccessibilityAction::SetScrollOffset {
                    x: point.x,
                    y: point.y,
                },
                _ => {
                    return Err(EngineError::InvalidInput(
                        "accessibility action needs a scroll offset",
                    ))
                }
            },
            Action::SetTextSelection => match request.data.as_ref() {
                Some(ActionData::SetTextSelection(selection)) => {
                    let control = self
                        .controls
                        .get(&handle.index)
                        .ok_or(EngineError::NotEditable)?;
                    AccessibilityAction::SetTextSelection {
                        anchor: character_to_byte(
                            &control.value,
                            selection.anchor.character_index,
                        )?,
                        focus: character_to_byte(&control.value, selection.focus.character_index)?,
                    }
                }
                _ => {
                    return Err(EngineError::InvalidInput(
                        "accessibility action needs a text selection",
                    ))
                }
            },
            Action::SetValue => match request.data.as_ref() {
                Some(ActionData::Value(value)) => AccessibilityAction::SetValue(value.to_string()),
                Some(ActionData::NumericValue(value)) => {
                    AccessibilityAction::SetValue(value.to_string())
                }
                _ => {
                    return Err(EngineError::InvalidInput(
                        "accessibility action needs a value",
                    ))
                }
            },
            _ => {
                return Err(EngineError::InvalidInput(
                    "unsupported accessibility action",
                ))
            }
        };
        Ok((handle, action))
    }

    fn handle_from_accessibility_id(&self, id: NodeId) -> Result<NodeHandle, EngineError> {
        let raw = id.0 & !(1_u64 << 63);
        let handle = NodeHandle {
            document: self.id,
            index: (raw >> 32) as u32,
            generation: raw as u32,
        };
        self.resolve(handle)?;
        Ok(handle)
    }

    fn build_accessibility_nodes(&self) -> Result<HashMap<NodeId, Node>, EngineError> {
        let mut nodes = HashMap::new();
        self.build_accessibility_subtree(self.root(), true, &mut nodes)?;
        Ok(nodes)
    }

    fn build_accessibility_subtree(
        &self,
        handle: NodeHandle,
        is_root: bool,
        nodes: &mut HashMap<NodeId, Node>,
    ) -> Result<(), EngineError> {
        let dom_id = self.resolve(handle)?;
        let data = self.document.node(dom_id);
        let authored = self.semantics.get(&handle.index);
        let control = self.controls.get(&handle.index);
        let role = authored
            .and_then(|semantic| semantic.role)
            .unwrap_or_else(|| inferred_role(data.tag, control.map(|state| state.role), is_root));
        let mut node = Node::new(role);
        let closed_details =
            data.tag == ElementTag::Details && self.document.attribute(dom_id, "open").is_none();
        let mut first_summary_seen = false;
        let visible_children: Vec<_> = self
            .children(handle)?
            .into_iter()
            .filter(|child| {
                if !closed_details {
                    return true;
                }
                let is_summary = self
                    .resolve(*child)
                    .is_ok_and(|node| self.document.node(node).tag == ElementTag::Summary);
                if is_summary && !first_summary_seen {
                    first_summary_seen = true;
                    true
                } else {
                    false
                }
            })
            .collect();
        let mut children: Vec<_> = visible_children.iter().copied().map(node_id).collect();
        if control.is_some_and(|state| is_editable(state.role)) {
            children.insert(0, text_node_id(handle));
        }
        node.set_children(children);

        let explicit_label = authored
            .and_then(|semantic| semantic.label.as_deref())
            .or_else(|| self.document.attribute(dom_id, "aria-label"))
            .or_else(|| {
                (data.tag == ElementTag::Image)
                    .then(|| self.document.attribute(dom_id, "alt"))
                    .flatten()
            });
        if let Some(label) = explicit_label {
            node.set_label(label);
        } else if role_needs_name(role) {
            let text = descendant_text(&self.document, dom_id);
            if !text.is_empty() {
                node.set_label(text);
            }
        }
        if let Some(description) = authored
            .and_then(|semantic| semantic.description.as_deref())
            .or_else(|| self.document.attribute(dom_id, "aria-description"))
        {
            node.set_description(description);
        }
        if let Some(value) = authored.and_then(|semantic| semantic.value.as_deref()) {
            node.set_value(value);
        }
        if let Some(semantic) = authored {
            if let Some(live) = semantic.live {
                node.set_live(live);
            }
            node.set_labelled_by(
                semantic
                    .labelled_by
                    .iter()
                    .copied()
                    .map(node_id)
                    .collect::<Vec<_>>(),
            );
            node.set_described_by(
                semantic
                    .described_by
                    .iter()
                    .copied()
                    .map(node_id)
                    .collect::<Vec<_>>(),
            );
            node.set_controls(
                semantic
                    .controls
                    .iter()
                    .copied()
                    .map(node_id)
                    .collect::<Vec<_>>(),
            );
            node.set_details(
                semantic
                    .details
                    .iter()
                    .copied()
                    .map(node_id)
                    .collect::<Vec<_>>(),
            );
        }
        let hidden = authored
            .and_then(|semantic| semantic.hidden)
            .unwrap_or(false)
            || self.document.attribute(dom_id, "aria-hidden") == Some("true")
            || data.style.visibility == openui_style::Visibility::Hidden
            || data.style.display == openui_style::Display::None;
        if hidden {
            node.set_hidden();
        }
        if self.document.attribute(dom_id, "required").is_some() {
            node.set_required();
        }
        if self.document.attribute(dom_id, "readonly").is_some() {
            node.set_read_only();
        }
        if self.modal_root == Some(handle) {
            node.set_modal();
        }
        if let Some(bounds) = self.accessibility_bounds(dom_id) {
            node.set_bounds(bounds);
        }
        if self.is_focusable(handle) {
            node.add_action(Action::Focus);
            node.add_action(Action::Blur);
        }
        if let Some(control) = control {
            populate_control_accessibility(&mut node, control);
        } else if matches!(data.tag, ElementTag::Summary) {
            node.add_action(Action::Click);
        } else if data.tag == ElementTag::Details {
            node.set_expanded(!closed_details);
        }
        if data.style.overflow_x != openui_style::Overflow::Visible
            || data.style.overflow_y != openui_style::Overflow::Visible
        {
            node.add_action(Action::ScrollDown);
            node.add_action(Action::ScrollUp);
            node.add_action(Action::ScrollLeft);
            node.add_action(Action::ScrollRight);
            node.set_scroll_x(data.scroll_left as f64);
            node.set_scroll_y(data.scroll_top as f64);
        }
        nodes.insert(node_id(handle), node);

        if let Some(control) = control.filter(|state| is_editable(state.role)) {
            let id = text_node_id(handle);
            let mut text = Node::new(Role::TextRun);
            text.set_value(&control.value);
            text.set_character_lengths(
                control
                    .value
                    .chars()
                    .map(|ch| ch.len_utf8() as u8)
                    .collect::<Vec<_>>(),
            );
            let byte_to_character = |offset: usize| {
                control.value[..offset.min(control.value.len())]
                    .chars()
                    .count()
            };
            text.set_text_selection(TextSelection {
                anchor: TextPosition {
                    node: id,
                    character_index: byte_to_character(control.selection_anchor),
                },
                focus: TextPosition {
                    node: id,
                    character_index: byte_to_character(control.selection_focus),
                },
            });
            if let Some(bounds) = self.accessibility_bounds(dom_id) {
                text.set_bounds(bounds);
            }
            nodes.insert(id, text);
        }
        for child in visible_children {
            self.build_accessibility_subtree(child, false, nodes)?;
        }
        Ok(())
    }

    fn accessibility_bounds(&self, dom_id: DomNodeId) -> Option<Rect> {
        self.hit_test
            .iter()
            .rev()
            .find(|entry| entry.node == dom_id)
            .map(|entry| {
                let corners = [
                    entry.local_to_world.map(0.0, 0.0),
                    entry.local_to_world.map(entry.width, 0.0),
                    entry.local_to_world.map(0.0, entry.height),
                    entry.local_to_world.map(entry.width, entry.height),
                ];
                let min_x = corners
                    .iter()
                    .map(|point| point.0)
                    .fold(f32::INFINITY, f32::min);
                let max_x = corners
                    .iter()
                    .map(|point| point.0)
                    .fold(f32::NEG_INFINITY, f32::max);
                let min_y = corners
                    .iter()
                    .map(|point| point.1)
                    .fold(f32::INFINITY, f32::min);
                let max_y = corners
                    .iter()
                    .map(|point| point.1)
                    .fold(f32::NEG_INFINITY, f32::max);
                Rect::new(min_x as f64, min_y as f64, max_x as f64, max_y as f64)
            })
    }

    fn scroll_into_view(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        let bounds = self.bounds(handle)?;
        let Some(bounds) = bounds else {
            return Ok(());
        };
        let mut ancestor = self.parent(handle)?;
        while let Some(node) = ancestor {
            let style = self.computed_style(node)?;
            if style.overflow_x != openui_style::Overflow::Visible
                || style.overflow_y != openui_style::Overflow::Visible
            {
                let (x, y) = self.scroll_offset(node)?;
                self.scroll_to(node, x.max(bounds.x as f64), y.max(bounds.y as f64))?;
                break;
            }
            ancestor = self.parent(node)?;
        }
        Ok(())
    }
}

fn populate_control_accessibility(node: &mut Node, control: &crate::ControlState) {
    if control.disabled {
        node.set_disabled();
    }
    match control.role {
        FormControlRole::Button => node.add_action(Action::Click),
        FormControlRole::Checkbox | FormControlRole::Radio => {
            node.set_toggled(if control.indeterminate {
                Toggled::Mixed
            } else {
                control.checked.into()
            });
            node.add_action(Action::Click);
        }
        FormControlRole::TextInput | FormControlRole::TextArea => {
            node.set_value(&control.value);
            if !control.placeholder.is_empty() {
                node.set_placeholder(&control.placeholder);
            }
            node.add_action(Action::SetValue);
            node.add_action(Action::ReplaceSelectedText);
            node.add_action(Action::SetTextSelection);
        }
        FormControlRole::Select => {
            node.set_value(&control.value);
            node.set_expanded(control.open);
            node.add_action(Action::Click);
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
        }
        FormControlRole::Option => {
            node.set_selected(control.selected);
            node.add_action(Action::Click);
        }
        FormControlRole::Range => {
            let value = control.value.parse::<f64>().unwrap_or(control.min);
            node.set_numeric_value(value);
            node.set_min_numeric_value(control.min);
            node.set_max_numeric_value(control.max);
            node.set_numeric_value_step(control.step);
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
            node.add_action(Action::SetValue);
        }
        _ => {}
    }
}

fn inferred_role(tag: ElementTag, control: Option<FormControlRole>, is_root: bool) -> Role {
    if is_root {
        return Role::Window;
    }
    if let Some(control) = control {
        return match control {
            FormControlRole::Button => Role::Button,
            FormControlRole::TextInput => Role::TextInput,
            FormControlRole::ColorInput => Role::ColorWell,
            FormControlRole::DateInput => Role::DateInput,
            FormControlRole::FileInput => Role::Button,
            FormControlRole::Checkbox => Role::CheckBox,
            FormControlRole::Radio => Role::RadioButton,
            FormControlRole::TextArea => Role::MultilineTextInput,
            FormControlRole::Select => Role::ComboBox,
            FormControlRole::Option => Role::ListBoxOption,
            FormControlRole::OptGroup => Role::Group,
            FormControlRole::Range => Role::Slider,
            FormControlRole::Meter => Role::Meter,
            FormControlRole::Progress => Role::ProgressIndicator,
            FormControlRole::Fieldset => Role::Group,
            FormControlRole::Legend => Role::Legend,
        };
    }
    match tag {
        ElementTag::Text => Role::TextRun,
        ElementTag::Break => Role::LineBreak,
        ElementTag::Image => Role::Image,
        ElementTag::Canvas => Role::Canvas,
        ElementTag::Svg => Role::SvgRoot,
        ElementTag::IFrame => Role::Iframe,
        ElementTag::Object | ElementTag::Embed => Role::EmbeddedObject,
        ElementTag::Audio => Role::Audio,
        ElementTag::Video => Role::Video,
        ElementTag::Table => Role::Table,
        ElementTag::TableCaption => Role::Caption,
        ElementTag::TableHead | ElementTag::TableBody | ElementTag::TableFoot => Role::RowGroup,
        ElementTag::TableRow => Role::Row,
        ElementTag::TableCell => Role::Cell,
        ElementTag::TableHeaderCell => Role::ColumnHeader,
        ElementTag::Details => Role::Details,
        ElementTag::Summary => Role::DisclosureTriangle,
        ElementTag::Form => Role::Form,
        ElementTag::Ruby => Role::Ruby,
        ElementTag::RubyText => Role::RubyAnnotation,
        _ => Role::GenericContainer,
    }
}

fn role_needs_name(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::CheckBox
            | Role::RadioButton
            | Role::ComboBox
            | Role::ListBoxOption
            | Role::DisclosureTriangle
            | Role::Image
    )
}

fn is_editable(role: FormControlRole) -> bool {
    matches!(role, FormControlRole::TextInput | FormControlRole::TextArea)
}

fn node_id(handle: NodeHandle) -> NodeId {
    NodeId(((handle.index as u64) << 32) | handle.generation as u64)
}

fn text_node_id(handle: NodeHandle) -> NodeId {
    NodeId(node_id(handle).0 | (1_u64 << 63))
}

fn character_to_byte(value: &str, character: usize) -> Result<usize, EngineError> {
    if character == value.chars().count() {
        return Ok(value.len());
    }
    value
        .char_indices()
        .nth(character)
        .map(|(offset, _)| offset)
        .ok_or(EngineError::InvalidSelection)
}

fn descendant_text(document: &openui_dom::Document, node: DomNodeId) -> String {
    let mut result = document.node(node).text.clone().unwrap_or_default();
    for child in document.children(node) {
        result.push_str(&descendant_text(document, child));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::{Display, LengthValue, StyleProperty};

    fn mounted_control(engine: &mut Engine, tag: ElementTag) -> NodeHandle {
        let node = engine.create_element(tag).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(80.0).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::px(24.0).into())
            .unwrap();
        node
    }

    #[test]
    fn initial_tree_infers_control_semantics_bounds_and_stable_ids() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap())
                .unwrap();
        let checkbox = mounted_control(&mut engine, ElementTag::Input);
        engine.set_attribute(checkbox, "type", "checkbox").unwrap();
        engine.set_accessibility_label(checkbox, "Ship").unwrap();
        let update = engine.accessibility_update().unwrap();
        assert!(update.tree.is_some());
        assert_eq!(
            update.focus,
            engine.accessibility_node_id(engine.root()).unwrap()
        );
        let (_, node) = update
            .nodes
            .iter()
            .find(|(id, _)| *id == engine.accessibility_node_id(checkbox).unwrap())
            .unwrap();
        assert_eq!(node.role(), Role::CheckBox);
        assert_eq!(node.label(), Some("Ship"));
        assert!(node.bounds().is_some());
        assert!(node.supports_action(Action::Click));
    }

    #[test]
    fn cloned_subtree_relations_target_cloned_descendants() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap())
                .unwrap();
        let group = engine.create_element(ElementTag::Div).unwrap();
        let label = engine.create_element(ElementTag::Span).unwrap();
        engine.append_child(group, label).unwrap();
        engine
            .set_accessibility_relation(group, AccessibilityRelation::LabelledBy, &[label])
            .unwrap();
        let cloned_group = engine.clone_subtree(group).unwrap();
        let cloned_label = engine.children(cloned_group).unwrap()[0];
        engine.append_child(engine.root(), cloned_group).unwrap();
        let snapshot = engine.accessibility_snapshot().unwrap();
        let cloned_id = engine.accessibility_node_id(cloned_group).unwrap();
        let label_id = engine.accessibility_node_id(cloned_label).unwrap();
        let (_, node) = snapshot.iter().find(|(id, _)| *id == cloned_id).unwrap();
        assert_eq!(node.labelled_by(), &[label_id]);
    }

    #[test]
    fn unchanged_updates_are_empty_and_actions_share_control_state() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap())
                .unwrap();
        let checkbox = mounted_control(&mut engine, ElementTag::Input);
        engine.set_attribute(checkbox, "type", "checkbox").unwrap();
        engine.accessibility_update().unwrap();
        let unchanged = engine.accessibility_update().unwrap();
        assert!(unchanged.nodes.is_empty());
        engine
            .perform_accessibility_action(checkbox, AccessibilityAction::Click)
            .unwrap();
        assert!(engine.control_state(checkbox).unwrap().unwrap().checked);
        let changed = engine.accessibility_update().unwrap();
        assert_eq!(changed.nodes.len(), 1);
        assert_eq!(changed.nodes[0].1.toggled(), Some(Toggled::True));
    }

    #[test]
    fn complete_snapshot_does_not_consume_incremental_updates() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap())
                .unwrap();
        let button = mounted_control(&mut engine, ElementTag::Button);
        engine.set_accessibility_label(button, "Run").unwrap();
        let id = engine.accessibility_node_id(button).unwrap();
        let snapshot = engine.accessibility_snapshot().unwrap();
        assert_eq!(
            snapshot
                .iter()
                .find(|(node_id, _)| *node_id == id)
                .unwrap()
                .1
                .label(),
            Some("Run")
        );
        assert!(engine.accessibility_update().unwrap().tree.is_some());

        engine.set_accessibility_label(button, "Stop").unwrap();
        let snapshot = engine.accessibility_snapshot().unwrap();
        assert_eq!(
            snapshot
                .iter()
                .find(|(node_id, _)| *node_id == id)
                .unwrap()
                .1
                .label(),
            Some("Stop")
        );
        let update = engine.accessibility_update().unwrap();
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.nodes[0].0, id);
    }

    #[test]
    fn editable_controls_expose_text_runs_and_selection() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap())
                .unwrap();
        let input = mounted_control(&mut engine, ElementTag::Input);
        engine.set_control_value(input, "a👩‍💻").unwrap();
        engine.set_selection(input, 1, "a👩‍💻".len()).unwrap();
        let update = engine.accessibility_update().unwrap();
        let text = update
            .nodes
            .iter()
            .find(|(id, _)| *id == text_node_id(input))
            .unwrap();
        assert_eq!(text.1.role(), Role::TextRun);
        assert!(text.1.text_selection().is_some());
    }

    #[test]
    fn platform_requests_validate_ids_and_convert_character_offsets() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap())
                .unwrap();
        let input = mounted_control(&mut engine, ElementTag::Input);
        engine.set_control_value(input, "a👩‍💻z").unwrap();
        let request = ActionRequest {
            action: Action::SetTextSelection,
            target_tree: TreeId::ROOT,
            target_node: text_node_id(input),
            data: Some(ActionData::SetTextSelection(TextSelection {
                anchor: TextPosition {
                    node: text_node_id(input),
                    character_index: 1,
                },
                focus: TextPosition {
                    node: text_node_id(input),
                    character_index: 4,
                },
            })),
        };
        let (target, action) = engine.accessibility_action_from_request(&request).unwrap();
        assert_eq!(target, input);
        assert_eq!(
            action,
            AccessibilityAction::SetTextSelection {
                anchor: 1,
                focus: "a👩‍💻".len(),
            }
        );
        engine.perform_accessibility_action(target, action).unwrap();
        assert_eq!(
            engine.control_state(input).unwrap().unwrap().selection(),
            (1, "a👩‍💻".len())
        );
    }
}
