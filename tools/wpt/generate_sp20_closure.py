#!/usr/bin/env python3
"""Freeze, validate, and publish the SP20 static-visual residual closure.

SP20 starts from the completed SP19 snapshot: 4,962 exact runnable rows and
2,711 explicitly-owned unported rows.  Every unported row which is neither
JavaScript-dependent nor a nonvisual/crash harness is an SP20 target.

The inventory is rebuilt independently from the frozen CSV and upstream WPT
sources.  All output is validated in memory and installed transactionally.
``--check`` performs no writes.  Promotion only accepts a complete wave whose
pixel results are exact and rejects all non-target drift.
"""

from __future__ import annotations

import csv
import hashlib
import html
import io
import json
import os
import re
import sys
import tempfile
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DATA = ROOT / "tools" / "accountability" / "data"
PORTED = DATA / "wpt_ported"
LIVE_MAPPING = DATA / "wpt_mapping.csv"
LIVE_SUMMARY = DATA / "pixel_comparison" / "results" / "summary.json"
WPT_BASE = Path(os.environ.get(
    "CHROMIUM_WPT_ROOT",
    os.path.expanduser("~/chromium/src/third_party/blink/web_tests/external/wpt"),
))
WPT_CSS = WPT_BASE / "css"

KICKOFF_MAPPING = PORTED / "sp20_kickoff_mapping.csv"
KICKOFF_SUMMARY = PORTED / "sp20_sp19_summary.json"
EXACT_BASELINE = HERE / "sp20_exact_baseline.json"
TARGETS = HERE / "sp20_targets.json"
JAVASCRIPT_EXCLUSIONS = HERE / "sp20_javascript_exclusions.json"
NONVISUAL_EXCLUSIONS = HERE / "sp20_nonvisual_exclusions.json"
FOCUSED = HERE / "sp20_focused_ids.json"
PROJECTED_UNPORTED = PORTED / "sp20_projected_unported.json"
PARTITIONS = PORTED / "sp20_partitions.json"
SOURCE_INVENTORY = PORTED / "sp20_source_inventory.json"

EXPECTED_MAPPING_ROWS = 7673
EXPECTED_BASELINE = 4962
EXPECTED_TARGETS = 769
EXPECTED_JAVASCRIPT = 1912
EXPECTED_NONVISUAL = 30
EXPECTED_FOCUSED = 5731
EXPECTED_UNPORTED = 1942

KICKOFF_MAPPING_SHA256 = (
    "70f9b8f62b1ea2fd89e77ce778033797f7dbc8b5f63d01bc964a7b34901e5d16"
)
KICKOFF_SUMMARY_SHA256 = (
    "be3f936b795a66ec6b7b508b261b77150dd372681eda8d8f53f58908aa50cba8"
)
MANIFEST_SHA256 = {
    EXACT_BASELINE: "a6cae4790a998600052ef7fb40f5199e32ad0bbd7f8a22855814df0cf59d14af",
    TARGETS: "cb0046279493bb4587e0243422d4bd16e008af01ef88670c30e9289f2474676f",
    JAVASCRIPT_EXCLUSIONS: "2f12ed9d2b7db42f0813961aaf04d55bc91298b7083db2a3288e92da70a0da60",
    NONVISUAL_EXCLUSIONS: "b7a9f12cae53d5283ffa7dc095c668fb1c18e95a2b4fd901a901d77cdd5ec21c",
    FOCUSED: "2d68221f88c8280249b45c304338a9ab312918fd8c82d80ae1a46c0a26a8816f",
    PROJECTED_UNPORTED: "c3a94ede63f556a668f280d0c4c898282c67569d2e7f1d93c97436bdfdf5486b",
}

PARTITION_AREAS = {
    "w2_break_multicol": ("css_break", "css_multicol"),
    "w3_flexbox_sizing": ("css_flexbox", "css_sizing"),
    "w4_overflow_backgrounds": ("css_overflow", "css_backgrounds"),
    "w5_box_position_display_floats": (
        "css_box", "css_position", "css_display", "css2_floats",
    ),
}
PARTITION_COUNTS = {
    "w2_break_multicol": 191,
    "w3_flexbox_sizing": 326,
    "w4_overflow_backgrounds": 144,
    "w5_box_position_display_floats": 108,
}
PARTITION_SHA256 = {
    "w2_break_multicol": "2494376ab8c53efe11f0a9eb1eea3c6f8021abd6719624eb7e141d9e8842aa42",
    "w3_flexbox_sizing": "2c07f2f1787a0e783293eb513fd84e9d7e54a67c52bbe9ac66bcca6c7d43fdf3",
    "w4_overflow_backgrounds": "b7ac79d622df3513637322680d7f9c7a26132db3e529321341cf471068057980",
    "w5_box_position_display_floats": "4db4b4975bf538854597aa9f694deaddb86f5b92d0bf8c9ce1b1960aa164df03",
}

