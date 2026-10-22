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


if __name__ == "__main__":
    unittest.main()
