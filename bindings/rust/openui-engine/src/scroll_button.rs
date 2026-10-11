//! Origin-owned native scroll buttons. Geometry and defaults live in Engine;
//! application callbacks use the ordinary retained event pipeline.
//!
//! Page arithmetic and directional selection follow Chromium 147.0.7727.50
//! scroll_button_pseudo_element.cc, scroll_utils.cc and scroll_snap_data.cc.

use crate::{ActivationResult, Engine, EngineError, FocusOrigin, NodeHandle};
pub use openui_dom::ScrollButtonDirection;
use openui_dom::{ElementTag, NodeId, PseudoElementKind};
use openui_geometry::{LayoutUnit, PhysicalOffset, WritingDirectionMode};
use openui_layout::Fragment;
use openui_style::{
    Display, InvalidationClass, ScrollSnapAlign, ScrollSnapAxis, StyleProperty, Transform2D,
};
use std::collections::{HashMap, HashSet};

/// The current native axis/align style API defaults to mandatory snapping.
/// Proximity is explicit and never silently substituted for mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollSnapStrictness {
    #[default]
    Mandatory,
    Proximity,
}

/// Why a native scroll button can or cannot activate at this layout barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollButtonAvailability {
    Available,
    Boundary,
    NoScrollport,
    AuthoredDisabled,
    /// The geometry needs a shared operation not yet supported by this API.
    /// This is an open behavior gate, not a successful unsnapped fallback.
    Unsupported(&'static str),
}

/// An owned snapshot; later mutations do not change its contents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollButtonState {
    pub origin: NodeHandle,
    pub direction: ScrollButtonDirection,
    pub enabled: bool,
    pub availability: ScrollButtonAvailability,
    pub current_offset: (f64, f64),
    pub target_offset: Option<(f64, f64)>,
    /// Identities committed by the last native page activation.
    pub current_snap_targets: [Option<NodeHandle>; 2],
    /// Preview identities for this button's next page activation.
    pub snap_targets: [Option<NodeHandle>; 2],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Binding {
    pub button: NodeHandle,
    pub origin: NodeHandle,
    pub direction: ScrollButtonDirection,
    pub order: u64,
    pub state: ScrollButtonState,
}

#[derive(Default)]
pub(crate) struct ScrollButtons {
    pub bindings: HashMap<NodeHandle, Binding>,
    pub directions: HashMap<(NodeHandle, ScrollButtonDirection), NodeHandle>,
    pub snap_targets: HashMap<NodeHandle, [Option<NodeHandle>; 2]>,
    pub strictness: HashMap<NodeHandle, ScrollSnapStrictness>,
    pub next_order: u64,
    pub dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    fn axis(self) -> usize {
        if matches!(self, Self::Left | Self::Right) {
            0
        } else {
            1
        }
    }
    fn sign(self) -> f32 {
        if matches!(self, Self::Left | Self::Up) {
            -1.0
        } else {
            1.0
        }
    }
}

fn physical_direction(
    direction: ScrollButtonDirection,
    writing: WritingDirectionMode,
) -> Direction {
    use Direction::{Down, Left, Right, Up};
    use ScrollButtonDirection as D;
    match direction {
        D::Up => Up,
        D::Right => Right,
        D::Down => Down,
        D::Left => Left,
        D::BlockStart if writing.is_horizontal() => Up,
        D::BlockEnd if writing.is_horizontal() => Down,
        D::BlockStart => {
            if writing.is_flipped_blocks() {
                Right
            } else {
                Left
            }
        }
        D::BlockEnd => {
            if writing.is_flipped_blocks() {
                Left
            } else {
                Right
            }
        }
        D::InlineStart if writing.is_horizontal() => {
            if writing.is_rtl() {
                Right
            } else {
                Left
            }
        }
        D::InlineEnd if writing.is_horizontal() => {
            if writing.is_rtl() {
                Left
            } else {
                Right
            }
        }
        D::InlineStart => {
            if writing.is_rtl() != writing.is_flipped_lines() {
                Down
            } else {
                Up
            }
        }
        D::InlineEnd => {
            if writing.is_rtl() != writing.is_flipped_lines() {
                Up
            } else {
                Down
            }
        }
    }
}

