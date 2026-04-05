//! SP12 H1 — WPT Test Translation Infrastructure for Block Layout.
//!
//! Provides ergonomic builders and assertion helpers for translating WPT
//! (Web Platform Tests) and Blink layout tests into Rust unit/integration
//! tests targeting the block layout modules.
//!
//! # Usage
//!
//! ```ignore
//! use crate::sp12_wpt_helpers::*;
//!
//! let result = BlockTestBuilder::new(800, 600)
//!     .add_child()
//!         .width(200.0)
//!         .height(100.0)
//!         .margin(10, 20, 10, 20)
//!         .done()
//!     .add_child()
//!         .width(300.0)
//!         .float_left()
//!         .done()
//!     .build();
//!
//! result.assert_child_position(0, 20, 10);
//! result.assert_child_size(0, 200, 100);
//! ```

#![allow(dead_code)]

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::*;

// ── LayoutUnit helpers ───────────────────────────────────────────────────

/// Convert an `i32` pixel value to a `LayoutUnit`.
pub fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

/// Create a root `ConstraintSpace` with the given pixel dimensions.
pub fn root_space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu(w), lu(h))
}

// ── TestContent ──────────────────────────────────────────────────────────

/// Describes the content of a test child element.
#[derive(Clone)]
pub enum TestContent {
    /// No content — height determined by style.
    Empty,
    /// Replaced element with a fixed intrinsic size.
    FixedSize(PhysicalSize),
    /// Text content (the child gets explicit width/height from style).
    Text(String),
    /// Nested block children.
    Children(Vec<TestChild>),
}

// ── TestChild ────────────────────────────────────────────────────────────

/// A single child element in a block layout test scenario.
#[derive(Clone)]
pub struct TestChild {
    pub style: ComputedStyle,
    pub content: TestContent,
}

// ── ChildBuilder ─────────────────────────────────────────────────────────

/// Fluent builder for constructing a single child element.
pub struct ChildBuilder<'a> {
    parent: &'a mut BlockTestBuilder,
    style: ComputedStyle,
    content: TestContent,
    nested_children: Vec<TestChild>,
}

impl<'a> ChildBuilder<'a> {
    fn new(parent: &'a mut BlockTestBuilder) -> Self {
        let mut style = ComputedStyle::initial();
        style.display = Display::Block;
        Self {
            parent,
            style,
            content: TestContent::Empty,
            nested_children: Vec::new(),
        }
    }

    // ── Sizing ───────────────────────────────────────────────────────

    pub fn width(mut self, v: f32) -> Self {
        self.style.width = Length::px(v);
        self
    }

