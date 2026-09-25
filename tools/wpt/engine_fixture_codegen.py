#!/usr/bin/env python3
"""Lower legacy porter Rust into immutable, Engine-backed fixtures.

The porter still computes already-resolved values, but emitted Rust never
receives a mutable DOM node or ComputedStyle. This pass is deterministic and
also provides the renderer-contract source audit used by CI.
"""

from __future__ import annotations

import argparse
import csv
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "bindings/rust/openui-style/property-schema.csv"
INTERNAL = ROOT / "bindings/rust/openui-style/internal-style-fields.csv"
PIXEL_SRC = ROOT / "bindings/rust/pixel-compare/src"


def _title(field: str) -> str:
    return "".join(part.capitalize() for part in field.split("_"))


def field_variants() -> tuple[dict[str, str], dict[str, str]]:
    authored: dict[str, str] = {}
    for row in csv.DictReader(SCHEMA.read_text().splitlines()):
        fields = row["computed_fields"].strip()
        if fields != "-":
            for field in fields.split(";"):
                if field in authored:
                    raise RuntimeError(f"duplicate property mapping for {field}")
                authored[field] = row["rust_name"]
    internal = {
        row["field"]: _title(row["field"])
        for row in csv.DictReader(INTERNAL.read_text().splitlines())
    }
    return authored, internal


NODE_VARIANTS = {
    "text": "Text",
    "replaced": "Replaced",
    "table_col_span": "TableColumnSpan",
    "table_row_span": "TableRowSpan",
    "form_control": "FormControl",
    "form_control_disabled": "FormControlDisabled",
    "form_control_native_appearance": "FormControlNativeAppearance",
    "embedded_document": "EmbeddedDocument",
    "embedded_canvas_color": "EmbeddedCanvasColor",
    "scroll_left": "ScrollLeft",
    "scroll_top": "ScrollTop",
    "scroll_marker_inactive_background": "ScrollMarkerInactiveBackground",
    "is_svg_foreign_object": "SvgForeignObject",
}


def _matching_paren(source: str, opening: int) -> int:
    depth = 0
    quote: str | None = None
    escape = False
    for index in range(opening, len(source)):
        char = source[index]
        if quote:
            if escape:
                escape = False
            elif char == "\\":
                escape = True
            elif char == quote:
                quote = None
            continue
        if char == '"':
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return index
    raise RuntimeError("unbalanced node_mut expression")


def _statement_end(source: str, start: int) -> int:
    round_depth = square_depth = brace_depth = 0
    quote: str | None = None
    escape = False
    for index in range(start, len(source)):
        char = source[index]
        if quote:
            if escape:
                escape = False
            elif char == "\\":
                escape = True
            elif char == quote:
                quote = None
            continue
        if char == '"':
            quote = char
        elif char == "(":
            round_depth += 1
        elif char == ")":
            round_depth -= 1
        elif char == "[":
            square_depth += 1
        elif char == "]":
            square_depth -= 1
        elif char == "{":
            brace_depth += 1
        elif char == "}":
            brace_depth -= 1
        elif char == ";" and round_depth == square_depth == brace_depth == 0:
            return index
    raise RuntimeError("unterminated node_mut assignment")


def lower_assignments(source: str) -> str:
    authored, internal = field_variants()
    marker = "doc.node_mut("
    assignment = re.compile(
        r"\s*\.\s*((?:style\s*\.\s*)?[a-z][a-z0-9_]*"
        r"(?:\s*\.\s*[a-z][a-z0-9_]*|\s*\[\s*\d+\s*\])?)\s*(=|\|=)\s*(?!=)"
    )
    cursor = 0
    output: list[str] = []
    while True:
        start = source.find(marker, cursor)
        if start < 0:
            output.append(source[cursor:])
            break
        output.append(source[cursor:start])
        opening = start + len(marker) - 1
        closing = _matching_paren(source, opening)
        node = source[opening + 1 : closing].strip()
        match = assignment.match(source, closing + 1)
        if not match:
            # A forbidden node_mut call which is not an assignment is retained
            # so the audit reports it with a useful source location.
            output.append(source[start : closing + 1])
            cursor = closing + 1
            continue
        target = re.sub(r"\s+", "", match.group(1))
        operator = match.group(2)
        value_start = match.end()
        end = _statement_end(source, value_start)
        value = source[value_start:end].strip()
        if target == "style":
            replacement = f"doc.install_derived_style({node}, {value});"
        elif target.startswith("style."):
            field = target.removeprefix("style.")
            if "[" in field:
                base, index = field.split("[", 1)
                index = index.rstrip("]")
                variant = authored.get(base)
                if not variant:
                    raise RuntimeError(f"unmapped indexed style field: {field}")
                replacement = (
                    "{ let mut value = doc.computed_style("
                    f"{node}).{base}.clone(); value[{index}] = {value}; "
                    f"doc.set_style({node}, RendererStyleValue::{variant}(value)); }}"
                )
            elif "." in field:
                base, member = field.split(".", 1)
                variant = authored.get(base)
                if not variant:
                    raise RuntimeError(f"unmapped compound style field: {field}")
                replacement = (
                    "{ let mut value = doc.computed_style("
                    f"{node}).{base}.clone(); value.{member} = {value}; "
                    f"doc.set_style({node}, RendererStyleValue::{variant}(value)); }}"
                )
            elif operator == "|=":
                variant = authored.get(field)
                if not variant:
                    raise RuntimeError(f"unmapped compound style field: {field}")
                replacement = (
                    f"doc.set_style({node}, RendererStyleValue::{variant}("
                    f"doc.computed_style({node}).{field} | {value}));"
                )
            elif field in authored:
                replacement = (
                    f"doc.set_style({node}, RendererStyleValue::{authored[field]}({value}));"
                )
            elif field in internal:
                replacement = (
                    "doc.set_internal_style("
                    f"{node}, RendererInternalStyleValue::{internal[field]}({value}));"
                )
            else:
                raise RuntimeError(f"unmapped computed style field: {field}")
        else:
            variant = NODE_VARIANTS.get(target)
            if not variant:
                raise RuntimeError(f"unmapped renderer node state: {target}")
            replacement = f"doc.set_node_state({node}, RendererNodeState::{variant}({value}));"
        output.append(replacement)
        cursor = end + 1
    return "".join(output)


