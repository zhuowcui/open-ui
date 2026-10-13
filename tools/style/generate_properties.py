#!/usr/bin/env python3
"""Generate Rust, C, and reference tables from the canonical style schema."""

from __future__ import annotations

import argparse
import csv
import io
import os
import re
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "bindings/rust/openui-style/property-schema.csv"
INTERNAL_SCHEMA = ROOT / "bindings/rust/openui-style/internal-style-fields.csv"
CANONICAL_SCHEMA = ROOT / "bindings/rust/openui-style/canonical-style-fields.csv"
COMPUTED = ROOT / "bindings/rust/openui-style/src/computed.rs"
RUST_OUT = ROOT / "bindings/rust/openui-style/src/generated_properties.rs"
ACCESSORS_OUT = ROOT / "bindings/rust/openui-style/src/generated_computed_accessors.rs"
C_OUT = ROOT / "include/openui_style_properties.h"
DOC_OUT = ROOT / "docs/v02/generated/style-properties.md"
FRAMEWORK_OUT = ROOT / "bindings/rust/openui/src/generated_style_setters.rs"
AUTHORED_OUT = ROOT / "bindings/rust/openui-style/src/generated_authored_fields.rs"

# Frozen from Chromium 147.0.7727.24's css_properties.json5 after excluding
# descriptors, SVG-only properties, and non-stable runtime flags. Keeping the
# set here makes a missing public typography property a generator/CI failure.
CHROMIUM_147_TYPOGRAPHY = frozenset(
    """
    -webkit-font-smoothing direction font font-family font-feature-settings
    font-kerning font-language-override font-optical-sizing font-palette
    font-size font-size-adjust font-stretch font-style font-synthesis
    font-synthesis-small-caps font-synthesis-style font-synthesis-weight
    font-variant font-variant-alternates font-variant-caps
    font-variant-east-asian font-variant-emoji font-variant-ligatures
    font-variant-numeric font-variant-position font-variation-settings
    font-weight hyphenate-character hyphenate-limit-chars hyphens
    initial-letter letter-spacing line-break line-height overflow-wrap
    ruby-align ruby-position tab-size text-align text-align-last text-autospace
    text-box text-box-edge text-box-trim text-combine-upright text-decoration
    text-decoration-color text-decoration-line text-decoration-skip-ink
    text-decoration-style text-decoration-thickness text-emphasis
    text-emphasis-color text-emphasis-position text-emphasis-style text-indent
    text-justify text-orientation text-overflow text-rendering text-shadow
    text-size-adjust text-spacing-trim text-transform text-underline-offset
    text-underline-position text-wrap text-wrap-mode text-wrap-style
    unicode-bidi vertical-align white-space white-space-collapse word-break
    word-spacing word-wrap writing-mode
    """.split()
)


def computed_fields() -> dict[str, str]:
    text = COMPUTED.read_text()
    start = text.index("pub struct ComputedStyleFields {")
    end = text.index("\n}\n", start)
    fields = dict(
        re.findall(
            r"^\s+(?:pub(?:\(crate\))?\s+)?([a-z][a-z0-9_]*):\s*(.+),$",
            text[start:end],
            re.M,
        )
    )
    if not fields:
        raise SystemExit("could not parse ComputedStyle fields")
    return fields


def field_names(row: dict[str, str]) -> list[str]:
    value = row["computed_fields"].strip()
    return [] if value == "-" else value.split(";")


def canonical_group_fields(schema: list[dict[str, str]]) -> dict[str, list[str]]:
    groups = {row["rust_name"]: field_names(row) for row in schema}
    groups["BackgroundColor"] += ["background_current_color"]
    groups["Transform"] += ["native_transform", "transform_matrix"]
    corners = ["BorderTopLeftRadius", "BorderTopRightRadius", "BorderBottomRightRadius", "BorderBottomLeftRadius"]
    for index, name in enumerate(corners):
        groups[name] += [f"border_radius_percent[{index}]", f"border_radius_lengths[{index}]"]
    # This is an explicit native computed override, not a CSS longhand. Its
    # identity is separate; its write set still intersects all corner sources.
    groups["BorderRadiusPercent"] = [
        field for index in range(4)
        for field in (f"border_radius_percent[{index}]", f"border_radius_lengths[{index}]")
    ]
    return groups