fn page_step(length: i32) -> f32 {
    // Linux ScrollUtils: preserve f32 multiplication and integer truncation.
    ((length as f32 * 0.875_f32) as i32)
        .max(length - i32::MAX)
        .max(1) as f32
}

fn pixel_snapped_size(size: LayoutUnit, location: LayoutUnit) -> i32 {
    // Pinned layout_unit.h::SnapSizeToPixel; this is not an enclosing rect.
    let fraction = location.fraction();
    let result = (fraction + size).round_i32() - fraction.round_i32();
    if result == 0 && size.raw().unsigned_abs() > 4 {
        if size.raw() > 0 {
            1
        } else {
            -1
        }
    } else {
        result
    }
}

#[derive(Clone, Copy)]
struct SnapArea {
    handle: NodeHandle,
    rect: [f32; 4],
    align: ScrollSnapAlign,
    focus: bool,
}

#[derive(Clone, Copy)]
struct Candidate {
    offset: f32,
    area: SnapArea,
    covering: bool,
}

fn contains(outer: [f32; 4], inner: [f32; 4]) -> bool {
    outer[0] <= inner[0]
        && outer[1] <= inner[1]
        && outer[0] + outer[2] >= inner[0] + inner[2]
        && outer[1] + outer[3] >= inner[1] + inner[3]
}

fn aligned_offset(area: SnapArea, axis: usize, port: [f32; 2], maximum: f32, reverse: bool) -> f32 {
    let start = area.rect[axis];
    let size = area.rect[axis + 2];
    let offset = match area.align {
        ScrollSnapAlign::Start if reverse => start + size - port[axis],
        ScrollSnapAlign::End if reverse => start,
        ScrollSnapAlign::Start => start,
        ScrollSnapAlign::Center => start + (size - port[axis]) * 0.5,
        ScrollSnapAlign::End => start + size - port[axis],
        ScrollSnapAlign::None => unreachable!("collected snap area has alignment"),
    };
    offset.clamp(0.0, maximum)
}

fn dodging_offset(
    area: SnapArea,
    axis: usize,
    range: [f32; 2],
    preferred: f32,
    port: f32,
    reverse: bool,
) -> Option<(f32, bool)> {
    if range[1] <= range[0] {
        return None;
    }
    let minimum = range[0];
    let maximum = range[1] - port;
    if maximum > minimum {
        return Some((preferred.clamp(minimum, maximum), true));
    }
    let aligned = match area.align {
        ScrollSnapAlign::Start if reverse => maximum,
        ScrollSnapAlign::End if reverse => minimum,
        ScrollSnapAlign::Start => minimum,
        ScrollSnapAlign::Center => (minimum + maximum) * 0.5,
        ScrollSnapAlign::End => maximum,
        ScrollSnapAlign::None => return None,
    };
    let lo = area.rect[axis];
    let hi = lo + area.rect[axis + 2] - port;
    (hi >= lo).then(|| (aligned.clamp(lo, hi), false))
}

fn covering_candidate(
    area: SnapArea,
    areas: &[SnapArea],
    axis: usize,
    preferred: f32,
    port: f32,
    reverse: bool,
) -> Option<Candidate> {
    if area.rect[axis + 2] < port {
        return None;
    }
    let lo = area.rect[axis];
    let hi = lo + area.rect[axis + 2];
    let mut backward = [lo, hi];
    let mut middle = [lo, hi];
    let mut forward = [lo, hi];
    let preferred_end = preferred + port;
    for intruder in areas {
        let start = intruder.rect[axis];
        let end = start + intruder.rect[axis + 2];
        if start > hi || end < lo || (start <= lo && end >= hi) {
            continue;
        }
        if end < preferred {
            backward[0] = backward[0].max(end);
        } else {
            backward[1] = backward[1].min(start);
        }
        if start > preferred_end {
            forward[1] = forward[1].min(start);
        } else {
            forward[0] = forward[0].max(end);
        }
        if (start <= preferred && end >= preferred_end)
            || (preferred <= start && preferred_end >= end)
        {
            middle = [0.0, 0.0];
        } else if start <= preferred {
            middle[0] = middle[0].max(end);
        } else {
            middle[1] = middle[1].min(start);
        }
    }
    let wrap = |(offset, covering)| Candidate {
        offset,
        area,
        covering,
    };
    if let Some(value) = dodging_offset(area, axis, middle, preferred, port, reverse) {
        return Some(wrap(value));
    }
    let back = dodging_offset(area, axis, backward, preferred, port, reverse).map(wrap);
    let front = dodging_offset(area, axis, forward, preferred, port, reverse).map(wrap);
    match (back, front) {
        (Some(b), Some(f)) => Some(
            if (b.offset - preferred).abs() < (f.offset - preferred).abs() {
                b
            } else {
                f
            },
        ),
        (b, f) => b.or(f),
    }
}

