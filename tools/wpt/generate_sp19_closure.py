#!/usr/bin/env python3
"""Freeze and validate the SP19 layout-systems mega-sweep.

SP19 starts from the immutable SP18 4,139-row summary and mapping.  The
layout inventory is deliberately recomputed from the frozen mapping and the
upstream sources instead of importing the accountability detectors: this
keeps the sprint boundary independent from later detector changes.

Generation is a transaction.  Every output is built and validated in memory,
then staged beside its destination and atomically installed.  ``--check`` is
strictly read-only and also validates the current live wave projection.
"""

from __future__ import annotations

import csv
import hashlib
import io
import json
import os
import re
import sys
import tempfile
from collections import Counter
from pathlib import Path

from generate_sp13r_multicol_closure import lowered_candidate_promotions
from generate_sp20_closure import MANIFEST_SHA256 as SP20_MANIFEST_SHA256
from generate_sp20_closure import TARGETS as SP20_TARGETS


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DATA = ROOT / "tools" / "accountability" / "data"
PORTED = DATA / "wpt_ported"
LIVE_MAPPING = DATA / "wpt_mapping.csv"
LIVE_SUMMARY = DATA / "pixel_comparison" / "results" / "summary.json"
WPT_ROOT = Path(os.environ.get(
    "CHROMIUM_WPT_CSS",
    os.path.expanduser(
        "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
    ),
))

KICKOFF_MAPPING = PORTED / "sp19_kickoff_mapping.csv"
KICKOFF_SUMMARY = PORTED / "sp19_sp18_summary.json"
INITIAL_MISMATCHES = PORTED / "sp19_initial_mismatches.json"
REPAIR_TARGETS = HERE / "sp18_runnable_residuals.json"
REPAIRED_BASELINE = HERE / "sp19_repaired_baseline.json"
LAYOUT_TARGETS = HERE / "sp19_layout_targets.json"
COMBINED_TARGETS = HERE / "sp19_targets.json"
FOCUSED = HERE / "sp19_focused_ids.json"
JAVASCRIPT_EXCLUSIONS = HERE / "sp19_javascript_exclusions.json"
PROJECTED_UNPORTED = PORTED / "sp19_projected_unported.json"
REPAIR_PARTITIONS = PORTED / "sp19_repair_partitions.json"
LAYOUT_PARTITIONS = PORTED / "sp19_layout_partitions.json"
SYNTAX_INVENTORY = PORTED / "sp19_layout_syntax_inventory.json"

EXPECTED_MAPPING_ROWS = 7673
EXPECTED_SP18_RUNNABLE = 4139
EXPECTED_SP18_EXACT = 4058
EXPECTED_REPAIRS = 81
EXPECTED_LAYOUT_TARGETS = 823
EXPECTED_JAVASCRIPT_EXCLUSIONS = 221
EXPECTED_COMBINED_TARGETS = 904
EXPECTED_FINAL_FOCUSED = 4962
EXPECTED_FINAL_UNPORTED = 2711

