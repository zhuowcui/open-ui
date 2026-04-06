//! LayoutUnit — Blink's fixed-point arithmetic type (1/64th pixel precision).
//!
//! Source: third_party/blink/renderer/platform/geometry/layout_unit.h
//!
//! Blink's `LayoutUnit` is `FixedPoint<6, int32_t>`: a fixed-point number
//! using an i32 with 6 fractional bits. One raw unit = 1/64th of a CSS pixel.
//!
//! Every constructor, operator, and rounding function here is extracted
//! character-by-character from Blink's source to produce identical results.

/// Fixed-point arithmetic type: 26 integer bits + 6 fractional bits.
///
/// Range: roughly -33,554,432 to +33,554,431.984375 pixels.
/// Precision: 1/64th of a pixel (0.015625).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LayoutUnit(i32);

const FRACTIONAL_BITS: u32 = 6;
const DENOMINATOR: i32 = 1 << FRACTIONAL_BITS; // 64
const RAW_MAX: i32 = i32::MAX;
const RAW_MIN: i32 = i32::MIN;
const INT_MAX: i32 = RAW_MAX / DENOMINATOR;
const INT_MIN: i32 = RAW_MIN / DENOMINATOR;

impl LayoutUnit {
    // ── Constants ────────────────────────────────────────────────────

    #[inline]
    pub const fn zero() -> Self { Self(0) }

    /// The smallest positive value: 1/64 pixel = 0.015625.
    #[inline]
    pub const fn epsilon() -> Self { Self(1) }

    /// Maximum representable value (i32::MAX raw).
    #[inline]
    pub const fn max() -> Self { Self(RAW_MAX) }

    /// Minimum representable value (i32::MIN raw).
    #[inline]
    pub const fn min() -> Self { Self(RAW_MIN) }

    /// Blink's `NearlyMax()` — slightly less than max to allow rounding.
    #[inline]
    pub const fn nearly_max() -> Self { Self(RAW_MAX - DENOMINATOR / 2) }

    /// Blink's `NearlyMin()` — slightly more than min to allow rounding.
    #[inline]
    pub const fn nearly_min() -> Self { Self(RAW_MIN + DENOMINATOR / 2) }

    // ── Constructors (matching Blink exactly) ────────────────────────

    /// From raw internal representation.
    /// Matches Blink's `FromRawValue(Storage)`.
    #[inline]
    pub const fn from_raw(raw: i32) -> Self { Self(raw) }

    /// From raw with clamping to i32 range.
    /// Matches Blink's `FromRawValueWithClamp(T)`.
    #[inline]
    pub fn from_raw_i64(raw: i64) -> Self {
        Self(raw.clamp(RAW_MIN as i64, RAW_MAX as i64) as i32)
    }

    /// From signed integer pixels — saturating shift left by 6.
    /// Matches Blink's `FixedPoint(std::signed_integral auto value)`.
    ///
    /// Blink's `SaturatedSetNonAsm(int)`:
    /// ```c++
    /// if (value > kIntMax) value_ = kRawValueMax;
    /// else if (value < kIntMin) value_ = kRawValueMin;
    /// else value_ = static_cast<unsigned>(value) << kFractionalBits;
    /// ```
    #[inline]
    pub const fn from_i32(value: i32) -> Self {
        if value > INT_MAX {
            Self(RAW_MAX)
        } else if value < INT_MIN {
            Self(RAW_MIN)
        } else {
            // Blink casts to unsigned first, then shifts, then stores in i32.
            // In Rust we replicate by wrapping_shl on the i32 (same bit pattern).
            Self((value as u32).wrapping_shl(FRACTIONAL_BITS) as i32)
        }
    }

    /// From float pixels — **TRUNCATES** toward zero (C cast semantics).
    /// Matches Blink's `FixedPoint(float value)`:
    /// ```c++
    /// value_(ClampRawValue(value * kFixedPointDenominator))
    /// ```
    /// ClampRawValue uses `base::saturated_cast<Storage>` which truncates.
    #[inline]
    pub fn from_f32(value: f32) -> Self {
        let scaled = value * DENOMINATOR as f32;
        // C truncation semantics: cast to int truncates toward zero.
        // saturated_cast clamps to [i32::MIN, i32::MAX] before truncating.
        if scaled.is_nan() {
            return Self(0);
        }
        let clamped = scaled.clamp(RAW_MIN as f32, RAW_MAX as f32);
        Self(clamped as i32) // Rust `as i32` truncates toward zero, same as C
    }

