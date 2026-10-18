//! Native independent corner axes. Computed lengths keep their source kind;
//! box overlap reduction and used-value clamping belong to the contour consumer.
use crate::{ComputedStyle, LengthValue, StyleProperty};
use openui_geometry::Length;

#[doc(hidden)]
pub const RADIUS_AXIS_PROPERTIES: [StyleProperty; 8] = [
    StyleProperty::BorderTopLeftRadiusX,
    StyleProperty::BorderTopLeftRadiusY,
    StyleProperty::BorderTopRightRadiusX,
    StyleProperty::BorderTopRightRadiusY,
    StyleProperty::BorderBottomRightRadiusX,
    StyleProperty::BorderBottomRightRadiusY,
    StyleProperty::BorderBottomLeftRadiusX,
    StyleProperty::BorderBottomLeftRadiusY,
];

impl StyleProperty {
    /// Physical corner order TL/TR/BR/BL, then horizontal/vertical axis.
    #[doc(hidden)]
    pub const fn radius_axis_coordinates(self) -> Option<(usize, usize)> {
        use StyleProperty::*;
        Some(match self {
            BorderTopLeftRadiusX => (0, 0),
            BorderTopLeftRadiusY => (0, 1),
            BorderTopRightRadiusX => (1, 0),
            BorderTopRightRadiusY => (1, 1),
            BorderBottomRightRadiusX => (2, 0),
            BorderBottomRightRadiusY => (2, 1),
            BorderBottomLeftRadiusX => (3, 0),
            BorderBottomLeftRadiusY => (3, 1),
            _ => return None,
        })
    }
}

impl LengthValue {
    /// New radius-axis declarations reject auto/none and negative simple
    /// lengths. Finite calc coefficients keep their expression; its used
    /// radius is clamped after the actual border-box basis is available.
    pub fn is_valid_for_radius_axis(self) -> bool {
        self.is_valid_for_line_height()
    }
}

impl ComputedStyle {
    fn radius_axis_source(&self, corner: usize, axis: usize) -> Length {
        if let Some((x, y)) = self.fields.border_radius_lengths[corner] {
            return if axis == 0 { x } else { y };
        }
        let numeric = [
            self.fields.border_top_left_radius,
            self.fields.border_top_right_radius,
            self.fields.border_bottom_right_radius,
            self.fields.border_bottom_left_radius,
        ][corner];
        let flag = self.fields.border_radius_percent[corner];
        let (value, percent) = if axis == 0 {
            (numeric.0, flag.0)
        } else {
            (numeric.1, flag.1)
        };
        if percent {
            Length::percent(value)
        } else {
            Length::px(value)
        }
    }

    /// Return the computed source length for one native axis. Percent/calc
    /// remain unresolved against the box; font/viewport units have been
    /// resolved by the Engine's ordinary declaration recomposition.
    pub fn border_radius_axis_length(&self, property: StyleProperty) -> Option<LengthValue> {
        let (corner, axis) = property.radius_axis_coordinates()?;
        Some(self.radius_axis_source(corner, axis).into())
    }

    pub(crate) fn set_border_radius_axis_resolved(
        &mut self,
        corner: usize,
        axis: usize,
        length: Length,
    ) {
        let mut pair = (
            self.radius_axis_source(corner, 0),
            self.radius_axis_source(corner, 1),
        );
        if axis == 0 {
            pair.0 = length;
        } else {
            pair.1 = length;
        }
        self.fields.border_radius_lengths[corner] = Some(pair);
        let numeric = match corner {
            0 => &mut self.fields.border_top_left_radius,
            1 => &mut self.fields.border_top_right_radius,
            2 => &mut self.fields.border_bottom_right_radius,
            3 => &mut self.fields.border_bottom_left_radius,
            _ => unreachable!("owned physical corner"),
        };
        if axis == 0 {
            numeric.0 = length.value();
            self.fields.border_radius_percent[corner].0 = length.is_percent();
        } else {
            numeric.1 = length.value();
            self.fields.border_radius_percent[corner].1 = length.is_percent();
        }
    }

    pub(crate) fn copy_border_radius_corner_from(&mut self, source: &Self, corner: usize) {
        self.fields.border_radius_lengths[corner] = source.fields.border_radius_lengths[corner];
        self.fields.border_radius_percent[corner] = source.fields.border_radius_percent[corner];
        match corner {
            0 => self.fields.border_top_left_radius = source.fields.border_top_left_radius,
            1 => self.fields.border_top_right_radius = source.fields.border_top_right_radius,
            2 => self.fields.border_bottom_right_radius = source.fields.border_bottom_right_radius,
            3 => self.fields.border_bottom_left_radius = source.fields.border_bottom_left_radius,
            _ => unreachable!("owned physical corner"),
        }
    }

