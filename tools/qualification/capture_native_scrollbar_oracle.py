#!/usr/bin/env python3
"""Capture pinned Chromium frames for the native scrollbar frame-clip guard.

The guard in bindings/rust/openui-engine/tests/native_resize_control_clip.rs
paints Auto and Thin forced scrollbars on 14px-high boxes of width 0, 1, 14,
15 and 16 at 32,32 in a 160x160 viewport, at five device scales. This tool
captures the same states from an empty ordinary Div in pinned Chromium, twice
each, and records every frame's digest and ink extent. With --historical it
also requires byte equality with an earlier capture of the same HTML.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import io
import json
import math
from pathlib import Path
import subprocess
import sys

from PIL import Image


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "tools/accountability/run_all_pixel_comparisons.py"
AHEM = ROOT / "bindings/rust/openui-text/fonts/Ahem.ttf"
CHROMIUM_VERSION = "147.0.7727.50"
CHROME_SHA256 = "0ba87684df0afc11faaea40ac57bd2f1d68b67191443b38215a36b90e6f08848"
VIEWPORT = 160
LEFT = TOP = 32
HEIGHT = 14
WIDTHS = (0, 1, 14, 15, 16)
SCALES = (1.0, 1.25, 1.5, 1.75, 2.0)
STYLES = ("auto", "thin")
REPEATS = 2


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fixture_html(style: str, width: int) -> bytes:
    font = base64.b64encode(AHEM.read_bytes()).decode("ascii")
    return (
        '<!DOCTYPE html><html><head><meta charset="utf-8"><style>'
        f'@font-face{{font-family:Ahem;src:url(data:font/truetype;base64,{font}) '
        'format("truetype");font-style:normal;font-weight:400}'
        "html,body{margin:0;padding:0;background:white;overflow:hidden}"
        f"body{{width:{VIEWPORT}px;height:{VIEWPORT}px}}</style></head><body>"
        f'<div id="target" style="position:absolute;left:{LEFT}px;top:{TOP}px;'
        f"display:block;box-sizing:border-box;width:{width}px;height:{HEIGHT}px;"
        "margin:0;padding:0;border:0;background:white;font:16px Ahem;resize:none;"
        f'overflow:scroll;scrollbar-width:{style};"></div></body></html>\n'
    ).encode("utf-8")


def case_name(style: str, width: int, scale: float) -> str:
    return f"ordinary-{style}-width{width}@{scale:g}"


def load_harness():
    spec = importlib.util.spec_from_file_location("run_all_pixel_comparisons", HARNESS)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def ink_extent(rgba: bytes, side: int) -> list[int] | None:
    """Half-open physical [x0, x1, y0, y1] of every non-white pixel."""
    xs: list[int] = []
    ys: list[int] = []
    for y in range(side):
        row = rgba[y * side * 4:(y + 1) * side * 4]
        for x in range(side):
            if row[x * 4:x * 4 + 3] != b"\xff\xff\xff":
                xs.append(x)
                ys.append(y)
    return [min(xs), max(xs) + 1, min(ys), max(ys) + 1] if xs else None


def crop_rect(width: int, scale: float) -> list[int]:
    """Enclosing physical [x, y, width, height] of the box; it bounds all ink."""
    x0, y0 = math.floor(LEFT * scale), math.floor(TOP * scale)
    x1, y1 = math.ceil((LEFT + width) * scale), math.ceil((TOP + HEIGHT) * scale)
    return [x0, y0, x1 - x0, y1 - y0]


def crop(rgba: bytes, side: int, rect: list[int]) -> tuple[bytes, bool]:
    """Crop bytes and whether every pixel outside the crop is opaque white."""
    x, y, w, h = rect
    rows = [rgba[(row * side + x) * 4:(row * side + x + w) * 4] for row in range(y, y + h)]
    outside = bytearray(rgba)
    for row in range(y, y + h):
        outside[(row * side + x) * 4:(row * side + x + w) * 4] = b"\xff" * (w * 4)
    return b"".join(rows), bytes(outside) == b"\xff" * len(outside)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--crops", type=Path,
                        help="directory for per-case enclosing-box crops and crops.json")
    parser.add_argument("--historical", type=Path,
                        help="earlier capture directory with <case>/repeat-0/chromium.rgba")
    args = parser.parse_args()
    chrome = args.chrome.resolve()
    if sha256(chrome.read_bytes()) != CHROME_SHA256:
        raise SystemExit("chrome binary is not the pinned Chrome-for-Testing build")
    version = subprocess.run([str(chrome), "--version"], capture_output=True, text=True,
                             env={"LD_LIBRARY_PATH": str(chrome.parent)}).stdout
    if CHROMIUM_VERSION not in version:
        raise SystemExit(f"unexpected chrome version: {version!r}")
    harness = load_harness()
    args.output.mkdir(parents=True, exist_ok=False)
    cases = []
    for style in STYLES:
        for width in WIDTHS:
            html = fixture_html(style, width)
            for scale in SCALES:
                name = case_name(style, width, scale)
                folder = args.output / name
                folder.mkdir()
                page = folder / "test.html"
                page.write_bytes(html)
                side = int(VIEWPORT * scale + 0.5)
                frames = []
                for repeat in range(REPEATS):
                    png = folder / f"chromium-{repeat}.png"
                    if not harness.render_chrome(
                        str(page), str(png), str(chrome), str(chrome.parent),
                        logical_width=VIEWPORT, logical_height=VIEWPORT, device_scale=scale,
                    ):
                        raise SystemExit(f"capture failed: {name} repeat {repeat}")
                    image = Image.open(io.BytesIO(png.read_bytes())).convert("RGBA")
                    if image.size != (side, side):
                        raise SystemExit(f"{name}: unexpected size {image.size}")
                    frames.append(image.tobytes())
                if any(frame != frames[0] for frame in frames):
                    raise SystemExit(f"{name}: repeated captures differ")
                rgba = frames[0]
                (folder / "chromium.rgba").write_bytes(rgba)
                row = {
                    "case": name, "style": style, "width": width, "height": HEIGHT,
                    "scale": scale, "physical_size": side, "html_sha256": sha256(html),
                    "rgba_sha256": sha256(rgba), "repeats_exact": True,
                    "ink_extent": ink_extent(rgba, side),
                }
                if args.crops:
                    rect = crop_rect(width, scale)
                    data, outside_white = crop(rgba, side, rect)
                    if not outside_white:
                        raise SystemExit(f"{name}: ink outside the enclosing box crop")
                    row["crop_xywh"] = rect
                    if data:
                        crop_name = f"{style}-w{width}-s{format(scale, 'g').replace('.', '_')}.rgba"
                        args.crops.mkdir(parents=True, exist_ok=True)
                        (args.crops / crop_name).write_bytes(data)
                        row["crop_path"] = crop_name
                        row["crop_sha256"] = sha256(data)
                if args.historical:
                    previous = (args.historical / name / "repeat-0/chromium.rgba").read_bytes()
                    row["historical_rgba_exact"] = previous == rgba
                cases.append(row)
                print(f"{name}: ink={row['ink_extent']} "
                      f"historical={row.get('historical_rgba_exact')}", flush=True)
    receipt = {
        "schema_version": 1,
        "chromium_version": CHROMIUM_VERSION,
        "chrome_sha256": CHROME_SHA256,
        "harness_sha256": sha256(HARNESS.read_bytes()),
        "tool_sha256": sha256(Path(__file__).read_bytes()),
        "ahem_sha256": sha256(AHEM.read_bytes()),
        "viewport_css": VIEWPORT, "box_origin_css": [LEFT, TOP],
        "cases": cases,
        "all_repeats_exact": True,
        "all_historical_exact": all(row.get("historical_rgba_exact", True) for row in cases),
    }
    (args.output / "receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return 0 if receipt["all_historical_exact"] else 1


if __name__ == "__main__":
    sys.exit(main())
