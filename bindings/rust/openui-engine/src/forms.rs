//! Native form ownership and ordered radio registration over the retained tree.

use crate::{Engine, EngineError, NodeHandle};
use openui_dom::{ElementTag, FormControlRole, NodeId};
use openui_style::InvalidationClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RadioGroupScope {
    Document,
    Form(NodeHandle),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RadioAssociation {
    scope: Option<RadioGroupScope>,
    name: String,
}

impl Engine {
    /// Query the current associated form without layout or mutation.
    /// Non-form-associated native kinds return `None`. Detached controls use
    /// their nearest form ancestor; connected explicit associations use the
    /// first ID match, which must itself be a form. Invalid handles fail.
    pub fn associated_form(&self, handle: NodeHandle) -> Result<Option<NodeHandle>, EngineError> {
        let tag = self.element_tag(handle)?;
        if !matches!(
            tag,
            ElementTag::Input
                | ElementTag::Button
                | ElementTag::Select
                | ElementTag::TextArea
                | ElementTag::Fieldset
                | ElementTag::Object
        ) {
            return Ok(None);
        }
        self.form_owner(handle)
    }

    pub(crate) fn form_owner(&self, handle: NodeHandle) -> Result<Option<NodeHandle>, EngineError> {
        self.resolve(handle)?;
        if let Some(id) = self.attribute(handle, "form")? {
            if self.is_connected(handle)? {
                return Ok(self
                    .element_by_id(id)
                    .filter(|candidate| self.element_tag(*candidate) == Ok(ElementTag::Form)));
            }
        }
        let mut current = self.parent(handle)?;
        while let Some(parent) = current {
            if self.element_tag(parent)? == ElementTag::Form {
                return Ok(Some(parent));
            }
            current = self.parent(parent)?;
        }
        Ok(None)
    }

    fn radio_association(
        &self,
        handle: NodeHandle,
    ) -> Result<Option<RadioAssociation>, EngineError> {
        self.resolve(handle)?;
        if !self
            .controls
            .get(&handle.index)
            .is_some_and(|c| c.role == FormControlRole::Radio)
        {
            return Ok(None);
        }
        let scope = if let Some(form) = self.form_owner(handle)? {
            Some(RadioGroupScope::Form(form))
        } else if self.is_connected(handle)? {
            Some(RadioGroupScope::Document)
        } else {
            None
        };
        Ok(Some(RadioAssociation {
            scope,
            name: self.attribute(handle, "name")?.unwrap_or_default().into(),
        }))
    }

    /// Keep registration state through removal and insertion separately.
    /// Scanning only the final tree loses temporary form-owner transitions and
    /// makes the first checked descendant win rather than the last inserted one.
    pub(crate) fn reset_radio_association(
        &mut self,
        handle: NodeHandle,
    ) -> Result<(), EngineError> {
        let Some(association) = self.radio_association(handle)? else {
            self.radio_associations.remove(&handle.index);
            return Ok(());
        };
        if self.radio_associations.get(&handle.index) == Some(&association) {
            return Ok(());
        }
        self.radio_associations.remove(&handle.index);
        if association.scope.is_some()
            && !association.name.is_empty()
            && self.controls[&handle.index].checked
        {
            let peers: Vec<_> = self
                .radio_associations
                .iter()
                .filter(|(_, peer)| {
                    peer.scope == association.scope && peer.name == association.name
                })
                .map(|(index, _)| self.handle_for_slot(*index))
                .collect();
            for peer in peers {
                if self.controls[&peer.index].checked {
                    self.set_checked_internal(peer, false);
                    self.mark_dirty(InvalidationClass::Paint);
                }
            }
        }
        self.radio_associations.insert(handle.index, association);
        Ok(())
    }

    pub(crate) fn radio_peers_to_uncheck(
        &self,
        handle: NodeHandle,
    ) -> Result<Vec<NodeHandle>, EngineError> {
        let Some(association) = self.radio_association(handle)? else {
            return Ok(Vec::new());
        };
        if association.name.is_empty() {
            return Ok(Vec::new());
        }
        if association.scope.is_none() {
            // An unowned detached radio has no managed group. Its checked
            // setter clears the first checked peer in detached tree order.
            return Ok(self
                .radio_group_members(handle)?
                .into_iter()
                .find(|peer| *peer != handle && self.controls[&peer.index].checked)
                .into_iter()
                .collect());
        }
        Ok(self
            .radio_associations
            .iter()
            .filter(|(index, peer)| {
                **index != handle.index
                    && peer.scope == association.scope
                    && peer.name == association.name
            })
            .map(|(index, _)| self.handle_for_slot(*index))
            .collect())
    }

    fn reset_radio_id_observers(
        &mut self,
        ids: &[&str],
        excluded: &[NodeHandle],
    ) -> Result<(), EngineError> {
        let mut candidates: Vec<_> = self.radio_associations.keys().copied().collect();
        candidates.sort_unstable();
        for id in ids.iter().filter(|id| !id.is_empty()) {
            for index in &candidates {
                let handle = self.handle_for_slot(*index);
                if !excluded.contains(&handle)
                    && self.is_connected(handle)?
                    && self.attribute(handle, "form")? == Some(*id)
                {
                    self.reset_radio_association(handle)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn radio_id_targets_changed(&mut self, ids: &[&str]) -> Result<(), EngineError> {
        self.reset_radio_id_observers(ids, &[])
    }

    fn radio_tree_change(&mut self, node: NodeId, inserting: bool) -> Result<(), EngineError> {
        let mut handles = Vec::new();
        let mut ids = Vec::new();
        let mut stack = vec![node];
        while let Some(node) = stack.pop() {
            if let Some(index) = self.node_slots.get(&node) {
                handles.push(self.handle_for_slot(*index));
            }
            if let Some(id) = self
                .document
                .attribute(node, "id")
                .filter(|id| !id.is_empty())
            {
                if !ids.iter().any(|candidate| candidate == id) {
                    ids.push(id.to_owned());
                }
            }
            stack.extend(
                self.document
                    .children(node)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev(),
            );
        }
        let ids: Vec<_> = ids.iter().map(String::as_str).collect();
        if inserting {
            self.reset_radio_id_observers(&ids, &handles)?;
        }
        for handle in &handles {
            self.reset_radio_association(*handle)?;
        }
        if !inserting {
            self.reset_radio_id_observers(&ids, &handles)?;
        }
        Ok(())
    }

    pub(crate) fn reconcile_radio_subtree(&mut self, node: NodeId) -> Result<(), EngineError> {
        self.radio_tree_change(node, true)
    }

    pub(crate) fn remove_radio_subtree(&mut self, node: NodeId) -> Result<(), EngineError> {
        self.radio_tree_change(node, false)
    }
}