    pub fn height(mut self, v: f32) -> Self {
        self.style.height = Length::px(v);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style.min_width = Length::px(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style.min_height = Length::px(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style.max_width = Length::px(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style.max_height = Length::px(v);
        self
    }

    pub fn width_pct(mut self, v: f32) -> Self {
        self.style.width = Length::percent(v);
        self
    }

    pub fn height_pct(mut self, v: f32) -> Self {
        self.style.height = Length::percent(v);
        self
    }

    pub fn width_auto(mut self) -> Self {
        self.style.width = Length::auto();
        self
    }

    pub fn height_auto(mut self) -> Self {
        self.style.height = Length::auto();
        self
    }

    /// Set the child as a replaced element with a fixed intrinsic size.
    pub fn fixed_size(mut self, w: f32, h: f32) -> Self {
        self.style.width = Length::px(w);
        self.style.height = Length::px(h);
        self.content = TestContent::FixedSize(PhysicalSize::new(lu(w as i32), lu(h as i32)));
        self
    }

    // ── Box model ────────────────────────────────────────────────────

    /// Set all four margins (top, right, bottom, left) in pixels.
    pub fn margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.margin_top = Length::px(top as f32);
        self.style.margin_right = Length::px(right as f32);
        self.style.margin_bottom = Length::px(bottom as f32);
        self.style.margin_left = Length::px(left as f32);
        self
    }

    /// Set margin-top only.
    pub fn margin_top(mut self, v: i32) -> Self {
        self.style.margin_top = Length::px(v as f32);
        self
    }

    /// Set margin-bottom only.
    pub fn margin_bottom(mut self, v: i32) -> Self {
        self.style.margin_bottom = Length::px(v as f32);
        self
    }

    /// Set horizontal margins to `auto` (for centering).
    pub fn margin_auto_horizontal(mut self) -> Self {
        self.style.margin_left = Length::auto();
        self.style.margin_right = Length::auto();
        self
    }

    /// Set all four paddings (top, right, bottom, left) in pixels.
    pub fn padding(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.padding_top = Length::px(top as f32);
        self.style.padding_right = Length::px(right as f32);
        self.style.padding_bottom = Length::px(bottom as f32);
        self.style.padding_left = Length::px(left as f32);
        self
    }

    /// Set all four border widths (top, right, bottom, left) in pixels.
    /// Also sets border-style to `solid` so the widths take effect.
    pub fn border(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.border_top_width = top;
        self.style.border_right_width = right;
        self.style.border_bottom_width = bottom;
        self.style.border_left_width = left;
        self.style.border_top_style = BorderStyle::Solid;
        self.style.border_right_style = BorderStyle::Solid;
        self.style.border_bottom_style = BorderStyle::Solid;
        self.style.border_left_style = BorderStyle::Solid;
        self
    }

    // ── Positioning & float ──────────────────────────────────────────

    pub fn float(mut self, f: Float) -> Self {
        self.style.float = f;
        self
    }

    pub fn float_left(mut self) -> Self {
        self.style.float = Float::Left;
        self
    }

    pub fn float_right(mut self) -> Self {
        self.style.float = Float::Right;
        self
    }

    pub fn clear(mut self, c: Clear) -> Self {
        self.style.clear = c;
        self
    }

    pub fn clear_left(mut self) -> Self {
        self.style.clear = Clear::Left;
        self
    }

    pub fn clear_right(mut self) -> Self {
        self.style.clear = Clear::Right;
        self
    }

    pub fn clear_both(mut self) -> Self {
        self.style.clear = Clear::Both;
        self
    }

    pub fn position(mut self, p: Position) -> Self {
        self.style.position = p;
        self
    }

    pub fn position_relative(mut self) -> Self {
        self.style.position = Position::Relative;
        self
    }

    pub fn position_absolute(mut self) -> Self {
        self.style.position = Position::Absolute;
        self
    }

    /// Set inset properties (top, right, bottom, left) for positioned elements.
    pub fn inset(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.top = Length::px(top as f32);
        self.style.right = Length::px(right as f32);
        self.style.bottom = Length::px(bottom as f32);
        self.style.left = Length::px(left as f32);
        self
    }

    // ── Display & overflow ───────────────────────────────────────────

    pub fn display(mut self, d: Display) -> Self {
        self.style.display = d;
        self
    }

    pub fn overflow(mut self, o: Overflow) -> Self {
        self.style.overflow_x = o;
        self.style.overflow_y = o;
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.style.overflow_x = Overflow::Hidden;
        self.style.overflow_y = Overflow::Hidden;
        self
    }

    pub fn box_sizing_border_box(mut self) -> Self {
        self.style.box_sizing = BoxSizing::BorderBox;
        self
    }

    // ── Content ──────────────────────────────────────────────────────

    pub fn text(mut self, t: &str) -> Self {
        self.content = TestContent::Text(t.to_string());
        self
    }

    /// Apply an arbitrary style mutation via a closure.
    pub fn with_style(mut self, f: impl FnOnce(&mut ComputedStyle)) -> Self {
        f(&mut self.style);
        self
    }

    // ── Nested children ──────────────────────────────────────────────

    /// Add a nested child. Returns a `NestedChildBuilder` that chains
    /// back to this `ChildBuilder` on `done()`.
    pub fn add_child(self) -> NestedChildBuilder<'a> {
        NestedChildBuilder::new(self)
    }

    /// Finish this child and return to the parent builder.
    pub fn done(self) -> &'a mut BlockTestBuilder {
        let content = if self.nested_children.is_empty() {
            self.content
        } else {
            TestContent::Children(self.nested_children)
        };
        self.parent.children.push(TestChild {
            style: self.style,
            content,
        });
        self.parent
    }
}

// ── NestedChildBuilder ───────────────────────────────────────────────────

/// Builder for a child nested inside another child (one level deep).
pub struct NestedChildBuilder<'a> {
    parent_builder: ChildBuilder<'a>,
    style: ComputedStyle,
    content: TestContent,
}

impl<'a> NestedChildBuilder<'a> {
    fn new(parent_builder: ChildBuilder<'a>) -> Self {
        let mut style = ComputedStyle::initial();
        style.display = Display::Block;
        Self {
            parent_builder,
            style,
            content: TestContent::Empty,
        }
    }

    pub fn width(mut self, v: f32) -> Self {
        self.style.width = Length::px(v);
        self
    }

    pub fn height(mut self, v: f32) -> Self {
        self.style.height = Length::px(v);
        self
    }

