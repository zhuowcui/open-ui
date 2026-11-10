"""Tests for the machine-wide census lock."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

from tools.qualification import census_lock


class CensusLockTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.lock = Path(self.temporary.name) / "out" / "census.lock"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_lock(self, *argv: str) -> int:
        return census_lock.main(["--lock", str(self.lock), *argv])

    def test_run_holds_the_lock_and_releases_it(self) -> None:
        probe = f"import pathlib,sys; sys.exit(0 if pathlib.Path({str(self.lock)!r}).exists() else 7)"
        self.assertEqual(self.run_lock("run", "--", sys.executable, "-c", probe), 0)
        self.assertFalse(self.lock.exists())

    def test_run_returns_the_command_status(self) -> None:
        self.assertEqual(self.run_lock("run", "--", sys.executable, "-c", "raise SystemExit(3)"), 3)
        self.assertFalse(self.lock.exists())

    def test_live_holder_blocks_a_second_run(self) -> None:
        self.assertTrue(census_lock.try_acquire(self.lock, ["census"]))
        with self.assertRaisesRegex(SystemExit, "a census is running"):
            self.run_lock("run", "--", sys.executable, "-c", "pass")
        self.assertTrue(self.lock.exists())

    def test_dead_holder_on_this_system_is_reclaimed(self) -> None:
        self.lock.parent.mkdir(parents=True)
        dead = {
            "pid": 2**22 + 12345,
            "system": census_lock.system_name(),
            "host": census_lock.socket.gethostname(),
        }
        self.lock.write_text(json.dumps(dead), encoding="utf-8")
        self.assertTrue(census_lock.holder_is_stale(dead))
        self.assertEqual(self.run_lock("run", "--", sys.executable, "-c", "pass"), 0)
        self.assertFalse(self.lock.exists())

    def test_holder_on_the_other_system_is_never_reclaimed(self) -> None:
        other = {
            "pid": 2**22 + 12345,
            "system": "elsewhere",
            "host": census_lock.socket.gethostname(),
        }
        self.assertFalse(census_lock.holder_is_stale(other))

    def test_worktrees_share_the_main_checkout_lock(self) -> None:
        main = Path(self.temporary.name) / "main"
        (main / ".git" / "worktrees" / "track").mkdir(parents=True)
        tree = main / ".claude" / "worktrees" / "track"
        (tree / "tools").mkdir(parents=True)
        (tree / ".git").write_text(
            f"gitdir: {(main / '.git' / 'worktrees' / 'track').as_posix()}\n", encoding="utf-8"
        )
        self.assertEqual(census_lock.main_checkout(tree / "tools").resolve(), main.resolve())
        self.assertEqual(census_lock.main_checkout(main).resolve(), main.resolve())

    def test_windows_gitdir_maps_to_wsl_mount(self) -> None:
        path = census_lock.native_path("E:/open-ui/.git/worktrees/track")
        if census_lock.os.name == "nt":
            self.assertEqual(path, Path("E:/open-ui/.git/worktrees/track"))
        else:
            self.assertEqual(path, Path("/mnt/e/open-ui/.git/worktrees/track"))

    def test_forced_release_removes_the_lock(self) -> None:
        self.assertTrue(census_lock.try_acquire(self.lock, ["census"]))
        self.assertEqual(self.run_lock("release", "--force"), 0)
        self.assertFalse(self.lock.exists())


if __name__ == "__main__":
    unittest.main()
