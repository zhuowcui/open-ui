//! Actual-box transform resolution shared by painting and native geometry.

use crate::Fragment;
use openui_dom::Document;
use openui_style::{
    transform_list_applied, AppliedTransform, ComputedStyle, TransformList, TransformMatrix,
    TransformOperation,
};

/// An immutable used transform for one physical authored fragment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedFragmentTransform {
    pub matrix: TransformMatrix,
    /// Source representation and origin applied through the actual operation loop.
    pub local_applied: AppliedTransform,
    /// Transform origin in this fragment's physical local coordinates.
    pub origin: (f64, f64),
    pub reference_box: (f64, f64),
}

impl ResolvedFragmentTransform {
    pub fn local_matrix(self) -> TransformMatrix {
        self.local_applied.matrix
    }
}

fn origin(length: openui_geometry::Length, size: f64) -> f64 {
    if length.is_fixed() || length.is_percent() || length.is_calculated() {
        openui_style::transform_length_for_reference_box(length, size as f32) as f64
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
    // Blink stores each origin coordinate in float, including addition of
    // the owned reference-box offset, before it applies the operation list.
    let origin = (
        (border_box.offset.left.to_f32() + origin(style.transform_origin.0, width) as f32) as f64,
        (border_box.offset.top.to_f32() + origin(style.transform_origin.1, height) as f32) as f64,
    );
    let local_applied = if let Some(list) = &style.native_transform {
        transform_list_applied(list, (width, height), Some((origin.0, origin.1, 0.0)))
    } else if style.has_transform() {
        // The compatibility affine field is an explicit matrix source. It
        // must not acquire Axis arithmetic from the matrix's component bytes.
        transform_list_applied(
            &TransformList(vec![TransformOperation::Matrix(style.transform)]),
            (width, height),
            Some((origin.0, origin.1, 0.0)),
        )
    } else {
        AppliedTransform::IDENTITY
    };
    ResolvedFragmentTransform {
        matrix,
        local_applied,
        origin,
        reference_box: (width, height),
    }
}

/// Apply a removed ancestor about its retained owned physical reference box
/// in the promoted parent's coordinate space. Both painting and native
/// geometry consume this same source descriptor and kind-sensitive origin.
pub fn promoted_ancestor_applied_transform(
    document: &Document,
    ancestor: &crate::PromotedTransformAncestor,
) -> AppliedTransform {
    let style = &document.node(ancestor.node_id).style;
    if !style.has_transform() {
        return AppliedTransform::IDENTITY;
    }
    let rect = ancestor.fragment_reference_box;
    let mut width = rect.width().to_f64();
    let mut height = rect.height().to_f64();
    if let Some(block) = ancestor.reference_block_size {
        if ancestor.reference_block_axis_is_x {
            width = block.to_f64();
        } else {
            height = block.to_f64();
        }
    }
    let local_origin = (
        (rect.x().to_f32() + origin(style.transform_origin.0, width) as f32) as f64,
        (rect.y().to_f32() + origin(style.transform_origin.1, height) as f32) as f64,
        0.0,
    );
    let legacy;
    let list = if let Some(list) = &style.native_transform {
        list
    } else {
        legacy = TransformList(vec![TransformOperation::Matrix(style.transform)]);
        &legacy
    };
    AppliedTransform::translation(
        ancestor.fragment_offset.left.to_f32(),
        ancestor.fragment_offset.top.to_f32(),
    )
    .concat(transform_list_applied(
        list,
        (width, height),
        Some(local_origin),
    ))
    .concat(AppliedTransform::translation(
        -ancestor.fragment_offset.left.to_f32(),
        -ancestor.fragment_offset.top.to_f32(),
    ))
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
    applied: AppliedTransform,
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
        if let Some(axis) = applied.axis_2d {
            [
                x.to_f32() * axis.scale[0] + axis.translation[0],
                y.to_f32() * axis.scale[1] + axis.translation[1],
                0.0,
                1.0,
            ]
        } else {
            applied
                .matrix
                .map_homogeneous([x.to_f32() as f64, y.to_f32() as f64, 0.0, 1.0])
                .map(|v| v as f32)
        }
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
    fn removed_ancestor_flat_boundary_retains_planar_perspective_and_rejects_full_singularity() {
        let mut document = Document::new();
        let node = document.create_node(openui_dom::ElementTag::Div);
        document.node_mut(node).style = ComputedStyle::initial().derive(|style| {
            style.native_transform = Some(TransformList(vec![
                TransformOperation::Perspective(openui_style::LengthValue::px(500.0)),
                TransformOperation::Rotate3d {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    degrees: 45.0,
                },
            ]));
            style.transform_origin = (
                openui_geometry::Length::fixed(0.0),
                openui_geometry::Length::fixed(0.0),
            );
        });
        let size =
            openui_geometry::PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(40));
        let owned = rect(0, 0, 100, 40);
        let ancestor = crate::PromotedTransformAncestor {
            node_id: node,
            source_offset: openui_geometry::PhysicalOffset::zero(),
            source_size: size,
            visual_offset: openui_geometry::PhysicalOffset::zero(),
            fragment_offset: openui_geometry::PhysicalOffset::zero(),
            fragment_size: size,
            source_reference_box: owned,
            fragment_reference_box: owned,
            reference_block_size: None,
            reference_block_axis_is_x: false,
        };
        let parent = promoted_ancestor_applied_transform(&document, &ancestor);
        let child = transform_list_applied(
            &TransformList(vec![TransformOperation::Rotate3d {
                x: 0.0,
                y: 1.0,
                z: 0.0,
                degrees: -45.0,
            }]),
            (100.0, 40.0),
            None,
        );
        assert!(parent.is_invertible());
        let flat = parent.flattened();
        assert_eq!(flat.matrix.0[8], 0.0);
        assert_eq!(flat.matrix.0[2], 0.0);
        assert_eq!(flat.matrix.0[3], parent.matrix.0[3]);
        assert_ne!(
            flat.concat(child).matrix.map_point(100.0, 0.0, 0.0),
            parent.concat(child).matrix.map_point(100.0, 0.0, 0.0)
        );
        document.node_mut(node).style = ComputedStyle::initial().derive(|style| {
            style.native_transform = Some(TransformList(vec![TransformOperation::Scale3d(
                1.0, 1.0, 0.0,
            )]))
        });
        let singular = promoted_ancestor_applied_transform(&document, &ancestor);
        assert!(!singular.is_invertible());
        assert!(singular.flattened().is_invertible());
        // Callers must latch the first result before following a flat child.
    }

    #[test]
    fn promoted_ancestor_uses_owned_caption_reference_and_retained_continuation_slice() {
        let mut document = Document::new();
        let node = document.create_node(openui_dom::ElementTag::Table);
        document.node_mut(node).style = ComputedStyle::initial().derive(|style| {
            style.native_transform = Some(TransformList(vec![TransformOperation::Scale(2.0, 2.0)]))
        });
        let size =
            openui_geometry::PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(60));
        let owned = rect(0, 20, 100, 40);
        let mut ancestor = crate::PromotedTransformAncestor {
            node_id: node,
            source_offset: openui_geometry::PhysicalOffset::new(
                LayoutUnit::from_i32(30),
                LayoutUnit::from_i32(40),
            ),
            source_size: size,
            visual_offset: openui_geometry::PhysicalOffset::zero(),
            fragment_offset: openui_geometry::PhysicalOffset::new(
                LayoutUnit::from_i32(30),
                LayoutUnit::from_i32(40),
            ),
            fragment_size: size,
            source_reference_box: owned,
            fragment_reference_box: owned,
            reference_block_size: None,
            reference_block_axis_is_x: false,
        };
        let used = promoted_ancestor_applied_transform(&document, &ancestor);
        assert_eq!(used.axis_2d.unwrap().translation, [-80.0, -80.0]);
        ancestor.fragment_size.height = LayoutUnit::from_i32(20);
        ancestor.resolve_reference_slice(LayoutUnit::from_i32(20));
        assert_eq!(ancestor.fragment_reference_box, rect(0, 0, 100, 20));
        assert_eq!(
            promoted_ancestor_applied_transform(&document, &ancestor)
                .axis_2d
                .unwrap()
                .translation,
            [-80.0, -50.0]
        );
        ancestor.reference_block_size = Some(LayoutUnit::from_i32(40));
        assert_eq!(
            promoted_ancestor_applied_transform(&document, &ancestor)
                .axis_2d
                .unwrap()
                .translation,
            [-80.0, -60.0]
        );
        document.node_mut(node).style = ComputedStyle::initial().derive(|style| {
            style.native_transform = Some(TransformList(vec![TransformOperation::Matrix(
                openui_style::Transform2D::IDENTITY,
            )]))
        });
        assert!(promoted_ancestor_applied_transform(&document, &ancestor)
            .axis_2d
            .is_none());
        document.node_mut(node).style = ComputedStyle::initial();
        assert_eq!(
            promoted_ancestor_applied_transform(&document, &ancestor),
            AppliedTransform::IDENTITY
        );
    }

    #[test]
    fn physical_fragment_keeps_explicit_matrix_identity_distinct_from_transform_none() {
        let fragment = Fragment::new_box(
            openui_dom::NodeId::NONE,
            openui_geometry::PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(40)),
        );
        let style = ComputedStyle::initial();
        assert_eq!(
            resolved_fragment_transform(&fragment, &style).local_applied,
            AppliedTransform::IDENTITY
        );
        let explicit = style.derive(|style| {
            style.native_transform = Some(TransformList(vec![TransformOperation::Matrix(
                openui_style::Transform2D::IDENTITY,
            )]))
        });
        assert!(explicit.has_transform());
        let used = resolved_fragment_transform(&fragment, &explicit);
        assert_eq!(used.matrix, TransformMatrix::IDENTITY);
        assert_eq!(used.local_matrix(), TransformMatrix::IDENTITY);
        assert!(used.local_applied.axis_2d.is_none());
        let translate = style.derive(|style| {
            style.native_transform = Some(TransformList(vec![TransformOperation::Translate(
                openui_style::LengthValue::percent(50.0),
                openui_style::LengthValue::px(0.0),
            )]))
        });
        let used = resolved_fragment_transform(&fragment, &translate);
        assert_eq!(used.local_applied.axis_2d.unwrap().translation, [50.0, 0.0]);
        assert_eq!(used.local_matrix().0[12], 50.0);
    }

    #[test]
    fn projective_overflow_uses_full_matrix_and_positive_w_edges() {
        let mut perspective = TransformMatrix::IDENTITY;
        perspective.0[3] = -0.01;
        assert_eq!(
            transformed_overflow_rect(
                AppliedTransform::from_matrix(perspective),
                rect(0, 0, 50, 20)
            ),
            rect(0, 0, 100, 40)
        );
        assert_eq!(
            transformed_overflow_rect(
                AppliedTransform::from_matrix(perspective),
                rect(150, 0, 20, 20)
            ),
            rect(0, 0, 0, 0)
        );
        let crossing = transformed_overflow_rect(
            AppliedTransform::from_matrix(perspective),
            rect(50, 0, 100, 20),
        );
        assert_eq!(crossing.right(), LayoutUnit::from_i32(1_000_000));
        assert_eq!(crossing.bottom(), LayoutUnit::from_i32(1_000_000));
    }
}
