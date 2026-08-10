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
"""

from __future__ import annotations

import csv
import io
import json
import os
import re
import stat
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
SP14_W4_LIST = os.path.join(WPT_PORTED_DIR, "sp14_w4_residuals.json")
REPORT_COLUMNS = ["filename", "status", "fn_name", "reason"]

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


def load_ids_file(path: str) -> list[str]:
    """Load a strict, unique JSON ID list used by transactional batches."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
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


def _rust_function_span(rust_src: str, fn_name: str) -> tuple[int, int]:
    """Find exactly one Rust function and return its brace-balanced span.

    Braces inside strings and comments are ignored, so retained source text
    such as ``"{"`` cannot make surgical replacement consume adjacent code.
    """
    pattern = re.compile(
        rf"(?m)^fn\s+{re.escape(fn_name)}\s*\(\s*\)\s*->\s*Document\s*\{{"
    )
    matches = list(pattern.finditer(rust_src))
    if len(matches) != 1:
        raise KeyError(
            f"expected exactly one function {fn_name}, found {len(matches)}"
        )

    start = matches[0].start()
    i = rust_src.find("{", matches[0].start(), matches[0].end())
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
                raw = re.match(r'r(#{0,255})"', rust_src[i:])
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
    test_id: str, mapping: dict[str, dict[str, str]]
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

    root_aware = test_id in load_root_aware_ids()
    if root_aware:
        frozen_targets = set(load_ids_file(SP15_TARGETS_LIST))
        if test_id not in frozen_targets:
            raise ValueError(f"{test_id}: not in the frozen SP15 root-aware allowlist")
    parser = port_wpt.parse_wpt_html(upstream, root_aware=root_aware)
    portable, reason = port_wpt.analyze_portability(parser)
    if not root_aware and not portable:
        raise ValueError(f"{test_id}: not text-portable ({reason})")
    if not root_aware and not port_wpt.has_layout_content(parser):
        raise ValueError(f"{test_id}: not text-portable (no_layout_content)")

    fn_name = f"{module}_{port_wpt.sanitize_fn_name(name)}"
    return GeneratedReplacement(
        test_id=test_id,
        module=module,
        fn_name=fn_name,
        chromium_path=upstream_rel,
        rust_code=port_wpt.generate_rust_fn(
            fn_name, parser.root, parser.html_styles, root_aware=root_aware
        ),
        template=port_wpt.generate_html_template(upstream, root_aware=root_aware),
        root_aware=root_aware,
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
    """Return the original and deterministically promoted porter report."""
    with open(path, newline="", encoding="utf-8") as f:
        original = f.read()
    reader = csv.DictReader(io.StringIO(original, newline=""))
    if reader.fieldnames != REPORT_COLUMNS:
        raise ValueError(f"unexpected porter report columns in {path}: {reader.fieldnames}")
    rows = list(reader)
    by_name = {replacement.test_id.rsplit("/", 1)[1]: replacement for replacement in replacements}
    seen: set[str] = set()
    for row in rows:
        name = row["filename"]
        replacement = by_name.get(name)
        if replacement is None:
            continue
        if name in seen:
            raise ValueError(f"duplicate porter report row for {replacement.test_id}")
        row.update(status="ported", fn_name=replacement.fn_name, reason="")
        seen.add(name)
    missing = sorted(set(by_name) - seen)
    if missing:
        raise ValueError(f"missing porter report rows in {path}: {', '.join(missing)}")

    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=REPORT_COLUMNS, lineterminator="\r\n")
    writer.writeheader()
    writer.writerows(rows)
    return original, output.getvalue()


def _function_count(rust_src: str, fn_name: str) -> int:
    return len(
        re.findall(rf"(?m)^fn\s+{re.escape(fn_name)}\s*\(", rust_src)
    )


def _registry_identities(rust_src: str, test_id: str) -> list[str]:
    pattern = re.compile(
        rf'\(\s*"{re.escape(test_id)}"\s*,\s*'
        rf'(\w+)\s+as\s+fn\(\)\s*->\s*Document\s*,?\s*\)',
        re.DOTALL,
    )
    return pattern.findall(rust_src)


