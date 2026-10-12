//! Origin-owned native scroll buttons. Geometry and defaults live in Engine;
//! application callbacks use the ordinary retained event pipeline.
//!
//! Page arithmetic and directional selection follow Chromium 147.0.7727.50
//! scroll_button_pseudo_element.cc, scroll_utils.cc and scroll_snap_data.cc.

use crate::{ActivationResult, Engine, EngineError, FocusOrigin, NodeHandle, WorldTransform};
pub use openui_dom::ScrollButtonDirection;
use openui_dom::{ElementTag, NodeId, PseudoElementKind};
use openui_geometry::{LayoutUnit, Length, LengthType, WritingDirectionMode};
use openui_layout::{Fragment, ScrollArea};
use openui_style::{
    ComputedStyle, Display, InvalidationClass, Overflow, RendererStyleValue, ScrollSnapAlign,
    ScrollSnapAxis, ScrollSnapStop, StyleProperty, StyleValue,
};
use std::collections::{HashMap, HashSet};

pub use openui_style::ScrollSnapStrictness;

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
    /// Identities committed by native scrolling or the latest layout resnap.
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
    pub targeted_areas: HashMap<NodeHandle, NodeHandle>,
    /// End-position requests awaiting their first complete layout. Snapshot
    /// construction can precede attachment/resources; native callers resolve
    /// endpoints immediately through the same selection operation.
    pub pending_end_axes: HashMap<NodeHandle, [bool; 2]>,
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
    align: [ScrollSnapAlign; 2],
    focus: bool,
    must_stop: bool,
}

#[derive(Clone, Copy)]
struct Candidate {
    offset: f32,
    area: SnapArea,
    covering: bool,
    alternative: Option<SnapArea>,
}

fn contains(outer: [f32; 4], inner: [f32; 4]) -> bool {
    outer[0] <= inner[0]
        && outer[1] <= inner[1]
        && outer[0] + outer[2] >= inner[0] + inner[2]
        && outer[1] + outer[3] >= inner[1] + inner[3]
}

fn aligned_offset(area: SnapArea, axis: usize, port: [f32; 4], maximum: f32) -> f32 {
    let start = area.rect[axis] - port[axis];
    let size = area.rect[axis + 2];
    let offset = match area.align[axis] {
        ScrollSnapAlign::Start => start,
        ScrollSnapAlign::Center => start + (size - port[axis + 2]) * 0.5,
        ScrollSnapAlign::End => start + size - port[axis + 2],
        ScrollSnapAlign::None => unreachable!("snap area does not align on this axis"),
    };
    offset.clamp(0.0, maximum)
}

fn snap_axes(style: &ComputedStyle) -> [bool; 2] {
    match style.scroll_snap_axis {
        ScrollSnapAxis::None => [false, false],
        ScrollSnapAxis::X => [true, false],
        ScrollSnapAxis::Y => [false, true],
        ScrollSnapAxis::Both => [true, true],
        ScrollSnapAxis::Inline => [
            style.writing_mode.is_horizontal(),
            !style.writing_mode.is_horizontal(),
        ],
        ScrollSnapAxis::Block => [
            !style.writing_mode.is_horizontal(),
            style.writing_mode.is_horizontal(),
        ],
    }
}

fn length_pixels(length: Length, reference: f32) -> f32 {
    let value = match length.length_type() {
        LengthType::Fixed => length.value(),
        LengthType::Percent => length.value() * reference / 100.0,
        LengthType::Calculated => length.calc_offset() + length.value() * reference / 100.0,
        _ => 0.0,
    };
    value
}

fn layout_length_pixels(length: Length, reference: f32) -> f32 {
    // MinimumValueForLength and ScrollMarginStrut use truncating LayoutUnit conversion.
    LayoutUnit::from_f32(length_pixels(length, reference)).to_f32()
}

pub(super) fn scroll_snapport(style: &ComputedStyle, area: openui_layout::ScrollArea) -> [f32; 4] {
    let width = area.client_rect.width().to_f32();
    let height = area.client_rect.height().to_f32();
    let top = layout_length_pixels(style.scroll_padding_top, height).max(0.0);
    let right = layout_length_pixels(style.scroll_padding_right, width).max(0.0);
    let bottom = layout_length_pixels(style.scroll_padding_bottom, height).max(0.0);
    let left = layout_length_pixels(style.scroll_padding_left, width).max(0.0);
    // The geometry uses the retained physical-fragment client coordinate space.
    [
        area.client_rect.x().to_f32() + left,
        area.client_rect.y().to_f32() + top,
        (width - left - right).max(0.0),
        (height - top - bottom).max(0.0),
    ]
}

/// Physical target outsets shared by snap selection and native reveal.
pub(super) fn scroll_margin(style: &ComputedStyle) -> [f32; 4] {
    [
        layout_length_pixels(style.scroll_margin_left, 0.0),
        layout_length_pixels(style.scroll_margin_top, 0.0),
        layout_length_pixels(style.scroll_margin_right, 0.0),
        layout_length_pixels(style.scroll_margin_bottom, 0.0),
    ]
}

fn normalized_alignment(
    style: &ComputedStyle,
    container: &ComputedStyle,
    rect: [f32; 4],
    size: [f32; 2],
) -> [ScrollSnapAlign; 2] {
    let block = style
        .scroll_snap_align_block
        .unwrap_or(style.scroll_snap_align);
    let inline = style
        .scroll_snap_align_inline
        .unwrap_or(style.scroll_snap_align);
    let own = style.direction.writing_direction(style.writing_mode);
    let mut writing = container
        .direction
        .writing_direction(container.writing_mode);
    if (own.is_horizontal() && rect[2] > size[0]) || (!own.is_horizontal() && rect[3] > size[1]) {
        writing = own;
    }
    let flip = |align| match align {
        ScrollSnapAlign::Start => ScrollSnapAlign::End,
        ScrollSnapAlign::End => ScrollSnapAlign::Start,
        value => value,
    };
    if writing.is_horizontal() {
        [
            if writing.is_rtl() {
                flip(inline)
            } else {
                inline
            },
            block,
        ]
    } else {
        [
            if writing.is_flipped_blocks() {
                flip(block)
            } else {
                block
            },
            if writing.is_rtl() {
                flip(inline)
            } else {
                inline
            },
        ]
    }
}

