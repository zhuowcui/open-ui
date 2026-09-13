#!/usr/bin/env python3
"""Verify the versioned v0.2 application conformance manifest."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "tools/conformance/v02-scenarios.json"
TEST_SOURCE = ROOT / "bindings/rust/openui/tests/v02_conformance.rs"
TEST_PATTERN = re.compile(r"#\[test\]\s*fn\s+([a-z][a-z0-9_]*)\s*\(")


def fail(message: str) -> None:
    print(f"v0.2 conformance manifest: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        fail("unsupported schema_version")

    scenarios = data.get("scenarios")
    if not isinstance(scenarios, list):
        fail("scenarios must be an array")
    minimum = data.get("minimum_scenarios")
    if not isinstance(minimum, int) or minimum < 30 or len(scenarios) < minimum:
        fail(f"requires at least 30 scenarios, found {len(scenarios)}")

    ids = [scenario.get("id") for scenario in scenarios]
    if any(not isinstance(identifier, str) or not identifier for identifier in ids):
        fail("every scenario requires a non-empty string id")
    duplicates = sorted({identifier for identifier in ids if ids.count(identifier) > 1})
    if duplicates:
        fail(f"duplicate ids: {', '.join(duplicates)}")

    required_domains = set(data.get("required_domains", []))
    covered_domains: set[str] = set()
    for scenario in scenarios:
        domains = scenario.get("domains")
        if not isinstance(domains, list) or not domains:
            fail(f"{scenario['id']} has no domains")
        covered_domains.update(domains)
    missing_domains = sorted(required_domains - covered_domains)
    if missing_domains:
        fail(f"uncovered required domains: {', '.join(missing_domains)}")

    source = TEST_SOURCE.read_text(encoding="utf-8")
    tests = set(TEST_PATTERN.findall(source))
    manifest_ids = set(ids)
    missing_tests = sorted(manifest_ids - tests)
    unclassified_tests = sorted(tests - manifest_ids)
    if missing_tests:
        fail(f"manifest entries without tests: {', '.join(missing_tests)}")
    if unclassified_tests:
        fail(f"tests absent from manifest: {', '.join(unclassified_tests)}")

    print(
        f"v0.2 conformance manifest: {len(ids)} scenarios, "
        f"{len(covered_domains)} domains, exact test parity"
    )


if __name__ == "__main__":
    main()
