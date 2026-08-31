//! Typed computed values shared by table, Grid, containment, and replaced layout.
//!
//! These values deliberately keep CSS grammar structure intact. Layout can
//! therefore distinguish, for example, an intrinsic track from a flexible
//! track or an authored empty line-name list from an omitted value without
//! consulting source text or a test identity.

use std::ops::{BitOr, BitOrAssign};

use openui_geometry::Length;

use crate::BackgroundPosition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableLayout {
    #[default]
    Auto,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderCollapse {
    #[default]
    Separate,
    Collapse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptionSide {
    #[default]
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyCells {
    #[default]
    Show,
    Hide,
}

/// One minimum or maximum component in a Grid track sizing function.
#[derive(Debug, Clone, Copy)]
pub enum GridTrackBreadth {
    Auto,
    MinContent,
    MaxContent,
    Length(Length),
    Flex(f32),
    FitContent(Length),
}

impl GridTrackBreadth {
    pub const fn auto() -> Self {
        Self::Auto
    }
}

/// A Grid track size, preserving `minmax()` instead of prematurely resolving it.
#[derive(Debug, Clone, Copy)]
pub enum GridTrackSize {
    Breadth(GridTrackBreadth),
    MinMax {
        min: GridTrackBreadth,
        max: GridTrackBreadth,
    },
}

impl GridTrackSize {
    pub const fn auto() -> Self {
        Self::Breadth(GridTrackBreadth::Auto)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridRepetition {
    Count(u32),
    AutoFill,
    AutoFit,
}

/// One component of an explicit Grid track list.
#[derive(Debug, Clone)]
pub enum GridTrackComponent {
    LineNames(Vec<String>),
    Track(GridTrackSize),
    Repeat {
        repetition: GridRepetition,
        tracks: Vec<GridTrackComponent>,
    },
}

#[derive(Debug, Clone, Default)]
pub enum GridTrackList {
    #[default]
    None,
    Tracks(Vec<GridTrackComponent>),
    Subgrid(Vec<Vec<String>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridLine {
    Auto,
    Line { index: i32, name: Option<String> },
    Span { count: u32, name: Option<String> },
}

impl Default for GridLine {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GridPlacement {
    pub start: GridLine,
    pub end: GridLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridAutoFlowDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GridAutoFlow {
    pub direction: GridAutoFlowDirection,
    pub dense: bool,
}

/// Rectangular `grid-template-areas` token matrix. A `None` cell is `.`.
#[derive(Debug, Clone, Default)]
pub struct GridTemplateAreas {
    pub rows: Vec<Vec<Option<String>>>,
}

/// Bit set used by `margin-trim`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarginTrim(pub u8);

impl MarginTrim {
    pub const NONE: Self = Self(0);
    pub const BLOCK_START: Self = Self(1 << 0);
    pub const BLOCK_END: Self = Self(1 << 1);
    pub const INLINE_START: Self = Self(1 << 2);
    pub const INLINE_END: Self = Self(1 << 3);
    pub const BLOCK: Self = Self(Self::BLOCK_START.0 | Self::BLOCK_END.0);
    pub const INLINE: Self = Self(Self::INLINE_START.0 | Self::INLINE_END.0);
    pub const ALL: Self = Self(Self::BLOCK.0 | Self::INLINE.0);

    pub const fn contains(self, value: Self) -> bool {
        self.0 & value.0 == value.0
    }
}

impl BitOr for MarginTrim {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MarginTrim {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Independent CSS containment axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Containment(pub u8);

impl Containment {
    pub const NONE: Self = Self(0);
    pub const SIZE: Self = Self(1 << 0);
    pub const INLINE_SIZE: Self = Self(1 << 1);
    pub const LAYOUT: Self = Self(1 << 2);
    pub const STYLE: Self = Self(1 << 3);
    pub const PAINT: Self = Self(1 << 4);
    pub const CONTENT: Self = Self(Self::LAYOUT.0 | Self::STYLE.0 | Self::PAINT.0);
    pub const STRICT: Self = Self(Self::SIZE.0 | Self::CONTENT.0);

    pub const fn contains(self, value: Self) -> bool {
        self.0 & value.0 == value.0
    }

    pub const fn has_size_containment(self) -> bool {
        self.contains(Self::SIZE) || self.contains(Self::INLINE_SIZE)
    }
}

impl BitOr for Containment {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Containment {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentVisibility {
    #[default]
    Visible,
    Auto,
    Hidden,
}

#[derive(Debug, Clone, Copy)]
pub struct ContainIntrinsicLength {
    /// `auto <length>` remembers a previously rendered size when available.
    pub auto: bool,
    /// `None` is the `none` keyword.
    pub fallback: Option<Length>,
}

impl ContainIntrinsicLength {
    pub const NONE: Self = Self {
        auto: false,
        fallback: None,
    };

    pub const fn length(value: Length) -> Self {
        Self {
            auto: false,
            fallback: Some(value),
        }
    }

    pub const fn auto_length(value: Length) -> Self {
        Self {
            auto: true,
            fallback: Some(value),
        }
    }
}

impl Default for ContainIntrinsicLength {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerType {
    #[default]
    Normal,
    InlineSize,
    Size,
}

/// Placement of the generated `::scroll-marker-group` box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollMarkerGroup {
    #[default]
    None,
    Before,
    After,
}

/// Whether anchors in this element establish a passive scroll-target group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollTargetGroup {
    #[default]
    None,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerAxis {
    Width,
    Height,
    InlineSize,
    BlockSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerComparison {
    LessThan,
    LessThanOrEqual,
    Equal,
    GreaterThanOrEqual,
    GreaterThan,
}

#[derive(Debug, Clone)]
pub enum ContainerCondition {
    Feature {
        axis: ContainerAxis,
        comparison: ContainerComparison,
        value: Length,
    },
    And(Vec<ContainerCondition>),
    Or(Vec<ContainerCondition>),
    Not(Box<ContainerCondition>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectFit {
    #[default]
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectPosition {
    pub x: BackgroundPosition,
    pub y: BackgroundPosition,
}

impl Default for ObjectPosition {
    fn default() -> Self {
        Self {
            x: BackgroundPosition::center(),
            y: BackgroundPosition::center(),
        }
    }
}

/// General affine CSS 2D transform in Skia/CSS matrix order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Debug, Clone)]
pub enum ShapeOutside {
    None,
    MarginBox,
    BorderBox,
    PaddingBox,
    ContentBox,
    Inset {
        offsets: [Length; 4],
        radii: [(f32, f32); 4],
    },
    Circle {
        radius: Length,
        center_x: BackgroundPosition,
        center_y: BackgroundPosition,
    },
    Polygon(Vec<(Length, Length)>),
}

impl Default for ShapeOutside {
    fn default() -> Self {
        Self::None
    }
}

/// A deterministic sampled animation value. SP19 samples documents at 0 ms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationSnapshot {
    pub document_time_ms: f32,
    pub progress: f32,
}