SP18_SUMMARY_SHA256 = (
    "dec8f5c330c94fab200afcb3f82da6af2888695284284b36d8906f9fc939aa7e"
)
SP18_MAPPING_SHA256 = (
    "7a89501a5473b5e16bcc60372ac685c1643f313724a0bc80c5b181b83fc61be1"
)
MANIFEST_SHA256 = {
    REPAIR_TARGETS: "1b9e7d88819931c0f0c67100f799cd95ec3a5ab9ea177fae4c29b37ceb4aff71",
    REPAIRED_BASELINE: "b8392afabf69482e051efb41ebdbb7a4d37b17909bfc2b41f45c540764608b8a",
    LAYOUT_TARGETS: "9d501844c6a2f254785268d81142e0402339565ea9c400588be2f06c426c004d",
    COMBINED_TARGETS: "00e2e6012aadffeaf96cdbc7d6bb2d66c6ae7929a5e35e5ebaf59a9cdff84b91",
    FOCUSED: "a6cae4790a998600052ef7fb40f5199e32ad0bbd7f8a22855814df0cf59d14af",
    JAVASCRIPT_EXCLUSIONS: "cdb888275b65341410f7442242358c85ac832bd14e8162d8ab28d8e01fa883c6",
    PROJECTED_UNPORTED: "2e17c0c2f582b6181479f25062762fbe434911db2d117a21dd76b43617e406f5",
}
INITIAL_MISMATCHES_SHA256 = (
    "b2acd93596824a998440805360c1a4e38dbaca5b2b14fdb7e773ef1387f04103"
)
DERIVED_LEDGER_SHA256 = {
    REPAIR_PARTITIONS: "ce3cb7d09759e80bc4d50958f515fd683f7197b07e7e963336f9443a398fa765",
    LAYOUT_PARTITIONS: "e08e3f652875d17eb7d156372c0be93011dbba9854b6be5c2cc3d8b911649079",
    SYNTAX_INVENTORY: "b92ee04c14a7c5d418ea7301e3a5fc6d8e86c23fac9a90d723d04fc008faee0f",
}

FEATURE_CATEGORY = {
    "table": "needs_table_layout",
    "grid": "needs_grid",
    "containment": "needs_containment",
}

# These expressions intentionally do not import shared_detectors.py.  Linked
# local stylesheets are included by source_text() below.
TABLE_SYNTAX = re.compile(
    r"<(?:table|thead|tbody|tfoot|tr|td|th|caption|col|colgroup)[\s>]|"
    r"display\s*:\s*(?:inline-)?table(?:-[a-z-]+)?\b|"
    r"(?:table-layout|border-collapse|border-spacing|caption-side)\s*:",
    re.IGNORECASE,
)
GRID_SYNTAX = re.compile(
    r"display\s*:\s*(?:inline-)?grid\b|"
    r"(?:grid|grid-template|grid-auto|grid-column|grid-row|grid-area|"
    r"grid-gap|place-items|place-content|justify-items)"
    r"(?:-[a-z-]+)?\s*:",
    re.IGNORECASE,
)
CONTAINMENT_SYNTAX = re.compile(
    r"(?<![a-zA-Z-])contain\s*:|content-visibility\s*:|"
    r"contain-intrinsic-(?:size|width|height|block-size|inline-size)\s*:|"
    r"container(?:-type|-name)?\s*:|@container\b",
    re.IGNORECASE,
)
LINKED_STYLESHEET = re.compile(
    r"<link\b(?=[^>]*\brel\s*=\s*['\"]?stylesheet['\"]?)"
    r"(?=[^>]*\bhref\s*=\s*(['\"])(.*?)\1)[^>]*>",
    re.IGNORECASE,
)
HTML_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
EXECUTABLE_SCRIPT = re.compile(r"<script[\s>]", re.IGNORECASE)

REPAIR_COUNTS = {
    "w1a_float_inline": 33,
    "w1b_flex_sizing": 28,
    "w1c_fragmentation": 10,
    "w1d_position_paint_display": 10,
}
LAYOUT_COUNTS = {
    "w3_table_only": 341,
    "w4_grid_only": 208,
    "w5_containment_only": 231,
    "w6_intersections": 43,
}


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_id(row: dict[str, str]) -> str:
    test_id = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != test_id:
        raise ValueError(f"mapping identity drift: {recorded!r} != {test_id!r}")
    return test_id


def parse_mapping(data: bytes) -> list[dict[str, str]]:
    rows = list(csv.DictReader(io.StringIO(data.decode("utf-8"), newline="")))
    if len(rows) != EXPECTED_MAPPING_ROWS:
        raise ValueError(f"SP19 mapping row count changed: {len(rows)}")
    ids = [canonical_id(row) for row in rows]
    if len(ids) != len(set(ids)):
        raise ValueError("SP19 mapping contains duplicate canonical IDs")
    return rows


