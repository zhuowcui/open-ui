//! Owned, versioned C snapshots of the shared AccessKit semantic tree.

use super::*;
use openui_engine::{
    AccessibilityNode, AccessibilityPlatformAction as Action, AccessibilityToggled,
};

fn snapshot(address: usize) -> Result<Rc<AccessibilitySnapshotState>, ApiError> {
    match get(address, HandleKind::AccessibilitySnapshot)? {
        LocalHandle::AccessibilitySnapshot(snapshot) => Ok(snapshot),
        _ => Err(invalid("accessibility snapshot handle has the wrong type")),
    }
}

fn node_by_id(
    snapshot: &AccessibilitySnapshotState,
    id: u64,
) -> Result<&AccessibilityNode, ApiError> {
    snapshot
        .nodes
        .binary_search_by_key(&id, |(node_id, _)| node_id.0)
        .ok()
        .map(|index| &snapshot.nodes[index].1)
        .ok_or_else(|| invalid("accessibility node ID is absent from this snapshot"))
}

fn role_number(role: AccessibilityRole) -> u32 {
    match role {
        AccessibilityRole::GenericContainer => 1,
        AccessibilityRole::Button => 2,
        AccessibilityRole::CheckBox => 3,
        AccessibilityRole::RadioButton => 4,
        AccessibilityRole::TextInput => 5,
        AccessibilityRole::MultilineTextInput => 6,
        AccessibilityRole::ComboBox => 7,
        AccessibilityRole::ListBoxOption => 8,
        AccessibilityRole::Slider => 9,
        AccessibilityRole::Image => 10,
        AccessibilityRole::Link => 11,
        AccessibilityRole::Dialog => 12,
        AccessibilityRole::Heading => 13,
        AccessibilityRole::Status => 14,
        AccessibilityRole::Alert => 15,
        _ => 16,
    }
}

// SAFETY CONTRACT: `document` and optional `previous` are live on this thread;
// `out_snapshot` points to writable handle storage. The returned snapshot owns
// its nodes and remains readable after the document is destroyed.
#[no_mangle]
pub extern "C" fn oui_document_accessibility_snapshot(
    document_handle: *mut OuiDocument,
    previous_handle: *const OuiAccessibilitySnapshot,
    out_snapshot: *mut *mut OuiAccessibilitySnapshot,
) -> OuiStatus {
    ffi(|| {
        if out_snapshot.is_null() {
            return Err(invalid("accessibility snapshot output is null"));
        }
        let state = document(document_handle as usize)?;
        let previous = if previous_handle.is_null() {
            None
        } else {
            let previous = snapshot(previous_handle as usize)?;
            if !previous.document.ptr_eq(&Rc::downgrade(&state)) {
                return Err(ApiError::new(
                    OuiStatus::WrongDocument,
                    "previous accessibility snapshot belongs to another document",
                ));
            }
            Some(previous)
        };
        let mut engine = borrow_engine_mut(&state)?;
        let nodes = engine.accessibility_snapshot()?;
        let generation = engine.dirty_generations().accessibility;
        let focus_id = engine
            .accessibility_node_id(engine.focused().unwrap_or_else(|| engine.root()))?
            .0;
        let reduced_motion = engine.prefers_reduced_motion();
        drop(engine);

        let mut changed = Vec::new();
        let mut removed = Vec::new();
        if let Some(previous) = &previous {
            let mut old_index = 0;
            let mut new_index = 0;
            while old_index < previous.nodes.len() || new_index < nodes.len() {
                match (previous.nodes.get(old_index), nodes.get(new_index)) {
                    (Some((old_id, _)), Some((new_id, _))) if old_id.0 < new_id.0 => {
                        removed.push(old_id.0);
                        old_index += 1;
                    }
                    (Some((old_id, _)), Some((new_id, new_node))) if old_id == new_id => {
                        if previous.nodes[old_index].1 != *new_node {
                            changed.push(new_id.0);
                        }
                        old_index += 1;
                        new_index += 1;
                    }
                    (_, Some((new_id, _))) => {
                        changed.push(new_id.0);
                        new_index += 1;
                    }
                    (Some((old_id, _)), None) => {
                        removed.push(old_id.0);
                        old_index += 1;
                    }
                    (None, None) => break,
                }
            }
        } else {
            changed.extend(nodes.iter().map(|(id, _)| id.0));
        }
        write_handle(
            out_snapshot,
            LocalHandle::AccessibilitySnapshot(Rc::new(AccessibilitySnapshotState {
                document: Rc::downgrade(&state),
                generation,
                focus_id,
                reduced_motion,
                nodes,
                changed,
                removed,
                full_tree: previous.is_none(),
            })),
        )
    })
}

