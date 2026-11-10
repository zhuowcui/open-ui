"""Record pinned Chromium 147's specificity for the selectors RFC 0001 supports.

Each accepted case is a selector, optionally nested in a chain of parent
style rules. The page gives every case its own rule, `X, * { --kN: 1 }` or,
nested, `P1, * { P2, * { X, & { --kN: 1 } } }`, so the rule matches the
probe element whatever X is. DevTools' `CSS.getMatchedStylesForNode` then
reports the specificity Blink computed for X, after nesting. Adding `*` to a
parent list leaves its largest specificity unchanged.

Each rejected case is a selector Chromium drops: its rule is missing from
the matched rules. The native test
`openui-macros/tests/selector_specificity.rs` parses every case with the
`stylesheet!` selector parser and compares against the JSON this script
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
OUTPUT = ROOT / "docs/progress/evidence/selector-specificity-v1/chromium-windows.json"

# (parent chain, outermost first; selector). Keep in step with the native test.
ACCEPTED: list[tuple[list[str], str]] = [
    ([], s)
    for s in [
        "*",
        "div",
        "DIV",
        ".a",
        "#x",
        ".a.a",
        ":hover",
        ":hover:hover",
        "div.a#x",
        "a b",
        "a > b",
        "a + b",
        "a ~ b",
        "a > b ~ c + d",
        "#x .b:hover",
        ".a .b .c .d",
        "[data-state]",
        "[data-state=open]",
        "[data-state='open' i]",
        "[aria-x~=b]",
        "[lang|=en]",
        "[href^=x]",
        "[href$=x]",
        "[href*=x]",
        ":is(.a, #b) div",
        ":where(#a) .b",
        ":where(.a):is(.b)",
        ":not(.x)",
        ":not(#a, .b, c)",
        ":not(:is(#a))",
        ":has(> .q #r)",
        ":has(.a, + #b)",
        ":has(~ c)",
        "div:has(> img)",
        ":nth-child(odd)",
        ":nth-child(2n+1)",
        ":nth-child(2n + 1 of .y)",
        "span:not(.x):nth-child(2n+1 of .y, #z)",
        ":nth-child(-n+3 of :is(#a, b))",
        ":first-child",
        ":last-child",
        ":only-child",
        ":empty",
        ":root",
        ":focus",
        ":focus-visible",
        ":focus-within",
        ":active",
        ":disabled",
        ":enabled",
        ":checked",
        ":indeterminate",
        ":placeholder-shown",
        ":read-only",
        ":read-write",
        ":required",
        ":invalid",
        ":open",
        ":modal",
        ":HOVER",
        r".hover\:bg-muted:hover",
        r".data-\[state\=open\]\:p-2[data-state=open]",
        r".group:hover .group-hover\:opacity-100",
        r".peer:checked ~ .peer-checked\:block",
        r".\32xl\:p-4",
        r".w-1\/2",
        ":where(.space-y-2 > :not(:last-child))",
        r".dark .dark\:text-white",
        r".has-\[\>svg\]\:px-3:has(>svg)",
        r"[dir=rtl] .rtl\:mr-2",
        "a /* comment */ > b",
        ".c" * 300,
        "#i" * 300,
        "a " * 299 + "a",
        # Order-dependent clamping: the subject's :is() is added before the
        # 255 classes on its left, so its type-like part survives.
        ".c" * 255 + " :is(.d e)",
    ]
] + [
    ([".a, #b"], ".c"),
    ([".a, #b"], "&:hover"),
    ([".a, #b"], "> .c"),
    ([".a, #b"], "+ .c"),
    ([".a, #b"], "~ .c"),
    ([".a, #b"], ".c &"),
    ([".a, #b"], ":is(&) .c"),
    ([".a, #b"], ":where(&) .c"),
    ([".a, #b"], ":not(&)"),
    ([".a, #b"], "&"),
    ([".a, #b"], "& &"),
    ([".a, #b"], ".c, #d"),
    (["div"], "&.x"),
    (["div"], ":hover"),
    ([".a"], ".b:nth-child(2 of &)"),
    ([".a", ".b"], ".c"),
    ([".a", "&:hover"], "> .c"),
    ([".a, #b", ".c &"], "&:focus"),
]

REJECTED = [
    "a,",
    ",a",
    "a > > b",
    ".1a",
    "#1a",
    "[x=]",
    "[x y]",
    "[x=y z]",
    '[data-state="open" s]',
    ":nth-child(2 n)",
    ":nth-child(n+)",
    ":nth-child(2n+-1)",
    ":has(:has(a))",
    "a::before b",
    ":hover(",
    "a|b",
    ":not-a-pseudo-class",
    "div div.x div:",
]

def page() -> str:
    rules = []
    for index, (parents, selector) in enumerate(ACCEPTED):
        rules.append(nested_rule(parents, selector, f"--k{index}"))
    for index, selector in enumerate(REJECTED):
        rules.append(nested_rule([], selector, f"--r{index}"))
    css = "\n".join(rules)
    return (
        "<!DOCTYPE html><html><head><style>" + css + "</style></head>"
        "<body><div><div id=probe></div></div></body></html>"
    )


def nested_rule(parents: list[str], selector: str, marker: str) -> str:
    alternative = "&" if parents else "*"
    text = f"{selector}, {alternative} {{ {marker}: 1; }}"
    for parent in reversed(parents):
        text = f"{parent}, * {{ {text} }}"
    return text


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import windows_chrome

    with tempfile.TemporaryDirectory(prefix="openui-selector-oracle-") as directory:
        html = Path(directory) / "selectors.html"
        html.write_text(page(), encoding="utf-8")
        with windows_chrome.WindowsChrome(
            args.chrome or windows_chrome.DEFAULT_CHROME, 200, 200, 1.0
        ) as browser:
            browser.load(html)
            client = browser.client
            client.command("DOM.enable")
            client.command("CSS.enable")
            root = client.command("DOM.getDocument")["root"]["nodeId"]
            probe = client.command(
                "DOM.querySelector", {"nodeId": root, "selector": "#probe"}
            )["nodeId"]
            matched = client.command("CSS.getMatchedStylesForNode", {"nodeId": probe})
    by_marker: dict[str, dict] = {}
    for entry in matched["matchedCSSRules"]:
        rule = entry["rule"]
        if rule["origin"] != "regular":
            continue
        for prop in rule["style"]["cssProperties"]:
            if prop["name"].startswith(("--k", "--r")):
                # Every selector but the trailing `*` or `&` alternative.
                by_marker[prop["name"]] = rule["selectorList"]["selectors"][:-1]
    accepted = []
    for index, (parents, selector) in enumerate(ACCEPTED):
        reported = by_marker.get(f"--k{index}")
        if reported is None:
            raise SystemExit(f"Chromium dropped the accepted case {parents} {selector!r}")
        accepted.append(
            {
                "parents": parents,
                "selector": selector,
                "chromium_text": [entry["text"] for entry in reported],
                # One [a, b, c] per selector of the list.
                "specificity": [
                    [entry["specificity"][key] for key in "abc"] for entry in reported
                ],
            }
        )
    rejected = []
    for index, selector in enumerate(REJECTED):
        if f"--r{index}" in by_marker:
            raise SystemExit(f"Chromium accepted the rejected case {selector!r}")
        rejected.append(selector)
    result = {
        "chromium": windows_chrome.CHROMIUM_VERSION,
        "platform": "windows",
        "accepted": accepted,
        "rejected": rejected,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    print(f"{len(accepted)} accepted, {len(rejected)} rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