def _insert_rust_additions(
    rust_src: str, module: str, additions: list[GeneratedReplacement]
) -> str:
    """Insert builders and registry entries without rewriting the module."""
    registry = re.compile(
        rf"(?ms)^(pub fn {re.escape(module)}_registry\(\)\s*"
        rf"->\s*Vec<\(&'static str, fn\(\) -> Document\)>\s*\{{\s*"
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
            f"            {replacement.fn_name} as fn() -> Document,\n"
            "        ),"
        )

    match = matches[0]
    body = match.group(2).strip("\r\n").rstrip()
    if body:
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


def prepare_changes(
    test_ids: list[str], mapping: dict[str, dict[str, str]]
) -> tuple[list[GeneratedReplacement], dict[str, str], dict[str, str]]:
    """Generate and validate a complete transaction without writing files."""
    if len(test_ids) != len(set(test_ids)):
        raise ValueError("duplicate test IDs in one splice transaction")

    generated = [_generate_one(test_id, mapping) for test_id in sorted(test_ids)]
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
        module_template_path = os.path.join(
            WPT_PORTED_DIR, f"wpt_{module}_templates.json"
        )
        additions: list[GeneratedReplacement] = []
        for replacement in replacements:
            fn_count = _function_count(original, replacement.fn_name)
            identities = _registry_identities(original, replacement.test_id)
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
            if not fully_present and not fully_absent:
                raise ValueError(
                    f"{replacement.test_id}: partial template/registry state"
                )
            if fully_present:
                updated = replace_fn(
                    updated, replacement.fn_name, replacement.rust_code
                )
            else:
                additions.append(replacement)

            template_data[global_path][replacement.test_id] = replacement.template
            template_data[module_template_path][replacement.test_id] = (
                replacement.template
            )
        if additions:
            updated = _insert_rust_additions(updated, module, additions)
        if any(replacement.root_aware for replacement in replacements):
            updated = updated.replace(
                "use crate::base_doc;", "use crate::{base_doc, root_doc};", 1
            )
        originals[rust_path] = original
        changes[rust_path] = updated

        report_path = os.path.join(WPT_PORTED_DIR, f"wpt_{module}_report.csv")
        if os.path.exists(report_path):
            report_original, report_updated = _promote_report_rows(
                report_path, replacements
            )
            originals[report_path] = report_original
            changes[report_path] = report_updated

    for path in sorted(template_paths):
        changes[path] = json.dumps(template_data[path], indent=2) + "\n"

    with open(TEXT_PORTED_LIST, encoding="utf-8") as f:
        original_manifest = f.read()
    ported = json.loads(original_manifest)
    if (
        not isinstance(ported, list)
        or any(not isinstance(t, str) or not t for t in ported)
        or len(ported) != len(set(ported))
    ):
        raise ValueError(f"invalid text-port manifest: {TEXT_PORTED_LIST}")
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
    positional = []
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == "--dry-run":
            dry_run = True
        elif arg == "--ids-file":
            index += 1
            if index >= len(args) or ids_path is not None:
                print("ERROR: --ids-file requires exactly one path", file=sys.stderr)
                return 2
            ids_path = args[index]
        elif arg.startswith("--"):
            print(f"ERROR: unknown option: {arg}", file=sys.stderr)
            return 2
        else:
            positional.append(arg)
        index += 1

    if ids_path and positional:
        print("ERROR: do not combine --ids-file with positional IDs", file=sys.stderr)
        return 2

    # Enable symmetric deterministic-text generation for the whole prepare
    # phase. The standalone batch porter remains box-only by default.
    port_wpt.EMIT_TEXT_NODES = True
    port_wpt.RETAIN_TEXT = True

    try:
        test_ids = load_ids_file(ids_path) if ids_path else positional
        if not test_ids:
            raise ValueError("no test IDs supplied")
        mapping = load_mapping_rows()
        generated, originals, changes = prepare_changes(test_ids, mapping)
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
