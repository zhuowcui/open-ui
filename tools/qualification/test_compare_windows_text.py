from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compare_windows_text as tool  # noqa: E402


class WindowsTextComparisonTests(unittest.TestCase):
    def test_faces_cover_every_bundled_family_from_repository_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fonts = Path(temporary)
            for _, name, _ in tool.BUNDLED_FACES:
                (fonts / name).write_bytes(name.encode())
            css = tool.bundled_font_faces_css(fonts)
        self.assertEqual(css.count("@font-face"), len(tool.BUNDLED_FACES))
        self.assertIn("font-family:'Droid Sans Fallback';font-weight:400", css)
        self.assertIn("font-family:'DejaVu Serif';font-weight:700", css)
        self.assertTrue(css.startswith("<style>") and css.endswith("</style>"))

    def test_faces_precede_every_author_style(self) -> None:
        page = tool.windows_page('<!DOCTYPE html><html><head lang=x><style>a{}</style></head></html>', "<F>")
        self.assertEqual(page, '<!DOCTYPE html><html><head lang=x><F><style>a{}</style></head></html>')
        with self.assertRaises(ValueError):
            tool.windows_page("<html><body></body></html>", "<F>")

    def test_chrome_features_add_sp18_only_for_sp18_targets(self) -> None:
        self.assertEqual(tool.chrome_features(False, "A,B"), "NoFontAntialiasing")
        self.assertEqual(tool.chrome_features(True, "A,B"), "NoFontAntialiasing,A,B")

    def test_selection_keeps_manifest_order_and_excludes_real_fonts(self) -> None:
        self.assertEqual(
            tool.select_ids(["c", "a", "b", "d"], {"a", "b", "c"}, {"b"}),
            ["c", "a"],
        )

    def test_cross_tabulation_marks_tests_absent_from_the_linux_census(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            summary = Path(temporary) / "summary.json"
            summary.write_text(json.dumps({"profiles": [
                {"profile": "other", "tests": [{"id": "a", "status": "different"}]},
                {"profile": "p", "tests": [{"id": "a", "status": "exact"},
                                           {"id": "b", "status": "different"}]},
            ]}), encoding="utf-8")
            linux = tool.linux_statuses(summary, "p")
        self.assertEqual(linux, {"a": "exact", "b": "different"})
        results = [{"id": "a", "status": "exact"}, {"id": "b", "status": "exact"},
                   {"id": "c", "status": "different"}]
        self.assertEqual(tool.cross_tabulate(results, linux), {
            "linux-different:windows-exact": 1,
            "linux-exact:windows-exact": 1,
            "linux-unmeasured:windows-different": 1,
        })
        self.assertEqual(tool.linux_statuses(None, "p"), {})

    def test_browser_session_restarts_once_then_records_an_error(self) -> None:
        events = []

        class Browser:
            def __init__(self, name):
                self.name = name

            def __enter__(self):
                events.append(f"start {self.name}")
                return self

            def __exit__(self, *_):
                events.append(f"stop {self.name}")

        names = iter("abcd")
        session = tool.BrowserSession(lambda: Browser(next(names)))
        outcomes = iter([TimeoutError(), {"id": "t1", "status": "exact"},
                         {"id": "t2", "status": "exact"},
                         RuntimeError(), OSError()])

        def action(browser):
            outcome = next(outcomes)
            if isinstance(outcome, Exception):
                raise outcome
            return {**outcome, "browser": browser.name}

        self.assertEqual(session.run("t1", action), {"id": "t1", "status": "exact", "browser": "b"})
        self.assertEqual(session.run("t2", action)["browser"], "b")
        self.assertEqual(session.run("t3", action),
                         {"id": "t3", "status": "error", "mismatched_pixels": -1})
        session.close()
        self.assertEqual(session.restarts, 3)
        self.assertEqual(events, ["start a", "stop a", "start b", "stop b", "start c", "stop c"])


if __name__ == "__main__":
    unittest.main()