/// One-axis page strategy. Page keys/buttons request one physical axis, so the
/// cross axis stays at its current clamped position. Unlike nearest-point snap,
/// preferred displacement, focus, nested ties and covering intervals matter.
fn select_snap(
    areas: &[SnapArea],
    direction: Direction,
    current: [f32; 2],
    port: [f32; 2],
    maximum: [f32; 2],
    page: i32,
    fractional: bool,
    reverse: bool,
    strictness: ScrollSnapStrictness,
) -> Option<Candidate> {
    let axis = direction.axis();
    let cross = 1 - axis;
    let intended = current[axis] + direction.sign() * page_step(page);
    let minimum_preferred = ((page as f32 * 0.4_f32) as i32).max(1) as f32;
    let maximum_preferred = page.max(1) as f32;
    let visible = |candidate: Candidate| {
        let lo = (candidate.area.rect[cross] - port[cross]).clamp(0.0, maximum[cross]);
        let hi = (candidate.area.rect[cross] + candidate.area.rect[cross + 2])
            .clamp(0.0, maximum[cross]);
        current[cross].clamp(0.0, maximum[cross]) >= lo
            && current[cross].clamp(0.0, maximum[cross]) <= hi
    };
    let preferred = |offset: f32| {
        let d = (offset - current[axis]).abs();
        d >= minimum_preferred && d <= maximum_preferred
    };
    let valid = |offset: f32| {
        let d = offset - current[axis];
        (if fractional { d } else { d.trunc() }) * direction.sign() > 0.0
    };
    let search = |relaxed: bool| {
        let base = if relaxed { current[axis] } else { intended };
        let mut best: Option<Candidate> = None;
        let mut covering: Option<Candidate> = None;
        let mut best_preferred = false;
        let mut distance = f32::MAX;
        for area in areas {
            let aligned = Candidate {
                offset: aligned_offset(*area, axis, port, maximum[axis], reverse),
                area: *area,
                covering: false,
            };
            let covered = covering_candidate(*area, areas, axis, base, port[axis], reverse);
            for candidate in [Some(aligned), covered].into_iter().flatten() {
                if !visible(candidate) {
                    continue;
                }
                if candidate.covering && candidate.offset == base {
                    if covering.is_none() || candidate.area.focus {
                        covering = Some(candidate);
                    }
                    continue;
                }
                if !relaxed && !valid(candidate.offset) {
                    continue;
                }
                let is_preferred = !relaxed && preferred(candidate.offset);
                let d = (candidate.offset - base).abs();
                if best_preferred && !is_preferred {
                    continue;
                }
                if d > distance && (best_preferred || !is_preferred) {
                    continue;
                }
                let wins_tie = best.is_some_and(|old| {
                    !old.area.focus
                        && contains(old.area.rect, candidate.area.rect)
                        && old.area.rect != candidate.area.rect
                });
                if d < distance
                    || (is_preferred && (!best_preferred || candidate.area.focus))
                    || wins_tie
                {
                    distance = d;
                    best = Some(candidate);
                    best_preferred = is_preferred;
                }
            }
        }
        covering.or(best)
    };
    search(false).or_else(|| {
        (strictness == ScrollSnapStrictness::Mandatory)
            .then(|| search(true))
            .flatten()
    })
}

