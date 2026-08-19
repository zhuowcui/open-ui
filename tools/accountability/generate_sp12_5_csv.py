#!/usr/bin/env python3
"""Generate SP12.5 deferred test artifacts.

Classifies failing WPT tests that depend on features from other SPs and
produces:
  1. data/sp12_5_deferred.csv — per-test dependency data
  2. docs/SP12.5-PLAN.md       — human-readable plan document
"""

import csv
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
TEMPLATES_PATH = SCRIPT_DIR / "data" / "wpt_ported" / "all_wpt_templates.json"
TEXT_PORTED_PATH = SCRIPT_DIR / "data" / "wpt_ported" / "text_ported_tests.json"
SUMMARY_PATH = SCRIPT_DIR / "data" / "pixel_comparison" / "results" / "summary.json"
WPT_MAPPING_PATH = SCRIPT_DIR / "data" / "wpt_mapping.csv"
CSV_OUTPUT = SCRIPT_DIR / "data" / "sp12_5_deferred.csv"
MD_OUTPUT = REPO_ROOT / "docs" / "SP12.5-PLAN.md"
CHROMIUM_WPT_BASE = Path(
    os.environ.get(
        "CHROMIUM_WPT_CSS",
        os.path.expanduser(
            "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
        ),
    )
)

# Import shared detectors (single source of truth)
sys.path.insert(0, str(SCRIPT_DIR))
from shared_detectors import classify_dependencies, DEPENDENCY_DEFS

def priority_for_count(count: int) -> str:
    if count > 100:
        return "high"
    if count >= 20:
        return "medium"
    return "low"


def load_text_port_upstream_paths(text_ported_tests: set[str]) -> dict[str, str]:
    """Resolve text ports to the original Chromium source used for ownership."""
    if not text_ported_tests:
        return {}
    if not WPT_MAPPING_PATH.exists():
        raise FileNotFoundError(
            f"missing WPT mapping required for text-port ownership: {WPT_MAPPING_PATH}"
        )

    result: dict[str, str] = {}
    with open(WPT_MAPPING_PATH, newline="") as f:
        for row in csv.DictReader(f):
            test_id = row.get("our_test_id", "")
            if test_id in text_ported_tests:
                result[test_id] = row.get("chromium_test_path", "")

    missing = text_ported_tests - set(result)
    if missing:
        raise ValueError(
            "text ports missing from WPT mapping: " + ", ".join(sorted(missing))
        )
    return result


