#!/usr/bin/env python3
"""Surgically re-port WPT tests with deterministic text retained.

Every requested replacement is generated and validated in memory before any
file is changed. A real run atomically replaces the affected Rust modules,
their global/module templates, and ``text_ported_tests.json``. If a commit
step fails, already-replaced files are restored from their in-memory originals.

Usage:
  python3 tools/wpt/splice_text_port.py wpt/css2_floats/example [...]
  python3 tools/wpt/splice_text_port.py --dry-run wpt/css2_floats/example
  python3 tools/wpt/splice_text_port.py --ids-file targets.json [--dry-run]
  python3 tools/wpt/splice_text_port.py --preserve-templates --ids-file targets.json
"""

from __future__ import annotations

import csv
import io
import json
import os
import re
import resource
import stat
import subprocess
import sys
import tempfile
from collections import defaultdict
from dataclasses import dataclass

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(os.path.dirname(SCRIPT_DIR))
WPT_ROOT = os.path.expanduser(
    "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
)
MAPPING_CSV = os.path.join(
    PROJECT_ROOT, "tools", "accountability", "data", "wpt_mapping.csv"
)
WPT_PORTED_DIR = os.path.join(
    PROJECT_ROOT, "tools", "accountability", "data", "wpt_ported"
)
RUST_WPT_DIR = os.path.join(
    PROJECT_ROOT, "bindings", "rust", "pixel-compare", "src", "wpt"
)
TEXT_PORTED_LIST = os.path.join(WPT_PORTED_DIR, "text_ported_tests.json")
SP15_TARGETS_LIST = os.path.join(WPT_PORTED_DIR, "sp15_actionable_targets.json")
SP13R_TARGETS_LIST = os.path.join(WPT_PORTED_DIR, "sp13r_multicol_targets.json")
SP16_REAL_LIST = os.path.join(WPT_PORTED_DIR, "sp16_real_font_tests.json")
SP14_W4_LIST = os.path.join(WPT_PORTED_DIR, "sp14_w4_residuals.json")
SP18_TARGETS_LIST = os.path.join(SCRIPT_DIR, "sp18_targets.json")
REPORT_COLUMNS = ["filename", "status", "fn_name", "reason"]
RUST_RAW_STRING_START = re.compile(r'r(#{0,255})"')

sys.path.insert(0, SCRIPT_DIR)
import port_wpt  # noqa: E402


@dataclass(frozen=True)
class GeneratedReplacement:
    test_id: str
    module: str
    fn_name: str
    chromium_path: str
    rust_code: str
    template: str
    root_aware: bool
    retains_text: bool


def canonical_test_id(row: dict[str, str]) -> str:
    """Derive the stable Open UI identity for any mapping row."""
    area = row.get("sp_area", "").strip()
    name = row.get("test_name", "").strip()
    if not area or not name or "/" in area or "/" in name:
        raise ValueError(
            "mapping row has invalid canonical identity: "
            f"area={area!r}, name={name!r}"
        )
    canonical = f"wpt/{area}/{name}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != canonical:
        raise ValueError(
            f"mapping identity mismatch: {recorded!r} != {canonical!r}"
        )
    return canonical