impl Engine {
    /// Create a real retained scroll-button control owned by `origin`.
    /// Each origin/direction pair is unique. Applications style its box and
    /// text through native operations; it is not a separately movable child.
    pub fn create_scroll_button(
        &mut self,
        origin: NodeHandle,
        direction: ScrollButtonDirection,
    ) -> Result<NodeHandle, EngineError> {
        let node = self.resolve(origin)?;
        if !self.is_authored_element(origin)? {
            return Err(EngineError::InvalidInput(
                "scroll-button origin must be authored",
            ));
        }
        if self
            .scroll_buttons
            .directions
            .contains_key(&(origin, direction))
        {
            return Err(EngineError::InvalidInput(
                "duplicate scroll-button direction",
            ));
        }
        let pseudo = self
            .document
            .insert_pseudo_element(node, PseudoElementKind::ScrollButton(direction));
        let button = self.register_native_node(pseudo);
        self.initialize_control(button, ElementTag::Button);
        self.document.node_mut(node).native_scroll_snap_managed = true;
        let state = ScrollButtonState {
            origin,
            direction,
            enabled: false,
            availability: ScrollButtonAvailability::NoScrollport,
            current_offset: self.scroll_offset(origin)?,
            target_offset: None,
            current_snap_targets: [None, None],
            snap_targets: [None, None],
        };
        let order = self.scroll_buttons.next_order;
        self.scroll_buttons.next_order = order.wrapping_add(1);
        self.scroll_buttons.bindings.insert(
            button,
            Binding {
                button,
                origin,
                direction,
                order,
                state,
            },
        );
        self.scroll_buttons
            .directions
            .insert((origin, direction), button);
        self.document.node_mut(pseudo).form_control_disabled = true;
        self.set_property(button, StyleProperty::Display, Display::InlineBlock.into())?;
        self.refresh_inherited_styles(pseudo)?;
        self.relocate_owned_scroll_buttons()?;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(button)
    }

    pub fn scroll_button(
        &self,
        origin: NodeHandle,
        direction: ScrollButtonDirection,
    ) -> Result<Option<NodeHandle>, EngineError> {
        self.resolve(origin)?;
        Ok(self
            .scroll_buttons
            .directions
            .get(&(origin, direction))
            .copied())
    }

    pub fn is_scroll_button(&self, button: NodeHandle) -> Result<bool, EngineError> {
        self.resolve(button)?;
        Ok(self.scroll_buttons.bindings.contains_key(&button))
    }

    pub fn set_scroll_button_direction(
        &mut self,
        button: NodeHandle,
        direction: ScrollButtonDirection,
    ) -> Result<(), EngineError> {
        self.resolve(button)?;
        let old = *self
            .scroll_buttons
            .bindings
            .get(&button)
            .ok_or(EngineError::NotAControl)?;
        if self
            .scroll_buttons
            .directions
            .get(&(old.origin, direction))
            .is_some_and(|value| *value != button)
        {
            return Err(EngineError::InvalidInput(
                "duplicate scroll-button direction",
            ));
        }
        if old.direction == direction {
            return Ok(());
        }
        self.scroll_buttons
            .directions
            .remove(&(old.origin, old.direction));
        self.scroll_buttons
            .directions
            .insert((old.origin, direction), button);
        self.scroll_buttons
            .bindings
            .get_mut(&button)
            .expect("binding resolved")
            .direction = direction;
        let node = self.resolve(button)?;
        self.document.node_mut(node).pseudo_kind = Some(PseudoElementKind::ScrollButton(direction));
        self.mark_dirty(InvalidationClass::Paint);
        Ok(())
    }

    pub fn scroll_button_state(
        &mut self,
        button: NodeHandle,
    ) -> Result<ScrollButtonState, EngineError> {
        self.resolve(button)?;
        if !self.scroll_buttons.bindings.contains_key(&button) {
            return Err(EngineError::NotAControl);
        }
        self.update()?;
        Ok(self.scroll_buttons.bindings[&button].state)
    }

    pub fn set_scroll_snap_strictness(
        &mut self,
        origin: NodeHandle,
        strictness: ScrollSnapStrictness,
    ) -> Result<(), EngineError> {
        let node = self.resolve(origin)?;
        if !self.is_authored_element(origin)? {
            return Err(EngineError::InvalidInput(
                "scroll-snap origin must be authored",
            ));
        }
        self.scroll_buttons.strictness.insert(origin, strictness);
        self.document.node_mut(node).native_scroll_snap_managed = true;
        self.mark_dirty(InvalidationClass::Layout);
        Ok(())
    }

