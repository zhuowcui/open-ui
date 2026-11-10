"""Record which elements pinned Chromium 147 matches with `:active`.

Real mouse input goes through the DevTools protocol
(`Input.dispatchMouseEvent`), so Chromium's own press handling sets the
active chain. After each step the page reports the ids of the elements
that match `:active`. The native test `openui/tests/active_chain.rs` builds
the same tree through the public API and checks `Element::is_active`
against the JSON this script writes.

Windows only: it uses the pinned Windows Chrome-for-Testing.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "docs/progress/evidence/active-chain-v1/chromium-windows.json"
VIEWPORT = 200

# Every element has an id so the page can report it. The geometry is
# absolute so the press points below are exact. The native test builds the
# same boxes; keep the two in step.
PAGE = """<!DOCTYPE html><html id="html"><head><style>
html, body { margin: 0 }
div, button, span { position: absolute; box-sizing: border-box; margin: 0;
  padding: 0; border: 0 }
</style></head><body id="body">
<div id="outer" style="left:0;top:0;width:200px;height:100px">
  <div id="middle" style="left:10px;top:10px;width:100px;height:80px">
    <div id="inner" style="left:10px;top:10px;width:40px;height:40px"></div>
  </div>
  <div id="sibling" style="left:130px;top:10px;width:40px;height:40px"></div>
</div>
<div id="controls" style="left:0;top:100px;width:200px;height:100px">
  <button id="enabled" style="left:10px;top:10px;width:60px;height:30px"></button>
  <button id="disabled" disabled style="left:100px;top:10px;width:60px;height:30px"></button>
</div>
</body></html>"""

# (name, [(event, x, y)]) where event is press, move or release.
STEPS = [
    ("press inner", [("press", 30, 30)]),
    ("move to sibling while pressed", [("move", 150, 30)]),
    ("release over sibling", [("release", 150, 30)]),
    ("press enabled button", [("move", 40, 125), ("press", 40, 125)]),
    ("release enabled button", [("release", 40, 125)]),
    ("press disabled button", [("move", 130, 125), ("press", 130, 125)]),
    ("release disabled button", [("release", 130, 125)]),
]

ACTIVE = (
    "Array.from(document.querySelectorAll('[id]'))"
    ".filter(e => e.matches(':active')).map(e => e.id)"
)


def mouse(browser, event: str, x: int, y: int) -> None:
    kind = {"press": "mousePressed", "move": "mouseMoved", "release": "mouseReleased"}[event]
    params = {"type": kind, "x": x, "y": y, "button": "left" if event != "move" else "none"}
    if event == "press":
        params.update(clickCount=1, buttons=1)
    elif event == "release":
        params.update(clickCount=1, buttons=0)
    else:
        params["buttons"] = 0
    browser.client.command("Input.dispatchMouseEvent", params)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    results = []
    with tempfile.TemporaryDirectory(prefix="openui-active-oracle-") as directory:
        page = Path(directory) / "active.html"
        page.write_text(PAGE, encoding="utf-8")
        with windows_chrome.WindowsChrome(
            args.chrome or windows_chrome.DEFAULT_CHROME, VIEWPORT, VIEWPORT, 1.0
        ) as browser:
            browser.load(page)
            pressed = False
            for name, events in STEPS:
                for event, x, y in events:
                    if event == "move" and pressed:
                        # A move with the left button held.
                        browser.client.command("Input.dispatchMouseEvent", {
                            "type": "mouseMoved", "x": x, "y": y,
                            "button": "left", "buttons": 1,
                        })
                    else:
                        mouse(browser, event, x, y)
                    pressed = event == "press" or (pressed and event == "move")
                results.append({
                    "step": name,
                    "events": [[event, x, y] for event, x, y in events],
                    "active": browser.evaluate(ACTIVE),
                })
    result = {
        "chromium": windows_chrome.CHROMIUM_VERSION,
        "platform": "windows",
        "viewport": [VIEWPORT, VIEWPORT, 1.0],
        "steps": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps(results, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
