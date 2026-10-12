//! Retained, untransformed scroll/view timeline geometry.
//! Pinned Blink 147.0.7727.50 uses 16 microseconds per CSS pixel.
use crate::{animation::RetainedAnimationTimeline, AnimationTimelineSource, Engine, NodeHandle};
use openui_dom::NodeId;
use openui_geometry::LayoutUnit;
use openui_layout::{Fragment, ScrollArea};
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, AnimationTimingSeconds, IterationCount,
    LengthValue, NamedTimelineRanges, Overflow, TimelineAxis, TimelineRangeBounds,
    TimelineRangeOffset, ViewTimelineInset,
};
use std::collections::HashMap;

const TIMELINE_MICROSECONDS_PER_CSS_PIXEL: f64 = 16.0;

fn cover_clock_seconds(scroll_offset: f64, cover_start: f64) -> f64 {
    let microseconds = ((scroll_offset - cover_start) * TIMELINE_MICROSECONDS_PER_CSS_PIXEL) as i64;
    microseconds as f64 / 1_000_000.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationTimelineSnapshot {
    pub source: Option<NodeHandle>,
    pub active: bool,
    pub physical_axis: Option<TimelineAxis>,
    pub scroll_offset: Option<f64>,
    pub cover_start: Option<f64>,
    pub cover_end: Option<f64>,
    pub range_start: f64,
    pub range_end: f64,
    pub duration_ms: Option<f64>,
    pub current_time_ms: Option<f64>,
    pub current_time_percentage: Option<f64>,
}

/// An immutable copy of authored browser attachment endpoints and their
/// currently resolved geometry. Inactive sources keep the descriptor and no bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationAttachmentSnapshot {
    pub range: AnimationAttachmentRange,
    pub resolved: Option<AnimationResolvedTimelineRanges>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationResolvedTimelineRanges {
    pub source: NodeHandle,
    /// Every range uses absolute CSS pixel scroll positions along the chosen axis.
    pub named: NamedTimelineRanges,
    pub attachment: TimelineRangeBounds,
    pub entry_crossing_distance: f64,
    pub exit_crossing_distance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProgressTimelineSnapshot {
    pub source: NodeHandle,
    pub axis: usize,
    pub scroll_offset: f64,
    pub cover_start: f64,
    pub cover_end: f64,
    pub range: TimelineRangeBounds,
    pub legacy_oriented: bool,
    pub attachment_start_fraction: f64,
    pub attachment_end_fraction: f64,
    pub attachment_bounds: TimelineRangeBounds,
    pub named_ranges: NamedTimelineRanges,
    pub entry_crossing_distance: f64,
    pub exit_crossing_distance: f64,
    // Cover-relative integer-us clock, before attachment alignment.
    pub cover_time_seconds: f64,
    pub attachment_start_seconds: f64,
    pub attachment_end_seconds: f64,
    pub scroll_maximum: f64,
    pub duration_seconds: f64,
}

impl ProgressTimelineSnapshot {
    pub fn reflection(self, local_time_seconds: Option<f64>) -> AnimationTimelineSnapshot {
        AnimationTimelineSnapshot {
            source: Some(self.source),
            active: true,
            physical_axis: Some(if self.axis == 0 {
                TimelineAxis::X
            } else {
                TimelineAxis::Y
            }),
            scroll_offset: Some(self.scroll_offset),
            cover_start: Some(self.cover_start),
            cover_end: Some(self.cover_end),
            range_start: self.range.start,
            range_end: self.range.end,
            duration_ms: Some(self.duration_seconds * 1000.0),
            current_time_ms: local_time_seconds.map(|time| time * 1000.0),
            current_time_percentage: local_time_seconds
                .map(|time| time / self.duration_seconds * 100.0),
        }
    }

    pub fn attachment_reflection(self) -> AnimationResolvedTimelineRanges {
        AnimationResolvedTimelineRanges {
            source: self.source,
            named: self.named_ranges,
            attachment: self.attachment_bounds,
            entry_crossing_distance: self.entry_crossing_distance,
            exit_crossing_distance: self.exit_crossing_distance,
        }
    }

    pub fn time_from_percentage(self, percentage: f64) -> f64 {
        (percentage / 100.0) * self.duration_seconds
    }

    // Legacy reversed numeric ranges are an existing native extension. Browser
    // attachment ranges retain the raw cover clock and their actual start/end.
    fn polarity(self) -> f64 {
        if self.legacy_oriented && self.range.end < self.range.start {
            -1.0
        } else {
            1.0
        }
    }

    pub fn clock_time_seconds(self) -> f64 {
        self.cover_time_seconds * self.polarity()
    }

    pub fn automatic_start_time_seconds(self, rate: f64) -> f64 {
        let offset = if rate < 0.0 {
            self.attachment_end_seconds
        } else {
            self.attachment_start_seconds
        };
        offset * self.polarity()
    }

    pub fn boundary_alignment(
        self,
        options: &AnimationOptions,
        timing: AnimationTimingSeconds,
        auto_align: bool,
    ) -> (bool, bool) {
        if !auto_align {
            return (false, false);
        }
        // animation.cc:2979-3036. This is timeline phase policy, independent
        // of the renderer's strict pixel comparison gate.
        let mut start = self.attachment_start_fraction;
        let end = self.attachment_end_fraction;
        if self.duration_seconds > 0.0 {
            start += timing.delay / self.duration_seconds;
        }
        let start_offset = start * self.cover_end + (1.0 - start) * self.cover_start;
        let end_offset = end * self.cover_end + (1.0 - end) * self.cover_start;
        (
            options.playback_rate < 0.0 && start_offset <= 0.5,
            options.playback_rate > 0.0
                && options.playback_rate * end_offset >= self.scroll_maximum - 0.5,
        )
    }

    pub fn normalize(
        self,
        specified: &AnimationOptions,
    ) -> (AnimationOptions, AnimationTimingSeconds) {
        let mut timing = specified.clone();
        // Default .50 AnimationTimeDelta stores double seconds. Keep that
        // operation order here and reflect native milliseconds only at the end.
        let duration = specified.duration_ms / 1000.0;
        let delay = specified.delay_ms / 1000.0;
        let count = match specified.iterations {
            IterationCount::Number(count) => count,
            IterationCount::Infinite => f64::INFINITY,
        };
        let active = if duration == 0.0 {
            0.0
        } else {
            duration * count
        };
        let end = (delay + active).max(0.0);
        let (delay, duration) = if end == 0.0 {
            (0.0, 0.0)
        } else if end.is_infinite() {
            // animation_effect.cc:132-150 evaluates the infinite-end limit
            // with reciprocal multiplication, including infinite iterations.
            (0.0, (1.0 / count) * self.duration_seconds)
        } else {
            let mut interval = 1.0;
            interval -= self.attachment_start_fraction;
            interval -= 1.0 - self.attachment_end_fraction;
            // The native API also preserves its pre-existing reversed ranges.
            if self.legacy_oriented && self.range.end < self.range.start {
                interval = -interval;
            }
            interval = interval.max(0.0);
            let intrinsic = if count > 0.0 {
                self.duration_seconds * interval / count
            } else {
                0.0
            };
            let range_duration = intrinsic * count;
            (
                delay / end * range_duration,
                duration / end * range_duration,
            )
        };
        timing.delay_ms = delay * 1000.0;
        timing.duration_ms = duration * 1000.0;
        (timing, AnimationTimingSeconds { delay, duration })
    }
}

#[derive(Clone, Copy)]
struct LayoutBox {
    origin: [f64; 2],
    scroll_origin: [f64; 2],
    rect: [f64; 4],
    area: Option<ScrollArea>,
    sticky: Option<openui_layout::StickyTimelineGeometry>,
}

fn boxes(root: &Fragment) -> HashMap<NodeId, LayoutBox> {
    fn walk(fragment: &Fragment, parent: [f64; 2], output: &mut HashMap<NodeId, LayoutBox>) {
        let offset = fragment
            .sticky_timeline_geometry
            .map_or(fragment.offset, |geometry| {
                geometry.parent_relative_normal_offset
            });
        let origin = [
            parent[0] + offset.left.to_f64(),
            parent[1] + offset.top.to_f64(),
        ];
        if fragment.owns_element_box() {
            let border = fragment.border_box_rect();
            let rect = [
                origin[0] + border.x().to_f64(),
                origin[1] + border.y().to_f64(),
                border.width().to_f64(),
                border.height().to_f64(),
            ];
            let rect = if fragment.is_inline_box_fragment {
                rect
            } else if let Some(geometry) = fragment.box_geometry {
                [
                    rect[0],
                    rect[1],
                    geometry.stitched_size.width.to_f64(),
                    geometry.stitched_size.height.to_f64(),
                ]
            } else {
                rect
            };
            let next = LayoutBox {
                origin,
                scroll_origin: origin,
                rect,
                area: fragment.scroll_area,
                sticky: fragment.sticky_timeline_geometry,
            };
            output
                .entry(fragment.node_id)
                .and_modify(|existing| {
                    if fragment.is_inline_box_fragment {
                        let right = (existing.rect[0] + existing.rect[2]).max(rect[0] + rect[2]);
                        let bottom = (existing.rect[1] + existing.rect[3]).max(rect[1] + rect[3]);
                        existing.rect[0] = existing.rect[0].min(rect[0]);
                        existing.rect[1] = existing.rect[1].min(rect[1]);
                        existing.rect[2] = right - existing.rect[0];
                        existing.rect[3] = bottom - existing.rect[1];
                    }
                })
                .or_insert(next);
        }
        if let Some(openui_layout::AnonymousLayoutBox::FieldsetContent { owner }) =
            fragment.anonymous_layout_box
        {
            if let Some(wrapper) = output.get_mut(&owner) {
                wrapper.area = fragment.scroll_area;
                wrapper.scroll_origin = origin;
            }
        }
        for child in &fragment.children {
            walk(child, origin, output);
        }
    }
    let mut output = HashMap::new();
    walk(root, [0.0; 2], &mut output);
    output
}

fn scroll_container(area: ScrollArea) -> bool {
    [area.overflow_x, area.overflow_y]
        .iter()
        .any(|value| matches!(value, Overflow::Hidden | Overflow::Auto | Overflow::Scroll))
}

// Pinned MinimumValueForLength: the percentage basis and returned offset are
// LayoutUnits, with distinct f32 multiplication order for percent and calc.
fn minimum_length(length: &openui_geometry::Length, extent: f64) -> f64 {
    let basis = LayoutUnit::from_f64(extent).to_f32();
    let value = if length.is_fixed() {
        length.value()
    } else if length.is_percent() {
        basis * length.value() / 100.0
    } else if length.is_calculated() {
        length.calc_offset() + length.value() / 100.0 * basis
    } else {
        0.0
    };
    LayoutUnit::from_f32(value).to_f64()
}

#[derive(Clone, Copy, PartialEq)]
enum StickinessRange {
    BeforeEntry,
    DuringEntry,
    Contained,
    Covering,
    DuringExit,
    AfterExit,
}
fn stickiness(stuck: f64, sticky: f64, viewport: f64, size: f64, target: f64) -> StickinessRange {
    let position = stuck + target - sticky;
    if position < 0.0 && position + size > viewport {
        StickinessRange::Covering
    } else if position > viewport {
        StickinessRange::BeforeEntry
    } else if position + size > viewport {
        StickinessRange::DuringEntry
    } else if position + size < 0.0 {
        StickinessRange::AfterExit
    } else if position < 0.0 {
        StickinessRange::DuringExit
    } else {
        StickinessRange::Contained
    }
}

impl Engine {
    fn resolve_timeline_length(
        &self,
        reference: NodeId,
        value: LengthValue,
    ) -> openui_geometry::Length {
        let style = &self.document.node(reference).style;
        let root_style = &self.document.node(self.document.root()).style;
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        if value.depends_on_font_metrics() {
            value.resolve_with_font_metrics(
                viewport,
                style.font_size,
                root_style.font_size,
                self.native_font_length_metrics(style),
            )
        } else {
            value.resolve(viewport, style.font_size, root_style.font_size)
        }
    }

    pub(crate) fn progress_timeline_snapshot(
        &self,
        timeline: RetainedAnimationTimeline,
    ) -> Option<ProgressTimelineSnapshot> {
        let root = self.latest_fragment.as_ref()?;
        let geometry = boxes(root);
        let legacy_range = timeline.legacy_range();
        let browser_range = timeline.attachment();
        let (source_node, subject, axis) = match timeline.source() {
            AnimationTimelineSource::Document => return None,
            AnimationTimelineSource::Scroll { source, axis } => {
                let source = self.resolve(source).ok()?;
                if !self.node_is_connected(source) {
                    return None;
                }
                (source, None, axis)
            }
            AnimationTimelineSource::View { subject, axis }
            | AnimationTimelineSource::ViewWithInset { subject, axis, .. } => {
                let subject = self.resolve(subject).ok()?;
                if !self.node_is_connected(subject) || !geometry.contains_key(&subject) {
                    return None;
                }
                let source = root.nearest_scroll_ancestor(subject);
                (source?, Some(subject), axis)
            }
        };
        let source_box = geometry.get(&source_node)?;
        let area = source_box.area?;
        if !scroll_container(area) && source_node != self.document.root() {
            return None;
        }
        let source = self
            .node_slots
            .get(&source_node)
            .map(|index| self.handle_for_slot(*index))?;
        let style = self.retained_scroll_style(source_node);
        let axis = match axis {
            TimelineAxis::X => 0,
            TimelineAxis::Y => 1,
            TimelineAxis::Inline => usize::from(!style.writing_mode.is_horizontal()),
            TimelineAxis::Block => usize::from(style.writing_mode.is_horizontal()),
        };
        let offsets = self.document.node(source_node);
        let scroll_offset = if axis == 0 {
            offsets.scroll_left as f64
        } else {
            offsets.scroll_top as f64
        }
        .abs();
        let limits = area.limits();
        let maximum = if axis == 0 {
            limits.0 .1 - limits.0 .0
        } else {
            limits.1 .1 - limits.1 .0
        };
        let (mut start, mut end) = (0.0, maximum);
        let mut entry_crossing_distance = 0.0;
        let mut exit_crossing_distance = 0.0;
        if let Some(subject) = subject {
            let target = geometry.get(&subject)?;
            let client_offset = if axis == 0 {
                area.client_rect.x()
            } else {
                area.client_rect.y()
            }
            .to_f64();
            let viewport = if axis == 0 {
                area.client_rect.width()
            } else {
                area.client_rect.height()
            }
            .to_f64();
            let position = target.rect[axis] - source_box.scroll_origin[axis] - client_offset;
            let size = target.rect[axis + 2];
            entry_crossing_distance = size;
            exit_crossing_distance = size;
            let writing = style.direction.writing_direction(style.writing_mode);
            let reversed = if axis == 0 {
                if writing.is_horizontal() {
                    writing.is_rtl()
                } else {
                    writing.is_flipped_blocks()
                }
            } else {
                !writing.is_horizontal()
                    && (writing.is_rtl()
                        != (style.writing_mode == openui_style::WritingMode::SidewaysLr))
            };
            let (low, high) = if axis == 0 {
                (&style.scroll_padding_left, &style.scroll_padding_right)
            } else {
                (&style.scroll_padding_top, &style.scroll_padding_bottom)
            };
            let (start_inset, end_inset) = if reversed { (high, low) } else { (low, high) };
            let inset = match timeline.source() {
                AnimationTimelineSource::ViewWithInset { inset, .. } => inset,
                _ => ViewTimelineInset::default(),
            };
            let start_inset = inset
                .start
                .map(|value| self.resolve_timeline_length(subject, value))
                .unwrap_or_else(|| start_inset.clone());
            let end_inset = inset
                .end
                .map(|value| self.resolve_timeline_length(subject, value))
                .unwrap_or_else(|| end_inset.clone());
            start = position - viewport + minimum_length(&end_inset, viewport);
            end = position + size - minimum_length(&start_inset, viewport);
            let mut sticky_node = subject;
            while sticky_node != source_node && !sticky_node.is_none() {
                if let Some(sticky_box) = geometry.get(&sticky_node) {
                    if let Some(sticky) = sticky_box
                        .sticky
                        .filter(|sticky| sticky.scroll_container == source_node)
                    {
                        let sticky_position = if axis == 0 {
                            sticky.scroll_container_relative_normal_offset.left
                        } else {
                            sticky.scroll_container_relative_normal_offset.top
                        }
                        .to_f64()
                            - client_offset;
                        let container_position = if axis == 0 {
                            sticky.containing_block_rect.x()
                        } else {
                            sticky.containing_block_rect.y()
                        }
                        .to_f64()
                            - client_offset;
                        let container_size = if axis == 0 {
                            sticky.containing_block_rect.width()
                        } else {
                            sticky.containing_block_rect.height()
                        }
                        .to_f64();
                        let sticky_size = sticky_box.rect[axis + 2];
                        let (minimum, maximum) = if axis == 0 {
                            (sticky.insets.left, sticky.insets.right)
                        } else {
                            (sticky.insets.top, sticky.insets.bottom)
                        };
                        let mut forward = StickinessRange::Contained;
                        let mut backward = StickinessRange::Contained;
                        let mut forward_adjust = 0.0;
                        let mut backward_adjust = 0.0;
                        if let Some(inset) = minimum {
                            forward_adjust =
                                container_position + container_size - sticky_position - sticky_size;
                            forward = stickiness(
                                inset.to_f64(),
                                sticky_position,
                                viewport,
                                size,
                                position,
                            );
                        }
                        if let Some(inset) = maximum {
                            backward_adjust = container_position - sticky_position;
                            backward = stickiness(
                                viewport - inset.to_f64() - sticky_size,
                                sticky_position,
                                viewport,
                                size,
                                position,
                            );
                        }
                        if forward == StickinessRange::BeforeEntry {
                            start += forward_adjust;
                        }
                        if backward != StickinessRange::BeforeEntry {
                            start += backward_adjust;
                        }
                        // Retain the independent crossing distances. Cover
                        // endpoints alone cannot resolve entry/exit named ranges.
                        if matches!(
                            forward,
                            StickinessRange::DuringEntry | StickinessRange::Covering
                        ) {
                            entry_crossing_distance += forward_adjust;
                        }
                        if matches!(
                            backward,
                            StickinessRange::DuringEntry | StickinessRange::Covering
                        ) {
                            entry_crossing_distance -= backward_adjust;
                        }
                        if matches!(
                            forward,
                            StickinessRange::DuringExit | StickinessRange::Covering
                        ) {
                            exit_crossing_distance += forward_adjust;
                        }
                        if matches!(
                            backward,
                            StickinessRange::DuringExit | StickinessRange::Covering
                        ) {
                            exit_crossing_distance -= backward_adjust;
                        }
                        if forward != StickinessRange::AfterExit {
                            end += forward_adjust;
                        }
                        if backward == StickinessRange::AfterExit {
                            end += backward_adjust;
                        }
                        break;
                    }
                }
                sticky_node = self.document.node(sticky_node).parent;
            }
        }
        if end <= start {
            return None;
        }
        let cover = TimelineRangeBounds { start, end };
        // ToFractionalOffset bypasses named conversion when ViewOffsets==(0,0),
        // including an ordinary Scroll timeline and a zero-size View subject.
        let named_ranges = if entry_crossing_distance == 0.0 && exit_crossing_distance == 0.0 {
            NamedTimelineRanges {
                cover,
                contain: cover,
                entry: cover,
                entry_crossing: cover,
                exit: cover,
                exit_crossing: cover,
                scroll: cover,
            }
        } else {
            let subject_start_view_start = end - exit_crossing_distance;
            let subject_end_view_end = start + entry_crossing_distance;
            let contain_start = subject_start_view_start.min(subject_end_view_end);
            let contain_end = subject_start_view_start.max(subject_end_view_end);
            NamedTimelineRanges {
                cover,
                contain: TimelineRangeBounds {
                    start: contain_start,
                    end: contain_end,
                },
                entry: TimelineRangeBounds {
                    start,
                    end: contain_start,
                },
                entry_crossing: TimelineRangeBounds {
                    start,
                    end: subject_end_view_end,
                },
                exit: TimelineRangeBounds {
                    start: contain_end,
                    end,
                },
                exit_crossing: TimelineRangeBounds {
                    start: subject_start_view_start,
                    end,
                },
                scroll: TimelineRangeBounds {
                    start: 0.0,
                    end: maximum,
                },
            }
        };
        let legacy_oriented = legacy_range.is_some();
        let (range, attachment_bounds, attachment_start_fraction, attachment_end_fraction) =
            if let Some(range) = legacy_range {
                (
                    TimelineRangeBounds {
                        start: range.start,
                        end: range.end,
                    },
                    TimelineRangeBounds {
                        start: start + range.start,
                        end: start + range.end,
                    },
                    range.start / (end - start),
                    range.end / (end - start),
                )
            } else {
                let (range, reference) = browser_range?;
                let reference = self.resolve(reference).ok()?;
                let resolve = |endpoint: TimelineRangeOffset| {
                    let bounds = named_ranges.get(endpoint.name);
                    let value = self.resolve_timeline_length(reference, endpoint.offset);
                    let offset = minimum_length(&value, bounds.end - bounds.start);
                    let absolute = bounds.start + offset;
                    let relative = absolute - start;
                    // Retain this first division; a multiply/divide round trip through
                    // reflected CSS pixels must not change timing or phase flags.
                    (absolute, relative, relative / (end - start))
                };
                let (absolute_start, relative_start, start_fraction) =
                    range.start.map_or((start, 0.0, 0.0), resolve);
                let (absolute_end, relative_end, end_fraction) =
                    range.end.map_or((end, end - start, 1.0), resolve);
                (
                    TimelineRangeBounds {
                        start: relative_start,
                        end: relative_end,
                    },
                    TimelineRangeBounds {
                        start: absolute_start,
                        end: absolute_end,
                    },
                    start_fraction,
                    end_fraction,
                )
            };
        let duration_seconds =
            ((end - start) * TIMELINE_MICROSECONDS_PER_CSS_PIXEL / 1000.0) / 1000.0;
        let attachment_start_seconds = attachment_start_fraction * duration_seconds;
        let attachment_end_seconds = attachment_end_fraction * duration_seconds;
        // ComputeTimelineState converts cover-relative time to integer
        // microseconds *before* animation attachment/start alignment. Rust's
        // float-to-i64 conversion has the same finite truncation/saturation.
        let timeline_seconds = cover_clock_seconds(scroll_offset, start);
        Some(ProgressTimelineSnapshot {
            source,
            axis,
            scroll_offset,
            cover_start: start,
            cover_end: end,
            range,
            legacy_oriented,
            attachment_start_fraction,
            attachment_end_fraction,
            attachment_bounds,
            named_ranges,
            entry_crossing_distance,
            exit_crossing_distance,
            cover_time_seconds: timeline_seconds,
            attachment_start_seconds,
            attachment_end_seconds,
            scroll_maximum: maximum,
            duration_seconds,
        })
    }
}

#[cfg(test)]
mod clock_tests {
    use super::*;

    #[test]
    fn cover_clock_truncates_integer_microseconds_before_attachment_alignment() {
        assert_eq!(cover_clock_seconds(1.0 / 32.0, 0.0), 0.0);
        assert_eq!(cover_clock_seconds(3.0 / 32.0, 0.0), 1.0 / 1_000_000.0);
        assert_eq!(cover_clock_seconds(-3.0 / 32.0, 0.0), -1.0 / 1_000_000.0);
        let duration_seconds = ((100.0 * 16.0 / 1000.0) / 1000.0) as f64;
        let attachment_seconds = (1.0 / 32.0 / 100.0) * duration_seconds;
        let local = cover_clock_seconds(0.0, 0.0) - attachment_seconds;
        assert_eq!(local * 1000.0, -0.0005000000000000001);
        assert_ne!(local, cover_clock_seconds(0.0, 1.0 / 32.0));
    }
}