    /// Public element identity for events and activeElement. Raw identities
    /// remain retained for hit testing, focus, capture and default activation.
    pub fn public_event_target(&self, raw: NodeHandle) -> NodeHandle {
        let mut node = raw;
        while let Ok(id) = self.resolve(node) {
            if let Some(binding) = self.scroll_buttons.bindings.get(&node) {
                return binding.origin;
            }
            let parent = self.document.node(id).parent;
            let Some(index) = self.node_slots.get(&parent) else {
                break;
            };
            node = self.handle_for_slot(*index);
        }
        raw
    }

    /// Logical lifetime ownership, including external pseudo boxes and text.
    /// `children()` remains the authored tree surface.
    pub fn owned_subtree_handles(&self, root: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let mut nodes = Vec::new();
        self.collect_subtree(self.resolve(root)?, &mut nodes);
        Ok(nodes
            .into_iter()
            .filter_map(|node| self.node_slots.get(&node).map(|i| self.handle_for_slot(*i)))
            .collect())
    }

    pub(crate) fn owned_buttons(&self, origin: NodeId) -> Vec<NodeId> {
        let Some(index) = self.node_slots.get(&origin) else {
            return Vec::new();
        };
        let origin = self.handle_for_slot(*index);
        use ScrollButtonDirection as D;
        let mut buttons: Vec<_> = [
            D::Up,
            D::Right,
            D::Down,
            D::Left,
            D::BlockStart,
            D::BlockEnd,
            D::InlineStart,
            D::InlineEnd,
        ]
        .into_iter()
        .filter_map(|direction| {
            self.scroll_buttons
                .directions
                .get(&(origin, direction))
                .copied()
        })
        .collect();
        buttons.sort_by_key(|button| self.scroll_buttons.bindings[button].order);
        buttons
            .into_iter()
            .filter_map(|b| self.resolve(b).ok())
            .collect()
    }

    pub(crate) fn validate_owned_scroll_button_move(
        &self,
        handle: NodeHandle,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if self.scroll_buttons.bindings.contains_key(&handle) {
            return Err(EngineError::InvalidInput(
                "scroll buttons move with their originating element",
            ));
        }
        Ok(())
    }

    /// Presentation children include origin-owned external boxes. Logical
    /// traversal avoids exposing these boxes as authored application children.
    pub(crate) fn presentation_children(
        &self,
        handle: NodeHandle,
    ) -> Result<Vec<NodeHandle>, EngineError> {
        let mut children = self.children(handle)?;
        children.extend(
            self.owned_buttons(self.resolve(handle)?)
                .into_iter()
                .filter_map(|node| {
                    self.node_slots
                        .get(&node)
                        .map(|index| self.handle_for_slot(*index))
                }),
        );
        Ok(children)
    }

    pub(crate) fn relocate_owned_scroll_buttons(&mut self) -> Result<(), EngineError> {
        let mut origins: Vec<_> = self
            .scroll_buttons
            .bindings
            .values()
            .map(|b| b.origin)
            .collect();
        origins.sort_by_key(|h| h.index());
        origins.dedup();
        for origin in origins {
            let node = self.resolve(origin)?;
            let buttons = self.owned_buttons(node);
            for button in &buttons {
                self.document.detach(*button);
            }
            let parent = self.document.node(node).parent;
            if parent.is_none() && node != self.document.root() {
                continue;
            }
            let next = self.document.node(node).next_sibling;
            for button in buttons {
                if node == self.document.root() {
                    self.document.append_child(node, button);
                } else if next.is_none() {
                    self.document.append_child(parent, button);
                } else {
                    self.document.insert_before(next, button);
                }
                self.refresh_inherited_styles(button)?;
            }
        }
        Ok(())
    }

