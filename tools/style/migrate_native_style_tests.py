#!/usr/bin/env python3
"""One-shot migration for low-level native layout style setup.

Production author changes use the generated property API. Layout tests still
need to construct already-resolved snapshots, but must do so through the DOM's
closure boundary instead of retaining a mutable ComputedStyle reference.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUST = ROOT / "bindings/rust"
DECLARATION = re.compile(
    r"^(?P<indent>\s*)let (?P<binding>[a-zA-Z_][a-zA-Z0-9_]*) = "
    r"(?P<document>[a-zA-Z_][a-zA-Z0-9_]*)\.node_mut\((?P<node>.+)\)"
    r"\.style_mut\(\);\s*$"
)
CHAIN = re.compile(
    r"^(?P<indent>\s*)(?P<document>[a-zA-Z_][a-zA-Z0-9_]*)"
    r"\.node_mut\((?P<node>.+)\)\.style_mut\(\)\.(?P<tail>.*)$"
)
DIRECT = re.compile(
    r"^(?P<indent>\s*)(?P<document>[a-zA-Z_][a-zA-Z0-9_]*)"
    r"\.node_mut\((?P<node>.+)\)\.style\.(?P<tail>.*)$"
)
WHOLE_STYLE = re.compile(
    r"^(?P<indent>\s*)(?P<document>[a-zA-Z_][a-zA-Z0-9_]*)"
    r"\.node_mut\((?P<node>.+)\)\.style\s*=\s*(?P<tail>.*)$"
)
BORROW_DECLARATION = re.compile(
    r"^(?P<indent>\s*)let (?P<binding>[a-zA-Z_][a-zA-Z0-9_]*) = &mut "
    r"(?P<document>[a-zA-Z_][a-zA-Z0-9_]*)\.node_mut\((?P<node>.+)\)"
    r"\.style;\s*$"
)


def brace_delta(line: str) -> int:
    source = line.split("//", 1)[0]
    source = re.sub(r'"(?:\\.|[^"\\])*"', '""', source)
    return source.count("{") - source.count("}")


def migrate_borrow_blocks(text: str) -> str:
    lines = text.splitlines(keepends=True)
    edits = []
    for index, line in enumerate(lines):
        match = BORROW_DECLARATION.match(line.rstrip("\n"))
        if not match or index == 0 or lines[index - 1].strip() != "{":
            continue
        depth = 0
        close = None
        for cursor in range(index - 1, len(lines)):
            depth += brace_delta(lines[cursor])
            if depth == 0:
                close = cursor
                break
        if close is None:
            raise ValueError(f"unclosed style borrow block on line {index + 1}")
        edits.append((index - 1, index, close, match))
    for open_line, declaration, close, match in reversed(edits):
        indent = match["indent"][:-4] if match["indent"].endswith("    ") else match["indent"]
        lines[open_line] = (
            f"{indent}{match['document']}.update_resolved_style("
            f"{match['node']}, |{match['binding']}| {{\n"
        )
        lines[close] = f"{indent}}});\n"
        del lines[declaration]
    return "".join(lines)


def statement_end(lines: list[str], start: int) -> int:
    """Find a Rust statement terminator outside strings and line comments."""
    quoted = False
    escaped = False
    depth = 0
    for index in range(start, len(lines)):
        line = lines[index]
        position = 0
        while position < len(line):
            char = line[position]
            if not quoted and char == "/" and line[position:position + 2] == "//":
                break
            if char == '"' and not escaped:
                quoted = not quoted
            if not quoted and char in "([{":
                depth += 1
            elif not quoted and char in ")]}":
                depth -= 1
            if not quoted and char == ";" and depth == 0:
                return index
            escaped = char == "\\" and not escaped
            if char != "\\":
                escaped = False
            position += 1
    raise ValueError(f"unterminated statement beginning on line {start + 1}")


def migrate_text(text: str) -> str:
    # Collapse calls accidentally nested by the earliest version of this
    # one-shot migrator. Both closures target the same resolved field set.
    text = text.replace("|style| style.update_derived(|style| ", "|style| ")
    text = migrate_borrow_blocks(text)
    lines = text.splitlines(keepends=True)
    output: list[str] = []
    index = 0
    while index < len(lines):
        declaration = DECLARATION.match(lines[index].rstrip("\n"))
        if declaration:
            binding = declaration["binding"]
            end = index
            cursor = index + 1
            while cursor < len(lines):
                stripped = lines[cursor].lstrip()
                if not stripped.startswith(f"{binding}."):
                    break
                end = statement_end(lines, cursor)
                cursor = end + 1
            if end == index:
                raise ValueError(
                    f"style binding {binding!r} has no contiguous mutation statements"
                )
            indent = declaration["indent"]
            output.append(
                f"{indent}{declaration['document']}.update_resolved_style("
                f"{declaration['node']}, |{binding}| {{\n"
            )
            output.extend(lines[index + 1:end + 1])
            output.append(f"{indent}}});\n")
            index = end + 1
            continue

        chained = CHAIN.match(lines[index].rstrip("\n"))
        direct = DIRECT.match(lines[index].rstrip("\n"))
        if direct and direct["tail"].startswith(("update_derived(", "derive(")):
            direct = None
        chained = chained or direct
        if chained:
            end = statement_end(lines, index)
            tail = chained["tail"] + "\n" + "".join(lines[index + 1:end + 1])
            semicolon = tail.rfind(";")
            if semicolon < 0:
                raise ValueError("chained style mutation has no terminator")
            tail = tail[:semicolon] + ");" + tail[semicolon + 1:]
            output.append(
                f"{chained['indent']}{chained['document']}.update_resolved_style("
                f"{chained['node']}, |style| style.{tail}"
            )
            if not output[-1].endswith("\n"):
                output[-1] += "\n"
            index = end + 1
            continue

        whole_style = WHOLE_STYLE.match(lines[index].rstrip("\n"))
        if whole_style:
            end = statement_end(lines, index)
            tail = whole_style["tail"] + "\n" + "".join(lines[index + 1:end + 1])
            semicolon = tail.rfind(";")
            if semicolon < 0:
                raise ValueError("resolved style installation has no terminator")
            tail = tail[:semicolon] + ");" + tail[semicolon + 1:]
            output.append(
                f"{whole_style['indent']}{whole_style['document']}.install_resolved_style("
                f"{whole_style['node']}, {tail}"
            )
            if not output[-1].endswith("\n"):
                output[-1] += "\n"
            index = end + 1
            continue

        output.append(lines[index])
        index += 1
    return "".join(output)


def candidate_paths() -> list[Path]:
    return sorted(
        path for path in RUST.rglob("*.rs")
        if any(
            marker in path.read_text(encoding="utf-8")
            for marker in ("style_mut(", ".node_mut(", "&mut ComputedStyle")
        )
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = candidate_paths()
    changed = []
    for path in paths:
        before = path.read_text(encoding="utf-8")
        after = migrate_text(before)
        if "style_mut(" in after:
            raise SystemExit(f"unmigrated style_mut call: {path.relative_to(ROOT)}")
        if before != after:
            changed.append(path)
            if not args.check:
                path.write_text(after, encoding="utf-8")
    if args.check and changed:
        raise SystemExit(
            "native style test migration required: "
            + ", ".join(str(path.relative_to(ROOT)) for path in changed)
        )
    print(f"native style migration: files={len(paths)} changed={len(changed)}")


if __name__ == "__main__":
    main()
