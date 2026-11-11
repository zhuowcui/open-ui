"""Record which elements pinned Chromium 147 matches with form-state pseudo-classes.

The fixture is one tree of form controls, built in the page with DOM calls
so that no parser fix-up adds elements. Every element has an `id`. After
building it, and after each scripted change (a property such as `checked`,
`indeterminate` or `value`, or an attribute), the page reports, for each
selector, the ids `document.querySelectorAll` returns. The native test
`openui-engine/tests/form_state_pseudo_classes.rs` builds the same tree,
makes the same changes through the engine and compares
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
OUTPUT = ROOT / "docs/progress/evidence/form-state-pseudo-classes-v1/chromium-windows.json"


def element(tag: str, id_: str, attributes: dict | None = None, children: list | None = None):
    return {"tag": tag, "attributes": {"id": id_, **(attributes or {})}, "children": children or []}


# The children of `body`. Keep in step with the native test's tag mapping.
FIXTURE = [
    element("div", "plain"),
    element("button", "b1"),
    element("button", "b2", {"disabled": ""}),
    element("input", "t1"),
    element("input", "t2", {"type": "text", "readonly": ""}),
    element("input", "t3", {"disabled": ""}),
    element("input", "t4", {"placeholder": "P"}),
    element("input", "t5", {"placeholder": "P", "value": "v"}),
    element("input", "t6", {"placeholder": ""}),
    element("input", "t7", {"required": ""}),
    element("input", "t8", {"type": "number", "required": ""}),
    element("input", "t9", {"type": "hidden", "required": ""}),
    element("input", "t10", {"type": "range", "required": ""}),
    element("input", "t11", {"type": "color", "required": ""}),
    element("input", "t12", {"type": "date", "required": "", "placeholder": "P"}),
    element("input", "t13", {"type": "button", "required": ""}),
    element("input", "t14", {"type": "file", "required": ""}),
    element("input", "t15", {"type": "email", "placeholder": "P", "readonly": ""}),
    element("input", "t16", {"type": "password", "placeholder": "P", "disabled": ""}),
    element("input", "t17", {"type": "image", "required": "", "placeholder": "P"}),
    element("input", "t18", {"type": "time", "placeholder": "P"}),
    element("input", "t19", {"type": "SEARCH", "placeholder": "P"}),
    element("input", "c1", {"type": "checkbox"}),
    element("input", "c2", {"type": "checkbox", "checked": ""}),
    element("input", "c3", {"type": "checkbox"}),
    element("input", "c4", {"type": "checkbox", "required": "", "disabled": ""}),
    element("input", "r1", {"type": "radio", "name": "g1"}),
    element("input", "r2", {"type": "radio", "name": "g1"}),
    element("input", "r3", {"type": "radio", "name": "g2", "checked": ""}),
    element("input", "r4", {"type": "radio", "name": "g2", "required": ""}),
    element("input", "r5", {"type": "radio"}),
    element("textarea", "ta1"),
    element("textarea", "ta2", {"readonly": ""}),
    element("textarea", "ta3", {"placeholder": "P"}),
    element("textarea", "ta4", {"required": "", "disabled": ""}),
    element("select", "s1", {}, [
        element("option", "o1"),
        element("option", "o2", {"selected": ""}),
    ]),
    element("select", "s2", {"disabled": ""}, [
        element("optgroup", "og1", {}, [element("option", "o3")]),
    ]),
    element("select", "s3", {"required": ""}, [element("option", "o4", {"value": ""})]),
    element("select", "s4", {}, [
        element("optgroup", "og2", {"disabled": ""}, [element("option", "o5")]),
        element("option", "o6", {"disabled": ""}),
    ]),
    element("select", "s5", {"multiple": ""}, [element("option", "o7")]),
    element("select", "s6", {"size": "3"}, [element("option", "o8")]),
    element("select", "s7", {}, [
        element("option", "o9", {"selected": ""}),
        element("option", "o10", {"selected": ""}),
        element("option", "o11"),
    ]),
    element("fieldset", "f1", {"disabled": ""}, [
        element("legend", "l1", {}, [element("input", "fl1")]),
        element("input", "fi1"),
        element("fieldset", "f2", {}, [element("button", "fb1")]),
    ]),
    element("fieldset", "f3", {}, [element("legend", "l2"), element("input", "fi2")]),
    element("progress", "p1"),
    element("progress", "p2", {"value": "0.5"}),
    element("progress", "p3", {"value": "x"}),
    element("meter", "m1"),
    element("form", "form1", {}, [element("input", "fo1")]),
    element("div", "d1", {"disabled": "", "readonly": "", "required": ""}),
]

SELECTORS = [
    ":disabled",
    ":enabled",
    ":checked",
    ":indeterminate",
    ":placeholder-shown",
    ":read-only",
    ":read-write",
    ":required",
    "input:not(:enabled)",
    "fieldset:disabled input",
    ":checked + input",
    "select:has(:checked)",
    "input:read-write:not(:placeholder-shown)",
]

# (name, [change]) where a change is ("property", id, name, value),
# ("attribute", id, name, value) or ("remove", id, name).
STEPS = [
    ("initial", []),
    ("check c1, set c3 indeterminate", [
        ("property", "c1", "checked", True), ("property", "c3", "indeterminate", True),
    ]),
    ("uncheck c2", [("property", "c2", "checked", False)]),
    ("check r1", [("property", "r1", "checked", True)]),
    ("type into t4 and ta3", [("property", "t4", "value", "x"), ("property", "ta3", "value", "y")]),
    ("clear t5", [("property", "t5", "value", "")]),
    ("enable f1", [("remove", "f1", "disabled")]),
    ("disable f3", [("attribute", "f3", "disabled", "")]),
    ("make t2 writable, t1 read-only", [("remove", "t2", "readonly"), ("attribute", "t1", "readonly", "")]),
    ("remove p2 value", [("remove", "p2", "value")]),
    ("make c1 a radio", [("attribute", "c1", "type", "radio")]),
]

BUILD = """(() => {
  const fixture = %s;
  document.head.remove();
  const body = document.body;
  while (body.firstChild) body.firstChild.remove();
  document.documentElement.id = 'html';
  body.id = 'body';
  const build = (spec) => {
    const el = document.createElement(spec.tag);
    for (const [name, value] of Object.entries(spec.attributes)) el.setAttribute(name, value);
    for (const child of spec.children) el.append(build(child));
    return el;
  };
  for (const spec of fixture) body.append(build(spec));
  return document.querySelectorAll('[id]').length;
})()"""

CHANGE = """((changes) => {
  for (const [kind, id, name, value] of changes) {
    const el = document.getElementById(id);
    if (kind === 'property') el[name] = value;
    else if (kind === 'attribute') el.setAttribute(name, value);
    else el.removeAttribute(name);
  }
})(%s)"""

QUERY = """((selectors) => selectors.map(s =>
  Array.from(document.querySelectorAll(s), e => e.id)))(%s)"""


def count(specs) -> int:
    return sum(1 + count(spec["children"]) for spec in specs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    steps = []
    with tempfile.TemporaryDirectory(prefix="openui-form-state-oracle-") as directory:
        html = Path(directory) / "blank.html"
        html.write_text("<!DOCTYPE html><html><head></head><body></body></html>", encoding="utf-8")
        with windows_chrome.WindowsChrome(
            args.chrome or windows_chrome.DEFAULT_CHROME, 200, 200, 1.0
        ) as browser:
            browser.load(html)
            built = browser.evaluate(BUILD % json.dumps(FIXTURE))
            assert built == count(FIXTURE) + 2, built
            for name, changes in STEPS:
                browser.evaluate(CHANGE % json.dumps(changes))
                matches = browser.evaluate(QUERY % json.dumps(SELECTORS))
                steps.append({
                    "step": name,
                    "changes": [list(change) for change in changes],
                    "matches": dict(zip(SELECTORS, matches)),
                })
    result = {
        "chromium": windows_chrome.CHROMIUM_VERSION,
        "platform": "windows",
        "fixture": FIXTURE,
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