    // Generated canonical masks copy one logical raw component rather than
    // replacing a paired source. If the target had only legacy fields, its
    // other component is materialized without changing its effective value.
    pub(crate) fn copy_border_radius_axis_from(
        &mut self,
        source: &Self,
        corner: usize,
        axis: usize,
    ) {
        let length = source.radius_axis_source(corner, axis);
        self.set_border_radius_axis_resolved(corner, axis, length);
        let original = [
            source.fields.border_top_left_radius,
            source.fields.border_top_right_radius,
            source.fields.border_bottom_right_radius,
            source.fields.border_bottom_left_radius,
        ][corner];
        let numeric = match corner {
            0 => &mut self.fields.border_top_left_radius,
            1 => &mut self.fields.border_top_right_radius,
            2 => &mut self.fields.border_bottom_right_radius,
            3 => &mut self.fields.border_bottom_left_radius,
            _ => unreachable!("owned physical corner"),
        };
        if axis == 0 {
            numeric.0 = original.0;
            self.fields.border_radius_percent[corner].0 =
                source.fields.border_radius_percent[corner].0;
        } else {
            numeric.1 = original.1;
            self.fields.border_radius_percent[corner].1 =
                source.fields.border_radius_percent[corner].1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{apply_to_computed, parse_literal, value_from_computed, StyleValue};
    #[test]
    fn native_axes_keep_independent_source_kinds_and_checked_values() {
        let mut style = ComputedStyle::initial();
        let values = [
            LengthValue::percent(25.0),
            LengthValue::px(8.0),
            LengthValue::calc_percent_px(50.0, -3.0),
            LengthValue::Em(1.0),
            LengthValue::ViewportWidth(10.0),
            LengthValue::Rem(1.0),
            LengthValue::px(0.0),
            LengthValue::percent(20.0),
        ];
        for (index, (property, value)) in RADIUS_AXIS_PROPERTIES.into_iter().zip(values).enumerate()
        {
            assert_eq!(property as u16, 251 + index as u16);
            apply_to_computed(&mut style, property, &value.into(), (200.0, 100.0)).unwrap();
        }
        assert_eq!(
            style.border_radii_for_box((100.0, 80.0)),
            [(25.0, 8.0), (47.0, 16.0), (20.0, 16.0), (0.0, 16.0)]
        );
        assert_eq!(
            value_from_computed(&style, StyleProperty::BorderTopRightRadiusX),
            LengthValue::calc_percent_px(50.0, -3.0).into()
        );
        assert_eq!(style.border_radii_for_box((200.0, 40.0))[0], (50.0, 8.0));
        assert_eq!(
            parse_literal(StyleProperty::BorderTopLeftRadiusX, "25%").unwrap(),
            LengthValue::percent(25.0).into()
        );
        let before = style.clone();
        for invalid in [
            LengthValue::px(-1.0),
            LengthValue::px(f32::NAN),
            LengthValue::percent(f32::INFINITY),
            LengthValue::auto(),
            LengthValue::none(),
            LengthValue::Em(-1.0),
            LengthValue::calc_percent_px(0.0, f32::NAN),
        ] {
            assert!(apply_to_computed(
                &mut style,
                StyleProperty::BorderTopLeftRadiusX,
                &StyleValue::Length(invalid),
                (200.0, 100.0)
            )
            .is_err());
            assert_eq!(
                style.fields.border_radius_lengths,
                before.fields.border_radius_lengths
            );
            assert_eq!(
                style.fields.border_radius_percent,
                before.fields.border_radius_percent
            );
            assert_eq!(
                [
                    style.fields.border_top_left_radius,
                    style.fields.border_top_right_radius,
                    style.fields.border_bottom_right_radius,
                    style.fields.border_bottom_left_radius,
                ],
                [
                    before.fields.border_top_left_radius,
                    before.fields.border_top_right_radius,
                    before.fields.border_bottom_right_radius,
                    before.fields.border_bottom_left_radius,
                ]
            );
            for property in RADIUS_AXIS_PROPERTIES {
                assert_eq!(
                    value_from_computed(&style, property),
                    value_from_computed(&before, property),
                    "invalid {invalid:?} changed {property:?}"
                );
            }
        }
        assert!(parse_literal(StyleProperty::BorderTopLeftRadiusX, "auto").is_err());
        assert!(style
            .border_radius_axis_length(StyleProperty::Width)
            .is_none());
    }
}
