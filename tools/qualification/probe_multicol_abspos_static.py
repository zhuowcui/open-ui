#!/usr/bin/env python3
"""Measure where Chrome places an abspos box after content that fills a column.

Each case is a 100x100 four-column multicol (`column-fill: auto`, no gap)
holding block or flex content, then a 25x20 absolutely positioned box. The
`flex-line-forced-*` cases instead measure the block after a definite column
flexbox in the five-column multicol of
`flexbox_multi-line-column-flex-fragmentation-025`. The script loads every case in
pinned Windows Chrome-for-Testing (`windows_chrome.py`) and prints the box's
client rects as JSON, once with the multicol as the containing block and once
with the initial containing block. Used for
`docs/progress/multicol-abspos-column-end-2026-10-27.md`.

    python tools/qualification/probe_multicol_abspos_static.py
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import windows_chrome  # noqa: E402


def box(insets: str) -> str:
    return f'<div id=x style="position:absolute;width:25px;height:20px;{insets}"></div>'


FLEX_400 = ('<div style="display:flex;flex-direction:column;width:25px">'
            '<div style="height:400px"></div></div>')
ROW_FLEX_MARGIN = ('<div style="display:flex"><div style="margin-bottom:150px">'
                   '<div style="height:50px;width:25px"></div></div></div>')

def column_flex(break_style: str, height: str = "") -> str:
    items = ["height:50px", "height:50px", "height:300px"]
    index, declaration = break_style.split(":", 1)
    items[int(index)] += ";" + declaration
    return (f'<div style="display:flex;flex-direction:column;width:25px;{height}">'
            + "".join(f'<div style="{item}"></div>' for item in items) + "</div>")


COLUMN_FLEX_BREAKS = {
    "avoid-before": "2:break-before:avoid",
    "avoid-after": "1:break-after:avoid",
    "forced": "1:break-before:column",
}
CASES = {
    "block-100-top-auto": '<div style="height:100px"></div>' + box(""),
    "block-100-top-10": '<div style="height:100px"></div>' + box("top:10px"),
    "block-100-top-10-then-50": ('<div style="height:100px"></div>' + box("top:10px")
                                 + '<div style="height:50px"></div>'),
    "block-100-top-auto-then-50": ('<div style="height:100px"></div>' + box("")
                                   + '<div style="height:50px"></div>'),
    "block-150-top-10": '<div style="height:150px"></div>' + box("top:10px"),
    "block-200-top-0": '<div style="height:200px"></div>' + box("top:0"),
    "block-200-top-0-then-50": ('<div style="height:200px"></div>' + box("top:0")
                                + '<div style="height:50px"></div>'),
    "block-400-top-0": '<div style="height:400px"></div>' + box("top:0"),
    "block-400-top-auto": '<div style="height:400px"></div>' + box(""),
    "column-flex-400-top-0": FLEX_400 + box("top:0"),
    "column-flex-400-top-auto": FLEX_400 + box(""),
    "row-flex-trailing-margin-top-50": (ROW_FLEX_MARGIN + box("top:50px")
                                        + '<div style="height:100px;width:25px"></div>'),
    "row-flex-trailing-margin-top-auto": ROW_FLEX_MARGIN + box(""),
}
for _name, _break in COLUMN_FLEX_BREAKS.items():
    for _height in ("", "height:400px"):
        for _insets in ("", "top:0"):
            CASES[f"column-flex-{_name}{'-h400' if _height else ''}-"
                  f"{'top-0' if _insets else 'top-auto'}"] = (
                column_flex(_break, _height) + box(_insets))
MULTICOL = "width:100px;height:100px;columns:4;column-gap:0;column-fill:auto"
FIVE_COLUMNS = "width:100px;height:100px;columns:5;column-gap:0;column-fill:auto"


def forced_line_flex(first_item: str) -> str:
    items = [f"height:50px;{first_item}", "height:50px", "height:250px",
             "height:100px;break-before:column", "height:50px", "height:50px", "height:150px"]
    block = '<div {}style="width:20px;height:50px"></div>'
    return (block.format("") + '<div style="display:flex;flex-direction:column;'
            'flex-wrap:wrap;height:350px">'
            + "".join(f'<div style="width:10px;{item}"></div>' for item in items)
            + "</div>" + block.format("id=x "))


FIVE_COLUMN_CASES = {
    "flex-line-forced-following-block": forced_line_flex(""),
    "flex-line-forced-avoid-first-following-block": forced_line_flex("break-before:avoid"),
}


def main() -> None:
    results: dict[str, dict[str, list[list[float]]]] = {}
    with tempfile.TemporaryDirectory() as temporary, \
            windows_chrome.WindowsChrome(windows_chrome.DEFAULT_CHROME, 800, 600, 1.0) as chrome:
        for containing_block, position in (("multicol", "position:relative;"),
                                           ("initial", "")):
            cases = [(name, body, MULTICOL) for name, body in CASES.items()]
            cases += [(name, body, FIVE_COLUMNS) for name, body in FIVE_COLUMN_CASES.items()]
            for name, body, multicol in cases:
                page = Path(temporary) / f"{containing_block}-{name}.html"
                page.write_text(
                    "<!DOCTYPE html><html><body style='margin:0'>"
                    f'<div style="{position}{multicol}">{body}</div></body></html>',
                    encoding="utf-8")
                chrome.load(page)
                rects = json.loads(chrome.evaluate(
                    "JSON.stringify([...x.getClientRects()].map(r => [r.x, r.y]))"))
                results.setdefault(containing_block, {})[name] = rects
    print(json.dumps({"chromium": windows_chrome.CHROMIUM_VERSION, "results": results},
                     indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