def exact_ids(summary: dict) -> set[str]:
    return {
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    }


def failure_categories(row: dict[str, str]) -> set[str]:
    return {
        value.strip()
        for value in row.get("failure_category", "").split(",")
        if value.strip()
    }


def source_text(row: dict[str, str]) -> str:
    """Read an upstream HTML file plus deterministic local CSS dependencies."""
    path = WPT_ROOT / row["chromium_test_path"].strip()
    if not path.is_file():
        raise FileNotFoundError(path)
    html = path.read_text(encoding="utf-8", errors="ignore")
    chunks = [html]
    for match in LINKED_STYLESHEET.finditer(html):
        href = match.group(2)
        if not href or "://" in href or href.startswith("/"):
            continue
        css_path = (path.parent / href.split("?", 1)[0].split("#", 1)[0]).resolve()
        try:
            css_path.relative_to(WPT_ROOT.resolve())
        except ValueError as exc:
            raise ValueError(f"SP19 stylesheet escaped WPT root: {css_path}") from exc
        if css_path.is_file():
            chunks.append(css_path.read_text(encoding="utf-8", errors="ignore"))
    return "\n".join(chunks)


def syntax_features(text: str) -> set[str]:
    result = set()
    if TABLE_SYNTAX.search(text):
        result.add("table")
    if GRID_SYNTAX.search(text):
        result.add("grid")
    if CONTAINMENT_SYNTAX.search(text):
        result.add("containment")
    return result


def has_executable_script(text: str) -> bool:
    return EXECUTABLE_SCRIPT.search(HTML_COMMENT.sub("", text)) is not None


def mapped_features(row: dict[str, str]) -> set[str]:
    categories = failure_categories(row)
    return {
        feature
        for feature, category in FEATURE_CATEGORY.items()
        if category in categories
    }


def build_repair_partitions(repairs: set[str]) -> dict[str, list[str]]:
    partitions = {
        "w1a_float_inline": sorted(
            test_id for test_id in repairs if test_id.startswith("wpt/css2_floats/")
        ),
        "w1b_flex_sizing": sorted(
            test_id
            for test_id in repairs
            if test_id.startswith(("wpt/css_flexbox/", "wpt/css_sizing/"))
        ),
        "w1c_fragmentation": sorted(
            test_id for test_id in repairs if test_id.startswith("wpt/css_break/")
        ),
        "w1d_position_paint_display": sorted(
            test_id
            for test_id in repairs
            if test_id.startswith((
                "wpt/css_position/", "wpt/css_overflow/",
                "wpt/css_display/", "wpt/css_backgrounds/",
            ))
        ),
    }
    if {key: len(value) for key, value in partitions.items()} != REPAIR_COUNTS:
        raise ValueError("SP19 repair partition counts changed")
    if set().union(*(set(value) for value in partitions.values())) != repairs:
        raise ValueError("SP19 repair partitions do not exactly cover repair targets")
    return partitions


def build_layout_partitions(
    targets: set[str], features_by_id: dict[str, set[str]],
) -> dict[str, list[str]]:
    partitions = {
        "w3_table_only": sorted(
            test_id for test_id in targets if features_by_id[test_id] == {"table"}
        ),
        "w4_grid_only": sorted(
            test_id for test_id in targets if features_by_id[test_id] == {"grid"}
        ),
        "w5_containment_only": sorted(
            test_id
            for test_id in targets
            if features_by_id[test_id] == {"containment"}
        ),
        "w6_intersections": sorted(
            test_id for test_id in targets if len(features_by_id[test_id]) > 1
        ),
    }
    if {key: len(value) for key, value in partitions.items()} != LAYOUT_COUNTS:
        raise ValueError("SP19 layout partition counts changed")
    if set().union(*(set(value) for value in partitions.values())) != targets:
        raise ValueError("SP19 layout partitions do not exactly cover layout targets")
    return partitions


