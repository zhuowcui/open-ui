#!/usr/bin/env python3
"""Rewrite rustc-reported ComputedStyle assignments to derived closures."""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUST = ROOT / "bindings/rust"


def statement_end(data: bytes, start: int) -> int:
    depth = 0
    quoted = False
    escaped = False
    index = start
    while index < len(data):
        char = data[index]
        if not quoted and data[index:index + 2] == b"//":
            newline = data.find(b"\n", index + 2)
            index = len(data) if newline < 0 else newline + 1
            continue
        if char == ord('"') and not escaped:
            quoted = not quoted
        if not quoted:
            if char in b"([{":
                depth += 1
            elif char in b")]}":
                if depth == 0:
                    return index
                depth -= 1
            elif char in b";," and depth == 0:
                return index
        escaped = quoted and char == ord('\\') and not escaped
        if char != ord('\\'):
            escaped = False
        index += 1
    raise ValueError("unterminated ComputedStyle assignment")


def diagnostic_starts(path: Path) -> dict[Path, set[int]]:
    result: dict[Path, set[int]] = defaultdict(set)
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("reason") != "compiler-message":
            continue
        message = event["message"]
        code = (message.get("code") or {}).get("code")
        if code != "E0594" or "ComputedStyle" not in message["message"]:
            continue
        for span in message["spans"]:
            if span.get("is_primary"):
                result[RUST / span["file_name"]].add(int(span["byte_start"]))
    return result


def replacement(statement: bytes) -> bytes:
    text = statement.decode("utf-8")
    match = re.fullmatch(
        r"(?s)((?:self|node)\.style|[a-zA-Z_][a-zA-Z0-9_]*)\.(.*)", text
    )
    if not match:
        raise ValueError(f"unsupported ComputedStyle statement: {text!r}")
    target, tail = match.groups()
    tail = re.sub(rf"\b{re.escape(target)}\.", "computed.", tail)
    return f"{target}.update_derived(|computed| computed.{tail})".encode()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics", type=Path)
    args = parser.parse_args()
    starts_by_path = diagnostic_starts(args.diagnostics)
    replacements = 0
    for path, starts in sorted(starts_by_path.items()):
        data = path.read_bytes()
        edits = []
        for start in starts:
            end = statement_end(data, start)
            edits.append((start, end, replacement(data[start:end])))
        for start, end, value in sorted(edits, reverse=True):
            data = data[:start] + value + data[end:]
        path.write_bytes(data)
        replacements += len(edits)
    print(f"computed style diagnostics migration: files={len(starts_by_path)} edits={replacements}")


if __name__ == "__main__":
    main()
