#!/usr/bin/env python3
"""Capture pinned Linux Chromium frames for the forced-scrollbar probe grid.

Each probe is the ordinary-Div fixture of `capture_native_scrollbar_oracle.py`
with its width and height replaced. Frames go to `--output` as
`<style>-<width>-<height>-<scale>.png`. Existing frames are kept, so an
interrupted capture resumes. See
`docs/progress/scrollbar-compositing-model-2026-10-25.md`.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import capture_native_scrollbar_oracle as oracle  # noqa: E402

STYLES = ("auto", "thin")
WIDTHS = (1, 1.2, 2, 3, 5, 7.7, 10, 13, 14, 14.6, 15, 15.2, 15.6, 16, 20, 30)
HEIGHTS = (1, 5, 10, 13.8, 14, 14.5, 15, 16, 30, 60)
SCALES = (1.0, 1.25, 1.5, 1.75, 2.0)


def cases():
    return [(style, width, height, scale)
            for scale in SCALES for style in STYLES for width in WIDTHS for height in HEIGHTS]


def case_name(style, width, height, scale) -> str:
    return f"{style}-{json.dumps(width)}-{json.dumps(height)}-{json.dumps(scale)}"


def probe_html(style: str, width, height) -> bytes:
    base = oracle.fixture_html(style, 16)
    needle = b"width:16px;height:14px"
    if base.count(needle) != 1:
        raise SystemExit("fixture layout changed")
    return base.replace(needle, f"width:{width}px;height:{height}px".encode())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--chrome", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("harness", oracle.HARNESS)
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as scratch:
        page = Path(scratch) / "test.html"
        for case in cases():
            png = args.output / f"{case_name(*case)}.png"
            if png.exists():
                continue
            page.write_bytes(probe_html(*case[:3]))
            if not harness.render_chrome(
                str(page), str(png), str(args.chrome), str(args.chrome.parent),
                logical_width=oracle.VIEWPORT, logical_height=oracle.VIEWPORT,
                device_scale=case[3],
            ):
                raise SystemExit(f"capture failed: {case_name(*case)}")
    print(f"{len(cases())} probes in {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