// SAFETY CONTRACT: `snapshot` is a live snapshot handle on its owning thread.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_destroy(
    snapshot_handle: *mut OuiAccessibilitySnapshot,
) -> OuiStatus {
    ffi(|| {
        destroy(snapshot_handle as usize, HandleKind::AccessibilitySnapshot)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `out_info` is readable and writable with an initialized header.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_get_info(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    out_info: *mut OuiAccessibilitySnapshotInfo,
) -> OuiStatus {
    ffi(|| {
        if out_info.is_null() {
            return Err(invalid("accessibility snapshot info output is null"));
        }
        let info = unsafe { &mut *out_info };
        check_header(
            info.struct_size,
            info.abi_version,
            size_of::<OuiAccessibilitySnapshotInfo>(),
        )?;
        let snapshot = snapshot(snapshot_handle as usize)?;
        info.generation = snapshot.generation;
        info.focus_id = snapshot.focus_id;
        info.node_count = snapshot.nodes.len();
        info.changed_count = snapshot.changed.len();
        info.removed_count = snapshot.removed.len();
        info.full_tree = u8::from(snapshot.full_tree);
        info.reduced_motion = u8::from(snapshot.reduced_motion);
        info.reserved = [0; 6];
        Ok(())
    })
}

// SAFETY CONTRACT: `out_node` is readable and writable with an initialized header.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_get_node(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    index: usize,
    out_node: *mut OuiAccessibilityNodeInfo,
) -> OuiStatus {
    ffi(|| {
        if out_node.is_null() {
            return Err(invalid("accessibility node info output is null"));
        }
        let output = unsafe { &mut *out_node };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiAccessibilityNodeInfo>(),
        )?;
        let snapshot = snapshot(snapshot_handle as usize)?;
        let (id, node) = snapshot
            .nodes
            .get(index)
            .ok_or_else(|| invalid("accessibility node index is out of bounds"))?;
        let bounds = node.bounds();
        let mut flags = 0;
        if node.is_hidden() {
            flags |= OUI_ACCESSIBILITY_NODE_HIDDEN;
        }
        if node.is_required() {
            flags |= OUI_ACCESSIBILITY_NODE_REQUIRED;
        }
        if node.is_read_only() {
            flags |= OUI_ACCESSIBILITY_NODE_READ_ONLY;
        }
        if node.is_modal() {
            flags |= OUI_ACCESSIBILITY_NODE_MODAL;
        }
        if bounds.is_some() {
            flags |= OUI_ACCESSIBILITY_NODE_HAS_BOUNDS;
        }
        let supported = [
            Action::Click,
            Action::Focus,
            Action::Blur,
            Action::Increment,
            Action::Decrement,
            Action::Expand,
            Action::Collapse,
            Action::ScrollIntoView,
            Action::SetValue,
            Action::ReplaceSelectedText,
            Action::SetTextSelection,
            Action::ScrollDown,
            Action::ScrollUp,
            Action::ScrollLeft,
            Action::ScrollRight,
        ];
        let actions = supported
            .iter()
            .enumerate()
            .fold(0u32, |bits, (index, action)| {
                bits | (u32::from(node.supports_action(*action)) << index)
            });
        output.id = id.0;
        output.role = role_number(node.role());
        output.flags = flags;
        output.actions = actions;
        output.reserved = 0;
        output.bounds = bounds.map_or(
            OuiRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            |rect| OuiRect {
                x: rect.x0 as f32,
                y: rect.y0 as f32,
                width: (rect.x1 - rect.x0) as f32,
                height: (rect.y1 - rect.y0) as f32,
            },
        );
        output.child_count = node.children().len();
        output.labelled_by_count = node.labelled_by().len();
        output.described_by_count = node.described_by().len();
        output.controls_count = node.controls().len();
        output.details_count = node.details().len();
        output.label_length = node.label().map_or(0, str::len);
        output.description_length = node.description().map_or(0, str::len);
        output.value_length = node.value().map_or(0, str::len);
        output.role_name_length = format!("{:?}", node.role()).len();
        Ok(())
    })
}

// SAFETY CONTRACT: `out_state` is readable and writable with an initialized header.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_get_node_state(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    node_id: u64,
    out_state: *mut OuiAccessibilityNodeState,
) -> OuiStatus {
    ffi(|| {
        if out_state.is_null() {
            return Err(invalid("accessibility node state output is null"));
        }
        let output = unsafe { &mut *out_state };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiAccessibilityNodeState>(),
        )?;
        let snapshot = snapshot(snapshot_handle as usize)?;
        let node = node_by_id(&snapshot, node_id)?;
        let mut flags = u32::from(node.is_disabled());
        if let Some(selected) = node.is_selected() {
            flags |= 1 << 1;
            if selected {
                flags |= 1 << 2;
            }
        }
        if let Some(expanded) = node.is_expanded() {
            flags |= 1 << 3;
            if expanded {
                flags |= 1 << 4;
            }
        }
        if let Some(toggled) = node.toggled() {
            flags |= 1 << 5;
            match toggled {
                AccessibilityToggled::False => {}
                AccessibilityToggled::True => flags |= 1 << 6,
                AccessibilityToggled::Mixed => flags |= 1 << 7,
            }
        }
        let numeric_value = node.numeric_value();
        let numeric_min = node.min_numeric_value();
        let numeric_max = node.max_numeric_value();
        let numeric_step = node.numeric_value_step();
        let scroll_x = node.scroll_x();
        let scroll_y = node.scroll_y();
        if numeric_value.is_some() {
            flags |= 1 << 8;
        }
        if numeric_min.is_some() {
            flags |= 1 << 9;
        }
        if numeric_max.is_some() {
            flags |= 1 << 10;
        }
        if numeric_step.is_some() {
            flags |= 1 << 11;
        }
        if scroll_x.is_some() {
            flags |= 1 << 12;
        }
        if scroll_y.is_some() {
            flags |= 1 << 13;
        }
        let selection = node.text_selection();
        if selection.is_some() {
            flags |= 1 << 14;
        }
        output.flags = flags;
        output.live = match node.live() {
            None => 0,
            Some(AccessibilityLive::Off) => 1,
            Some(AccessibilityLive::Polite) => 2,
            Some(AccessibilityLive::Assertive) => 3,
        };
        output.numeric_value = numeric_value.unwrap_or(0.0);
        output.numeric_min = numeric_min.unwrap_or(0.0);
        output.numeric_max = numeric_max.unwrap_or(0.0);
        output.numeric_step = numeric_step.unwrap_or(0.0);
        output.scroll_x = scroll_x.unwrap_or(0.0);
        output.scroll_y = scroll_y.unwrap_or(0.0);
        output.placeholder_length = node.placeholder().map_or(0, str::len);
        output.character_lengths_count = node.character_lengths().len();
        output.selection_anchor_node = selection.map_or(0, |s| s.anchor.node.0);
        output.selection_anchor_index = selection.map_or(0, |s| s.anchor.character_index);
        output.selection_focus_node = selection.map_or(0, |s| s.focus.node.0);
        output.selection_focus_index = selection.map_or(0, |s| s.focus.character_index);
        Ok(())
    })
}

