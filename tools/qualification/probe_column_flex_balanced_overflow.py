#!/usr/bin/env python3
"""Measure the fragments of a column flex item after a forced break in a balanced multicol.

The content is that of `flexbox_single-line-column-flex-fragmentation-018`: a
four-column, gapless, `column-fill: balance` multicol 100px wide holding a
25px column flexbox whose items are 50, 50 and 300px tall, with
`break-before: column` on the second. Each case varies only the multicol's
height. The script loads every case in pinned Windows Chrome-for-Testing
(`windows_chrome.py`) and prints the client rects of the 300px item as JSON
`[x, y, width, height]`. Used for
`docs/progress/column-flex-balanced-overflow-2026-10-27.md`.

    python tools/qualification/probe_column_flex_balanced_overflow.py
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import windows_chrome  # noqa: E402

FLEX = ('<div style="display:flex;flex-direction:column;width:25px">'
        '<div style="height:50px"></div>'
        '<div style="height:50px;break-before:column"></div>'
        '<div id=x style="height:300px"></div></div>')
MULTICOL = "column-count:4;column-gap:0;width:100px;"
CASES = {
    "height-100": MULTICOL + "height:100px",
    "height-auto": MULTICOL,
}


def main() -> None:
    results: dict[str, list[list[float]]] = {}
    with tempfile.TemporaryDirectory() as temporary, \
            windows_chrome.WindowsChrome(windows_chrome.DEFAULT_CHROME, 800, 600, 1.0) as chrome:
        for name, multicol in CASES.items():
            page = Path(temporary) / f"{name}.html"
            page.write_bytes(
                ("<!DOCTYPE html><html><body style='margin:0'>"
                 f'<div style="{multicol}">{FLEX}</div></body></html>\n').encode())
            chrome.load(page)
            results[name] = json.loads(chrome.evaluate(
                "JSON.stringify([...x.getClientRects()]"
                ".map(r => [r.x, r.y, r.width, r.height]))"))
    print(json.dumps({"chromium": windows_chrome.CHROMIUM_VERSION, "results": results},
                     indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