def load_mapping_rows() -> dict[str, dict[str, str]]:
    """Load canonical identities for ported and unported mapping rows."""
    rows: dict[str, dict[str, str]] = {}
    with open(MAPPING_CSV, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            test_id = canonical_test_id(row)
            if test_id in rows:
                raise ValueError(f"ambiguous mapping for {test_id}")
            rows[test_id] = row
    return rows


def load_ids_file(path: str, partition: str | None = None) -> list[str]:
    """Load a strict, unique JSON ID list used by transactional batches."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    if partition is not None:
        if not isinstance(data, dict) or partition not in data:
            raise ValueError(f"missing IDs partition {partition!r}: {path}")
        data = data[partition]
    if (
        not isinstance(data, list)
        or any(not isinstance(test_id, str) or not test_id for test_id in data)
        or len(data) != len(set(data))
    ):
        raise ValueError(f"invalid IDs ledger: {path}")
    return data


def load_root_aware_ids() -> set[str]:
    """Recover root/body membership from history and committed templates."""
    with open(SP14_W4_LIST, encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, list):
        raise ValueError(f"invalid SP14 W4 ledger: {SP14_W4_LIST}")
    result = set()
    for item in data:
        if not isinstance(item, dict):
            raise ValueError(f"invalid SP14 W4 ledger entry: {item!r}")
        test_id = item.get("test_id", "")
        owners = item.get("owner_categories", [])
        if not isinstance(test_id, str) or not isinstance(owners, list):
            raise ValueError(f"invalid SP14 W4 ledger entry: {item!r}")
        if "needs_root_body_layout" in owners:
            result.add(test_id)
    templates_path = os.path.join(WPT_PORTED_DIR, "all_wpt_templates.json")
    if os.path.exists(templates_path):
        with open(templates_path, encoding="utf-8") as f:
            templates = json.load(f)
        if not isinstance(templates, dict):
            raise ValueError(f"invalid global templates: {templates_path}")
        result.update(
            test_id
            for test_id, template in templates.items()
            if isinstance(template, str)
            and template.startswith("<!--OPENUI_ROOT_AWARE-->")
        )
    return result


_DISTINCT_ROOT_BOX_PROPERTIES = {
    "display", "position", "float", "clear", "overflow", "overflow-x",
    "overflow-y", "opacity", "box-shadow", "outline", "columns",
    "column-count", "column-width", "column-height", "column-gap",
    "column-rule", "column-rule-width", "column-rule-style",
    "column-rule-color", "column-fill", "column-span",
    "background", "background-color", "background-image", "background-repeat",
    "background-position", "background-size", "background-origin",
    "background-clip", "background-attachment",
    "contain", "content-visibility", "contain-intrinsic-size",
    "contain-intrinsic-width", "contain-intrinsic-height", "container-type",
}


def requires_distinct_root_box(parser: port_wpt.WptHtmlParser) -> bool:
    """Whether author CSS gives the document element observable box styling.

    The compact porter normally represents ``body`` directly with ``base_doc``.
    That is insufficient when an author selector also gives ``html`` its own
    border, sizing, spacing, or multicol box. Detect that semantic case from
    author rules so regeneration selects the existing root-aware document
    model without relying on a test-ID allowlist.
    """
    rules = parser.external_css_rules + parser.css_rules
    for selector, styles in rules:
        matches_html = port_wpt.match_selector(
            selector, "html", [], "", [], 1, 1, []
        )
        matches_body = port_wpt.match_selector(
            selector, "body", [], "", [("html", [], "")], 1, 1, []
        )
        if not matches_html and not matches_body:
            continue
        for prop in styles:
            if matches_body and prop in {
                "contain", "content-visibility", "container-type",
            }:
                return True
            if (
                prop in _DISTINCT_ROOT_BOX_PROPERTIES
                or prop.startswith(("border-", "margin-", "padding-"))
                or prop in {
                    "width", "min-width", "max-width", "height", "min-height",
                    "max-height", "inline-size", "min-inline-size", "max-inline-size",
                    "block-size", "min-block-size", "max-block-size", "transform",
                    "transform-origin",
                }
            ):
                return True
    return False


_ENGINE_FUNCTION_HEADER = re.compile(
    r"(?m)^fn\s+(?P<name>\w+)\s*\(\s*"
    r"viewport:\s*ViewportMetrics\s*,?\s*\)\s*->\s*"
    r"Result<Engine,\s*EngineError>\s*\{"
)

_FROZEN_FONT_OVERRIDE = re.compile(
    r"body\s*,\s*body\s*\*\s*\{[^}]*?font-family\s*:\s*"
    r"(?P<family>.*?)\s*!important",
    re.IGNORECASE | re.DOTALL,
)


def _frozen_font_family_rust(template: str) -> str | None:
    """Return the exact deterministic family encoded by a frozen fixture.

    Earlier qualification templates intentionally use a smaller fallback set
    than newly generated fixtures.  Rust-only regeneration must retain that
    order: changing it alters fallback glyph metrics and therefore changes the
    oracle even when the authored WPT markup is untouched.
    """
    matches = list(_FROZEN_FONT_OVERRIDE.finditer(template))
    if not matches:
        return None
    return port_wpt._font_family_to_rust(matches[-1].group("family"))


def _rust_function_span(rust_src: str, fn_name: str) -> tuple[int, int]:
    """Find exactly one Rust function and return its brace-balanced span.

    Braces inside strings and comments are ignored, so retained source text
    such as ``"{"`` cannot make surgical replacement consume adjacent code.
    """
    matches = [
        match
        for match in _ENGINE_FUNCTION_HEADER.finditer(rust_src)
        if match.group("name") == fn_name
    ]
    if len(matches) != 1:
        raise KeyError(
            f"expected exactly one function {fn_name}, found {len(matches)}"
        )
    return _rust_function_span_from_header(rust_src, fn_name, matches[0])


def _rust_function_span_from_header(
    rust_src: str, fn_name: str, header: re.Match[str]
) -> tuple[int, int]:
    start = header.start()
    i = rust_src.find("{", header.start(), header.end())
    depth = 0
    state = "code"
    block_depth = 0
    raw_hashes = 0
    while i < len(rust_src):
        c = rust_src[i]
        nxt = rust_src[i + 1] if i + 1 < len(rust_src) else ""
        if state == "line_comment":
            if c == "\n":
                state = "code"
        elif state == "block_comment":
            if c == "/" and nxt == "*":
                block_depth += 1
                i += 1
            elif c == "*" and nxt == "/":
                block_depth -= 1
                i += 1
                if block_depth == 0:
                    state = "code"
        elif state == "string":
            if c == "\\":
                i += 1
            elif c == '"':
                state = "code"
        elif state == "char":
            if c == "\\":
                i += 1
            elif c == "'":
                state = "code"
        elif state == "raw":
            closing = '"' + ("#" * raw_hashes)
            if rust_src.startswith(closing, i):
                i += len(closing) - 1
                state = "code"
        else:
            if c == "/" and nxt == "/":
                state = "line_comment"
                i += 1
            elif c == "/" and nxt == "*":
                state = "block_comment"
                block_depth = 1
                i += 1
            elif c == '"':
                state = "string"
            elif c == "'":
                state = "char"
            elif c == "r":
                raw = RUST_RAW_STRING_START.match(rust_src, i)
                if raw:
                    raw_hashes = len(raw.group(1))
                    i += len(raw.group(0)) - 1
                    state = "raw"
            elif c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    return start, i + 1
        i += 1
    raise ValueError(f"unbalanced braces replacing {fn_name}")


def replace_fn(rust_src: str, fn_name: str, new_fn_code: str) -> str:
    start, end = _rust_function_span(rust_src, fn_name)
    return rust_src[:start] + new_fn_code.rstrip("\n") + rust_src[end:]


def _generate_one(
    test_id: str,
    mapping: dict[str, dict[str, str]],
    profile: port_wpt.PorterProfile,
    text_manifest: set[str],
    modern_line_clamp_ids: set[str],
    *,
    paint_layers: bool = False,
    frozen_template: str | None = None,
) -> GeneratedReplacement:
    parts = test_id.split("/")
    if len(parts) != 3 or parts[0] != "wpt" or not parts[1] or not parts[2]:
        raise ValueError(f"invalid test id: {test_id}")
    row = mapping.get(test_id)
    if row is None:
        raise KeyError(f"{test_id}: not in wpt_mapping.csv")

    module, name = parts[1], parts[2]
    if row.get("sp_area") != module:
        raise ValueError(
            f"{test_id}: mapping area {row.get('sp_area')!r} does not match {module!r}"
        )
    upstream_rel = row.get("chromium_test_path", "")
    upstream = os.path.join(WPT_ROOT, upstream_rel)
    if not upstream_rel or not os.path.isfile(upstream):
        raise FileNotFoundError(f"{test_id}: upstream missing ({upstream})")

    frozen_targets = set(load_ids_file(SP15_TARGETS_LIST)) | set(
        load_ids_file(SP13R_TARGETS_LIST)
    )
    # Historical root-aware ports remain authorized by their frozen ledgers.
    # New cohorts must independently re-derive the need for a distinct root
    # box from upstream CSS on every splice; a marker written by a prior run is
    # not itself authorization and therefore cannot make idempotence fail.
    root_aware = test_id in load_root_aware_ids() and test_id in frozen_targets
    retains_text = (
        True
        if profile is port_wpt.PorterProfile.DETERMINISTIC_AHEM
        else test_id in text_manifest
    )
    port_wpt.set_porter_profile(profile, retain_text=retains_text)
    port_wpt.set_modern_line_clamp_enabled(test_id in modern_line_clamp_ids)
    mutation_ir = port_wpt._candidate_mutation_ir(test_id)
    if frozen_template is None:
        parser = port_wpt.parse_wpt_html(
            upstream,
            root_aware=root_aware,
            mutation_ir=mutation_ir,
        )
    else:
        parser = port_wpt._parse_wpt_markup(
            frozen_template,
            os.path.dirname(upstream),
            root_aware=root_aware,
        )
    if not root_aware and requires_distinct_root_box(parser):
        root_aware = True
        if frozen_template is None:
            parser = port_wpt.parse_wpt_html(
                upstream,
                root_aware=True,
                mutation_ir=mutation_ir,
            )
        else:
            parser = port_wpt._parse_wpt_markup(
                frozen_template,
                os.path.dirname(upstream),
                root_aware=True,
            )
    portable, reason = port_wpt.analyze_portability(parser)
    if not root_aware and not portable and not paint_layers:
        raise ValueError(f"{test_id}: not text-portable ({reason})")
    if not root_aware and not port_wpt.has_layout_content(parser) and not paint_layers:
        raise ValueError(f"{test_id}: not text-portable (no_layout_content)")

    fn_name = f"{module}_{port_wpt.sanitize_fn_name(name)}"
    previous_font_family = port_wpt.DETERMINISTIC_FONT_FAMILY_RUST
    frozen_font_family = (
        _frozen_font_family_rust(frozen_template)
        if frozen_template is not None
        else None
    )
    if frozen_font_family is not None:
        port_wpt.DETERMINISTIC_FONT_FAMILY_RUST = frozen_font_family
    try:
        rust_code = port_wpt.generate_rust_fn(
            fn_name, parser.root, parser.html_styles, root_aware=root_aware
        )
    finally:
        port_wpt.DETERMINISTIC_FONT_FAMILY_RUST = previous_font_family
    rust_code = _rustfmt_source(rust_code, f"{fn_name}.rs").rstrip("\n")
    return GeneratedReplacement(
        test_id=test_id,
        module=module,
        fn_name=fn_name,
        chromium_path=upstream_rel,
        rust_code=rust_code,
        template=(
            frozen_template
            if frozen_template is not None
            else port_wpt.generate_html_template(
                upstream,
                root_aware=root_aware,
                final_state_parser=(
                    parser if parser.lowered_final_state is not None else None
                ),
            )
        ),
        root_aware=root_aware,
        retains_text=retains_text,
    )


def _load_json_object(path: str) -> tuple[str, dict[str, str]]:
    """Read a JSON object while rejecting duplicate keys."""
    with open(path, encoding="utf-8") as f:
        original = f.read()

    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key {key!r} in {path}")
            result[key] = value
        return result

    value = json.loads(original, object_pairs_hook=unique_object)
    if not isinstance(value, dict) or any(
        not isinstance(key, str) or not isinstance(item, str)
        for key, item in value.items()
    ):
        raise ValueError(f"template file is not a string object: {path}")
    return original, value


def _promote_report_rows(
    path: str, replacements: list[GeneratedReplacement]
) -> tuple[str, str]:
    """Return the original and deterministically synchronized porter report."""
    with open(path, newline="", encoding="utf-8") as f:
        original = f.read()
    reader = csv.DictReader(io.StringIO(original, newline=""))
    if reader.fieldnames != REPORT_COLUMNS:
        raise ValueError(f"unexpected porter report columns in {path}: {reader.fieldnames}")
    rows = list(reader)
    by_name = {replacement.test_id.rsplit("/", 1)[1]: replacement for replacement in replacements}
    seen: set[str] = set()
    updated = original

    def encoded_row(row: dict[str, str], line_terminator: str) -> str:
        output = io.StringIO(newline="")
        writer = csv.DictWriter(
            output,
            fieldnames=REPORT_COLUMNS,
            lineterminator=line_terminator,
        )
        writer.writerow(row)
        return output.getvalue()

    def line_occurrences(haystack: str, needle: str) -> list[int]:
        """Find complete CSV records, not basename substrings of other rows."""
        result = []
        start = 0
        while True:
            index = haystack.find(needle, start)
            if index < 0:
                return result
            if index == 0 or haystack[index - 1] == "\n":
                result.append(index)
            start = index + 1

    for row in rows:
        name = row["filename"]
        replacement = by_name.get(name)
        if replacement is None:
            continue
        if name in seen:
            raise ValueError(f"duplicate porter report row for {replacement.test_id}")
        replacement_row = dict(row)
        replacement_row.update(
            status="ported", fn_name=replacement.fn_name, reason=""
        )
        if replacement_row != row:
            old_variants = (
                encoded_row(row, "\r\n"),
                encoded_row(row, "\n"),
            )
            matches = [
                (value, offsets[0])
                for value in old_variants
                if len(offsets := line_occurrences(updated, value)) == 1
            ]
            if len(matches) != 1:
                raise ValueError(
                    f"cannot isolate porter report row for {replacement.test_id}"
                )
            # Changed records use LF so `git diff --check` does not treat a
            # newly introduced CR as trailing whitespace. All untouched report
            # bytes, including historical CRLF records and quoted newlines,
            # remain byte-identical.
            old_value, offset = matches[0]
            updated = (
                updated[:offset]
                + encoded_row(replacement_row, "\n")
                + updated[offset + len(old_value):]
            )
        seen.add(name)
    missing = sorted(set(by_name) - seen)
    for name in missing:
        replacement = by_name[name]
        if updated and not updated.endswith(("\n", "\r")):
            updated += "\n"
        updated += encoded_row(
            {
                "filename": name,
                "status": "ported",
                "fn_name": replacement.fn_name,
                "reason": "",
            },
            "\n",
        )
    return original, updated


def _function_count(rust_src: str, fn_name: str) -> int:
    return len(
        re.findall(rf"(?m)^fn\s+{re.escape(fn_name)}\s*\(", rust_src)
    )


_ENGINE_REGISTRY_ENTRY = re.compile(
    r'\(\s*"(?P<test_id>[^"]+)"\s*,\s*'
    r'(?P<fn_name>\w+)\s+as\s+fn\(ViewportMetrics\)\s*->\s*'
    r'Result<Engine,\s*EngineError>\s*,?\s*\)',
    re.DOTALL,
)


def _registry_identities(rust_src: str, test_id: str) -> list[str]:
    return [
        match.group("fn_name")
        for match in _ENGINE_REGISTRY_ENTRY.finditer(rust_src)
        if match.group("test_id") == test_id
    ]


def _insert_rust_additions(
    rust_src: str, module: str, additions: list[GeneratedReplacement]
) -> str:
    """Insert builders and registry entries without rewriting the module."""
    registry = re.compile(
        rf"(?ms)^(pub fn {re.escape(module)}_registry\(\)\s*"
        rf"->\s*Vec<\(\s*&'static str,\s*fn\(ViewportMetrics\)\s*"
        rf"->\s*Result<Engine,\s*EngineError>,?\s*\)>\s*\{{\s*"
        rf"vec!\[)(.*?)(\s*\]\s*\}}\s*)\Z"
    )
    matches = list(registry.finditer(rust_src))
    if len(matches) != 1:
        raise ValueError(
            f"{module}: expected one terminal registry, found {len(matches)}"
        )

    additions = sorted(additions, key=lambda item: item.test_id)
    builders = []
    entries = []
    for replacement in additions:
        builders.append(
            f"// Source: {replacement.chromium_path}\n"
            f"{replacement.rust_code.rstrip()}\n\n"
        )
        entries.append(
            "        (\n"
            f'            "{replacement.test_id}",\n'
            f"            {replacement.fn_name} as fn(ViewportMetrics) -> Result<Engine, EngineError>,\n"
            "        ),"
        )

    match = matches[0]
    body = match.group(2).strip("\r\n").rstrip()
    if body:
        # Existing generated registries are not uniformly rustfmt-expanded;
        # a compact final tuple may omit its optional trailing comma. Make
        # that separator explicit before appending the first new tuple.
        if not body.endswith(","):
            body += ","
        body += "\n"
    body += "\n".join(entries) + "\n"
    prefix = match.group(1)
    if not prefix.endswith("\n"):
        prefix += "\n"
    suffix = match.group(3).lstrip("\r\n")
    new_registry = prefix + body + suffix
    return (
        rust_src[: match.start()]
        + "".join(builders)
        + new_registry
        + rust_src[match.end() :]
    )


def _preserve_existing_builder_profile(existing_fn: str, generated_fn: str) -> str:
    """Retain frozen builder-level runner semantics during regeneration.

    Historical retained-text builders explicitly blockified the synthetic
    body while newer builders intentionally keep ``base_doc``'s flow-root.
    This distinction is already part of their frozen pixel profiles, so a
    surgical re-port preserves the explicit marker when it exists instead of
    silently migrating the historical builder.
    """
    body_display = "doc.set_style(vp, RendererStyleValue::Display(Display::Block));"
    if body_display not in existing_fn or body_display in generated_fn:
        return generated_fn
    constructor = re.search(
        r"(?m)^(\s*let \(mut doc, (?:html, )?vp\) = "
        r"(?:base|root)_doc\(viewport\);\s*)$",
        generated_fn,
    )
    if constructor is None:
        raise ValueError("generated builder has no document constructor")
    indent = re.match(r"\s*", constructor.group(1)).group(0)
    insertion = constructor.end()
    return (
        generated_fn[:insertion]
        + f"\n{indent}{body_display}"
        + generated_fn[insertion:]
    )


def prepare_changes(
    test_ids: list[str],
    mapping: dict[str, dict[str, str]],
    *,
    profile: port_wpt.PorterProfile = port_wpt.PorterProfile.DETERMINISTIC_AHEM,
    paint_layers: bool = False,
    preserve_templates: bool = False,
) -> tuple[list[GeneratedReplacement], dict[str, str], dict[str, str]]:
    """Generate and validate a complete transaction without writing files."""
    if len(test_ids) != len(set(test_ids)):
        raise ValueError("duplicate test IDs in one splice transaction")

    with open(TEXT_PORTED_LIST, encoding="utf-8") as f:
        original_manifest = f.read()
    ported = json.loads(original_manifest)
    if (
        not isinstance(ported, list)
        or any(not isinstance(t, str) or not t for t in ported)
        or len(ported) != len(set(ported))
    ):
        raise ValueError(f"invalid text-port manifest: {TEXT_PORTED_LIST}")
    text_manifest = set(ported)

    frozen_templates: dict[str, str] = {}
    if preserve_templates:
        frozen_template_path = os.path.join(
            WPT_PORTED_DIR, "all_wpt_templates.json"
        )
        _, frozen_templates = _load_json_object(frozen_template_path)
        missing_frozen = sorted(set(test_ids) - set(frozen_templates))
        if missing_frozen:
            raise ValueError(
                "--preserve-templates IDs missing from frozen template ledger: "
                + ", ".join(missing_frozen)
            )

    real_font_ids = (
        set(load_ids_file(SP16_REAL_LIST))
        if os.path.exists(SP16_REAL_LIST)
        else set()
    )
    modern_line_clamp_ids = set(load_ids_file(SP18_TARGETS_LIST))
    if profile is port_wpt.PorterProfile.REAL_FONT:
        allowed = real_font_ids
        outside = set(test_ids) - allowed
        if outside:
            raise ValueError(
                "real-font profile contains IDs outside the frozen SP16 manifest: "
                + ", ".join(sorted(outside))
            )

    previous_profile = port_wpt.ACTIVE_PORTER_PROFILE
    previous_emit_text = port_wpt.EMIT_TEXT_NODES
    previous_retain_text = port_wpt.RETAIN_TEXT
    previous_paint_layers = port_wpt.EMIT_PAINT_LAYERS
    previous_modern_line_clamp = port_wpt.MODERN_LINE_CLAMP_ENABLED
    port_wpt.set_paint_layer_emission(paint_layers)
    try:
        generated = [
            _generate_one(
                test_id,
                mapping,
                (
                    port_wpt.PorterProfile.REAL_FONT
                    if test_id in real_font_ids
                    else profile
                ),
                text_manifest,
                modern_line_clamp_ids,
                paint_layers=paint_layers,
                frozen_template=frozen_templates.get(test_id),
            )
            for test_id in sorted(test_ids)
        ]
    finally:
        port_wpt.ACTIVE_PORTER_PROFILE = previous_profile
        port_wpt.EMIT_TEXT_NODES = previous_emit_text
        port_wpt.RETAIN_TEXT = previous_retain_text
        port_wpt.set_paint_layer_emission(previous_paint_layers)
        port_wpt.set_modern_line_clamp_enabled(previous_modern_line_clamp)
    fn_names = [replacement.fn_name for replacement in generated]
    if len(fn_names) != len(set(fn_names)):
        raise ValueError("generated function-name collision in splice transaction")
    originals: dict[str, str] = {}
    changes: dict[str, str] = {}

    by_module: dict[str, list[GeneratedReplacement]] = defaultdict(list)
    for replacement in generated:
        by_module[replacement.module].append(replacement)

    template_paths = {os.path.join(WPT_PORTED_DIR, "all_wpt_templates.json")}
    template_paths.update(
        os.path.join(WPT_PORTED_DIR, f"wpt_{module}_templates.json")
        for module in by_module
    )
    template_data: dict[str, dict[str, str]] = {}
    for path in sorted(template_paths):
        original, templates = _load_json_object(path)
        originals[path] = original
        template_data[path] = templates

    global_path = os.path.join(WPT_PORTED_DIR, "all_wpt_templates.json")
    for module, replacements in sorted(by_module.items()):
        rust_path = os.path.join(RUST_WPT_DIR, f"wpt_{module}.rs")
        with open(rust_path, encoding="utf-8") as f:
            original = f.read()
        updated = original
        function_headers: dict[str, list[re.Match[str]]] = defaultdict(list)
        for header in _ENGINE_FUNCTION_HEADER.finditer(original):
            function_headers[header.group("name")].append(header)
        registry_identities: dict[str, list[str]] = defaultdict(list)
        for entry in _ENGINE_REGISTRY_ENTRY.finditer(original):
            registry_identities[entry.group("test_id")].append(
                entry.group("fn_name")
            )
        module_template_path = os.path.join(
            WPT_PORTED_DIR, f"wpt_{module}_templates.json"
        )
        additions: list[GeneratedReplacement] = []
        planned_replacements: list[tuple[int, int, str]] = []
        for replacement in replacements:
            headers = function_headers.get(replacement.fn_name, [])
            fn_count = len(headers)
            identities = registry_identities.get(replacement.test_id, [])
            global_has = replacement.test_id in template_data[global_path]
            module_has = replacement.test_id in template_data[module_template_path]
            if fn_count > 1 or len(identities) > 1:
                raise ValueError(
                    f"{replacement.test_id}: duplicate Rust function or registry identity"
                )
            fully_present = (
                fn_count == 1
                and identities == [replacement.fn_name]
                and global_has
                and module_has
            )
            fully_absent = (
                fn_count == 0
                and not identities
                and not global_has
                and not module_has
            )
            if preserve_templates and not fully_present:
                raise ValueError(
                    f"{replacement.test_id}: --preserve-templates requires an "
                    "existing frozen template and registry entry"
                )
            if not fully_present and not fully_absent:
                raise ValueError(
                    f"{replacement.test_id}: partial template/registry state"
            )
            if fully_present:
                existing_start, existing_end = _rust_function_span_from_header(
                    original, replacement.fn_name, headers[0]
                )
                replacement_code = _preserve_existing_builder_profile(
                    original[existing_start:existing_end], replacement.rust_code
                )
                if replacement_code != original[existing_start:existing_end]:
                    planned_replacements.append(
                        (existing_start, existing_end, replacement_code.rstrip("\n"))
                    )
            else:
                additions.append(replacement)

            if not preserve_templates:
                template_data[global_path][replacement.test_id] = replacement.template
                template_data[module_template_path][replacement.test_id] = (
                    replacement.template
                )
        # Apply offsets from the end once. Replacing one function at a time
        # repeatedly copied multi-megabyte modules and made a 132-item
        # no-write verification quadratic in module size.
        for start, end, replacement_code in sorted(
            planned_replacements, reverse=True
        ):
            updated = updated[:start] + replacement_code + updated[end:]
        if additions:
            updated = _insert_rust_additions(updated, module, additions)
        if any(replacement.root_aware for replacement in replacements):
            updated = updated.replace(
                "use crate::base_doc;", "use crate::{base_doc, root_doc};", 1
            )
        originals[rust_path] = original
        # Generated functions are canonicalized individually in `_generate_one`.
        # Formatting the complete multi-megabyte area module here made a
        # surgical no-op rewrite every unrelated builder and turned an
        # idempotence check into minutes of redundant work.
        changes[rust_path] = updated

        report_path = os.path.join(WPT_PORTED_DIR, f"wpt_{module}_report.csv")
        if os.path.exists(report_path):
            report_original, report_updated = _promote_report_rows(
                report_path, replacements
            )
            originals[report_path] = report_original
            changes[report_path] = report_updated

    if not preserve_templates:
        for path in sorted(template_paths):
            changes[path] = json.dumps(template_data[path], indent=2) + "\n"

    # Paint-layer emission is orthogonal to the authored-content profile.  In
    # particular, SP20 background/overflow rows still retain their text while
    # opting into generated background, border-image, and pseudo paint data.
    if profile is port_wpt.PorterProfile.DETERMINISTIC_AHEM:
        ported = sorted(set(ported).union(test_ids))
        originals[TEXT_PORTED_LIST] = original_manifest
        changes[TEXT_PORTED_LIST] = json.dumps(ported, indent=2) + "\n"

    return generated, originals, changes


def _stage_text(path: str, content: str, mode: int) -> str:
    fd, staged = tempfile.mkstemp(
        dir=os.path.dirname(path), prefix=f".{os.path.basename(path)}.", suffix=".tmp"
    )
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(content)
            f.flush()
            os.fsync(f.fileno())
        os.chmod(staged, stat.S_IMODE(mode))
        return staged
    except BaseException:
        if os.path.exists(staged):
            os.unlink(staged)
        raise


def _rustfmt_source(source: str, path: str) -> str:
    """Format one generated Rust item before the transaction compares it.

    Canonicalizing the replacement before insertion makes repeat generation
    byte-idempotent without formatting megabytes of unrelated builders.
    """
    rustfmt_env = os.environ.copy()
    # Large generated WPT modules contain thousands of builder functions;
    # rustfmt's recursive syntax visitor needs a larger stack than its small
    # worker-thread default when an entire area is updated transactionally.
    rustfmt_env["RUST_MIN_STACK"] = str(512 * 1024 * 1024)

    def raise_main_stack_limit() -> None:
        _soft, hard = resource.getrlimit(resource.RLIMIT_STACK)
        desired = 512 * 1024 * 1024
        if hard != resource.RLIM_INFINITY:
            desired = min(desired, hard)
        resource.setrlimit(resource.RLIMIT_STACK, (desired, hard))

    completed = subprocess.run(
        ["rustfmt", "--edition", "2021"],
        input=source,
        text=True,
        capture_output=True,
        cwd=os.path.join(PROJECT_ROOT, "bindings", "rust"),
        env=rustfmt_env,
        preexec_fn=raise_main_stack_limit,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or "unknown rustfmt failure"
        raise ValueError(f"rustfmt failed for {path}: {detail}")
    return completed.stdout


def commit_changes(originals: dict[str, str], changes: dict[str, str]) -> None:
    """Atomically replace each file and roll back the set on any failure."""
    staged: dict[str, str] = {}
    modes = {path: os.stat(path).st_mode for path in changes}
    try:
        for path in sorted(changes):
            staged[path] = _stage_text(path, changes[path], modes[path])
        replaced: list[str] = []
        try:
            for path in sorted(changes):
                os.replace(staged[path], path)
                staged.pop(path)
                replaced.append(path)
        except BaseException:
            for path in reversed(replaced):
                rollback = _stage_text(path, originals[path], modes[path])
                os.replace(rollback, path)
            raise
    finally:
        for staged_path in staged.values():
            if os.path.exists(staged_path):
                os.unlink(staged_path)


def main() -> int:
    args = sys.argv[1:]
    dry_run = False
    ids_path = None
    partition = None
    profile = port_wpt.PorterProfile.DETERMINISTIC_AHEM
    paint_layers = False
    preserve_templates = False
    positional = []
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == "--dry-run":
            dry_run = True
        elif arg == "--paint-layers":
            paint_layers = True
        elif arg == "--preserve-templates":
            preserve_templates = True
        elif arg == "--ids-file":
            index += 1
            if index >= len(args) or ids_path is not None:
                print("ERROR: --ids-file requires exactly one path", file=sys.stderr)
                return 2
            ids_path = args[index]
        elif arg == "--partition":
            index += 1
            if index >= len(args) or partition is not None:
                print("ERROR: --partition requires exactly one key", file=sys.stderr)
                return 2
            partition = args[index]
        elif arg == "--profile":
            index += 1
            if index >= len(args):
                print("ERROR: --profile requires a value", file=sys.stderr)
                return 2
            profiles = {item.value: item for item in port_wpt.PorterProfile}
            try:
                profile = profiles[args[index]]
            except KeyError:
                print(
                    "ERROR: --profile must be legacy-box-only, "
                    "deterministic-ahem, or real-font",
                    file=sys.stderr,
                )
                return 2
        elif arg.startswith("--"):
            print(f"ERROR: unknown option: {arg}", file=sys.stderr)
            return 2
        else:
            positional.append(arg)
        index += 1

    if ids_path and positional:
        print("ERROR: do not combine --ids-file with positional IDs", file=sys.stderr)
        return 2
    if partition is not None and ids_path is None:
        print("ERROR: --partition requires --ids-file", file=sys.stderr)
        return 2

    try:
        test_ids = load_ids_file(ids_path, partition) if ids_path else positional
        if not test_ids:
            raise ValueError("no test IDs supplied")
        mapping = load_mapping_rows()
        generated, originals, changes = prepare_changes(
            test_ids,
            mapping,
            profile=profile,
            paint_layers=paint_layers,
            preserve_templates=preserve_templates,
        )
        if dry_run:
            for replacement in generated:
                print(f"=== {replacement.test_id} (fn {replacement.fn_name}) ===")
                print(replacement.rust_code)
                print("--- template ---")
                print(replacement.template)
            print(f"DRY RUN: {len(generated)}/{len(test_ids)} validated; no files written")
            return 0
        commit_changes(originals, changes)
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    for replacement in generated:
        rust_path = os.path.join(
            RUST_WPT_DIR, f"wpt_{replacement.module}.rs"
        )
        print(
            f"SPLICED {replacement.test_id} -> "
            f"{os.path.relpath(rust_path, PROJECT_ROOT)}"
        )
    print(f"{len(generated)}/{len(test_ids)} spliced transactionally")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
