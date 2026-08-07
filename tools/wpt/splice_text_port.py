#!/usr/bin/env python3
"""Surgically re-port WPT tests with deterministic text retained.

Every requested replacement is generated and validated in memory before any
file is changed. A real run atomically replaces the affected Rust modules,
their global/module templates, and ``text_ported_tests.json``. If a commit
step fails, already-replaced files are restored from their in-memory originals.

Usage:
  python3 tools/wpt/splice_text_port.py wpt/css2_floats/example [...]
  python3 tools/wpt/splice_text_port.py --dry-run wpt/css2_floats/example
"""

from __future__ import annotations

import csv
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

sys.path.insert(0, SCRIPT_DIR)
import port_wpt  # noqa: E402


@dataclass(frozen=True)
class GeneratedReplacement:
    test_id: str
    module: str
    fn_name: str
    rust_code: str
    template: str


def load_mapping_rows() -> dict[str, dict[str, str]]:
    """Load the one-to-one test-id mapping, rejecting ambiguous IDs."""
    rows: dict[str, dict[str, str]] = {}
    with open(MAPPING_CSV, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            test_id = row.get("our_test_id", "")
            if not test_id:
                continue
            if test_id in rows:
                raise ValueError(f"ambiguous mapping for {test_id}")
            rows[test_id] = row
    return rows


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

    parser = port_wpt.parse_wpt_html(upstream)
    portable, reason = port_wpt.analyze_portability(parser)
    if not portable:
        raise ValueError(f"{test_id}: not text-portable ({reason})")

    fn_name = f"{module}_{port_wpt.sanitize_fn_name(name)}"
    return GeneratedReplacement(
        test_id=test_id,
        module=module,
        fn_name=fn_name,
        rust_code=port_wpt.generate_rust_fn(
            fn_name, parser.root, parser.html_styles
        ),
        template=port_wpt.generate_html_template(upstream),
    )


def prepare_changes(
    test_ids: list[str], mapping: dict[str, dict[str, str]]
) -> tuple[list[GeneratedReplacement], dict[str, str], dict[str, str]]:
    """Generate and validate a complete transaction without writing files."""
    if len(test_ids) != len(set(test_ids)):
        raise ValueError("duplicate test IDs in one splice transaction")

    generated = [_generate_one(test_id, mapping) for test_id in test_ids]
    originals: dict[str, str] = {}
    changes: dict[str, str] = {}

    by_module: dict[str, list[GeneratedReplacement]] = defaultdict(list)
    for replacement in generated:
        by_module[replacement.module].append(replacement)

    for module, replacements in sorted(by_module.items()):
        rust_path = os.path.join(RUST_WPT_DIR, f"wpt_{module}.rs")
        with open(rust_path, encoding="utf-8") as f:
            original = f.read()
        updated = original
        for replacement in replacements:
            registry_pattern = re.compile(
                rf'\(\s*"{re.escape(replacement.test_id)}"\s*,\s*'
                rf'{re.escape(replacement.fn_name)}\s+as\s+fn\(\)\s*->\s*Document\s*,?\s*\)'
            )
            registry_matches = registry_pattern.findall(original)
            if len(registry_matches) != 1:
                raise ValueError(
                    f"{replacement.test_id}: expected exactly one registry identity, "
                    f"found {len(registry_matches)}"
                )
            updated = replace_fn(updated, replacement.fn_name, replacement.rust_code)
        originals[rust_path] = original
        changes[rust_path] = updated

    template_paths = {
        os.path.join(WPT_PORTED_DIR, "all_wpt_templates.json")
    }
    template_paths.update(
        os.path.join(WPT_PORTED_DIR, f"wpt_{module}_templates.json")
        for module in by_module
    )
    for path in sorted(template_paths):
        with open(path, encoding="utf-8") as f:
            original = f.read()
        templates = json.loads(original)
        if not isinstance(templates, dict):
            raise ValueError(f"template file is not an object: {path}")
        relevant = [
            r
            for r in generated
            if os.path.basename(path) == "all_wpt_templates.json"
            or os.path.basename(path) == f"wpt_{r.module}_templates.json"
        ]
        for replacement in relevant:
            if replacement.test_id not in templates:
                raise KeyError(
                    f"{replacement.test_id}: missing matching template in {path}"
                )
            templates[replacement.test_id] = replacement.template
        originals[path] = original
        changes[path] = json.dumps(templates, indent=2) + "\n"

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
    dry_run = "--dry-run" in args
    unknown_options = [a for a in args if a.startswith("--") and a != "--dry-run"]
    if unknown_options:
        print(f"ERROR: unknown options: {' '.join(unknown_options)}", file=sys.stderr)
        return 2
    test_ids = [a for a in args if not a.startswith("--")]
    if not test_ids:
        print(__doc__)
        return 2

    # Enable symmetric deterministic-text generation for the whole prepare
    # phase. The standalone batch porter remains box-only by default.
    port_wpt.EMIT_TEXT_NODES = True
    port_wpt.RETAIN_TEXT = True

    try:
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
