#!/usr/bin/env python3
"""Analyze GN build target dependencies for Open UI extraction.

Usage:
    python3 tools/analyze_deps.py <chromium_src> <gn_target> [--out-dir <dir>]

Examples:
    python3 tools/analyze_deps.py ~/chromium/src //cc --out-dir out/Default
    python3 tools/analyze_deps.py ~/chromium/src //third_party/skia:skia
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path


def run_gn(chromium_src: str, out_dir: str, *args) -> str:
    """Run a gn command in the Chromium source tree."""
    cmd = ["gn"] + list(args)
    result = subprocess.run(
        cmd,
        cwd=chromium_src,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"ERROR: gn command failed: {' '.join(cmd)}", file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        sys.exit(1)
    return result.stdout


def get_direct_deps(chromium_src: str, out_dir: str, target: str) -> list[str]:
    """Get direct dependencies of a GN target."""
    output = run_gn(chromium_src, out_dir, "desc", out_dir, target, "deps")
    return [line.strip() for line in output.splitlines() if line.strip()]


def get_dep_tree(chromium_src: str, out_dir: str, target: str) -> str:
    """Get full dependency tree of a GN target."""
    return run_gn(chromium_src, out_dir, "desc", out_dir, target, "deps", "--tree")


def get_sources(chromium_src: str, out_dir: str, target: str) -> list[str]:
    """Get source files for a GN target."""
    output = run_gn(chromium_src, out_dir, "desc", out_dir, target, "sources")
    return [line.strip() for line in output.splitlines() if line.strip()]


def categorize_dep(dep: str) -> str:
    """Categorize a dependency by its component."""
    if dep.startswith("//base"):
        return "base"
    elif dep.startswith("//cc"):
        return "cc"
    elif dep.startswith("//gpu"):
        return "gpu"
    elif dep.startswith("//viz"):
        return "viz"
    elif dep.startswith("//ui/gfx"):
        return "ui/gfx"
    elif dep.startswith("//third_party/skia"):
        return "skia"
    elif dep.startswith("//third_party/blink"):
        return "blink"
    elif dep.startswith("//third_party/icu"):
        return "icu"
    elif dep.startswith("//third_party/harfbuzz"):
        return "harfbuzz"
    elif dep.startswith("//third_party/freetype"):
        return "freetype"
    elif dep.startswith("//third_party/abseil"):
        return "abseil"
    elif dep.startswith("//third_party"):
        return "third_party_other"
    elif dep.startswith("//build"):
        return "build"
    elif dep.startswith("//testing"):
        return "testing"
    else:
        return "other"


def analyze_target(chromium_src: str, out_dir: str, target: str):
    """Produce a full dependency analysis for a GN target."""
    print(f"=== Dependency Analysis: {target} ===\n")

    # Direct dependencies
    print("--- Direct Dependencies ---")
    direct_deps = get_direct_deps(chromium_src, out_dir, target)
    categories = defaultdict(list)
    for dep in direct_deps:
        cat = categorize_dep(dep)
        categories[cat].append(dep)

    for cat in sorted(categories.keys()):
        deps = categories[cat]
        print(f"\n  [{cat}] ({len(deps)} targets)")
        for dep in sorted(deps):
            print(f"    {dep}")

    # Source files
    print("\n--- Source Files ---")
    sources = get_sources(chromium_src, out_dir, target)
    print(f"  Total: {len(sources)} files")
    source_exts = defaultdict(int)
    for s in sources:
        ext = Path(s).suffix
        source_exts[ext] += 1
    for ext, count in sorted(source_exts.items(), key=lambda x: -x[1]):
        print(f"    {ext}: {count}")

    # Dependency tree (first 3 levels)
    print("\n--- Dependency Tree (first 100 lines) ---")
    tree = get_dep_tree(chromium_src, out_dir, target)
    for i, line in enumerate(tree.splitlines()):
        if i >= 100:
            print("    ... (truncated)")
            break
        print(f"  {line}")

    # Summary
    print(f"\n--- Summary ---")
    print(f"  Target: {target}")
    print(f"  Direct dependencies: {len(direct_deps)}")
    print(f"  Source files: {len(sources)}")
    print(f"  Component breakdown:")
    for cat in sorted(categories.keys()):
        print(f"    {cat}: {len(categories[cat])}")


def main():
    parser = argparse.ArgumentParser(description="Analyze GN target dependencies")
    parser.add_argument("chromium_src", help="Path to Chromium src/ directory")
    parser.add_argument("target", help="GN target to analyze (e.g., //cc)")
    parser.add_argument("--out-dir", default="out/Default", help="GN output directory")
    args = parser.parse_args()

    if not os.path.isdir(args.chromium_src):
        print(f"ERROR: {args.chromium_src} is not a directory", file=sys.stderr)
        sys.exit(1)

    analyze_target(args.chromium_src, args.out_dir, args.target)


if __name__ == "__main__":
    main()