fn visible(
    candidate: Candidate,
    axis: usize,
    cross_offset: f32,
    port: [f32; 4],
    maximum: [f32; 2],
) -> bool {
    let cross = 1 - axis;
    let lo =
        (candidate.area.rect[cross] - port[cross] - port[cross + 2]).clamp(0.0, maximum[cross]);
    let hi = (candidate.area.rect[cross] + candidate.area.rect[cross + 2] - port[cross])
        .clamp(0.0, maximum[cross]);
    cross_offset >= lo && cross_offset <= hi
}

fn mutual_visible(x: Candidate, y: Candidate, port: [f32; 4], maximum: [f32; 2]) -> bool {
    visible(x, 0, y.offset, port, maximum) && visible(y, 1, x.offset, port, maximum)
}

fn dodging_offset(
    area: SnapArea,
    axis: usize,
    range: [f32; 2],
    preferred: f32,
    port: f32,
    padding: f32,
) -> Option<(f32, bool)> {
    if range[1] <= range[0] {
        return None;
    }
    let minimum = range[0] - padding;
    let maximum = range[1] - padding - port;
    if maximum > minimum {
        return Some((preferred.clamp(minimum, maximum), true));
    }
    let aligned = match area.align[axis] {
        ScrollSnapAlign::Start => minimum,
        ScrollSnapAlign::Center => (minimum + maximum) * 0.5,
        ScrollSnapAlign::End => maximum,
        ScrollSnapAlign::None => return None,
    };
    let lo = area.rect[axis] - padding;
    let hi = lo + area.rect[axis + 2] - port;
    (hi >= lo).then(|| (aligned.clamp(lo, hi), false))
}

fn covering_candidate(
    area: SnapArea,
    areas: &[SnapArea],
    axis: usize,
    preferred: f32,
    port: f32,
    padding: f32,
) -> Option<Candidate> {
    if area.rect[axis + 2] < port {
        return None;
    }
    let lo = area.rect[axis];
    let hi = lo + area.rect[axis + 2];
    let mut backward = [lo, hi];
    let mut middle = [lo, hi];
    let mut forward = [lo, hi];
    let preferred = preferred + padding;
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
        alternative: None,
    };
    if let Some(value) = dodging_offset(area, axis, middle, preferred - padding, port, padding) {
        return Some(wrap(value));
    }
    let back = dodging_offset(area, axis, backward, preferred - padding, port, padding).map(wrap);
    let front = dodging_offset(area, axis, forward, preferred - padding, port, padding).map(wrap);
    match (back, front) {
        (Some(b), Some(f)) => Some(
            if (b.offset - (preferred - padding)).abs() < (f.offset - (preferred - padding)).abs() {
                b
            } else {
                f
            },
        ),
        (b, f) => b.or(f),
    }
}

/// Shared axis search for page activation and end-position/layout settling.
/// The selected identity is retained separately from the preview landing point.
fn search_axis(
    areas: &[SnapArea],
    axis: usize,
    current: [f32; 2],
    port: [f32; 4],
    maximum: [f32; 2],
    cross_offset: f32,
    page: Option<(Direction, i32, bool)>,
    strictness: ScrollSnapStrictness,
    targeted: Option<NodeHandle>,
    active_range: Option<[f32; 2]>,
) -> Option<Candidate> {
    let base = page.map_or(current[axis], |(direction, size, _)| {
        current[axis] + direction.sign() * page_step(size)
    });
    let proximity = if strictness == ScrollSnapStrictness::Proximity {
        LayoutUnit::from_f32(port[axis + 2] * (1.0_f32 / 3.0_f32)).to_f32()
    } else {
        f32::MAX
    };
    let mut closest: Option<Candidate> = None;
    let mut covering: Option<Candidate> = None;
    let mut best_preferred = false;
    let mut distance = proximity;
    for area in areas
        .iter()
        .copied()
        .filter(|area| area.align[axis] != ScrollSnapAlign::None)
    {
        if active_range.is_some_and(|range| {
            area.rect[axis] > range[1] || area.rect[axis] + area.rect[axis + 2] < range[0]
        }) {
            continue;
        }
        let aligned = Candidate {
            offset: aligned_offset(area, axis, port, maximum[axis]),
            area,
            covering: false,
            alternative: None,
        };
        let covered = covering_candidate(area, areas, axis, base, port[axis + 2], port[axis]);
        for candidate in [Some(aligned), covered].into_iter().flatten() {
            if candidate.covering && candidate.offset == base {
                // Chromium preserves an intended covered position and combines
                // equivalent covering identities, favoring focused content.
                if let Some(previous) = covering.as_mut() {
                    let l = previous.area.rect[0].min(candidate.area.rect[0]);
                    let t = previous.area.rect[1].min(candidate.area.rect[1]);
                    let r = (previous.area.rect[0] + previous.area.rect[2])
                        .max(candidate.area.rect[0] + candidate.area.rect[2]);
                    let b = (previous.area.rect[1] + previous.area.rect[3])
                        .max(candidate.area.rect[1] + candidate.area.rect[3]);
                    previous.area.rect = [l, t, r - l, b - t];
                    if candidate.area.focus {
                        previous.area.handle = candidate.area.handle;
                        previous.area.focus = true;
                    }
                } else {
                    covering = Some(candidate);
                }
                continue;
            }
            if !visible(candidate, axis, cross_offset, port, maximum) {
                continue;
            }
            let delta = candidate.offset - current[axis];
            if page.is_some_and(|(direction, _, fractional)| {
                (if fractional { delta } else { delta.trunc() }) * direction.sign() <= 0.0
            }) {
                continue;
            }
            let d = (candidate.offset - base).abs();
            if d > proximity {
                continue;
            }
            let preferred = page.is_none_or(|(_, size, _)| {
                delta.abs() >= ((size as f32 * 0.4) as i32).max(1) as f32
                    && delta.abs() <= size.max(1) as f32
            });
            if (best_preferred && !preferred) || (d > distance && (best_preferred || !preferred)) {
                continue;
            }
            if d < distance || (preferred && (!best_preferred || candidate.area.focus)) {
                distance = d;
                closest = Some(candidate);
                best_preferred = preferred;
            } else if let Some(old) = closest.as_mut().filter(|old| !old.area.focus) {
                if targeted == Some(old.area.handle) {
                    continue;
                }
                if targeted == Some(candidate.area.handle)
                    || (contains(old.area.rect, candidate.area.rect)
                        && old.area.rect != candidate.area.rect)
                {
                    *old = candidate;
                    best_preferred = preferred;
                } else if candidate.area.align[1 - axis] != ScrollSnapAlign::None
                    && preferred == best_preferred
                    && !contains(candidate.area.rect, old.area.rect)
                {
                    let next_distance =
                        (aligned_offset(candidate.area, 1 - axis, port, maximum[1 - axis])
                            - current[1 - axis])
                            .abs();
                    let replace = old.alternative.is_none_or(|previous| {
                        let old_distance =
                            (aligned_offset(previous, 1 - axis, port, maximum[1 - axis])
                                - current[1 - axis])
                                .abs();
                        next_distance < old_distance
                            || (next_distance == old_distance
                                && contains(previous.rect, candidate.area.rect)
                                && previous.rect != candidate.area.rect)
                    });
                    if replace {
                        old.alternative = Some(candidate.area);
                    }
                }
            }
        }
    }
    // EndPosition and page-distance strategies prefer the intended covering result.
    let mut result = covering.or(closest);
    if let (Some(selected), Some((direction, _, fractional))) = (result, page) {
        // DirectionStrategy always respects snap-stop. Its second search has
        // direction preference and includes only stop-always areas.
        let first_stop = areas
            .iter()
            .copied()
            .filter(|area| area.must_stop && area.align[axis] != ScrollSnapAlign::None)
            .filter_map(|area| {
                let candidate = Candidate {
                    offset: aligned_offset(area, axis, port, maximum[axis]),
                    area,
                    covering: false,
                    alternative: None,
                };
                let d = candidate.offset - current[axis];
                let valid = (if fractional { d } else { d.trunc() }) * direction.sign() > 0.0;
                (valid
                    && visible(candidate, axis, cross_offset, port, maximum)
                    && d.abs() <= proximity)
                    .then_some(candidate)
            })
            .min_by(|a, b| {
                (a.offset - current[axis])
                    .abs()
                    .total_cmp(&(b.offset - current[axis]).abs())
            });
        if let Some(stop) = first_stop {
            if (stop.offset - current[axis]).abs() <= (selected.offset - current[axis]).abs() {
                result = Some(stop);
            }
        }
    }
    if result.is_none() && page.is_some() && strictness == ScrollSnapStrictness::Mandatory {
        return search_axis(
            areas,
            axis,
            current,
            port,
            maximum,
            cross_offset,
            None,
            strictness,
            targeted,
            active_range,
        );
    }
    result
}

