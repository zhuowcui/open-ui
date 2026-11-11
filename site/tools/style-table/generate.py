#!/usr/bin/env python3
"""Build the style-property table for the site's Styling reference.

Reads `openui-style` source on this checkout (read-only) and runs the
`openui-style-table` binary next to this file, which calls
`openui_style::parse_literal` and `openui_style::value_matches_property` for
every property with a fixed set of candidate literals. Writes
`site/src/data/properties.json`, plus a Markdown table and raw TSVs under
`out/style-table/` for review.

Rerun from the repository root:
    cargo build --release --locked --manifest-path site/tools/style-table/Cargo.toml
    python3 site/tools/style-table/generate.py
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RUST = ROOT / "bindings" / "rust"
STYLE = RUST / "openui-style" / "src"
PROPERTY_RS = STYLE / "property.rs"
GENERATED_RS = STYLE / "generated_properties.rs"
SETTERS_RS = RUST / "openui" / "src" / "generated_style_setters.rs"
OUT = ROOT / "out" / "style-table"
OUT.mkdir(parents=True, exist_ok=True)
JSON_OUT = ROOT / "site" / "src" / "data" / "properties.json"
_target = Path(os.environ.get("CARGO_TARGET_DIR") or Path(__file__).resolve().parent / "target")
EXE = _target / "release" / ("openui-style-table.exe" if sys.platform == "win32" else "openui-style-table")


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


# ── 1. Metadata, from the probe (PROPERTY_METADATA itself) ──────────────────
meta_rows = []
for line in subprocess.run([str(EXE), "meta"], capture_output=True, text=True, check=True).stdout.splitlines():
    pid, css, variant, rust_type, kind, initial, inherited, invalidation, interp = line.split("\t")
    meta_rows.append(dict(id=int(pid), css=css, variant=variant, rust_type=rust_type, kind=kind,
                          initial=initial, inherited=inherited == "true", interp=interp))

gen_lines = GENERATED_RS.read_text(encoding="utf-8").splitlines()
meta_line = {}
for i, text in enumerate(gen_lines, 1):
    m = re.match(r'\s+css_name: "([^"]+)",', text)
    if m and m.group(1) not in meta_line:
        meta_line[m.group(1)] = i

setter = {}
for i, text in enumerate(SETTERS_RS.read_text(encoding="utf-8").splitlines(), 1):
    m = re.match(r"\s+pub fn (set_\w+)\(&self, value: (.+)\) -> Result", text)
    if m:
        nxt = None
        setter_line = i
        setter[setter_line] = (m.group(1), m.group(2))
setter_by_variant = {}
setter_lines = SETTERS_RS.read_text(encoding="utf-8").splitlines()
for i, text in enumerate(setter_lines, 1):
    m = re.match(r"\s+pub fn (set_\w+)\(&self, value: (.+)\) -> Result", text)
    if m and i < len(setter_lines):
        v = re.search(r"StyleProperty::(\w+)", setter_lines[i])
        if v and v.group(1) not in setter_by_variant:
            setter_by_variant[v.group(1)] = (m.group(1), m.group(2), i)

# ── 2. Literal parser arms ──────────────────────────────────────────────────
prop_lines = PROPERTY_RS.read_text(encoding="utf-8").splitlines()


def function_body(lines, signature):
    start = next(i for i, t in enumerate(lines) if t.startswith(signature))
    end = next(i for i in range(start + 1, len(lines)) if lines[i] == "}")
    return start, end


def arms(lines, signature, path, prefix):
    """Map property variant -> (file:line, quoted keywords) for one parser."""
    start, end = function_body(lines, signature)
    arm_re = re.compile(r"^ {8}(\| )?" + prefix + r"(\w+)")
    result = {}
    current: list[str] = []
    body: list[str] = []
    first_line = 0

    def flush():
        if current:
            joined = "\n".join(body)
            words = re.findall(r'"((?:[^"\\]|\\.)*)"', joined)
            words += called_words(joined)
            words = [w for w in words if "{" not in w and w not in ("validated typed style literal",)]
            for v in current:
                result.setdefault(v, (f"{rel(path)}:{first_line}", words))

    collecting_names = False
    for idx in range(start + 1, end):
        text = lines[idx]
        m = arm_re.match(text)
        if m:
            if not collecting_names:
                flush()
                current, body, first_line = [], [], idx + 1
            current.extend(re.findall(prefix + r"(\w+)", text.split("=>")[0]))
            collecting_names = "=>" not in text
            body.append(text.split("=>", 1)[1] if "=>" in text else "")
            continue
        collecting_names = False
        body.append(text)
    flush()
    return result


# Quoted keywords inside helper functions that the arms call (up to three levels).
HELPER_FILES = [PROPERTY_RS, GENERATED_RS, STYLE / "grid_literal.rs", STYLE / "transform_literal.rs",
                STYLE / "color_function.rs"]
TOP_PARSERS = {"parse_literal", "parse_typography_literal", "parse_renderer_author_literal"}
helper_words: dict[str, list[str]] = {}
helper_calls: dict[str, set[str]] = {}
for path in HELPER_FILES:
    text_lines = path.read_text(encoding="utf-8").splitlines()
    for i, t in enumerate(text_lines):
        m = re.match(r"^(?:pub(?:\(crate\))? )?fn (\w+)\(", t)
        if not m or m.group(1) in TOP_PARSERS:
            continue
        end = next((j for j in range(i + 1, len(text_lines)) if text_lines[j] == "}"), len(text_lines))
        body = "\n".join(text_lines[i + 1:end])
        key = m.group(1) if path in (PROPERTY_RS, GENERATED_RS) else f"{path.stem}::{m.group(1)}"
        helper_words[key] = re.findall(r'"((?:[^"\\]|\\.)*)"', body)
        helper_calls[key] = set(re.findall(r"\b(\w+)\(", body))


def called_words(body: str) -> list[str]:
    names = set(re.findall(r"\b(\w+)\(", body))
    names |= {f"grid_literal::{n}" for n in re.findall(r"grid_literal::(\w+)", body)}
    if "transform_literal::parse" in body:
        names.add("transform_literal::parse")
    seen: set[str] = set()
    frontier = list(names)
    words: list[str] = []
    for _ in range(3):
        nxt: list[str] = []
        for n in frontier:
            if n in seen or n not in helper_words:
                continue
            seen.add(n)
            words += helper_words[n]
            prefix = n.split("::")[0] + "::" if "::" in n else ""
            nxt += [prefix + c for c in helper_calls[n]] + list(helper_calls[n])
        frontier = nxt
    return [w for w in words if "{" not in w]


literal_arms = {}
literal_arms.update(arms(prop_lines, "pub fn parse_literal(", PROPERTY_RS, "P::"))
typo = arms(prop_lines, "fn parse_typography_literal(", PROPERTY_RS, "P::")
renderer = arms(gen_lines, "fn parse_renderer_author_literal(", GENERATED_RS, "StyleProperty::")
for table in (typo, renderer):
    for k, v in table.items():
        literal_arms.setdefault(k, v)

# ── 3. From<T> for StyleValue (what `style:prop={expr}` can convert) ─────────
from_types = {}
for m in re.finditer(r"impl_(style|typography)_value!\(([^,]+), (\w+)\);", PROPERTY_RS.read_text(encoding="utf-8")):
    kind, ty, variant = m.groups()
    from_types[ty.strip()] = variant if kind == "style" else f"Typography({variant}"

# ── 4. Probe candidates ─────────────────────────────────────────────────────
GENERIC = [
    "0", "1", "2", "1.5", "-1", "10px", "1.5em", "2rem", "50%", "10vw", "10vh", "10vmin", "2ch", "1ex",
    "1lh", "1fr", "calc(10px + 5%)", "min(10px, 5%)", "clamp(1px, 2vw, 3px)", "auto", "none", "normal",
    "inherit", "initial", "unset", "var(--x)", "red", "rebeccapurple", "#3b82f6", "#fff8", "rgb(255 0 0)",
    "rgb(255, 0, 0)", "rgba(0, 0, 0, 0.5)", "rgb(255 0 0 / 50%)", "rgb(255 0 0 / 0.5)", "RED", "#FFF", "#12345", "hsl(210 50% 40%)", "hsl(210, 50%, 40%)",
    "hwb(210 10% 20%)", "lab(50% 20 30)", "lch(50% 30 120)", "oklab(0.7 0.1 -0.1)", "oklch(0.7 0.1 200)",
    "color(display-p3 1 0 0)", "color-mix(in srgb, red 50%, blue)", "light-dark(white, black)",
    "currentcolor", "transparent", "1px solid red", "1px solid #000", "1px solid rgb(0,0,0)",
    "2px dashed currentcolor", "linear-gradient(red, blue)", "url(a.png)", "0 1px 2px red",
    "rotate(45deg)", "translate(10px, 20px)", "scale(1.5)", "repeat(3, 1fr)", "1fr 2fr", "200px 1fr",
    "auto 1fr", "minmax(100px, 1fr)", "span 2", "1 / 3", "2 / span 2", "16px/1.5 sans-serif",
    "bold 16px Inter", "italic 600 14px/20px system-ui", "sans-serif", '"Inter", sans-serif',
    "\"wght\" 600", "\"liga\" 0", "\"tnum\"", "italic", "oblique 10deg", "bold", "600", "16/9", "4 / 3",
    "10px 20px", "1px 2px 3px 4px", "center", "start", "space-between", "underline", "hidden", "solid",
    "border-box", "10px 10px", "0 0", "50% 50%", "content", "size", "inline-size", "stable both-edges", "safe center", "unsafe end", "first baseline", "last baseline",
    "start center", "underline overline", "2px wavy red underline", "common-ligatures tabular-nums",
]


def candidates_for(variant: str) -> list[str]:
    words = literal_arms.get(variant, (None, []))[1]
    seen, out = set(), []
    for c in list(GENERIC) + words:
        if c not in seen and "\t" not in c and "\n" not in c:
            seen.add(c)
            out.append(c)
    return out


cand_file = OUT / "candidates.tsv"
with cand_file.open("w", encoding="utf-8", newline="\n") as f:
    for row in meta_rows:
        for c in candidates_for(row["variant"]):
            f.write(f"{row['css']}\t{c}\n")

probe = subprocess.run([str(EXE), "probe", str(cand_file)], capture_output=True, text=True,
                       encoding="utf-8", check=True).stdout
(OUT / "probe-results.tsv").write_text(probe, encoding="utf-8")
accepted = defaultdict(list)   # css -> [(candidate, applies, value)]
for line in probe.splitlines():
    css, cand, status, applies, value = line.split("\t")
    if status == "ok":
        accepted[css].append((cand, applies == "true", value))

# ── 5. Grouping ─────────────────────────────────────────────────────────────
GROUPS = [
    ("Layout and box model", r"^(display|width|height|min-|max-|margin|padding|box-sizing|aspect-ratio|"
     r"contain|container|content-visibility|visibility|box-decoration-break|-webkit-box-orient|"
     r"vertical-align$|object-|float|clear)"),
    ("Flexbox", r"^(flex|order$)"),
    ("Box alignment (flex and grid)", r"^(align-|justify-|gap|row-gap|column-gap)"),
    ("Grid", r"^grid"),
    ("Positioning", r"^(position|top$|right$|bottom$|left$|z-index|anchor-name|position-)"),
    ("Tables", r"^(border-collapse|border-spacing|caption-side|empty-cells|table-layout)"),
    ("Multi-column and fragmentation", r"^(column-|break-|orphans|widows)"),
    ("Typography", r"^(font|line-|letter-|word-|text-|white-space|tab-size|hyphen|direction|unicode-bidi|"
     r"writing-mode|ruby-|hanging-|initial-letter|-webkit-font|-webkit-text|block-ellipsis|locale|quotes|"
     r"overflow-wrap|color$)"),
    ("Color and background", r"^(background|mask)"),
    ("Borders, outline and radius", r"^(border|outline)"),
    ("Effects (opacity, shadow, filter, transform, clip)", r"^(opacity|box-shadow|filter|transform|clip-path)"),
    ("Overflow and scrolling", r"^(overflow|scroll|scrollbar)"),
    ("Lists and generated content", r"^(list-style|content$|counter-)"),
    ("Interaction", r"^(cursor|pointer-events|resize)"),
    ("Shapes", r"^shape-"),
    ("Other", r"."),
]


def group_of(css: str) -> str:
    if css == "column-count":
        return "Multi-column and fragmentation"
    for name, pattern in GROUPS:
        if re.search(pattern, css):
            return name
    return "Other"


def is_ident(t: str) -> bool:
    return re.fullmatch(r"[A-Za-z_]\w*", t) is not None


typed_ok = set()
for line in subprocess.run([str(EXE), "typed"], capture_output=True, text=True, check=True).stdout.splitlines():
    css, tyname, ok = line.split("\t")
    if ok == "true":
        typed_ok.add((css, tyname))
CANNED = {"f32", "i32", "LengthValue", "Edges<LengthValue>", "Color", "StyleColor"}


def expr_support(row) -> str:
    """Whether `style:prop={expr}` compiles and is admitted (view.rs dynamic_type_tokens)."""
    ty, css = row["rust_type"], row["css"]
    if ty != "Edges<LengthValue>" and not is_ident(ty):
        return f"no: `view!` cannot name `{ty}`"
    if ty != "Edges<LengthValue>" and ty not in from_types:
        return f"no: `{ty}` has no `Into<StyleValue>`"
    if ty in CANNED:
        return f"yes, `{ty}`" if (css, ty) in typed_ok else f"compiles, rejected at runtime (`{ty}`)"
    variant = from_types[ty]
    lits = accepted.get(css, [])
    if any(v.startswith(variant) for _, ok, v in lits if ok):
        return f"yes, `{ty}`"
    return f"`{ty}` compiles; admission unverified"


LENGTH_SET = ["0", "10px", "50%", "1.5em", "2rem", "10vw", "10vh", "10vmin", "2ch", "1ex", "1lh", "auto", "none"]
COLOR_SET = ["red", "RED", "rebeccapurple", "#3b82f6", "#fff8", "#FFF", "rgb(255 0 0)", "rgb(255, 0, 0)",
             "rgba(0, 0, 0, 0.5)", "rgb(255 0 0 / 0.5)", "oklab(0.7 0.1 -0.1)", "oklch(0.7 0.1 200)",
             "color-mix(in srgb, red 50%, blue)", "currentcolor", "transparent"]
P = "\\|"
GRAMMAR = {
    "font-family": "comma list; generic keywords (`serif`, `sans-serif`, `monospace`, `cursive`, `fantasy`, "
                   "`system-ui`, `math`, `emoji`, `fangsong`, `ui-serif`, `ui-sans-serif`, `ui-monospace`, "
                   "`ui-rounded`); any other item is a family name (quotes stripped, not otherwise validated)",
    "font": f"`[normal{P}italic{P}oblique{P}small-caps{P}<weight>{P}<stretch>]* <px size>[/<line-height>] <family list>`, "
            "e.g. `bold 16px Inter`, `16px/1.5 sans-serif`",
    "font-feature-settings": f"`normal` or comma list of `<4-char tag> [on{P}off{P}<integer>]`, e.g. `\"tnum\"`, "
                             "`\"liga\" 0`; any 4 printable ASCII characters count as a tag",
    "font-variation-settings": "`normal` or comma list of `<4-char tag> <number>`, e.g. `\"wght\" 600`",
    "font-language-override": "`normal` or a 4-character tag",
    "hyphenate-character": "`auto` or a string (quotes stripped)",
    "block-ellipsis": "`auto`, `no-ellipsis` or any string",
    "font-palette": "`normal`, `light`, `dark` or a `--name` registered with `Document::register_font_palette_values`",
    "font-size-adjust": f"`none`, `from-font`, `<number>` or `<ex-height{P}cap-height{P}ch-width{P}ic-width{P}ic-height> <number>`",
    "font-stretch": "keyword (`ultra-condensed` … `ultra-expanded`) or `<percent>`",
    "font-style": "`normal`, `italic`, `oblique`, `oblique <n>deg`",
    "font-weight": "`normal`, `bold` or a number 1–1000 (no `bolder`/`lighter`)",
    "line-height": "`normal`, a number, px, %, or em/rem/vw/vh/ch/ex/lh",
    "transform": "`none` or space-separated `translate()`, `translateX()`, `translateY()`, `scale()`, `scaleX()`, "
                 "`scaleY()`, `rotate()`, `matrix()` with comma-separated arguments; no skew, 3D or math functions",
    "text-shadow": "`none` or comma list of `<x> <y> [<blur>]` (px) plus an optional one-token color",
    "border": "`none` or exactly `<px width> <style> <color>`; the color must be one token (no spaces inside `rgb()`)",
    "grid-template-columns": "space-separated tracks: px, %, `<n>fr`, `auto`, `min-content`, `max-content`, "
                             "`minmax(a, b)`, `fit-content(...)`, or `none`; no `repeat()`, named lines or `subgrid`",
    "text-decoration": "space-separated line keywords, style keyword, thickness and a one-token color",
    "text-emphasis-style": f"`none` or `[filled{P}open] [dot{P}circle{P}double-circle{P}triangle{P}sesame{P}<one character>]`",
    "hyphenate-limit-chars": "`auto` or 1–3 integers/`auto`",
    "initial-letter": "`normal` or `<size> [<sink>]`",
    "tab-size": "integer (spaces) or px length",
    "line-clamp": "`none`, `auto` or an integer",
    "opacity": "a bare number (`%` not accepted)",
    "flex-grow": "a bare number",
    "flex-shrink": "a bare number",
    "z-index": "an integer (`auto` not accepted)",
    "order": "an integer",
    "border-spacing": "one or two non-negative px lengths",
    "letter-spacing": "`normal` or a px length",
    "word-spacing": "`normal` or a px length",
    "column-count": "`auto` or a positive integer",
    "orphans": "a non-negative integer",
    "widows": "a non-negative integer",
    "overflow-clip-margin": "a bare number (px)",
    "shape-image-threshold": "a bare number",
}
for name in ("border-top", "border-right", "border-bottom", "border-left"):
    GRAMMAR[name] = GRAMMAR["border"]
GRAMMAR["grid-template-rows"] = GRAMMAR["grid-template-columns"]
for name in ("grid-auto-columns", "grid-auto-rows"):
    GRAMMAR[name] = GRAMMAR["grid-template-columns"].replace(", or `none`", "")
for name in ("border-top-width", "border-right-width", "border-bottom-width", "border-left-width",
             "outline-width", "outline-offset", "column-rule-width"):
    GRAMMAR[name] = "a bare integer in px (`2`, not `2px`)"
GRAMMAR["filter-blur"] = "a bare number (blur radius, px)"
GRAMMAR["filter-grayscale"] = "a bare number (amount)"


def literal_summary(css: str, applied: list[str], unapplied: list[str]) -> str:
    if css in GRAMMAR:
        return GRAMMAR[css]
    remaining = list(applied)
    parts = []
    lengths = [v for v in LENGTH_SET if v in remaining]
    if len(lengths) >= 5:
        missing = [v for v in LENGTH_SET if v not in remaining]
        parts.append("**L**" + (f" except {', '.join(f'`{m}`' for m in missing)}" if missing else ""))
        remaining = [v for v in remaining if v not in LENGTH_SET]
    colors = [v for v in COLOR_SET if v in remaining]
    if len(colors) >= 8:
        missing = [v for v in COLOR_SET if v not in remaining]
        parts.append("**C**" + (f" except {', '.join(f'`{m}`' for m in missing)}" if missing else ""))
        remaining = [v for v in remaining if v not in COLOR_SET]
    if remaining:
        parts.append(shorten(remaining, 16))
    if unapplied:
        parts.append("parsed but rejected on apply: " + ", ".join(f"`{u}`" for u in unapplied))
    return "; ".join(parts)


def md(s: str) -> str:
    return s.replace("|", "\\|")


def shorten(values: list[str], limit: int = 14) -> str:
    shown = values[:limit]
    text = ", ".join(f"`{md(v)}`" for v in shown)
    if len(values) > limit:
        text += f", … (+{len(values) - limit})"
    return text


rows_by_group = defaultdict(list)
for row in meta_rows:
    rows_by_group[group_of(row["css"])].append(row)

lines = [
    "<!-- generated by site/tools/style-table/generate.py; rerun it, do not edit by hand -->",
    "",
    f"{len(meta_rows)} properties in `PROPERTY_METADATA` ({rel(GENERATED_RS)}:{meta_line['display'] - 3}). "
    "\"Literal\" lists the probe candidates that `parse_literal` accepted and that "
    "`value_matches_property` then applied to an initial `ComputedStyle`. Keyword candidates come from "
    "each property's parser arm, so the list is complete for keyword properties and a sample for "
    "length, color and number grammars. **L** = the length grammar `0`, `auto`, `none`, or a number with "
    "`px % em rem vw vh vmin vmax ch ex lh` (no `calc()`); **C** = the color grammar (hex, `rgb()`/`rgba()`, "
    "named colors incl. `transparent`, `currentcolor` where supported, `oklab()`, `oklch()`, `color-mix()`). "
    "The `{expr}` column says whether `style:prop={expr}` in `view!` compiles and is admitted by the Engine.",
    "",
]
order = [g for g, _ in GROUPS]
counts = {"literal": 0, "typed-only": 0}
for group in order:
    rows = rows_by_group.get(group)
    if not rows:
        continue
    lines += [f"### {group}", "",
              "| Property | Typed setter (value type) | Literal accepted (`style:prop=\"…\"`) | `style:prop={expr}` | Inherited | Initial | Interpolation | Source |",
              "|---|---|---|---|:---:|---|---|---|"]
    for row in rows:
        lits = accepted.get(row["css"], [])
        applied = [c for c, ok, _ in lits if ok]
        unapplied = [c for c, ok, _ in lits if not ok]
        if lits:
            counts["literal"] += 1
            literal = literal_summary(row["css"], applied, unapplied)
        else:
            counts["typed-only"] += 1
            literal = "**typed value only**"
        s = setter_by_variant.get(row["variant"])
        setter_txt = f"`{s[0]}({md(s[1])})`" if s else "unknown"
        arm = literal_arms.get(row["variant"], (None,))[0]
        src = f"meta {rel(GENERATED_RS).split('/')[-1]}:{meta_line[row['css']]}"
        if arm:
            src += f"; parser {arm.split('/')[-1]}"
        if s:
            src += f"; setter {SETTERS_RS.name}:{s[2]}"
        lines.append(
            f"| `{row['css']}` | {setter_txt} | {literal} | {md(expr_support(row))} | "
            f"{'yes' if row['inherited'] else 'no'} | `{md(row['initial'])}` | {row['interp']} | {src} |")
    lines.append("")
lines.insert(4, f"Literal-capable properties: {counts['literal']}; typed-value-only: {counts['typed-only']}.")
lines.insert(5, "")
(OUT / "property-table.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

# The site's table: one row per property, grouped as above.
site_rows = []
for group in order:
    for row in rows_by_group.get(group, []):
        lits = accepted.get(row["css"], [])
        applied = [c for c, ok, _ in lits if ok]
        unapplied = [c for c, ok, _ in lits if not ok]
        s = setter_by_variant.get(row["variant"])
        site_rows.append({
            "property": row["css"],
            "group": group,
            "setter": f"{s[0]}({s[1]})" if s else None,
            "literal": literal_summary(row["css"], applied, unapplied) if lits else None,
            "expr": expr_support(row),
            "inherited": row["inherited"],
            "initial": None if row["initial"] == "renderer-initial" else row["initial"],
        })
JSON_OUT.parent.mkdir(parents=True, exist_ok=True)
JSON_OUT.write_text(json.dumps({"groups": order, "properties": site_rows}, indent=2) + "\n",
                    encoding="utf-8", newline="\n")
print(f"wrote {rel(JSON_OUT)}")

# Machine-readable summary.
with (OUT / "property-summary.tsv").open("w", encoding="utf-8", newline="\n") as f:
    for row in meta_rows:
        lits = accepted.get(row["css"], [])
        f.write("\t".join([str(row["id"]), row["css"], group_of(row["css"]), row["rust_type"],
                           "literal" if lits else "typed-only", expr_support(row),
                           " ; ".join(c for c, ok, _ in lits if ok)]) + "\n")
print(f"{len(meta_rows)} properties; literal {counts['literal']}; typed-only {counts['typed-only']}")
print(f"wrote {rel(OUT / 'property-table.md')}")
