//! Retained, untransformed scroll/view timeline geometry.
//! Pinned Blink 147.0.7727.50 uses 16 microseconds per CSS pixel.
use crate::{AnimationTimeline, Engine, NodeHandle};
use openui_dom::NodeId;
use openui_layout::{Fragment, ScrollArea};
use openui_style::{AnimationOptions, IterationCount, Overflow, TimelineAxis, TimelineRange};
use std::collections::HashMap;

const TIMELINE_MICROSECONDS_PER_CSS_PIXEL: f64 = 16.0;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProgressTimelineSnapshot {
    pub source: NodeHandle,
    pub axis: usize,
    pub scroll_offset: f64,
    pub cover_start: f64,
    pub cover_end: f64,
    pub range: TimelineRange,
    pub time_ms: f64,
    pub duration_ms: f64,
    pub range_duration_ms: f64,
    pub scroll_maximum: f64,
    pub duration_seconds: f64,
}

impl ProgressTimelineSnapshot {
    pub fn reflection(self, local_time_ms: Option<f64>) -> AnimationTimelineSnapshot {
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
            duration_ms: Some(self.duration_ms),
            current_time_ms: local_time_ms,
            current_time_percentage: local_time_ms.map(|time| time / self.duration_ms * 100.0),
        }
    }

    pub fn time_from_percentage(self, percentage: f64) -> f64 {
        (percentage / 100.0 * self.duration_seconds) * 1000.0
    }

    pub fn boundary_alignment(self, timing: &AnimationOptions, auto_align: bool) -> (bool, bool) {
        if !auto_align {
            return (false, false);
        }
        // animation.cc:2979-3036. This is timeline phase policy, independent
        // of the renderer's strict pixel comparison gate.
        let size = self.cover_end - self.cover_start;
        let mut start = self.range.start / size;
        let end = self.range.end / size;
        if self.duration_seconds > 0.0 {
            start += (timing.delay_ms / 1000.0) / self.duration_seconds;
        }
        let start_offset = start * self.cover_end + (1.0 - start) * self.cover_start;
        let end_offset = end * self.cover_end + (1.0 - end) * self.cover_start;
        (
            timing.playback_rate < 0.0 && start_offset <= 0.5,
            timing.playback_rate > 0.0
                && timing.playback_rate * end_offset >= self.scroll_maximum - 0.5,
        )
    }

    pub fn normalize(self, specified: &AnimationOptions) -> AnimationOptions {
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
            let size = self.cover_end - self.cover_start;
            let mut interval = 1.0;
            interval -= self.range.start / size;
            interval -= 1.0 - self.range.end / size;
            // The native API also preserves its pre-existing reversed ranges.
            if self.range.end < self.range.start {
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
        timing
    }
}

#[derive(Clone, Copy)]
struct LayoutBox {
    origin: [f64; 2],
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
            let next = LayoutBox {
                origin,
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
            if let Some(slice) = fragment.decoration_slice {
                let horizontal = fragment
                    .fragmentation_writing_direction
                    .map_or(true, |direction| direction.is_horizontal());
                let physical_block = if horizontal { 1 } else { 0 };
                let entry = output.get_mut(&fragment.node_id).expect("inserted box");
                entry.rect[physical_block + 2] =
                    entry.rect[physical_block + 2].max(slice.source_block_size.to_f64());
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

fn length(length: &openui_geometry::Length, extent: f64) -> f64 {
    if length.is_fixed() {
        length.value() as f64
    } else if length.is_percent() {
        extent * length.value() as f64 / 100.0
    } else if length.is_calculated() {
        length.calc_offset() as f64 + extent * length.value() as f64 / 100.0
    } else {
        0.0
    }
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
    pub(crate) fn progress_timeline_snapshot(
        &self,
        timeline: AnimationTimeline,
    ) -> Option<ProgressTimelineSnapshot> {
        let root = self.latest_fragment.as_ref()?;
        let geometry = boxes(root);
        let (source_node, subject, axis, range) = match timeline {
            AnimationTimeline::Document => return None,
            AnimationTimeline::Scroll {
                source,
                axis,
                range,
            } => {
                let source = self.resolve(source).ok()?;
                if !self.node_is_connected(source) {
                    return None;
                }
                (source, None, axis, range)
            }
            AnimationTimeline::View {
                subject,
                axis,
                range,
            } => {
                let subject = self.resolve(subject).ok()?;
                if !self.node_is_connected(subject) || !geometry.contains_key(&subject) {
                    return None;
                }
                let mut ancestor = self.document.node(subject).parent;
                let mut source = None;
                while !ancestor.is_none() {
                    if geometry
                        .get(&ancestor)
                        .and_then(|box_| box_.area)
                        .is_some_and(scroll_container)
                        || ancestor == self.document.root()
                    {
                        source = Some(ancestor);
                        break;
                    }
                    ancestor = self.document.node(ancestor).parent;
                }
                (source?, Some(subject), axis, range)
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
        let style = &self.document.node(source_node).style;
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
            let position = target.rect[axis] - source_box.origin[axis] - client_offset;
            let size = target.rect[axis + 2];
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
            start = position - viewport + length(end_inset, viewport);
            end = position + size - length(start_inset, viewport);
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
        let polarity = if range.end > range.start { 1.0 } else { -1.0 };
        let duration_seconds =
            ((end - start) * TIMELINE_MICROSECONDS_PER_CSS_PIXEL / 1000.0) / 1000.0;
        let attachment_start_seconds = (range.start / (end - start)) * duration_seconds;
        let attachment_end_seconds = (range.end / (end - start)) * duration_seconds;
        // ComputeTimelineState converts cover-relative time to integer
        // microseconds *before* animation attachment/start alignment. Rust's
        // float-to-i64 conversion has the same finite truncation/saturation.
        let current_microseconds =
            ((scroll_offset - start) * TIMELINE_MICROSECONDS_PER_CSS_PIXEL) as i64;
        let timeline_seconds = current_microseconds as f64 / 1_000_000.0;
        Some(ProgressTimelineSnapshot {
            source,
            axis,
            scroll_offset,
            cover_start: start,
            cover_end: end,
            range,
            time_ms: ((timeline_seconds - attachment_start_seconds) * polarity) * 1000.0,
            duration_ms: duration_seconds * 1000.0,
            range_duration_ms: (attachment_end_seconds - attachment_start_seconds).abs() * 1000.0,
            scroll_maximum: maximum,
            duration_seconds,
        })
    }
}
