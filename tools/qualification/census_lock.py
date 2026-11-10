#!/usr/bin/env python3
"""Allow one full census at a time on this machine.

A full census takes about 75 minutes, 16 jobs and half the host's memory, so
two at once slow both and can exhaust WSL. Every worktree, from Windows or
WSL, shares one lock file in the main checkout's ``out/`` directory
(``E:\\open-ui\\out\\census.lock`` is ``/mnt/e/open-ui/out/census.lock``).
The lock is an exclusively created file, which both operating systems see;
OS file locks do not cross the WSL boundary.

    python3 tools/qualification/census_lock.py run [--wait] -- <census command>
    python3 tools/qualification/census_lock.py status
    python3 tools/qualification/census_lock.py release --force

``run`` takes the lock, runs the command, releases the lock and exits with the
command's status. A lock left by a dead process on the same system is
reclaimed automatically; one from the other system needs ``release --force``
after checking that its census has ended.
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import platform
import re
import socket
import subprocess
import sys
import time
from pathlib import Path

POLL_SECONDS = 30


def native_path(text: str) -> Path:
    """Read a ``gitdir:`` path written by Windows git from either system."""
    match = re.match(r"^([A-Za-z]):[\\/](.*)$", text)
    if match and os.name != "nt":
        drive, rest = match.groups()
        return Path("/mnt") / drive.lower() / rest.replace("\\", "/")
    return Path(text)


def main_checkout(start: Path) -> Path:
    """Find the main checkout from a checkout or any of its worktrees.

    Git is not used: WSL git cannot follow a worktree made by Windows git.
    """
    for directory in [start, *start.parents]:
        dot_git = directory / ".git"
        if dot_git.is_dir():
            return directory
        if dot_git.is_file():
            gitdir = dot_git.read_text(encoding="utf-8").strip().removeprefix("gitdir:").strip()
            # <main>/.git/worktrees/<name>
            return native_path(gitdir).parent.parent.parent
    raise SystemExit(f"census_lock: {start} is not inside a git checkout")


def default_lock_path() -> Path:
    return main_checkout(Path(__file__).resolve().parent) / "out" / "census.lock"


def system_name() -> str:
    if os.name == "nt":
        return "windows"
    if "microsoft" in platform.uname().release.lower():
        return "wsl"
    return sys.platform


def pid_alive(pid: int) -> bool:
    if os.name == "nt":
        import ctypes

        process_query_limited_information = 0x1000
        still_active = 259
        kernel32 = ctypes.windll.kernel32
        handle = kernel32.OpenProcess(process_query_limited_information, False, pid)
        if not handle:
            return False
        try:
            code = ctypes.c_ulong()
            if not kernel32.GetExitCodeProcess(handle, ctypes.byref(code)):
                return True
            return code.value == still_active
        finally:
            kernel32.CloseHandle(handle)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def read_holder(path: Path) -> dict | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None
    except (OSError, ValueError):
        return {"unreadable": True}


def holder_is_stale(holder: dict) -> bool:
    """A holder is stale only if it is provably dead: same system and host."""
    return (
        holder.get("system") == system_name()
        and holder.get("host") == socket.gethostname()
        and isinstance(holder.get("pid"), int)
        and not pid_alive(holder["pid"])
    )


def describe(holder: dict) -> str:
    if holder.get("unreadable"):
        return "an unreadable lock file"
    return (
        f"pid {holder.get('pid')} on {holder.get('system')}/{holder.get('host')} "
        f"since {holder.get('started')} running {holder.get('command')!r} "
        f"from {holder.get('cwd')}"
    )


def try_acquire(path: Path, command: list[str]) -> bool:
    record = {
        "pid": os.getpid(),
        "system": system_name(),
        "host": socket.gethostname(),
        "cwd": os.getcwd(),
        "command": " ".join(command),
        "started": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
    except FileExistsError:
        return False
    with os.fdopen(fd, "w", encoding="utf-8") as handle:
        json.dump(record, handle)
    return True


def release(path: Path) -> None:
    holder = read_holder(path)
    if holder and holder.get("pid") == os.getpid() and holder.get("system") == system_name():
        path.unlink(missing_ok=True)


def acquire(path: Path, command: list[str], wait: bool) -> None:
    announced = False
    while not try_acquire(path, command):
        holder = read_holder(path)
        if holder is None:
            continue
        if holder_is_stale(holder):
            print(f"census_lock: reclaiming the lock of dead {describe(holder)}", file=sys.stderr)
            path.unlink(missing_ok=True)
            continue
        if not wait:
            raise SystemExit(f"census_lock: a census is running: {describe(holder)}")
        if not announced:
            print(f"census_lock: waiting for {describe(holder)}", file=sys.stderr)
            announced = True
        time.sleep(POLL_SECONDS)


def cmd_run(path: Path, command: list[str], wait: bool) -> int:
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise SystemExit("census_lock: run needs a command after --")
    acquire(path, command, wait)
    try:
        return subprocess.run(command).returncode
    except KeyboardInterrupt:
        return 130
    finally:
        release(path)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--lock", type=Path, default=None, help="lock file path")
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="run a command while holding the lock")
    run.add_argument("--wait", action="store_true", help="wait instead of failing when held")
    run.add_argument("argv", nargs=argparse.REMAINDER)
    sub.add_parser("status", help="print the current holder")
    rel = sub.add_parser("release", help="remove a lock whose census has ended")
    rel.add_argument("--force", action="store_true", required=True)
    args = parser.parse_args(argv)
    path = args.lock or default_lock_path()
    if args.command == "run":
        return cmd_run(path, args.argv, args.wait)
    holder = read_holder(path)
    if args.command == "status":
        if holder is None:
            print(f"free ({path})")
        else:
            stale = " (stale: its process has exited)" if holder_is_stale(holder) else ""
            print(f"held by {describe(holder)}{stale} ({path})")
        return 0
    if holder is not None:
        print(f"census_lock: removing the lock of {describe(holder)}", file=sys.stderr)
        path.unlink(missing_ok=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
