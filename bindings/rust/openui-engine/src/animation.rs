use crate::{Engine, EngineError, NodeHandle};
use openui_style::{
    value_from_computed, AnimationOptions, AnimationPhase, CompositeOperation, Keyframes,
    LengthValue, PlayState, PropertyKeyframes, StyleProperty, StyleValue, TimelineAxis,
    TimelineRange, TransformList,
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
    pub timeline: AnimationTimeline,
    pub underlying: StyleValue,
    pub start_time_ms: f64,
    pub offset_ms: f64,
    pub hold_time_ms: Option<f64>,
    pub last_applied: Option<StyleValue>,
    pub last_phase: Option<AnimationPhase>,
    pub last_iteration: u64,
    pub last_current_time_ms: f64,
    pub ended: bool,
}

impl AnimationInstance {
    // offset_ms is a LOCAL time at the timeline anchor start_time_ms.
    // Applying rate here, rather than in the effect sampler, preserves seeks
    // and continuity when a native application changes playback rate.
    fn current_time(&self, timeline_time_ms: f64) -> f64 {
        self.hold_time_ms.unwrap_or_else(|| {
            self.offset_ms + (timeline_time_ms - self.start_time_ms) * self.options.playback_rate
        })
    }

    fn reached_endpoint(&self, local_time_ms: f64) -> bool {
        self.options.play_state == PlayState::Running
            && ((self.options.playback_rate > 0.0 && local_time_ms >= self.options.effect_end_ms())
                || (self.options.playback_rate < 0.0 && local_time_ms <= 0.0))
    }

    fn reanchor(&mut self, timeline_time_ms: f64, local_time_ms: f64) {
        self.start_time_ms = timeline_time_ms;
        self.offset_ms = local_time_ms;
        self.hold_time_ms = (self.options.play_state == PlayState::Paused
            || self.options.playback_rate == 0.0
            || (matches!(self.timeline, AnimationTimeline::Document)
                && self.reached_endpoint(local_time_ms)))
        .then_some(local_time_ms);
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
        self.start_animation(target, keyframes, options, timeline, None)
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
        self.slots[target.index as usize]
            .authored
            .insert(property as u16, to.clone());
        let order = &mut self.slots[target.index as usize].authored_order;
        order.retain(|id| *id != property as u16);
        order.push(property as u16);
        self.start_animation(
            target,
            keyframes,
            options,
            AnimationTimeline::Document,
            Some(to),
        )
    }

    fn start_animation(
        &mut self,
        target: NodeHandle,
        keyframes: PropertyKeyframes,
        options: AnimationOptions,
        timeline: AnimationTimeline,
        underlying_override: Option<StyleValue>,
    ) -> Result<AnimationId, EngineError> {
        let node = self.resolve(target)?;
        options.validate().map_err(EngineError::from)?;
        self.validate_timeline(timeline)?;
        let initial_time = initial_local_time(&options)?;
        let property = keyframes.property();
        let underlying = underlying_override
            .or_else(|| {
                self.slots[target.index as usize]
                    .authored
                    .get(&(property as u16))
                    .cloned()
            })
            .unwrap_or_else(|| value_from_computed(&self.document.node(node).style, property));
        let id = AnimationId(self.next_animation_id);
        self.next_animation_id = self.next_animation_id.wrapping_add(1).max(1);
        let timeline_now = self.timeline_time(timeline, &options)?;
        let start_time_ms = if matches!(timeline, AnimationTimeline::Document) {
            timeline_now
        } else {
            0.0
        };
        let held = options.play_state == PlayState::Paused || options.playback_rate == 0.0;
        self.animations.insert(
            id,
            AnimationInstance {
                target,
                keyframes,
                options,
                timeline,
                underlying,
                start_time_ms,
                offset_ms: initial_time,
                hold_time_ms: held.then_some(initial_time),
                last_applied: None,
                last_phase: None,
                last_iteration: 0,
                last_current_time_ms: initial_time,
                ended: false,
            },
        );
        self.sample_animations()?;
        Ok(id)
    }

