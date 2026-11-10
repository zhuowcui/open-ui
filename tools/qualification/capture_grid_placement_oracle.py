"""Record where pinned Chromium 147 auto-places grid items.

Each case is a grid whose items have fixed sizes, so only placement decides
their positions. The page reports each item's border box relative to its
grid. The native test `openui-layout/tests/grid_auto_placement_cursor_tests.rs`
builds the same grids and compares against the JSON this script writes.

Windows only: it uses the pinned Windows Chrome-for-Testing.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "docs/progress/evidence/grid-placement-cursor-v1/chromium-windows.json"
VIEWPORT = 400

# (name, grid style, [(item id, item style)]). Keep in step with the native
# test.
CASES = [
    (
        "card-header",
        "width:300px; grid-template-columns:1fr auto; grid-template-rows:auto auto;"
        " grid-auto-rows:min-content; gap:8px",
        [
            ("title", "height:16px"),
            ("description", "height:20px"),
            ("action", "grid-column:2; grid-row:1 / span 2; width:50px; height:16px"),
        ],
    ),
    (
        "definite-column-behind-cursor",
        "width:150px; grid-template-columns:50px 50px 50px; grid-auto-rows:20px",
        [
            ("a", "grid-column:2; height:20px"),
            ("b", "grid-column:1; height:20px"),
        ],
    ),
    (
        "row-locked-sparse-cursor",
        "width:300px; grid-template-columns:50px 50px 50px; grid-auto-columns:50px;"
        " grid-auto-rows:20px",
        [
            ("x", "grid-row:1; grid-column:2; height:20px"),
            ("l1", "grid-row:1; grid-column:span 2; height:20px"),
            ("l2", "grid-row:1; height:20px"),
        ],
    ),
    (
        "column-flow-definite-first",
        "width:300px; grid-auto-flow:column; grid-template-rows:20px 20px;"
        " grid-auto-columns:50px",
        [
            ("d", "grid-row:2; grid-column:1; height:20px"),
            ("a", "height:20px"),
            ("b", "height:20px"),
        ],
    ),
]

REPORT = """Array.from(document.querySelectorAll('.grid')).map(grid => {
  const g = grid.getBoundingClientRect();
  return {case: grid.id, items: Array.from(grid.children).map(item => {
    const r = item.getBoundingClientRect();
    return {id: item.id, x: r.left - g.left, y: r.top - g.top,
            width: r.width, height: r.height};
  })};
})"""


def page() -> str:
    grids = []
    for name, grid_style, items in CASES:
        children = "".join(f'<div id="{item}" style="{style}"></div>' for item, style in items)
        grids.append(
            f'<div class="grid" id="{name}" style="display:grid; {grid_style}">{children}</div>'
        )
    return (
        "<!DOCTYPE html><html><head><style>html,body{margin:0}"
        ".grid{margin-bottom:10px}</style></head><body>" + "".join(grids) + "</body></html>"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    with tempfile.TemporaryDirectory(prefix="openui-grid-placement-oracle-") as directory:
        html = Path(directory) / "grid.html"
        html.write_text(page(), encoding="utf-8")
        with windows_chrome.WindowsChrome(
            args.chrome or windows_chrome.DEFAULT_CHROME, VIEWPORT, VIEWPORT, 1.0
        ) as browser:
            browser.load(html)
            cases = browser.evaluate(REPORT)
    result = {
        "chromium": windows_chrome.CHROMIUM_VERSION,
        "platform": "windows",
        "cases": [
            {"case": name, "grid": grid_style, "items": dict(items), "boxes": measured["items"]}
            for (name, grid_style, items), measured in zip(CASES, cases)
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    for case in result["cases"]:
        print(case["case"], [(b["id"], b["x"], b["y"], b["width"]) for b in case["boxes"]])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
