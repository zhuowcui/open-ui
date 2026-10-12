//! Actual-box transform resolution shared by painting and native geometry.

use crate::Fragment;
use openui_dom::Document;
use openui_style::{ComputedStyle, TransformMatrix};

/// An immutable used transform for one physical authored fragment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedFragmentTransform {
    pub matrix: TransformMatrix,
    /// Transform origin in this fragment's physical local coordinates.
    pub origin: (f64, f64),
    pub reference_box: (f64, f64),
}

impl ResolvedFragmentTransform {
    pub fn local_matrix(self) -> TransformMatrix {
        self.matrix.about_origin(self.origin.0, self.origin.1, 0.0)
    }
}

fn origin(length: openui_geometry::Length, size: f64) -> f64 {
    if length.is_fixed() {
        length.value() as f64
    } else if length.is_percent() {
        (length.value() * size as f32 / 100.0) as f64
    } else if length.is_calculated() {
        (length.calc_offset() + length.value() * size as f32 / 100.0) as f64
    } else {
        size * 0.5
    }
}

/// Resolve against the fragment's owned border box rather than a viewport,
/// stale layout, or caption/helper extent. Sliced block decorations retain
/// the source block dimension; each actual physical fragment owns its matrix.
pub fn resolved_fragment_transform(
    fragment: &Fragment,
    style: &ComputedStyle,
) -> ResolvedFragmentTransform {
    let border_box = fragment.border_box_rect();
    let mut width = border_box.size.width.to_f64();
    let mut height = border_box.size.height.to_f64();
    if let Some(block_size) = fragment.decoration_paint_block_size {
        let direction = fragment
            .fragmentation_writing_direction
            .unwrap_or_else(|| style.direction.writing_direction(style.writing_mode));
        if direction.is_horizontal() {
            height = block_size.to_f64();
        } else {
            width = block_size.to_f64();
        }
    }
    let matrix = style.transform_for_reference_box(width, height);
    ResolvedFragmentTransform {
        matrix,
        origin: (
            border_box.offset.left.to_f64() + origin(style.transform_origin.0, width),
            border_box.offset.top.to_f64() + origin(style.transform_origin.1, height),
        ),
        reference_box: (width, height),
    }
}

/// Call after final physical layout/sticky/snap resolution. Transform-only
/// updates refresh this metadata without another layout, while unchanged
/// frames keep the existing immutable fragment tree.
pub fn finalize_fragment_transforms(document: &Document, fragment: &mut Fragment) {
    fragment.resolved_transform = if fragment.owns_element_box() {
        Some(resolved_fragment_transform(
            fragment,
            &document.node(fragment.node_id).style,
        ))
    } else {
        None
    };
    for child in &mut fragment.children {
        finalize_fragment_transforms(document, child);
    }
}

/// Project one overflow quad with the pinned cc::MathUtil positive-w clipping.
/// Its float coordinate arithmetic and infinity bound are source rules, not
/// pixel tolerances; the owned transform itself remains a double matrix.
pub(crate) fn transformed_overflow_rect(
    matrix: TransformMatrix,
    rect: openui_geometry::PhysicalRect,
) -> openui_geometry::PhysicalRect {
    use openui_geometry::{LayoutUnit, PhysicalRect};
    const INFINITE: f32 = 1_000_000.0;
    fn cartesian(p: [f32; 4]) -> [f32; 2] {
        if p[3] == 1.0 {
            return [p[0], p[1]];
        }
        let inv = 1.0 / p[3];
        [
            (p[0] * inv).clamp(-INFINITE, INFINITE),
            (p[1] * inv).clamp(-INFINITE, INFINITE),
        ]
    }
    fn edge(a: [f32; 4], b: [f32; 4]) -> [f32; 2] {
        let t = a[3] / (a[3] - b[3]);
        std::array::from_fn(|i| {
            // Exact stationary-edge decision from cc::homogeneousLimitAtZero.
            if (a[i] * b[3] / a[3] / b[i] - 1.0).abs() > 0.00001 {
                if (1.0 - t) * a[i] + t * b[i] > 0.0 {
                    INFINITE
                } else {
                    -INFINITE
                }
            } else {
                a[i] / a[3]
            }
        })
    }
    let corners = [
        (rect.x(), rect.y()),
        (rect.right(), rect.y()),
        (rect.right(), rect.bottom()),
        (rect.x(), rect.bottom()),
    ];
    let homogeneous = corners.map(|(x, y)| {
        matrix
            .map_homogeneous([x.to_f32() as f64, y.to_f32() as f64, 0.0, 1.0])
            .map(|v| v as f32)
    });
    if homogeneous.iter().all(|p| p[3] <= 0.0) {
        return PhysicalRect::from_xywh(
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
    }
    let mut bounds = [f32::MAX, f32::MAX, -f32::MAX, -f32::MAX];
    let mut include = |p: [f32; 2]| {
        bounds[0] = bounds[0].min(p[0]);
        bounds[1] = bounds[1].min(p[1]);
        bounds[2] = bounds[2].max(p[0]);
        bounds[3] = bounds[3].max(p[1]);
    };
    for i in 0..4 {
        let a = homogeneous[i];
        let b = homogeneous[(i + 1) % 4];
        if a[3] > 0.0 {
            include(cartesian(a));
        }
        if (a[3] <= 0.0) != (b[3] <= 0.0) {
            include(edge(a, b));
        }
    }
    PhysicalRect::from_xywh(
        LayoutUnit::from_f32(bounds[0]),
        LayoutUnit::from_f32(bounds[1]),
        LayoutUnit::from_f32(bounds[2] - bounds[0]),
        LayoutUnit::from_f32(bounds[3] - bounds[1]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::{LayoutUnit, PhysicalRect};
    fn rect(x: i32, y: i32, w: i32, h: i32) -> PhysicalRect {
        PhysicalRect::from_xywh(
            LayoutUnit::from_i32(x),
            LayoutUnit::from_i32(y),
            LayoutUnit::from_i32(w),
            LayoutUnit::from_i32(h),
        )
    }
    #[test]
    fn projective_overflow_uses_full_matrix_and_positive_w_edges() {
        let mut perspective = TransformMatrix::IDENTITY;
        perspective.0[3] = -0.01;
        assert_eq!(
            transformed_overflow_rect(perspective, rect(0, 0, 50, 20)),
            rect(0, 0, 100, 40)
        );
        assert_eq!(
            transformed_overflow_rect(perspective, rect(150, 0, 20, 20)),
            rect(0, 0, 0, 0)
        );
        let crossing = transformed_overflow_rect(perspective, rect(50, 0, 100, 20));
        assert_eq!(crossing.right(), LayoutUnit::from_i32(1_000_000));
        assert_eq!(crossing.bottom(), LayoutUnit::from_i32(1_000_000));
    }
}