    pub fn pause_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        let snapshot = self.animation_instance(id)?.clone();
        if snapshot.options.play_state == PlayState::Paused {
            return Ok(());
        }
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let current = snapshot.current_time(now);
        let instance = self.animations.get_mut(&id).expect("live animation");
        instance.options.play_state = PlayState::Paused;
        instance.reanchor(now, current);
        self.sample_animations()
    }

    pub fn play_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        let snapshot = self.animation_instance(id)?.clone();
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let mut current = snapshot.current_time(now);
        if matches!(snapshot.timeline, AnimationTimeline::Document) {
            let end = snapshot.options.effect_end_ms();
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
        let snapshot = self.animation_instance(id)?.clone();
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        self.animations
            .get_mut(&id)
            .expect("live animation")
            .reanchor(now, current_time_ms);
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
        let snapshot = self.animation_instance(id)?.clone();
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let current = snapshot.current_time(now);
        let instance = self.animations.get_mut(&id).expect("live animation");
        instance.options.playback_rate = playback_rate;
        instance.reanchor(now, current);
        self.sample_animations()
    }

    pub fn finish_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
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
            snapshot.options.effect_end_ms()
        };
        if !endpoint.is_finite() {
            return Err(EngineError::InvalidInput(
                "cannot finish at an infinite effect end",
            ));
        }
        let now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
        let instance = self.animations.get_mut(&id).expect("live animation");
        // Finishing a paused animation resolves its start time and finished state.
        instance.options.play_state = PlayState::Running;
        instance.reanchor(now, endpoint);
        self.sample_animations()
    }

    pub fn cancel_animation(&mut self, id: AnimationId) -> Result<(), EngineError> {
        let snapshot = self.animation_instance(id)?.clone();
        let elapsed_time_ms = self.cancel_elapsed_time(&snapshot);
        self.animations.remove(&id);
        if let Err(error) = self.apply_animation_value(
            snapshot.target,
            snapshot.keyframes.property(),
            &snapshot.underlying,
        ) {
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

    pub fn animation_state(&self, id: AnimationId) -> Result<AnimationState, EngineError> {
        let instance = self.animation_instance(id)?;
        let timeline_now = self.timeline_time(instance.timeline, &instance.options)?;
        let current_time_ms = instance.current_time(timeline_now);
        let effective = self.effective_elapsed(instance, current_time_ms);
        let sample = instance.options.sample_local_time(effective);
        Ok(AnimationState {
            id,
            target: instance.target,
            property: instance.keyframes.property(),
            play_state: instance.options.play_state,
            current_time_ms,
            iteration: sample.iteration,
            phase: sample.phase,
            finished: instance.ended,
        })
    }

    pub fn is_animating(&self) -> bool {
        !self.reduced_motion
            && (self.animations.values().any(|animation| {
                animation.options.play_state == PlayState::Running
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
        let ids = self.animations.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let Some(snapshot) = self.animations.get(&id).cloned() else {
                continue;
            };
            if self.resolve(snapshot.target).is_err() {
                self.animations.remove(&id);
                continue;
            }
            let timeline_now = self.timeline_time(snapshot.timeline, &snapshot.options)?;
            let raw_time = snapshot.current_time(timeline_now);
            let current_time = if snapshot.hold_time_ms.is_none()
                && matches!(snapshot.timeline, AnimationTimeline::Document)
                && snapshot.reached_endpoint(raw_time)
            {
                if snapshot.options.playback_rate < 0.0 {
                    0.0
                } else {
                    snapshot.options.effect_end_ms()
                }
            } else {
                raw_time
            };
            let effective = self.effective_elapsed(&snapshot, current_time);
            let (sample, current_direction_is_reversed) =
                snapshot.options.sample_local_time_with_direction(effective);
            let finished = self.reduced_motion || snapshot.reached_endpoint(current_time);
            let sampled = sample
                .eased_progress(&snapshot.options.easing, current_direction_is_reversed)
                .map(|progress| {
                    let value = snapshot.keyframes.sample_with_before(
                        progress,
                        &openui_style::Easing::Linear,
                        // Pinned .50 KeyframeEffect uses Before directly here;
                        // its reverse-after limit differs from effect easing.
                        sample.phase == AnimationPhase::Before,
                    );
                    compose_value(
                        &snapshot.underlying,
                        value,
                        snapshot.options.composite,
                        sample.iteration,
                    )
                });
            let next_value = sampled.as_ref().unwrap_or(&snapshot.underlying);
            if snapshot.last_applied.as_ref() != sampled.as_ref() {
                // Inherited values recompute this node and its descendants.
                // Publish this sample before rebuilding the computed style so
                // the rebuild sees the new frame rather than the previous one.
                self.animations
                    .get_mut(&id)
                    .expect("live animation")
                    .last_applied = sampled.clone();
                if let Err(error) = self.apply_animation_value(
                    snapshot.target,
                    snapshot.keyframes.property(),
                    next_value,
                ) {
                    self.animations
                        .get_mut(&id)
                        .expect("live animation")
                        .last_applied = snapshot.last_applied;
                    return Err(error);
                }
            }

            let phase_changed = snapshot.last_phase != Some(sample.phase);
            let was_before = snapshot.last_phase.is_none()
                || snapshot.last_phase == Some(AnimationPhase::Before);
            let was_after = snapshot.last_phase == Some(AnimationPhase::After);
            let interval_start = (-snapshot.options.delay_ms)
                .min(snapshot.options.active_duration_ms())
                .max(0.0);
            let interval_end = (snapshot.options.effect_end_ms() - snapshot.options.delay_ms)
                .min(snapshot.options.active_duration_ms())
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
                    snapshot.options.duration_ms * boundary as f64,
                ));
            }
            for (kind, elapsed_time_ms) in events {
                self.animation_events.push(AnimationEvent {
                    animation: id,
                    target: snapshot.target,
                    property: snapshot.keyframes.property(),
                    kind,
                    elapsed_time_ms,
                    iteration: sample.iteration,
                });
            }

            if let Some(instance) = self.animations.get_mut(&id) {
                if matches!(snapshot.timeline, AnimationTimeline::Document) && finished {
                    if self.reduced_motion && effective.is_finite() {
                        instance.hold_time_ms = Some(effective);
                    } else if snapshot.hold_time_ms.is_none() {
                        instance.hold_time_ms = Some(current_time);
                    }
                }
                instance.last_applied = sampled;
                instance.last_phase = Some(sample.phase);
                instance.last_iteration = sample.iteration;
                instance.last_current_time_ms = current_time;
                instance.ended = finished;
            }
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
            } else if instance.options.effect_end_ms().is_finite() {
                instance.options.effect_end_ms()
            } else {
                instance.options.delay_ms + instance.options.duration_ms
            }
        } else {
            current_time
        }
    }

    fn cancel_elapsed_time(&self, instance: &AnimationInstance) -> f64 {
        let current = self
            .timeline_time(instance.timeline, &instance.options)
            .map(|time| instance.current_time(time))
            .unwrap_or(instance.last_current_time_ms);
        (current - instance.options.delay_ms)
            .min(instance.options.active_duration_ms())
            .max(0.0)
    }

    fn apply_animation_value(
        &mut self,
        target: NodeHandle,
        property: StyleProperty,
        value: &StyleValue,
    ) -> Result<(), EngineError> {
        let node = self.resolve(target)?;
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        let font = if property == StyleProperty::FontSize {
            let parent = self.document.node(node).parent;
            if parent.is_none() {
                openui_style::ComputedStyle::initial().font_size
            } else {
                self.document.node(parent).style.font_size
            }
        } else {
            self.document.node(node).style.font_size
        };
        let root_font = if node == self.document.root() && property == StyleProperty::FontSize {
            openui_style::ComputedStyle::initial().font_size
        } else {
            self.document.node(self.document.root()).style.font_size
        };
        let metrics = value.depends_on_font_metrics().then(|| {
            let style = if property == StyleProperty::FontSize {
                let parent = self.document.node(node).parent;
                if parent.is_none() {
                    openui_style::ComputedStyle::initial()
                } else {
                    self.document.node(parent).style.clone()
                }
            } else {
                self.document.node(node).style.clone()
            };
            let mut metrics = self.native_font_length_metrics(&style);
            if property == StyleProperty::LineHeight {
                let parent = self.document.node(node).parent;
                let parent_style = if parent.is_none() {
                    openui_style::ComputedStyle::initial()
                } else {
                    self.document.node(parent).style.clone()
                };
                metrics.lh = self.native_font_length_metrics(&parent_style).lh;
            }
            metrics
        });
        let resolved =
            Self::resolve_native_lengths(property, value, font, root_font, viewport, metrics);
        // Publish to resolved targets as well as authored targets. The shared
        // refresh then updates relative lengths and native descendants.
        self.document
            .apply_style_property(node, property, &resolved, viewport)
            .map_err(|_| EngineError::PropertyType { property })?;
        if property.metadata().inherited || value.depends_on_font_metrics() {
            self.refresh_inherited_styles(node)?;
        }
        self.dirty.hit_test = true;
        self.mark_dirty(property.metadata().invalidation);
        Ok(())
    }

    fn animation_instance(&self, id: AnimationId) -> Result<&AnimationInstance, EngineError> {
        self.animations
            .get(&id)
            .ok_or(EngineError::InvalidInput("unknown animation"))
    }

    fn validate_timeline(&self, timeline: AnimationTimeline) -> Result<(), EngineError> {
        match timeline {
            AnimationTimeline::Document => Ok(()),
            AnimationTimeline::Scroll { source, range, .. } => {
                self.resolve(source)?;
                range.validate().map_err(EngineError::from)
            }
            AnimationTimeline::View { subject, range, .. } => {
                self.resolve(subject)?;
                range.validate().map_err(EngineError::from)
            }
        }
    }

    fn timeline_time(
        &self,
        timeline: AnimationTimeline,
        options: &AnimationOptions,
    ) -> Result<f64, EngineError> {
        let virtual_duration = if options.active_duration_ms().is_finite() {
            options.active_duration_ms()
        } else {
            options.duration_ms
        };
        match timeline {
            AnimationTimeline::Document => Ok(self.animation_time_ms),
            AnimationTimeline::Scroll {
                source,
                axis,
                range,
            } => {
                let node = self.document.node(self.resolve(source)?);
                let position = match axis {
                    TimelineAxis::Inline | TimelineAxis::X => node.scroll_left as f64,
                    TimelineAxis::Block | TimelineAxis::Y => node.scroll_top as f64,
                };
                Ok(options.delay_ms + range.progress(position) * virtual_duration)
            }
            AnimationTimeline::View {
                subject,
                axis,
                range,
            } => {
                let node = self.resolve(subject)?;
                let position = self
                    .node_bounds(node)
                    .map(|bounds| match axis {
                        TimelineAxis::Inline | TimelineAxis::X => {
                            (bounds.x + bounds.width * 0.5) as f64
                        }
                        TimelineAxis::Block | TimelineAxis::Y => {
                            (bounds.y + bounds.height * 0.5) as f64
                        }
                    })
                    .unwrap_or_default();
                Ok(options.delay_ms + range.progress(position) * virtual_duration)
            }
        }
    }
}