    pub fn fixed_size(mut self, w: f32, h: f32) -> Self {
        self.style.width = Length::px(w);
        self.style.height = Length::px(h);
        self.content = TestContent::FixedSize(PhysicalSize::new(lu(w as i32), lu(h as i32)));
        self
    }

    pub fn margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.margin_top = Length::px(top as f32);
        self.style.margin_right = Length::px(right as f32);
        self.style.margin_bottom = Length::px(bottom as f32);
        self.style.margin_left = Length::px(left as f32);
        self
    }

    pub fn padding(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.padding_top = Length::px(top as f32);
        self.style.padding_right = Length::px(right as f32);
        self.style.padding_bottom = Length::px(bottom as f32);
        self.style.padding_left = Length::px(left as f32);
        self
    }

    pub fn border(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.border_top_width = top;
        self.style.border_right_width = right;
        self.style.border_bottom_width = bottom;
        self.style.border_left_width = left;
        self.style.border_top_style = BorderStyle::Solid;
        self.style.border_right_style = BorderStyle::Solid;
        self.style.border_bottom_style = BorderStyle::Solid;
        self.style.border_left_style = BorderStyle::Solid;
        self
    }

    pub fn float_left(mut self) -> Self {
        self.style.float = Float::Left;
        self
    }

    pub fn float_right(mut self) -> Self {
        self.style.float = Float::Right;
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.style.overflow_x = Overflow::Hidden;
        self.style.overflow_y = Overflow::Hidden;
        self
    }

    pub fn display(mut self, d: Display) -> Self {
        self.style.display = d;
        self
    }

    pub fn with_style(mut self, f: impl FnOnce(&mut ComputedStyle)) -> Self {
        f(&mut self.style);
        self
    }

    /// Finish this nested child and return to the parent child builder.
    pub fn done(mut self) -> ChildBuilder<'a> {
        self.parent_builder.nested_children.push(TestChild {
            style: self.style,
            content: self.content,
        });
        self.parent_builder
    }
}

// ── BlockTestBuilder ─────────────────────────────────────────────────────

/// Fluent builder for constructing block layout test scenarios.
///
/// Creates a `Document`, builds the DOM tree from the builder state,
/// runs `block_layout`, and returns a `LayoutTestResult` for assertions.
pub struct BlockTestBuilder {
    container_style: ComputedStyle,
    container_width: i32,
    container_height: i32,
    children: Vec<TestChild>,
}

impl BlockTestBuilder {
    /// Create a new builder with the given container (viewport) dimensions.
    pub fn new(width: i32, height: i32) -> Self {
        let mut style = ComputedStyle::initial();
        style.display = Display::Block;
        style.width = Length::px(width as f32);
        style.height = Length::px(height as f32);
        Self {
            container_style: style,
            container_width: width,
            container_height: height,
            children: Vec::new(),
        }
    }

    /// Override the container style entirely.
    pub fn container_style(mut self, style: ComputedStyle) -> Self {
        self.container_style = style;
        self
    }

    /// Mutate the container style via a closure.
    pub fn with_container_style(mut self, f: impl FnOnce(&mut ComputedStyle)) -> Self {
        f(&mut self.container_style);
        self
    }

    /// Start adding a child element. Returns a `ChildBuilder`.
    pub fn add_child(&mut self) -> ChildBuilder<'_> {
        ChildBuilder::new(self)
    }

    /// Build the DOM, run block layout, and return a `LayoutTestResult`.
    pub fn build(self) -> LayoutTestResult {
        let mut doc = Document::new();
        let vp = doc.root();

        // Create the container node under the viewport.
        let container = doc.create_node(ElementTag::Div);
        {
            let node = doc.node_mut(container);
            node.style = self.container_style;
        }
        doc.append_child(vp, container);

        // Recursively build children.
        fn build_children(doc: &mut Document, parent: NodeId, children: &[TestChild]) {
            for child in children {
                let node_id = doc.create_node(ElementTag::Div);
                {
                    let node = doc.node_mut(node_id);
                    node.style = child.style.clone();
                }
                doc.append_child(parent, node_id);

                if let TestContent::Children(ref nested) = child.content {
                    build_children(doc, node_id, nested);
                }
            }
        }

        build_children(&mut doc, container, &self.children);

        // Run layout.
        let space = root_space(self.container_width, self.container_height);
        let root_fragment = block_layout(&doc, vp, &space);

        LayoutTestResult {
            root_fragment,
        }
    }
}

// ── LayoutTestResult ─────────────────────────────────────────────────────