def validate_kickoff(summary: dict, rows: list[dict[str, str]]) -> None:
    tests = summary.get("tests", [])
    status = Counter(item.get("status") for item in tests)
    if (
        len(tests), len(exact_ids(summary)), status["fail"], status["error"]
    ) != (EXPECTED_SP18_RUNNABLE, EXPECTED_SP18_EXACT, EXPECTED_REPAIRS, 0):
        raise ValueError("SP19 frozen SP18 summary totals changed")
    if sum(row["ported"] == "yes" for row in rows) != EXPECTED_SP18_RUNNABLE:
        raise ValueError("SP19 frozen mapping runnable count changed")


def build_outputs(
    kickoff_mapping_bytes: bytes, kickoff_summary_bytes: bytes,
) -> dict[Path, bytes]:
    if digest(kickoff_mapping_bytes) != SP18_MAPPING_SHA256:
        raise ValueError("SP19 kickoff mapping byte drift")
    if digest(kickoff_summary_bytes) != SP18_SUMMARY_SHA256:
        raise ValueError("SP19 SP18 summary byte drift")
    rows = parse_mapping(kickoff_mapping_bytes)
    summary = json.loads(kickoff_summary_bytes)
    validate_kickoff(summary, rows)

    repair_targets = set(json.loads(REPAIR_TARGETS.read_text(encoding="utf-8")))
    if len(repair_targets) != EXPECTED_REPAIRS:
        raise ValueError("SP19 repair target count changed")
    summary_by_id = {item["id"]: item for item in summary["tests"]}
    failures = {item["id"] for item in summary["tests"] if item["status"] == "fail"}
    if repair_targets != failures:
        raise ValueError("SP19 repair manifest is not the initial failure set")

    syntax_records = []
    mapped_by_id: dict[str, set[str]] = {}
    script_ids = set()
    all_unported = set()
    for row in rows:
        test_id = canonical_id(row)
        if row["ported"] == "no":
            all_unported.add(test_id)
        features = mapped_features(row)
        if row["ported"] != "no" or not features:
            continue
        text = source_text(row)
        syntax = syntax_features(text)
        scripted = has_executable_script(text)
        # Most ownership is direct syntax.  A small, frozen set is admitted by
        # the porter's first rejection (for example the ``container`` token in
        # a new pseudo/property); keep both facts visible in the evidence.
        if not syntax and not row.get("notes", "").startswith("Porter deferred:"):
            raise ValueError(f"SP19 owner has no upstream evidence: {test_id}")
        mapped_by_id[test_id] = features
        if scripted:
            script_ids.add(test_id)
        syntax_records.append({
            "test_id": test_id,
            "chromium_test_path": row["chromium_test_path"].strip(),
            "mapped_features": sorted(features),
            "syntax_features": sorted(syntax),
            "executable_script": scripted,
            "initial_failure_categories": sorted(failure_categories(row)),
            "initial_rejection": row["notes"].strip(),
        })
    syntax_records.sort(key=lambda item: item["test_id"])

    inventory = set(mapped_by_id)
    layout_targets = inventory - script_ids
    javascript_exclusions = inventory & script_ids
    repaired_baseline = exact_ids(summary) | repair_targets
    combined_targets = repair_targets | layout_targets
    focused = repaired_baseline | layout_targets
    projected_unported = all_unported - layout_targets
    expected_sizes = (
        len(layout_targets), len(javascript_exclusions), len(combined_targets),
        len(focused), len(projected_unported),
    )
    if expected_sizes != (
        EXPECTED_LAYOUT_TARGETS, EXPECTED_JAVASCRIPT_EXCLUSIONS,
        EXPECTED_COMBINED_TARGETS, EXPECTED_FINAL_FOCUSED,
        EXPECTED_FINAL_UNPORTED,
    ):
        raise ValueError(f"SP19 inventory projection changed: {expected_sizes}")

    repair_partitions = build_repair_partitions(repair_targets)
    layout_partitions = build_layout_partitions(layout_targets, mapped_by_id)
    mismatches = [
        {
            "test_id": test_id,
            "status": summary_by_id[test_id]["status"],
            "mismatch_pct": summary_by_id[test_id]["mismatch_pct"],
        }
        for test_id in sorted(repair_targets)
    ]

    outputs = {
        KICKOFF_MAPPING: kickoff_mapping_bytes,
        KICKOFF_SUMMARY: kickoff_summary_bytes,
        INITIAL_MISMATCHES: encoded(mismatches).encode("utf-8"),
        REPAIRED_BASELINE: encoded(sorted(repaired_baseline)).encode("utf-8"),
        LAYOUT_TARGETS: encoded(sorted(layout_targets)).encode("utf-8"),
        COMBINED_TARGETS: encoded(sorted(combined_targets)).encode("utf-8"),
        FOCUSED: encoded(sorted(focused)).encode("utf-8"),
        JAVASCRIPT_EXCLUSIONS: encoded(sorted(javascript_exclusions)).encode("utf-8"),
        PROJECTED_UNPORTED: encoded(sorted(projected_unported)).encode("utf-8"),
        REPAIR_PARTITIONS: encoded(repair_partitions).encode("utf-8"),
        LAYOUT_PARTITIONS: encoded(layout_partitions).encode("utf-8"),
        SYNTAX_INVENTORY: encoded(syntax_records).encode("utf-8"),
    }
    for path, expected_hash in MANIFEST_SHA256.items():
        content = outputs[path] if path in outputs else path.read_bytes()
        if digest(content) != expected_hash:
            raise ValueError(f"SP19 {path.name} projection hash changed")
    if digest(outputs[INITIAL_MISMATCHES]) != INITIAL_MISMATCHES_SHA256:
        raise ValueError("SP19 initial mismatch evidence changed")
    for path, expected_hash in DERIVED_LEDGER_SHA256.items():
        if digest(outputs[path]) != expected_hash:
            raise ValueError(f"SP19 {path.name} byte projection changed")
    return outputs


