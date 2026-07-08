#!/usr/bin/env python3
"""Agent memory store for open-ui.

A tiny, zero-dependency memory system so an agent (or human) can persist and
retrieve durable project context across sessions WITHOUT loading everything
into context at once.

Design goals (see docs/memory/README.md):
  * SHORT PER CHUNK  - one fact/decision/gotcha/status per file, kept small.
  * INDEXABLE        - a flat index.tsv (id, status, tags, title, updated,
                       path) that is cheap to grep/read.
  * QUICK SEARCH     - `mem.py search QUERY` scans metadata + bodies and prints
                       compact one-line hits you can then `get` individually.
  * SECOND NATURE    - one command to read, one to search, one to write; the
                       index is maintained automatically, never by hand.

Entries are plain Markdown files under docs/memory/entries/<id>-<slug>.md with
a simple `key: value` frontmatter block, so they stay git-friendly, reviewable
in PRs, and usable with plain grep/view/edit even without this CLI.

Usage:
  mem.py add   --title T --tags a,b --status active --body "..." [--refs r1,r2]
  mem.py add   --title T --tags a,b --body-file path      # or pipe body on stdin
  mem.py search QUERY [--tag T] [--status S] [-v]
  mem.py get   ID [ID ...]
  mem.py list  [--tag T] [--status S]
  mem.py update ID [--status S] [--add-tags a,b] [--add-refs r] [--append "text"] [--title T]
  mem.py reindex

Run with no arguments for a short cheat sheet.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import os
import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locations
# ---------------------------------------------------------------------------

def _mem_root() -> Path:
    env = os.environ.get("MEM_DIR")
    if env:
        return Path(env).resolve()
    # tools/memory/mem.py -> repo_root/docs/memory
    return (Path(__file__).resolve().parent.parent.parent / "docs" / "memory").resolve()

MEM_ROOT = _mem_root()
ENTRIES_DIR = MEM_ROOT / "entries"
INDEX_PATH = MEM_ROOT / "index.tsv"

FRONT_KEYS = ["id", "title", "tags", "status", "created", "updated", "refs"]
LIST_KEYS = {"tags", "refs"}
VALID_STATUS = {"active", "superseded", "archived"}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _today() -> str:
    return _dt.date.today().isoformat()


def _slugify(text: str, maxlen: int = 48) -> str:
    s = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
    return (s[:maxlen].rstrip("-")) or "entry"


def _split_list(value: str) -> list[str]:
    if not value:
        return []
    value = value.strip().lstrip("[").rstrip("]")
    return [p.strip() for p in re.split(r"[,\n]", value) if p.strip()]


def _normalize_id(raw: str) -> str:
    """Normalize/validate an id. Digits are zero-padded to 4 (so `17` and
    `0017` are the same entry). Custom ids may only use [A-Za-z0-9_-] — this
    rejects path separators / traversal like `../../outside`."""
    raw = raw.strip()
    if raw.isdigit():
        return f"{int(raw):04d}"
    if not re.fullmatch(r"[A-Za-z0-9_-]+", raw):
        raise ValueError(
            f"invalid id {raw!r}: use digits or [A-Za-z0-9_-] (no path separators)"
        )
    return raw


class Entry:
    def __init__(self, path: Path, meta: dict, body: str):
        self.path = path
        self.meta = meta
        self.body = body

    @property
    def id(self) -> str:
        return self.meta.get("id", "")

    def rel(self) -> str:
        try:
            return str(self.path.relative_to(MEM_ROOT.parent.parent))
        except ValueError:
            return str(self.path)

    def index_row(self) -> str:
        tags = " ".join(_split_list(self.meta.get("tags", "")))
        return "\t".join([
            self.meta.get("id", ""),
            self.meta.get("status", "active"),
            tags,
            self.meta.get("title", "").replace("\t", " "),
            self.meta.get("updated", ""),
            os.path.relpath(self.path, MEM_ROOT),
        ])

    def render(self) -> str:
        lines = ["---"]
        for k in FRONT_KEYS:
            if k in self.meta and self.meta[k] != "":
                lines.append(f"{k}: {self.meta[k]}")
        lines.append("---")
        text = "\n".join(lines) + "\n\n" + self.body.strip() + "\n"
        return text


def parse_entry(path: Path) -> Entry:
    raw = path.read_text(encoding="utf-8")
    meta: dict = {}
    body = raw
    if raw.startswith("---"):
        parts = raw.split("---", 2)
        if len(parts) >= 3:
            block, body = parts[1], parts[2]
            for line in block.strip().splitlines():
                if ":" in line:
                    k, v = line.split(":", 1)
                    meta[k.strip()] = v.strip()
    return Entry(path, meta, body.strip())


def load_entries() -> list[Entry]:
    if not ENTRIES_DIR.exists():
        return []
    entries = [parse_entry(p) for p in ENTRIES_DIR.glob("*.md")]
    entries.sort(key=lambda e: e.id)
    return entries


def next_id(entries: list[Entry]) -> str:
    nums = [int(e.id) for e in entries if e.id.isdigit()]
    return f"{(max(nums) + 1) if nums else 1:04d}"


def write_index(entries: list[Entry]) -> None:
    MEM_ROOT.mkdir(parents=True, exist_ok=True)
    header = "# id\tstatus\ttags\ttitle\tupdated\tpath\n"
    rows = "\n".join(e.index_row() for e in entries)
    INDEX_PATH.write_text(header + rows + ("\n" if rows else ""), encoding="utf-8")


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

def cmd_add(args) -> int:
    ENTRIES_DIR.mkdir(parents=True, exist_ok=True)
    body = args.body
    if args.body_file:
        body = Path(args.body_file).read_text(encoding="utf-8")
    if body is None and not sys.stdin.isatty():
        body = sys.stdin.read()
    if not body or not body.strip():
        print("error: empty body (use --body, --body-file, or pipe on stdin)", file=sys.stderr)
        return 2

    entries = load_entries()
    if args.id:
        try:
            eid = _normalize_id(args.id)
        except ValueError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
    else:
        eid = next_id(entries)
    if any(e.id == eid for e in entries):
        print(f"error: id {eid} already exists", file=sys.stderr)
        return 2
    status = args.status or "active"
    if status not in VALID_STATUS:
        print(f"error: status must be one of {sorted(VALID_STATUS)}", file=sys.stderr)
        return 2

    today = _today()
    meta = {
        "id": eid,
        "title": args.title.strip(),
        "tags": ", ".join(_split_list(args.tags or "")),
        "status": status,
        "created": today,
        "updated": today,
        "refs": ", ".join(_split_list(args.refs or "")),
    }
    path = ENTRIES_DIR / f"{eid}-{_slugify(args.title)}.md"
    if ENTRIES_DIR.resolve() not in path.resolve().parents:
        print("error: refusing to write outside the entries directory", file=sys.stderr)
        return 2
    entry = Entry(path, meta, body)
    path.write_text(entry.render(), encoding="utf-8")

    entries = load_entries()
    write_index(entries)
    print(f"added {eid}  {path.relative_to(MEM_ROOT.parent.parent)}")
    return 0


def cmd_reindex(_args) -> int:
    entries = load_entries()
    write_index(entries)
    print(f"reindexed {len(entries)} entries -> {INDEX_PATH.relative_to(MEM_ROOT.parent.parent)}")
    return 0


def _matches(entry: Entry, tag: str | None, status: str | None) -> bool:
    if status and entry.meta.get("status", "active") != status:
        return False
    if tag:
        tags = [t.lower() for t in _split_list(entry.meta.get("tags", ""))]
        if tag.lower() not in tags:
            return False
    return True


def cmd_search(args) -> int:
    try:
        rx = re.compile(args.query, re.IGNORECASE)
    except re.error:
        rx = re.compile(re.escape(args.query), re.IGNORECASE)

    hits = 0
    for e in load_entries():
        if not _matches(e, args.tag, args.status):
            continue
        haystack = "\n".join([
            e.meta.get("id", ""), e.meta.get("title", ""),
            e.meta.get("tags", ""), e.meta.get("refs", ""), e.body,
        ])
        if not rx.search(haystack):
            continue
        hits += 1
        tags = ",".join(_split_list(e.meta.get("tags", "")))
        print(f"{e.id}  [{e.meta.get('status','active')}] [{tags}]  {e.meta.get('title','')}")
        if args.verbose:
            for line in e.body.splitlines():
                if rx.search(line):
                    print(f"      | {line.strip()}")
    if hits == 0:
        print("(no matches)")
    return 0


def cmd_get(args) -> int:
    by_id = {e.id: e for e in load_entries()}
    rc = 0
    for i, eid in enumerate(args.ids):
        eid = eid.zfill(4) if eid.isdigit() else eid
        e = by_id.get(eid)
        if not e:
            print(f"error: no entry {eid}", file=sys.stderr)
            rc = 1
            continue
        if i:
            print("\n" + "=" * 70 + "\n")
        print(e.path.read_text(encoding="utf-8").rstrip())
    return rc


def cmd_list(args) -> int:
    rows = [e for e in load_entries() if _matches(e, args.tag, args.status)]
    if not rows:
        print("(no entries)")
        return 0
    for e in rows:
        tags = ",".join(_split_list(e.meta.get("tags", "")))
        print(f"{e.id}  [{e.meta.get('status','active')}] [{tags}]  {e.meta.get('title','')}")
    return 0


def cmd_update(args) -> int:
    by_id = {e.id: e for e in load_entries()}
    eid = args.id.zfill(4) if args.id.isdigit() else args.id
    e = by_id.get(eid)
    if not e:
        print(f"error: no entry {eid}", file=sys.stderr)
        return 1
    if args.status:
        if args.status not in VALID_STATUS:
            print(f"error: status must be one of {sorted(VALID_STATUS)}", file=sys.stderr)
            return 2
        e.meta["status"] = args.status
    if args.title:
        e.meta["title"] = args.title.strip()
    if args.add_tags:
        cur = _split_list(e.meta.get("tags", ""))
        for t in _split_list(args.add_tags):
            if t not in cur:
                cur.append(t)
        e.meta["tags"] = ", ".join(cur)
    if args.add_refs:
        cur = _split_list(e.meta.get("refs", ""))
        for r in _split_list(args.add_refs):
            if r not in cur:
                cur.append(r)
        e.meta["refs"] = ", ".join(cur)
    if args.append:
        e.body = e.body.rstrip() + "\n" + args.append.strip()
    e.meta["updated"] = _today()
    e.path.write_text(e.render(), encoding="utf-8")
    write_index(load_entries())
    print(f"updated {eid}")
    return 0


def _cheatsheet() -> None:
    print(__doc__)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(prog="mem.py", add_help=True,
                                     description="open-ui agent memory store")
    sub = parser.add_subparsers(dest="cmd")

    p_add = sub.add_parser("add", help="add a new memory entry")
    p_add.add_argument("--title", required=True)
    p_add.add_argument("--tags", default="")
    p_add.add_argument("--status", default="active")
    p_add.add_argument("--refs", default="")
    p_add.add_argument("--body", default=None)
    p_add.add_argument("--body-file", default=None)
    p_add.add_argument("--id", default=None)
    p_add.set_defaults(func=cmd_add)

    p_search = sub.add_parser("search", help="search entries (metadata + body)")
    p_search.add_argument("query")
    p_search.add_argument("--tag", default=None)
    p_search.add_argument("--status", default=None)
    p_search.add_argument("-v", "--verbose", action="store_true",
                          help="also print matching body lines")
    p_search.set_defaults(func=cmd_search)

    p_get = sub.add_parser("get", help="print full entry by id")
    p_get.add_argument("ids", nargs="+")
    p_get.set_defaults(func=cmd_get)

    p_list = sub.add_parser("list", help="list index (optionally filtered)")
    p_list.add_argument("--tag", default=None)
    p_list.add_argument("--status", default=None)
    p_list.set_defaults(func=cmd_list)

    p_upd = sub.add_parser("update", help="update an entry's metadata/body")
    p_upd.add_argument("id")
    p_upd.add_argument("--status", default=None)
    p_upd.add_argument("--title", default=None)
    p_upd.add_argument("--add-tags", default=None)
    p_upd.add_argument("--add-refs", default=None)
    p_upd.add_argument("--append", default=None)
    p_upd.set_defaults(func=cmd_update)

    p_re = sub.add_parser("reindex", help="regenerate index.tsv from entries")
    p_re.set_defaults(func=cmd_reindex)

    args = parser.parse_args(argv)
    if not getattr(args, "cmd", None):
        _cheatsheet()
        return 0
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
