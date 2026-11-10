"""Record which elements pinned Chromium 147 matches for each selector.

The fixture is one element tree, built in the page with DOM calls rather
than parsed from HTML, so that no parser fix-up (an implied `tbody`, a
`head`) adds elements. Every element has an `id`. For each selector the page
reports the ids `document.querySelectorAll` returns. The native test
`openui-engine/tests/selector_matching.rs` builds the same tree through the
engine and compares `Engine::matches_selector` against the JSON this script
writes.

Windows only: it uses the pinned Windows Chrome-for-Testing.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "docs/progress/evidence/selector-matching-v1/chromium-windows.json"


def element(tag: str, id_: str, attributes: dict | None = None, children: list | None = None):
    return {"tag": tag, "attributes": {"id": id_, **(attributes or {})}, "children": children or []}


def text(data: str):
    return {"text": data}


# The children of `body`. Keep in step with the native test's tag mapping.
FIXTURE = [
    element(
        "div",
        "a",
        {"class": "box card", "data-state": "open", "lang": "en-US"},
        [
            element("span", "a1", {"class": "title"}, [text("Hello")]),
            element("span", "a2", {"class": "desc muted"}, [text("")]),
            element(
                "div",
                "a3",
                {"class": "actions", "dir": "RTL"},
                [
                    element("button", "a3b", {"class": "btn", "type": "SUBMIT"}),
                    element("img", "a3i"),
                ],
            ),
        ],
    ),
    element(
        "div",
        "b",
        {"class": "box", "data-state": "closed"},
        [
            element("div", "b1", {"class": "item"}, [text(" ")]),
            element("div", "b2", {"class": "item active"}),
            element("div", "b3", {"class": "item"}),
            element("div", "b4", {"class": "item"}),
            element("div", "b5", {"class": "item active"}),
        ],
    ),
    element(
        "table",
        "t",
        {},
        [
            element(
                "tbody",
                "tb",
                {},
                [element("tr", "tr1", {}, [element("td", "td1"), element("td", "td2")])],
            )
        ],
    ),
    element(
        "div",
        "c",
        {"class": "  spaced\tclasses  ", "data-x": "a-b c"},
        [element("div", "c1"), text("tail")],
    ),
]

SELECTORS = [
    "div",
    "span",
    "*",
    "#a1",
    ".box",
    ".item.active",
    ".spaced.classes",
    "div.box > span",
    "#a span",
    "#a > span",
    "span + span",
    "span ~ div",
    "#b1 ~ .active",
    ".title + *",
    "[data-state]",
    "[data-state=open]",
    "[data-state=OPEN]",
    "[data-state=OPEN i]",
    "[dir=rtl]",
    "[type=submit]",
    "[lang|=en]",
    "[lang|=EN]",
    "[data-x|=a]",
    "[data-x~=c]",
    "[data-x~='a-b c']",
    "[class~=muted]",
    "[class^=box]",
    "[class$=card]",
    "[class*=ca]",
    "[class*='']",
    "[class^='']",
    ":is(#a1, .desc)",
    ":where(.box) > .title",
    ":not(.item)",
    "div:not(.item, .box)",
    ".box:has(> .title)",
    ".box:has(.btn)",
    ":has(+ .box)",
    ":has(~ table)",
    "div:has(> span + span)",
    ":has(.active ~ .item)",
    "div:has(> .active + .item)",
    ".item:first-child",
    ".item:last-child",
    ":only-child",
    ":nth-child(2)",
    ":nth-child(odd)",
    ":nth-child(-n+2)",
    ":nth-child(2 of .active)",
    ".item:nth-child(even of .item)",
    ":empty",
    "span:empty",
    ":root",
    ":root > body",
    "html > body > div",
    "* > * > * > span",
    "body *",
    "td:last-child",
    "tr > td + td",
    "div div div button",
    ":is(.box .desc)",
    ":not(:has(*))",
    "#a3 > :first-child:not(img)",
    "body > :nth-child(3) td",
]

BUILD = """(() => {
  const fixture = %s;
  document.head.remove();
  const body = document.body;
  while (body.firstChild) body.firstChild.remove();
  document.documentElement.id = 'html';
  body.id = 'body';
  const build = (spec) => {
    if ('text' in spec) return document.createTextNode(spec.text);
    const el = document.createElement(spec.tag);
    for (const [name, value] of Object.entries(spec.attributes)) el.setAttribute(name, value);
    for (const child of spec.children) el.append(build(child));
    return el;
  };
  for (const spec of fixture) body.append(build(spec));
  const selectors = %s;
  return selectors.map(s => Array.from(document.querySelectorAll(s), e => e.id));
})()"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    with tempfile.TemporaryDirectory(prefix="openui-selector-matching-") as directory:
        html = Path(directory) / "blank.html"
        html.write_text("<!DOCTYPE html><html><head></head><body></body></html>", encoding="utf-8")
        with windows_chrome.WindowsChrome(
            args.chrome or windows_chrome.DEFAULT_CHROME, 200, 200, 1.0
        ) as browser:
            browser.load(html)
            matches = browser.evaluate(BUILD % (json.dumps(FIXTURE), json.dumps(SELECTORS)))
    result = {
        "chromium": windows_chrome.CHROMIUM_VERSION,
        "platform": "windows",
        "fixture": FIXTURE,
        "cases": [
            {"selector": selector, "matches": ids} for selector, ids in zip(SELECTORS, matches)
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    for case in result["cases"]:
        print(case["selector"], case["matches"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