// SAFETY CONTRACT: `destination` is writable for `capacity` bytes and
// `out_length` is writable. A null destination with zero capacity queries size.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_copy_text(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    node_id: u64,
    field: u32,
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi(|| {
        let snapshot = snapshot(snapshot_handle as usize)?;
        let node = node_by_id(&snapshot, node_id)?;
        let role_name;
        let value = match field {
            0 => node.label().unwrap_or(""),
            1 => node.description().unwrap_or(""),
            2 => node.value().unwrap_or(""),
            3 => {
                role_name = format!("{:?}", node.role());
                &role_name
            }
            4 => node.placeholder().unwrap_or(""),
            _ => return Err(invalid("unknown accessibility text field")),
        };
        copy_bytes_to_c(
            value.as_bytes(),
            destination,
            capacity,
            out_length,
            "accessibility text",
        )
    })
}

// SAFETY CONTRACT: `destination` is writable for `capacity` bytes and
// `out_count` is writable. A null destination with zero capacity queries size.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_copy_character_lengths(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    node_id: u64,
    destination: *mut u8,
    capacity: usize,
    out_count: *mut usize,
) -> OuiStatus {
    ffi(|| {
        let snapshot = snapshot(snapshot_handle as usize)?;
        let node = node_by_id(&snapshot, node_id)?;
        copy_bytes_to_c(
            node.character_lengths(),
            destination,
            capacity,
            out_count,
            "accessibility character lengths",
        )
    })
}

// SAFETY CONTRACT: `destination` is writable for `capacity` IDs and
// `out_count` is writable. A null destination with zero capacity queries size.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_copy_ids(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    node_id: u64,
    kind: u32,
    destination: *mut u64,
    capacity: usize,
    out_count: *mut usize,
) -> OuiStatus {
    ffi(|| {
        let snapshot = snapshot(snapshot_handle as usize)?;
        let node = node_by_id(&snapshot, node_id)?;
        let ids = match kind {
            0 => node.children(),
            1 => node.labelled_by(),
            2 => node.described_by(),
            3 => node.controls(),
            4 => node.details(),
            _ => return Err(invalid("unknown accessibility ID relation")),
        };
        let ids: Vec<_> = ids.iter().map(|id| id.0).collect();
        copy_array_to_c(&ids, destination, capacity, out_count, "accessibility IDs")
    })
}

// SAFETY CONTRACT: `destination` is writable for `capacity` IDs and
// `out_count` is writable. A null destination with zero capacity queries size.
#[no_mangle]
pub extern "C" fn oui_accessibility_snapshot_copy_changes(
    snapshot_handle: *const OuiAccessibilitySnapshot,
    removed: u8,
    destination: *mut u64,
    capacity: usize,
    out_count: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if removed > 1 {
            return Err(invalid("removed must be zero or one"));
        }
        let snapshot = snapshot(snapshot_handle as usize)?;
        let ids = if removed == 1 {
            &snapshot.removed
        } else {
            &snapshot.changed
        };
        copy_array_to_c(
            ids,
            destination,
            capacity,
            out_count,
            "accessibility changes",
        )
    })
}
