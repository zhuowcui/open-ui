#!/usr/bin/env python3
"""Analyze base/ usage across a set of Chromium source files.

Scans #include directives to identify which base/ subsystems a component uses.

Usage:
    python3 tools/analyze_base_usage.py <chromium_src> <component_dir>

Examples:
    python3 tools/analyze_base_usage.py ~/chromium/src cc/
    python3 tools/analyze_base_usage.py ~/chromium/src third_party/blink/renderer/core/layout/
"""

import argparse
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# base/ subsystem categorization
BASE_SUBSYSTEMS = {
    "threading": [
        "base/threading/",
        "base/task/",
        "base/sequence",
        "base/single_thread",
        "base/thread_pool",
        "base/run_loop",
        "base/message_loop",
    ],
    "memory": [
        "base/memory/",
        "base/ref_counted",
        "base/weak_ptr",
        "base/scoped_refptr",
    ],
    "callbacks": [
        "base/callback",
        "base/bind",
        "base/once_callback",
        "base/repeating_callback",
        "base/functional/",
    ],
    "containers": [
        "base/containers/",
        "base/flat_map",
        "base/flat_set",
        "base/circular",
        "base/lru_cache",
    ],
    "strings": [
        "base/strings/",
        "base/string_piece",
        "base/string_util",
        "base/utf_string",
    ],
    "time": [
        "base/time/",
        "base/timer/",
        "base/tick_clock",
    ],
    "logging": [
        "base/logging",
        "base/check",
        "base/notreached",
        "base/debug/",
    ],
    "numerics": [
        "base/numerics/",
        "base/clamped",
        "base/checked",
        "base/saturated",
    ],
    "files": [
        "base/files/",
        "base/file_path",
        "base/file_util",
    ],
    "synchronization": [
        "base/synchronization/",
        "base/lock",
        "base/waitable_event",
        "base/atomic",
    ],
    "observer": [
        "base/observer_list",
    ],
    "tracing": [
        "base/trace_event/",
        "base/tracing/",
    ],
    "metrics": [
        "base/metrics/",
        "base/histogram",
    ],
    "other": [],
}


def categorize_base_include(include_path: str) -> str:
    """Categorize a base/ include into a subsystem."""
    for subsystem, patterns in BASE_SUBSYSTEMS.items():
        if subsystem == "other":
            continue
        for pattern in patterns:
            if pattern in include_path:
                return subsystem
    return "other"


def scan_file(filepath: str) -> list[str]:
    """Extract all base/ includes from a source file."""
    base_includes = []
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            for line in f:
                match = re.match(r'#include\s+"(base/[^"]+)"', line)
                if match:
                    base_includes.append(match.group(1))
    except Exception as e:
        print(f"  Warning: Could not read {filepath}: {e}", file=sys.stderr)
    return base_includes


def analyze_component(chromium_src: str, component_dir: str):
    """Analyze base/ usage in a Chromium component."""
    full_path = os.path.join(chromium_src, component_dir)
    if not os.path.isdir(full_path):
        print(f"ERROR: {full_path} is not a directory", file=sys.stderr)
        sys.exit(1)

    print(f"=== base/ Usage Analysis: {component_dir} ===\n")

    # Scan all source files
    all_includes = defaultdict(list)  # include_path -> [source_files]
    subsystem_usage = defaultdict(set)  # subsystem -> set of include_paths
    file_count = 0

    for root, dirs, files in os.walk(full_path):
        for fname in files:
            if fname.endswith((".cc", ".h", ".cpp", ".hpp")):
                filepath = os.path.join(root, fname)
                rel_path = os.path.relpath(filepath, chromium_src)
                file_count += 1

                base_includes = scan_file(filepath)
                for inc in base_includes:
                    all_includes[inc].append(rel_path)
                    subsystem = categorize_base_include(inc)
                    subsystem_usage[subsystem].add(inc)

    # Report
    print(f"Source files scanned: {file_count}")
    print(f"Unique base/ includes: {len(all_includes)}")
    print()

    # By subsystem
    print("--- Usage by Subsystem ---")
    for subsystem in sorted(subsystem_usage.keys(), key=lambda s: -len(subsystem_usage[s])):
        includes = sorted(subsystem_usage[subsystem])
        print(f"\n  [{subsystem}] ({len(includes)} unique includes)")
        for inc in includes:
            user_count = len(all_includes[inc])
            print(f"    {inc}  (used by {user_count} files)")

    # Most-used includes
    print("\n--- Most Referenced base/ Headers (top 20) ---")
    sorted_includes = sorted(all_includes.items(), key=lambda x: -len(x[1]))
    for inc, files in sorted_includes[:20]:
        subsystem = categorize_base_include(inc)
        print(f"  {inc}  [{subsystem}]  ({len(files)} files)")

    # Summary table
    print("\n--- Subsystem Summary ---")
    print(f"  {'Subsystem':<20} {'Unique Includes':<20} {'Extraction Notes'}")
    print(f"  {'─'*20} {'─'*20} {'─'*40}")
    for subsystem in sorted(subsystem_usage.keys()):
        count = len(subsystem_usage[subsystem])
        notes = {
            "threading": "Must extract (core functionality)",
            "memory": "Must extract (pervasive ownership model)",
            "callbacks": "Must extract (used everywhere)",
            "containers": "Can replace with std/absl",
            "strings": "Can partially replace with std::string_view",
            "time": "Can partially replace with std::chrono",
            "logging": "Can stub with simple implementation",
            "numerics": "Can replace with std equivalents",
            "files": "May not be needed for rendering",
            "synchronization": "Must extract (thread safety)",
            "observer": "Can reimplement simply",
            "tracing": "Can stub (no-op)",
            "metrics": "Can stub (no-op)",
            "other": "Evaluate individually",
        }.get(subsystem, "Evaluate individually")
        print(f"  {subsystem:<20} {count:<20} {notes}")


def main():
    parser = argparse.ArgumentParser(description="Analyze base/ usage in a Chromium component")
    parser.add_argument("chromium_src", help="Path to Chromium src/ directory")
    parser.add_argument("component_dir", help="Component directory relative to src/ (e.g., cc/)")
    args = parser.parse_args()

    analyze_component(args.chromium_src, args.component_dir)


if __name__ == "__main__":
    main()
