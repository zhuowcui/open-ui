"""Guards for the Windows Chrome reference tooling that run on any host."""

from __future__ import annotations

from pathlib import Path
import re
import tempfile
import unittest

from tools.qualification import probe_windows_text_oracle as probe
from tools.qualification import windows_chrome


class WindowsChromeToolTests(unittest.TestCase):
    def test_unpinned_binaries_are_rejected_before_launch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fake = Path(temporary) / "chrome.exe"
            fake.write_bytes(b"not the pinned browser")
            (Path(temporary) / f"{windows_chrome.CHROMIUM_VERSION}.manifest").write_text("")
            with self.assertRaisesRegex(SystemExit, "pinned Windows Chrome"):
                windows_chrome.verify_chrome(fake)

    def test_pages_are_deterministic_and_load_repository_ahem(self) -> None:
        first = probe.page("<div></div>", "div{color:red}")
        self.assertEqual(first, probe.page("<div></div>", "div{color:red}"))
        self.assertIn("@font-face{font-family:Ahem;src:url(data:font/truetype;base64,", first)

    def test_every_scenario_names_an_existing_test_function(self) -> None:
        for name in probe.SCENARIOS:
            path, function = name.split("::")
            source = (windows_chrome.ROOT / path).read_text(encoding="utf-8")
            self.assertRegex(source, rf"\bfn {re.escape(function)}\b", name)

    def test_aliased_scenarios_disable_font_antialiasing(self) -> None:
        for name, (_body, _style, flags, _measure, _scales) in probe.SCENARIOS.items():
            source = (windows_chrome.ROOT / name.split("::")[0]).read_text(encoding="utf-8")
            if "deterministic_aliased" in source and flags:
                self.assertEqual(flags, probe.ALIASED, name)


if __name__ == "__main__":
    unittest.main()
