//! Shared native transform operations and their actual-box application.
//!
//! Operation matching, matrix suffixes, and decomposition follow the pinned
//! Chromium 147.0.7727.50 transform implementation. Lengths retain their units
//! until the Engine resolves fonts and the layout supplies the reference box.
//! Matrix/quaternion arithmetic retains doubles until the raster boundary.
//!
//! The matrix/quaternion algorithms are derived from Chromium ui/gfx (BSD
//! license; Copyright 2013–2023 The Chromium Authors). Source receipts and the
//! operation-specific provenance are retained in the qualification evidence.

use crate::{LengthValue, Transform2D, Transform3D, TransformList, TransformOperation};

/// Full column-major transform used by retained geometry. The old affine
/// `Transform2D` field is a compatibility view, not the applied 3D transform.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformMatrix(pub [f64; 16]);

impl TransformMatrix {
    pub const IDENTITY: Self = Self([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    pub fn from_affine(m: Transform2D) -> Self {
        let mut out = Self::IDENTITY;
        out.0[0] = m.a as f64;
        out.0[1] = m.b as f64;
        out.0[4] = m.c as f64;
        out.0[5] = m.d as f64;
        out.0[12] = m.e as f64;
        out.0[13] = m.f as f64;
        out
    }

    pub fn compatibility_affine(self) -> Transform2D {
        Transform2D {
            a: self.0[0] as f32,
            b: self.0[1] as f32,
            c: self.0[4] as f32,
            d: self.0[5] as f32,
            e: self.0[12] as f32,
            f: self.0[13] as f32,
        }
    }

    pub fn raster_matrix(self) -> Transform3D {
        Transform3D(self.0.map(|v| v as f32))
    }

    pub fn is_2d(self) -> bool {
        let m = self.0;
        m[2] == 0.0
            && m[3] == 0.0
            && m[6] == 0.0
            && m[7] == 0.0
            && m[8] == 0.0
            && m[9] == 0.0
            && m[10] == 1.0
            && m[11] == 0.0
            && m[14] == 0.0
            && m[15] == 1.0
    }

    /// `self * rhs`: CSS list order, including transformed translations.
    pub fn concat(self, rhs: Self) -> Self {
        if self.is_2d() && rhs.is_2d() {
            let a = self.0;
            let b = rhs.0;
            let mut out = Self::IDENTITY;
            out.0[0] = a[0] * b[0] + a[4] * b[1];
            out.0[1] = a[1] * b[0] + a[5] * b[1];
            out.0[4] = a[0] * b[4] + a[4] * b[5];
            out.0[5] = a[1] * b[4] + a[5] * b[5];
            out.0[12] = a[0] * b[12] + a[4] * b[13] + a[12];
            out.0[13] = a[1] * b[12] + a[5] * b[13] + a[13];
            return out;
        }
        let mut out = Self::IDENTITY;
        for col in 0..4 {
            for row in 0..4 {
                out.0[col * 4 + row] = self.0[row] * rhs.0[col * 4]
                    + self.0[4 + row] * rhs.0[col * 4 + 1]
                    + self.0[8 + row] * rhs.0[col * 4 + 2]
                    + self.0[12 + row] * rhs.0[col * 4 + 3];
            }
        }
        out
    }

    pub fn translation(x: f64, y: f64, z: f64) -> Self {
        let mut out = Self::IDENTITY;
        out.0[12] = x;
        out.0[13] = y;
        out.0[14] = z;
        out
    }

    pub fn about_origin(self, x: f64, y: f64, z: f64) -> Self {
        Self::translation(x, y, z)
            .concat(self)
            .concat(Self::translation(-x, -y, -z))
    }

    pub fn map_homogeneous(self, point: [f64; 4]) -> [f64; 4] {
        std::array::from_fn(|row| {
            self.0[row] * point[0]
                + self.0[4 + row] * point[1]
                + self.0[8 + row] * point[2]
                + self.0[12 + row] * point[3]
        })
    }

    pub fn map_point(self, x: f64, y: f64, z: f64) -> Option<[f64; 3]> {
        let p = self.map_homogeneous([x, y, z, 1.0]);
        if p[3] <= 0.0 || !p.iter().all(|v| v.is_finite()) {
            return None;
        }
        Some([p[0] / p[3], p[1] / p[3], p[2] / p[3]])
    }

    /// Pinned gfx::Matrix44::Flatten: preserve the projected z=0 plane and
    /// discard inherited depth before a default-flat descendant's transform.
    pub fn flattened(mut self) -> Self {
        self.0[2] = 0.0;
        self.0[6] = 0.0;
        self.0[14] = 0.0;
        self.0[8] = 0.0;
        self.0[9] = 0.0;
        self.0[10] = 1.0;
        self.0[11] = 0.0;
        self
    }

    pub fn inverse(self) -> Option<Self> {
        if self.is_2d() {
            let m = self.0;
            let determinant = m[0] * m[5] - m[1] * m[4];
            if !(determinant as f32).is_normal() {
                return None;
            }
            let inverse = 1.0 / determinant;
            let mut result = Self::IDENTITY;
            result.0[0] = m[5] * inverse;
            result.0[1] = -m[1] * inverse;
            result.0[4] = -m[4] * inverse;
            result.0[5] = m[0] * inverse;
            result.0[12] = (m[4] * m[13] - m[5] * m[12]) * inverse;
            result.0[13] = (m[1] * m[12] - m[0] * m[13]) * inverse;
            return Some(result);
        }
        let cols = inverse_columns([self.col(0), self.col(1), self.col(2), self.col(3)])?;
        let mut out = Self::IDENTITY;
        for (i, col) in cols.into_iter().enumerate() {
            out.set_col(i, col);
        }
        Some(out)
    }

    /// Invert the local z=0 plane homography. This remains valid when the
    /// full 4x4 matrix is singular solely in depth (for example scaleZ(0)).
    pub fn unproject_plane_point(self, x: f64, y: f64) -> Option<[f64; 2]> {
        let m = self.0;
        let [a, b, c, d, e, f, g, h, i] = [m[0], m[4], m[12], m[1], m[5], m[13], m[3], m[7], m[15]];
        let adj = [
            e * i - f * h,
            c * h - b * i,
            b * f - c * e,
            f * g - d * i,
            a * i - c * g,
            c * d - a * f,
            d * h - e * g,
            b * g - a * h,
            a * e - b * d,
        ];
        let determinant = a * adj[0] + b * adj[3] + c * adj[6];
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        let q = [
            adj[0] * x + adj[1] * y + adj[2],
            adj[3] * x + adj[4] * y + adj[5],
            adj[6] * x + adj[7] * y + adj[8],
        ];
        if q[2] == 0.0 || !q.iter().all(|v| v.is_finite()) {
            return None;
        }
        let result = [q[0] / q[2], q[1] / q[2]];
        self.map_point(result[0], result[1], 0.0)?;
        Some(result)
    }

    fn col(self, i: usize) -> [f64; 4] {
        self.0[i * 4..i * 4 + 4].try_into().unwrap()
    }
    fn set_col(&mut self, i: usize, v: [f64; 4]) {
        self.0[i * 4..i * 4 + 4].copy_from_slice(&v);
    }

    pub fn is_invertible(self) -> bool {
        if self.is_2d() {
            return ((self.0[0] * self.0[5] - self.0[1] * self.0[4]) as f32).is_normal();
        }
        self.inverse().is_some()
    }

    fn decompose(self) -> Option<Decomposed> {
        let mut out = Decomposed::identity();
        if self.is_2d() {
            let [mut a, mut b, mut c, mut d] = [self.0[0], self.0[1], self.0[4], self.0[5]];
            let det = a * d - b * c;
            if det == 0.0 {
                return None;
            }
            out.translate = [self.0[12], self.0[13], 0.0];
            if det < 0.0 {
                if a < d {
                    out.scale[0] = -1.0;
                } else {
                    out.scale[1] = -1.0;
                }
            }
            out.scale[0] *= (a * a + b * b).sqrt();
            a /= out.scale[0];
            b /= out.scale[0];
            let shear = a * c + b * d;
            c -= a * shear;
            d -= b * shear;
            out.scale[1] *= (c * c + d * d).sqrt();
            out.skew[0] = shear / out.scale[1];
            let angle = b.atan2(a);
            out.q = Quaternion([0.0, 0.0, (0.5 * angle).sin(), (0.5 * angle).cos()]);
            return Some(out);
        }
        let mut cols = [self.col(0), self.col(1), self.col(2), self.col(3)];
        if !cols[3][3].is_normal() {
            return None;
        }
        let inv_w = 1.0 / cols[3][3];
        for c in &mut cols {
            *c = mul(*c, inv_w);
        }
        let perspective = [cols[0][3], cols[1][3], cols[2][3], 1.0];
        cols[0][3] = 0.0;
        cols[1][3] = 0.0;
        cols[2][3] = 0.0;
        cols[3][3] = 1.0;
        let inverse = inverse_columns(cols)?;
        if perspective != [0.0, 0.0, 0.0, 1.0] {
            out.perspective = inverse.map(|c| sum4(product(perspective, c)));
        }
        out.translate = [cols[3][0], cols[3][1], cols[3][2]];
        let mut c0 = cols[0];
        let mut c1 = cols[1];
        let mut c2 = cols[2];
        out.scale[0] = normalize_column(&mut c0)?;
        out.skew[0] = epsilon_to_zero(sum3(product(c0, c1)));
        c1 = sub(c1, mul(c0, out.skew[0]));
        out.scale[1] = normalize_column(&mut c1)?;
        out.skew[0] /= out.scale[1];
        out.skew[1] = epsilon_to_zero(sum3(product(c0, c2)));
        c2 = sub(c2, mul(c0, out.skew[1]));
        out.skew[2] = epsilon_to_zero(sum3(product(c1, c2)));
        c2 = sub(c2, mul(c1, out.skew[2]));
        out.scale[2] = normalize_column(&mut c2)?;
        out.skew[1] /= out.scale[2];
        out.skew[2] /= out.scale[2];
        let cross = [
            c1[1] * c2[2] - c1[2] * c2[1],
            c1[2] * c2[0] - c1[0] * c2[2],
            c1[0] * c2[1] - c1[1] * c2[0],
            0.0,
        ];
        if sum3(product(c0, cross)) < 0.0 {
            out.scale = out.scale.map(|v| -v);
            c0 = mul(c0, -1.0);
            c1 = mul(c1, -1.0);
            c2 = mul(c2, -1.0);
        }
        let trace = c0[0] + c1[1] + c2[2];
        out.q = if 1.0 + trace > 0.001 {
            let r = (1.0 + trace).sqrt();
            let s = 0.5 / r;
            Quaternion([
                (c1[2] - c2[1]) * s,
                (c2[0] - c0[2]) * s,
                (c0[1] - c1[0]) * s,
                0.5 * r,
            ])
        } else if c0[0] > c1[1] && c0[0] > c2[2] {
            let r = (1.0 + c0[0] - c1[1] - c2[2]).sqrt();
            let s = 0.5 / r;
            Quaternion([
                0.5 * r,
                (c1[0] + c0[1]) * s,
                (c2[0] + c0[2]) * s,
                (c1[2] - c2[1]) * s,
            ])
        } else if c1[1] > c2[2] {
            let r = (1.0 - c0[0] + c1[1] - c2[2]).sqrt();
            let s = 0.5 / r;
            Quaternion([
                (c1[0] + c0[1]) * s,
                0.5 * r,
                (c2[1] + c1[2]) * s,
                (c2[0] - c0[2]) * s,
            ])
        } else {
            let r = (1.0 - c0[0] - c1[1] + c2[2]).sqrt();
            let s = 0.5 / r;
            Quaternion([
                (c2[0] + c0[2]) * s,
                (c2[1] + c1[2]) * s,
                0.5 * r,
                (c0[1] - c1[0]) * s,
            ])
        };
        Some(out)
    }

    fn blend(self, from: Self, progress: f64) -> Option<Self> {
        let to = self.decompose()?;
        let from = from.decompose()?;
        let components = |a: f64, b: f64| a * progress + b * (1.0 - progress);
        let out = Decomposed {
            translate: std::array::from_fn(|i| components(to.translate[i], from.translate[i])),
            scale: std::array::from_fn(|i| components(to.scale[i], from.scale[i])),
            skew: std::array::from_fn(|i| components(to.skew[i], from.skew[i])),
            perspective: std::array::from_fn(|i| {
                components(to.perspective[i], from.perspective[i])
            }),
            q: from.q.slerp(to.q, progress),
        };
        Some(out.compose())
    }

    fn accumulate(self, effect: Self) -> Option<Self> {
        let a = self.decompose()?;
        let b = effect.decompose()?;
        let out = Decomposed {
            translate: std::array::from_fn(|i| a.translate[i] + b.translate[i]),
            scale: std::array::from_fn(|i| a.scale[i] + b.scale[i] - 1.0),
            skew: std::array::from_fn(|i| a.skew[i] + b.skew[i]),
            perspective: std::array::from_fn(|i| {
                a.perspective[i] + b.perspective[i] - if i == 3 { 1.0 } else { 0.0 }
            }),
            q: a.q.multiply(b.q),
        };
        Some(out.compose())
    }
}

/// The source's optimized float axis representation. This tag is retained
/// from operations, never inferred from zero-skew matrix components.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisTransform2d {
    pub scale: [f32; 2],
    pub translation: [f32; 2],
}
impl AxisTransform2d {
    pub const IDENTITY: Self = Self {
        scale: [1.0, 1.0],
        translation: [0.0, 0.0],
    };
}

/// A gfx::Transform-equivalent applied value. `None` means full Matrix44,
/// including explicit identity/axis-aligned matrices which stay full matrices.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedTransform {
    pub matrix: TransformMatrix,
    pub axis_2d: Option<AxisTransform2d>,
}
impl AppliedTransform {
    pub const IDENTITY: Self = Self {
        matrix: TransformMatrix::IDENTITY,
        axis_2d: Some(AxisTransform2d::IDENTITY),
    };
    pub fn identity() -> Self {
        Self::IDENTITY
    }
    pub fn from_matrix(matrix: TransformMatrix) -> Self {
        Self {
            matrix,
            axis_2d: None,
        }
    }
    pub fn from_axis(axis: AxisTransform2d) -> Self {
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[0] = axis.scale[0] as f64;
        matrix.0[5] = axis.scale[1] as f64;
        matrix.0[12] = axis.translation[0] as f64;
        matrix.0[13] = axis.translation[1] as f64;
        Self {
            matrix,
            axis_2d: Some(axis),
        }
    }
    pub fn translation(x: f32, y: f32) -> Self {
        Self::from_axis(AxisTransform2d {
            translation: [x, y],
            ..AxisTransform2d::IDENTITY
        })
    }
    /// `self * rhs`, with exact source Axis/Matrix promotion and arithmetic.
    pub fn concat(self, rhs: Self) -> Self {
        match (self.axis_2d, rhs.axis_2d) {
            (Some(a), Some(b)) => Self::from_axis(AxisTransform2d {
                translation: [
                    a.translation[0] + b.translation[0] * a.scale[0],
                    a.translation[1] + b.translation[1] * a.scale[1],
                ],
                scale: [a.scale[0] * b.scale[0], a.scale[1] * b.scale[1]],
            }),
            (None, Some(b)) => {
                let mut matrix = self.matrix;
                matrix.set_col(
                    3,
                    add(
                        add(
                            mul(matrix.col(0), b.translation[0] as f64),
                            mul(matrix.col(1), b.translation[1] as f64),
                        ),
                        matrix.col(3),
                    ),
                );
                matrix.set_col(0, mul(matrix.col(0), b.scale[0] as f64));
                matrix.set_col(1, mul(matrix.col(1), b.scale[1] as f64));
                Self::from_matrix(matrix)
            }
            (Some(a), None) => {
                let mut matrix = rhs.matrix;
                for row in 0..2 {
                    if a.scale[row] != 1.0 {
                        for col in 0..4 {
                            matrix.0[col * 4 + row] *= a.scale[row] as f64;
                        }
                    }
                }
                let perspective = matrix.0[3] != 0.0
                    || matrix.0[7] != 0.0
                    || matrix.0[11] != 0.0
                    || matrix.0[15] != 1.0;
                if !perspective {
                    matrix.0[12] += a.translation[0] as f64;
                    matrix.0[13] += a.translation[1] as f64;
                } else {
                    for row in 0..2 {
                        if a.translation[row] != 0.0 {
                            for col in 0..4 {
                                matrix.0[col * 4 + row] +=
                                    matrix.0[col * 4 + 3] * a.translation[row] as f64;
                            }
                        }
                    }
                }
                Self::from_matrix(matrix)
            }
            (None, None) => Self::from_matrix(self.matrix.concat(rhs.matrix)),
        }
    }
    pub fn inverse(self) -> Option<Self> {
        if let Some(mut axis) = self.axis_2d {
            if !(axis.scale[0] * axis.scale[1]).is_normal() {
                return None;
            }
            axis.scale = [1.0 / axis.scale[0], 1.0 / axis.scale[1]];
            axis.translation = [
                axis.translation[0] * -axis.scale[0],
                axis.translation[1] * -axis.scale[1],
            ];
            Some(Self::from_axis(axis))
        } else {
            self.matrix.inverse().map(Self::from_matrix)
        }
    }
    pub fn is_invertible(self) -> bool {
        self.axis_2d.map_or_else(
            || self.matrix.is_invertible(),
            |axis| (axis.scale[0] * axis.scale[1]).is_normal(),
        )
    }
    pub fn flattened(self) -> Self {
        if self.axis_2d.is_some() {
            self
        } else {
            Self::from_matrix(self.matrix.flattened())
        }
    }
    pub fn is_2d(self) -> bool {
        self.matrix.is_2d()
    }
    fn from_internal(value: Applied) -> Self {
        if value.axis {
            Self::from_axis(AxisTransform2d {
                scale: [value.matrix.0[0] as f32, value.matrix.0[5] as f32],
                translation: [value.matrix.0[12] as f32, value.matrix.0[13] as f32],
            })
        } else {
            Self::from_matrix(value.matrix)
        }
    }
    fn internal(self) -> Applied {
        Applied {
            matrix: self.matrix,
            axis: self.axis_2d.is_some(),
        }
    }
}