/// Wraps the layout result with ergonomic assertion helpers.
pub struct LayoutTestResult {
    /// The full viewport-level fragment (container is `root_fragment.children[0]`).
    pub root_fragment: Fragment,
}

impl LayoutTestResult {
    /// The container fragment (first child of root viewport).
    pub fn container(&self) -> &Fragment {
        &self.root_fragment.children[0]
    }

    /// Access a child fragment by index.
    pub fn child(&self, index: usize) -> &Fragment {
        &self.container().children[index]
    }

    /// Get the offset (position) of a child fragment.
    pub fn child_offset(&self, index: usize) -> PhysicalOffset {
        self.child(index).offset
    }

    /// Get the size of a child fragment.
    pub fn child_size(&self, index: usize) -> PhysicalSize {
        self.child(index).size
    }

    /// Number of children in the container fragment.
    pub fn child_count(&self) -> usize {
        self.container().children.len()
    }

    /// The size of the container fragment.
    pub fn container_size(&self) -> PhysicalSize {
        self.container().size
    }

    /// Assert that a child is at the expected (x, y) position.
    pub fn assert_child_position(&self, index: usize, x: i32, y: i32) {
        let frag = self.child(index);
        assert_eq!(
            frag.offset.left,
            lu(x),
            "child[{}] left: expected {}, got {}",
            index,
            x,
            frag.offset.left.to_i32()
        );
        assert_eq!(
            frag.offset.top,
            lu(y),
            "child[{}] top: expected {}, got {}",
            index,
            y,
            frag.offset.top.to_i32()
        );
    }

    /// Assert that a child has the expected (w, h) size.
    pub fn assert_child_size(&self, index: usize, w: i32, h: i32) {
        let frag = self.child(index);
        assert_eq!(
            frag.size.width,
            lu(w),
            "child[{}] width: expected {}, got {}",
            index,
            w,
            frag.size.width.to_i32()
        );
        assert_eq!(
            frag.size.height,
            lu(h),
            "child[{}] height: expected {}, got {}",
            index,
            h,
            frag.size.height.to_i32()
        );
    }

    /// Assert a child's full margin box: position (x, y) and size (w, h).
    pub fn assert_child_margin_box(&self, index: usize, x: i32, y: i32, w: i32, h: i32) {
        self.assert_child_position(index, x, y);
        self.assert_child_size(index, w, h);
    }

    /// Assert the container fragment's height.
    pub fn assert_container_height(&self, h: i32) {
        assert_eq!(
            self.container().size.height,
            lu(h),
            "container height: expected {}, got {}",
            h,
            self.container().size.height.to_i32()
        );
    }

    /// Assert the container fragment's width.
    pub fn assert_container_width(&self, w: i32) {
        assert_eq!(
            self.container().size.width,
            lu(w),
            "container width: expected {}, got {}",
            w,
            self.container().size.width.to_i32()
        );
    }

    /// Assert the total number of children.
    pub fn assert_child_count(&self, expected: usize) {
        assert_eq!(
            self.child_count(),
            expected,
            "child count: expected {}, got {}",
            expected,
            self.child_count()
        );
    }

    /// Access a nested child: `result.nested_child(0, 1)` gets child 0's child 1.
    pub fn nested_child(&self, parent_index: usize, child_index: usize) -> &Fragment {
        &self.child(parent_index).children[child_index]
    }

    /// Assert position of a nested child.
    pub fn assert_nested_child_position(
        &self,
        parent_index: usize,
        child_index: usize,
        x: i32,
        y: i32,
    ) {
        let frag = self.nested_child(parent_index, child_index);
        assert_eq!(
            frag.offset.left,
            lu(x),
            "child[{}].children[{}] left: expected {}, got {}",
            parent_index,
            child_index,
            x,
            frag.offset.left.to_i32()
        );
        assert_eq!(
            frag.offset.top,
            lu(y),
            "child[{}].children[{}] top: expected {}, got {}",
            parent_index,
            child_index,
            y,
            frag.offset.top.to_i32()
        );
    }

    /// Assert size of a nested child.
    pub fn assert_nested_child_size(
        &self,
        parent_index: usize,
        child_index: usize,
        w: i32,
        h: i32,
    ) {
        let frag = self.nested_child(parent_index, child_index);
        assert_eq!(
            frag.size.width,
            lu(w),
            "child[{}].children[{}] width: expected {}, got {}",
            parent_index,
            child_index,
            w,
            frag.size.width.to_i32()
        );
        assert_eq!(
            frag.size.height,
            lu(h),
            "child[{}].children[{}] height: expected {}, got {}",
            parent_index,
            child_index,
            h,
            frag.size.height.to_i32()
        );
    }
}