fn select_end_position(
    areas: &[SnapArea],
    current: [f32; 2],
    port: [f32; 4],
    maximum: [f32; 2],
    axes: [bool; 2],
    strictness: ScrollSnapStrictness,
    old_targets: [Option<NodeHandle>; 2],
    prioritize: bool,
    targeted: Option<NodeHandle>,
    horizontal: bool,
) -> ([f32; 2], [Option<NodeHandle>; 2]) {
    let mut selected: [Option<Candidate>; 2] = [None, None];
    let mut targets = old_targets
        .map(|target| target.filter(|target| areas.iter().any(|area| area.handle == *target)));
    for axis in 0..2 {
        if !axes[axis] {
            continue;
        }
        if prioritize {
            if let Some(area) = areas.iter().copied().find(|area| {
                Some(area.handle) == targets[axis] && area.align[axis] != ScrollSnapAlign::None
            }) {
                let aligned = Candidate {
                    offset: aligned_offset(area, axis, port, maximum[axis]),
                    area,
                    covering: false,
                    alternative: None,
                };
                selected[axis] = if area.rect[axis + 2] >= port[axis + 2] {
                    search_axis(
                        areas,
                        axis,
                        current,
                        port,
                        maximum,
                        current[1 - axis].clamp(0.0, maximum[1 - axis]),
                        None,
                        strictness,
                        targeted,
                        Some([area.rect[axis], area.rect[axis] + area.rect[axis + 2]]),
                    )
                    .or(Some(aligned))
                } else {
                    Some(aligned)
                };
            }
        }
        if selected[axis].is_none() {
            selected[axis] = search_axis(
                areas,
                axis,
                current,
                port,
                maximum,
                current[1 - axis].clamp(0.0, maximum[1 - axis]),
                None,
                strictness,
                targeted,
                None,
            );
        }
    }
    if let (Some(x), Some(y)) = (selected[0], selected[1]) {
        if !mutual_visible(x, y, port, maximum) {
            let follow_x = if prioritize {
                if targets[0].is_none() {
                    false
                } else if targets[1].is_none() || x.area.focus {
                    true
                } else if y.area.focus {
                    false
                } else if Some(x.area.handle) == targeted {
                    true
                } else if Some(y.area.handle) == targeted {
                    false
                } else {
                    !horizontal
                }
            } else {
                (x.offset - current[0]).abs() <= (y.offset - current[1]).abs()
            };
            let axis = if follow_x { 1 } else { 0 };
            selected[axis] = search_axis(
                areas,
                axis,
                current,
                port,
                maximum,
                selected[1 - axis].unwrap().offset,
                None,
                strictness,
                targeted,
                None,
            );
        }
    }
    if axes == [true, true] && selected.iter().all(Option::is_none) {
        // Both corridors can miss a diagonal area. Prefer minimum block
        // distance from the snapport corridor, then inline distance.
        let distance = |delta: f32, size: f32| {
            if delta < 0.0 {
                -delta
            } else {
                (delta - size).max(0.0)
            }
        };
        let proximity = |axis: usize| {
            if strictness == ScrollSnapStrictness::Proximity {
                LayoutUnit::from_f32(port[axis + 2] * (1.0_f32 / 3.0_f32)).to_f32()
            } else {
                f32::MAX
            }
        };
        let winner = areas
            .iter()
            .copied()
            .filter(|area| {
                area.align
                    .iter()
                    .all(|align| *align != ScrollSnapAlign::None)
            })
            .filter_map(|area| {
                let offset = [
                    aligned_offset(area, 0, port, maximum[0]),
                    aligned_offset(area, 1, port, maximum[1]),
                ];
                let dx = offset[0] - current[0];
                let dy = offset[1] - current[1];
                (dx.abs() <= proximity(0) && dy.abs() <= proximity(1)).then_some((
                    area,
                    offset,
                    [distance(dx, port[2]), distance(dy, port[3])],
                ))
            })
            .min_by(|a, b| a.2[1].total_cmp(&b.2[1]).then(a.2[0].total_cmp(&b.2[0])));
        if let Some((_area, offset, _)) = winner {
            // Pinned fallback returns an aligned position with no target IDs.
            return (offset, [None, None]);
        }
    }
    let mut position = current;
    for axis in 0..2 {
        if let Some(mut result) = selected[axis] {
            if let Some(alternative) = result.alternative {
                let cross = selected[1 - axis]
                    .map_or(current[1 - axis].clamp(0.0, maximum[1 - axis]), |value| {
                        value.offset
                    });
                if (aligned_offset(alternative, 1 - axis, port, maximum[1 - axis]) - cross).abs()
                    <= 1.0
                {
                    result.area = alternative;
                }
            }
            position[axis] = result.offset;
            targets[axis] = Some(result.area.handle);
        } else if axes[axis] {
            // A completed proximity endpoint without a candidate has no
            // committed snap target on that axis.
            targets[axis] = None;
        }
    }
    (position, targets)
}