def canonical_rows(schema: list[dict[str, str]]) -> list[dict[str, str]]:
    parsed = list(csv.DictReader(CANONICAL_SCHEMA.read_text().splitlines()))
    if not parsed or set(parsed[0]) != {"property", "canonical_groups", "mutation_fields"}:
        raise SystemExit("canonical style schema columns do not match the generator contract")
    if [row["property"] for row in parsed] != [row["rust_name"] for row in schema]:
        raise SystemExit("canonical ownership must cover every property exactly once in ID order")
    groups = canonical_group_fields(schema)
    fields = computed_fields()
    for row in parsed:
        names = row["canonical_groups"].split(";")
        if len(names) != len(set(names)) or any(name not in groups for name in names):
            raise SystemExit("invalid canonical groups for " + row["property"])
        if any(not groups[name] and name != "Cursor" for name in names):
            raise SystemExit("canonical owner has no projection: " + row["property"])
        expected = list(dict.fromkeys(field for name in names for field in groups[name]))
        writes = [] if row["mutation_fields"] == "-" else row["mutation_fields"].split(";")
        if set(writes) != set(expected) or len(writes) != len(set(writes)):
            raise SystemExit("canonical mutation fields disagree for " + row["property"])
        for field in writes:
            base = field.split("[")[0]
            if base not in fields:
                raise SystemExit("unknown canonical projection field: " + field)
            if "[" in field and (base not in {"border_radius_percent", "border_radius_lengths"} or field not in {f"{base}[{i}]" for i in range(4)}):
                raise SystemExit("invalid canonical array projection: " + field)
    return parsed


def internal_rows() -> list[dict[str, str]]:
    parsed = list(csv.DictReader(INTERNAL_SCHEMA.read_text().splitlines()))
    names = [row["field"] for row in parsed]
    if len(names) != len(set(names)):
        raise SystemExit("internal style field names must be unique")
    if any(not row["rationale"].strip() for row in parsed):
        raise SystemExit("every internal style field requires a rationale")
    fields = computed_fields()
    stale = sorted(set(names) - set(fields))
    if stale:
        raise SystemExit("stale internal style fields: " + ", ".join(stale))
    return parsed


def rows() -> list[dict[str, str]]:
    parsed = list(csv.DictReader(SCHEMA.read_text().splitlines()))
    required = {
        "id", "rust_name", "css_name", "rust_type", "value_kind", "initial",
        "inherited", "invalidation", "interpolation", "computed_fields",
    }
    if not parsed or set(parsed[0]) != required:
        raise SystemExit("style schema columns do not match the generator contract")
    ids = [int(row["id"]) for row in parsed]
    names = [row["css_name"] for row in parsed]
    if ids != sorted(ids) or len(ids) != len(set(ids)) or len(names) != len(set(names)):
        raise SystemExit("style schema IDs and names must be unique and ordered")
    if ids != list(range(1, len(ids) + 1)):
        raise SystemExit("style schema IDs must be contiguous")
    mapped = [field for row in parsed for field in field_names(row)]
    duplicates = sorted(field for field, count in Counter(mapped).items() if count > 1)
    if duplicates:
        raise SystemExit("duplicate computed-field mappings: " + ", ".join(duplicates))
    fields = computed_fields()
    unknown = sorted(set(mapped) - set(fields))
    if unknown:
        raise SystemExit("unknown computed-field mappings: " + ", ".join(unknown))
    internal = {row["field"] for row in internal_rows()}
    overlap = sorted(set(mapped) & internal)
    if overlap:
        raise SystemExit("fields cannot be both authored and internal: " + ", ".join(overlap))
    missing_typography = sorted(CHROMIUM_147_TYPOGRAPHY.difference(names))
    if missing_typography:
        raise SystemExit(
            "style schema is missing Chromium 147 typography properties: "
            + ", ".join(missing_typography)
        )
    return parsed


def title(value: str) -> str:
    return "".join(part.capitalize() for part in value.split("-"))


def screaming(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9]", "_", value).upper()


def rust_method(value: str) -> str:
    return value.strip("-").replace("-", "_")