    /// From double pixels — truncates toward zero.
    /// Matches Blink's `FixedPoint(double value)`.
    #[inline]
    pub fn from_f64(value: f64) -> Self {
        let scaled = value * DENOMINATOR as f64;
        if scaled.is_nan() {
            return Self(0);
        }
        let clamped = scaled.clamp(RAW_MIN as f64, RAW_MAX as f64);
        Self(clamped as i32)
    }

    /// From float, rounding UP (ceiling).
    /// Matches Blink's `FromFloatCeil(float)`.
    #[inline]
    pub fn from_f32_ceil(value: f32) -> Self {
        let scaled = (value * DENOMINATOR as f32).ceil();
        Self::from_raw_i64(scaled as i64)
    }

    /// From float, rounding DOWN (floor).
    /// Matches Blink's `FromFloatFloor(float)`.
    #[inline]
    pub fn from_f32_floor(value: f32) -> Self {
        let scaled = (value * DENOMINATOR as f32).floor();
        Self::from_raw_i64(scaled as i64)
    }

    /// From float, rounding to NEAREST.
    /// Matches Blink's `FromFloatRound(float)`.
    #[inline]
    pub fn from_f32_round(value: f32) -> Self {
        let scaled = (value * DENOMINATOR as f32).round();
        Self::from_raw_i64(scaled as i64)
    }

    // ── Access ───────────────────────────────────────────────────────

    /// Raw internal value (in 1/64-pixel units).
    #[inline]
    pub const fn raw(self) -> i32 { self.0 }

