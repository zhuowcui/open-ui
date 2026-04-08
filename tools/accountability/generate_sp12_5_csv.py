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
SUMMARY_PATH = SCRIPT_DIR / "data" / "pixel_comparison" / "results" / "summary.json"
CSV_OUTPUT = SCRIPT_DIR / "data" / "sp12_5_deferred.csv"
MD_OUTPUT = REPO_ROOT / "docs" / "SP12.5-PLAN.md"

# ---------------------------------------------------------------------------
# Dependency detection helpers
# ---------------------------------------------------------------------------

def _strip_style_blocks(html: str) -> str:
    """Remove <style>...</style> blocks (case-insensitive, dotall)."""
    return re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.DOTALL | re.IGNORECASE)

def _has_visible_text(html: str) -> bool:
    """Detect visible text between tags after stripping style blocks."""
    stripped = _strip_style_blocks(html)
    # Find text runs between > and <
    for m in re.finditer(r">([^<]+)<", stripped):
        text = m.group(1).strip()
        if len(text) > 1 and not text.isspace() and text != "{":
            return True
    return False

def _has_font_metrics(html: str) -> bool:
    """Detect dependency on font metrics: font-relative units or line-height."""
    if re.search(r"[\d.]+(?:ch|ex|em|rem)\b", html):
        return True
    if re.search(r"line-height\s*:", html, re.IGNORECASE):
        return True
    return False

def _has_image_ref(html: str) -> bool:
    """Detect url() near background or border-image."""
    return bool(re.search(r"(background|border-image)[^;]*url\(", html, re.IGNORECASE))

def _has_containment(html: str) -> bool:
    """Detect CSS `contain` or `content-visibility` property."""
    return bool(
        re.search(r"(?<![a-zA-Z-])contain\s*:", html)
        or re.search(r"content-visibility\s*:", html, re.IGNORECASE)
    )

def _has_gradient(html: str) -> bool:
    """Detect gradient() function (case-insensitive)."""
    return bool(re.search(r"gradient\(", html, re.IGNORECASE))

def _has_margin_trim(html: str) -> bool:
    """Detect margin-trim property."""
    return bool(re.search(r"margin-trim\s*:", html, re.IGNORECASE))

# Ordered list: (key, label, owning_sp, detector)
DEPENDENCY_DEFS = [
    ("text_rendering",  "SP11/SP13: Text Rendering",  "SP11,SP13", _has_visible_text),
    ("font_metrics",    "SP11: Font Metrics",          "SP11",      _has_font_metrics),
    ("image_rendering", "Image Rendering",             "TBD",       _has_image_ref),
    ("css_containment", "CSS Containment",             "TBD",       _has_containment),
    ("gradient",        "Gradient Rendering",          "TBD",       _has_gradient),
    ("margin_trim",     "margin-trim",                 "TBD",       _has_margin_trim),
]


def classify_test(html: str) -> list[str]:
    """Return list of dependency keys that apply to this test's HTML."""
    deps = []
    for key, _label, _sp, detector in DEPENDENCY_DEFS:
        if detector(html):
            deps.append(key)
    return deps


def priority_for_count(count: int) -> str:
    if count > 100:
        return "high"
    if count >= 20:
        return "medium"
    return "low"


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    # Load data
    with open(TEMPLATES_PATH) as f:
        templates = json.load(f)
    with open(SUMMARY_PATH) as f:
        summary = json.load(f)

    # Build lookup: id -> test record (only failures)
    fail_map: dict[str, dict] = {}
    for t in summary["tests"]:
        if t["status"] == "fail":
            fail_map[t["id"]] = t

    # Classify every failing test
    rows: list[dict] = []
    dep_counts: dict[str, int] = defaultdict(int)
    dep_tests: dict[str, list[str]] = defaultdict(list)

    for test_id, html in templates.items():
        if test_id not in fail_map:
            continue
        deps = classify_test(html)
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