def rust_output(schema: list[dict[str, str]]) -> bytes:
    variants = "\n".join(f"    {row['rust_name']} = {row['id']}," for row in schema)
    metadata = "\n".join(
        "\n".join(
            (
                "    PropertyMetadata {",
                f"        property: StyleProperty::{row['rust_name']},",
                f"        css_name: \"{row['css_name']}\",",
                f"        rust_type: \"{row['rust_type']}\",",
                f"        value_kind: ValueKind::{title(row['value_kind'])},",
                f"        initial: \"{row['initial']}\",",
                f"        inherited: {row['inherited']},",
                f"        invalidation: InvalidationClass::{title(row['invalidation'])},",
                f"        interpolation: InterpolationKind::{title(row['interpolation'])},",
                "    },",
            )
        )
        for row in schema
    )
    lookups = "\n".join(
        f'            "{row["css_name"]}" => Some(Self::{row["rust_name"]}),' for row in schema
    )
    # Renderer fixtures and engine-internal typed setters use the exact
    # computed-field type, while the stable public builders retain their
    # higher-level author value types (for example LengthValue). Every row in
    # this list owns exactly one field by schema construction.
    fields = computed_fields()
    renderer_rows = [row for row in schema if len(field_names(row)) == 1]
    renderer_variants = "    ScrollSnapAlignPair([ScrollSnapAlign; 2]),\n" + "\n".join(
        f"    {row['rust_name']}({fields[field_names(row)[0]]})," for row in renderer_rows
    )
    renderer_apply = "            (StyleProperty::ScrollSnapAlign, Self::ScrollSnapAlignPair(value)) => { style.fields.scroll_snap_align = ScrollSnapAlign::None; style.fields.scroll_snap_align_block = Some(value[0]); style.fields.scroll_snap_align_inline = Some(value[1]); true },\n" + "\n".join(
        f"            (StyleProperty::{row['rust_name']}, Self::{row['rust_name']}(value)) => {{ style.fields.{field_names(row)[0]} = value.clone();"
        + (" style.fields.scroll_snap_align_block = None; style.fields.scroll_snap_align_inline = None;" if row['rust_name'] == 'ScrollSnapAlign' else "")
        + (f" style.fields.border_radius_lengths[{['BorderTopLeftRadius', 'BorderTopRightRadius', 'BorderBottomRightRadius', 'BorderBottomLeftRadius'].index(row['rust_name'])}] = None;" if row['rust_name'] in {'BorderTopLeftRadius', 'BorderTopRightRadius', 'BorderBottomRightRadius', 'BorderBottomLeftRadius'} else "")
        + (" style.fields.border_radius_lengths = [None; 4];" if row['rust_name'] == 'BorderRadiusPercent' else "")
        + " true },"
        for row in renderer_rows
    )
    renderer_read = "\n".join(
        "            StyleProperty::ScrollSnapAlign => Some(if style.fields.scroll_snap_align_block.is_some() || style.fields.scroll_snap_align_inline.is_some() { Self::ScrollSnapAlignPair([style.fields.scroll_snap_align_block.unwrap_or(style.fields.scroll_snap_align), style.fields.scroll_snap_align_inline.unwrap_or(style.fields.scroll_snap_align)]) } else { Self::ScrollSnapAlign(style.fields.scroll_snap_align) }),"
        if row['rust_name'] == 'ScrollSnapAlign' else
        f"            StyleProperty::{row['rust_name']} => Some(Self::{row['rust_name']}(style.fields.{field_names(row)[0]}.clone())),"
        for row in renderer_rows
    )
    # Animation endpoints and current computed values share one semantic
    # conversion. Derive native wrappers from the public constructor surface;
    # the renderer representation remains property-bound and lossless.
    property_source = (ROOT / "bindings/rust/openui-style/src/property.rs").read_text()
    native_variants = dict(re.findall(
        r"impl_style_value!\(([^,]+),\s*([A-Za-z0-9_]+)\);", property_source
    ))
    typography_variants = dict(re.findall(
        r"impl_typography_value!\(([^,]+),\s*([A-Za-z0-9_]+)\);", property_source
    ))
    animation_cases = []
    for row in renderer_rows:
        name = row["rust_name"]
        field_type = fields[field_names(row)[0]].removeprefix("crate::")
        kind = row["value_kind"]
        expression = None
        if kind == "number" and field_type == "f32":
            expression = "Some(crate::CompositedStyleValue::Number { value: *value as f64, kind: crate::AnimationNumberKind::Number })"
        elif kind == "integer" and field_type in {"i32", "u32"}:
            expression = "Some(crate::CompositedStyleValue::Number { value: *value as f64, kind: crate::AnimationNumberKind::Integer })"
        elif kind == "integer" and field_type in {"Option<i32>", "Option<u32>"}:
            expression = "value.as_ref().map(|value| crate::CompositedStyleValue::Number { value: *value as f64, kind: crate::AnimationNumberKind::Integer })"
        elif kind == "length" and field_type == "f32":
            expression = "Some(crate::CompositedStyleValue::from_style_value(StyleValue::Length(LengthValue::px(*value))))"
        elif kind == "length" and field_type == "Length":
            expression = "Some(crate::CompositedStyleValue::from_style_value(StyleValue::Length(LengthValue::Computed(*value))))"
        elif kind == "length" and field_type == "LengthValue":
            expression = "Some(crate::CompositedStyleValue::from_style_value(StyleValue::Length(*value)))"
        elif kind == "length" and field_type == "Option<Length>":
            expression = "value.as_ref().map(|value| crate::CompositedStyleValue::from_style_value(StyleValue::Length(LengthValue::Computed(*value))))"
        elif kind == "color" and field_type == "StyleColor":
            # Keep keywords as owned working fractions; the property-bound
            # sampler canonicalizes the schema's Color/compound representation.
            expression = "Some(crate::CompositedStyleValue::from_style_value(StyleValue::Typography(TypographyValue::StyleColor(*value))))"
        elif kind == "color" and field_type == "Option<Color>":
            expression = "value.as_ref().map(|value| crate::CompositedStyleValue::from_style_value(StyleValue::Color(*value)))"
        elif field_type == "Transform2D":
            expression = "Some(crate::CompositedStyleValue::from_style_value(StyleValue::Transform(if *value == Transform2D::IDENTITY { TransformList::default() } else { TransformList(vec![TransformOperation::Matrix(*value)]) })))"
        elif field_type in native_variants:
            expression = f"Some(crate::CompositedStyleValue::from_style_value(StyleValue::{native_variants[field_type]}(value.clone())))"
        elif field_type in typography_variants:
            expression = f"Some(crate::CompositedStyleValue::from_style_value(StyleValue::Typography(TypographyValue::{typography_variants[field_type]}(value.clone()))))"
        if expression is not None:
            animation_cases.append(f"            Self::{name}(value) => {expression},")
    animation_conversion = "\n".join(animation_cases)
    # Copy computed values, without resolving inherited lengths again.
    inherited_fields = sorted({field for row in schema if row["inherited"] == "true" for field in field_names(row)})
    inherited_copy = "\n".join(f"        self.fields.{field} = parent.fields.{field}.clone();" for field in inherited_fields)
    # Shorthands and normalization write more than their canonical inventory
    # field. Use those write sets when deciding whether a repeated declaration
    # is still effective after later declarations.
    mutation_cases = []
    for row in canonical_rows(schema):
        names = [] if row["mutation_fields"] == "-" else row["mutation_fields"].split(";")
        values = ", ".join(f'"{name}"' for name in names)
        mutation_cases.append(f"            Self::{row['property']} => &[{values}],")
    mutation_match = "\n".join(mutation_cases)

    # Author values keep relative lengths until the engine resolves them. This
    # bridge also serves C's scalar transport; computed renderer values remain
    # a separate, lossless path through the same fields.
    primitive_shapes = {
        ("length", "Length"): ("Length", "resolve_length(*value)", "length(input).map(StyleValue::Length)"),
        ("length", "Option<Length>"): (
            "Length", "{ let value = resolve_length(*value); (!value.is_auto()).then_some(value) }",
            "length(input).map(StyleValue::Length)",
        ),
        ("number", "f32"): ("Number", "*value", "input.parse::<f32>().ok().filter(|value| value.is_finite()).map(StyleValue::Number)"),
        ("integer", "i32"): ("Integer", "*value", "input.parse::<i32>().ok().map(StyleValue::Integer)"),
        ("integer", "u32"): ("Integer", "u32::try_from(*value).ok()?", None),
        ("integer", "Option<u32>"): ("Integer", "Some(u32::try_from(*value).ok().filter(|value| *value > 0)?)", None),
        ("color", "StyleColor"): ("Color", "StyleColor::Resolved(*value)", None),
        ("color", "Option<Color>"): ("Color", "Some(*value)", None),
        ("overflow", "Overflow"): ("Overflow", "*value", "overflow_literal(input).map(StyleValue::Overflow)"),
        ("item-alignment", "ItemAlignment"): ("ItemAlignment", "*value", "item_alignment_literal(input).map(StyleValue::ItemAlignment)"),
        ("content-alignment", "ContentAlignment"): ("ContentAlignment", "*value", "content_alignment_literal(input).map(StyleValue::ContentAlignment)"),
    }
    # These native value constructors are property-bound, owned renderer
    # values. Keep every variant of each supported enum available through the
    # same path that C uses, rather than interpreting an untyped enum number.
    enum_keywords = {
        "BorderCollapse": {"separate": "Separate", "collapse": "Collapse"},
        "BorderStyle": {
            "none": "None", "hidden": "Hidden", "dotted": "Dotted",
            "dashed": "Dashed", "solid": "Solid", "double": "Double",
            "groove": "Groove", "ridge": "Ridge", "inset": "Inset",
            "outset": "Outset",
        },
        "BoxDecorationBreak": {"slice": "Slice", "clone": "Clone"},
        "BreakValue": {
            "auto": "Auto", "avoid": "Avoid", "avoid-page": "AvoidPage",
            "avoid-column": "AvoidColumn", "page": "Page", "column": "Column",
            "left": "Left", "right": "Right", "always": "Always",
        },
        "BreakInside": {
            "auto": "Auto", "avoid": "Avoid", "avoid-page": "AvoidPage",
            "avoid-column": "AvoidColumn",
        },
        "ColumnFill": {
            "balance": "Balance", "balance-all": "BalanceAll", "auto": "Auto",
        },
        "ColumnSpan": {"none": "None", "all": "All"},
        "ColumnWrap": {"auto": "Auto", "wrap": "Wrap", "nowrap": "NoWrap"},
        "ScrollSnapStrictness": {"mandatory": "Mandatory", "proximity": "Proximity"},
        "ScrollSnapStop": {"normal": "Normal", "always": "Always"},
        "ScrollSnapAlign": {"none": "None", "start": "Start", "center": "Center", "end": "End"},
        "ScrollSnapAxis": {"none": "None", "x": "X", "y": "Y", "both": "Both", "inline": "Inline", "block": "Block"},
    }
    enum_source = (ROOT / "bindings/rust/openui-style/src/enums.rs").read_text() + (ROOT / "bindings/rust/openui-style/src/layout_systems.rs").read_text()
    for enum_type, keywords in enum_keywords.items():
        declaration = re.search(
            rf"pub enum {re.escape(enum_type)} \{{(.*?)\n\}}", enum_source, re.S
        )
        if declaration is None:
            raise SystemExit(f"missing native enum declaration: {enum_type}")
        declared_variants = set(re.findall(
            r"^\s*(\w+)\s*(?:=\s*\d+)?\s*,?\s*$", declaration[1], re.M
        ))
        if declared_variants != set(keywords.values()):
            raise SystemExit(f"native keyword constructors do not cover {enum_type}")
    primitive_apply_cases = []
    primitive_parse_cases = []
    for row in renderer_rows:
        field_type = fields[field_names(row)[0]]
        if int(row["id"]) <= 125:
            continue
        shape = primitive_shapes.get((row["value_kind"], fields[field_names(row)[0]]))
        if shape is None:
            enum_type = fields[field_names(row)[0]]
            if enum_type == "Option<ScrollSnapAlign>":
                name = row["rust_name"]
                cases = "\n".join(f'            "{keyword}" => Some(Some(ScrollSnapAlign::{variant})),' for keyword, variant in enum_keywords["ScrollSnapAlign"].items())
                primitive_parse_cases.append(
                    f"        StyleProperty::{name} => (match input.trim() {{\n{cases}\n            \"auto\" => Some(None),\n            _ => None,\n        }}).map(|value| StyleValue::Renderer(RendererStyleValue::{name}(value))),")
                continue
            if row["rust_name"] == "ScrollSnapAlign":
                primitive_parse_cases.append("""        StyleProperty::ScrollSnapAlign => {
            let align = |value| match value {
                "none" => Some(ScrollSnapAlign::None), "start" => Some(ScrollSnapAlign::Start),
                "center" => Some(ScrollSnapAlign::Center), "end" => Some(ScrollSnapAlign::End), _ => None,
            };
            match input.split_whitespace().collect::<Vec<_>>().as_slice() {
                [value] => align(value).map(|value| StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(value))),
                [block, inline] => Some(StyleValue::Renderer(RendererStyleValue::ScrollSnapAlignPair([align(block)?, align(inline)?]))),
                _ => None,
            }
        },""")
                continue
            if enum_type in enum_keywords:
                name = row["rust_name"]
                cases = "\n".join(
                    f'            "{keyword}" => Some({enum_type}::{variant}),'
                    for keyword, variant in enum_keywords[enum_type].items()
                )
                primitive_parse_cases.append(
                    f"        StyleProperty::{name} => (match input.trim() {{\n"
                    f"{cases}\n            _ => None,\n"
                    f"        }}).map(|value| StyleValue::Renderer(RendererStyleValue::{name}(value))),"
                )
                continue
            if row["value_kind"] in {key[0] for key in primitive_shapes}:
                raise SystemExit(f"missing author-value bridge for {row['css_name']}: {row['rust_type']}")
            continue
        variant, value, parse = shape
        name = row["rust_name"]
        primitive_apply_cases.append(
            f"            (StyleProperty::{name}, StyleValue::{variant}(value)) => Some(Self::{name}({value})),"
        )
        if row["value_kind"] == "color" and field_type == "StyleColor":
            primitive_apply_cases.append(
                f"            (StyleProperty::{name}, StyleValue::Typography(TypographyValue::StyleColor(value))) => Some(Self::{name}(*value)),"
            )
        # ColumnCount's auto/positive literal parser is handwritten. Its
        # native integer bridge still uses the checked generated apply path.
        if name == "ColumnCount":
            continue
        if parse is None:
            if row["rust_type"] == "u32":
                parse = f"input.parse::<u32>().ok().map(|value| StyleValue::Renderer(RendererStyleValue::{name}(value)))"
            elif row["rust_type"] == "StyleColor":
                parse = f'if input.eq_ignore_ascii_case("currentcolor") {{ Some(StyleValue::Renderer(RendererStyleValue::{name}(StyleColor::CurrentColor))) }} else {{ color(input).map(StyleValue::Color) }}'
            elif row["rust_type"] == "Option<Color>":
                parse = f'if input == "auto" {{ Some(StyleValue::Renderer(RendererStyleValue::{name}(None))) }} else {{ color(input).map(StyleValue::Color) }}'
        primitive_parse_cases.append(f"        StyleProperty::{name} => {parse},")
    primitive_apply = "\n".join(primitive_apply_cases)
    primitive_parse = "\n".join(primitive_parse_cases)
    internal = internal_rows()
    internal_variants = "\n".join(
        f"    {title(row['field'].replace('_', '-'))}({fields[row['field']]}),"
        for row in internal
    )
    internal_apply = "\n".join(
        f"            Self::{title(row['field'].replace('_', '-'))}(value) => style.{row['field']} = value,"
        for row in internal
    )
    builders = "\n".join(
        "\n".join(
            (
                f"    pub fn {rust_method(row['css_name'])}(self, value: {row['rust_type']}) -> Self {{",
                (
                    f"        self.with(StyleProperty::{row['rust_name']}, value)"
                    if int(row["id"]) <= 125 or row["rust_type"] == "LengthValue"
                    else f"        self.with_renderer(StyleProperty::{row['rust_name']}, RendererStyleValue::{row['rust_name']}(value))"
                ),
                "    }",
            )
        )
        for row in schema
    )
    compile_pass = "\n".join(
        f"        let _ = Style::default().{rust_method(row['css_name'])}(unimplemented!());"
        for row in schema
    )
    compile_fail = "\n".join(
        f"///     let _ = Style::default().{rust_method(row['css_name'])}(());"
        for row in schema
    )
    text = f"""// @generated by tools/style/generate_properties.py; do not edit.

/// Stable property identifiers shared by Rust and the v0.2 C ABI.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleProperty {{
{variants}
}}

/// Schema-generated values for renderer longhands appended after the stable
/// v0.2 property surface. Variants are property-bound; the paired snap value
/// writes both logical alignments through the existing align property.
#[derive(Debug, Clone)]
pub enum RendererStyleValue {{
{renderer_variants}
}}

impl PartialEq for RendererStyleValue {{
    fn eq(&self, other: &Self) -> bool {{
        // Several renderer value types intentionally avoid semantic equality
        // (notably decoded images and gradients). Their deterministic Debug
        // representation is sufficient for the authored-value no-op cache.
        std::mem::discriminant(self) == std::mem::discriminant(other)
            && format!("{{self:?}}") == format!("{{other:?}}")
    }}
}}

impl RendererStyleValue {{
    pub fn property(&self) -> StyleProperty {{
        match self {{
            Self::ScrollSnapAlignPair(..) => StyleProperty::ScrollSnapAlign,
{chr(10).join(f"            Self::{row['rust_name']}(..) => StyleProperty::{row['rust_name']}," for row in renderer_rows)}
        }}
    }}

    pub(crate) fn apply(&self, style: &mut ComputedStyle, property: StyleProperty) -> bool {{
        if matches!((property, self),
            (StyleProperty::WhiteSpace, Self::WhiteSpace(_)) |
            (StyleProperty::TextWrap, Self::TextWrap(_))) {{
            return apply_to_computed(style, property, &StyleValue::Renderer(self.clone()), (0.0, 0.0)).is_ok();
        }}
        match (property, self) {{
{renderer_apply}
            _ => false,
        }}
    }}

    /// Convert supported computed payloads through the shared native animation
    /// value path. Optional/auto and unresolved keyword sources remain owned.
    pub(crate) fn animation_value(&self) -> Option<crate::CompositedStyleValue> {{
        match self {{
{animation_conversion}
            _ => None,
        }}
    }}

    pub(crate) fn from_author_value(
        property: StyleProperty,
        value: &StyleValue,
        resolve_length: impl Fn(LengthValue) -> Length,
    ) -> Option<Self> {{
        match (property, value) {{
{primitive_apply}
            _ => None,
        }}
    }}

    pub(crate) fn from_computed(
        style: &ComputedStyle,
        property: StyleProperty,
    ) -> Option<Self> {{
        match property {{
{renderer_read}
            _ => None,
        }}
    }}
}}

fn parse_renderer_author_literal(property: StyleProperty, input: &str) -> Option<StyleValue> {{
    match property {{
{primitive_parse}
        _ => None,
    }}
}}

/// Engine-owned renderer state which is deliberately not an author property.
/// Each entry has an explicit rationale in `internal-style-fields.csv`.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum RendererInternalStyleValue {{
{internal_variants}
}}

impl RendererInternalStyleValue {{
    #[doc(hidden)]
    pub fn apply_to(self, computed: &mut ComputedStyle) {{
        computed.update_derived(|style| match self {{
{internal_apply}
        }});
    }}
}}

impl ComputedStyle {{
    /// Copy modeled inherited computed fields from a parent snapshot.
    /// Authored values are reapplied by the retained engine afterwards.
    #[doc(hidden)]
    pub fn inherit_properties_from(&mut self, parent: &Self) {{
{inherited_copy}
        self.resolve_current_color_dependencies();
    }}
}}

impl StyleProperty {{
    #[doc(hidden)]
    pub fn affects_same_fields_as(self, other: Self) -> bool {{
        self == other || self.mutation_fields().iter().any(|field| other.mutation_fields().contains(field))
    }}

    fn mutation_fields(self) -> &'static [&'static str] {{
        match self {{
{mutation_match}
        }}
    }}

    pub fn from_u16(value: u16) -> Option<Self> {{
        PROPERTY_METADATA
            .get(value.checked_sub(1)? as usize)
            .map(|metadata| metadata.property)
    }}

    /// Look up a schema property. This is intended for tooling and macro expansion;
    /// application mutation APIs accept `StyleProperty`, never a runtime name.
    pub fn from_css_name(name: &str) -> Option<Self> {{
        match name {{
{lookups}
            _ => None,
        }}
    }}

    pub fn metadata(self) -> &'static PropertyMetadata {{
        &PROPERTY_METADATA[self as usize - 1]
    }}
}}

pub const PROPERTY_METADATA: &[PropertyMetadata] = &[
{metadata}
];

/// Compile contract for every schema-generated Rust property builder.
///
/// ```compile_fail
/// use openui_style::Style;
/// fn every_property_rejects_an_untyped_unit_value() {{
{compile_fail}
/// }}
/// ```
#[doc(hidden)]
pub struct GeneratedPropertyCompileContract;

#[cfg(test)]
mod generated_property_compile_pass {{
    use super::Style;

    #[allow(dead_code, unreachable_code)]
    fn every_property_accepts_its_generated_type() {{
{compile_pass}
    }}
}}

#[rustfmt::skip]
impl Style {{
{builders}
}}
"""
    return text.encode()