    pub(crate) fn forget_scroll_button_handles(&mut self, handles: &[NodeHandle]) {
        let removed: HashSet<_> = handles.iter().copied().collect();
        self.scroll_buttons.bindings.retain(|button, binding| {
            !removed.contains(button) && !removed.contains(&binding.origin)
        });
        self.scroll_buttons
            .directions
            .retain(|(origin, _), button| !removed.contains(origin) && !removed.contains(button));
        self.scroll_buttons
            .strictness
            .retain(|origin, _| !removed.contains(origin));
        self.scroll_buttons.snap_targets.retain(|origin, targets| {
            for target in targets {
                if target.is_some_and(|target| removed.contains(&target)) {
                    *target = None;
                }
            }
            !removed.contains(origin)
        });
    }

    fn resolve_scroll_button(&self, binding: Binding) -> ScrollButtonState {
        use ScrollButtonAvailability as A;
        let mut state = ScrollButtonState {
            origin: binding.origin,
            direction: binding.direction,
            enabled: false,
            availability: A::NoScrollport,
            current_offset: self.scroll_offset(binding.origin).unwrap_or_default(),
            target_offset: None,
            current_snap_targets: self
                .scroll_buttons
                .snap_targets
                .get(&binding.origin)
                .copied()
                .unwrap_or([None, None]),
            snap_targets: self
                .scroll_buttons
                .snap_targets
                .get(&binding.origin)
                .copied()
                .unwrap_or([None, None]),
        };
        let Ok(node) = self.resolve(binding.origin) else {
            return state;
        };
        if !self.node_is_connected(node) {
            return state;
        }
        let Some(area) = self.layout_scroll_area(node) else {
            return state;
        };
        let style = &self.document.node(node).style;
        if node != self.document.root() && !style.is_scroll_container() {
            return state;
        }
        let direction = physical_direction(
            binding.direction,
            style.direction.writing_direction(style.writing_mode),
        );
        let axis = direction.axis();
        let ranges = area.limits();
        let minimum = [ranges.0 .0 as f32, ranges.1 .0 as f32];
        let maximum = [
            (ranges.0 .1 - ranges.0 .0) as f32,
            (ranges.1 .1 - ranges.1 .0) as f32,
        ];
        let current = [
            state.current_offset.0 as f32 - minimum[0],
            state.current_offset.1 as f32 - minimum[1],
        ];
        let port = [
            area.client_rect.width().to_f32(),
            area.client_rect.height().to_f32(),
        ];
        let size = [area.client_rect.width(), area.client_rect.height()];
        let offset = [state.current_offset.0 as f32, state.current_offset.1 as f32];
        let page = pixel_snapped_size(size[axis], LayoutUnit::from_f32_round(offset[axis])).max(0);
        let mut position = current;
        position[axis] += direction.sign() * page_step(page);
        let writing = style.direction.writing_direction(style.writing_mode);
        let snap_axis = match style.scroll_snap_axis {
            ScrollSnapAxis::None => false,
            ScrollSnapAxis::X => axis == 0,
            ScrollSnapAxis::Y => axis == 1,
            ScrollSnapAxis::Both => true,
            ScrollSnapAxis::Inline => axis == usize::from(!writing.is_horizontal()),
            ScrollSnapAxis::Block => axis == usize::from(writing.is_horizontal()),
        };
        if style.scroll_snap_axis == ScrollSnapAxis::None {
            state.snap_targets = [None, None];
        }
        if snap_axis {
            let Some(fragment) = self
                .latest_fragment
                .as_ref()
                .and_then(|root| find_fragment(root, node))
            else {
                return state;
            };
            let mut areas = Vec::new();
            let mut seen = HashSet::new();
            if let Err(reason) = self.collect_snap_areas(
                fragment,
                PhysicalOffset::default(),
                area,
                minimum,
                false,
                &mut seen,
                &mut areas,
            ) {
                state.availability = A::Unsupported(reason);
                return state;
            }
            for target in &mut state.snap_targets {
                if target.is_some_and(|target| !areas.iter().any(|area| area.handle == target)) {
                    *target = None;
                }
            }
            let reverse = if axis == 0 {
                if writing.is_horizontal() {
                    writing.is_rtl()
                } else {
                    writing.is_flipped_blocks()
                }
            } else {
                !writing.is_horizontal() && (writing.is_rtl() != writing.is_flipped_lines())
            };
            let strictness = self
                .scroll_buttons
                .strictness
                .get(&binding.origin)
                .copied()
                .unwrap_or_default();
            if strictness == ScrollSnapStrictness::Proximity {
                state.availability = A::Unsupported("proximity snap range is not implemented");
                return state;
            }
            if let Some(target) = select_snap(
                &areas,
                direction,
                current,
                port,
                maximum,
                page,
                !area.integer_offsets,
                reverse,
                strictness,
            ) {
                position[axis] = target.offset;
                state.snap_targets[axis] = Some(target.area.handle);
            } else {
                state.snap_targets[axis] = None;
            }
        }
        let rounded_current = current[axis].round();
        let rounded_target = position[axis].clamp(0.0, maximum[axis]).round();
        state.enabled = (rounded_target - rounded_current) * direction.sign() > 0.0;
        state.availability = if state.enabled {
            A::Available
        } else {
            A::Boundary
        };
        let target = area.clamp_offset(
            (position[0] + minimum[0]) as f64,
            (position[1] + minimum[1]) as f64,
        );
        state.target_offset = Some((target.0 as f64, target.1 as f64));
        if self
            .controls
            .get(&binding.button.index())
            .is_some_and(|control| control.disabled)
        {
            state.enabled = false;
            state.availability = A::AuthoredDisabled;
        }
        state
    }

