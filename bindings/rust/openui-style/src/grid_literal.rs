//! Checked native grid track values for the property-value API.
//!
//! This reader accepts computed px/percentage tracks, intrinsic and flexible
//! tracks, minmax and fit-content. Named lines, repeats, subgrid and relative
//! lengths continue to use the native Rust structural types. This is a single
//! property reader, with bounded allocation and no stylesheet execution.

use crate::{GridTrackBreadth, GridTrackComponent, GridTrackList, GridTrackSize};
use openui_geometry::Length;

fn nonnegative(input: &str) -> Option<f32> {
    input
        .parse::<f32>()
        .ok()
        .filter(|number| number.is_finite() && *number >= 0.0)
}

fn length(input: &str) -> Option<Length> {
    let input = input.trim();
    if input == "0" {
        Some(Length::px(0.0))
    } else if let Some(number) = input.strip_suffix("px") {
        nonnegative(number).map(Length::px)
    } else {
        nonnegative(input.strip_suffix('%')?).map(Length::percent)
    }
}

fn breadth(input: &str) -> Option<GridTrackBreadth> {
    let input = input.trim();
    match input {
        "auto" => Some(GridTrackBreadth::Auto),
        "min-content" => Some(GridTrackBreadth::MinContent),
        "max-content" => Some(GridTrackBreadth::MaxContent),
        _ => {
            if let Some(number) = input.strip_suffix("fr") {
                nonnegative(number).map(GridTrackBreadth::Flex)
            } else {
                length(input).map(GridTrackBreadth::Length)
            }
        }
    }
}

fn track(input: &str) -> Option<GridTrackSize> {
    if let Some(inner) = input
        .strip_prefix("minmax(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let (min, max) = inner.split_once(',')?;
        let min = breadth(min)?;
        if matches!(min, GridTrackBreadth::Flex(_)) {
            return None;
        }
        Some(GridTrackSize::MinMax {
            min,
            max: breadth(max)?,
        })
    } else if let Some(inner) = input
        .strip_prefix("fit-content(")
        .and_then(|value| value.strip_suffix(')'))
    {
        Some(GridTrackSize::Breadth(GridTrackBreadth::FitContent(
            length(inner)?,
        )))
    } else {
        breadth(input).map(GridTrackSize::Breadth)
    }
}

pub(super) fn sizes(input: &str) -> Option<Vec<GridTrackSize>> {
    // Keep all native value readers bounded. There is no recursive parsing or
    // expansion; the layout implementation receives the typed track sizes.
    if input.len() > 65536 {
        return None;
    }
    let input = input.trim();
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (index, character) in input.char_indices() {
        match character {
            '(' if depth == 0 => depth = 1,
            '(' => return None,
            ')' if depth == 1 => depth = 0,
            ')' => return None,
            c if c.is_ascii_whitespace() && depth == 0 => {
                if start < index {
                    if result.len() == 4096 {
                        return None;
                    }
                    result.push(track(&input[start..index])?);
                }
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 || start == input.len() || result.len() == 4096 {
        return None;
    }
    result.push(track(&input[start..])?);
    Some(result)
}

pub(super) fn list(input: &str) -> Option<GridTrackList> {
    if input.trim() == "none" {
        Some(GridTrackList::None)
    } else {
        Some(GridTrackList::Tracks(
            sizes(input)?
                .into_iter()
                .map(GridTrackComponent::Track)
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_tracks_preserve_percent_intrinsic_flex_and_sizing_functions() {
        let parsed = sizes(
            "100% 12px 0 auto min-content max-content 2fr minmax(10%, 1fr) fit-content(40px)",
        )
        .unwrap();
        assert_eq!(parsed.len(), 9);
        assert!(
            matches!(parsed[0], GridTrackSize::Breadth(GridTrackBreadth::Length(v)) if v.is_percent() && v.value() == 100.0)
        );
        assert!(matches!(
            parsed[6],
            GridTrackSize::Breadth(GridTrackBreadth::Flex(2.0))
        ));
        assert!(
            matches!(parsed[7], GridTrackSize::MinMax { min: GridTrackBreadth::Length(v), max: GridTrackBreadth::Flex(1.0) } if v.is_percent() && v.value() == 10.0)
        );
        assert!(matches!(list("none"), Some(GridTrackList::None)));
    }

    #[test]
    fn invalid_tracks_fail_before_native_layout() {
        for input in [
            "",
            "none",
            "-1px",
            "-1%",
            "-1fr",
            "NaNfr",
            "inffr",
            "1",
            "1em",
            "minmax(1fr, 10px)",
            "minmax(1px,)",
            "minmax(1px,2px,3px)",
            "fit-content(auto)",
            "fit-content(-1px)",
            "fit-content(1px",
            "1px)",
            "minmax(minmax(0, 1px), 2px)",
            "repeat(2, 1px)",
            "[name] 1px",
            "subgrid",
        ] {
            assert!(sizes(input).is_none(), "{input}");
        }
        assert!(sizes(&"1px ".repeat(4097)).is_none());
        assert!(sizes(&" ".repeat(65537)).is_none());
    }
}