def c_output(schema: list[dict[str, str]]) -> bytes:
    # Preserve the published spelling; new vendor properties avoid a reserved
    # double underscore in C++ identifiers.
    legacy_c_names = {"-webkit-font-smoothing": "_WEBKIT_FONT_SMOOTHING"}
    enum_rows = "\n".join(
        f"  OUI_STYLE_PROPERTY_{legacy_c_names.get(row['css_name'], screaming(row['css_name'].lstrip('-')))} = {row['id']}," for row in schema
    )
    text = f"""/* @generated by tools/style/generate_properties.py; do not edit. */
#ifndef OPENUI_STYLE_PROPERTIES_H_
#define OPENUI_STYLE_PROPERTIES_H_

#include <stdint.h>

#define OUI_STYLE_SCHEMA_VERSION 2u

typedef enum OuiStyleProperty {{
{enum_rows}
}} OuiStyleProperty;

typedef enum OuiStyleValueTag {{
  OUI_STYLE_VALUE_LENGTH = 1,
  OUI_STYLE_VALUE_NUMBER = 2,
  OUI_STYLE_VALUE_INTEGER = 3,
  OUI_STYLE_VALUE_COLOR = 4,
  OUI_STYLE_VALUE_ENUM = 5,
  OUI_STYLE_VALUE_COMPOUND = 6
}} OuiStyleValueTag;

#endif  /* OPENUI_STYLE_PROPERTIES_H_ */
"""
    return text.encode()


