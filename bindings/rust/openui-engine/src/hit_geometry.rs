//! Full retained transforms for native geometry and element hit testing.
//!
//! Float geometry boundaries follow pinned Chromium 147.0.7727.50 ui/gfx.
//! The retained matrix remains double precision; clipping and hit coordinates
//! use inverse plane projection after checking the full matrix is invertible.

use openui_compositor::SceneRect;
use openui_geometry::LayoutUnit;
use openui_style::AppliedTransform;
#[cfg(test)]
use openui_style::TransformMatrix;

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldTransform(pub(crate) AppliedTransform);

fn clamp_geometry(value: f64) -> f32 {
    // gfx::FloatGeometrySaturationHandler<float>::max() divides by double 1e6
    // and stores float. Keep that boundary rather than using f32 division.
    const LIMIT: f32 = (f32::MAX as f64 / 1_000_000.0) as f32;
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-(LIMIT as f64), LIMIT as f64) as f32
    }
}

pub(crate) fn enclosing_layout_rect(rect: SceneRect) -> SceneRect {
    // PhysicalRect::EnclosingRect uses fixed-point floor/ceil of the four
    // float edges. This is a geometry operation; raster equality stays exact.
    let x = LayoutUnit::from_f32_floor(rect.x);
    let y = LayoutUnit::from_f32_floor(rect.y);
    let right = LayoutUnit::from_f32_ceil(rect.x + rect.width);
    let bottom = LayoutUnit::from_f32_ceil(rect.y + rect.height);
    SceneRect {
        x: x.to_f32(),
        y: y.to_f32(),
        width: (right - x).to_f32(),
        height: (bottom - y).to_f32(),
    }
}

impl WorldTransform {
    pub(crate) const IDENTITY: Self = Self(AppliedTransform::IDENTITY);

    #[cfg(test)]
    fn from_matrix(matrix: TransformMatrix) -> Self {
        Self(AppliedTransform::from_matrix(matrix))
    }

    pub(crate) fn translate(x: f64, y: f64) -> Self {
        Self(AppliedTransform::translation(x as f32, y as f32))
    }

    pub(crate) fn then(self, rhs: Self) -> Self {
        Self(self.0.concat(rhs.0))
    }

    pub(crate) fn is_invertible(self) -> bool {
        self.0.is_invertible()
    }

    pub(crate) fn inverse(self) -> Option<Self> {
        self.0.inverse().map(Self)
    }

    /// The matrix at the next flat element boundary. An ancestor's original
    /// full invertibility remains a separate input eligibility condition.
    pub(crate) fn flattened(self) -> Self {
        Self(self.0.flattened())
    }

    /// Forward MapPointInternal for client rectangles. Unlike raster clipping,
    /// this geometry operation divides by a normal negative w as well.
    fn map(self, x: f32, y: f32) -> (f32, f32) {
        if let Some(axis) = self.0.axis_2d {
            // AxisTransform2d::MapPoint performs each product and sum in
            // float. Its source representation survives matrix identity.
            return (
                clamp_geometry((x * axis.scale[0] + axis.translation[0]) as f64),
                clamp_geometry((y * axis.scale[1] + axis.translation[1]) as f64),
            );
        }
        let m = self.0.matrix.0;
        let (x, y) = (x as f64, y as f64);
        // MapVector2 omits depth rather than adding a zero depth product.
        let px = x * m[0] + y * m[4] + m[12];
        let py = x * m[1] + y * m[5] + m[13];
        let w = x * m[3] + y * m[7] + m[15];
        if w != 1.0 && w.is_normal() {
            let reciprocal = 1.0 / w;
            (
                clamp_geometry(px * reciprocal),
                clamp_geometry(py * reciprocal),
            )
        } else {
            (clamp_geometry(px), clamp_geometry(py))
        }
    }

    /// ProjectPoint on an already inverted full matrix, as used by Blink's
    /// HitTestingTransformState::MappedPoint. The boolean reports clamping for
    /// ProjectQuad; point hit testing uses the projected coordinates directly.
    fn project(self, x: f32, y: f32) -> ((f32, f32), bool) {
        if self.0.axis_2d.is_some() {
            return (self.map(x, y), false);
        }
        let m = self.0.matrix.0;
        if !m[10].is_normal() {
            return ((0.0, 0.0), true);
        }
        let (x, y) = (x as f64, y as f64);
        let z = -(m[2] * x + m[6] * y + m[14]) / m[10];
        if !z.is_finite() {
            return ((0.0, 0.0), true);
        }
        let p = self.0.matrix.map_homogeneous([x, y, z, 1.0]);
        if p[3] <= 0.0 {
            const INFINITE_PROJECTION: f64 = (1_u32 << 23) as f64;
            return (
                (
                    INFINITE_PROJECTION.copysign(p[0]) as f32,
                    INFINITE_PROJECTION.copysign(p[1]) as f32,
                ),
                true,
            );
        }
        let (px, py) = if p[3] != 1.0 {
            (p[0] / p[3], p[1] / p[3])
        } else {
            (p[0], p[1])
        };
        ((clamp_geometry(px), clamp_geometry(py)), false)
    }

