use crate::{Engine, EngineError, NodeHandle};
use openui_style::{
    apply_to_computed, value_from_computed, AnimationAttachmentRange, AnimationOptions,
    AnimationPhase, AnimationTimingSeconds, CompositedStyleValue, Keyframes, PlayState,
    PropertyKeyframes, StyleProperty, StyleValue, TimelineAxis, TimelineRange, ViewTimelineInset,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct AnimationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ScrollAnimationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationEventKind {
    Start,
    Iteration,
    End,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationEvent {
    pub animation: AnimationId,
    pub target: NodeHandle,
    pub property: StyleProperty,
    pub kind: AnimationEventKind,
    pub elapsed_time_ms: f64,
    pub iteration: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AnimationTimeline {
    #[default]
    Document,
    Scroll {
        source: NodeHandle,
        axis: TimelineAxis,
        range: TimelineRange,
    },
    View {
        subject: NodeHandle,
        axis: TimelineAxis,
        range: TimelineRange,
    },
}

/// An additive timeline source independent of attachment range semantics.
/// This preserves the existing `AnimationTimeline` variants and matches.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AnimationTimelineSource {
    #[default]
    Document,
    Scroll {
        source: NodeHandle,
        axis: TimelineAxis,
    },
    View {
        subject: NodeHandle,
        axis: TimelineAxis,
    },
    ViewWithInset {
        subject: NodeHandle,
        axis: TimelineAxis,
        inset: ViewTimelineInset,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RetainedAnimationTimeline {
    Legacy(AnimationTimeline),
    Attachment {
        source: AnimationTimelineSource,
        range: AnimationAttachmentRange,
        offset_reference: NodeHandle,
    },
}

impl RetainedAnimationTimeline {
    pub(crate) fn source(self) -> AnimationTimelineSource {
        match self {
            Self::Legacy(AnimationTimeline::Document) => AnimationTimelineSource::Document,
            Self::Legacy(AnimationTimeline::Scroll { source, axis, .. }) => {
                AnimationTimelineSource::Scroll { source, axis }
            }
            Self::Legacy(AnimationTimeline::View { subject, axis, .. }) => {
                AnimationTimelineSource::View { subject, axis }
            }
            Self::Attachment { source, .. } => source,
        }
    }

    pub(crate) fn is_document(self) -> bool {
        matches!(self.source(), AnimationTimelineSource::Document)
    }

    pub(crate) fn legacy_range(self) -> Option<TimelineRange> {
        match self {
            Self::Legacy(
                AnimationTimeline::Scroll { range, .. } | AnimationTimeline::View { range, .. },
            ) => Some(range),
            _ => None,
        }
    }

    pub(crate) fn attachment(self) -> Option<(AnimationAttachmentRange, NodeHandle)> {
        match self {
            Self::Attachment {
                range,
                offset_reference,
                ..
            } => Some((range, offset_reference)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationState {
    pub id: AnimationId,
    pub target: NodeHandle,
    pub property: StyleProperty,
    pub play_state: PlayState,
    pub current_time_ms: f64,
    pub iteration: u64,
    pub phase: AnimationPhase,
    pub finished: bool,
}

#[derive(Clone)]
pub(crate) struct AnimationInstance {
    pub target: NodeHandle,
    pub keyframes: PropertyKeyframes,
    pub options: AnimationOptions,
    pub timing_options: AnimationOptions,
    pub timing_seconds: AnimationTimingSeconds,
    pub timeline: RetainedAnimationTimeline,
    pub timeline_snapshot: Option<crate::animation_timeline::ProgressTimelineSnapshot>,
    pub auto_align_start_time: bool,
    pub pending_progress_seek: Option<f64>,
    pub start_time_seconds: f64,
    pub offset_seconds: f64,
    pub hold_time_seconds: Option<f64>,
    pub last_sample: Option<EffectSample>,
    pub last_phase: Option<AnimationPhase>,
    pub last_iteration: u64,
    pub last_current_time_seconds: f64,
    pub ended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EffectSample {
    progress: f64,
    before: bool,
}

impl AnimationInstance {
    pub(crate) fn sample_local_time_with_direction(
        &self,
        local_time_seconds: f64,
    ) -> (openui_style::AnimationSample, bool) {
        let (start, end) = self.timeline_snapshot.map_or((false, false), |timeline| {
            timeline.boundary_alignment(
                &self.timing_options,
                self.timing_seconds,
                self.auto_align_start_time,
            )
        });
        self.timing_options
            .sample_local_time_seconds_with_direction_and_boundaries(
                local_time_seconds,
                self.timing_seconds,
                start,
                end,
            )
    }

    // Document running time uses the authoritative start-time expression.
    // Progress timelines retain their separate local-offset adapter; held
    // time is already local and remains independent of the playback rate.
    fn current_time(&self, timeline_time_seconds: f64) -> f64 {
        self.hold_time_seconds.unwrap_or_else(|| {
            let running_time =
                (timeline_time_seconds - self.start_time_seconds) * self.options.playback_rate;
            if self.timeline.is_document() {
                running_time
            } else {
                self.offset_seconds + running_time
            }
        })
    }

    fn reached_endpoint(&self, local_time_seconds: f64) -> bool {
        self.options.play_state == PlayState::Running
            && ((self.options.playback_rate > 0.0
                && local_time_seconds >= self.timing_seconds.effect_end(self.options.iterations))
                || (self.options.playback_rate < 0.0 && local_time_seconds <= 0.0))
    }

    fn reanchor_progress(&mut self, timeline_time_seconds: f64, local_time_seconds: f64) {
        self.offset_seconds = 0.0;
        if self.options.play_state == PlayState::Paused
            || self.options.playback_rate == 0.0
            || self.timeline_snapshot.is_none()
        {
            self.hold_time_seconds = Some(local_time_seconds);
            self.start_time_seconds = timeline_time_seconds;
        } else {
            self.hold_time_seconds = None;
            self.start_time_seconds =
                timeline_time_seconds - local_time_seconds / self.options.playback_rate;
        }
    }

    fn reanchor(&mut self, timeline_time_seconds: f64, local_time_seconds: f64) {
        // Pinned .50 CalculateStartTime and CommitPendingPlay keep the
        // authoritative start time. An additive local offset is algebraically
        // equivalent, but changes the required IEEE operation order.
        self.offset_seconds = 0.0;
        self.hold_time_seconds = (self.options.play_state == PlayState::Paused
            || self.options.playback_rate == 0.0
            || (self.timeline.is_document() && self.reached_endpoint(local_time_seconds)))
        .then_some(local_time_seconds);
        self.start_time_seconds = if self.hold_time_seconds.is_some() {
            timeline_time_seconds
        } else {
            timeline_time_seconds - local_time_seconds / self.options.playback_rate
        };
    }
}

#[derive(Clone)]
pub(crate) struct ScrollAnimationInstance {
    pub target: NodeHandle,
    from: (f64, f64),
    to: (f64, f64),
    start_time_ms: f64,
    duration_ms: f64,
    easing: openui_style::Easing,
}

impl Engine {
    pub fn smooth_scroll_to(
        &mut self,
        target: NodeHandle,
        x: f64,
        y: f64,
        duration_ms: f64,
        easing: openui_style::Easing,
    ) -> Result<ScrollAnimationId, EngineError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::InvalidInput(
                "smooth-scroll coordinates must be finite",
            ));
        }
        validate_scroll_animation_timing(duration_ms, &easing)?;
        let node = self.resolve(target)?;
        let clamped = self.clamp_scroll_offset(target, x, y)?;
        let clamped = self.resolve_native_scroll_endpoint(target, clamped.0, clamped.1)?;
        let source = self.document.node(node);
        let from = (source.scroll_left as f64, source.scroll_top as f64);
        let to = (clamped.0 as f64, clamped.1 as f64);
        self.scroll_animations
            .retain(|_, animation| animation.target != target);
        let id = ScrollAnimationId(self.next_scroll_animation_id);
        self.next_scroll_animation_id = self.next_scroll_animation_id.wrapping_add(1).max(1);
        if self.reduced_motion || duration_ms == 0.0 {
            if from != to {
                let data = self.document.node_mut(node);
                data.scroll_left = to.0 as f32;
                data.scroll_top = to.1 as f32;
                self.invalidate_scroll(node);
            }
            return Ok(id);
        }
        self.scroll_animations.insert(
            id,
            ScrollAnimationInstance {
                target,
                from,
                to,
                start_time_ms: self.animation_time_ms,
                duration_ms,
                easing,
            },
        );
        Ok(id)
    }

    pub fn settle_scroll_snap(
        &mut self,
        target: NodeHandle,
        snap_points_x: &[f64],
        snap_points_y: &[f64],
        duration_ms: f64,
        easing: openui_style::Easing,
    ) -> Result<Option<ScrollAnimationId>, EngineError> {
        if snap_points_x
            .iter()
            .chain(snap_points_y)
            .any(|v| !v.is_finite())
        {
            return Err(EngineError::InvalidInput(
                "scroll snap points must be finite",
            ));
        }
        // Validate even when the nearest point already equals the current offset.
        // Invalid inputs must not become successful no-ops or replace a live scroll.
        validate_scroll_animation_timing(duration_ms, &easing)?;
        let (x, y) = self.scroll_offset(target)?;
        let nearest = |current: f64, points: &[f64]| {
            points
                .iter()
                .copied()
                .min_by(|left, right| (left - current).abs().total_cmp(&(right - current).abs()))
        };
        let target_x = nearest(x, snap_points_x).unwrap_or(x);
        let target_y = nearest(y, snap_points_y).unwrap_or(y);
        if (target_x, target_y) == (x, y) {
            return Ok(None);
        }
        self.smooth_scroll_to(target, target_x, target_y, duration_ms, easing)
            .map(Some)
    }

    pub fn cancel_smooth_scroll(
        &mut self,
        animation: ScrollAnimationId,
    ) -> Result<(), EngineError> {
        self.scroll_animations
            .remove(&animation)
            .map(|_| ())
            .ok_or(EngineError::InvalidInput("unknown smooth-scroll animation"))
    }

    pub fn animate(
        &mut self,
        target: NodeHandle,
        keyframes: PropertyKeyframes,
        options: AnimationOptions,
        timeline: AnimationTimeline,
    ) -> Result<AnimationId, EngineError> {
        self.start_animation(
            target,
            keyframes,
            options,
            RetainedAnimationTimeline::Legacy(timeline),
        )
    }

    /// Animate on a typed source with browser attachment/named-range semantics.
    /// The retained Engine owns geometry, controls, events and style sampling.
    pub fn animate_with_attachment_range(
        &mut self,
        target: NodeHandle,
        keyframes: PropertyKeyframes,
        options: AnimationOptions,
        source: AnimationTimelineSource,
        range: AnimationAttachmentRange,
    ) -> Result<AnimationId, EngineError> {
        self.start_animation(
            target,
            keyframes,
            options,
            RetainedAnimationTimeline::Attachment {
                source,
                range,
                offset_reference: target,
            },
        )
    }

    pub fn transition<T>(
        &mut self,
        target: NodeHandle,
        property: StyleProperty,
        to: T,
        options: AnimationOptions,
    ) -> Result<AnimationId, EngineError>
    where
        T: Into<StyleValue>,
    {
        let node = self.resolve(target)?;
        let to = to.into();
        if !openui_style::value_matches_property(property, &to) {
            return Err(EngineError::PropertyType { property });
        }
        options.validate().map_err(EngineError::from)?;
        initial_local_time(&options)?;
        let from = value_from_computed(&self.document.node(node).style, property);
        let keyframes =
            PropertyKeyframes::typed(property, Keyframes::from_values(from, to.clone()))
                .map_err(EngineError::from)?;
        // The transition's target is an ordinary source declaration. It
        // shares shorthand ownership/removal with setters, while its first
        // keyframe captures the current presentation before the source changes.
        let candidate = self.staged_authored_property(target, property, to)?;
        let previous = candidate.map(|candidate| {
            std::mem::replace(&mut self.slots[target.index as usize].authored, candidate)
        });
        match self.start_animation(
            target,
            keyframes,
            options,
            RetainedAnimationTimeline::Legacy(AnimationTimeline::Document),
        ) {
            Ok(id) => Ok(id),
            Err(error) => {
                if let Some(previous) = previous {
                    self.slots[target.index as usize].authored = previous;
                }
                Err(error)
            }
        }
    }

    fn start_animation(
        &mut self,
        target: NodeHandle,
        keyframes: PropertyKeyframes,
        options: AnimationOptions,
        timeline: RetainedAnimationTimeline,
    ) -> Result<AnimationId, EngineError> {
        let node = self.resolve(target)?;
        options.validate().map_err(EngineError::from)?;
        self.validate_timeline(timeline)?;
        if !timeline.is_document() {
            self.update()?;
        }
        let timeline_snapshot = self.progress_timeline_snapshot(timeline);
        let (timing_options, timing_seconds) = timeline_snapshot.map_or_else(
            || {
                (
                    options.clone(),
                    AnimationTimingSeconds::from_options(&options),
                )
            },
            |snapshot| snapshot.normalize(&options),
        );
        let initial_time = if timeline.is_document() {
            initial_local_time(&options)?
        } else {
            0.0
        };
        let timeline_now = self.timeline_time(timeline, &options)?;
        let start_time_seconds = if timeline.is_document() {
            if options.play_state == PlayState::Paused || options.playback_rate == 0.0 {
                timeline_now
            } else {
                timeline_now - initial_time / options.playback_rate
            }
        } else {
            timeline_snapshot.map_or(0.0, |snapshot| {
                snapshot.automatic_start_time_seconds(options.playback_rate)
            })
        };
        let held = options.play_state == PlayState::Paused
            || (timeline.is_document() && options.playback_rate == 0.0);
        let initial_hold = if timeline.is_document() {
            initial_time
        } else {
            (timeline_now - start_time_seconds) * options.playback_rate
        };
        let previous_base = self.slots[target.index as usize]
            .animation_base_style
            .clone();
        let previous_properties = self.slots[target.index as usize]
            .animation_properties
            .clone();
        if previous_base.is_none() {
            let base = self.unanimated_style(node, target.index)?;
            self.slots[target.index as usize].animation_base_style = Some(base);
        }
        self.slots[target.index as usize]
            .animation_properties
            .extend(
                keyframes
                    .effect_properties()
                    .iter()
                    .map(|property| *property as u16),
            );
        let id = AnimationId(self.next_animation_id);
        self.next_animation_id = self.next_animation_id.wrapping_add(1).max(1);
        self.animations.insert(
            id,
            AnimationInstance {
                target,
                keyframes,
                options,
                timing_options,
                timing_seconds,
                timeline,
                timeline_snapshot,
                auto_align_start_time: true,
                pending_progress_seek: None,
                start_time_seconds,
                offset_seconds: 0.0,
                hold_time_seconds: held.then_some(initial_hold),
                last_sample: None,
                last_phase: None,
                last_iteration: 0,
                last_current_time_seconds: initial_time,
                ended: false,
            },
        );
        if let Err(error) = self
            .sample_animations()
            .and_then(|()| self.refresh_inherited_styles(node))
        {
            // No callback runs while the Engine is borrowed. Failed source
            // resolution must not leave a returned-error effect or its events.
            self.animations.remove(&id);
            self.animation_events.retain(|event| event.animation != id);
            let slot = &mut self.slots[target.index as usize];
            slot.animation_base_style = previous_base;
            slot.animation_properties = previous_properties;
            return Err(error);
        }
        Ok(id)
    }

    pub fn pause_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        self.update()?;
        let snapshot = self.animation_instance(id)?.clone();
        if snapshot.options.play_state == PlayState::Paused {
            return Ok(());
        }
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let current = snapshot.current_time(now);
        let instance = self.animations.get_mut(&id).expect("live animation");
        instance.options.play_state = PlayState::Paused;
        instance.timing_options.play_state = PlayState::Paused;
        if instance.timeline.is_document() {
            instance.reanchor(now, current);
        } else {
            instance.auto_align_start_time = false;
            instance.reanchor_progress(now, current);
        }
        self.sample_animations()
    }

    pub fn play_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        self.update()?;
        let snapshot = self.animation_instance(id)?.clone();
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let mut current = snapshot.current_time(now);
        if !snapshot.timeline.is_document() {
            let instance = self.animations.get_mut(&id).expect("live animation");
            instance.options.play_state = PlayState::Running;
            instance.timing_options.play_state = PlayState::Running;
            instance.auto_align_start_time = true;
            // A newer play request supersedes an unresolved percentage seek.
            // Its time will come from the retained range's automatic alignment.
            instance.pending_progress_seek = None;
            instance.start_time_seconds = instance.timeline_snapshot.map_or(0.0, |snapshot| {
                snapshot.automatic_start_time_seconds(instance.options.playback_rate)
            });
            instance.offset_seconds = 0.0;
            instance.hold_time_seconds = None;
            return self.sample_animations();
        }
        if snapshot.timeline.is_document() {
            let end = snapshot
                .timing_seconds
                .effect_end(snapshot.options.iterations);
            let rate = snapshot.options.playback_rate;
            if rate > 0.0 && (current < 0.0 || current >= end) {
                current = 0.0;
            } else if rate < 0.0 && (current <= 0.0 || current > end) {
                if !end.is_finite() {
                    return Err(EngineError::InvalidInput(
                        "cannot start reverse playback at an infinite effect end",
                    ));
                }
                current = end;
            }
        }
        if snapshot.options.play_state == PlayState::Running
            && !snapshot.ended
            && current == snapshot.current_time(now)
        {
            return Ok(());
        }
        let instance = self.animations.get_mut(&id).expect("live animation");
        instance.options.play_state = PlayState::Running;
        instance.timing_options.play_state = PlayState::Running;
        instance.reanchor(now, current);
        self.sample_animations()
    }

    pub fn seek_animation(
        &mut self,
        id: AnimationId,
        current_time_ms: f64,
    ) -> Result<(), EngineError> {
        if !current_time_ms.is_finite() {
            return Err(EngineError::InvalidInput(
                "animation seek time must be finite",
            ));
        }
        self.seek_animation_seconds(id, current_time_ms / 1000.0)
    }

    fn seek_animation_seconds(
        &mut self,
        id: AnimationId,
        current_time_seconds: f64,
    ) -> Result<(), EngineError> {
        self.update()?;
        let snapshot = self.animation_instance(id)?.clone();
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let instance = self.animations.get_mut(&id).expect("live animation");
        // Explicit native local time replaces any earlier unresolved percentage.
        instance.pending_progress_seek = None;
        if instance.timeline.is_document() {
            instance.reanchor(now, current_time_seconds);
        } else {
            instance.auto_align_start_time = false;
            instance.reanchor_progress(now, current_time_seconds);
        }
        self.sample_animations()
    }

    pub fn set_animation_playback_rate(
        &mut self,
        id: AnimationId,
        playback_rate: f64,
    ) -> Result<(), EngineError> {
        if !playback_rate.is_finite() {
            return Err(EngineError::InvalidInput(
                "animation playback rate must be finite",
            ));
        }
        self.update()?;
        let snapshot = self.animation_instance(id)?.clone();
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let current = snapshot.current_time(now);
        let current = if snapshot.timeline.is_document() {
            // Pinned .50 setPlaybackRate preserves through currentTime's
            // millisecond reflection and setCurrentTime's seconds conversion.
            (current * 1000.0) / 1000.0
        } else {
            current
        };
        let instance = self.animations.get_mut(&id).expect("live animation");
        instance.options.playback_rate = playback_rate;
        instance.timing_options.playback_rate = playback_rate;
        if instance.timeline.is_document() {
            instance.reanchor(now, current);
        } else if (snapshot.options.playback_rate < 0.0) != (playback_rate < 0.0) {
            instance.start_time_seconds = if instance.auto_align_start_time {
                instance.timeline_snapshot.map_or(0.0, |snapshot| {
                    snapshot.automatic_start_time_seconds(playback_rate)
                })
            } else {
                instance
                    .timing_seconds
                    .effect_end(instance.options.iterations)
                    - instance.start_time_seconds
            };
        }
        self.sample_animations()
    }

    pub fn finish_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        self.update()?;
        let snapshot = self.animation_instance(id)?.clone();
        let rate = snapshot.options.playback_rate;
        if rate == 0.0 {
            return Err(EngineError::InvalidInput(
                "cannot finish an animation with zero playback rate",
            ));
        }
        let endpoint = if rate < 0.0 {
            0.0
        } else {
            snapshot
                .timing_seconds
                .effect_end(snapshot.options.iterations)
        };
        if !endpoint.is_finite() {
            return Err(EngineError::InvalidInput(
                "cannot finish at an infinite effect end",
            ));
        }
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let instance = self.animations.get_mut(&id).expect("live animation");
        // Finishing a paused animation resolves its start time and finished state.
        // The validated endpoint replaces any unresolved percentage seek.
        instance.pending_progress_seek = None;
        instance.options.play_state = PlayState::Running;
        instance.timing_options.play_state = PlayState::Running;
        if instance.timeline.is_document() {
            instance.reanchor(now, endpoint);
        } else {
            instance.auto_align_start_time = false;
            instance.reanchor_progress(now, endpoint);
        }
        self.sample_animations()
    }

    pub fn cancel_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        let snapshot = self.animation_instance(id)?.clone();
        let elapsed_time_ms = self.cancel_elapsed_time(&snapshot);
        self.animations.remove(&id);
        let node = self.resolve(snapshot.target)?;
        if let Err(error) = self.refresh_inherited_styles(node) {
            self.animations.insert(id, snapshot);
            return Err(error);
        }
        if snapshot.last_phase.is_some() && snapshot.last_phase != Some(AnimationPhase::After) {
            self.animation_events.push(AnimationEvent {
                animation: id,
                target: snapshot.target,
                property: snapshot.keyframes.property(),
                kind: AnimationEventKind::Cancel,
                elapsed_time_ms,
                iteration: snapshot.last_iteration,
            });
        }
        Ok(())
    }

    /// An owned progress-timeline snapshot, resolved at a native layout barrier.
    /// Document animations use the ordinary millisecond state and return None.
    pub fn animation_timeline_snapshot(
        &mut self,
        id: AnimationId,
    ) -> Result<Option<crate::AnimationTimelineSnapshot>, EngineError> {
        self.animation_instance(id)?;
        self.update()?;
        let instance = self.animation_instance(id)?;
        if instance.timeline.is_document() {
            return Ok(None);
        }
        if let Some(snapshot) = instance.timeline_snapshot {
            return Ok(Some(snapshot.reflection(Some(
                instance.current_time(snapshot.clock_time_seconds()),
            ))));
        }
        // Browser endpoints cannot be resolved while geometry is inactive.
        // Its additive attachment snapshot carries explicit optional bounds.
        let range = instance.timeline.legacy_range().unwrap_or(TimelineRange {
            start: 0.0,
            end: 0.0,
        });
        Ok(Some(crate::AnimationTimelineSnapshot {
            source: None,
            active: false,
            physical_axis: None,
            scroll_offset: None,
            cover_start: None,
            cover_end: None,
            range_start: range.start,
            range_end: range.end,
            duration_ms: None,
            current_time_ms: None,
            current_time_percentage: None,
        }))
    }

    /// Owned attachment/range data. `None` denotes the legacy numeric route.
    pub fn animation_attachment_snapshot(
        &mut self,
        id: AnimationId,
    ) -> Result<Option<crate::AnimationAttachmentSnapshot>, EngineError> {
        self.animation_instance(id)?;
        self.update()?;
        let instance = self.animation_instance(id)?;
        Ok(instance
            .timeline
            .attachment()
            .map(|(range, _)| crate::AnimationAttachmentSnapshot {
                range,
                resolved: instance
                    .timeline_snapshot
                    .map(|snapshot| snapshot.attachment_reflection()),
            }))
    }

    /// Set browser-style endpoints on an existing animation, retaining its source
    /// and identity. Document timelines retain the descriptor without changing time.
    pub fn set_animation_attachment_range(
        &mut self,
        id: AnimationId,
        range: AnimationAttachmentRange,
    ) -> Result<(), EngineError> {
        range.validate().map_err(EngineError::from)?;
        self.animation_instance(id)?;
        self.update()?;
        let snapshot = self.animation_instance(id)?.clone();
        let changed = snapshot.timeline.attachment().map(|(previous, _)| previous) != Some(range);
        let instance = self.animations.get_mut(&id).expect("live animation");
        instance.timeline = RetainedAnimationTimeline::Attachment {
            source: snapshot.timeline.source(),
            range,
            offset_reference: snapshot.target,
        };
        instance.auto_align_start_time = true;
        if !instance.timeline.is_document() && instance.options.play_state == PlayState::Running {
            // Setting the range resumes automatic running alignment, including
            // an unchanged descriptor. Paused time requests remain held.
            instance.pending_progress_seek = None;
        }
        if changed
            && !instance.timeline.is_document()
            && instance.options.play_state == PlayState::Running
        {
            // Pinned OnRangeUpdate invokes nonmonotonic PlayInternal for running
            // or finished animations; paused effects preserve their hold time.
            instance.hold_time_seconds = None;
            instance.offset_seconds = 0.0;
        }
        self.sample_animations()
    }

    /// Set a scroll/view animation's current time as a percentage of its
    /// resolved timeline duration. Finite percentages outside 0..100 are valid.
    /// An inactive timeline retains the request until its range resolves.
    pub fn seek_animation_progress(
        &mut self,
        id: AnimationId,
        percentage: f64,
    ) -> Result<(), EngineError> {
        if !percentage.is_finite() {
            return Err(EngineError::InvalidInput(
                "animation progress must be finite",
            ));
        }
        let instance = self.animation_instance(id)?;
        if instance.timeline.is_document() {
            return Err(EngineError::InvalidInput(
                "percentage time requires a scroll or view timeline",
            ));
        }
        self.update()?;
        let snapshot = self.animation_instance(id)?.timeline_snapshot;
        if let Some(snapshot) = snapshot {
            self.seek_animation_seconds(id, snapshot.time_from_percentage(percentage))
        } else {
            let instance = self.animations.get_mut(&id).expect("live animation");
            instance.auto_align_start_time = false;
            instance.pending_progress_seek = Some(percentage);
            Ok(())
        }
    }

    pub fn animation_state(&self, id: AnimationId) -> Result<AnimationState, EngineError> {
        let instance = self.animation_instance(id)?;
        let timeline_now = self.timeline_time(instance.timeline, &instance.options)?;
        let current_time_seconds = instance.current_time(timeline_now);
        let effective = self.effective_elapsed(instance, current_time_seconds);
        let sample = instance.sample_local_time_with_direction(effective).0;
        Ok(AnimationState {
            id,
            target: instance.target,
            property: instance.keyframes.property(),
            play_state: instance.options.play_state,
            current_time_ms: current_time_seconds * 1000.0,
            iteration: sample.iteration,
            phase: sample.phase,
            finished: instance.ended,
        })
    }

    pub fn is_animating(&self) -> bool {
        !self.reduced_motion
            && (self.animations.values().any(|animation| {
                animation.timeline.is_document()
                    && animation.options.play_state == PlayState::Running
                    && animation.options.playback_rate != 0.0
                    && !animation.ended
            }) || !self.scroll_animations.is_empty())
    }

    pub fn active_animation_count(&self) -> usize {
        self.animations
            .values()
            .filter(|animation| !animation.ended)
            .count()
    }

    pub fn drain_animation_events(&mut self) -> Vec<AnimationEvent> {
        std::mem::take(&mut self.animation_events)
    }

    pub(crate) fn cancel_animations_for_handles(&mut self, handles: &[NodeHandle]) {
        self.scroll_animations
            .retain(|_, animation| !handles.contains(&animation.target));
        let canceled = self
            .animations
            .iter()
            .filter(|(_, animation)| handles.contains(&animation.target))
            .map(|(id, animation)| (*id, animation.clone(), self.cancel_elapsed_time(animation)))
            .collect::<Vec<_>>();
        for (id, instance, elapsed_time_ms) in canceled {
            self.animations.remove(&id);
            if instance.last_phase.is_some() && instance.last_phase != Some(AnimationPhase::After) {
                self.animation_events.push(AnimationEvent {
                    animation: id,
                    target: instance.target,
                    property: instance.keyframes.property(),
                    kind: AnimationEventKind::Cancel,
                    elapsed_time_ms,
                    iteration: instance.last_iteration,
                });
            }
        }
    }

    pub(crate) fn sample_animations(&mut self) -> Result<(), EngineError> {
        if self
            .animations
            .values()
            .any(|instance| !instance.timeline.is_document())
        {
            self.update_once()?;
        }
        self.sample_animations_at_snapshot(false)
    }

    pub(crate) fn sample_progress_animations_after_layout(&mut self) -> Result<(), EngineError> {
        self.sample_animations_at_snapshot(true)
    }

    fn sample_animations_at_snapshot(&mut self, progress_only: bool) -> Result<(), EngineError> {
        let ids = self
            .animations
            .iter()
            .filter_map(|(id, instance)| {
                (!progress_only || !instance.timeline.is_document()).then_some(*id)
            })
            .collect::<Vec<_>>();
        let mut changed = false;
        for id in ids {
            self.refresh_progress_timeline_snapshot(id);
            let Some(snapshot) = self.animations.get(&id).cloned() else {
                continue;
            };
            if self.resolve(snapshot.target).is_err() {
                self.animations.remove(&id);
                continue;
            }
            if !snapshot.timeline.is_document() && snapshot.timeline_snapshot.is_none() {
                if snapshot.last_sample.is_some() {
                    self.animations
                        .get_mut(&id)
                        .expect("live animation")
                        .last_sample = None;
                    changed = true;
                }
                if snapshot.last_phase.is_some()
                    && snapshot.last_phase != Some(AnimationPhase::After)
                {
                    self.animation_events.push(AnimationEvent {
                        animation: id,
                        target: snapshot.target,
                        property: snapshot.keyframes.property(),
                        kind: AnimationEventKind::Cancel,
                        elapsed_time_ms: self.cancel_elapsed_time(&snapshot),
                        iteration: snapshot.last_iteration,
                    });
                }
                if let Some(instance) = self.animations.get_mut(&id) {
                    instance.last_phase = None;
                    instance.last_iteration = 0;
                    instance.ended = true;
                }
                continue;
            }
            let timeline_now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
            let raw_time = snapshot.current_time(timeline_now);
            let current_time = if snapshot.hold_time_seconds.is_none()
                && snapshot.timeline.is_document()
                && snapshot.reached_endpoint(raw_time)
            {
                if snapshot.options.playback_rate < 0.0 {
                    0.0
                } else {
                    snapshot
                        .timing_seconds
                        .effect_end(snapshot.options.iterations)
                }
            } else {
                raw_time
            };
            let effective = self.effective_elapsed(&snapshot, current_time);
            let (sample, current_direction_is_reversed) =
                snapshot.sample_local_time_with_direction(effective);
            let finished = self.reduced_motion || snapshot.reached_endpoint(current_time);
            let sampled = sample
                .eased_progress(&snapshot.options.easing, current_direction_is_reversed)
                .map(|progress| EffectSample {
                    progress,
                    before: sample.phase == AnimationPhase::Before,
                });
            changed |= snapshot.last_sample != sampled;

            let phase_changed = snapshot.last_phase != Some(sample.phase);
            let was_before = snapshot.last_phase.is_none()
                || snapshot.last_phase == Some(AnimationPhase::Before);
            let was_after = snapshot.last_phase == Some(AnimationPhase::After);
            let interval_start = (-snapshot.timing_seconds.delay)
                .min(
                    snapshot
                        .timing_seconds
                        .active_duration(snapshot.options.iterations),
                )
                .max(0.0);
            let interval_end = (snapshot
                .timing_seconds
                .effect_end(snapshot.options.iterations)
                - snapshot.timing_seconds.delay)
                .min(
                    snapshot
                        .timing_seconds
                        .active_duration(snapshot.options.iterations),
                )
                .max(0.0);
            let mut events = Vec::new();
            if phase_changed
                && ((was_before && sample.phase != AnimationPhase::Before)
                    || (was_after && sample.phase != AnimationPhase::After))
            {
                events.push((
                    AnimationEventKind::Start,
                    if was_after {
                        interval_end
                    } else {
                        interval_start
                    },
                ));
            }
            if phase_changed
                && (sample.phase == AnimationPhase::After
                    || (sample.phase == AnimationPhase::Before && !was_before))
            {
                events.push((
                    AnimationEventKind::End,
                    if sample.phase == AnimationPhase::After {
                        interval_end
                    } else {
                        interval_start
                    },
                ));
            }
            if !phase_changed
                && sample.phase == AnimationPhase::Active
                && sample.iteration != snapshot.last_iteration
            {
                // Chromium emits one event for all skipped iterations, in either direction.
                let boundary = if snapshot.last_iteration > sample.iteration {
                    sample.iteration.saturating_add(1)
                } else {
                    sample.iteration
                };
                events.push((
                    AnimationEventKind::Iteration,
                    snapshot.timing_seconds.duration * boundary as f64,
                ));
            }
            for (kind, elapsed_time_seconds) in events {
                self.animation_events.push(AnimationEvent {
                    animation: id,
                    target: snapshot.target,
                    property: snapshot.keyframes.property(),
                    kind,
                    elapsed_time_ms: elapsed_time_seconds * 1000.0,
                    iteration: sample.iteration,
                });
            }

            if let Some(instance) = self.animations.get_mut(&id) {
                if snapshot.timeline.is_document() && finished {
                    if self.reduced_motion && effective.is_finite() {
                        instance.hold_time_seconds = Some(effective);
                    } else if snapshot.hold_time_seconds.is_none() {
                        instance.hold_time_seconds = Some(current_time);
                    }
                }
                instance.last_sample = sampled;
                instance.last_phase = Some(sample.phase);
                instance.last_iteration = sample.iteration;
                instance.last_current_time_seconds = current_time;
                instance.ended = finished;
            }
        }
        if changed {
            self.recompute_authored_styles()?;
        }
        Ok(())
    }

    pub(crate) fn sample_scroll_animations(&mut self) -> Result<(), EngineError> {
        let samples = self
            .scroll_animations
            .iter()
            .map(|(id, animation)| {
                let raw_progress =
                    (self.animation_time_ms - animation.start_time_ms) / animation.duration_ms;
                let progress = animation.easing.sample(raw_progress.clamp(0.0, 1.0));
                let x = animation.from.0 + (animation.to.0 - animation.from.0) * progress;
                let y = animation.from.1 + (animation.to.1 - animation.from.1) * progress;
                (*id, animation.target, x, y, raw_progress >= 1.0)
            })
            .collect::<Vec<_>>();
        for (id, target, x, y, finished) in samples {
            let node = self.resolve(target)?;
            let next = self.clamp_scroll_offset(target, x, y)?;
            let data = self.document.node_mut(node);
            if (data.scroll_left, data.scroll_top) != next {
                data.scroll_left = next.0;
                data.scroll_top = next.1;
                self.invalidate_scroll(node);
            }
            if finished {
                self.scroll_animations.remove(&id);
            }
        }
        Ok(())
    }

    fn effective_elapsed(&self, instance: &AnimationInstance, current_time: f64) -> f64 {
        if self.reduced_motion {
            if instance.options.playback_rate < 0.0 {
                0.0
            } else if instance
                .timing_seconds
                .effect_end(instance.options.iterations)
                .is_finite()
            {
                instance
                    .timing_seconds
                    .effect_end(instance.options.iterations)
            } else {
                instance.timing_seconds.delay + instance.timing_seconds.duration
            }
        } else {
            current_time
        }
    }

    fn cancel_elapsed_time(&self, instance: &AnimationInstance) -> f64 {
        let current = if !instance.timeline.is_document() && instance.timeline_snapshot.is_none() {
            instance
                .hold_time_seconds
                .unwrap_or(instance.last_current_time_seconds)
        } else {
            self.timeline_time(instance.timeline, &instance.options)
                .map(|time| instance.current_time(time))
                .unwrap_or(instance.last_current_time_seconds)
        };
        (current - instance.timing_seconds.delay)
            .min(
                instance
                    .timing_seconds
                    .active_duration(instance.options.iterations),
            )
            .max(0.0)
            * 1000.0
    }

    /// Fold active effects in creation order against the current unanimated
    /// computed environment. Values remain unclamped until the entire property
    /// stack is published, and no application callbacks occur in this method.
    pub(crate) fn animation_values_for_node(
        &self,
        index: u32,
        style: &openui_style::ComputedStyle,
        parent: &openui_style::ComputedStyle,
        root_font: f32,
        viewport: (f32, f32),
        font_properties: bool,
    ) -> Vec<(StyleProperty, StyleValue)> {
        let mut values = std::collections::BTreeMap::new();
        for (id, instance) in &self.animations {
            if instance.target != self.handle_for_slot(index) {
                continue;
            }
            let Some(sample) = instance.last_sample else {
                continue;
            };
            for &property in instance.keyframes.effect_properties() {
                let font_property = property.metadata().css_name.starts_with("font")
                    || property == StyleProperty::LineHeight;
                if font_property != font_properties {
                    continue;
                }
                let underlying = values
                    .get(&(property as u16))
                    .map(|(_, value): &(AnimationId, CompositedStyleValue)| value.clone())
                    .unwrap_or_else(|| {
                        CompositedStyleValue::from_style_value_for_property(
                            property,
                            value_from_computed(style, property),
                        )
                    });
                let value = instance
                    .keyframes
                    .sample_component_resolved_composited_value_with_before(
                        property,
                        sample.progress,
                        &openui_style::Easing::Linear,
                        sample.before,
                        &underlying,
                        instance.options.composite,
                        |value| {
                            // Own color currentcolor is an inherited numeric endpoint;
                            // dependent keywords remain owned working fractions.
                            let value = value.capture_own_current_color(property, parent.color);
                            let metrics = value.depends_on_font_metrics().then(|| {
                                let mut metrics = self.native_font_length_metrics(
                                    if property == StyleProperty::FontSize {
                                        parent
                                    } else {
                                        style
                                    },
                                );
                                if property == StyleProperty::LineHeight {
                                    metrics.lh = self.native_font_length_metrics(parent).lh;
                                }
                                metrics
                            });
                            Self::resolve_native_lengths(
                                property,
                                &value,
                                if property == StyleProperty::FontSize {
                                    parent.font_size
                                } else {
                                    style.font_size
                                },
                                if self.slots[index as usize].node == Some(self.document.root())
                                    && property != StyleProperty::FontSize
                                {
                                    style.font_size
                                } else {
                                    root_font
                                },
                                viewport,
                                metrics,
                            )
                        },
                    )
                    .expect("owned typed animation component");
                values.insert(property as u16, (*id, value));
            }
        }
        let mut ordered = values
            .into_iter()
            .map(|(property, (id, value))| {
                (
                    id,
                    StyleProperty::from_u16(property).expect("animated property"),
                    value,
                )
            })
            .collect::<Vec<_>>();
        // Color dependencies are resolved from the final ordered effect stacks,
        // independently of animation creation order. Text-decoration currentcolor
        // uses final text fill; the other color properties use final own color.
        ordered.sort_by_key(|(id, property, _)| {
            (
                match property {
                    StyleProperty::Color => 0,
                    StyleProperty::WebkitTextFillColor => 1,
                    _ => 2,
                },
                *id,
            )
        });
        let mut color_style = style.clone();
        ordered
            .into_iter()
            .map(|(_, property, value)| {
                let context = if property == StyleProperty::Color {
                    parent.color
                } else if property == StyleProperty::TextDecorationColor {
                    color_style.effective_text_fill_color()
                } else {
                    color_style.color
                };
                let value = value.into_style_value_with_color_context(property, context);
                if matches!(
                    property,
                    StyleProperty::Color | StyleProperty::WebkitTextFillColor
                ) {
                    apply_to_computed(&mut color_style, property, &value, viewport)
                        .expect("owned typed color effect");
                }
                (property, value)
            })
            .collect()
    }

    fn refresh_progress_timeline_snapshot(&mut self, id: AnimationId) {
        let Some(previous) = self.animations.get(&id).cloned() else {
            return;
        };
        if previous.timeline.is_document() {
            return;
        }
        let next = self.progress_timeline_snapshot(previous.timeline);
        let instance = self.animations.get_mut(&id).expect("live animation");
        if let Some(next) = next {
            if let Some(old) = previous.timeline_snapshot {
                let duration_changed = old.duration_seconds != next.duration_seconds;
                if duration_changed {
                    // OnValidateSnapshot explicitly divides reflected milliseconds;
                    // preserve that operation order only for resize rescaling.
                    if let Some(held) = instance.hold_time_seconds {
                        let progress = (held * 1000.0) / (old.duration_seconds * 1000.0);
                        instance.hold_time_seconds = Some(progress * next.duration_seconds);
                    } else if !instance.auto_align_start_time
                        && instance.options.playback_rate != 0.0
                    {
                        // Blink replaces the timeline snapshot before this
                        // callback: UnlimitedCurrentTime uses its NEW clock with
                        // the PREVIOUS animation anchor and old duration.
                        let progress = (previous.current_time(next.clock_time_seconds()) * 1000.0)
                            / (old.duration_seconds * 1000.0);
                        let current = progress * next.duration_seconds;
                        instance.start_time_seconds =
                            next.clock_time_seconds() - current / instance.options.playback_rate;
                    }
                }
            }
            if instance.auto_align_start_time {
                instance.start_time_seconds =
                    next.automatic_start_time_seconds(instance.options.playback_rate);
                instance.offset_seconds = 0.0;
            }
            (instance.timing_options, instance.timing_seconds) = next.normalize(&instance.options);
            instance.timeline_snapshot = Some(next);
            if let Some(percentage) = instance.pending_progress_seek.take() {
                instance.reanchor_progress(
                    next.clock_time_seconds(),
                    next.time_from_percentage(percentage),
                );
            }
        }
        instance.timeline_snapshot = next;
    }

    fn animation_instance(&self, id: AnimationId) -> Result<&AnimationInstance, EngineError> {
        self.animations
            .get(&id)
            .ok_or(EngineError::InvalidInput("unknown animation"))
    }

    fn validate_timeline(&self, timeline: RetainedAnimationTimeline) -> Result<(), EngineError> {
        match timeline.source() {
            AnimationTimelineSource::Document => {}
            AnimationTimelineSource::Scroll { source, .. } => {
                self.resolve(source)?;
            }
            AnimationTimelineSource::View { subject, .. }
            | AnimationTimelineSource::ViewWithInset { subject, .. } => {
                self.resolve(subject)?;
            }
        }
        if let AnimationTimelineSource::ViewWithInset { inset, .. } = timeline.source() {
            inset.validate().map_err(EngineError::from)?;
        }
        if let Some(range) = timeline.legacy_range() {
            range.validate().map_err(EngineError::from)?;
        }
        if let Some((range, reference)) = timeline.attachment() {
            self.resolve(reference)?;
            range.validate().map_err(EngineError::from)?;
        }
        Ok(())
    }

    fn timeline_time(
        &self,
        timeline: RetainedAnimationTimeline,
        options: &AnimationOptions,
    ) -> Result<f64, EngineError> {
        let _ = options;
        Ok(if timeline.is_document() {
            self.animation_time_ms / 1000.0
        } else {
            self.progress_timeline_snapshot(timeline)
                .map_or(0.0, |snapshot| snapshot.clock_time_seconds())
        })
    }
}

fn initial_local_time(options: &AnimationOptions) -> Result<f64, EngineError> {
    if options.playback_rate < 0.0 {
        let end = AnimationTimingSeconds::from_options(options).effect_end(options.iterations);
        if !end.is_finite() {
            return Err(EngineError::InvalidInput(
                "cannot start reverse playback at an infinite effect end",
            ));
        }
        Ok(end)
    } else {
        Ok(0.0)
    }
}

fn validate_scroll_animation_timing(
    duration_ms: f64,
    easing: &openui_style::Easing,
) -> Result<(), EngineError> {
    if !duration_ms.is_finite() || duration_ms < 0.0 {
        return Err(EngineError::InvalidInput(
            "smooth-scroll duration must be finite and non-negative",
        ));
    }
    easing.validate().map_err(EngineError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewportMetrics;
    use openui_dom::ElementTag;
    use openui_style::{Display, FillMode, Keyframe, LengthValue, Overflow};

    fn opacity_frames() -> PropertyKeyframes {
        PropertyKeyframes::typed(
            StyleProperty::Opacity,
            Keyframes::new(vec![
                Keyframe::new(0.0, 0.0_f32),
                Keyframe::new(1.0, 1.0_f32),
            ])
            .unwrap(),
        )
        .unwrap()
    }

    fn options() -> AnimationOptions {
        AnimationOptions {
            duration_ms: 100.0,
            fill: FillMode::Both,
            ..AnimationOptions::default()
        }
    }

    #[test]
    fn manual_clock_samples_and_emits_lifecycle_events() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        let id = engine
            .animate(
                node,
                opacity_frames(),
                options(),
                AnimationTimeline::Document,
            )
            .unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.0);
        engine.set_animation_time(50.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.5);
        engine.set_animation_time(100.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 1.0);
        let events = engine.drain_animation_events();
        assert_eq!(events[0].kind, AnimationEventKind::Start);
        assert!(events
            .iter()
            .any(|event| event.kind == AnimationEventKind::End));
        assert!(engine.animation_state(id).unwrap().finished);
        assert!(!engine.is_animating());
    }

    #[test]
    fn pause_seek_reverse_and_cancel_are_deterministic() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Opacity, StyleValue::Number(0.75))
            .unwrap();
        let id = engine
            .animate(
                node,
                opacity_frames(),
                options(),
                AnimationTimeline::Document,
            )
            .unwrap();
        engine.set_animation_time(25.0).unwrap();
        engine.pause_animation(id).unwrap();
        engine.set_animation_time(75.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.25);
        engine.seek_animation(id, 80.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.8);
        engine.set_animation_playback_rate(id, -1.0).unwrap();
        engine.play_animation(id).unwrap();
        engine.set_animation_time(85.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.7);
        // Pinned double-seconds start/current arithmetic reflects one ULP below
        // 70ms; wallclock TimeDelta inputs do not quantize animation local time.
        assert_eq!(
            engine
                .animation_state(id)
                .unwrap()
                .current_time_ms
                .to_bits(),
            0x4051_7fff_ffff_ffff
        );
        engine.cancel_animation(id).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 0.75);
        assert_eq!(
            engine.drain_animation_events().last().unwrap().kind,
            AnimationEventKind::Cancel
        );
    }

    #[test]
    fn transition_keeps_its_authored_target_after_completion() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Opacity, StyleValue::Number(0.2))
            .unwrap();
        engine
            .transition(node, StyleProperty::Opacity, 0.8_f32, options())
            .unwrap();
        engine.set_animation_time(100.0).unwrap();
        assert!((engine.computed_style(node).unwrap().opacity - 0.8).abs() < 1e-6);
    }

    #[test]
    fn reduced_motion_finishes_finite_animations() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine.set_prefers_reduced_motion(true);
        engine
            .animate(
                node,
                opacity_frames(),
                options(),
                AnimationTimeline::Document,
            )
            .unwrap();
        assert_eq!(engine.computed_style(node).unwrap().opacity, 1.0);
        assert!(!engine.is_animating());
    }

    #[test]
    fn smooth_scroll_and_snap_use_the_manual_clock() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        let content = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(node, content).unwrap();
        for (target, property, value) in [
            (node, StyleProperty::Display, Display::Block.into()),
            (node, StyleProperty::Width, LengthValue::px(100.0).into()),
            (node, StyleProperty::Height, LengthValue::px(80.0).into()),
            (node, StyleProperty::Overflow, Overflow::Hidden.into()),
            (content, StyleProperty::Display, Display::Block.into()),
            (content, StyleProperty::Width, LengthValue::px(300.0).into()),
            (
                content,
                StyleProperty::Height,
                LengthValue::px(240.0).into(),
            ),
        ] {
            engine.set_property(target, property, value).unwrap();
        }
        engine
            .smooth_scroll_to(node, 100.0, 40.0, 100.0, openui_style::Easing::Linear)
            .unwrap();
        engine.set_animation_time(50.0).unwrap();
        assert_eq!(engine.scroll_offset(node).unwrap(), (50.0, 20.0));
        engine.set_animation_time(100.0).unwrap();
        assert_eq!(engine.scroll_offset(node).unwrap(), (100.0, 40.0));
        assert!(!engine.is_animating());
        let snap = engine
            .settle_scroll_snap(
                node,
                &[0.0, 80.0, 160.0],
                &[0.0, 50.0],
                50.0,
                openui_style::Easing::Linear,
            )
            .unwrap();
        assert!(snap.is_some());
        engine.set_animation_time(150.0).unwrap();
        assert_eq!(engine.scroll_offset(node).unwrap(), (80.0, 50.0));
    }
}