def load_partitions(path: Path, expected: dict[str, int]) -> dict[str, list[str]]:
    values = json.loads(path.read_text(encoding="utf-8"))
    if list(values) != list(expected):
        raise ValueError(f"SP19 partition order changed: {path.name}")
    if {key: len(value) for key, value in values.items()} != expected:
        raise ValueError(f"SP19 partition counts changed: {path.name}")
    for value in values.values():
        if value != sorted(set(value)):
            raise ValueError(f"SP19 partition is not sorted and unique: {path.name}")
    return values


def validate_live_snapshot(rows: list[dict[str, str]], summary: dict) -> str:
    """Accept only complete SP19 wave prefixes and reject non-target drift."""
    repaired = set(json.loads(REPAIRED_BASELINE.read_text(encoding="utf-8")))
    repair_targets = set(json.loads(REPAIR_TARGETS.read_text(encoding="utf-8")))
    layout_targets = set(json.loads(LAYOUT_TARGETS.read_text(encoding="utf-8")))
    projected = set(json.loads(PROJECTED_UNPORTED.read_text(encoding="utf-8")))
    if hashlib.sha256(SP20_TARGETS.read_bytes()).hexdigest() != SP20_MANIFEST_SHA256[SP20_TARGETS]:
        raise ValueError("SP20 later-promotion manifest hash changed")
    sp20_targets = set(json.loads(SP20_TARGETS.read_text(encoding="utf-8")))
    repair_parts = load_partitions(REPAIR_PARTITIONS, REPAIR_COUNTS)
    layout_parts = load_partitions(LAYOUT_PARTITIONS, LAYOUT_COUNTS)

    tests = summary.get("tests", [])
    by_id = {item.get("id"): item for item in tests}
    if len(by_id) != len(tests):
        raise ValueError("SP19 live summary has duplicate IDs")
    exact = exact_ids(summary)
    initial_exact = repaired - repair_targets
    if not initial_exact <= exact:
        raise ValueError("SP19 changed an SP18 exact non-target result")

    wave_steps: list[tuple[str, set[str], set[str]]] = [("w0", set(), set())]
    repaired_prefix = set()
    for name, ids in repair_parts.items():
        repaired_prefix |= set(ids)
        wave_steps.append((name, set(repaired_prefix), set()))
    layout_prefix = set()
    for name, ids in layout_parts.items():
        layout_prefix |= set(ids)
        wave_steps.append((name, set(repair_targets), set(layout_prefix)))

    current_repaired = repair_targets & exact
    current_layout = layout_targets & exact
    later_exact = sp20_targets & exact
    if later_exact not in (set(), sp20_targets):
        raise ValueError("SP20 later exact promotions are incomplete")
    matches = [
        name for name, expected_repaired, expected_layout in wave_steps
        if current_repaired == expected_repaired and current_layout == expected_layout
    ]
    if len(matches) != 1:
        raise ValueError("SP19 live exact targets are not a complete wave prefix")
    wave = matches[0]
    expected_ids = repaired | current_layout | later_exact
    if set(by_id) != expected_ids:
        raise ValueError("SP19 runnable expansion or non-target summary IDs drifted")
    if exact != initial_exact | current_repaired | current_layout | later_exact:
        raise ValueError("SP19 live exact set contains target or non-target drift")
    failures = set(by_id) - exact
    if failures != repair_targets - current_repaired:
        raise ValueError("SP19 live failure set changed outside the repair prefix")
    if any(item.get("status") == "error" for item in tests):
        raise ValueError("SP19 live summary contains errors")

    row_by_id = {canonical_id(row): row for row in rows}
    candidate_promotions = lowered_candidate_promotions(row_by_id, require_complete=False)
    if candidate_promotions & later_exact:
        raise ValueError("AST candidates overlap SP20 static promotions")
    live_unported = {
        test_id for test_id, row in row_by_id.items() if row["ported"] == "no"
    }
    if live_unported != (projected | (layout_targets - current_layout)) - later_exact - candidate_promotions:
        raise ValueError("SP19 mapping target/non-target runnable state drifted")
    for test_id in current_layout | repair_targets:
        row = row_by_id[test_id]
        item = by_id[test_id]
        if row["ported"] != "yes":
            raise ValueError(f"SP19 runnable target is not ported: {test_id}")
        if test_id in exact and (
            row["pixel_result"] != "pass" or row["mismatch_pct"] != "0.0"
        ):
            raise ValueError(f"SP19 exact target mapping is stale: {test_id}")
        if item["status"] == "pass" and item.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP19 pass is not zero-pixel exact: {test_id}")
    return wave


