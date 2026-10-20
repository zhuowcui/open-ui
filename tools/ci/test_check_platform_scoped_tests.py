"""Ledger guards for tests ignored outside Linux."""

from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from tools.ci import check_platform_scoped_tests as scoped


MARKED = '''
#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "linux-oracle: fontconfig DejaVu metrics")]
fn measures_dejavu() {}

#[cfg_attr(not(target_os = "linux"), ignore = "linux-oracle: FreeType glyph pixels")]
#[test]
fn paints_glyphs() {}

#[test]
fn portable() {}
'''


class PlatformScopedTests(unittest.TestCase):
    def scan(self, files: dict[str, str]):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            root = base / "bindings/rust"
            for name, text in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
            return scoped.marked_tests(root, base)

    def test_markers_above_and_below_test_attribute_are_recorded_with_reasons(self) -> None:
        tests, errors = self.scan({"crate/tests/text.rs": MARKED})
        self.assertEqual(errors, [])
        self.assertEqual(tests, {
            "bindings/rust/crate/tests/text.rs::measures_dejavu": "fontconfig DejaVu metrics",
            "bindings/rust/crate/tests/text.rs::paints_glyphs": "FreeType glyph pixels",
        })
        self.assertTrue(scoped.render(tests).endswith(
            "bindings/rust/crate/tests/text.rs::paints_glyphs\tFreeType glyph pixels\n"))

    def test_unconditional_or_malformed_markers_are_rejected(self) -> None:
        for marker in (
            '#[ignore = "linux-oracle: everywhere"]',
            '#[cfg_attr(windows, ignore = "linux-oracle: wrong predicate")]',
            '#[cfg_attr(not(target_os = "linux"), ignore = "linux-oracle:")]',
        ):
            with self.subTest(marker=marker):
                _, errors = self.scan({"crate/tests/a.rs": f"#[test]\n{marker}\nfn a() {{}}\n"})
                self.assertEqual(len(errors), 1)
                self.assertIn("malformed", errors[0])

    def test_marker_must_attach_to_a_unique_test_function(self) -> None:
        marker = '#[cfg_attr(not(target_os = "linux"), ignore = "linux-oracle: pixels")]'
        _, errors = self.scan({"crate/src/a.rs": f"{marker}\nfn helper() {{}}\n"})
        self.assertIn("not on a #[test] function", errors[0])
        _, errors = self.scan({"crate/src/b.rs": f"#[test]\n{marker}\nfn same() {{}}\n"
                                                 f"#[test]\n{marker}\nfn same() {{}}\n"})
        self.assertIn("duplicate test name", errors[0])

    def test_rustfmt_wrapped_marker_is_recorded(self) -> None:
        wrapped = (
            "    #[test]\n    #[cfg_attr(\n        not(target_os = \"linux\"),\n"
            "        ignore = \"linux-oracle: FreeType Ahem line metrics\"\n    )]\n"
            "    fn wrapped() {}\n"
        )
        tests, errors = self.scan({"crate/src/lib.rs": wrapped})
        self.assertEqual(errors, [])
        self.assertEqual(tests, {"bindings/rust/crate/src/lib.rs::wrapped": "FreeType Ahem line metrics"})

    def test_neighbouring_test_attribute_does_not_qualify_a_helper(self) -> None:
        marker = '#[cfg_attr(not(target_os = "linux"), ignore = "linux-oracle: pixels")]'
        source = f"#[test]\nfn real() {{}}\n{marker}\nfn helper() {{}}\n"
        _, errors = self.scan({"crate/src/c.rs": source})
        self.assertIn("not on a #[test] function", errors[0])

    def test_vendored_sources_are_not_scanned(self) -> None:
        tests, errors = self.scan({"vendor/x/src/lib.rs": '"linux-oracle: data"'})
        self.assertEqual((tests, errors), ({}, []))


if __name__ == "__main__":
    unittest.main()