fn add(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: [f64; 4], b: f64) -> [f64; 4] {
    a.map(|v| v * b)
}
fn product(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|i| a[i] * b[i])
}
fn swap_pairs(v: [f64; 4]) -> [f64; 4] {
    [v[1], v[0], v[3], v[2]]
}
fn swap_halves(v: [f64; 4]) -> [f64; 4] {
    [v[2], v[3], v[0], v[1]]
}
fn sum3(v: [f64; 4]) -> f64 {
    v[0] + v[1] + v[2]
}
fn sum4(v: [f64; 4]) -> f64 {
    v[0] + v[1] + v[2] + v[3]
}
fn epsilon_to_zero(v: f64) -> f64 {
    if v.abs() < f32::EPSILON as f64 {
        0.0
    } else {
        v
    }
}
fn normalize_column(c: &mut [f64; 4]) -> Option<f64> {
    let length = sum3(product(*c, *c)).sqrt();
    if !length.is_normal() {
        return None;
    }
    *c = mul(*c, 1.0 / length);
    Some(length)
}

fn inverse_columns(cols: [[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let [a, b, c, d] = cols;
    let r0 = [a[0], b[0], c[0], d[0]];
    let r1 = [c[1], d[1], a[1], b[1]];
    let mut r2 = [a[2], b[2], c[2], d[2]];
    let r3 = [c[3], d[3], a[3], b[3]];
    let mut t = swap_pairs(product(r2, r3));
    let mut c0 = product(r1, t);
    let mut c1 = product(r0, t);
    t = swap_halves(t);
    c0 = sub(product(r1, t), c0);
    c1 = swap_halves(sub(product(r0, t), c1));
    t = swap_pairs(product(r1, r2));
    c0 = add(c0, product(r3, t));
    let mut c3 = product(r0, t);
    t = swap_halves(t);
    c0 = sub(c0, product(r3, t));
    c3 = swap_halves(sub(product(r0, t), c3));
    t = swap_pairs(product(swap_halves(r1), r3));
    r2 = swap_halves(r2);
    c0 = add(c0, product(r2, t));
    let mut c2 = product(r0, t);
    t = swap_halves(t);
    c0 = sub(c0, product(r2, t));
    let det = sum4(product(r0, c0));
    if !(det as f32).is_normal() {
        return None;
    }
    c2 = swap_halves(sub(product(r0, t), c2));
    t = swap_pairs(product(r0, r1));
    c2 = add(product(r3, t), c2);
    c3 = sub(product(r2, t), c3);
    t = swap_halves(t);
    c2 = sub(product(r3, t), c2);
    c3 = sub(c3, product(r2, t));
    t = swap_pairs(product(r0, r3));
    c1 = sub(c1, product(r2, t));
    c2 = add(product(r1, t), c2);
    t = swap_halves(t);
    c1 = add(product(r2, t), c1);
    c2 = sub(c2, product(r1, t));
    t = swap_pairs(product(r0, r2));
    c1 = add(product(r3, t), c1);
    c3 = sub(c3, product(r1, t));
    t = swap_halves(t);
    c1 = sub(c1, product(r3, t));
    c3 = add(product(r1, t), c3);
    Some([
        mul(c0, 1.0 / det),
        mul(c1, 1.0 / det),
        mul(c2, 1.0 / det),
        mul(c3, 1.0 / det),
    ])
}

#[derive(Debug, Clone, Copy)]
struct Quaternion([f64; 4]);
impl Quaternion {
    const IDENTITY: Self = Self([0.0, 0.0, 0.0, 1.0]);
    fn axis_angle(axis: [f32; 3], degrees: f64) -> Self {
        let [x, y, z] = axis.map(f64::from);
        let length = (x * x + y * y + z * z).sqrt();
        if length.abs() < 1e-5 {
            return Self::IDENTITY;
        }
        let angle = degrees * (std::f64::consts::PI / 180.0);
        let scale = (0.5 * angle).sin() / length;
        Self([scale * x, scale * y, scale * z, (0.5 * angle).cos()])
    }
    fn multiply(self, q: Self) -> Self {
        let [x, y, z, w] = self.0;
        let [a, b, c, d] = q.0;
        Self([
            w * a + x * d + y * c - z * b,
            w * b - x * c + y * d + z * a,
            w * c + x * b - y * a + z * d,
            w * d - x * a - y * b - z * c,
        ])
    }
    fn slerp(self, to: Self, t: f64) -> Self {
        let mut from = self.0;
        let mut cos = from[0] * to.0[0] + from[1] * to.0[1] + from[2] * to.0[2] + from[3] * to.0[3];
        if cos < 0.0 {
            from = mul(from, -1.0);
            cos = -cos;
        }
        if cos > 1.0 {
            cos = 1.0;
        }
        let sin = (1.0 - cos * cos).sqrt();
        if sin < 1e-5 {
            return self;
        }
        let half = cos.acos();
        Self(add(
            mul(from, ((1.0 - t) * half).sin() / sin),
            mul(to.0, (t * half).sin() / sin),
        ))
    }
    fn matrix(self) -> TransformMatrix {
        let [x, y, z, w] = self.0;
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let xw = x * w;
        let yw = y * w;
        let zw = z * w;
        TransformMatrix([
            1.0 - 2.0 * (yy + zz),
            2.0 * (xy + zw),
            2.0 * (xz - yw),
            0.0,
            2.0 * (xy - zw),
            1.0 - 2.0 * (xx + zz),
            2.0 * (yz + xw),
            0.0,
            2.0 * (xz + yw),
            2.0 * (yz - xw),
            1.0 - 2.0 * (xx + yy),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ])
    }
    fn rotation(self) -> ([f32; 3], f64) {
        (
            normalize_axis([self.0[0] as f32, self.0[1] as f32, self.0[2] as f32]),
            (2.0 * self.0[3].acos()) * (180.0 / std::f64::consts::PI),
        )
    }
}

struct Decomposed {
    translate: [f64; 3],
    scale: [f64; 3],
    skew: [f64; 3],
    perspective: [f64; 4],
    q: Quaternion,
}
impl Decomposed {
    fn identity() -> Self {
        Self {
            translate: [0.0; 3],
            scale: [1.0; 3],
            skew: [0.0; 3],
            perspective: [0.0, 0.0, 0.0, 1.0],
            q: Quaternion::IDENTITY,
        }
    }
    fn compose(self) -> TransformMatrix {
        let mut out = TransformMatrix::IDENTITY;
        for i in 0..4 {
            out.0[i * 4 + 3] = self.perspective[i];
        }
        out = out.concat(TransformMatrix::translation(
            self.translate[0] as f32 as f64,
            self.translate[1] as f32 as f64,
            self.translate[2] as f32 as f64,
        ));
        out = out.concat(self.q.matrix());
        let c0 = out.col(0);
        let c1 = out.col(1);
        let c2 = out.col(2);
        out.set_col(1, add(c1, mul(c0, self.skew[0])));
        out.set_col(
            2,
            add(add(mul(c0, self.skew[1]), mul(c1, self.skew[2])), c2),
        );
        for i in 0..3 {
            out.set_col(i, mul(out.col(i), self.scale[i] as f32 as f64));
        }
        out
    }
}

fn axis_squared(axis: [f32; 3]) -> f64 {
    let [x, y, z] = axis.map(f64::from);
    x * x + y * y + z * z
}
fn normalize_axis(axis: [f32; 3]) -> [f32; 3] {
    let squared = axis_squared(axis);
    if squared < 1e-12 {
        return [0.0, 0.0, 1.0];
    }
    let length = squared.sqrt() as f32;
    axis.map(|v| v / length)
}

fn common_axis(a: ([f32; 3], f64), b: ([f32; 3], f64)) -> Option<([f32; 3], f64, f64)> {
    let zero_a = a.0 == [0.0; 3];
    let zero_b = b.0 == [0.0; 3];
    let (za, zb) = if zero_a || zero_b {
        (zero_a, zero_b)
    } else {
        (a.1.abs() < 1e-4, b.1.abs() < 1e-4)
    };
    if za && zb {
        return Some(([0.0, 0.0, 1.0], 0.0, 0.0));
    }
    if za {
        return Some((normalize_axis(b.0), 0.0, b.1));
    }
    if zb {
        return Some((normalize_axis(a.0), a.1, 0.0));
    }
    let dot = (a.0[0] * b.0[0] + a.0[1] * b.0[1] + a.0[2] * b.0[2]) as f64;
    if dot < 0.0 || (1.0 - dot * dot / (axis_squared(a.0) * axis_squared(b.0))).abs() > 1e-4 {
        return None;
    }
    Some((normalize_axis(a.0), a.1, b.1))
}

fn rotation_matrix(axis: [f32; 3], degrees: f64) -> TransformMatrix {
    if axis == [0.0; 3] {
        return TransformMatrix::IDENTITY;
    }
    let [mut x, mut y, mut z] = axis.map(f64::from);
    let length = (x * x + y * y + z * z).sqrt();
    if length != 1.0 {
        let inv = 1.0 / length;
        x *= inv;
        y *= inv;
        z *= inv;
    }
    let (s, c) = sin_cos_degrees(degrees);
    if (s, c) == (0.0, 1.0) {
        return TransformMatrix::IDENTITY;
    }
    if z == 1.0 {
        return TransformMatrix([
            c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
    }
    if y == 1.0 {
        return TransformMatrix([
            c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
    }
    if x == 1.0 {
        return TransformMatrix([
            1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
    }
    let big_c = 1.0 - c;
    let xs = x * s;
    let ys = y * s;
    let zs = z * s;
    let xc = x * big_c;
    let yc = y * big_c;
    let zc = z * big_c;
    let xyc = x * yc;
    let yzc = y * zc;
    let zxc = z * xc;
    TransformMatrix([
        x * xc + c,
        xyc + zs,
        zxc - ys,
        0.0,
        xyc - zs,
        y * yc + c,
        yzc + xs,
        0.0,
        zxc + ys,
        yzc - xs,
        z * zc + c,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ])
}

fn sin_cos_degrees(mut degrees: f64) -> (f64, f64) {
    if degrees > -90_000_000.0 && degrees < 90_000_000.0 {
        let quotient = degrees / 45.0;
        let mut octant = quotient as i32;
        if octant as f64 == quotient {
            let h = std::f64::consts::SQRT_2 / 2.0;
            return [
                (0.0, 1.0),
                (h, h),
                (1.0, 0.0),
                (h, -h),
                (0.0, -1.0),
                (-h, -h),
                (-1.0, 0.0),
                (-h, h),
            ][(octant & 7) as usize];
        }
        if degrees < 0.0 {
            octant -= 1;
        }
        degrees -= (octant * 45) as f64;
        if octant & 1 != 0 {
            degrees = 45.0 - degrees;
        }
        let rad = degrees * (std::f64::consts::PI / 180.0);
        let mut s = rad.sin();
        let mut c = rad.cos();
        if (octant + 1) & 2 != 0 {
            std::mem::swap(&mut s, &mut c);
        }
        if octant & 4 != 0 {
            s = -s;
        }
        if (octant + 2) & 4 != 0 {
            c = -c;
        }
        return (s, c);
    }
    degrees %= 360.0;
    let rad = degrees * (std::f64::consts::PI / 180.0);
    (rad.sin(), rad.cos())
}

/// Retained length expressions are resolved recursively with the selected
/// font, then combined with Blink's float length arithmetic. This allows an
/// interpolated endpoint to contain font-relative native lengths as well as
/// percentages without prematurely choosing a reference box.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub enum TransformLengthExpression {
    Value(LengthValue),
    Blend(Box<Self>, Box<Self>, f64),
    Add(Box<Self>, Box<Self>),
    ScalarBlend(Box<Self>, Box<Self>, f64),
    ScalarAdd(Box<Self>, Box<Self>),
}

#[derive(Clone, Copy, PartialEq)]
enum TransformLengthKind {
    Fixed,
    Percent,
    Calculated,
}
#[derive(Clone, Copy)]
struct TransformLengthCoefficients {
    pixels: f32,
    percent: f32,
    kind: TransformLengthKind,
}
impl TransformLengthCoefficients {
    fn scalar(self) -> f32 {
        if self.kind == TransformLengthKind::Percent {
            self.percent
        } else {
            self.pixels
        }
    }
    fn simple(kind: TransformLengthKind, value: f32) -> Self {
        if kind == TransformLengthKind::Percent {
            Self {
                pixels: 0.0,
                percent: value,
                kind,
            }
        } else {
            Self {
                pixels: value,
                percent: 0.0,
                kind,
            }
        }
    }
}

/// Resolve a computed transform length with the same kind-sensitive float
/// arithmetic used by Blink's FloatValueForLength/CalculationValue::Evaluate.
#[doc(hidden)]
pub fn transform_length_for_reference_box(length: openui_geometry::Length, reference: f32) -> f32 {
    TransformLengthExpression::Value(LengthValue::Computed(length)).evaluate(reference as f64)
        as f32
}

impl TransformLengthExpression {
    fn zero() -> Self {
        Self::Value(LengthValue::px(0.0))
    }
    fn map(&self, resolve: &impl Fn(LengthValue) -> LengthValue) -> Self {
        match self {
            Self::Value(v) => Self::Value(resolve(*v)),
            Self::Blend(a, b, p) => {
                Self::Blend(Box::new(a.map(resolve)), Box::new(b.map(resolve)), *p)
            }
            Self::Add(a, b) => Self::Add(Box::new(a.map(resolve)), Box::new(b.map(resolve))),
            Self::ScalarBlend(a, b, p) => {
                Self::ScalarBlend(Box::new(a.map(resolve)), Box::new(b.map(resolve)), *p)
            }
            Self::ScalarAdd(a, b) => {
                Self::ScalarAdd(Box::new(a.map(resolve)), Box::new(b.map(resolve)))
            }
        }
    }
    fn coefficients(&self) -> TransformLengthCoefficients {
        use TransformLengthKind::{Calculated, Fixed, Percent};
        match self {
            Self::Value(LengthValue::Computed(value)) => {
                if value.is_percent() {
                    TransformLengthCoefficients {
                        pixels: 0.0,
                        percent: value.value(),
                        kind: Percent,
                    }
                } else if value.is_calculated() {
                    TransformLengthCoefficients {
                        pixels: value.calc_offset(),
                        percent: value.value(),
                        kind: Calculated,
                    }
                } else {
                    TransformLengthCoefficients {
                        pixels: if value.is_fixed() { value.value() } else { 0.0 },
                        percent: 0.0,
                        kind: Fixed,
                    }
                }
            }
            Self::Value(_) => unreachable!("font-relative transform length was not resolved"),
            Self::Blend(a, b, p) => {
                let a = a.coefficients();
                let b = b.coefficients();
                // Length::Blend preserves the original endpoint's type at
                // endpoints, including calculated zero-pixel coefficients.
                if *p == 0.0 {
                    return a;
                }
                if *p == 1.0 {
                    return b;
                }
                if a.kind == Calculated
                    || b.kind == Calculated
                    || (a.scalar() != 0.0 && b.scalar() != 0.0 && a.kind != b.kind)
                {
                    return TransformLengthCoefficients {
                        pixels: blend_float(a.pixels, b.pixels, *p),
                        percent: blend_float(a.percent, b.percent, *p),
                        kind: Calculated,
                    };
                }
                if a.scalar() == 0.0 && b.scalar() == 0.0 {
                    return b;
                }
                let kind = if b.scalar() == 0.0 { a.kind } else { b.kind };
                TransformLengthCoefficients::simple(kind, blend_float(a.scalar(), b.scalar(), *p))
            }
            Self::Add(a, b) => {
                // TranslateTransformOperation::AddLengths collapses its
                // summed coefficients, unlike Length::Blend mixed types.
                let a = a.coefficients();
                let b = b.coefficients();
                let pixels = a.pixels + b.pixels;
                let percent = a.percent + b.percent;
                let kind = if percent == 0.0 {
                    Fixed
                } else if pixels == 0.0 {
                    Percent
                } else {
                    Calculated
                };
                TransformLengthCoefficients {
                    pixels,
                    percent,
                    kind,
                }
            }
            Self::ScalarBlend(..) | Self::ScalarAdd(..) => {
                unreachable!("scalar z length used for x/y")
            }
        }
    }
    fn evaluate(&self, reference: f64) -> f64 {
        let value = self.coefficients();
        let result = match value.kind {
            TransformLengthKind::Fixed => value.pixels,
            TransformLengthKind::Percent => (reference as f32) * value.percent / 100.0,
            // CalculationValue::Evaluate deliberately divides before the
            // reference-box multiply. Pure percent uses a different order.
            TransformLengthKind::Calculated => {
                value.pixels + value.percent / 100.0 * (reference as f32)
            }
        };
        if result.is_nan() {
            0.0
        } else {
            result.clamp(f32::MIN, f32::MAX) as f64
        }
    }
    fn depends_on_box(&self) -> bool {
        match self {
            Self::Value(LengthValue::Computed(v)) => v.is_percent() || v.is_calculated(),
            Self::Value(_) => false,
            Self::Blend(a, b, _)
            | Self::Add(a, b)
            | Self::ScalarBlend(a, b, _)
            | Self::ScalarAdd(a, b) => a.depends_on_box() || b.depends_on_box(),
        }
    }
    fn has_unresolved_units(&self) -> bool {
        match self {
            Self::Value(v) => !matches!(v, LengthValue::Computed(_)),
            Self::Blend(a, b, _)
            | Self::Add(a, b)
            | Self::ScalarBlend(a, b, _)
            | Self::ScalarAdd(a, b) => a.has_unresolved_units() || b.has_unresolved_units(),
        }
    }
    fn depends_on_font(&self) -> bool {
        match self {
            Self::Value(v) => v.depends_on_font_metrics(),
            Self::Blend(a, b, _)
            | Self::Add(a, b)
            | Self::ScalarBlend(a, b, _)
            | Self::ScalarAdd(a, b) => a.depends_on_font() || b.depends_on_font(),
        }
    }
    fn evaluate_scalar(&self) -> f64 {
        match self {
            Self::ScalarBlend(a, b, p) => {
                blend_double(a.evaluate_scalar(), b.evaluate_scalar(), *p)
            }
            Self::ScalarAdd(a, b) => a.evaluate_scalar() + b.evaluate_scalar(),
            _ => self.evaluate(0.0),
        }
    }
}

fn blend_float(from: f32, to: f32, p: f64) -> f32 {
    (from as f64 + ((to - from) as f64) * p) as f32
}
fn blend_double(from: f64, to: f64, p: f64) -> f64 {
    from + (to - from) * p
}

/// Operations produced by composition retain double scalar values. Existing
/// native constructors remain unchanged; this is an internal publication
/// form, not a replacement public CSS syntax.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedTransformOperation {
    Translate {
        x: TransformLengthExpression,
        y: TransformLengthExpression,
        z: TransformLengthExpression,
        is_3d: bool,
    },
    Scale {
        values: [f64; 3],
        is_3d: bool,
    },
    Rotate {
        axis: [f32; 3],
        degrees: f64,
        is_3d: bool,
    },
    Perspective(Option<f64>),
    Matrix {
        value: TransformMatrix,
        is_3d: bool,
    },
}

impl ResolvedTransformOperation {
    fn primitive(&self) -> u8 {
        match self {
            Self::Translate { .. } => 0,
            Self::Scale { .. } => 1,
            Self::Rotate { .. } => 2,
            Self::Perspective(_) => 3,
            Self::Matrix { is_3d: false, .. } => 4,
            Self::Matrix { is_3d: true, .. } => 5,
        }
    }
    fn identity_like(&self) -> Self {
        match self {
            Self::Translate { is_3d, .. } => Self::Translate {
                x: TransformLengthExpression::zero(),
                y: TransformLengthExpression::zero(),
                z: TransformLengthExpression::zero(),
                is_3d: *is_3d,
            },
            Self::Scale { is_3d, .. } => Self::Scale {
                values: [1.0; 3],
                is_3d: *is_3d,
            },
            Self::Rotate { axis, is_3d, .. } => Self::Rotate {
                axis: *axis,
                degrees: 0.0,
                is_3d: *is_3d,
            },
            Self::Perspective(_) => Self::Perspective(None),
            Self::Matrix { is_3d, .. } => Self::Matrix {
                value: TransformMatrix::IDENTITY,
                is_3d: *is_3d,
            },
        }
    }
    fn depends_on_box(&self) -> bool {
        match self {
            Self::Translate { x, y, z, .. } => {
                x.depends_on_box() || y.depends_on_box() || z.depends_on_box()
            }
            _ => false,
        }
    }
    fn blend(&self, from: &Self, p: f64) -> Option<Self> {
        match (from, self) {
            (
                Self::Translate {
                    x: a,
                    y: b,
                    z: c,
                    is_3d: i,
                },
                Self::Translate {
                    x: x,
                    y: y,
                    z: z,
                    is_3d: j,
                },
            ) => Some(Self::Translate {
                x: TransformLengthExpression::Blend(Box::new(a.clone()), Box::new(x.clone()), p),
                y: TransformLengthExpression::Blend(Box::new(b.clone()), Box::new(y.clone()), p),
                z: TransformLengthExpression::ScalarBlend(
                    Box::new(c.clone()),
                    Box::new(z.clone()),
                    p,
                ),
                is_3d: *i || *j,
            }),
            (
                Self::Scale {
                    values: a,
                    is_3d: i,
                },
                Self::Scale {
                    values: b,
                    is_3d: j,
                },
            ) => Some(Self::Scale {
                values: std::array::from_fn(|k| blend_double(a[k], b[k], p)),
                is_3d: *i || *j,
            }),
            (
                Self::Rotate {
                    axis: a,
                    degrees: ad,
                    is_3d: i,
                },
                Self::Rotate {
                    axis: b,
                    degrees: bd,
                    is_3d: j,
                },
            ) => {
                let (axis, degrees) =
                    if let Some((axis, fa, ta)) = common_axis((*a, *ad), (*b, *bd)) {
                        (axis, blend_double(fa, ta, p))
                    } else {
                        Quaternion::axis_angle(*a, *ad)
                            .slerp(Quaternion::axis_angle(*b, *bd), p)
                            .rotation()
                    };
                Some(Self::Rotate {
                    axis,
                    degrees,
                    is_3d: *i || *j,
                })
            }
            (Self::Perspective(a), Self::Perspective(b)) => {
                let inverse = blend_double(
                    a.map_or(0.0, |v| 1.0 / v.max(1.0)),
                    b.map_or(0.0, |v| 1.0 / v.max(1.0)),
                    p,
                );
                Some(Self::Perspective(if inverse > 0.0 && inverse.is_normal() {
                    Some(1.0 / inverse)
                } else {
                    None
                }))
            }
            (Self::Matrix { value: a, .. }, Self::Matrix { value: b, is_3d }) => {
                Some(Self::Matrix {
                    value: b.blend(*a, p)?,
                    is_3d: *is_3d,
                })
            }
            _ => None,
        }
    }
    fn accumulate(&self, effect: &Self) -> Option<Self> {
        match (self, effect) {
            (
                Self::Translate {
                    x: a,
                    y: b,
                    z: c,
                    is_3d: i,
                },
                Self::Translate {
                    x: x,
                    y: y,
                    z: z,
                    is_3d: j,
                },
            ) => Some(Self::Translate {
                x: TransformLengthExpression::Add(Box::new(a.clone()), Box::new(x.clone())),
                y: TransformLengthExpression::Add(Box::new(b.clone()), Box::new(y.clone())),
                z: TransformLengthExpression::ScalarAdd(Box::new(c.clone()), Box::new(z.clone())),
                is_3d: *i || *j,
            }),
            (
                Self::Scale {
                    values: a,
                    is_3d: i,
                },
                Self::Scale {
                    values: b,
                    is_3d: j,
                },
            ) => Some(Self::Scale {
                values: std::array::from_fn(|k| a[k] + b[k] - 1.0),
                is_3d: *i || *j,
            }),
            (
                Self::Rotate {
                    axis: a,
                    degrees: ad,
                    is_3d: i,
                },
                Self::Rotate {
                    axis: b,
                    degrees: bd,
                    is_3d: j,
                },
            ) => {
                let (axis, degrees) =
                    if let Some((axis, fa, ta)) = common_axis((*a, *ad), (*b, *bd)) {
                        (axis, fa + ta)
                    } else {
                        let mut q = Quaternion::axis_angle(*a, *ad)
                            .multiply(Quaternion::axis_angle(*b, *bd));
                        if q.0[3] < 0.0 {
                            q.0 = mul(q.0, -1.0);
                        }
                        q.rotation()
                    };
                Some(Self::Rotate {
                    axis,
                    degrees,
                    is_3d: *i || *j,
                })
            }
            (Self::Perspective(a), Self::Perspective(b)) => Some(Self::Perspective(match (a, b) {
                (None, b) => *b,
                (a, None) => *a,
                (Some(a), Some(b)) => {
                    let a = a.max(1.0);
                    let b = b.max(1.0);
                    Some((a * b) / (a + b))
                }
            })),
            (Self::Matrix { value: a, is_3d }, Self::Matrix { value: b, .. }) => {
                Some(Self::Matrix {
                    value: a.accumulate(*b)?,
                    is_3d: *is_3d,
                })
            }
            _ => None,
        }
    }
}

fn resolved(op: &TransformOperation) -> Option<ResolvedTransformOperation> {
    use ResolvedTransformOperation as R;
    let l = |v| TransformLengthExpression::Value(v);
    Some(match op {
        TransformOperation::Translate(x, y) => R::Translate {
            x: l(*x),
            y: l(*y),
            z: TransformLengthExpression::zero(),
            is_3d: false,
        },
        TransformOperation::Translate3d(x, y, z) => R::Translate {
            x: l(*x),
            y: l(*y),
            z: l(*z),
            is_3d: true,
        },
        TransformOperation::Scale(x, y) => R::Scale {
            values: [*x as f64, *y as f64, 1.0],
            is_3d: false,
        },
        TransformOperation::Scale3d(x, y, z) => R::Scale {
            values: [*x as f64, *y as f64, *z as f64],
            is_3d: true,
        },
        TransformOperation::Rotate(degrees) => R::Rotate {
            axis: [0.0, 0.0, 1.0],
            degrees: *degrees as f64,
            is_3d: false,
        },
        TransformOperation::Rotate3d { x, y, z, degrees } => R::Rotate {
            axis: [*x, *y, *z],
            degrees: *degrees as f64,
            is_3d: true,
        },
        TransformOperation::Perspective(LengthValue::Computed(v)) => {
            R::Perspective(if v.is_fixed() {
                Some(v.value() as f64)
            } else {
                None
            })
        }
        TransformOperation::Perspective(_) => return None,
        TransformOperation::Matrix(m) => R::Matrix {
            value: TransformMatrix::from_affine(*m),
            is_3d: false,
        },
        TransformOperation::Matrix3d(m) => R::Matrix {
            value: TransformMatrix(m.0.map(f64::from)),
            is_3d: true,
        },
        TransformOperation::Resolved(op) => (**op).clone(),
        TransformOperation::Interpolated { .. }
        | TransformOperation::PendingInterpolation { .. }
        | TransformOperation::PendingAccumulation { .. } => return None,
    })
}

fn matching_prefix(a: &TransformList, b: &TransformList) -> usize {
    for (i, (a, b)) in a.0.iter().zip(&b.0).enumerate() {
        let compatible = match (resolved(a), resolved(b)) {
            (Some(a), Some(b)) => a.primitive() == b.primitive(),
            (None, None) => matches!(
                (a, b),
                (
                    TransformOperation::Interpolated { .. },
                    TransformOperation::Interpolated { .. }
                )
            ),
            _ => false,
        };
        if !compatible {
            return i;
        }
    }
    a.0.len().max(b.0.len())
}

fn discrete(from: &TransformList, to: &TransformList, p: f64) -> TransformList {
    if p < 0.5 {
        from.clone()
    } else {
        to.clone()
    }
}

/// Interpolate matching primitives and identity-pad shorter lists. A matrix
/// suffix with percentage dependence remains owned and deferred until layout.
/// Unresolved font/viewport units remain owned until [`map_transform_lengths`]
/// supplies the Engine's metrics and materializes this same sampler.
pub fn interpolate_transform_lists(
    from: &TransformList,
    to: &TransformList,
    p: f64,
) -> TransformList {
    if from == to {
        return to.clone();
    }
    if transform_has_unresolved_lengths(from) || transform_has_unresolved_lengths(to) {
        return TransformList(vec![TransformOperation::PendingInterpolation {
            from: Box::new(from.clone()),
            to: Box::new(to.clone()),
            progress: p,
        }]);
    }
    let prefix = matching_prefix(from, to);
    let mut output = Vec::with_capacity(from.0.len().max(to.0.len()));
    for i in 0..prefix {
        let a = from.0.get(i);
        let b = to.0.get(i);
        if a.is_some_and(|v| matches!(v, TransformOperation::Interpolated { .. }))
            || b.is_some_and(|v| matches!(v, TransformOperation::Interpolated { .. }))
        {
            output.push(TransformOperation::Interpolated {
                from: Box::new(TransformList(a.cloned().into_iter().collect())),
                to: Box::new(TransformList(b.cloned().into_iter().collect())),
                starting_index: 0,
                progress: p,
            });
            continue;
        }
        let (a, b) = match (a.and_then(resolved), b.and_then(resolved)) {
            (Some(a), Some(b)) => (a, b),
            (Some(a), None) => {
                let b = a.identity_like();
                (a, b)
            }
            (None, Some(b)) => {
                let a = b.identity_like();
                (a, b)
            }
            (None, None) => return discrete(from, to, p),
        };
        let Some(op) = b.blend(&a, p) else {
            return discrete(from, to, p);
        };
        output.push(TransformOperation::Resolved(Box::new(op)));
    }
    if prefix < from.0.len().max(to.0.len()) {
        if suffix_depends_on_box(from, prefix) || suffix_depends_on_box(to, prefix) {
            output.push(TransformOperation::Interpolated {
                from: Box::new(from.clone()),
                to: Box::new(to.clone()),
                starting_index: prefix,
                progress: p,
            });
        } else {
            let a = apply_remaining(from, (0.0, 0.0), prefix);
            let b = apply_remaining(to, (0.0, 0.0), prefix);
            if !a.matrix.is_invertible() || !b.matrix.is_invertible() {
                return discrete(from, to, p);
            }
            let value = match b.blend(a, p) {
                Some(m) => m,
                None => {
                    if p < 0.5 {
                        a.matrix
                    } else {
                        b.matrix
                    }
                }
            };
            output.push(TransformOperation::Resolved(Box::new(
                ResolvedTransformOperation::Matrix { value, is_3d: true },
            )));
        }
    }
    TransformList(output)
}

/// Accumulate compatible components, then use Chromium's zero-box matrix
/// decomposition for a mismatched suffix. Failure replaces the entire list
/// with the effect; concatenation belongs to the separate Add operation.
/// Unresolved font/viewport units remain owned until [`map_transform_lengths`].
pub fn accumulate_transform_lists(base: &TransformList, effect: &TransformList) -> TransformList {
    if transform_has_unresolved_lengths(base) || transform_has_unresolved_lengths(effect) {
        return TransformList(vec![TransformOperation::PendingAccumulation {
            underlying: Box::new(base.clone()),
            effect: Box::new(effect.clone()),
        }]);
    }
    let prefix = matching_prefix(base, effect);
    let mut output = Vec::with_capacity(base.0.len().max(effect.0.len()));
    for i in 0..prefix {
        match (base.0.get(i), effect.0.get(i)) {
            (Some(a), Some(b)) => {
                let Some(a) = resolved(a) else {
                    return effect.clone();
                };
                let Some(b) = resolved(b) else {
                    return effect.clone();
                };
                let Some(op) = a.accumulate(&b) else {
                    return effect.clone();
                };
                output.push(TransformOperation::Resolved(Box::new(op)));
            }
            (Some(a), None) | (None, Some(a)) => output.push(a.clone()),
            (None, None) => break,
        }
    }
    if prefix < base.0.len().max(effect.0.len()) {
        let a = apply_remaining(base, (0.0, 0.0), prefix);
        let b = apply_remaining(effect, (0.0, 0.0), prefix);
        let Some(value) = a.accumulate(b) else {
            return effect.clone();
        };
        output.push(TransformOperation::Resolved(Box::new(
            ResolvedTransformOperation::Matrix { value, is_3d: true },
        )));
    }
    TransformList(output)
}

fn suffix_depends_on_box(list: &TransformList, start: usize) -> bool {
    list.0.iter().skip(start).any(|op| match op {
        TransformOperation::Interpolated {
            from,
            to,
            starting_index,
            ..
        } => {
            suffix_depends_on_box(from, *starting_index)
                || suffix_depends_on_box(to, *starting_index)
        }
        TransformOperation::PendingInterpolation { from, to, .. } => {
            transform_depends_on_box(from) || transform_depends_on_box(to)
        }
        TransformOperation::PendingAccumulation { underlying, effect } => {
            transform_depends_on_box(underlying) || transform_depends_on_box(effect)
        }
        _ => resolved(op).is_some_and(|op| op.depends_on_box()),
    })
}

/// Map every authored length, including nested interpolated suffixes and
/// composition expressions. Engine font/viewport resolution uses this once
/// instead of losing a deferred endpoint's selected-font units.
#[doc(hidden)]
pub fn map_transform_lengths(
    list: &TransformList,
    resolve: &impl Fn(LengthValue) -> LengthValue,
) -> TransformList {
    let mut mapped = Vec::with_capacity(list.0.len());
    for op in &list.0 {
        let materialized = match op {
            TransformOperation::PendingInterpolation { from, to, progress } => {
                Some(interpolate_transform_lists(
                    &map_transform_lengths(from, resolve),
                    &map_transform_lengths(to, resolve),
                    *progress,
                ))
            }
            TransformOperation::PendingAccumulation { underlying, effect } => {
                Some(accumulate_transform_lists(
                    &map_transform_lengths(underlying, resolve),
                    &map_transform_lengths(effect, resolve),
                ))
            }
            _ => None,
        };
        if let Some(value) = materialized {
            mapped.extend(value.0);
            continue;
        }
        mapped.push(match op {
            TransformOperation::Translate(x, y) => {
                TransformOperation::Translate(resolve(*x), resolve(*y))
            }
            TransformOperation::Translate3d(x, y, z) => {
                TransformOperation::Translate3d(resolve(*x), resolve(*y), resolve(*z))
            }
            TransformOperation::Perspective(v) => TransformOperation::Perspective(resolve(*v)),
            TransformOperation::Interpolated {
                from,
                to,
                starting_index,
                progress,
            } => TransformOperation::Interpolated {
                from: Box::new(map_transform_lengths(from, resolve)),
                to: Box::new(map_transform_lengths(to, resolve)),
                starting_index: *starting_index,
                progress: *progress,
            },
            TransformOperation::Resolved(v) => TransformOperation::Resolved(Box::new(match &**v {
                ResolvedTransformOperation::Translate { x, y, z, is_3d } => {
                    ResolvedTransformOperation::Translate {
                        x: x.map(resolve),
                        y: y.map(resolve),
                        z: z.map(resolve),
                        is_3d: *is_3d,
                    }
                }
                v => v.clone(),
            })),
            v => v.clone(),
        });
    }
    TransformList(mapped)
}

#[doc(hidden)]
pub fn transform_depends_on_font_metrics(list: &TransformList) -> bool {
    list.0.iter().any(|op|match op {
        TransformOperation::Translate(x,y)=>x.depends_on_font_metrics()||y.depends_on_font_metrics(),
        TransformOperation::Translate3d(x,y,z)=>[*x,*y,*z].into_iter().any(LengthValue::depends_on_font_metrics),
        TransformOperation::Perspective(v)=>v.depends_on_font_metrics(),
        TransformOperation::Interpolated{from,to,..}=>transform_depends_on_font_metrics(from)||transform_depends_on_font_metrics(to),
        TransformOperation::PendingInterpolation{from,to,..}=>transform_depends_on_font_metrics(from)||transform_depends_on_font_metrics(to),
        TransformOperation::PendingAccumulation{underlying,effect}=>transform_depends_on_font_metrics(underlying)||transform_depends_on_font_metrics(effect),
        TransformOperation::Resolved(v)=>matches!(&**v,ResolvedTransformOperation::Translate{x,y,z,..} if x.depends_on_font()||y.depends_on_font()||z.depends_on_font()),
        _=>false,
    })
}

fn transform_has_unresolved_lengths(list: &TransformList) -> bool {
    let unresolved = |value| !matches!(value, LengthValue::Computed(_));
    list.0.iter().any(|op| match op {
        TransformOperation::Translate(x,y) => unresolved(*x) || unresolved(*y),
        TransformOperation::Translate3d(x,y,z) => [*x,*y,*z].into_iter().any(unresolved),
        TransformOperation::Perspective(value) => unresolved(*value),
        TransformOperation::Interpolated{from,to,..}|TransformOperation::PendingInterpolation{from,to,..} =>
            transform_has_unresolved_lengths(from) || transform_has_unresolved_lengths(to),
        TransformOperation::PendingAccumulation{underlying,effect} =>
            transform_has_unresolved_lengths(underlying) || transform_has_unresolved_lengths(effect),
        TransformOperation::Resolved(value) => matches!(&**value,
            ResolvedTransformOperation::Translate{x,y,z,..} if x.has_unresolved_units() || y.has_unresolved_units() || z.has_unresolved_units()),
        _ => false,
    })
}

/// Source-aware dimensional classification shared by native geometry routes.
#[doc(hidden)]
pub fn transform_contains_3d(list: &TransformList) -> bool {
    list.0.iter().any(|op| match op {
        TransformOperation::Translate3d(..)
        | TransformOperation::Scale3d(..)
        | TransformOperation::Rotate3d { .. }
        | TransformOperation::Perspective(..)
        | TransformOperation::Matrix3d(..) => true,
        TransformOperation::Interpolated { from, to, .. }
        | TransformOperation::PendingInterpolation { from, to, .. } => {
            transform_contains_3d(from) || transform_contains_3d(to)
        }
        TransformOperation::PendingAccumulation { underlying, effect } => {
            transform_contains_3d(underlying) || transform_contains_3d(effect)
        }
        TransformOperation::Resolved(value) => match &**value {
            ResolvedTransformOperation::Translate { is_3d, .. }
            | ResolvedTransformOperation::Scale { is_3d, .. }
            | ResolvedTransformOperation::Rotate { is_3d, .. }
            | ResolvedTransformOperation::Matrix { is_3d, .. } => *is_3d,
            ResolvedTransformOperation::Perspective(..) => true,
        },
        _ => false,
    })
}

#[derive(Clone, Copy)]
struct Applied {
    matrix: TransformMatrix,
    axis: bool,
}
impl Applied {
    fn identity() -> Self {
        Self {
            matrix: TransformMatrix::IDENTITY,
            axis: true,
        }
    }
    fn decompose(self) -> Option<Decomposed> {
        if !self.axis {
            return self.matrix.decompose();
        }
        let x = self.matrix.0[0] as f32;
        let y = self.matrix.0[5] as f32;
        if !(x * y).is_normal() {
            return None;
        }
        let mut out = Decomposed::identity();
        out.translate = [self.matrix.0[12], self.matrix.0[13], 0.0];
        if x >= 0.0 || y >= 0.0 {
            out.scale = [x as f64, y as f64, 1.0];
        } else {
            out.scale = [-x as f64, -y as f64, 1.0];
            out.q = Quaternion([0.0, 0.0, 1.0, 0.0]);
        }
        Some(out)
    }
    fn blend(self, from: Self, p: f64) -> Option<TransformMatrix> {
        let a = from.decompose()?;
        let b = self.decompose()?;
        Some(
            Decomposed {
                translate: std::array::from_fn(|i| b.translate[i] * p + a.translate[i] * (1.0 - p)),
                scale: std::array::from_fn(|i| b.scale[i] * p + a.scale[i] * (1.0 - p)),
                skew: std::array::from_fn(|i| b.skew[i] * p + a.skew[i] * (1.0 - p)),
                perspective: std::array::from_fn(|i| {
                    b.perspective[i] * p + a.perspective[i] * (1.0 - p)
                }),
                q: a.q.slerp(b.q, p),
            }
            .compose(),
        )
    }
    fn accumulate(self, effect: Self) -> Option<TransformMatrix> {
        let a = self.decompose()?;
        let b = effect.decompose()?;
        Some(
            Decomposed {
                translate: std::array::from_fn(|i| a.translate[i] + b.translate[i]),
                scale: std::array::from_fn(|i| a.scale[i] + b.scale[i] - 1.0),
                skew: std::array::from_fn(|i| a.skew[i] + b.skew[i]),
                perspective: std::array::from_fn(|i| {
                    a.perspective[i] + b.perspective[i] - if i == 3 { 1.0 } else { 0.0 }
                }),
                q: a.q.multiply(b.q),
            }
            .compose(),
        )
    }
    fn translate(&mut self, x: f64, y: f64, z: f64) {
        let [x, y, z] = [x as f32, y as f32, z as f32];
        if self.axis && z == 0.0 {
            self.matrix.0[12] = ((self.matrix.0[12] as f32) + x * (self.matrix.0[0] as f32)) as f64;
            self.matrix.0[13] = ((self.matrix.0[13] as f32) + y * (self.matrix.0[5] as f32)) as f64;
        } else {
            self.axis = false;
            let xy = add(
                mul(self.matrix.col(0), x as f64),
                mul(self.matrix.col(1), y as f64),
            );
            let xyz = if z == 0.0 {
                xy
            } else {
                add(xy, mul(self.matrix.col(2), z as f64))
            };
            self.matrix.set_col(3, add(xyz, self.matrix.col(3)));
        }
    }
    fn scale(&mut self, values: [f64; 3]) {
        let [x, y, z] = values.map(|v| v as f32);
        if self.axis && z == 1.0 {
            self.matrix.0[0] = ((self.matrix.0[0] as f32) * x) as f64;
            self.matrix.0[5] = ((self.matrix.0[5] as f32) * y) as f64;
        } else {
            self.axis = false;
            // gfx::Transform redirects Scale3d with z==1 to the 2D
            // PreScale operation, which does not touch the depth column.
            for (i, v) in [x, y, z]
                .into_iter()
                .take(if z == 1.0 { 2 } else { 3 })
                .enumerate()
            {
                self.matrix.set_col(i, mul(self.matrix.col(i), v as f64));
            }
        }
    }
}

fn apply_remaining(list: &TransformList, size: (f64, f64), start: usize) -> Applied {
    apply_remaining_into(list, size, start, Applied::identity())
}
fn apply_remaining_into(
    list: &TransformList,
    size: (f64, f64),
    start: usize,
    mut out: Applied,
) -> Applied {
    for op in list.0.iter().skip(start) {
        if let TransformOperation::PendingInterpolation { .. }
        | TransformOperation::PendingAccumulation { .. } = op
        {
            let materialized = map_transform_lengths(&TransformList(vec![op.clone()]), &|v| v);
            assert!(
                !transform_has_unresolved_lengths(&materialized),
                "transform units must be mapped before actual-box application"
            );
            let value = apply_remaining(&materialized, size, 0);
            out = AppliedTransform::from_internal(out)
                .concat(AppliedTransform::from_internal(value))
                .internal();
            continue;
        }
        if let TransformOperation::Interpolated {
            from,
            to,
            starting_index,
            progress,
        } = op
        {
            let a = apply_remaining(from, size, *starting_index);
            let b = apply_remaining(to, size, *starting_index);
            let value =
                b.blend(a, *progress)
                    .unwrap_or(if *progress < 0.5 { a.matrix } else { b.matrix });
            out = AppliedTransform::from_internal(out)
                .concat(AppliedTransform::from_matrix(value))
                .internal();
            continue;
        }
        // Font-dependent perspective is mapped to a computed length by Engine.
        let value = resolved(op).expect("transform lengths must be resolved before application");
        match value {
            ResolvedTransformOperation::Translate { x, y, z, .. } => {
                out.translate(x.evaluate(size.0), y.evaluate(size.1), z.evaluate_scalar())
            }
            ResolvedTransformOperation::Scale { values, .. } => out.scale(values),
            ResolvedTransformOperation::Rotate { axis, degrees, .. } => {
                let matrix = rotation_matrix(axis, degrees);
                if matrix != TransformMatrix::IDENTITY {
                    out.matrix = out.matrix.concat(matrix);
                    out.axis = false;
                }
            }
            ResolvedTransformOperation::Perspective(Some(v)) => {
                out.matrix.set_col(
                    2,
                    add(out.matrix.col(2), mul(out.matrix.col(3), -1.0 / v.max(1.0))),
                );
                out.axis = false;
            }
            ResolvedTransformOperation::Perspective(None) => {}
            ResolvedTransformOperation::Matrix { value, .. } => {
                out = AppliedTransform::from_internal(out)
                    .concat(AppliedTransform::from_matrix(value))
                    .internal();
            }
        }
    }
    out
}

/// Apply actual transform operations, retaining the source representation.
/// Origin brackets follow ComputedStyle::ApplyTransform and are skipped for
/// lists containing only translation operations. Font/viewport units must be
/// mapped before this post-layout application.
#[doc(hidden)]
pub fn transform_list_applied(
    list: &TransformList,
    size: (f64, f64),
    origin: Option<(f64, f64, f64)>,
) -> AppliedTransform {
    let brackets = origin.filter(|_| {
        list.0.iter().any(|op| match op {
            TransformOperation::Translate(..) | TransformOperation::Translate3d(..) => false,
            TransformOperation::Resolved(value) => {
                !matches!(&**value, ResolvedTransformOperation::Translate { .. })
            }
            _ => true,
        })
    });
    let mut start = Applied::identity();
    if let Some((x, y, z)) = brackets {
        start.translate(x, y, z);
    }
    let mut result = apply_remaining_into(list, size, 0, start);
    if let Some((x, y, z)) = brackets {
        result.translate(-x, -y, -z);
    }
    AppliedTransform::from_internal(result)
}

/// Resolve a complete list against the actual physical reference box after
/// font-relative lengths have been mapped by the Engine. This same operation
/// applies static native lists and deferred animation suffixes.
#[doc(hidden)]
pub fn transform_list_matrix(list: &TransformList, size: (f64, f64)) -> TransformMatrix {
    apply_remaining(list, size, 0).matrix
}

#[doc(hidden)]
pub fn transform_depends_on_box(list: &TransformList) -> bool {
    suffix_depends_on_box(list, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(values: Vec<TransformOperation>) -> TransformList {
        TransformList(values)
    }
    fn translate(x: f32, y: f32) -> TransformOperation {
        TransformOperation::Translate(LengthValue::px(x), LengthValue::px(y))
    }
    fn rotate(axis: [f32; 3], degrees: f32) -> TransformOperation {
        TransformOperation::Rotate3d {
            x: axis[0],
            y: axis[1],
            z: axis[2],
            degrees,
        }
    }

    #[test]
    fn transform_calc_preserves_length_kind_and_blink_division_order_through_composition() {
        use openui_geometry::Length;
        let calculated = TransformLengthExpression::Value(LengthValue::Computed(
            Length::calc_percent_px(29.0, 1.0),
        ));
        assert_eq!((calculated.evaluate(5.0) as f32).to_bits(), 1075629260);
        let pure = TransformLengthExpression::Value(LengthValue::percent(29.0));
        let calculated_zero = TransformLengthExpression::Value(LengthValue::Computed(
            Length::calc_percent_px(29.0, 0.0),
        ));
        assert_ne!(pure.evaluate(5.0), calculated_zero.evaluate(5.0));
        let blended = TransformLengthExpression::Blend(
            Box::new(pure.clone()),
            Box::new(calculated_zero.clone()),
            0.5,
        );
        assert_eq!(blended.evaluate(5.0), calculated_zero.evaluate(5.0));
        let from_endpoint = TransformLengthExpression::Blend(
            Box::new(pure.clone()),
            Box::new(calculated_zero.clone()),
            0.0,
        );
        assert_eq!(from_endpoint.evaluate(5.0), pure.evaluate(5.0));
        let accumulated = TransformLengthExpression::Add(
            Box::new(calculated_zero),
            Box::new(TransformLengthExpression::zero()),
        );
        assert_eq!(accumulated.evaluate(5.0), pure.evaluate(5.0));
        let zero_to_percent = TransformLengthExpression::Blend(
            Box::new(TransformLengthExpression::zero()),
            Box::new(pure.clone()),
            0.5,
        );
        assert_eq!(
            zero_to_percent.evaluate(5.0),
            TransformLengthExpression::Value(LengthValue::percent(14.5)).evaluate(5.0)
        );
        let source = list(vec![TransformOperation::Translate(
            LengthValue::Computed(Length::calc_percent_px(29.0, 1.0)),
            LengthValue::px(0.0),
        )]);
        let used = transform_list_applied(&source, (5.0, 40.0), None);
        assert_eq!((used.matrix.0[12] as f32).to_bits(), 1075629260);
        let effect = list(vec![translate(1.0, 0.0)]);
        let accumulated = accumulate_transform_lists(
            &list(vec![TransformOperation::Translate(
                LengthValue::percent(29.0),
                LengthValue::px(0.0),
            )]),
            &effect,
        );
        assert_eq!(
            (transform_list_applied(&accumulated, (5.0, 40.0), None)
                .matrix
                .0[12] as f32)
                .to_bits(),
            1075629260
        );
    }

    #[test]
    fn applied_source_representation_preserves_axis_concat_and_explicit_matrix_rounding() {
        let axis = AppliedTransform::translation(16_777_216.0, 0.0)
            .concat(AppliedTransform::translation(1.0, 0.0))
            .concat(AppliedTransform::translation(-16_777_216.0, 0.0));
        assert_eq!(axis.matrix.0[12], 0.0);
        assert!(axis.axis_2d.is_some());
        let full = AppliedTransform::from_matrix(TransformMatrix::IDENTITY)
            .concat(AppliedTransform::translation(16_777_216.0, 0.0))
            .concat(AppliedTransform::translation(1.0, 0.0))
            .concat(AppliedTransform::translation(-16_777_216.0, 0.0));
        assert_eq!(full.matrix.0[12], 1.0);
        assert!(full.axis_2d.is_none());
        let explicit = transform_list_applied(
            &list(vec![TransformOperation::Matrix(Transform2D::IDENTITY)]),
            (100.0, 40.0),
            None,
        );
        assert_eq!(explicit.matrix, TransformMatrix::IDENTITY);
        assert!(explicit.axis_2d.is_none());
        assert_eq!(
            transform_list_applied(&TransformList::default(), (100.0, 40.0), None),
            AppliedTransform::IDENTITY
        );
    }

    #[test]
    fn applied_inverse_and_flatten_retain_source_arithmetic_and_singularity() {
        let axis = AppliedTransform::from_axis(AxisTransform2d {
            scale: [3.0, 2.0],
            translation: [7.0, 5.0],
        });
        let axis_inverse = axis.inverse().unwrap();
        assert_eq!(axis_inverse.axis_2d.unwrap().scale, [1.0_f32 / 3.0, 0.5]);
        assert_eq!(
            axis_inverse.matrix.0[12],
            (7.0_f32 * -(1.0_f32 / 3.0)) as f64
        );
        let full_inverse = AppliedTransform::from_matrix(axis.matrix)
            .inverse()
            .unwrap();
        assert_eq!(full_inverse.matrix.0[0], 1.0 / 3.0);
        assert_eq!(full_inverse.matrix.0[12], -14.0 * (1.0 / 6.0));
        assert_ne!(axis_inverse.matrix.0[0], full_inverse.matrix.0[0]);
        assert!(full_inverse.axis_2d.is_none());
        assert_eq!(axis.flattened(), axis);
        let mut depth = TransformMatrix::IDENTITY;
        depth.0[10] = 0.0;
        let depth = AppliedTransform::from_matrix(depth);
        assert!(!depth.is_invertible());
        assert!(depth.inverse().is_none());
        assert!(depth.flattened().is_invertible());
        assert!(depth.flattened().axis_2d.is_none());
        assert!(!AppliedTransform::from_axis(AxisTransform2d {
            scale: [f32::MIN_POSITIVE, 0.5],
            ..AxisTransform2d::IDENTITY
        })
        .is_invertible());
    }

    #[test]
    fn transform_origin_uses_the_actual_operation_loop_and_skips_translate_only_lists() {
        let origin = Some((16_777_216.0, 0.0, 0.0));
        let translation =
            transform_list_applied(&list(vec![translate(1.0, 0.0)]), (100.0, 40.0), origin);
        assert_eq!(translation.matrix.0[12], 1.0);
        let axis = transform_list_applied(
            &list(vec![
                TransformOperation::Scale(1.0, 1.0),
                translate(1.0, 0.0),
            ]),
            (100.0, 40.0),
            origin,
        );
        assert_eq!(axis.matrix.0[12], 0.0);
        assert!(axis.axis_2d.is_some());
        let full = transform_list_applied(
            &list(vec![
                TransformOperation::Matrix(Transform2D::IDENTITY),
                translate(1.0, 0.0),
            ]),
            (100.0, 40.0),
            origin,
        );
        assert_eq!(full.matrix.0[12], 1.0);
        assert!(full.axis_2d.is_none());
        let mut matrix = TransformMatrix::IDENTITY;
        matrix.0[3] = 0.125;
        let left = AppliedTransform::from_axis(AxisTransform2d {
            scale: [2.0, 3.0],
            translation: [7.0, 11.0],
        });
        let product = left.concat(AppliedTransform::from_matrix(matrix));
        assert_eq!(product.matrix.0[0], 2.0 + 0.125 * 7.0);
        assert_eq!(product.matrix.0[1], 0.125 * 11.0);
        assert_eq!(product.matrix.0[12], 7.0);
        assert_eq!(product.matrix.0[13], 11.0);
        assert!(product.axis_2d.is_none());
    }

    #[test]
    fn public_keyframe_sampling_keeps_font_and_viewport_units_until_native_resolution() {
        let from = list(vec![TransformOperation::Perspective(LengthValue::Em(1.0))]);
        let to = list(vec![TransformOperation::Perspective(LengthValue::Em(2.0))]);
        let frames = crate::PropertyKeyframes::typed(
            crate::StyleProperty::Transform,
            crate::Keyframes::from_values(from, to),
        )
        .unwrap();
        let crate::StyleValue::Transform(sample) = frames.sample(0.5, &crate::Easing::Linear)
        else {
            panic!("transform sample");
        };
        assert!(matches!(
            &sample.0[0],
            TransformOperation::PendingInterpolation { .. }
        ));
        let resolved = map_transform_lengths(&sample, &|v| match v {
            LengthValue::Em(v) => LengthValue::px(v * 20.0),
            v => v,
        });
        let depth = 1.0 / ((1.0 / 20.0) * 0.5 + (1.0 / 40.0) * 0.5);
        assert_eq!(
            transform_list_matrix(&resolved, (100.0, 80.0)).0[11],
            -1.0 / depth
        );
        let from = list(vec![TransformOperation::Translate(
            LengthValue::Ch(2.0),
            LengthValue::px(0.0),
        )]);
        let to = list(vec![TransformOperation::Scale(2.0, 2.0)]);
        let frames = crate::PropertyKeyframes::typed(
            crate::StyleProperty::Transform,
            crate::Keyframes::from_values(from.clone(), to.clone()),
        )
        .unwrap();
        let crate::StyleValue::Transform(sample) = frames.sample(0.5, &crate::Easing::Linear)
        else {
            panic!("transform sample");
        };
        let mapped = map_transform_lengths(&sample, &|v| match v {
            LengthValue::Ch(v) => LengthValue::px(v * 12.0),
            v => v,
        });
        assert_eq!(transform_list_matrix(&mapped, (100.0, 80.0)).0[12], 12.0);
        let accumulated = accumulate_transform_lists(&from, &to);
        assert!(matches!(
            &accumulated.0[0],
            TransformOperation::PendingAccumulation { .. }
        ));
        let mapped = map_transform_lengths(&accumulated, &|v| match v {
            LengthValue::Ch(v) => LengthValue::px(v * 12.0),
            v => v,
        });
        assert_eq!(transform_list_matrix(&mapped, (100.0, 80.0)).0[12], 24.0);
        let viewport = list(vec![TransformOperation::Translate(
            LengthValue::ViewportWidth(25.0),
            LengthValue::px(0.0),
        )]);
        let pending = interpolate_transform_lists(&viewport, &to, 0.5);
        let mapped = map_transform_lengths(&pending, &|v| match v {
            LengthValue::ViewportWidth(v) => LengthValue::px(v * 4.0),
            v => v,
        });
        assert_eq!(transform_list_matrix(&mapped, (100.0, 80.0)).0[12], 50.0);
    }

    #[test]
    fn legacy_renderer_transform_clears_retained_source_and_native_getter_precedes_affine_projection(
    ) {
        let mut style = crate::ComputedStyle::initial();
        let input = list(vec![TransformOperation::Rotate3d {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            degrees: 45.0,
        }]);
        crate::apply_to_computed(
            &mut style,
            crate::StyleProperty::Transform,
            &input.clone().into(),
            (400.0, 200.0),
        )
        .unwrap();
        assert_eq!(
            crate::value_from_computed(&style, crate::StyleProperty::Transform),
            crate::StyleValue::Transform(input)
        );
        let affine = Transform2D {
            e: 30.0,
            ..Transform2D::IDENTITY
        };
        crate::apply_to_computed(
            &mut style,
            crate::StyleProperty::Transform,
            &crate::StyleValue::Renderer(crate::RendererStyleValue::Transform(affine)),
            (400.0, 200.0),
        )
        .unwrap();
        assert_eq!(
            style.transform_for_reference_box(100.0, 80.0),
            TransformMatrix::from_affine(affine)
        );
        assert_eq!(style.native_transform.as_ref().unwrap().0.len(), 1);
        crate::apply_to_computed(
            &mut style,
            crate::StyleProperty::Transform,
            &crate::StyleValue::Renderer(crate::RendererStyleValue::Transform(
                Transform2D::IDENTITY,
            )),
            (400.0, 200.0),
        )
        .unwrap();
        assert!(!style.has_transform());
        assert_eq!(
            crate::value_from_computed(&style, crate::StyleProperty::Transform),
            crate::StyleValue::Transform(TransformList::default())
        );
        crate::apply_to_computed(
            &mut style,
            crate::StyleProperty::Transform,
            &list(vec![TransformOperation::Scale(1.0, 1.0)]).into(),
            (400.0, 200.0),
        )
        .unwrap();
        assert!(style.has_transform());
        assert!(style.has_transform_containing_block());
    }

    #[test]
    fn complete_static_lists_multiply_in_css_order_and_keep_depth() {
        let input = list(vec![
            TransformOperation::Scale(2.0, 3.0),
            translate(10.0, 5.0),
            TransformOperation::Rotate(90.0),
        ]);
        assert_eq!(
            transform_list_matrix(&input, (100.0, 80.0)).map_point(1.0, 2.0, 0.0),
            Some([16.0, 18.0, 0.0])
        );
        let input = list(vec![
            TransformOperation::Matrix(Transform2D {
                e: 10.0,
                ..Transform2D::IDENTITY
            }),
            TransformOperation::Translate3d(
                LengthValue::px(5.0),
                LengthValue::px(6.0),
                LengthValue::px(7.0),
            ),
            rotate([1.0, 0.0, 0.0], 90.0),
        ]);
        assert_eq!(
            transform_list_matrix(&input, (100.0, 80.0)).map_point(0.0, 1.0, 0.0),
            Some([15.0, 6.0, 8.0])
        );
    }

    #[test]
    fn compatible_translate_scale_and_identity_padding_preserve_units_and_components() {
        let a = list(vec![TransformOperation::Translate(
            LengthValue::percent(50.0),
            LengthValue::px(10.0),
        )]);
        let b = list(vec![
            TransformOperation::Translate3d(
                LengthValue::calc_percent_px(100.0, 4.0),
                LengthValue::px(20.0),
                LengthValue::px(8.0),
            ),
            TransformOperation::Scale3d(3.0, 5.0, 7.0),
        ]);
        let middle = interpolate_transform_lists(&a, &b, 0.5);
        let m = transform_list_matrix(&middle, (100.0, 80.0));
        assert_eq!(m.0[12], 77.0);
        assert_eq!(m.0[13], 15.0);
        assert_eq!(m.0[14], 4.0);
        assert_eq!([m.0[0], m.0[5], m.0[10]], [2.0, 3.0, 4.0]);
        assert_eq!(transform_list_matrix(&middle, (200.0, 80.0)).0[12], 152.0);
    }

    #[test]
    fn mismatched_percentage_suffix_uses_each_actual_box_and_keeps_prefix_on_failure() {
        let a = list(vec![TransformOperation::Translate(
            LengthValue::percent(50.0),
            LengthValue::px(0.0),
        )]);
        let b = list(vec![TransformOperation::Scale(2.0, 2.0)]);
        let middle = interpolate_transform_lists(&a, &b, 0.5);
        assert!(matches!(
            &middle.0[0],
            TransformOperation::Interpolated { .. }
        ));
        assert_eq!(transform_list_matrix(&middle, (100.0, 80.0)).0[12], 25.0);
        assert_eq!(transform_list_matrix(&middle, (200.0, 80.0)).0[12], 50.0);
        let a = list(vec![
            translate(10.0, 0.0),
            TransformOperation::Scale(0.0, 0.0),
            TransformOperation::Translate(LengthValue::percent(50.0), LengthValue::px(0.0)),
        ]);
        let b = list(vec![translate(20.0, 0.0), TransformOperation::Rotate(90.0)]);
        let interpolated = interpolate_transform_lists(&a, &b, 0.25);
        let m = transform_list_matrix(&interpolated, (100.0, 80.0));
        assert_eq!(m.0[12], 12.5);
        assert_eq!(m.0[0], 0.0);
    }

    #[test]
    fn nondependent_singular_suffix_is_discrete_but_matching_scale_is_continuous() {
        let a = list(vec![TransformOperation::Scale(0.0, 0.0)]);
        let b = list(vec![TransformOperation::Rotate(90.0)]);
        assert_eq!(interpolate_transform_lists(&a, &b, 0.25), a);
        assert_eq!(interpolate_transform_lists(&a, &b, 0.75), b);
        let b = list(vec![TransformOperation::Scale(2.0, 2.0)]);
        assert_eq!(
            transform_list_matrix(&interpolate_transform_lists(&a, &b, 0.5), (100.0, 80.0)).0[0],
            1.0
        );
    }

    #[test]
    fn accumulation_matches_components_and_uses_zero_box_only_for_matrix_suffix() {
        let a = list(vec![TransformOperation::Translate(
            LengthValue::percent(50.0),
            LengthValue::px(0.0),
        )]);
        let b = list(vec![TransformOperation::Translate(
            LengthValue::percent(25.0),
            LengthValue::px(0.0),
        )]);
        assert_eq!(
            transform_list_matrix(&accumulate_transform_lists(&a, &b), (100.0, 80.0)).0[12],
            75.0
        );
        let b = list(vec![TransformOperation::Scale(2.0, 2.0)]);
        let accumulated = accumulate_transform_lists(&a, &b);
        assert_eq!(
            transform_list_matrix(&accumulated, (100.0, 80.0)).0[12],
            0.0
        );
        assert_eq!(transform_list_matrix(&accumulated, (200.0, 80.0)).0[0], 2.0);
        let singular = list(vec![TransformOperation::Matrix3d(Transform3D([0.0; 16]))]);
        assert_eq!(accumulate_transform_lists(&singular, &b), b);
    }

    #[test]
    fn different_axes_use_quaternions_and_reflections_keep_matrix_decomposition_rules() {
        let a = list(vec![rotate([1.0, 0.0, 0.0], 90.0)]);
        let b = list(vec![rotate([0.0, 1.0, 0.0], 90.0)]);
        let middle = interpolate_transform_lists(&a, &b, 0.5);
        let TransformOperation::Resolved(value) = &middle.0[0] else {
            panic!("resolved rotation");
        };
        let ResolvedTransformOperation::Rotate { axis, degrees, .. } = &**value else {
            panic!("rotation primitive");
        };
        assert_eq!(axis[0], axis[1]);
        assert!(axis[0] > 0.0);
        assert_eq!(axis[2], 0.0);
        assert!(*degrees > 60.0 && *degrees < 90.0);
        let accumulated = accumulate_transform_lists(&a, &b);
        let TransformOperation::Resolved(value) = &accumulated.0[0] else {
            panic!("resolved rotation");
        };
        let ResolvedTransformOperation::Rotate { axis, degrees, .. } = &**value else {
            panic!("rotation primitive");
        };
        assert_eq!(axis[0], axis[1]);
        assert_eq!(axis[1], axis[2]);
        assert!(*degrees > 90.0 && *degrees < 180.0);
        let reflected = |x, y| {
            list(vec![TransformOperation::Matrix(Transform2D {
                a: x,
                d: y,
                ..Transform2D::IDENTITY
            })])
        };
        let m = transform_list_matrix(
            &accumulate_transform_lists(&reflected(-2.0, -3.0), &reflected(-4.0, -5.0)),
            (100.0, 80.0),
        );
        assert_eq!([m.0[0], m.0[5]], [5.0, 7.0]);
    }

    #[test]
    fn perspective_reciprocal_and_plane_hits_do_not_drop_depth_or_reject_flat_depth() {
        let a = list(vec![TransformOperation::Perspective(LengthValue::px(
            500.0,
        ))]);
        let none = list(vec![TransformOperation::Perspective(LengthValue::none())]);
        assert_eq!(
            transform_list_matrix(&interpolate_transform_lists(&a, &none, 0.5), (100.0, 80.0)).0
                [11],
            -0.001
        );
        assert_eq!(
            transform_list_matrix(&accumulate_transform_lists(&a, &a), (100.0, 80.0)).0[11],
            -0.004
        );
        let flat = transform_list_matrix(
            &list(vec![TransformOperation::Scale3d(1.0, 1.0, 0.0)]),
            (100.0, 80.0),
        );
        assert!(flat.inverse().is_none());
        assert_eq!(flat.unproject_plane_point(15.0, 7.0), Some([15.0, 7.0]));
        let edge_on =
            transform_list_matrix(&list(vec![rotate([0.0, 1.0, 0.0], 90.0)]), (100.0, 80.0));
        assert!(edge_on.unproject_plane_point(15.0, 7.0).is_none());
        let projected = transform_list_matrix(
            &list(vec![
                TransformOperation::Perspective(LengthValue::px(500.0)),
                rotate([0.0, 1.0, 0.0], 45.0),
            ]),
            (100.0, 80.0),
        );
        let screen = projected.map_point(20.0, 10.0, 0.0).unwrap();
        let local = projected
            .unproject_plane_point(screen[0], screen[1])
            .unwrap();
        // Round-trip assertions are relational geometry guards, not pixel
        // qualification or a relaxed renderer comparison.
        assert_eq!(local[0] as f32, 20.0);
        assert_eq!(local[1] as f32, 10.0);
    }

    #[test]
    fn selected_font_units_survive_deferred_owned_endpoints() {
        let from = list(vec![TransformOperation::Translate(
            LengthValue::Ch(2.0),
            LengthValue::percent(50.0),
        )]);
        let to = list(vec![TransformOperation::Scale(2.0, 2.0)]);
        let middle = interpolate_transform_lists(&from, &to, 0.5);
        assert!(transform_depends_on_font_metrics(&middle));
        let mapped = map_transform_lengths(&middle, &|value| match value {
            LengthValue::Ch(v) => LengthValue::px(v * 9.0),
            v => v,
        });
        assert!(!transform_depends_on_font_metrics(&mapped));
        let matrix = transform_list_matrix(&mapped, (100.0, 80.0));
        assert_eq!(matrix.0[12], 9.0);
        assert_eq!(matrix.0[13], 20.0);
    }
}