def classification_html(
    test_id: str,
    template_html: str,
    text_ported_tests: set[str],
    upstream_paths: dict[str, str],
) -> str:
    """Use upstream HTML once normalized text is an implemented capability."""
    if test_id not in text_ported_tests:
        return template_html

    relative_path = upstream_paths.get(test_id, "")
    upstream_path = CHROMIUM_WPT_BASE / relative_path
    if not relative_path or not upstream_path.is_file():
        raise FileNotFoundError(
            f"missing upstream source for text-port ownership: {test_id}: "
            f"{upstream_path}"
        )
    return upstream_path.read_text(encoding="utf-8", errors="ignore")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    # Load data
    with open(TEMPLATES_PATH) as f:
        templates = json.load(f)
    with open(SUMMARY_PATH) as f:
        summary = json.load(f)
    text_ported_tests: set[str] = set()
    if TEXT_PORTED_PATH.exists():
        with open(TEXT_PORTED_PATH) as f:
            text_ported_data = json.load(f)
        if (
            not isinstance(text_ported_data, list)
            or any(not isinstance(test_id, str) or not test_id for test_id in text_ported_data)
            or len(text_ported_data) != len(set(text_ported_data))
        ):
            raise ValueError(f"invalid text-port manifest: {TEXT_PORTED_PATH}")
        text_ported_tests = set(text_ported_data)
        missing_templates = text_ported_tests - set(templates)
        if missing_templates:
            raise ValueError(
                "text-port manifest contains tests without templates: "
                + ", ".join(sorted(missing_templates))
            )
    text_port_upstream_paths = load_text_port_upstream_paths(text_ported_tests)

    # Build lookup: id -> test record (failures + errors are both deferrable)
    fail_map: dict[str, dict] = {}
    for t in summary["tests"]:
        if t["status"] in ("fail", "error"):
            fail_map[t["id"]] = t

    # Classify every failing test
    rows: list[dict] = []
    dep_counts: dict[str, int] = defaultdict(int)
    dep_tests: dict[str, list[str]] = defaultdict(list)

    for test_id, template_html in templates.items():
        if test_id not in fail_map:
            continue
        html = classification_html(
            test_id,
            template_html,
            text_ported_tests,
            text_port_upstream_paths,
        )
        deps = classify_dependencies(
            html,
            test_id=test_id,
            excluded={"text_rendering"} if test_id in text_ported_tests else None,
        )
        if not deps:
            continue
        for d in deps:
            dep_counts[d] += 1
            dep_tests[d].append(test_id)

        owning_sps = set()
        for key, _label, sp, _det in DEPENDENCY_DEFS:
            if key in deps:
                for s in sp.split(","):
                    owning_sps.add(s)

        rows.append({
            "test_id": test_id,
            "sp_area": "SP12",
            "dependency": ",".join(deps),
            "owning_sp": ",".join(sorted(owning_sps)),
            "mismatch_pct": fail_map[test_id]["mismatch_pct"],
            "priority": "",  # filled below
            "notes": "",
        })

    # Compute priority per-row based on the highest-count dependency
    for row in rows:
        deps = row["dependency"].split(",")
        max_count = max(dep_counts[d] for d in deps)
        row["priority"] = priority_for_count(max_count)

    # Sort by test_id for deterministic output
    rows.sort(key=lambda r: r["test_id"])

    # --- Write CSV ---
    CSV_OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with open(CSV_OUTPUT, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=["test_id", "sp_area", "dependency", "owning_sp",
                         "mismatch_pct", "priority", "notes"],
            lineterminator="\n",
        )
        writer.writeheader()
        writer.writerows(rows)
    print(f"CSV written: {CSV_OUTPUT}  ({len(rows)} tests)")

    # --- Write Markdown ---
    total_deferred = len(rows)
    dep_label = {key: label for key, label, _sp, _det in DEPENDENCY_DEFS}
    dep_sp = {key: sp for key, _label, sp, _det in DEPENDENCY_DEFS}

    lines: list[str] = []
    lines.append("# SP12.5 — Cross-SP Dependency Tests\n")
    lines.append("## Overview\n")
    lines.append(
        f"{total_deferred} WPT tests from SP12 scope that require features from other SPs.\n"
        "These tests are tracked but cannot pass until their dependencies are implemented.\n"
    )

    # Dependency Summary Table
    lines.append("## Dependency Summary Table\n")
    lines.append("| Dependency | Count | Owning SP | Priority |")
    lines.append("|---|---|---|---|")
    for key, label, sp, _det in DEPENDENCY_DEFS:
        count = dep_counts.get(key, 0)
        if count == 0:
            continue
        lines.append(f"| {label} | {count} | {sp} | {priority_for_count(count)} |")
    lines.append("")

    # Tests By Dependency
    lines.append("## Tests By Dependency\n")
    for key, label, sp, _det in DEPENDENCY_DEFS:
        tests = dep_tests.get(key, [])
        if not tests:
            continue
        tests_sorted = sorted(tests)
        lines.append(f"### {label} ({len(tests)} tests)\n")
        show = tests_sorted[:20]
        for tid in show:
            lines.append(f"- `{tid}`")
        remaining = len(tests_sorted) - 20
        if remaining > 0:
            lines.append(f"\n*... and {remaining} more*\n")
        else:
            lines.append("")

    # When These Become Actionable
    lines.append("## When These Become Actionable\n")
    lines.append("- **Text rendering**: After SP11 (text shaping) + SP13 (inline layout) are complete")
    lines.append("- **Font metrics**: After SP11 font measurement API")
    lines.append("- **Images**: After image loading pipeline is built")
    lines.append("- **Containment**: After CSS containment model implementation")
    lines.append("- **Gradients**: After gradient painting is implemented")
    lines.append("- **margin-trim**: After margin-trim property support")
    lines.append("")

    MD_OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with open(MD_OUTPUT, "w") as f:
        f.write("\n".join(lines))
    print(f"Plan written: {MD_OUTPUT}")

    # Summary stats
    print(f"\nSummary: {total_deferred} deferred tests across {sum(1 for c in dep_counts.values() if c > 0)} dependency categories")
    for key, label, _sp, _det in DEPENDENCY_DEFS:
        c = dep_counts.get(key, 0)
        if c:
            print(f"  {label}: {c}")


if __name__ == "__main__":
    main()