    /// Convert to float. Matches Blink's `ToFloat()`:
    /// `static_cast<float>(value_) / kFixedPointDenominator`
    #[inline]
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / DENOMINATOR as f32
    }

    /// Convert to double. Matches Blink's `ToDouble()`.
    #[inline]
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / DENOMINATOR as f64
    }

    /// Truncate to integer, toward zero. Matches Blink's `ToInt()`:
    /// `value_ / kFixedPointDenominator`
    #[inline]
    pub const fn to_i32(self) -> i32 {
        self.0 / DENOMINATOR
    }

    // ── Rounding (matching Blink's Ceil/Floor/Round exactly) ─────────

    /// Ceiling: round up to the nearest integer.
    /// Matches Blink's `Ceil()`:
    /// ```c++
    /// if (value_ >= 0) return (value_ + 63) / 64;
    /// return ToInt(); // negative: truncate toward zero = ceiling for negatives
    /// ```
    #[inline]
    pub fn ceil(self) -> LayoutUnit {
        if self.0 >= RAW_MAX - DENOMINATOR + 1 {
            return Self::from_i32(INT_MAX);
        }
        if self.0 >= 0 {
            Self::from_i32((self.0 + DENOMINATOR - 1) / DENOMINATOR)
        } else {
            Self::from_i32(self.to_i32())
        }
    }

    /// Ceiling to integer value directly.
    #[inline]
    pub fn ceil_i32(self) -> i32 {
        if self.0 >= RAW_MAX - DENOMINATOR + 1 {
            return INT_MAX;
        }
        if self.0 >= 0 {
            (self.0 + DENOMINATOR - 1) / DENOMINATOR
        } else {
            self.to_i32()
        }
    }

    /// Floor: round down toward negative infinity.
    /// Matches Blink's `Floor()`:
    /// `value_ >> kFractionalBits` (arithmetic shift).
    #[inline]
    pub fn floor(self) -> LayoutUnit {
        if self.0 <= RAW_MIN + DENOMINATOR - 1 {
            return Self::from_i32(INT_MIN);
        }
        Self::from_i32(self.0 >> FRACTIONAL_BITS)
    }

    /// Floor to integer value directly.
    #[inline]
    pub fn floor_i32(self) -> i32 {
        if self.0 <= RAW_MIN + DENOMINATOR - 1 {
            return INT_MIN;
        }
        self.0 >> FRACTIONAL_BITS
    }

    /// Round to nearest integer.
    /// Matches Blink's `Round()`:
    /// `ToInt() + ((Fraction().RawValue() + 32) >> 6)`
    #[inline]
    pub fn round_i32(self) -> i32 {
        self.to_i32() + ((self.fraction().0 + (DENOMINATOR / 2)) >> FRACTIONAL_BITS)
    }

    /// Fractional part only (always non-negative for positive values,
    /// sign-preserving for negative, matching C++ `%` semantics).
    /// Matches Blink's `Fraction()`:
    /// `FromRawValue(RawValue() % kFixedPointDenominator)`
    #[inline]
    pub const fn fraction(self) -> Self {
        Self(self.0 % DENOMINATOR)
    }

    // ── Utility ──────────────────────────────────────────────────────

    #[inline]
    pub fn abs(self) -> Self {
        Self(self.0.wrapping_abs()) // Blink uses ::abs()
    }

    /// Clamp negative values to zero.
    /// Matches Blink's `ClampNegativeToZero()`.
    #[inline]
    pub fn clamp_negative_to_zero(self) -> Self {
        if self.0 < 0 { Self(0) } else { self }
    }

    /// Clamp indefinite (-1 pixel) to zero.
    /// Matches Blink's `ClampIndefiniteToZero()`.
    #[inline]
    pub fn clamp_indefinite_to_zero(self) -> Self {
        if self.is_indefinite() { Self(0) } else { self }
    }

    #[inline]
    pub fn max_of(self, other: Self) -> Self {
        if self.0 >= other.0 { self } else { other }
    }

    #[inline]
    pub fn min_of(self, other: Self) -> Self {
        if self.0 <= other.0 { self } else { other }
    }

    /// Clamp between min and max.
    /// CSS 2.1 §10.4/§10.7: when min > max, max is treated as min.
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        let effective_max = max.max_of(min);
        self.max_of(min).min_of(effective_max)
    }

    /// Blink's `kIndefiniteSize` is `LayoutUnit(-1)` = raw -64.
    #[inline]
    pub fn is_indefinite(self) -> bool {
        self.0 == -DENOMINATOR
    }

    /// Add one epsilon (1/64 px).
    /// Matches Blink's `AddEpsilon()`.
    #[inline]
    pub fn add_epsilon(self) -> Self {
        if self.0 < RAW_MAX { Self(self.0 + 1) } else { self }
    }

    /// Fused multiply-divide: `(self * m) / d` using i64 intermediate.
    /// Matches Blink's `MulDiv(FixedPoint m, FixedPoint d)`:
    /// ```c++
    /// int64_t n = (int64_t)RawValue() * m.RawValue();
    /// int64_t q = n / d.RawValue();
    /// return FromRawValueWithClamp(q);
    /// ```
    #[inline]
    pub fn mul_div(self, m: Self, d: Self) -> Self {
        let n = self.0 as i64 * m.0 as i64;
        let q = n / d.0 as i64;
        Self::from_raw_i64(q)
    }
}

/// Blink's `kIndefiniteSize` — represents an unconstrained/undefined dimension.
/// `LayoutUnit(-1)` = raw value -64.
#[allow(dead_code)]
pub const INDEFINITE_SIZE: LayoutUnit = LayoutUnit(-DENOMINATOR);

// ── Arithmetic operators (matching Blink's exact semantics) ──────────

