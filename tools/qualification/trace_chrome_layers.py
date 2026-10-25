#!/usr/bin/env python3
"""Print Chromium's cc layers and recorded clip and rect operations for a page.

The page is loaded in Windows Chrome-for-Testing (`windows_chrome.py`) under a
DevTools trace with cc debug categories. The script prints the renderer's last
layer tree and, for each display list, its clip and draw-rect operations
decoded from the traced SkPictures (`CLIP_RECT` parameter bit 4 is
antialiasing). Used for `docs/progress/scrollbar-compositing-model-2026-10-25.md`.

    python tools/qualification/trace_chrome_layers.py page.html --scale 1.25
"""

from __future__ import annotations

import argparse
import base64
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import windows_chrome  # noqa: E402

CATEGORIES = [
    "disabled-by-default-cc.debug",
    "disabled-by-default-cc.debug.quads",
    "disabled-by-default-cc.debug.display_items",
    "disabled-by-default-viz.quads",
]
SAVE, RESTORE, CLIP_RECT, DRAW_RECT = 30, 28, 3, 21


def decode_ops(picture: bytes) -> list[str]:
    """The clip and rect operations of one serialized SkPicture."""
    start = picture.find(b"daer")
    if start < 0:
        return []
    size = struct.unpack_from("<I", picture, start + 4)[0]
    cursor, end, ops = start + 8, start + 8 + size, []
    while cursor < end:
        word = struct.unpack_from("<I", picture, cursor)[0]
        op, length = word >> 24, (word & 0xFFFFFF) or 4
        body = picture[cursor + 4:cursor + length]
        if op == CLIP_RECT:
            left, top, right, bottom, params = struct.unpack_from("<4fI", body)
            ops.append(f"clipRect({left:g},{top:g},{right:g},{bottom:g}) aa={(params >> 4) & 1}")
        elif op == DRAW_RECT:
            _, left, top, right, bottom = struct.unpack_from("<I4f", body)
            ops.append(f"drawRect({left:g},{top:g},{right:g},{bottom:g})")
        elif op in (SAVE, RESTORE):
            ops.append("save" if op == SAVE else "restore")
        else:
            ops.append(f"op{op}")
        cursor += length
    return ops


def trace(chrome: windows_chrome.WindowsChrome, page: Path) -> list[dict]:
    client = chrome.client
    client.command("Tracing.start", {
        "traceConfig": {"includedCategories": CATEGORIES},
        "transferMode": "ReportEvents",
    })
    chrome.load(page)
    chrome.screenshot()
    command_id = client.next_id
    client.next_id += 1
    client._send_frame(json.dumps({"id": command_id, "method": "Tracing.end"}).encode())
    events = []
    while True:
        message = json.loads(client._read_message())
        if message.get("method") == "Tracing.dataCollected":
            events += message["params"]["value"]
        if message.get("method") == "Tracing.tracingComplete":
            return events


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("page", type=Path)
    parser.add_argument("--scale", type=float, default=1.0)
    parser.add_argument("--viewport", type=int, default=160)
    parser.add_argument("--chrome", type=Path, default=windows_chrome.DEFAULT_CHROME)
    args = parser.parse_args()
    with windows_chrome.WindowsChrome(args.chrome, args.viewport, args.viewport, args.scale) as chrome:
        events = trace(chrome, args.page)
    snapshots = [
        event["args"]["snapshot"] for event in events
        if event.get("ph") == "O" and event["name"] == "cc::LayerTreeHostImpl"
        and any("LayoutView" in layer.get("layer_name", "")
                for layer in event["args"]["snapshot"]["active_tree"]["layers"])
    ]
    if not snapshots:
        raise SystemExit("no renderer layer tree in the trace")
    for layer in snapshots[-1]["active_tree"]["layers"]:
        print("layer", layer.get("layer_id"), layer.get("layer_name", ""), layer["bounds"],
              "draws" if layer.get("draws_content") else "", layer.get("layer_quad", ""))
    seen = set()
    for event in events:
        if event.get("ph") != "O" or event["name"] != "cc::DisplayItemList":
            continue
        params = event["args"]["snapshot"].get("params", {})
        for item in params.get("items", []):
            ops = decode_ops(base64.b64decode(item["skp64"])) if "skp64" in item else []
            key = json.dumps([params.get("layer_rect"), item["name"], item.get("visual_rect"), ops])
            if key not in seen and any(op.startswith(("clip", "draw")) for op in ops):
                seen.add(key)
                print("list", params.get("layer_rect"), item["name"], item.get("visual_rect"), ops)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