    pub(crate) fn project_point(self, x: f32, y: f32) -> (f32, f32) {
        self.project(x, y).0
    }

    pub(crate) fn map_rect(self, width: f32, height: f32) -> SceneRect {
        // Blink's element client/bounding geometry maps an absolute quad and
        // takes its bounding box, including for an optimized Axis transform.
        // gfx::Transform::MapRect's dimension shortcut belongs to other APIs.
        bounds([
            self.map(0.0, 0.0),
            self.map(width, 0.0),
            self.map(width, height),
            self.map(0.0, height),
        ])
    }

    pub(crate) fn project_rect(self, rect: SceneRect) -> SceneRect {
        let projected = [
            self.project(rect.x, rect.y),
            self.project(rect.x + rect.width, rect.y),
            self.project(rect.x + rect.width, rect.y + rect.height),
            self.project(rect.x, rect.y + rect.height),
        ];
        if projected.iter().all(|(_, clamped)| *clamped) {
            // gfx::Transform::ProjectQuad returns an empty quad here.
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }
        } else {
            bounds(projected.map(|(point, _)| point))
        }
    }
}

fn bounds(points: [(f32, f32); 4]) -> SceneRect {
    let x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let right = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let bottom = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    SceneRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_and_explicit_matrix_keep_distinct_point_arithmetic() {
        let axis = AppliedTransform::from_axis(openui_style::AxisTransform2d {
            scale: [0.1, 1.0],
            translation: [-3.0, 0.0],
        });
        let point = WorldTransform(axis).project_point(29.0, 1.0);
        let full = WorldTransform::from_matrix(axis.matrix).project_point(29.0, 1.0);
        assert_eq!(point.0.to_bits(), (-0.099_999_905_f32).to_bits());
        assert_eq!(full.0.to_bits(), (-0.099_999_96_f32).to_bits());
        assert_ne!(point.0.to_bits(), full.0.to_bits());
    }

    #[test]
    fn client_geometry_keeps_the_absolute_quad_path_for_axis_translation() {
        let world = WorldTransform::translate(16_777_216.0, 0.0);
        let rect = world.map_rect(3.25, 1.0);
        assert_eq!(rect.x, 16_777_216.0);
        assert_eq!(rect.width, 4.0);
    }

    #[test]
    fn inverse_reveal_encloses_fractional_edges_in_fixed_point() {
        assert_eq!(
            enclosing_layout_rect(SceneRect {
                x: 0.1,
                y: -0.1,
                width: 0.1,
                height: 0.1,
            }),
            SceneRect {
                x: 6.0 / 64.0,
                y: -7.0 / 64.0,
                width: 7.0 / 64.0,
                height: 7.0 / 64.0,
            }
        );
    }

    #[test]
    fn full_singular_depth_is_rejected_before_plane_projection() {
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[10] = 0.0;
        let world = WorldTransform::from_matrix(matrix);
        assert!(!world.is_invertible());
        assert!(world.inverse().is_none());
        // Flattening a matrix does not revive its CSS subtree eligibility.
        assert!(world.flattened().is_invertible());
    }

    #[test]
    fn tilted_plane_uses_projected_inverse_instead_of_zero_depth_map() {
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[0] = 0.5;
        matrix.0[2] = -0.75;
        matrix.0[8] = 0.75;
        matrix.0[10] = 0.5;
        let inverse = WorldTransform::from_matrix(matrix).inverse().unwrap();
        assert_eq!(inverse.project_point(20.0, 7.0), (40.0, 7.0));
        assert_ne!(inverse.map(20.0, 7.0), (40.0, 7.0));
    }

    #[test]
    fn projective_plane_retains_homogeneous_position() {
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[3] = 0.125;
        let world = WorldTransform::from_matrix(matrix);
        assert_eq!(world.map(8.0, 4.0), (4.0, 2.0));
        assert_eq!(world.inverse().unwrap().project_point(4.0, 2.0), (8.0, 4.0));
    }

    #[test]
    fn projection_clamp_and_forward_geometry_are_distinct_source_operations() {
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[15] = -1.0;
        let transform = WorldTransform::from_matrix(matrix);
        assert_eq!(transform.map(4.0, 2.0), (-4.0, -2.0));
        assert_eq!(
            transform.project_point(4.0, 2.0),
            (8_388_608.0, 8_388_608.0)
        );
        assert_eq!(
            transform.project_rect(SceneRect {
                x: 1.0,
                y: 1.0,
                width: 2.0,
                height: 2.0
            }),
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0
            }
        );
        matrix.0[10] = 0.0;
        assert_eq!(
            WorldTransform::from_matrix(matrix).project_point(4.0, 2.0),
            (0.0, 0.0)
        );
    }

    #[test]
    fn flat_boundary_prevents_child_depth_from_entering_parent_plane() {
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[8] = 2.0;
        matrix.0[2] = 3.0;
        let parent = WorldTransform::from_matrix(matrix);
        let child = WorldTransform::from_matrix(TransformMatrix::translation(0.0, 0.0, 5.0));
        assert_eq!(parent.then(child).map(0.0, 0.0), (10.0, 0.0));
        assert_eq!(parent.flattened().then(child).map(0.0, 0.0), (0.0, 0.0));
    }
}
