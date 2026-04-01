#!/usr/bin/env python3
"""Generate skia_link_deps.rsp from a Chromium build.

Extracts all .a, .rlib, and .o files from Chromium's link command for
the openui:skia_poc target and writes them to a linker response file.

Usage:
    python3 tools/gen-skia-link-deps.py /path/to/chromium/src [out/Release]

The response file is written to build/config/chromium/skia_link_deps.rsp.
"""

import os
import re
import sys


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <chromium_src> [build_dir]")
        print(f"  chromium_src: Path to Chromium's src/ directory")
        print(f"  build_dir:   Build output dir (default: out/Release)")
        sys.exit(1)

    chromium_src = os.path.abspath(sys.argv[1])
    build_dir = sys.argv[2] if len(sys.argv) > 2 else "out/Release"
    ninja_file = os.path.join(chromium_src, build_dir, "obj/openui/skia_poc.ninja")

    if not os.path.exists(ninja_file):
        print(f"ERROR: {ninja_file} not found.")
        print(f"Make sure you've built openui:skia_poc in Chromium's tree first.")
        print(f"  1. Copy openui/ to {chromium_src}/openui/")
        print(f"  2. Add '\"//openui:skia_poc\",' to deps in {chromium_src}/BUILD.gn gn_all group")
        print(f"  3. gn gen {build_dir} && ninja -C {build_dir} openui:skia_poc")
        sys.exit(1)

    # Read the ninja file and find the link command
    with open(ninja_file) as f:
        content = f.read()

    # Find the "build ./skia_poc: link ..." line
    match = re.search(r'^build \./skia_poc: link (.+?)$', content, re.MULTILINE | re.DOTALL)
    if not match:
        print("ERROR: Could not find 'build ./skia_poc: link' in ninja file")
        sys.exit(1)

    # The link line continues with ' $\n' for continuation
    link_line = match.group(1).replace(' $\n', ' ')
    tokens = link_line.split()

    # Extract .a, .rlib, and .o files, convert to absolute paths
    # Skip our own openui .o files (we compile those ourselves)
    deps = []
    build_out = os.path.join(chromium_src, build_dir)
    for token in tokens:
        if token.endswith(('.a', '.rlib', '.o')):
            if 'openui/' in token:
                continue  # Skip our own compiled objects
            if os.path.isabs(token):
                deps.append(token)
            else:
                deps.append(os.path.join(build_out, token))

    # Also add Rust --start-group/--end-group around rlibs
    rlibs = [d for d in deps if d.endswith('.rlib')]
    non_rlibs = [d for d in deps if not d.endswith('.rlib')]

    # Write response file
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(script_dir)
    rsp_path = os.path.join(repo_root, "build/config/chromium/skia_link_deps.rsp")

    os.makedirs(os.path.dirname(rsp_path), exist_ok=True)
    with open(rsp_path, 'w') as f:
        for dep in non_rlibs:
            f.write(dep + '\n')
        f.write('-Wl,--start-group\n')
        for dep in rlibs:
            f.write(dep + '\n')
        f.write('-Wl,--end-group\n')

    print(f"Generated {rsp_path}")
    print(f"  {len(non_rlibs)} static libraries/objects")
    print(f"  {len(rlibs)} Rust rlibs")
    print(f"  {len(deps)} total dependencies")


if __name__ == '__main__':
    main()