EXCLUDED_OWNERS = {"needs_javascript", "non_visual_test"}
HTML_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
EXECUTABLE_SCRIPT = re.compile(r"<script(?:\s|>)", re.IGNORECASE)
CSS_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)
CLASS_ADD = re.compile(
    r"\s*document\.body\.classList\.add\(\s*(['\"])([^'\"]+)\1\s*\)\s*;?\s*"
)


def encoded(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_id(row: dict[str, str]) -> str:
    area = row.get("sp_area", "").strip()
    name = row.get("test_name", "").strip()
    if not area or not name or "/" in area or "/" in name:
        raise ValueError(f"SP20 invalid mapping identity: {area!r}/{name!r}")
    result = f"wpt/{area}/{name}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != result:
        raise ValueError(f"SP20 mapping identity drift: {recorded!r} != {result!r}")
    return result


def categories(row: dict[str, str]) -> set[str]:
    return {
        value.strip()
        for value in row.get("failure_category", "").split(",")
        if value.strip()
    }


def parse_mapping(data: bytes) -> list[dict[str, str]]:
    rows = list(csv.DictReader(io.StringIO(data.decode("utf-8"), newline="")))
    if len(rows) != EXPECTED_MAPPING_ROWS:
        raise ValueError(f"SP20 mapping row count changed: {len(rows)}")
    ids = [canonical_id(row) for row in rows]
    if len(ids) != len(set(ids)):
        raise ValueError("SP20 mapping has duplicate canonical IDs")
    return rows


def exact_ids(summary: dict) -> set[str]:
    return {
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    }


def _within_wpt(path: Path) -> Path:
    resolved = path.resolve()
    try:
        resolved.relative_to(WPT_BASE.resolve())
    except ValueError as exc:
        raise ValueError(f"SP20 dependency escaped WPT root: {resolved}") from exc
    return resolved


def source_path(row: dict[str, str]) -> Path:
    value = row.get("chromium_test_path", "").strip()
    path = _within_wpt(WPT_CSS / value)
    if not path.is_file():
        raise FileNotFoundError(path)
    return path


class _SourceScanner(HTMLParser):
    """Collect linked CSS and event attributes without executing markup."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.stylesheets: list[str] = []
        self.handlers: list[tuple[str, str, str]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = {name.lower(): value or "" for name, value in attrs}
        if tag.lower() == "link" and "stylesheet" in values.get("rel", "").lower().split():
            self.stylesheets.append(values.get("href", ""))
        for name, value in attrs:
            if name.lower().startswith("on"):
                self.handlers.append((tag.lower(), name.lower(), html.unescape(value or "")))

    handle_startendtag = handle_starttag


def scan_source(path: Path) -> tuple[str, list[dict[str, str]], list[dict[str, str]]]:
    raw = path.read_bytes()
    text = raw.decode("utf-8", errors="ignore")
    scanner = _SourceScanner()
    scanner.feed(text)
    stylesheets = []
    for href in scanner.stylesheets:
        clean = href.split("?", 1)[0].split("#", 1)[0]
        if not clean or "://" in clean or clean.startswith("data:"):
            raise ValueError(f"SP20 non-local stylesheet in {path}: {href!r}")
        dependency = WPT_BASE / clean.lstrip("/") if clean.startswith("/") else path.parent / clean
        dependency = _within_wpt(dependency)
        if not dependency.is_file():
            # A WPT may intentionally exercise a failed stylesheet fetch (and
            # one historical flex test has a stale ``resources/`` URL).  Pin
            # that deterministic absence instead of substituting a resource.
            stylesheets.append({
                "path": dependency.relative_to(WPT_BASE).as_posix(),
                "sha256": "missing",
            })
            continue
        css = dependency.read_bytes()
        # Exercise comment removal while inventorying CSS; parser support uses
        # the same primitive and tests pin comments around declarations/rules.
        CSS_COMMENT.sub("", css.decode("utf-8", errors="ignore"))
        stylesheets.append({
            "path": dependency.relative_to(WPT_BASE).as_posix(),
            "sha256": digest(css),
        })
    handlers = [
        {"element": tag, "event": event, "source": value}
        for tag, event, value in scanner.handlers
    ]
    return digest(raw), sorted(stylesheets, key=lambda item: item["path"]), handlers


def lowered_startup_classes(handlers: list[dict[str, str]]) -> list[str]:
    """Validate and lower the narrowly deterministic startup handler subset."""
    result = []
    for handler in handlers:
        if handler["element"] != "body" or handler["event"] != "onload":
            raise ValueError(f"SP20 unsupported event handler: {handler}")
        match = CLASS_ADD.fullmatch(handler["source"])
        if not match:
            raise ValueError(f"SP20 unsupported body load handler: {handler['source']!r}")
        tokens = match.group(2).split()
        if not tokens or any(not re.fullmatch(r"-?[_a-zA-Z]+[_a-zA-Z0-9-]*", token) for token in tokens):
            raise ValueError(f"SP20 invalid literal class token: {match.group(2)!r}")
        result.extend(tokens)
    return result


def build_partitions(targets: set[str]) -> dict[str, list[str]]:
    partitions = {
        wave: sorted(
            test_id for test_id in targets if test_id.split("/", 2)[1] in areas
        )
        for wave, areas in PARTITION_AREAS.items()
    }
    if {key: len(value) for key, value in partitions.items()} != PARTITION_COUNTS:
        raise ValueError("SP20 partition counts changed")
    if set().union(*(set(value) for value in partitions.values())) != targets:
        raise ValueError("SP20 partitions do not exactly cover targets")
    for wave, values in partitions.items():
        if digest(encoded(values)) != PARTITION_SHA256[wave]:
            raise ValueError(f"SP20 partition hash changed: {wave}")
    return partitions


def validate_kickoff(rows: list[dict[str, str]], summary: dict) -> None:
    tests = summary.get("tests", [])
    statuses = Counter(item.get("status") for item in tests)
    if (
        len(tests), len(exact_ids(summary)), statuses["fail"], statuses["error"]
    ) != (EXPECTED_BASELINE, EXPECTED_BASELINE, 0, 0):
        raise ValueError("SP20 frozen SP19 summary totals changed")
    if sum(row.get("ported") == "yes" for row in rows) != EXPECTED_BASELINE:
        raise ValueError("SP20 frozen mapping runnable count changed")
    if {canonical_id(row) for row in rows if row.get("ported") == "yes"} != exact_ids(summary):
        raise ValueError("SP20 frozen mapping and exact summary IDs differ")


def build_outputs(mapping_bytes: bytes, summary_bytes: bytes) -> dict[Path, bytes]:
    if digest(mapping_bytes) != KICKOFF_MAPPING_SHA256:
        raise ValueError("SP20 kickoff mapping byte drift")
    if digest(summary_bytes) != KICKOFF_SUMMARY_SHA256:
        raise ValueError("SP20 kickoff summary byte drift")
    rows = parse_mapping(mapping_bytes)
    summary = json.loads(summary_bytes)
    validate_kickoff(rows, summary)

    baseline = exact_ids(summary)
    all_unported = {canonical_id(row) for row in rows if row.get("ported") == "no"}
    javascript = set()
    nonvisual = set()
    targets = set()
    inventory = []
    for row in rows:
        if row.get("ported") != "no":
            continue
        test_id = canonical_id(row)
        owners = categories(row)
        if not owners or "not_ported" in owners:
            raise ValueError(f"SP20 target has no explicit owner: {test_id}")
        if "needs_javascript" in owners:
            javascript.add(test_id)
            continue
        if "non_visual_test" in owners:
            nonvisual.add(test_id)
            continue
        targets.add(test_id)
        path = source_path(row)
        source_sha, stylesheets, handlers = scan_source(path)
        text = path.read_text(encoding="utf-8", errors="ignore")
        if EXECUTABLE_SCRIPT.search(HTML_COMMENT.sub("", text)):
            raise ValueError(f"SP20 target contains executable script: {test_id}")
        lowered = lowered_startup_classes(handlers)
        inventory.append({
            "test_id": test_id,
            "chromium_test_path": row["chromium_test_path"].strip(),
            "source_sha256": source_sha,
            "linked_stylesheets": stylesheets,
            "failure_categories": sorted(owners),
            "event_handlers": handlers,
            "lowered_body_classes": lowered,
        })

    projected = javascript | nonvisual
    focused = baseline | targets
    counts = (
        len(baseline), len(targets), len(javascript), len(nonvisual),
        len(focused), len(projected),
    )
    expected = (
        EXPECTED_BASELINE, EXPECTED_TARGETS, EXPECTED_JAVASCRIPT,
        EXPECTED_NONVISUAL, EXPECTED_FOCUSED, EXPECTED_UNPORTED,
    )
    if counts != expected:
        raise ValueError(f"SP20 inventory projection changed: {counts}")
    if javascript & nonvisual or targets & projected or baseline & all_unported:
        raise ValueError("SP20 inventory partitions overlap")
    if targets | projected != all_unported or focused | projected != {canonical_id(r) for r in rows}:
        raise ValueError("SP20 inventory partitions are incomplete")
    event_targets = [item for item in inventory if item["event_handlers"]]
    if len(event_targets) != 1 or event_targets[0]["lowered_body_classes"] != ["changed"]:
        raise ValueError("SP20 deterministic startup mutation inventory changed")

    partitions = build_partitions(targets)
    outputs = {
        KICKOFF_MAPPING: mapping_bytes,
        KICKOFF_SUMMARY: summary_bytes,
        EXACT_BASELINE: encoded(sorted(baseline)),
        TARGETS: encoded(sorted(targets)),
        JAVASCRIPT_EXCLUSIONS: encoded(sorted(javascript)),
        NONVISUAL_EXCLUSIONS: encoded(sorted(nonvisual)),
        FOCUSED: encoded(sorted(focused)),
        PROJECTED_UNPORTED: encoded(sorted(projected)),
        PARTITIONS: encoded(partitions),
        SOURCE_INVENTORY: encoded(sorted(inventory, key=lambda item: item["test_id"])),
    }
    for path, expected_hash in MANIFEST_SHA256.items():
        if digest(outputs[path]) != expected_hash:
            raise ValueError(f"SP20 manifest hash changed: {path.name}")
    return outputs


def load_partitions() -> dict[str, list[str]]:
    value = json.loads(PARTITIONS.read_text(encoding="utf-8"))
    if list(value) != list(PARTITION_COUNTS):
        raise ValueError("SP20 partition order changed")
    if {key: len(ids) for key, ids in value.items()} != PARTITION_COUNTS:
        raise ValueError("SP20 partition counts changed")
    for wave, ids in value.items():
        if ids != sorted(set(ids)) or digest(encoded(ids)) != PARTITION_SHA256[wave]:
            raise ValueError(f"SP20 partition content changed: {wave}")
    return value


def validate_live_snapshot(rows: list[dict[str, str]], summary: dict) -> str:
    baseline = set(json.loads(EXACT_BASELINE.read_text(encoding="utf-8")))
    targets = set(json.loads(TARGETS.read_text(encoding="utf-8")))
    projected = set(json.loads(PROJECTED_UNPORTED.read_text(encoding="utf-8")))
    partitions = load_partitions()
    tests = summary.get("tests", [])
    by_id = {item.get("id"): item for item in tests}
    if len(by_id) != len(tests):
        raise ValueError("SP20 live summary has duplicate IDs")
    exact = exact_ids(summary)
    if not baseline <= exact:
        raise ValueError("SP20 changed an SP19 exact result")
    if any(item.get("status") == "error" for item in tests):
        raise ValueError("SP20 live summary contains errors")

    steps: list[tuple[str, set[str]]] = [("w0", set())]
    prefix = set()
    for wave, ids in partitions.items():
        prefix |= set(ids)
        steps.append((wave, set(prefix)))
    current = targets & exact
    matches = [wave for wave, expected in steps if current == expected]
    if len(matches) != 1:
        raise ValueError("SP20 exact targets are not a complete wave prefix")
    wave = matches[0]
    expected_ids = baseline | current
    if set(by_id) != expected_ids or exact != expected_ids:
        raise ValueError("SP20 runnable results or non-target results drifted")
    if any(item.get("mismatch_pct") != 0.0 for item in tests):
        raise ValueError("SP20 pass is not zero-pixel exact")

    row_by_id = {canonical_id(row): row for row in rows}
    live_unported = {test_id for test_id, row in row_by_id.items() if row.get("ported") == "no"}
    if live_unported != projected | (targets - current):
        raise ValueError("SP20 mapping target/non-target state drifted")
    for test_id in baseline | current:
        row = row_by_id[test_id]
        if (
            row.get("ported") != "yes" or row.get("our_test_id") != test_id
            or row.get("pixel_result") != "pass" or row.get("mismatch_pct") != "0.0"
        ):
            raise ValueError(f"SP20 exact mapping row is stale: {test_id}")
    return wave


def _historical_bytes() -> tuple[bytes, bytes]:
    if KICKOFF_MAPPING.is_file() or KICKOFF_SUMMARY.is_file():
        if not KICKOFF_MAPPING.is_file() or not KICKOFF_SUMMARY.is_file():
            raise ValueError("SP20 kickoff snapshot is only partially present")
        return KICKOFF_MAPPING.read_bytes(), KICKOFF_SUMMARY.read_bytes()
    mapping = LIVE_MAPPING.read_bytes()
    summary = LIVE_SUMMARY.read_bytes()
    if digest(mapping) != KICKOFF_MAPPING_SHA256 or digest(summary) != KICKOFF_SUMMARY_SHA256:
        raise ValueError("SP20 bootstrap requires the exact completed SP19 snapshot")
    return mapping, summary


def write_transaction(outputs: dict[Path, bytes]) -> None:
    staged: dict[Path, Path] = {}
    originals: dict[Path, bytes | None] = {}
    try:
        for path, content in outputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
            staged[path] = Path(name)
            with os.fdopen(fd, "wb") as stream:
                stream.write(content)
                stream.flush()
                os.fsync(stream.fileno())
        for path in outputs:
            originals[path] = path.read_bytes() if path.exists() else None
        installed = []
        try:
            for path, temporary in staged.items():
                os.replace(temporary, path)
                installed.append(path)
        except Exception:
            for path in reversed(installed):
                previous = originals[path]
                if previous is None:
                    path.unlink(missing_ok=True)
                else:
                    fd, name = tempfile.mkstemp(prefix=f".{path.name}.rollback.", dir=path.parent)
                    with os.fdopen(fd, "wb") as stream:
                        stream.write(previous)
                    os.replace(name, path)
            raise
    finally:
        for path in staged.values():
            path.unlink(missing_ok=True)


def _encode_csv_row(fieldnames: list[str], row: dict[str, str]) -> str:
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=fieldnames, lineterminator="\n")
    writer.writerow(row)
    return output.getvalue()


def promoted_mapping_bytes(
    original: bytes, summary_by_id: dict[str, dict], targets: set[str],
) -> bytes:
    text = original.decode("utf-8")
    lines = text.splitlines(keepends=True)
    reader = csv.reader(lines)
    fieldnames = next(reader)
    record_start = reader.line_num
    output = ["".join(lines[:record_start])]
    seen = set()
    desired = {
        "ported": "yes", "pixel_result": "pass", "mismatch_pct": "0.0",
        "failure_category": "", "dependency": "", "notes": "",
    }
    for values in reader:
        record_end = reader.line_num
        original_record = "".join(lines[record_start:record_end])
        record_start = record_end
        row = dict(zip(fieldnames, values))
        test_id = canonical_id(row)
        if test_id not in targets:
            output.append(original_record)
            continue
        result = summary_by_id.get(test_id)
        if not result or result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"cannot promote non-exact SP20 target: {test_id}")
        desired["our_test_id"] = test_id
        if all(row.get(key) == value for key, value in desired.items()):
            output.append(original_record)
        else:
            row.update(desired)
            output.append(_encode_csv_row(fieldnames, row))
        seen.add(test_id)
    if seen != targets:
        raise ValueError("SP20 mapping splice did not find every target")
    return "".join(output).encode("utf-8")


def promote_wave(wave: str) -> None:
    partitions = load_partitions()
    if wave not in partitions:
        raise ValueError(f"unknown SP20 wave: {wave}")
    summary = json.loads(LIVE_SUMMARY.read_text(encoding="utf-8"))
    by_id = {item["id"]: item for item in summary.get("tests", [])}
    candidate = promoted_mapping_bytes(LIVE_MAPPING.read_bytes(), by_id, set(partitions[wave]))
    actual = validate_live_snapshot(parse_mapping(candidate), summary)
    if actual != wave:
        raise ValueError(f"SP20 promotion would produce {actual}, not {wave}")
    write_transaction({LIVE_MAPPING: candidate})


def check() -> str:
    mapping, summary = _historical_bytes()
    outputs = build_outputs(mapping, summary)
    for path, content in outputs.items():
        if not path.is_file() or path.read_bytes() != content:
            raise ValueError(f"SP20 generated artifact drift: {path}")
    return validate_live_snapshot(
        parse_mapping(LIVE_MAPPING.read_bytes()),
        json.loads(LIVE_SUMMARY.read_text(encoding="utf-8")),
    )


def main() -> int:
    args = sys.argv[1:]
    if args[:1] == ["--promote-wave"] and len(args) == 2:
        promote_wave(args[1])
        wave = check()
    elif args == ["--check"]:
        wave = check()
    elif not args:
        mapping, summary = _historical_bytes()
        write_transaction(build_outputs(mapping, summary))
        wave = check()
    else:
        print("Usage: generate_sp20_closure.py [--check|--promote-wave WAVE]", file=sys.stderr)
        return 2
    print(
        "SP20 closure: baseline=4962 targets=769 javascript=1912 "
        f"nonvisual=30 focused=5731 projected-unported=1942 wave={wave}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
