#!/usr/bin/env python3
"""Generate Rust, C, and reference tables from the canonical style schema."""

from __future__ import annotations

import argparse
import csv
import io
import re
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "bindings/rust/openui-style/property-schema.csv"
INTERNAL_SCHEMA = ROOT / "bindings/rust/openui-style/internal-style-fields.csv"
COMPUTED = ROOT / "bindings/rust/openui-style/src/computed.rs"
RUST_OUT = ROOT / "bindings/rust/openui-style/src/generated_properties.rs"
ACCESSORS_OUT = ROOT / "bindings/rust/openui-style/src/generated_computed_accessors.rs"
C_OUT = ROOT / "include/openui_style_properties.h"
DOC_OUT = ROOT / "docs/v02/generated/style-properties.md"
FRAMEWORK_OUT = ROOT / "bindings/rust/openui/src/generated_style_setters.rs"

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
    renderer_variants = "\n".join(
        f"    {row['rust_name']}({fields[field_names(row)[0]]})," for row in renderer_rows
    )
    renderer_apply = "\n".join(
        f"            (StyleProperty::{row['rust_name']}, Self::{row['rust_name']}(value)) => {{ style.fields.{field_names(row)[0]} = value.clone(); true }},"
        for row in renderer_rows
    )
    renderer_read = "\n".join(
        f"            StyleProperty::{row['rust_name']} => Some(Self::{row['rust_name']}(style.fields.{field_names(row)[0]}.clone())),"
        for row in renderer_rows
    )
    # Copy computed values, without resolving inherited lengths again.
    inherited_fields = sorted({field for row in schema if row["inherited"] == "true" for field in field_names(row)})
    inherited_copy = "\n".join(f"        self.fields.{field} = parent.fields.{field}.clone();" for field in inherited_fields)
    # Shorthands and normalization write more than their canonical inventory
    # field. Use those write sets when deciding whether a repeated declaration
    # is still effective after later declarations.
    mutation_overrides = {
        "Font": "font_style font_variant_caps font_weight font_stretch font_size line_height font_family font_variant_ligatures font_variant_numeric font_variant_east_asian font_variant_alternates font_variant_position font_variant_emoji font_optical_sizing font_size_adjust font_kerning font_feature_settings font_variation_settings font_language_override",
        "FontVariant": "font_variant_ligatures font_variant_caps font_variant_alternates font_variant_numeric font_variant_east_asian font_variant_position font_variant_emoji",
        "FontSynthesis": "font_synthesis_weight font_synthesis_style font_synthesis_small_caps font_synthesis_position",
        "WordWrap": "overflow_wrap",
        "TextEmphasisStyle": "text_emphasis_mark text_emphasis_fill",
        "TextEmphasis": "text_emphasis_mark text_emphasis_fill text_emphasis_color",
        "TextDecoration": "text_decoration_line text_decoration_style text_decoration_color text_decoration_thickness",
        "TextBox": "text_box_trim text_box_edge",
        "WhiteSpace": "white_space_collapse text_wrap_mode white_space text_wrap",
        "TextWrap": "text_wrap_mode text_wrap_style text_wrap white_space",
        "WhiteSpaceCollapse": "white_space_collapse white_space",
        "TextWrapMode": "text_wrap_mode white_space text_wrap",
        "TextWrapStyle": "text_wrap_style text_wrap",
    }
    assert set(mutation_overrides) <= {row["rust_name"] for row in schema}
    mutation_cases = []
    for row in schema:
        names = mutation_overrides.get(row["rust_name"], " ".join(field_names(row))).split()
        assert set(names) <= set(fields), row["rust_name"]
        values = ", ".join(f'"{name}"' for name in names)
        mutation_cases.append(f"            Self::{row['rust_name']} => &[{values}],")
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
    }
    enum_source = (ROOT / "bindings/rust/openui-style/src/enums.rs").read_text()
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
        if int(row["id"]) <= 125 or row["rust_name"] == "ColumnCount":
            continue
        shape = primitive_shapes.get((row["value_kind"], fields[field_names(row)[0]]))
        if shape is None:
            enum_type = fields[field_names(row)[0]]
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
                    if int(row["id"]) <= 125
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
/// v0.2 property surface. Each variant is tied to exactly one computed field.
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
{chr(10).join(f"            Self::{row['rust_name']}(..) => StyleProperty::{row['rust_name']}," for row in renderer_rows)}
        }}
    }}

    pub(crate) fn apply(&self, style: &mut ComputedStyle, property: StyleProperty) -> bool {{
        match (property, self) {{
{renderer_apply}
            _ => false,
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
    enum_rows = "\n".join(
        f"  OUI_STYLE_PROPERTY_{screaming(row['css_name'])} = {row['id']}," for row in schema
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
                    if int(row["id"]) <= 125
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


def outputs() -> dict[Path, bytes]:
    schema = rows()
    return {
        RUST_OUT: rust_output(schema),
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
        path.write_bytes(data)
    print(f"style schema: wrote={len(generated)} properties={len(rows())}")


if __name__ == "__main__":
    main()
