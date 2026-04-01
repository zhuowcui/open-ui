#!/usr/bin/env python3
"""Analyze cross-layer dependencies between Chromium components.

Finds #include directives that cross component boundaries to identify
coupling points for extraction.

Usage:
    python3 tools/analyze_cross_layer.py <chromium_src> <from_dir> <to_dir>

Examples:
    # Find all includes from cc/ into blink/
    python3 tools/analyze_cross_layer.py ~/chromium/src cc/ third_party/blink/

    # Find all includes from layout/ into dom/
    python3 tools/analyze_cross_layer.py ~/chromium/src \
        third_party/blink/renderer/core/layout/ \
        third_party/blink/renderer/core/dom/
"""

import argparse
import os
import re
import sys
from collections import defaultdict
from pathlib import Path


def scan_cross_includes(chromium_src: str, from_dir: str, to_dir: str):
    """Find all #include directives from from_dir that reference to_dir."""
    full_from = os.path.join(chromium_src, from_dir)
    if not os.path.isdir(full_from):
        print(f"ERROR: {full_from} is not a directory", file=sys.stderr)
        sys.exit(1)

    print(f"=== Cross-Layer Analysis: {from_dir} → {to_dir} ===\n")

    # Normalize to_dir for matching (strip trailing slash, handle both formats)
    to_patterns = [
        to_dir.rstrip("/"),
        to_dir.rstrip("/").replace("/", "\\"),
    ]

    crossings = defaultdict(list)  # included_header -> [(source_file, line_num)]
    file_count = 0

    for root, dirs, files in os.walk(full_from):
        for fname in files:
            if not fname.endswith((".cc", ".h", ".cpp", ".hpp")):
                continue

            filepath = os.path.join(root, fname)
            rel_path = os.path.relpath(filepath, chromium_src)
            file_count += 1

            try:
                with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
                    for line_num, line in enumerate(f, 1):
                        match = re.match(r'#include\s+"([^"]+)"', line)
                        if match:
                            included = match.group(1)
                            for pattern in to_patterns:
                                if included.startswith(pattern) or f"/{pattern}" in included:
                                    crossings[included].append((rel_path, line_num))
                                    break
            except Exception as e:
                print(f"  Warning: Could not read {filepath}: {e}", file=sys.stderr)

    if not crossings:
        print(f"  No cross-layer includes found from {from_dir} to {to_dir}")
        return

    # Report
    print(f"Files scanned in {from_dir}: {file_count}")
    print(f"Unique cross-layer includes: {len(crossings)}")
    print(f"Total cross-references: {sum(len(v) for v in crossings.values())}")

    # By included header
    print("\n--- Cross-Layer Includes (by target header) ---")
    for header in sorted(crossings.keys()):
        sources = crossings[header]
        print(f"\n  {header}  (referenced by {len(sources)} files)")
        for src, line in sorted(sources):
            print(f"    ← {src}:{line}")

    # By source file
    print("\n--- Source Files with Cross-Layer Includes ---")
    source_crossings = defaultdict(list)
    for header, sources in crossings.items():
        for src, line in sources:
            source_crossings[src].append((header, line))

    for src in sorted(source_crossings.keys()):
        includes = source_crossings[src]
        print(f"\n  {src} ({len(includes)} cross-layer includes)")
        for header, line in sorted(includes, key=lambda x: x[1]):
            print(f"    line {line}: {header}")

    # Summary: which symbols are actually used?
    print("\n--- Extraction Guidance ---")
    print(f"  To decouple {from_dir} from {to_dir}:")
    print(f"  {len(crossings)} headers need to be either:")
    print(f"    - Extracted alongside {from_dir}")
    print(f"    - Replaced with an adapter/interface")
    print(f"    - Stubbed with a minimal implementation")
    print(f"\n  High-traffic headers (most referenced):")
    for header in sorted(crossings.keys(), key=lambda h: -len(crossings[h]))[:10]:
        count = len(crossings[header])
        print(f"    {header}  ({count} references)")


def main():
    parser = argparse.ArgumentParser(
        description="Analyze cross-layer includes between Chromium components"
    )
    parser.add_argument("chromium_src", help="Path to Chromium src/ directory")
    parser.add_argument("from_dir", help="Source component directory (e.g., cc/)")
    parser.add_argument("to_dir", help="Target component directory to find includes into")
    args = parser.parse_args()

    scan_cross_includes(args.chromium_src, args.from_dir, args.to_dir)


if __name__ == "__main__":
    main()