struct SnapGeometry {
    area: openui_layout::ScrollArea,
    minimum: [f32; 2],
    maximum: [f32; 2],
    port: [f32; 4],
    areas: Vec<SnapArea>,
    axes: [bool; 2],
    strictness: ScrollSnapStrictness,
    horizontal: bool,
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
        self.resolve(origin)?;
        self.set_property(
            origin,
            StyleProperty::ScrollSnapStrictness,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapStrictness(strictness)),
        )
    }

    /// Native equivalent of selecting a targeted snap area for tie-breaking.
    /// It changes the shared target preference without starting a scroll.
    pub fn set_scroll_snap_target(
        &mut self,
        origin: NodeHandle,
        target: Option<NodeHandle>,
    ) -> Result<(), EngineError> {
        self.resolve(origin)?;
        if let Some(target) = target {
            self.resolve(target)?;
            if !self.is_authored_element(target)? || !self.has_focus_within(origin, target) {
                return Err(EngineError::InvalidInput(
                    "snap target must be an authored descendant of its scroll origin",
                ));
            }
            self.scroll_buttons.targeted_areas.insert(origin, target);
        } else {
            self.scroll_buttons.targeted_areas.remove(&origin);
        }
        self.scroll_buttons.dirty = true;
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
            let mut current = Some(node);
            let mut hidden = false;
            while let Some(ancestor) = current {
                let data = self.document.node(ancestor);
                if data.style.display == Display::None {
                    hidden = true;
                    break;
                }
                current = (!data.parent.is_none()).then_some(data.parent);
            }
            if hidden {
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
            .targeted_areas
            .retain(|origin, target| !removed.contains(origin) && !removed.contains(target));
        self.scroll_buttons
            .pending_end_axes
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
        if !area.is_scroll_container() {
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
        let port = scroll_snapport(style, area);
        let size = LayoutUnit::from_f32_round(port[axis + 2]);
        // VisibleScrollSnapportRect uses the layout-quantized scroll position
        // minus scroll origin, then the padding inset (not the box border).
        let location = LayoutUnit::from_f32_round(
            current[axis] + minimum[axis] + port[axis]
                - [area.client_rect.x().to_f32(), area.client_rect.y().to_f32()][axis],
        );
        let page = pixel_snapped_size(size, location).max(0);
        let mut position = current;
        position[axis] += direction.sign() * page_step(page);
        if style.scroll_snap_axis == ScrollSnapAxis::None {
            state.snap_targets = [None, None];
        }
        if snap_axes(style)[axis] {
            let Some(root) = self.latest_fragment.as_ref() else {
                return state;
            };
            let geometry = match self.snap_geometry(root, binding.origin) {
                Ok(Some(value)) => value,
                Ok(None) => return state,
                Err(reason) => {
                    state.availability = A::Unsupported(reason);
                    return state;
                }
            };
            for target in &mut state.snap_targets {
                if target
                    .is_some_and(|target| !geometry.areas.iter().any(|area| area.handle == target))
                {
                    *target = None;
                }
            }
            if let Some(target) = search_axis(
                &geometry.areas,
                axis,
                current,
                port,
                maximum,
                current[1 - axis].clamp(0.0, maximum[1 - axis]),
                Some((direction, page, !area.integer_offsets)),
                style.scroll_snap_strictness,
                self.scroll_buttons
                    .targeted_areas
                    .get(&binding.origin)
                    .copied(),
                None,
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

    fn snap_geometry(
        &self,
        root: &Fragment,
        origin: NodeHandle,
    ) -> Result<Option<SnapGeometry>, &'static str> {
        let node = self
            .resolve(origin)
            .map_err(|_| "snap origin has no retained identity")?;
        let Some(fragment) = find_fragment(root, node) else {
            return Ok(None);
        };
        let Some(area) = fragment.scroll_area else {
            return Ok(None);
        };
        let style = &self.document.node(node).style;
        if !area.is_scroll_container() {
            return Ok(None);
        }
        let limits = area.limits();
        let minimum = [limits.0 .0 as f32, limits.1 .0 as f32];
        let maximum = [
            (limits.0 .1 - limits.0 .0) as f32,
            (limits.1 .1 - limits.1 .0) as f32,
        ];
        let mut areas = Vec::new();
        if snap_axes(style) != [false, false] {
            self.collect_snap_areas(
                fragment,
                WorldTransform::IDENTITY,
                minimum,
                None,
                &mut HashSet::new(),
                &mut areas,
            )?;
            for candidate in &mut areas {
                let data = self.document.node(
                    self.resolve(candidate.handle)
                        .map_err(|_| "snap area has no retained identity")?,
                );
                let margin = scroll_margin(&data.style);
                candidate.rect[0] -= margin[0];
                candidate.rect[1] -= margin[1];
                candidate.rect[2] += margin[0] + margin[2];
                candidate.rect[3] += margin[1] + margin[3];
                candidate.align = normalized_alignment(
                    &data.style,
                    style,
                    candidate.rect,
                    [fragment.size.width.to_f32(), fragment.size.height.to_f32()],
                );
            }
        }
        Ok(Some(SnapGeometry {
            area,
            minimum,
            maximum,
            port: scroll_snapport(style, area),
            areas,
            axes: snap_axes(style),
            strictness: style.scroll_snap_strictness,
            horizontal: style.writing_mode.is_horizontal(),
        }))
    }

    fn collect_snap_areas(
        &self,
        fragment: &Fragment,
        parent: WorldTransform,
        minimum: [f32; 2],
        unsupported_ancestor: Option<&'static str>,
        seen: &mut HashSet<NodeId>,
        out: &mut Vec<SnapArea>,
    ) -> Result<(), &'static str> {
        for child in &fragment.children {
            let mut inherited = parent;
            let mut unsupported = unsupported_ancestor;
            for ancestor in &child.promoted_transform_ancestors {
                unsupported = unsupported.or_else(|| {
                    unsupported_snap_transform(&self.document.node(ancestor.node_id).style)
                });
                inherited = inherited.then(WorldTransform(
                    openui_layout::promoted_ancestor_applied_transform(&self.document, ancestor),
                ));
            }
            let translated = inherited.then(WorldTransform::translate(
                child.offset.left.to_f64(),
                child.offset.top.to_f64(),
            ));
            if child.node_id.is_none() {
                self.collect_snap_areas(child, translated, minimum, unsupported, seen, out)?;
                continue;
            }
            let data = self.document.node(child.node_id);
            let owns = child.owns_element_box();
            // kIgnoreScrollOffset does not imply kIgnoreStickyOffset: pinned
            // LayoutBoxModelObject keeps the current sticky translation.
            let unsupported = unsupported.or_else(|| {
                owns.then(|| unsupported_snap_transform(&data.style))
                    .flatten()
            });
            let world = if owns && data.style.has_transform() {
                let resolved = child.resolved_transform.unwrap_or_else(|| {
                    openui_layout::resolved_fragment_transform(child, &data.style)
                });
                translated.then(WorldTransform(resolved.local_applied))
            } else {
                translated
            };
            let has_alignment = data
                .style
                .scroll_snap_align_block
                .unwrap_or(data.style.scroll_snap_align)
                != ScrollSnapAlign::None
                || data
                    .style
                    .scroll_snap_align_inline
                    .unwrap_or(data.style.scroll_snap_align)
                    != ScrollSnapAlign::None;
            if owns && has_alignment {
                if let Some(reason) = unsupported {
                    return Err(reason);
                }
                let Some(index) = self.node_slots.get(&child.node_id) else {
                    return Err("snap area has no retained identity");
                };
                let handle = self.handle_for_slot(*index);
                let border = child.border_box_rect();
                let mapped = world
                    .then(WorldTransform::translate(
                        border.offset.left.to_f64(),
                        border.offset.top.to_f64(),
                    ))
                    .map_rect(border.size.width.to_f32(), border.size.height.to_f32());
                // Pinned QuadsInAncestor ignores retained scroll offsets;
                // enclosing physical rectangles quantize all four quad edges.
                let left = LayoutUnit::from_f32_floor(mapped.x).to_f32() - minimum[0];
                let top = LayoutUnit::from_f32_floor(mapped.y).to_f32() - minimum[1];
                let right =
                    LayoutUnit::from_f32_ceil(mapped.x + mapped.width).to_f32() - minimum[0];
                let bottom =
                    LayoutUnit::from_f32_ceil(mapped.y + mapped.height).to_f32() - minimum[1];
                if right > left && bottom > top {
                    let candidate = SnapArea {
                        handle,
                        rect: [left, top, right - left, bottom - top],
                        align: [ScrollSnapAlign::None; 2],
                        focus: self
                            .focused
                            .is_some_and(|focus| self.has_focus_within(handle, focus)),
                        must_stop: data.style.scroll_snap_stop == ScrollSnapStop::Always,
                    };
                    if seen.insert(child.node_id) {
                        out.push(candidate);
                    } else if let Some(area) = out.iter_mut().find(|area| area.handle == handle) {
                        let l = area.rect[0].min(left);
                        let t = area.rect[1].min(top);
                        let r = (area.rect[0] + area.rect[2]).max(right);
                        let b = (area.rect[1] + area.rect[3]).max(bottom);
                        area.rect = [l, t, r - l, b - t];
                    }
                }
            }
            // Each nested scrollport owns its descendant areas; helper table
            // boxes do not acquire transforms or scroller ownership again.
            if fragment_owns_scrollport(child, &self.document) {
                continue;
            }
            self.collect_snap_areas(child, world, minimum, unsupported, seen, out)?;
        }
        Ok(())
    }

    pub(crate) fn queue_snap_end_axis(&mut self, origin: NodeHandle, axis: usize) {
        self.scroll_buttons
            .pending_end_axes
            .entry(origin)
            .or_default()[axis] = true;
        self.scroll_animations
            .retain(|_, animation| animation.target != origin);
    }

    /// Settle all authored snap containers against the new layout,
    /// retaining still-present target identities before distance selection.
    /// Called within the existing layout barrier, never recursively via update.
    pub(crate) fn settle_native_snap_after_layout(&mut self, root: &Fragment) -> bool {
        fn collect(
            fragment: &Fragment,
            document: &openui_dom::Document,
            pending_nodes: &HashSet<NodeId>,
            out: &mut HashSet<NodeId>,
        ) {
            let pending = pending_nodes.contains(&fragment.node_id);
            if fragment.owns_element_box()
                && !fragment.node_id.is_none()
                && (document.node(fragment.node_id).style.scroll_snap_axis != ScrollSnapAxis::None
                    || pending)
            {
                out.insert(fragment.node_id);
            }
            for child in &fragment.children {
                collect(child, document, pending_nodes, out);
            }
        }
        let pending_nodes = self
            .scroll_buttons
            .pending_end_axes
            .keys()
            .filter_map(|origin| self.resolve(*origin).ok())
            .collect();
        let mut origins = HashSet::new();
        collect(root, &self.document, &pending_nodes, &mut origins);
        let mut ordered: Vec<_> = origins.into_iter().collect();
        ordered.sort_by_key(|node| self.node_slots.get(node).copied().unwrap_or(u32::MAX));
        let mut changed = false;
        for node in ordered {
            let Some(index) = self.node_slots.get(&node) else {
                continue;
            };
            let origin = self.handle_for_slot(*index);
            let requested_axes = self.scroll_buttons.pending_end_axes.remove(&origin);
            let geometry = match self.snap_geometry(root, origin) {
                Ok(Some(value)) => value,
                _ => continue,
            };
            if geometry.axes == [false, false] {
                self.scroll_buttons.snap_targets.remove(&origin);
                continue;
            }
            if self
                .scroll_animations
                .values()
                .any(|animation| animation.target == origin)
            {
                continue;
            }
            let data = self.document.node(node);
            let current = [
                data.scroll_left - geometry.minimum[0],
                data.scroll_top - geometry.minimum[1],
            ];
            let has_prior_layout = self.scroll_buttons.snap_targets.contains_key(&origin);
            let old = self
                .scroll_buttons
                .snap_targets
                .get(&origin)
                .copied()
                .unwrap_or([None, None]);
            let (position, targets) = select_end_position(
                &geometry.areas,
                current,
                geometry.port,
                geometry.maximum,
                requested_axes
                    .filter(|_| has_prior_layout)
                    .map_or(geometry.axes, |axes| {
                        [axes[0] && geometry.axes[0], axes[1] && geometry.axes[1]]
                    }),
                geometry.strictness,
                old,
                requested_axes.is_none(),
                self.scroll_buttons.targeted_areas.get(&origin).copied(),
                geometry.horizontal,
            );
            self.scroll_buttons.snap_targets.insert(origin, targets);
            let next = geometry.area.clamp_offset(
                (position[0] + geometry.minimum[0]) as f64,
                (position[1] + geometry.minimum[1]) as f64,
            );
            if next != (data.scroll_left, data.scroll_top) {
                let data = self.document.node_mut(node);
                data.scroll_left = next.0;
                data.scroll_top = next.1;
                changed = true;
            }
        }
        changed
    }

    /// Resolve an authored endpoint through the same retained snap data used
    /// by buttons and layout resnapping. Axes that did not move keep their IDs.
    pub(crate) fn resolve_native_scroll_endpoint(
        &mut self,
        origin: NodeHandle,
        x: f32,
        y: f32,
    ) -> Result<(f32, f32), EngineError> {
        self.resolve_native_endpoint(origin, x, y, false)
    }

    /// Blink reveal selects an end position on both configured snap axes,
    /// including an axis whose alignment leaves its current offset unchanged.
    pub(crate) fn resolve_native_reveal_endpoint(
        &mut self,
        origin: NodeHandle,
        x: f32,
        y: f32,
    ) -> Result<(f32, f32), EngineError> {
        self.resolve_native_endpoint(origin, x, y, true)
    }

    fn resolve_native_endpoint(
        &mut self,
        origin: NodeHandle,
        x: f32,
        y: f32,
        snap_unchanged_axes: bool,
    ) -> Result<(f32, f32), EngineError> {
        let node = self.resolve(origin)?;
        if self.document.node(node).style.scroll_snap_axis == ScrollSnapAxis::None {
            return Ok((x, y));
        }
        let Some(root) = self.latest_fragment.as_ref() else {
            return Ok((x, y));
        };
        let geometry = self
            .snap_geometry(root, origin)
            .map_err(EngineError::InvalidInput)?;
        let Some(geometry) = geometry else {
            return Ok((x, y));
        };
        let old_offset = self.scroll_offset(origin)?;
        let axes = [
            geometry.axes[0] && (snap_unchanged_axes || x as f64 != old_offset.0),
            geometry.axes[1] && (snap_unchanged_axes || y as f64 != old_offset.1),
        ];
        let current = [x - geometry.minimum[0], y - geometry.minimum[1]];
        let old = self
            .scroll_buttons
            .snap_targets
            .get(&origin)
            .copied()
            .unwrap_or([None, None]);
        let (position, targets) = select_end_position(
            &geometry.areas,
            current,
            geometry.port,
            geometry.maximum,
            axes,
            geometry.strictness,
            old,
            false,
            self.scroll_buttons.targeted_areas.get(&origin).copied(),
            geometry.horizontal,
        );
        self.scroll_buttons.snap_targets.insert(origin, targets);
        Ok(geometry.area.clamp_offset(
            (position[0] + geometry.minimum[0]) as f64,
            (position[1] + geometry.minimum[1]) as f64,
        ))
    }

    /// End a native scrolling operation at the nearest authored snap state.
    /// This consumes retained layout data, not application-provided point lists.
    pub fn settle_native_scroll_snap(&mut self, origin: NodeHandle) -> Result<bool, EngineError> {
        self.resolve(origin)?;
        self.update()?;
        let Some(root) = self.latest_fragment.as_ref() else {
            return Ok(false);
        };
        let Some(geometry) = self
            .snap_geometry(root, origin)
            .map_err(EngineError::InvalidInput)?
        else {
            return Ok(false);
        };
        if geometry.axes == [false, false] {
            return Ok(false);
        }
        let before = self.scroll_offset(origin)?;
        let current = [
            before.0 as f32 - geometry.minimum[0],
            before.1 as f32 - geometry.minimum[1],
        ];
        let old = self
            .scroll_buttons
            .snap_targets
            .get(&origin)
            .copied()
            .unwrap_or([None, None]);
        let (position, targets) = select_end_position(
            &geometry.areas,
            current,
            geometry.port,
            geometry.maximum,
            geometry.axes,
            geometry.strictness,
            old,
            false,
            self.scroll_buttons.targeted_areas.get(&origin).copied(),
            geometry.horizontal,
        );
        let next = geometry.area.clamp_offset(
            (position[0] + geometry.minimum[0]) as f64,
            (position[1] + geometry.minimum[1]) as f64,
        );
        self.scroll_buttons.snap_targets.insert(origin, targets);
        self.scroll_to_resolved(origin, next.0, next.1)?;
        Ok(before != (next.0 as f64, next.1 as f64))
    }

    fn prune_native_snap_identities(&mut self) {
        let origins: HashSet<_> = self
            .scroll_buttons
            .snap_targets
            .keys()
            .chain(self.scroll_buttons.targeted_areas.keys())
            .copied()
            .chain(
                self.scroll_buttons
                    .bindings
                    .values()
                    .map(|binding| binding.origin),
            )
            .collect();
        for origin in origins {
            let geometry = self
                .latest_fragment
                .as_ref()
                .and_then(|root| self.snap_geometry(root, origin).ok().flatten());
            let targets = self
                .scroll_buttons
                .snap_targets
                .entry(origin)
                .or_insert([None, None]);
            for axis in 0..2 {
                targets[axis] = targets[axis].filter(|target| {
                    geometry.as_ref().is_some_and(|geometry| {
                        geometry.axes[axis]
                            && geometry.areas.iter().any(|area| {
                                area.handle == *target && area.align[axis] != ScrollSnapAlign::None
                            })
                    })
                });
            }
            if self
                .scroll_buttons
                .targeted_areas
                .get(&origin)
                .is_some_and(|target| {
                    !geometry.as_ref().is_some_and(|geometry| {
                        geometry.areas.iter().any(|area| area.handle == *target)
                    })
                })
            {
                self.scroll_buttons.targeted_areas.remove(&origin);
            }
        }
    }

    pub(crate) fn refresh_scroll_button_states(&mut self) {
        if !self.scroll_buttons.dirty {
            return;
        }
        self.prune_native_snap_identities();
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
        self.scroll_to_resolved(state.origin, x as f32, y as f32)?;
        self.scroll_buttons
            .snap_targets
            .insert(state.origin, state.snap_targets);
        self.refresh_scroll_button_states();
        Ok(ActivationResult { changed: vec![] })
    }
}

fn unsupported_snap_transform(style: &ComputedStyle) -> Option<&'static str> {
    // Read the effective retained operations, including sampled/deferred
    // animation values and removed ancestors. Until the 3D flattening policy
    // is shared here, those quads must remain an explicit unsupported result.
    (style
        .native_transform
        .as_ref()
        .is_some_and(openui_style::transform_contains_3d)
        || style.transform_matrix.is_some_and(|matrix| !matrix.is_2d()))
    .then_some("three-dimensional snap quads require retained 3D scene geometry")
}

/// Actual nested scrolling ownership follows retained layout. A table,
/// replaced box or viewport-defining body may have non-visible authored
/// overflow without creating a local scrollport. Helpers cannot acquire
/// their source element's scrolling boundary.
fn fragment_owns_scrollport(fragment: &Fragment, document: &openui_dom::Document) -> bool {
    if !fragment.owns_element_box() {
        return false;
    }
    fragment.scroll_area.map_or_else(
        || {
            let (x, y) = openui_layout::used_overflow_pair(document, fragment.node_id);
            [x, y]
                .into_iter()
                .any(|value| matches!(value, Overflow::Hidden | Overflow::Auto | Overflow::Scroll))
        },
        ScrollArea::is_scroll_container,
    )
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
#[path = "snapshot_scroll_snap_tests.rs"]
mod snapshot_scroll_snap_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FontCollection, ViewportMetrics};
    use openui_style::{LengthValue, Position, ScrollbarWidth};

    fn retained_engine(scale: f64) -> Engine {
        Engine::new_with_font_collection(
            ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
            FontCollection::deterministic_test(),
        )
        .unwrap()
    }

    fn native_box(engine: &mut Engine, tag: ElementTag, width: f32, height: f32) -> NodeHandle {
        let node = engine.create_native_element(tag).unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(width).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::px(height).into())
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::ScrollbarWidth,
                StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
            )
            .unwrap();
        node
    }

    #[test]
    fn native_scroll_button_state_follows_retained_scroll_ownership() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let mut engine = retained_engine(scale);
            let root = engine.root();
            let block = native_box(&mut engine, ElementTag::Div, 100.0, 80.0);
            let content = native_box(&mut engine, ElementTag::Div, 300.0, 20.0);
            engine.append_child(root, block).unwrap();
            engine.append_child(block, content).unwrap();
            let button = engine
                .create_scroll_button(block, ScrollButtonDirection::Right)
                .unwrap();
            for (overflow, expected) in [
                (Overflow::Hidden, ScrollButtonAvailability::Available),
                (Overflow::Clip, ScrollButtonAvailability::NoScrollport),
                (Overflow::Visible, ScrollButtonAvailability::NoScrollport),
                (Overflow::Scroll, ScrollButtonAvailability::Available),
                (Overflow::Hidden, ScrollButtonAvailability::Available),
            ] {
                engine
                    .set_property(block, StyleProperty::Overflow, overflow.into())
                    .unwrap();
                let state = engine.scroll_button_state(button).unwrap();
                assert_eq!(
                    state.availability, expected,
                    "block {overflow:?}, scale={scale}"
                );
                assert_eq!(
                    state.enabled,
                    expected == ScrollButtonAvailability::Available
                );
            }
            engine.remove(block).unwrap();

            let table = native_box(&mut engine, ElementTag::Table, 100.0, 80.0);
            let row = engine.create_native_element(ElementTag::TableRow).unwrap();
            let cell = native_box(&mut engine, ElementTag::TableCell, 100.0, 80.0);
            engine.append_child(root, table).unwrap();
            engine.append_child(table, row).unwrap();
            engine.append_child(row, cell).unwrap();
            engine
                .set_property(
                    table,
                    StyleProperty::BorderSpacing,
                    StyleValue::Renderer(RendererStyleValue::BorderSpacing((
                        Length::px(0.0),
                        Length::px(0.0),
                    ))),
                )
                .unwrap();
            let button = engine
                .create_scroll_button(table, ScrollButtonDirection::Right)
                .unwrap();
            for (x, y, expected) in [
                (
                    Overflow::Auto,
                    Overflow::Auto,
                    ScrollButtonAvailability::NoScrollport,
                ),
                (
                    Overflow::Scroll,
                    Overflow::Scroll,
                    ScrollButtonAvailability::NoScrollport,
                ),
                (
                    Overflow::Hidden,
                    Overflow::Auto,
                    ScrollButtonAvailability::NoScrollport,
                ),
                (
                    Overflow::Hidden,
                    Overflow::Hidden,
                    ScrollButtonAvailability::Boundary,
                ),
                (
                    Overflow::Clip,
                    Overflow::Clip,
                    ScrollButtonAvailability::NoScrollport,
                ),
            ] {
                engine
                    .set_property(table, StyleProperty::OverflowX, x.into())
                    .unwrap();
                engine
                    .set_property(table, StyleProperty::OverflowY, y.into())
                    .unwrap();
                let state = engine.scroll_button_state(button).unwrap();
                assert_eq!(
                    state.availability, expected,
                    "table {x:?}/{y:?}, scale={scale}"
                );
                assert!(!state.enabled);
                assert_eq!(state.current_snap_targets, [None, None]);
            }
            engine.remove(table).unwrap();

            let image = native_box(&mut engine, ElementTag::Image, 100.0, 80.0);
            let resource = engine.register_image_resource(
                "memory:scroll-ownership-pixel.svg",
                "image/svg+xml",
                "949c974a51bfed3daa2be2aed981026336857d4cfd494dd120dbb6da66c891f7",
                br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="black"/></svg>"#.to_vec(),
            );
            engine
                .set_image_resource(image, resource, Some((1.0, 1.0)))
                .unwrap();
            engine.append_child(root, image).unwrap();
            let button = engine
                .create_scroll_button(image, ScrollButtonDirection::Right)
                .unwrap();
            for overflow in [Overflow::Scroll, Overflow::Hidden, Overflow::Clip] {
                engine
                    .set_property(image, StyleProperty::Overflow, overflow.into())
                    .unwrap();
                let state = engine.scroll_button_state(button).unwrap();
                assert_eq!(state.availability, ScrollButtonAvailability::NoScrollport);
                assert_eq!(state.target_offset, None);
                engine.scroll_to(image, 1000.0, 1000.0).unwrap();
                assert_eq!(engine.scroll_offset(image).unwrap(), (0.0, 0.0));
            }
            engine.remove(image).unwrap();

            let html = engine.create_native_element(ElementTag::Html).unwrap();
            let body = native_box(&mut engine, ElementTag::Body, 100.0, 80.0);
            let content = native_box(&mut engine, ElementTag::Div, 300.0, 20.0);
            engine.append_child(root, html).unwrap();
            engine.append_child(html, body).unwrap();
            engine.append_child(body, content).unwrap();
            engine
                .set_property(body, StyleProperty::Overflow, Overflow::Scroll.into())
                .unwrap();
            let button = engine
                .create_scroll_button(body, ScrollButtonDirection::Right)
                .unwrap();
            for (html_overflow, expected) in [
                (Overflow::Visible, ScrollButtonAvailability::NoScrollport),
                (Overflow::Hidden, ScrollButtonAvailability::Available),
                (Overflow::Visible, ScrollButtonAvailability::NoScrollport),
            ] {
                engine
                    .set_property(html, StyleProperty::Overflow, html_overflow.into())
                    .unwrap();
                let state = engine.scroll_button_state(button).unwrap();
                assert_eq!(
                    state.availability, expected,
                    "body propagation, scale={scale}"
                );
                assert_eq!(
                    state.enabled,
                    expected == ScrollButtonAvailability::Available
                );
            }
        }
    }

    #[test]
    fn native_snap_descendants_follow_retained_container_boundaries() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            for (tag, x, y, descendant_belongs_to_outer) in [
                (ElementTag::Table, Overflow::Scroll, Overflow::Scroll, true),
                (ElementTag::Table, Overflow::Auto, Overflow::Auto, true),
                (ElementTag::Table, Overflow::Hidden, Overflow::Auto, true),
                (ElementTag::Table, Overflow::Hidden, Overflow::Hidden, false),
                (ElementTag::Table, Overflow::Clip, Overflow::Clip, true),
                (ElementTag::Div, Overflow::Scroll, Overflow::Scroll, false),
                (ElementTag::Div, Overflow::Hidden, Overflow::Hidden, false),
                (ElementTag::Div, Overflow::Clip, Overflow::Clip, true),
            ] {
                let mut engine = retained_engine(scale);
                let root = engine.root();
                let outer = native_box(&mut engine, ElementTag::Div, 100.0, 80.0);
                engine.append_child(root, outer).unwrap();
                engine
                    .set_property(outer, StyleProperty::Overflow, Overflow::Hidden.into())
                    .unwrap();
                let middle = native_box(&mut engine, tag, 300.0, 80.0);
                engine.append_child(outer, middle).unwrap();
                engine
                    .set_property(middle, StyleProperty::OverflowX, x.into())
                    .unwrap();
                engine
                    .set_property(middle, StyleProperty::OverflowY, y.into())
                    .unwrap();
                let positioning_parent = if tag == ElementTag::Table {
                    engine
                        .set_property(
                            middle,
                            StyleProperty::BorderSpacing,
                            StyleValue::Renderer(RendererStyleValue::BorderSpacing((
                                Length::px(0.0),
                                Length::px(0.0),
                            ))),
                        )
                        .unwrap();
                    let row = engine.create_native_element(ElementTag::TableRow).unwrap();
                    let cell = native_box(&mut engine, ElementTag::TableCell, 300.0, 80.0);
                    engine.append_child(middle, row).unwrap();
                    engine.append_child(row, cell).unwrap();
                    cell
                } else {
                    middle
                };
                engine
                    .set_property(
                        positioning_parent,
                        StyleProperty::Position,
                        Position::Relative.into(),
                    )
                    .unwrap();
                let target = native_box(&mut engine, ElementTag::Div, 20.0, 20.0);
                engine
                    .set_property(target, StyleProperty::Position, Position::Absolute.into())
                    .unwrap();
                engine
                    .set_property(target, StyleProperty::Left, LengthValue::px(160.0).into())
                    .unwrap();
                engine
                    .set_property(target, StyleProperty::Top, LengthValue::px(0.0).into())
                    .unwrap();
                engine
                    .set_property(
                        target,
                        StyleProperty::ScrollSnapAlign,
                        StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(
                            ScrollSnapAlign::Start,
                        )),
                    )
                    .unwrap();
                engine.append_child(positioning_parent, target).unwrap();
                engine
                    .set_property(
                        outer,
                        StyleProperty::ScrollSnapAxis,
                        StyleValue::Renderer(RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::X)),
                    )
                    .unwrap();
                let button = engine
                    .create_scroll_button(outer, ScrollButtonDirection::Right)
                    .unwrap();
                let state = engine.scroll_button_state(button).unwrap();
                assert_eq!(
                    state.current_snap_targets[0],
                    descendant_belongs_to_outer.then_some(target),
                    "tag={tag:?}, overflow={x:?}/{y:?}, scale={scale}"
                );
                assert_eq!(
                    engine.scroll_offset(outer).unwrap().0,
                    if descendant_belongs_to_outer {
                        160.0
                    } else {
                        0.0
                    }
                );
            }
        }
    }

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
            pixel_snapped_size(LayoutUnit::from_f32(99.5), LayoutUnit::zero()),
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