// ── StyleBuilder ─────────────────────────────────────────────────────────

/// Convenience builder for constructing a `ComputedStyle` with defaults.
///
/// Starts from `ComputedStyle::initial()` and provides chainable setters.
pub struct StyleBuilder {
    style: ComputedStyle,
}

/// Create a new `StyleBuilder` starting from initial CSS values.
pub fn style_builder() -> StyleBuilder {
    StyleBuilder {
        style: ComputedStyle::initial(),
    }
}

impl StyleBuilder {
    pub fn width(mut self, v: f32) -> Self {
        self.style.width = Length::px(v);
        self
    }

    pub fn height(mut self, v: f32) -> Self {
        self.style.height = Length::px(v);
        self
    }

    pub fn width_auto(mut self) -> Self {
        self.style.width = Length::auto();
        self
    }

    pub fn height_auto(mut self) -> Self {
        self.style.height = Length::auto();
        self
    }

    pub fn width_pct(mut self, v: f32) -> Self {
        self.style.width = Length::percent(v);
        self
    }

    pub fn height_pct(mut self, v: f32) -> Self {
        self.style.height = Length::percent(v);
        self
    }

    pub fn margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.margin_top = Length::px(top as f32);
        self.style.margin_right = Length::px(right as f32);
        self.style.margin_bottom = Length::px(bottom as f32);
        self.style.margin_left = Length::px(left as f32);
        self
    }

    pub fn padding(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.padding_top = Length::px(top as f32);
        self.style.padding_right = Length::px(right as f32);
        self.style.padding_bottom = Length::px(bottom as f32);
        self.style.padding_left = Length::px(left as f32);
        self
    }

    pub fn border_width(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.style.border_top_width = top;
        self.style.border_right_width = right;
        self.style.border_bottom_width = bottom;
        self.style.border_left_width = left;
        self.style.border_top_style = BorderStyle::Solid;
        self.style.border_right_style = BorderStyle::Solid;
        self.style.border_bottom_style = BorderStyle::Solid;
        self.style.border_left_style = BorderStyle::Solid;
        self
    }

    pub fn float_left(mut self) -> Self {
        self.style.float = Float::Left;
        self
    }

    pub fn float_right(mut self) -> Self {
        self.style.float = Float::Right;
        self
    }

    pub fn clear(mut self, c: Clear) -> Self {
        self.style.clear = c;
        self
    }

    pub fn clear_both(mut self) -> Self {
        self.style.clear = Clear::Both;
        self
    }

    pub fn position_relative(mut self) -> Self {
        self.style.position = Position::Relative;
        self
    }

    pub fn position_absolute(mut self) -> Self {
        self.style.position = Position::Absolute;
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.style.overflow_x = Overflow::Hidden;
        self.style.overflow_y = Overflow::Hidden;
        self
    }

    pub fn display(mut self, d: Display) -> Self {
        self.style.display = d;
        self
    }

    pub fn box_sizing_border_box(mut self) -> Self {
        self.style.box_sizing = BoxSizing::BorderBox;
        self
    }

    /// Apply an arbitrary style mutation via a closure.
    pub fn with(mut self, f: impl FnOnce(&mut ComputedStyle)) -> Self {
        f(&mut self.style);
        self
    }

    pub fn build(self) -> ComputedStyle {
        self.style
    }
}

// ── assert_layout! macro ─────────────────────────────────────────────────

/// Convenience macro for asserting layout results.
///
/// # Examples
///
/// ```ignore
/// assert_layout!(result, child(0) at (10, 20) size (200, 100));
/// assert_layout!(result, container height 300);
/// assert_layout!(result, child_count 3);
/// ```
#[macro_export]
macro_rules! assert_layout {
    ($result:expr, child($idx:expr) at ($x:expr, $y:expr) size ($w:expr, $h:expr)) => {
        $result.assert_child_margin_box($idx, $x, $y, $w, $h);
    };
    ($result:expr, child($idx:expr) at ($x:expr, $y:expr)) => {
        $result.assert_child_position($idx, $x, $y);
    };
    ($result:expr, child($idx:expr) size ($w:expr, $h:expr)) => {
        $result.assert_child_size($idx, $w, $h);
    };
    ($result:expr, container height $h:expr) => {
        $result.assert_container_height($h);
    };
    ($result:expr, container width $w:expr) => {
        $result.assert_container_width($w);
    };
    ($result:expr, child_count $n:expr) => {
        $result.assert_child_count($n);
    };
}
