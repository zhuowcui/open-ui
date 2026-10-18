//! Bounded native value-literal adapter for the existing owned 2D API.
//!
//! Supported functions are translate/translateX/translateY, scale/scaleX/scaleY,
//! rotate and matrix, in authored order. Arguments use commas; translate keeps
//! native px/percent/font/viewport units; scale accepts numbers or percentages;
//! rotate accepts deg/rad/grad/turn (and unitless zero). Function names/units are
//! ASCII case-insensitive. Math, comments, escapes, skew and 3D textual forms
//! are not accepted. Every existing typed TransformList operation remains usable.
//! This helper is not a CSS stylesheet parser or a browser runtime.

use crate::{LengthValue, Transform2D, TransformList, TransformOperation};

// Keep the same bounded operation count as the existing C owned constructor.
const MAX_OPERATIONS: usize = 4096;

fn css_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}
fn trim(value: &str) -> &str {
    value.trim_matches(css_space)
}

// CSS number tokens, rather than Rust's broader float spellings. The native
// payload is f32; reject non-finite conversions before retaining an operation.
fn numeric(value: &str) -> Option<f64> {
    let bytes = value.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut digits = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
        // A decimal point requires following digits in a CSS number token.
        if i == start {
            return None;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    if i != bytes.len() {
        return None;
    }
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}
fn finite_f32(value: f64) -> Option<f32> {
    let value = value as f32;
    value.is_finite().then_some(value)
}
fn number(value: &str) -> Option<f32> {
    finite_f32(numeric(value)?)
}
fn suffix<'a>(value: &'a str, unit: &str) -> Option<&'a str> {
    let split = value.len().checked_sub(unit.len())?;
    let (number, end) = (value.get(..split)?, value.get(split..)?);
    end.eq_ignore_ascii_case(unit).then_some(number)
}
fn scale(value: &str) -> Option<f32> {
    if let Some(percent) = suffix(value, "%") {
        finite_f32(numeric(percent)? / 100.0)
    } else {
        number(value)
    }
}
fn length(value: &str) -> Option<LengthValue> {
    if let Some(zero) = numeric(value).filter(|value| *value == 0.0) {
        return Some(LengthValue::px(finite_f32(zero)?));
    }
    for (unit, make) in [
        ("px", LengthValue::px as fn(f32) -> LengthValue),
        ("%", LengthValue::percent),
        ("em", LengthValue::Em),
        ("rem", LengthValue::Rem),
        ("vw", LengthValue::ViewportWidth),
        ("vh", LengthValue::ViewportHeight),
        ("vmin", LengthValue::ViewportMin),
        ("vmax", LengthValue::ViewportMax),
        ("ch", LengthValue::Ch),
        ("ex", LengthValue::Ex),
        ("lh", LengthValue::Lh),
    ] {
        if let Some(value) = suffix(value, unit).and_then(number) {
            return Some(make(value));
        }
    }
    None
}
fn angle(value: &str) -> Option<f32> {
    if let Some(zero) = numeric(value).filter(|value| *value == 0.0) {
        return finite_f32(zero);
    }
    for (unit, degrees) in [
        ("deg", 1.0),
        ("rad", 180.0 / std::f64::consts::PI),
        ("grad", 0.9),
        ("turn", 360.0),
    ] {
        if let Some(value) = suffix(value, unit).and_then(numeric) {
            return finite_f32(value * degrees);
        }
    }
    None
}
fn operation(name: &str, args: &[&str]) -> Option<TransformOperation> {
    let zero = LengthValue::px(0.0);
    if name.eq_ignore_ascii_case("translate") {
        match args {
            [x] => Some(TransformOperation::Translate(length(x)?, zero)),
            [x, y] => Some(TransformOperation::Translate(length(x)?, length(y)?)),
            _ => None,
        }
    } else if name.eq_ignore_ascii_case("translateX") && args.len() == 1 {
        Some(TransformOperation::Translate(length(args[0])?, zero))
    } else if name.eq_ignore_ascii_case("translateY") && args.len() == 1 {
        Some(TransformOperation::Translate(zero, length(args[0])?))
    } else if name.eq_ignore_ascii_case("scale") {
        match args {
            [x] => {
                let x = scale(x)?;
                Some(TransformOperation::Scale(x, x))
            }
            [x, y] => Some(TransformOperation::Scale(scale(x)?, scale(y)?)),
            _ => None,
        }
    } else if name.eq_ignore_ascii_case("scaleX") && args.len() == 1 {
        Some(TransformOperation::Scale(scale(args[0])?, 1.0))
    } else if name.eq_ignore_ascii_case("scaleY") && args.len() == 1 {
        Some(TransformOperation::Scale(1.0, scale(args[0])?))
    } else if name.eq_ignore_ascii_case("rotate") && args.len() == 1 {
        Some(TransformOperation::Rotate(angle(args[0])?))
    } else if name.eq_ignore_ascii_case("matrix") && args.len() == 6 {
        Some(TransformOperation::Matrix(Transform2D {
            a: number(args[0])?,
            b: number(args[1])?,
            c: number(args[2])?,
            d: number(args[3])?,
            e: number(args[4])?,
            f: number(args[5])?,
        }))
    } else {
        None
    }
}