    fn collect_snap_areas(
        &self,
        fragment: &Fragment,
        offset: PhysicalOffset,
        port: openui_layout::ScrollArea,
        minimum: [f32; 2],
        unsupported_ancestor: bool,
        seen: &mut HashSet<NodeId>,
        out: &mut Vec<SnapArea>,
    ) -> Result<(), &'static str> {
        for child in &fragment.children {
            let offset = PhysicalOffset::new(
                offset.left + child.offset.left,
                offset.top + child.offset.top,
            );
            if child.node_id.is_none() {
                self.collect_snap_areas(
                    child,
                    offset,
                    port,
                    minimum,
                    unsupported_ancestor,
                    seen,
                    out,
                )?;
                continue;
            }
            let data = self.document.node(child.node_id);
            let unsupported = unsupported_ancestor
                || (child.owns_element_box()
                    && (data.style.transform != Transform2D::IDENTITY
                        || data.style.position == openui_style::Position::Sticky));
            if child.owns_element_box() && data.style.scroll_snap_align != ScrollSnapAlign::None {
                if unsupported {
                    return Err("transformed or sticky snap-area geometry is not implemented");
                }
                let Some(index) = self.node_slots.get(&child.node_id) else {
                    return Err("snap area has no retained identity");
                };
                let handle = self.handle_for_slot(*index);
                let border = child.border_box_rect();
                let candidate = SnapArea {
                    handle,
                    rect: [
                        (offset.left + border.offset.left - port.client_rect.x()).to_f32()
                            - minimum[0],
                        (offset.top + border.offset.top - port.client_rect.y()).to_f32()
                            - minimum[1],
                        border.size.width.to_f32(),
                        border.size.height.to_f32(),
                    ],
                    align: data.style.scroll_snap_align,
                    focus: self
                        .focused
                        .is_some_and(|focus| self.has_focus_within(handle, focus)),
                };
                if seen.insert(child.node_id) {
                    out.push(candidate);
                } else if let Some(area) = out.iter_mut().find(|area| area.handle == handle) {
                    let left = area.rect[0].min(candidate.rect[0]);
                    let top = area.rect[1].min(candidate.rect[1]);
                    let right =
                        (area.rect[0] + area.rect[2]).max(candidate.rect[0] + candidate.rect[2]);
                    let bottom =
                        (area.rect[1] + area.rect[3]).max(candidate.rect[1] + candidate.rect[3]);
                    area.rect = [left, top, right - left, bottom - top];
                }
            }
            // A nested scroll container owns its own descendant snap areas.
            if child.owns_element_box() && data.style.is_scroll_container() {
                continue;
            }
            self.collect_snap_areas(child, offset, port, minimum, unsupported, seen, out)?;
        }
        Ok(())
    }

    pub(crate) fn refresh_scroll_button_states(&mut self) {
        if !self.scroll_buttons.dirty {
            return;
        }
        let bindings: Vec<_> = self.scroll_buttons.bindings.values().copied().collect();
        for binding in bindings {
            let state = self.resolve_scroll_button(binding);
            if let Ok(node) = self.resolve(binding.button) {
                let disabled = !state.enabled;
                if self.document.node(node).form_control_disabled != disabled {
                    self.document.node_mut(node).form_control_disabled = disabled;
                    self.mark_dirty(InvalidationClass::Paint);
                    self.dirty.hit_test = true;
                }
            }
            self.scroll_buttons
                .bindings
                .get_mut(&binding.button)
                .expect("snapshot binding remains live")
                .state = state;
        }
        self.scroll_buttons.dirty = false;
    }

    fn has_focus_within(&self, ancestor: NodeHandle, focus: NodeHandle) -> bool {
        let mut current = Some(focus);
        while let Some(node) = current {
            if node == ancestor {
                return true;
            }
            current = self.parent(node).ok().flatten();
        }
        false
    }

    pub(crate) fn activate_scroll_button(
        &mut self,
        button: NodeHandle,
    ) -> Result<ActivationResult, EngineError> {
        let state = self.scroll_button_state(button)?;
        if let ScrollButtonAvailability::Unsupported(reason) = state.availability {
            return Err(EngineError::InvalidInput(reason));
        }
        if !state.enabled || self.is_effectively_disabled(button)? {
            return Ok(ActivationResult { changed: vec![] });
        }
        let (x, y) = state.target_offset.expect("enabled button has page target");
        // Focus while the activation-time eligibility snapshot remains valid;
        // the subsequent post-layout snapshot may disable the endpoint button.
        self.focus_with_origin(button, FocusOrigin::Script)?;
        self.scroll_to(state.origin, x, y)?;
        self.scroll_buttons
            .snap_targets
            .insert(state.origin, state.snap_targets);
        self.refresh_scroll_button_states();
        Ok(ActivationResult { changed: vec![] })
    }
}