def _historical_bytes() -> tuple[bytes, bytes]:
    if KICKOFF_MAPPING.is_file() or KICKOFF_SUMMARY.is_file():
        if not (KICKOFF_MAPPING.is_file() and KICKOFF_SUMMARY.is_file()):
            raise ValueError("SP19 kickoff snapshot is only partially present")
        return KICKOFF_MAPPING.read_bytes(), KICKOFF_SUMMARY.read_bytes()
    mapping = LIVE_MAPPING.read_bytes()
    summary = LIVE_SUMMARY.read_bytes()
    if digest(mapping) != SP18_MAPPING_SHA256 or digest(summary) != SP18_SUMMARY_SHA256:
        raise ValueError("SP19 bootstrap requires the exact SP18 mapping and summary")
    return mapping, summary


def write_transaction(outputs: dict[Path, bytes]) -> None:
    staged: dict[Path, Path] = {}
    backups: dict[Path, bytes | None] = {}
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
            backups[path] = path.read_bytes() if path.exists() else None
        installed = []
        try:
            for path, staged_path in staged.items():
                os.replace(staged_path, path)
                installed.append(path)
        except Exception:
            for path in reversed(installed):
                old = backups[path]
                if old is None:
                    path.unlink(missing_ok=True)
                else:
                    fd, name = tempfile.mkstemp(prefix=f".{path.name}.rollback.", dir=path.parent)
                    with os.fdopen(fd, "wb") as stream:
                        stream.write(old)
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
    """Promote exact targets while retaining every non-target record byte."""
    text = original.decode("utf-8")
    lines = text.splitlines(keepends=True)
    reader = csv.reader(lines)
    fieldnames = next(reader)
    record_start = reader.line_num
    updated = ["".join(lines[:record_start])]
    seen: set[str] = set()
    desired = {
        "ported": "yes",
        "pixel_result": "pass",
        "mismatch_pct": "0.0",
        "failure_category": "",
        "dependency": "",
        "notes": "",
    }
    for values in reader:
        record_end = reader.line_num
        original_record = "".join(lines[record_start:record_end])
        record_start = record_end
        row = dict(zip(fieldnames, values))
        test_id = canonical_id(row)
        if test_id not in targets:
            updated.append(original_record)
            continue
        result = summary_by_id.get(test_id)
        if (
            not result
            or result.get("status") != "pass"
            or result.get("mismatch_pct") != 0.0
        ):
            raise ValueError(f"cannot promote non-exact SP19 target: {test_id}")
        desired["our_test_id"] = test_id
        if all(row.get(key) == value for key, value in desired.items()):
            updated.append(original_record)
        else:
            row.update(desired)
            updated.append(_encode_csv_row(fieldnames, row))
        seen.add(test_id)
    if seen != targets:
        raise ValueError("SP19 mapping splice did not find every wave target")
    return "".join(updated).encode("utf-8")


