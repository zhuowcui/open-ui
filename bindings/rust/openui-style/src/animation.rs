//! Deterministic typed animation timing and property interpolation.

use crate::{
    apply_to_computed, Color, ComputedStyle, Edges, FontSizeAdjust, FontStretch, FontWeight, Gap,
    InitialLetterValue, InterpolationKind, LengthValue, LineHeight, StyleColor, StyleProperty,
    StyleValue, TextDecorationThickness, TextSizeAdjust, TransformList, TransformOperation,
    TypographyValue, VerticalAlign,
};
use openui_geometry::{Length, LengthType};

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationError {
    InvalidTiming(&'static str),
    InvalidEasing(&'static str),
    InvalidKeyframes(&'static str),
    PropertyType { property: StyleProperty },
}

impl std::fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTiming(message) => {
                write!(formatter, "invalid animation timing: {message}")
            }
            Self::InvalidEasing(message) => write!(formatter, "invalid easing: {message}"),
            Self::InvalidKeyframes(message) => write!(formatter, "invalid keyframes: {message}"),
            Self::PropertyType { property } => write!(
                formatter,
                "keyframe value does not match `{}`",
                property.metadata().css_name
            ),
        }
    }
}

impl std::error::Error for AnimationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPosition {
    JumpStart,
    JumpEnd,
    JumpNone,
    JumpBoth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearStop {
    pub input: f64,
    pub output: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Easing {
    #[default]
    Linear,
    CubicBezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Steps {
        count: u32,
        position: StepPosition,
    },
    LinearStops(Vec<LinearStop>),
}

impl Easing {
    pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> Result<Self, AnimationError> {
        if [x1, y1, x2, y2].iter().any(|value| !value.is_finite())
            || !(0.0..=1.0).contains(&x1)
            || !(0.0..=1.0).contains(&x2)
        {
            return Err(AnimationError::InvalidEasing(
                "cubic-bezier x coordinates must be finite and in [0, 1]",
            ));
        }
        Ok(Self::CubicBezier { x1, y1, x2, y2 })
    }

    pub fn steps(count: u32, position: StepPosition) -> Result<Self, AnimationError> {
        if count == 0 || (position == StepPosition::JumpNone && count == 1) {
            return Err(AnimationError::InvalidEasing(
                "steps requires a positive interval count (at least two for jump-none)",
            ));
        }
        Ok(Self::Steps { count, position })
    }

    pub fn linear_stops(stops: Vec<LinearStop>) -> Result<Self, AnimationError> {
        if stops.len() < 2
            || stops
                .iter()
                .any(|stop| !stop.input.is_finite() || !stop.output.is_finite())
            || stops.windows(2).any(|pair| pair[0].input > pair[1].input)
            || stops.first().is_some_and(|stop| stop.input != 0.0)
            || stops.last().is_some_and(|stop| stop.input != 1.0)
        {
            return Err(AnimationError::InvalidEasing(
                "linear easing requires ordered finite stops spanning 0 through 1",
            ));
        }
        Ok(Self::LinearStops(stops))
    }

    pub fn sample(&self, input: f64) -> f64 {
        let input = input.clamp(0.0, 1.0);
        match self {
            Self::Linear => input,
            Self::CubicBezier { x1, y1, x2, y2 } => sample_cubic_bezier(input, *x1, *y1, *x2, *y2),
            Self::Steps { count, position } => sample_steps(input, *count, *position),
            Self::LinearStops(stops) => {
                let upper = stops.partition_point(|stop| stop.input < input);
                if upper == 0 {
                    return stops[0].output;
                }
                if upper == stops.len() {
                    return stops[stops.len() - 1].output;
                }
                let left = stops[upper - 1];
                let right = stops[upper];
                if right.input == left.input {
                    right.output
                } else {
                    let progress = (input - left.input) / (right.input - left.input);
                    left.output + (right.output - left.output) * progress
                }
            }
        }
    }
}

fn cubic(value: f64, a: f64, b: f64) -> f64 {
    let inverse = 1.0 - value;
    3.0 * inverse * inverse * value * a + 3.0 * inverse * value * value * b + value.powi(3)
}

fn sample_cubic_bezier(input: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    if input == 0.0 || input == 1.0 {
        return input;
    }
    let mut low = 0.0;
    let mut high = 1.0;
    let mut parameter = input;
    for _ in 0..24 {
        let x = cubic(parameter, x1, x2);
        if (x - input).abs() <= 1e-8 {
            break;
        }
        if x < input {
            low = parameter;
        } else {
            high = parameter;
        }
        parameter = (low + high) * 0.5;
    }
    cubic(parameter, y1, y2)
}

fn sample_steps(input: f64, count: u32, position: StepPosition) -> f64 {
    let intervals = count as f64;
    let value = match position {
        StepPosition::JumpStart => (input * intervals).floor() + 1.0,
        StepPosition::JumpEnd => (input * intervals).floor(),
        StepPosition::JumpNone => (input * intervals).floor().min(intervals - 1.0),
        StepPosition::JumpBoth => (input * intervals).floor() + 1.0,
    };
    let denominator = match position {
        StepPosition::JumpNone => intervals - 1.0,
        StepPosition::JumpBoth => intervals + 1.0,
        _ => intervals,
    };
    (value / denominator).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

impl FillMode {
    pub fn fills_before(self) -> bool {
        matches!(self, Self::Backwards | Self::Both)
    }

    pub fn fills_after(self) -> bool {
        matches!(self, Self::Forwards | Self::Both)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IterationCount {
    Number(f64),
    Infinite,
}

impl Default for IterationCount {
    fn default() -> Self {
        Self::Number(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeOperation {
    Replace,
    Add,
    Accumulate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationOptions {
    pub delay_ms: f64,
    pub duration_ms: f64,
    pub iterations: IterationCount,
    pub direction: PlaybackDirection,
    pub fill: FillMode,
    pub play_state: PlayState,
    pub playback_rate: f64,
    pub composite: CompositeOperation,
    pub easing: Easing,
}

impl Default for AnimationOptions {
    fn default() -> Self {
        Self {
            delay_ms: 0.0,
            duration_ms: 0.0,
            iterations: IterationCount::Number(1.0),
            direction: PlaybackDirection::Normal,
            fill: FillMode::None,
            play_state: PlayState::Running,
            playback_rate: 1.0,
            composite: CompositeOperation::Replace,
            easing: Easing::Linear,
        }
    }
}

impl AnimationOptions {
    pub fn validate(&self) -> Result<(), AnimationError> {
        if !self.delay_ms.is_finite() {
            return Err(AnimationError::InvalidTiming("delay must be finite"));
        }
        if !self.duration_ms.is_finite() || self.duration_ms < 0.0 {
            return Err(AnimationError::InvalidTiming(
                "duration must be finite and non-negative",
            ));
        }
        if !self.playback_rate.is_finite() || self.playback_rate == 0.0 {
            return Err(AnimationError::InvalidTiming(
                "playback rate must be finite and non-zero",
            ));
        }
        if let IterationCount::Number(iterations) = self.iterations {
            if !iterations.is_finite() || iterations < 0.0 {
                return Err(AnimationError::InvalidTiming(
                    "iteration count must be finite and non-negative",
                ));
            }
        }
        match &self.easing {
            Easing::CubicBezier { x1, y1, x2, y2 } => {
                Self::validate_easing(Easing::cubic_bezier(*x1, *y1, *x2, *y2)?)
            }
            Easing::Steps { count, position } => {
                Self::validate_easing(Easing::steps(*count, *position)?)
            }
            Easing::LinearStops(stops) => {
                Self::validate_easing(Easing::linear_stops(stops.clone())?)
            }
            Easing::Linear => Ok(()),
        }
    }

    fn validate_easing(_: Easing) -> Result<(), AnimationError> {
        Ok(())
    }

    pub fn active_duration_ms(&self) -> f64 {
        match self.iterations {
            IterationCount::Number(iterations) => self.duration_ms * iterations,
            IterationCount::Infinite => f64::INFINITY,
        }
    }

    pub fn sample(&self, elapsed_since_start_ms: f64) -> AnimationSample {
        if self.duration_ms == 0.0 {
            let after_delay = elapsed_since_start_ms >= self.delay_ms;
            return AnimationSample {
                phase: if after_delay {
                    AnimationPhase::After
                } else {
                    AnimationPhase::Before
                },
                iteration: 0,
                progress: if after_delay && self.fill.fills_after() {
                    Some(directed_progress(self.direction, 0, 1.0))
                } else if !after_delay && self.fill.fills_before() {
                    Some(directed_progress(self.direction, 0, 0.0))
                } else {
                    None
                },
            };
        }

        let active_duration = self.active_duration_ms();
        let unscaled = elapsed_since_start_ms - self.delay_ms;
        let active_time = if self.playback_rate < 0.0 {
            active_duration + unscaled * self.playback_rate
        } else {
            unscaled * self.playback_rate
        };
        if active_time < 0.0 {
            return AnimationSample {
                phase: AnimationPhase::Before,
                iteration: 0,
                progress: self
                    .fill
                    .fills_before()
                    .then(|| directed_progress(self.direction, 0, 0.0)),
            };
        }
        if active_time >= active_duration {
            let final_iteration = match self.iterations {
                IterationCount::Number(value) if value > 0.0 => value.ceil() as u64 - 1,
                _ => 0,
            };
            let terminal = match self.iterations {
                IterationCount::Number(value) if value.fract() != 0.0 => value.fract(),
                _ => 1.0,
            };
            return AnimationSample {
                phase: AnimationPhase::After,
                iteration: final_iteration,
                progress: self
                    .fill
                    .fills_after()
                    .then(|| directed_progress(self.direction, final_iteration, terminal)),
            };
        }

        let iteration = (active_time / self.duration_ms).floor() as u64;
        let iteration_time = active_time - iteration as f64 * self.duration_ms;
        AnimationSample {
            phase: AnimationPhase::Active,
            iteration,
            progress: Some(directed_progress(
                self.direction,
                iteration,
                iteration_time / self.duration_ms,
            )),
        }
    }
}

fn directed_progress(direction: PlaybackDirection, iteration: u64, progress: f64) -> f64 {
    let reversed = match direction {
        PlaybackDirection::Normal => false,
        PlaybackDirection::Reverse => true,
        PlaybackDirection::Alternate => iteration % 2 == 1,
        PlaybackDirection::AlternateReverse => iteration & 1 == 0,
    };
    if reversed {
        1.0 - progress
    } else {
        progress
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPhase {
    Before,
    Active,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationSample {
    pub phase: AnimationPhase,
    pub iteration: u64,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe<T> {
    pub offset: f64,
    pub value: T,
    pub easing: Option<Easing>,
}

impl<T> Keyframe<T> {
    pub fn new(offset: f64, value: T) -> Self {
        Self {
            offset,
            value,
            easing: None,
        }
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = Some(easing);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keyframes<T> {
    frames: Vec<Keyframe<T>>,
}

impl<T> Keyframes<T> {
    pub fn new(frames: Vec<Keyframe<T>>) -> Result<Self, AnimationError> {
        if frames.len() < 2
            || frames
                .iter()
                .any(|frame| !frame.offset.is_finite() || !(0.0..=1.0).contains(&frame.offset))
            || frames
                .windows(2)
                .any(|pair| pair[0].offset >= pair[1].offset)
            || frames.first().is_some_and(|frame| frame.offset != 0.0)
            || frames.last().is_some_and(|frame| frame.offset != 1.0)
        {
            return Err(AnimationError::InvalidKeyframes(
                "offsets must be strictly ordered and span 0 through 1",
            ));
        }
        Ok(Self { frames })
    }

    pub fn from_values(from: T, to: T) -> Self {
        Self {
            frames: vec![Keyframe::new(0.0, from), Keyframe::new(1.0, to)],
        }
    }

    pub fn frames(&self) -> &[Keyframe<T>] {
        &self.frames
    }

    pub fn into_frames(self) -> Vec<Keyframe<T>> {
        self.frames
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyKeyframe {
    pub offset: f64,
    pub value: StyleValue,
    pub easing: Option<Easing>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyKeyframes {
    property: StyleProperty,
    frames: Vec<PropertyKeyframe>,
}

impl PropertyKeyframes {
    pub fn typed<T>(
        property: StyleProperty,
        keyframes: Keyframes<T>,
    ) -> Result<Self, AnimationError>
    where
        T: Into<StyleValue>,
    {
        let frames = keyframes
            .into_frames()
            .into_iter()
            .map(|frame| PropertyKeyframe {
                offset: frame.offset,
                value: frame.value.into(),
                easing: frame.easing,
            })
            .collect::<Vec<_>>();
        if frames
            .iter()
            .any(|frame| !value_matches_property(property, &frame.value))
        {
            return Err(AnimationError::PropertyType { property });
        }
        Ok(Self { property, frames })
    }

    pub fn property(&self) -> StyleProperty {
        self.property
    }

    pub fn frames(&self) -> &[PropertyKeyframe] {
        &self.frames
    }

    pub fn sample(&self, progress: f64, default_easing: &Easing) -> StyleValue {
        let progress = progress.clamp(0.0, 1.0);
        let upper = self.frames.partition_point(|frame| frame.offset < progress);
        if upper == 0 {
            return self.frames[0].value.clone();
        }
        if upper == self.frames.len() {
            return self.frames[self.frames.len() - 1].value.clone();
        }
        let from = &self.frames[upper - 1];
        let to = &self.frames[upper];
        let interval = (progress - from.offset) / (to.offset - from.offset);
        let eased = from
            .easing
            .as_ref()
            .unwrap_or(default_easing)
            .sample(interval);
        interpolate(self.property, &from.value, &to.value, eased).unwrap_or_else(|| {
            if eased < 0.5 {
                from.value.clone()
            } else {
                to.value.clone()
            }
        })
    }
}

pub fn value_matches_property(property: StyleProperty, value: &StyleValue) -> bool {
    apply_to_computed(
        &mut ComputedStyle::initial(),
        property,
        value,
        (800.0, 600.0),
    )
    .is_ok()
}

pub fn interpolate(
    property: StyleProperty,
    from: &StyleValue,
    to: &StyleValue,
    progress: f64,
) -> Option<StyleValue> {
    let progress = progress as f32;
    if property.metadata().interpolation == InterpolationKind::Discrete {
        return Some(if progress < 0.5 {
            from.clone()
        } else {
            to.clone()
        });
    }
    match (from, to) {
        (StyleValue::Number(from), StyleValue::Number(to)) => {
            Some(StyleValue::Number(lerp(*from, *to, progress)))
        }
        (StyleValue::Integer(from), StyleValue::Integer(to)) => Some(StyleValue::Integer(
            lerp(*from as f32, *to as f32, progress).round() as i32,
        )),
        (StyleValue::FontWeight(from), StyleValue::FontWeight(to)) => Some(StyleValue::FontWeight(
            FontWeight(lerp(from.0, to.0, progress)),
        )),
        (StyleValue::Color(from), StyleValue::Color(to)) => {
            Some(StyleValue::Color(interpolate_color(*from, *to, progress)))
        }
        (StyleValue::Length(from), StyleValue::Length(to)) => {
            interpolate_length(*from, *to, progress).map(StyleValue::Length)
        }
        (StyleValue::Edges(from), StyleValue::Edges(to)) => Some(StyleValue::Edges(Edges {
            top: interpolate_length(from.top, to.top, progress)?,
            right: interpolate_length(from.right, to.right, progress)?,
            bottom: interpolate_length(from.bottom, to.bottom, progress)?,
            left: interpolate_length(from.left, to.left, progress)?,
        })),
        (StyleValue::Gap(from), StyleValue::Gap(to)) => Some(StyleValue::Gap(Gap {
            row: interpolate_length(from.row, to.row, progress)?,
            column: interpolate_length(from.column, to.column, progress)?,
        })),
        (StyleValue::CornerRadii(from), StyleValue::CornerRadii(to)) => {
            let from = from.0;
            let to = to.0;
            Some(StyleValue::CornerRadii(crate::CornerRadii(Edges {
                top: interpolate_length(from.top, to.top, progress)?,
                right: interpolate_length(from.right, to.right, progress)?,
                bottom: interpolate_length(from.bottom, to.bottom, progress)?,
                left: interpolate_length(from.left, to.left, progress)?,
            })))
        }
        (StyleValue::Transform(from), StyleValue::Transform(to)) => {
            interpolate_transform(from, to, progress).map(StyleValue::Transform)
        }
        (StyleValue::Typography(from), StyleValue::Typography(to)) => {
            interpolate_typography(from, to, progress).map(StyleValue::Typography)
        }
        _ => Some(if progress < 0.5 {
            from.clone()
        } else {
            to.clone()
        }),
    }
}

fn interpolate_typography(
    from: &TypographyValue,
    to: &TypographyValue,
    progress: f32,
) -> Option<TypographyValue> {
    Some(match (from, to) {
        (TypographyValue::FontStretch(from), TypographyValue::FontStretch(to)) => {
            TypographyValue::FontStretch(FontStretch(lerp(from.0, to.0, progress)))
        }
        (TypographyValue::FontSizeAdjust(from), TypographyValue::FontSizeAdjust(to)) => {
            let value = match (from, to) {
                (FontSizeAdjust::ExHeight(a), FontSizeAdjust::ExHeight(b)) => {
                    FontSizeAdjust::ExHeight(lerp(*a, *b, progress))
                }
                (FontSizeAdjust::CapHeight(a), FontSizeAdjust::CapHeight(b)) => {
                    FontSizeAdjust::CapHeight(lerp(*a, *b, progress))
                }
                (FontSizeAdjust::ChWidth(a), FontSizeAdjust::ChWidth(b)) => {
                    FontSizeAdjust::ChWidth(lerp(*a, *b, progress))
                }
                (FontSizeAdjust::IcWidth(a), FontSizeAdjust::IcWidth(b)) => {
                    FontSizeAdjust::IcWidth(lerp(*a, *b, progress))
                }
                (FontSizeAdjust::IcHeight(a), FontSizeAdjust::IcHeight(b)) => {
                    FontSizeAdjust::IcHeight(lerp(*a, *b, progress))
                }
                _ => return None,
            };
            TypographyValue::FontSizeAdjust(value)
        }
        (TypographyValue::LineHeight(from), TypographyValue::LineHeight(to)) => {
            let value = match (from, to) {
                (LineHeight::Number(a), LineHeight::Number(b)) => {
                    LineHeight::Number(lerp(*a, *b, progress))
                }
                (LineHeight::Length(a), LineHeight::Length(b)) => {
                    LineHeight::Length(lerp(*a, *b, progress))
                }
                (LineHeight::Percentage(a), LineHeight::Percentage(b)) => {
                    LineHeight::Percentage(lerp(*a, *b, progress))
                }
                _ => return None,
            };
            TypographyValue::LineHeight(value)
        }
        (
            TypographyValue::TextDecorationThickness(TextDecorationThickness::Length(a)),
            TypographyValue::TextDecorationThickness(TextDecorationThickness::Length(b)),
        ) => TypographyValue::TextDecorationThickness(TextDecorationThickness::Length(lerp(
            *a, *b, progress,
        ))),
        (
            TypographyValue::VerticalAlign(VerticalAlign::Length(a)),
            TypographyValue::VerticalAlign(VerticalAlign::Length(b)),
        ) => TypographyValue::VerticalAlign(VerticalAlign::Length(lerp(*a, *b, progress))),
        (
            TypographyValue::VerticalAlign(VerticalAlign::Percentage(a)),
            TypographyValue::VerticalAlign(VerticalAlign::Percentage(b)),
        ) => TypographyValue::VerticalAlign(VerticalAlign::Percentage(lerp(*a, *b, progress))),
        (
            TypographyValue::TextSizeAdjust(TextSizeAdjust::Percentage(a)),
            TypographyValue::TextSizeAdjust(TextSizeAdjust::Percentage(b)),
        ) => TypographyValue::TextSizeAdjust(TextSizeAdjust::Percentage(lerp(*a, *b, progress))),
        (
            TypographyValue::InitialLetter(InitialLetterValue::Value(a)),
            TypographyValue::InitialLetter(InitialLetterValue::Value(b)),
        ) => TypographyValue::InitialLetter(InitialLetterValue::Value(crate::InitialLetter {
            size: lerp(a.size, b.size, progress),
            sink: match (a.sink, b.sink) {
                (Some(a), Some(b)) => Some(lerp(a, b, progress)),
                (None, None) => None,
                _ => return None,
            },
        })),
        (
            TypographyValue::StyleColor(StyleColor::Resolved(a)),
            TypographyValue::StyleColor(StyleColor::Resolved(b)),
        ) => TypographyValue::StyleColor(StyleColor::Resolved(interpolate_color(*a, *b, progress))),
        _ => return None,
    })
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

fn interpolate_color(from: Color, to: Color, progress: f32) -> Color {
    Color::from_rgba_f32(
        lerp(from.r, to.r, progress),
        lerp(from.g, to.g, progress),
        lerp(from.b, to.b, progress),
        lerp(from.a, to.a, progress),
    )
}

fn interpolate_length(from: LengthValue, to: LengthValue, progress: f32) -> Option<LengthValue> {
    match (from, to) {
        (LengthValue::Computed(from), LengthValue::Computed(to))
            if from.length_type() == to.length_type() =>
        {
            let value = lerp(from.value(), to.value(), progress);
            let offset = lerp(from.calc_offset(), to.calc_offset(), progress);
            let result = match from.length_type() {
                LengthType::Fixed => Length::px(value),
                LengthType::Percent => Length::percent(value),
                LengthType::Calculated => Length::calc_percent_px(value, offset),
                LengthType::Flex => Length::flex(value),
                _ if from == to => from,
                _ => return None,
            };
            Some(LengthValue::Computed(result))
        }
        (LengthValue::Em(from), LengthValue::Em(to)) => {
            Some(LengthValue::Em(lerp(from, to, progress)))
        }
        (LengthValue::Rem(from), LengthValue::Rem(to)) => {
            Some(LengthValue::Rem(lerp(from, to, progress)))
        }
        (LengthValue::ViewportWidth(from), LengthValue::ViewportWidth(to)) => {
            Some(LengthValue::ViewportWidth(lerp(from, to, progress)))
        }
        (LengthValue::ViewportHeight(from), LengthValue::ViewportHeight(to)) => {
            Some(LengthValue::ViewportHeight(lerp(from, to, progress)))
        }
        (LengthValue::ViewportMin(from), LengthValue::ViewportMin(to)) => {
            Some(LengthValue::ViewportMin(lerp(from, to, progress)))
        }
        (LengthValue::ViewportMax(from), LengthValue::ViewportMax(to)) => {
            Some(LengthValue::ViewportMax(lerp(from, to, progress)))
        }
        _ => None,
    }
}

fn interpolate_transform(
    from: &TransformList,
    to: &TransformList,
    progress: f32,
) -> Option<TransformList> {
    if from.0.len() != to.0.len() {
        return None;
    }
    let operations = from
        .0
        .iter()
        .zip(&to.0)
        .map(|(from, to)| match (from, to) {
            (
                TransformOperation::Translate(from_x, from_y),
                TransformOperation::Translate(to_x, to_y),
            ) => Some(TransformOperation::Translate(
                interpolate_length(*from_x, *to_x, progress)?,
                interpolate_length(*from_y, *to_y, progress)?,
            )),
            (
                TransformOperation::Translate3d(from_x, from_y, from_z),
                TransformOperation::Translate3d(to_x, to_y, to_z),
            ) => Some(TransformOperation::Translate3d(
                interpolate_length(*from_x, *to_x, progress)?,
                interpolate_length(*from_y, *to_y, progress)?,
                interpolate_length(*from_z, *to_z, progress)?,
            )),
            (TransformOperation::Scale(from_x, from_y), TransformOperation::Scale(to_x, to_y)) => {
                Some(TransformOperation::Scale(
                    lerp(*from_x, *to_x, progress),
                    lerp(*from_y, *to_y, progress),
                ))
            }
            (
                TransformOperation::Scale3d(from_x, from_y, from_z),
                TransformOperation::Scale3d(to_x, to_y, to_z),
            ) => Some(TransformOperation::Scale3d(
                lerp(*from_x, *to_x, progress),
                lerp(*from_y, *to_y, progress),
                lerp(*from_z, *to_z, progress),
            )),
            (TransformOperation::Rotate(from), TransformOperation::Rotate(to)) => {
                Some(TransformOperation::Rotate(lerp(*from, *to, progress)))
            }
            (
                TransformOperation::Rotate3d {
                    x: from_x,
                    y: from_y,
                    z: from_z,
                    degrees: from_degrees,
                },
                TransformOperation::Rotate3d {
                    x: to_x,
                    y: to_y,
                    z: to_z,
                    degrees: to_degrees,
                },
            ) => Some(TransformOperation::Rotate3d {
                x: lerp(*from_x, *to_x, progress),
                y: lerp(*from_y, *to_y, progress),
                z: lerp(*from_z, *to_z, progress),
                degrees: lerp(*from_degrees, *to_degrees, progress),
            }),
            (TransformOperation::Perspective(from), TransformOperation::Perspective(to)) => Some(
                TransformOperation::Perspective(interpolate_length(*from, *to, progress)?),
            ),
            (TransformOperation::Matrix(from), TransformOperation::Matrix(to)) => {
                Some(TransformOperation::Matrix(crate::Transform2D {
                    a: lerp(from.a, to.a, progress),
                    b: lerp(from.b, to.b, progress),
                    c: lerp(from.c, to.c, progress),
                    d: lerp(from.d, to.d, progress),
                    e: lerp(from.e, to.e, progress),
                    f: lerp(from.f, to.f, progress),
                }))
            }
            (TransformOperation::Matrix3d(from), TransformOperation::Matrix3d(to)) => {
                let mut values = [0.0; 16];
                for (index, value) in values.iter_mut().enumerate() {
                    *value = lerp(from.0[index], to.0[index], progress);
                }
                Some(TransformOperation::Matrix3d(crate::Transform3D(values)))
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TransformList(operations))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineAxis {
    Block,
    Inline,
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRange {
    pub start: f64,
    pub end: f64,
}

impl TimelineRange {
    pub fn new(start: f64, end: f64) -> Result<Self, AnimationError> {
        if !start.is_finite() || !end.is_finite() || start == end {
            return Err(AnimationError::InvalidTiming(
                "timeline range must have distinct finite endpoints",
            ));
        }
        Ok(Self { start, end })
    }

    pub fn progress(self, position: f64) -> f64 {
        ((position - self.start) / (self.end - self.start)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> AnimationOptions {
        AnimationOptions {
            duration_ms: 100.0,
            fill: FillMode::Both,
            ..AnimationOptions::default()
        }
    }

    #[test]
    fn timing_handles_delay_direction_and_iteration_boundaries() {
        let mut timing = options();
        timing.delay_ms = -25.0;
        timing.iterations = IterationCount::Number(2.0);
        timing.direction = PlaybackDirection::Alternate;
        assert_eq!(timing.sample(0.0).progress, Some(0.25));
        assert_eq!(timing.sample(100.0).iteration, 1);
        assert_eq!(timing.sample(100.0).progress, Some(0.75));
        assert_eq!(timing.sample(175.0).progress, Some(0.0));
    }

    #[test]
    fn fill_modes_control_out_of_range_samples() {
        let mut timing = options();
        timing.delay_ms = 10.0;
        timing.fill = FillMode::None;
        assert_eq!(timing.sample(0.0).progress, None);
        assert_eq!(timing.sample(110.0).progress, None);
        timing.fill = FillMode::Both;
        assert_eq!(timing.sample(0.0).progress, Some(0.0));
        assert_eq!(timing.sample(110.0).progress, Some(1.0));
    }

    #[test]
    fn easing_endpoints_and_steps_are_deterministic() {
        let ease = Easing::cubic_bezier(0.42, 0.0, 0.58, 1.0).unwrap();
        assert_eq!(ease.sample(0.0), 0.0);
        assert_eq!(ease.sample(1.0), 1.0);
        assert!((ease.sample(0.5) - 0.5).abs() < 1e-6);
        let steps = Easing::steps(4, StepPosition::JumpEnd).unwrap();
        assert_eq!(steps.sample(0.49), 0.25);
        assert_eq!(steps.sample(1.0), 1.0);
    }

    #[test]
    fn typed_keyframes_reject_a_property_type_mismatch() {
        let keyframes = Keyframes::from_values(Color::BLACK, Color::WHITE);
        assert!(PropertyKeyframes::typed(StyleProperty::Opacity, keyframes).is_err());
    }

    #[test]
    fn interpolates_lengths_colors_and_compatible_transforms() {
        assert_eq!(
            interpolate(
                StyleProperty::Width,
                &StyleValue::Length(LengthValue::px(10.0)),
                &StyleValue::Length(LengthValue::px(30.0)),
                0.25,
            ),
            Some(StyleValue::Length(LengthValue::px(15.0)))
        );
        let frames = PropertyKeyframes::typed(
            StyleProperty::BackgroundColor,
            Keyframes::from_values(Color::BLACK, Color::WHITE),
        )
        .unwrap();
        let StyleValue::Color(color) = frames.sample(0.5, &Easing::Linear) else {
            panic!("expected a color")
        };
        assert_eq!(color, Color::from_rgba_f32(0.5, 0.5, 0.5, 1.0));
    }
}