fn find_fragment(fragment: &Fragment, node: NodeId) -> Option<&Fragment> {
    if fragment.owns_element_box() && fragment.node_id == node && fragment.scroll_area.is_some() {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| find_fragment(child, node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewportMetrics;
    use openui_style::LengthValue;

    #[test]
    fn unchanged_native_scroll_buttons_keep_the_scene_and_idle_state() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 80.0, 1.0).unwrap()).unwrap();
        let origin = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), origin).unwrap();
        engine
            .set_property(origin, StyleProperty::Width, LengthValue::px(50.0).into())
            .unwrap();
        engine
            .set_property(origin, StyleProperty::Height, LengthValue::px(40.0).into())
            .unwrap();
        engine
            .set_property(
                origin,
                StyleProperty::Overflow,
                openui_style::Overflow::Hidden.into(),
            )
            .unwrap();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::px(200.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Height, LengthValue::px(100.0).into())
            .unwrap();
        engine.append_child(origin, child).unwrap();
        let button = engine
            .create_scroll_button(origin, ScrollButtonDirection::Right)
            .unwrap();
        assert!(engine.scroll_button_state(button).unwrap().enabled);
        let stats = engine.stats();
        assert!(!engine.scroll_buttons.dirty);
        engine.update().unwrap();
        assert_eq!(engine.stats(), stats);
        assert!(!engine.scroll_buttons.dirty);
        engine.remove(origin).unwrap();
        assert!(engine.scroll_buttons.bindings.is_empty());
        assert!(engine.scroll_buttons.directions.is_empty());
        assert_eq!(engine.object_counts().controls, 0);
    }

    #[test]
    fn page_size_uses_location_phase_and_linux_fraction() {
        assert_eq!(
            pixel_snapped_size(LayoutUnit::from_f32(99.5), LayoutUnit::ZERO),
            100
        );
        assert_eq!(
            pixel_snapped_size(LayoutUnit::from_f32(99.5), LayoutUnit::from_f32(0.5)),
            99
        );
        assert_eq!(page_step(100), 87.0);
        assert_eq!(page_step(99), 86.0);
        assert_eq!(page_step(0), 1.0);
    }
}