def framework_output(schema: list[dict[str, str]]) -> bytes:
    setters = "\n".join(
        "\n".join(
            (
                f"    pub fn set_{rust_method(row['css_name'])}(&self, value: {row['rust_type']}) -> Result<(), Error> {{",
                (
                    f"        self.set_property(StyleProperty::{row['rust_name']}, value.into())"
                    if int(row["id"]) <= 125 or row["rust_type"] == "LengthValue"
                    else f"        self.set_property(StyleProperty::{row['rust_name']}, StyleValue::Renderer(RendererStyleValue::{row['rust_name']}(value)))"
                ),
                "    }",
            )
        )
        for row in schema
    )
    text = f"""// @generated by tools/style/generate_properties.py; do not edit.

use crate::style::Error;
use crate::typed_style::*;
use crate::Element;

#[rustfmt::skip]
impl Element {{
{setters}
}}
"""
    return text.encode()


def docs_output(schema: list[dict[str, str]]) -> bytes:
    lines = [
        "<!-- @generated by tools/style/generate_properties.py; do not edit. -->",
        "# Open UI v0.2 style properties",
        "",
        "| ID | Property | Computed field(s) | Rust type | Initial | Inherited | Invalidation | Interpolation |",
        "|---:|---|---|---|---|:---:|---|---|",
    ]
    lines.extend(
        f"| {row['id']} | `{row['css_name']}` | `{row['computed_fields']}` | `{row['rust_type']}` | `{row['initial']}` | "
        f"{row['inherited']} | {row['invalidation']} | {row['interpolation']} |"
        for row in schema
    )
    return ("\n".join(lines) + "\n").encode()


