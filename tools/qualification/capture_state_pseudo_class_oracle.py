"""Record which elements pinned Chromium 147 matches with interaction states.

Real mouse and keyboard input goes through the DevTools protocol
(`Input.dispatchMouseEvent`, `Input.dispatchKeyEvent`), so Chromium's own
event handling sets hover, active and focus. Script focus calls
`element.focus()`. After each step the page reports, for each selector, the
ids `document.querySelectorAll` returns. The native test
`openui-engine/tests/state_pseudo_classes.rs` builds the same tree, replays
the same steps through the engine and compares
`Engine::matches_selector_list` against the JSON this script writes.

Windows only: it uses the pinned Windows Chrome-for-Testing.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "docs/progress/evidence/state-pseudo-classes-v1/chromium-windows.json"
VIEWPORT = (200, 300)

# Every element has an id so the page can report it. The geometry is
# absolute so the pointer positions below are exact. The native test builds
# the same boxes from `BOXES`.
BOXES = [
    # (id, tag, parent id, left, top, width, height, attributes)
    ("outer", "div", "body", 0, 0, 200, 100, {}),
    ("middle", "div", "outer", 10, 10, 100, 80, {}),
    ("inner", "div", "middle", 10, 10, 40, 40, {}),
    ("sibling", "div", "outer", 130, 10, 40, 40, {}),
    ("controls", "div", "body", 0, 100, 200, 100, {}),
    ("button", "button", "controls", 10, 10, 60, 30, {}),
    ("text", "input", "controls", 100, 10, 80, 30, {"type": "text"}),
    ("group", "div", "controls", 10, 50, 180, 40, {}),
    ("button2", "button", "group", 10, 5, 60, 30, {}),
    ("tabbable", "div", "group", 100, 5, 60, 30, {"tabindex": "0"}),
    ("fields", "div", "body", 0, 200, 200, 100, {}),
    ("checkbox", "input", "fields", 10, 10, 20, 20, {"type": "checkbox"}),
    ("readonly", "input", "fields", 40, 10, 60, 30, {"type": "text", "readonly": ""}),
    ("textarea", "textarea", "fields", 110, 10, 80, 40, {}),
]

SELECTORS = [
    ":hover",
    ":active",
    ":focus",
    ":focus-visible",
    ":focus-within",
    "div:hover > div",
    "#outer:hover #inner",
    "div:not(:hover)",
    "div:has(> :hover)",
    ":hover:active",
    ":is(:hover, :focus)",
    ":focus-within > :focus",
    "button:focus:not(:focus-visible)",
    ":where(:focus-visible)",
    "div:has(:focus-visible)",
]

# (name, [action]) where an action is ("move" | "press" | "release", x, y),
# ("key", key, shift) or ("focus", id).
STEPS = [
    ("initial", []),
    ("hover inner", [("move", 30, 30)]),
    ("press inner", [("press", 30, 30)]),
    ("move to sibling while pressed", [("move", 150, 30)]),
    ("move back to inner while pressed", [("move", 30, 30)]),
    ("move to sibling while pressed again", [("move", 150, 30)]),
    ("release over sibling", [("release", 150, 30)]),
    ("move within sibling", [("move", 151, 31)]),
    ("move to controls", [("move", 195, 195)]),
    ("click button", [("move", 40, 125), ("press", 40, 125), ("release", 40, 125)]),
    ("click text input", [("move", 140, 125), ("press", 140, 125), ("release", 140, 125)]),
    ("tab", [("key", "Tab", False)]),
    ("click button after keyboard", [("move", 40, 125), ("press", 40, 125), ("release", 40, 125)]),
    ("script focus after pointer", [("focus", "tabbable")]),
    ("shift tab", [("key", "Tab", True)]),
    ("script focus after keyboard", [("focus", "button")]),
    ("press shift after pointer focus", [
        ("move", 40, 125), ("press", 40, 125), ("release", 40, 125), ("key", "Shift", False),
    ]),
    ("click focused button again", [("press", 40, 125), ("release", 40, 125)]),
    ("click checkbox", [("move", 20, 220), ("press", 20, 220), ("release", 20, 220)]),
    ("click read-only text input", [("move", 70, 225), ("press", 70, 225), ("release", 70, 225)]),
    ("click textarea", [("move", 150, 230), ("press", 150, 230), ("release", 150, 230)]),
    ("script focus after text pointer focus", [("focus", "button")]),
]

SETUP = """(() => {
  const boxes = %s;
  document.documentElement.id = 'html';
  document.body.id = 'body';
  document.head.remove();
  const style = document.createElement('style');
  style.textContent = 'html, body { margin: 0 } div, button, input, textarea { position: absolute;'
    + ' box-sizing: border-box; margin: 0; padding: 0; border: 0 }';
  document.documentElement.prepend(style);
  for (const [id, tag, parent, left, top, width, height, attributes] of boxes) {
    const element = document.createElement(tag);
    element.id = id;
    for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
    Object.assign(element.style, {left: left + 'px', top: top + 'px', width: width + 'px',
      height: height + 'px'});
    document.getElementById(parent).append(element);
  }
  return document.querySelectorAll('[id]').length;
})()"""

QUERY = """((selectors) => selectors.map(s =>
  Array.from(document.querySelectorAll(s)).map(e => e.id)))(%s)"""

KEYS = {"Tab": ("Tab", 9), "Shift": ("ShiftLeft", 16)}


def act(browser, action, pressed: bool) -> bool:
    command = browser.client.command
    kind = action[0]
    if kind in ("move", "press", "release"):
        _, x, y = action
        params = {"x": x, "y": y}
        if kind == "move":
            params.update(type="mouseMoved", button="left" if pressed else "none",
                          buttons=1 if pressed else 0)
        elif kind == "press":
            params.update(type="mousePressed", button="left", buttons=1, clickCount=1)
            pressed = True
        else:
            params.update(type="mouseReleased", button="left", buttons=0, clickCount=1)
            pressed = False
        command("Input.dispatchMouseEvent", params)
    elif kind == "key":
        _, key, shift = action
        code, vk = KEYS[key]
        # Shift is a modifier while it is down, so its own key down carries it.
        down = 8 if shift or key == "Shift" else 0
        up = 8 if shift else 0
        for type_, modifiers in (("rawKeyDown", down), ("keyUp", up)):
            command("Input.dispatchKeyEvent", {
                "type": type_, "key": key, "code": code, "windowsVirtualKeyCode": vk,
                "nativeVirtualKeyCode": vk, "modifiers": modifiers,
            })
    elif kind == "focus":
        browser.evaluate(f"document.getElementById({json.dumps(action[1])}).focus()")
    else:
        raise ValueError(action)
    return pressed


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    steps = []
    with tempfile.TemporaryDirectory(prefix="openui-state-oracle-") as directory:
        page = Path(directory) / "state.html"
        page.write_text("<!DOCTYPE html><html><head></head><body></body></html>", encoding="utf-8")
        with windows_chrome.WindowsChrome(
            args.chrome or windows_chrome.DEFAULT_CHROME, *VIEWPORT, 1.0
        ) as browser:
            browser.client.command("Emulation.setFocusEmulationEnabled", {"enabled": True})
            browser.load(page)
            count = browser.evaluate(SETUP % json.dumps(BOXES))
            assert count == len(BOXES) + 2, count
            pressed = False
            for name, actions in STEPS:
                for action in actions:
                    pressed = act(browser, action, pressed)
                matches = browser.evaluate(QUERY % json.dumps(SELECTORS))
                steps.append({
                    "step": name,
                    "actions": [list(action) for action in actions],
                    "matches": dict(zip(SELECTORS, matches)),
                })
    result = {
        "chromium": windows_chrome.CHROMIUM_VERSION,
        "platform": "windows",
        "viewport": [*VIEWPORT, 1.0],
        "boxes": [list(box) for box in BOXES],
        "selectors": SELECTORS,
        "steps": steps,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    for step in steps:
        print(step["step"])
        for selector, ids in step["matches"].items():
            print(f"  {selector}: {' '.join(ids)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