fn initial_local_time(options: &AnimationOptions) -> Result<f64, EngineError> {
    if options.playback_rate < 0.0 {
        let end = options.effect_end_ms();
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

fn compose_value(
    underlying: &StyleValue,
    sampled: StyleValue,
    operation: CompositeOperation,
    iteration: u64,
) -> StyleValue {
    if operation == CompositeOperation::Replace {
        return sampled;
    }
    let factor = if operation == CompositeOperation::Accumulate {
        iteration.saturating_add(1) as f32
    } else {
        1.0
    };
    match (underlying, sampled) {
        (StyleValue::Number(base), StyleValue::Number(value)) => {
            StyleValue::Number(*base + value * factor)
        }
        (StyleValue::Integer(base), StyleValue::Integer(value)) => {
            StyleValue::Integer(base.saturating_add((value as f32 * factor).round() as i32))
        }
        (
            StyleValue::Length(LengthValue::Computed(base)),
            StyleValue::Length(LengthValue::Computed(value)),
        ) if base.length_type() == value.length_type() && base.is_fixed() => {
            StyleValue::Length(LengthValue::px(base.value() + value.value() * factor))
        }
        (StyleValue::Transform(base), StyleValue::Transform(value)) => {
            let mut operations = base.0.clone();
            for _ in 0..factor.max(1.0).round() as usize {
                operations.extend(value.0.clone());
            }
            StyleValue::Transform(TransformList(operations))
        }
        (_, value) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewportMetrics;
    use openui_dom::ElementTag;
    use openui_style::{Display, FillMode, Keyframe, Overflow};

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
        assert_eq!(engine.animation_state(id).unwrap().current_time_ms, 70.0);
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