def lower_style_reads(source: str) -> str:
    # Generated reads use simple handle variables. Keep this intentionally
    # narrow: a new expression form fails the audit instead of becoming a
    # permissive mutable-document escape hatch.
    source = source.replace(
        "&doc.node(doc.root()).style", "doc.computed_style(doc.root())"
    ).replace("doc.node(doc.root()).style", "doc.computed_style(doc.root())")
    source = re.sub(
        r"&doc\.node_mut\(([A-Za-z_][A-Za-z0-9_]*)\)\.style",
        r"doc.computed_style(\1)",
        source,
    )
    source = re.sub(
        r"doc\.node_mut\(([A-Za-z_][A-Za-z0-9_]*)\)\.style",
        r"doc.computed_style(\1)",
        source,
    )
    source = re.sub(
        r"&doc\.node\(([A-Za-z_][A-Za-z0-9_]*)\)\.style",
        r"doc.computed_style(\1)",
        source,
    )
    return re.sub(
        r"doc\.node\(([A-Za-z_][A-Za-z0-9_]*)\)\.style",
        r"doc.computed_style(\1)",
        source,
    )


def _builder_bodies(source: str) -> str:
    signature = re.compile(
        r"(?m)^(?P<prefix>(?:pub\s+)?fn\s+[A-Za-z0-9_]+)"
        r"\(\s*\)\s*->\s*Document\s*\{"
    )
    offset = 0
    output: list[str] = []
    while True:
        match = signature.search(source, offset)
        if not match:
            output.append(source[offset:])
            return "".join(output)
        output.append(source[offset:match.start()])
        brace = source.find("{", match.start())
        depth = 0
        quote: str | None = None
        escape = False
        end = None
        for index in range(brace, len(source)):
            char = source[index]
            if quote:
                if escape:
                    escape = False
                elif char == "\\":
                    escape = True
                elif char == quote:
                    quote = None
                continue
            if char == '"':
                quote = char
            elif char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    end = index
                    break
        if end is None:
            raise RuntimeError(f"unterminated builder {match.group('prefix')}")
        body = source[brace + 1 : end]
        body = re.sub(r"\b(base_doc|root_doc)\(\)", r"\1(viewport)", body)
        body, count = re.subn(r"\n([ \t]*)doc\s*\n\s*$", r"\n\1Ok(doc.into_engine())\n", body)
        if count != 1:
            delegated = re.fullmatch(
                r"\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)\s*", body, re.S
            )
            if not delegated:
                raise RuntimeError(f"builder does not end in doc: {match.group('prefix')}")
            separator = ", " if delegated.group(2).strip() else ""
            body = (
                "\n    Ok("
                f"{delegated.group(1)}(viewport{separator}{delegated.group(2)}).into_engine())\n"
            )
        replacement = (
            f"{match.group('prefix')}(viewport: ViewportMetrics) "
            f"-> Result<Engine, EngineError> {{{body}}}"
        )
        output.append(replacement)
        offset = end + 1


def engineify_module(source: str) -> str:
    source = source.replace(
        "use openui_dom::{Document, ElementTag, NodeId};",
        "use openui_dom::ElementTag;\n"
        "use openui_engine::{Engine, EngineError, NodeHandle as NodeId, RendererNodeState};",
    )
    source = source.replace(
        "use openui_dom::{Document, ElementTag};",
        "use openui_dom::ElementTag;\n"
        "use openui_engine::{Engine, EngineError, NodeHandle as NodeId, RendererNodeState};",
    )
    source = source.replace(
        "use openui_geometry::Length;",
        "use openui_geometry::{Length, ViewportMetrics};",
    )
    source = source.replace("use crate::base_doc;", "use crate::base_doc;")
    source = lower_assignments(source)
    source = lower_style_reads(source)
    source = _builder_bodies(source)
    source = source.replace("fn() -> Document", "fn(ViewportMetrics) -> Result<Engine, EngineError>")
    source = source.replace("as fn() -> Document", "as fn(ViewportMetrics) -> Result<Engine, EngineError>")
    return source


FORBIDDEN = (
    re.compile(r"\.node_mut\s*\("),
    re.compile(r"\.style\.[a-z][a-z0-9_]*\s*="),
    re.compile(r"&mut\s+ComputedStyle\b"),
    re.compile(r"fn\(\)\s*->\s*Document"),
)


def audit(paths: list[Path]) -> list[str]:
    failures: list[str] = []
    for path in paths:
        text = path.read_text()
        for pattern in FORBIDDEN:
            for match in pattern.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                failures.append(f"{path.relative_to(ROOT)}:{line}: {pattern.pattern}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--migrate", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = sorted(PIXEL_SRC.rglob("*.rs"))
    if args.migrate:
        for path in paths:
            if path.name == "main.rs":
                continue
            converted = engineify_module(path.read_text())
            path.write_text(converted)
    if args.check:
        failures = audit(paths)
        if failures:
            print("\n".join(failures[:100]))
            print(f"renderer fixture audit: failures={len(failures)}")
            return 1
        print(f"renderer fixture audit: files={len(paths)} failures=0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