impl std::ops::Add for LayoutUnit {
    type Output = Self;
    /// Saturating add. Matches Blink's `operator+` which uses `base::ClampAdd`.
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::AddAssign for LayoutUnit {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl std::ops::Sub for LayoutUnit {
    type Output = Self;
    /// Saturating subtract.
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl std::ops::SubAssign for LayoutUnit {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl std::ops::Neg for LayoutUnit {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

/// LayoutUnit * LayoutUnit — the critical operation.
/// Matches Blink's `BoundedMultiply`:
/// ```c++
/// int64_t result = (int64_t)a.RawValue() * (int64_t)b.RawValue() / kFixedPointDenominator;
/// int32_t high = (int32_t)(result >> 32);
/// int32_t low = (int32_t)result;
/// uint32_t saturated = ((uint32_t)(a.RawValue() ^ b.RawValue()) >> 31) + kRawValueMax;
/// if (high != low >> 31) result = saturated;
/// ```
impl std::ops::Mul for LayoutUnit {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let result = self.0 as i64 * rhs.0 as i64 / DENOMINATOR as i64;
        let high = (result >> 32) as i32;
        let low = result as i32;
        // Overflow detection: if high bits don't match sign extension of low bits
        if high != (low >> 31) {
            // Saturate: positive overflow → MAX, negative overflow → MIN
            let saturated = ((self.0 as u32 ^ rhs.0 as u32) >> 31).wrapping_add(RAW_MAX as u32);
            Self(saturated as i32)
        } else {
            Self(result as i32)
        }
    }
}

/// LayoutUnit * integer — clamping multiply.
/// Matches Blink's `operator*(FixedPoint, integral)`: `ClampMul(a.RawValue(), b)`.
impl std::ops::Mul<i32> for LayoutUnit {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Self(self.0.saturating_mul(rhs))
    }
}

/// LayoutUnit * float — convert to float, multiply, convert back.
impl std::ops::Mul<f32> for LayoutUnit {
    type Output = f32;
    #[inline]
    fn mul(self, rhs: f32) -> f32 {
        self.to_f32() * rhs
    }
}

/// LayoutUnit / LayoutUnit.
/// Matches Blink's `operator/`:
/// ```c++
/// int64_t raw_val = (int64_t)kFixedPointDenominator * a.RawValue() / b.RawValue();
/// return FromRawValueWithClamp(raw_val);
/// ```
impl std::ops::Div for LayoutUnit {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        let raw_val = DENOMINATOR as i64 * self.0 as i64 / rhs.0 as i64;
        Self::from_raw_i64(raw_val)
    }
}

/// LayoutUnit / integer.
impl std::ops::Div<i32> for LayoutUnit {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Self(self.0 / rhs)
    }
}

impl std::fmt::Debug for LayoutUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LayoutUnit({:.4}px [raw={}])", self.to_f32(), self.0)
    }
}