pub(crate) fn parse(input: &str) -> Option<TransformList> {
    let mut remaining = trim(input);
    if remaining.eq_ignore_ascii_case("none") {
        return Some(TransformList::default());
    }
    let mut operations = Vec::new();
    while !remaining.is_empty() {
        if operations.len() == MAX_OPERATIONS {
            return None;
        }
        let open = remaining.find('(')?;
        let name = &remaining[..open];
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
        let tail = &remaining[open + 1..];
        let close = tail.find(')')?;
        let body = &tail[..close];
        if body.contains('(') {
            return None;
        }
        let mut args = [""; 6];
        let mut count = 0;
        for value in body.split(',') {
            if count == args.len() {
                return None;
            }
            let value = trim(value);
            if value.is_empty() {
                return None;
            }
            args[count] = value;
            count += 1;
        }
        let operation = operation(name, &args[..count])?;
        operations.try_reserve(1).ok()?;
        operations.push(operation);
        remaining = trim(&tail[close + 1..]);
    }
    (!operations.is_empty()).then_some(TransformList(operations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_literal, StyleProperty, StyleValue};
    #[test]
    fn native_function_literals_keep_owned_operations_units_and_order() {
        let input = " Translate(20PX,7px)scale(2,3) translateX(50%) translateY(1em) scaleX(-2) scaleY(125%) rotate(.25turn) matrix(1,0,0,1,4,5) ";
        let want = TransformList(vec![
            TransformOperation::Translate(LengthValue::px(20.0), LengthValue::px(7.0)),
            TransformOperation::Scale(2.0, 3.0),
            TransformOperation::Translate(LengthValue::percent(50.0), LengthValue::px(0.0)),
            TransformOperation::Translate(LengthValue::px(0.0), LengthValue::Em(1.0)),
            TransformOperation::Scale(-2.0, 1.0),
            TransformOperation::Scale(1.0, 1.25),
            TransformOperation::Rotate(90.0),
            TransformOperation::Matrix(Transform2D {
                e: 4.0,
                f: 5.0,
                ..Transform2D::IDENTITY
            }),
        ]);
        assert_eq!(
            parse_literal(StyleProperty::Transform, input).unwrap(),
            StyleValue::Transform(want)
        );
        for (input, want) in [
            ("scale(2)", TransformOperation::Scale(2.0, 2.0)),
            (
                "translate(-0)",
                TransformOperation::Translate(LengthValue::px(-0.0), LengthValue::px(0.0)),
            ),
            ("rotate(0)", TransformOperation::Rotate(0.0)),
            ("rotate(100grad)", TransformOperation::Rotate(90.0)),
        ] {
            assert_eq!(parse(input), Some(TransformList(vec![want])));
        }
        assert_eq!(parse("NoNe"), Some(TransformList::default()));
    }
    #[test]
    fn malformed_or_unrepresented_text_never_retains_a_partial_transform_list() {
        for input in [
            "",
            "none scale(2)",
            "scale()",
            "scale(2 3)",
            "scale(2,)",
            "scale(2,3,4)",
            "scale(NaN)",
            "scale(1e100)",
            "scale(2.)",
            "translate(1 px,2px)",
            "translate(1,0)",
            "translate(auto)",
            "translate(none)",
            "translate(calc(50% + 2px))",
            "rotate(10)",
            "rotate(1e38turn)",
            "matrix(1,0,0,1,0)",
            "scale(2); color:red",
            "scale(2) garbage",
            "scale(2) translate(1px",
            "scale(2)unknown(1)",
            "translate (1px)",
            "rotateZ(1deg)",
            "matrix3d(1)",
            "skew(1deg)",
            "scale(2)\u{00a0}scale(3)",
        ] {
            assert!(
                parse_literal(StyleProperty::Transform, input).is_err(),
                "{input}"
            );
        }
        assert!(parse(&"scale(1)".repeat(MAX_OPERATIONS)).is_some());
        assert!(parse(&"scale(1)".repeat(MAX_OPERATIONS + 1)).is_none());
    }
}