def promote_wave(wave: str) -> None:
    """Atomically publish one complete, exact layout wave to the live mapping."""
    partitions = load_partitions(LAYOUT_PARTITIONS, LAYOUT_COUNTS)
    if wave not in partitions:
        raise ValueError(f"unknown SP19 layout wave: {wave}")
    summary = json.loads(LIVE_SUMMARY.read_text(encoding="utf-8"))
    summary_by_id = {item["id"]: item for item in summary.get("tests", [])}
    if len(summary_by_id) != len(summary.get("tests", [])):
        raise ValueError("SP19 live summary has duplicate IDs")
    candidate = promoted_mapping_bytes(
        LIVE_MAPPING.read_bytes(), summary_by_id, set(partitions[wave]),
    )
    candidate_rows = parse_mapping(candidate)
    actual_wave = validate_live_snapshot(candidate_rows, summary)
    if actual_wave != wave:
        raise ValueError(
            f"SP19 promotion would produce {actual_wave}, not requested {wave}"
        )
    write_transaction({LIVE_MAPPING: candidate})


def check() -> str:
    kickoff_mapping, kickoff_summary = _historical_bytes()
    outputs = build_outputs(kickoff_mapping, kickoff_summary)
    for path, content in outputs.items():
        if not path.is_file() or path.read_bytes() != content:
            raise ValueError(f"SP19 generated artifact drift: {path}")
    live_rows = parse_mapping(LIVE_MAPPING.read_bytes())
    live_summary = json.loads(LIVE_SUMMARY.read_text(encoding="utf-8"))
    return validate_live_snapshot(live_rows, live_summary)


def main() -> int:
    args = sys.argv[1:]
    if args[:1] == ["--promote-wave"] and len(args) == 2:
        promote_wave(args[1])
        wave = check()
    elif args == ["--check"]:
        wave = check()
    elif not args:
        kickoff_mapping, kickoff_summary = _historical_bytes()
        outputs = build_outputs(kickoff_mapping, kickoff_summary)
        write_transaction(outputs)
        wave = check()
    else:
        print(
            "Usage: generate_sp19_closure.py "
            "[--check|--promote-wave WAVE]",
            file=sys.stderr,
        )
        return 2
    print(
        "SP19 closure: repairs=81 layout=823 combined=904 "
        f"focused=4962 javascript-exclusions=221 projected-unported=2711 wave={wave}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