impl std::fmt::Display for LayoutUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}px", self.to_f32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_bits_and_denominator() {
        assert_eq!(FRACTIONAL_BITS, 6);
        assert_eq!(DENOMINATOR, 64);
    }

    #[test]
    fn from_integer_saturating() {
        let u = LayoutUnit::from_i32(10);
        assert_eq!(u.raw(), 640); // 10 * 64
        assert_eq!(u.to_i32(), 10);
        assert_eq!(u.to_f32(), 10.0);

        // Overflow saturates
        let big = LayoutUnit::from_i32(i32::MAX);
        assert_eq!(big.raw(), i32::MAX);
    }

    #[test]
    fn from_float_truncates_not_rounds() {
        // CRITICAL: Blink truncates toward zero, does NOT round.
        let u = LayoutUnit::from_f32(2.7);
        // 2.7 * 64 = 172.8 → truncate to 172
        assert_eq!(u.raw(), 172);
        assert_eq!(u.to_i32(), 2); // 172 / 64 = 2 (truncated)

        // Negative truncation
        let v = LayoutUnit::from_f32(-2.7);
        // -2.7 * 64 = -172.8 → truncate to -172
        assert_eq!(v.raw(), -172);
        assert_eq!(v.to_i32(), -2); // -172 / 64 = -2 (truncated toward zero)
    }

    #[test]
    fn from_float_ceil_floor_round() {
        let ceil = LayoutUnit::from_f32_ceil(2.7);
        // 2.7 * 64 = 172.8 → ceil to 173
        assert_eq!(ceil.raw(), 173);

        let floor = LayoutUnit::from_f32_floor(2.7);
        // 2.7 * 64 = 172.8 → floor to 172
        assert_eq!(floor.raw(), 172);

        let round = LayoutUnit::from_f32_round(2.7);
        // 2.7 * 64 = 172.8 → round to 173
        assert_eq!(round.raw(), 173);
    }

    #[test]
    fn nan_produces_zero() {
        let u = LayoutUnit::from_f32(f32::NAN);
        assert_eq!(u.raw(), 0);
    }

    #[test]
    fn sub_pixel_precision() {
        let u = LayoutUnit::from_f32(0.015625); // 1/64
        assert_eq!(u.raw(), 1);
        assert_eq!(u, LayoutUnit::epsilon());
    }

    #[test]
    fn ceil_positive() {
        // Blink: positive → (value + 63) / 64
        let u = LayoutUnit::from_f32(10.3);
        let c = u.ceil_i32();
        assert_eq!(c, 11);
    }

    #[test]
    fn ceil_negative() {
        // Blink: negative → ToInt() (truncate toward zero = ceiling for negatives)
        let u = LayoutUnit::from_f32(-10.3);
        let c = u.ceil_i32();
        assert_eq!(c, -10); // Toward zero
    }

    #[test]
    fn floor_positive() {
        let u = LayoutUnit::from_f32(10.9);
        assert_eq!(u.floor_i32(), 10);
    }

    #[test]
    fn floor_negative() {
        // Arithmetic right shift: -10.3 → floor to -11
        let u = LayoutUnit::from_f32(-10.3);
        assert_eq!(u.floor_i32(), -11); // Toward negative infinity
    }

    #[test]
    fn round_to_nearest() {
        let u = LayoutUnit::from_f32(10.3);
        assert_eq!(u.round_i32(), 10);

        let v = LayoutUnit::from_f32(10.7);
        assert_eq!(v.round_i32(), 11);
    }

    #[test]
    fn saturating_add() {
        let a = LayoutUnit::max();
        let b = LayoutUnit::from_i32(1);
        assert_eq!((a + b).raw(), i32::MAX);
    }

    #[test]
    fn multiply_layout_units() {
        let a = LayoutUnit::from_i32(3);
        let b = LayoutUnit::from_i32(4);
        let result = a * b;
        assert_eq!(result.to_i32(), 12);

        // Fractional multiply
        let c = LayoutUnit::from_f32(2.5);
        let d = LayoutUnit::from_f32(4.0);
        let r2 = c * d;
        assert_eq!(r2.to_f32(), 10.0);
    }

    #[test]
    fn multiply_overflow_saturates() {
        let a = LayoutUnit::max();
        let b = LayoutUnit::from_i32(2);
        let result = a * b;
        // Should saturate to max, not wrap
        assert!(result.raw() == i32::MAX || result.raw() == i32::MIN);
    }

    #[test]
    fn divide_layout_units() {
        let a = LayoutUnit::from_i32(12);
        let b = LayoutUnit::from_i32(4);
        let result = a / b;
        assert_eq!(result.to_i32(), 3);

        // Fractional result
        let c = LayoutUnit::from_i32(10);
        let d = LayoutUnit::from_i32(4);
        let r2 = c / d;
        assert_eq!(r2.to_f32(), 2.5);
    }

    #[test]
    fn indefinite_size() {
        assert_eq!(INDEFINITE_SIZE.raw(), -64);
        assert!(INDEFINITE_SIZE.is_indefinite());
        assert_eq!(INDEFINITE_SIZE.clamp_indefinite_to_zero(), LayoutUnit::zero());
    }

    #[test]
    fn fraction_preserves_sign() {
        // Blink uses % operator which preserves dividend sign in C++
        let pos = LayoutUnit::from_f32(10.5);
        assert_eq!(pos.fraction().raw(), 32); // 0.5 * 64 = 32

        let neg = LayoutUnit::from_f32(-10.5);
        assert_eq!(neg.fraction().raw(), -32); // Sign preserved
    }

    #[test]
    fn margin_collapsing_math() {
        let m1 = LayoutUnit::from_i32(20);
        let m2 = LayoutUnit::from_i32(30);
        assert_eq!(m1.max_of(m2), LayoutUnit::from_i32(30));
    }

    #[test]
    fn mul_div_fused() {
        let a = LayoutUnit::from_i32(100);
        let m = LayoutUnit::from_i32(3);
        let d = LayoutUnit::from_i32(4);
        let result = a.mul_div(m, d);
        assert_eq!(result.to_i32(), 75); // 100 * 3 / 4
    }
}

