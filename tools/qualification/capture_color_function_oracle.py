#!/usr/bin/env python3
"""Capture pinned Chromium 147 center pixels for the color function swatches.

The swatch list is docs/progress/evidence/color-functions-v1/swatches.txt.
Each swatch is a 20px square: a backdrop with a child painted in the tested
color. The native test `openui/tests/native_color_functions.rs` builds the
same tree through the public API and compares its center pixels with the
JSON this script writes.

Linux uses the census harness's Chromium launcher (pass the pinned Linux
binary with --chrome); Windows uses the pinned Windows Chrome-for-Testing.
"""

from __future__ import annotations

import argparse
import html
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "docs/progress/evidence/color-functions-v1"
SPEC = EVIDENCE / "swatches.txt"
SIZE = 20
COLUMNS = 10
VIEWPORT = 200


def swatches() -> list[tuple[str, str]]:
    result = []
    for line in SPEC.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        backdrop, color = (part.strip() for part in line.split("|", 1))
        result.append((backdrop, color))
    if len(result) > COLUMNS * (VIEWPORT // SIZE):
        raise SystemExit("too many swatches for the viewport")
    return result


def document(items: list[tuple[str, str]]) -> str:
    cells = []
    for index, (backdrop, color) in enumerate(items):
        x, y = (index % COLUMNS) * SIZE, (index // COLUMNS) * SIZE
        cells.append(
            f'<div style="position:absolute;left:{x}px;top:{y}px;width:{SIZE}px;'
            f'height:{SIZE}px;background-color:{html.escape(backdrop)}">'
            f'<div style="width:{SIZE}px;height:{SIZE}px;'
            f'background-color:{html.escape(color)}"></div></div>'
        )
    return (
        "<!DOCTYPE html><html><head><style>html,body{margin:0;overflow:hidden;"
        "background:white}</style></head><body>" + "".join(cells) + "</body></html>"
    )


def center_pixels(png: bytes, items: list[tuple[str, str]]) -> list[list[int]]:
    image = Image.open(io.BytesIO(png)).convert("RGBA")
    if image.size != (VIEWPORT, VIEWPORT):
        raise SystemExit(f"unexpected capture size {image.size}")
    pixels = []
    for index in range(len(items)):
        x = (index % COLUMNS) * SIZE + SIZE // 2
        y = (index // COLUMNS) * SIZE + SIZE // 2
        pixels.append(list(image.getpixel((x, y))))
    return pixels


def capture_linux(page: Path, chrome: Path) -> bytes:
    harness_path = ROOT / "tools/accountability/run_all_pixel_comparisons.py"
    spec = importlib.util.spec_from_file_location("run_all_pixel_comparisons", harness_path)
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    output = page.with_suffix(".png")
    if not harness.render_chrome(
        str(page), str(output), str(chrome), str(chrome.parent),
        logical_width=VIEWPORT, logical_height=VIEWPORT, device_scale=1.0,
    ):
        raise SystemExit("Chromium capture failed")
    return output.read_bytes()


def capture_windows(page: Path, chrome: Path | None) -> bytes:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    with windows_chrome.WindowsChrome(
        chrome or windows_chrome.DEFAULT_CHROME, VIEWPORT, VIEWPORT, 1.0
    ) as browser:
        browser.load(page)
        return browser.screenshot()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--platform", choices=["linux", "windows"], required=True)
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    items = swatches()
    with tempfile.TemporaryDirectory(prefix="openui-color-oracle-") as directory:
        page = Path(directory) / "swatches.html"
        page.write_text(document(items), encoding="utf-8")
        if args.platform == "linux":
            if args.chrome is None:
                raise SystemExit("--chrome is required on Linux")
            png = capture_linux(page, args.chrome)
        else:
            png = capture_windows(page, args.chrome)
    result = {
        "chromium": "147.0.7727.50",
        "platform": args.platform,
        "viewport": [VIEWPORT, VIEWPORT, 1.0],
        "swatches": [
            {"backdrop": backdrop, "color": color, "rgba": rgba}
            for (backdrop, color), rgba in zip(items, center_pixels(png, items))
        ],
    }
    output = args.output or EVIDENCE / f"chromium-{args.platform}.json"
    output.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {len(items)} swatches to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