def accessors_output(schema: list[dict[str, str]]) -> bytes:
    fields = computed_fields()
    authored = {field for row in schema for field in field_names(row)}
    internal = {row["field"] for row in internal_rows()}
    accessors = []
    for field in sorted(authored | internal):
        accessors.extend(
            (
                "    #[inline]",
                f"    pub fn {field}(&self) -> &{fields[field]} {{",
                f"        &self.fields.{field}",
                "    }",
            )
        )
    return (
        "// @generated by tools/style/generate_properties.py; do not edit.\n\n"
        "impl ComputedStyle {\n"
        + "\n".join(accessors)
        + "\n}\n"
    ).encode()


def authored_output(schema: list[dict[str, str]]) -> bytes:
    ownership = canonical_rows(schema)
    group_fields = canonical_group_fields(schema)
    used = {name for row in ownership for name in row["canonical_groups"].split(";")}
    names = [row["rust_name"] for row in schema if row["rust_name"] in used]
    if len(names) != len(used):
        raise SystemExit("canonical owner must have one existing property identity")
    variants = "\n".join(f"    {name} = {index}," for index, name in enumerate(names))
    values = "\n".join(f"    CanonicalStyleField::{name}," for name in names)
    copy_cases = []
    write_cases = []
    words = (len(names) + 63) // 64
    for name in names:
        fields = group_fields[name]
        assignments = " ".join(f"target.fields.{field} = source.fields.{field}.clone();" for field in fields)
        copy_cases.append(f"            Self::{name} => {{ {assignments} }},")
        dependent = [0] * words
        for index, other in enumerate(names):
            if set(fields).intersection(group_fields[other]):
                dependent[index // 64] |= 1 << (index % 64)
        write_cases.append(f"            Self::{name} => CanonicalStyleFields([{', '.join(str(word) for word in dependent)}]),")
    property_cases = []
    for row in ownership:
        groups = ", ".join(f"CanonicalStyleField::{name}" for name in row["canonical_groups"].split(";"))
        property_cases.append(f"            Self::{row['property']} => &[{groups}],")
    return ("// @generated by tools/style/generate_properties.py; do not edit.\n"
            "// Canonical ownership is separate from derived/write dependencies.\n"
            f"\nconst CANONICAL_STYLE_FIELD_WORDS: usize = {words};\n\n"
            "#[doc(hidden)]\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(u16)]\n"
            "pub enum CanonicalStyleField {\n" + variants + "\n}\n\n"
            "const CANONICAL_STYLE_FIELDS: &[CanonicalStyleField] = &[\n" + values + "\n];\n\n"
            "#[rustfmt::skip]\nimpl CanonicalStyleField {\n"
            "    #[allow(clippy::clone_on_copy)]\n"
            "    fn copy_from(self, target: &mut ComputedStyle, source: &ComputedStyle) {\n"
            "        match self {\n" + "\n".join(copy_cases) + "\n        }\n    }\n"
            "    fn dependent_mutation_groups(self) -> CanonicalStyleFields {\n"
            "        match self {\n" + "\n".join(write_cases) + "\n        }\n    }\n}\n"
            "\n#[rustfmt::skip]\nimpl StyleProperty {\n"
            "    #[doc(hidden)]\n"
            "    pub fn canonical_field_groups(self) -> &'static [CanonicalStyleField] {\n"
            "        match self {\n" + "\n".join(property_cases) + "\n        }\n    }\n}\n").encode()


def outputs() -> dict[Path, bytes]:
    schema = rows()
    return {
        RUST_OUT: rust_output(schema),
        AUTHORED_OUT: authored_output(schema),
        ACCESSORS_OUT: accessors_output(schema),
        FRAMEWORK_OUT: framework_output(schema),
        C_OUT: c_output(schema),
        DOC_OUT: docs_output(schema),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    if args.check:
        drift = [str(path.relative_to(ROOT)) for path, data in generated.items() if not path.is_file() or path.read_bytes() != data]
        if drift:
            raise SystemExit("generated style schema drift: " + ", ".join(drift))
        print(f"style schema: properties={len(rows())} drift=0")
        return
    for path, data in generated.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        mode = path.stat().st_mode if path.exists() else 0o644
        temporary = path.with_name(path.name + ".generated-new")
        temporary.write_bytes(data)
        os.chmod(temporary, mode)
        temporary.replace(path)
    print(f"style schema: wrote={len(generated)} properties={len(rows())}")


if __name__ == "__main__":
    main()
