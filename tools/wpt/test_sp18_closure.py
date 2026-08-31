#!/usr/bin/env python3
"""Seven focused closure and porter regressions for SP18."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import generate_sp18_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import splice_text_port  # noqa: E402


class Sp18ClosureTests(unittest.TestCase):
    def setUp(self):
        self.old_profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)

    def tearDown(self):
        port_wpt.ACTIVE_PORTER_PROFILE = self.old_profile
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain

    def generate(self, css: str, body: str = "<div id='x'>text</div>") -> str:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "case.html"
            path.write_text(
                f"<!doctype html><style>{css}</style>{body}", encoding="utf-8"
            )
            parser = port_wpt.parse_wpt_html(str(path))
            self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
            return port_wpt.generate_rust_fn(
                "sp18_case", parser.root, parser.html_styles
            )

    def test_manifest_pins_and_complete_atomic_partitions(self):
        manifests = closure.load_manifests()
        expected_counts = (3807, 552, 251, 292, 4058, 81)
        self.assertEqual(tuple(map(len, manifests)), expected_counts)
        for path, expected in closure.MANIFEST_SHA256.items():
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), expected)
        closure.validate_partitions(*manifests)
        misses = closure.load_verified_misses()
        self.assertEqual(len(misses), 3)
        self.assertEqual(
            {item["rejection_owner"] for item in misses},
            {"needs_first_line", "needs_image", "needs_ruby"},
        )
        self.assertTrue(
            {item["test_id"] for item in misses}.issubset(set(manifests[3]))
        )

    def test_strict_syntax_classification_preserves_detector_false_positives(self):
        self.assertIsNone(closure.PROPERTY_SYNTAX.search("align-content: center"))
        self.assertIsNone(closure.PROPERTY_SYNTAX.search("justify-content: end"))
        self.assertIsNotNone(closure.PROPERTY_SYNTAX.search("content: 'x'"))
        self.assertIsNotNone(closure.PROPERTY_SYNTAX.search("line-clamp: 2"))
        self.assertIsNotNone(closure.PSEUDO_SYNTAX.search("div::first-letter"))
        rows = closure.load_rows()
        self.assertEqual(
            closure.strict_syntax_inventory(rows), closure.load_manifests()[1]
        )

    def test_terminal_pseudo_cascade_emits_ordered_generated_nodes(self):
        rust = self.generate(
            "div::before{content:'a';color:red}"
            "#x::before{content:'b';color:blue}"
            "div::after{content:attr(data-tail)}"
            "div::first-line{font-size:20px;font-family:monospace}"
            "div::first-letter{float:left}",
            "<div id='x' data-tail='z'>text</div>",
        )
        self.assertIn("PseudoElementKind::Before", rust)
        self.assertIn("PseudoElementKind::After", rust)
        self.assertLess(
            rust.index("PseudoElementKind::Before"),
            rust.index('text = Some("text".to_string())'),
        )
        self.assertLess(
            rust.index('text = Some("text".to_string())'),
            rust.index("PseudoElementKind::After"),
        )
        self.assertIn('GeneratedContentItem::String("b".to_string())', rust)
        self.assertIn('GeneratedContentItem::Attribute("data-tail".to_string())', rust)
        self.assertIn("first_line_style = Some", rust)
        self.assertIn("first_letter_style = Some", rust)
        self.assertIn("GenericFontFamily::Monospace", rust)

        nested = self.generate(
            ".box{position:relative;&::before{content:'x';position:absolute}}",
            "<div class='box'>text</div>",
        )
        self.assertIn("PseudoElementKind::Before", nested)
        self.assertIn("Position::Relative", nested)
        self.assertIn("Position::Absolute", nested)

    def test_generated_values_counters_quotes_escapes_and_shadows_emit(self):
        rust = self.generate(
            "div{counter-reset:chap 2;counter-increment:chap;"
            "quotes:'<' '>';text-shadow:1px 2px 3px currentColor}"
            "div::before{content:'\\41' counter(chap,upper-roman) open-quote}"
            "div::after{content:close-quote}",
        )
        self.assertIn('GeneratedContentItem::String("A".to_string())', rust)
        self.assertIn('name: "chap".to_string(), value: 2', rust)
        self.assertIn('name: "chap".to_string(), value: 1', rust)
        self.assertIn('CounterStyle::UpperRoman', rust)
        self.assertIn('GeneratedContentItem::OpenQuote', rust)
        self.assertIn('GeneratedContentItem::CloseQuote', rust)
        self.assertIn('open: "<".to_string()', rust)
        self.assertIn("style.text_shadow = vec![TextShadow", rust)
        solid_svg = port_wpt._css_image_rust(
            'url("data:image/svg+xml,<svg style=\'background: green\'></svg>")',
            "sp18_svg",
        )
        self.assertIsNotNone(solid_svg)
        self.assertIn("CssImage::LinearGradient", solid_svg[1])

    def test_modern_and_legacy_line_clamp_values_emit(self):
        rust = self.generate(
            ".modern{line-clamp:3 'more'}"
            ".legacy{display:-webkit-box;-webkit-box-orient:vertical;"
            "-webkit-line-clamp:2;text-overflow:ellipsis}",
            "<div class='modern'>one two three</div>"
            "<div class='legacy'>four five six</div>",
        )
        self.assertIn("LineClamp::Lines(3)", rust)
        self.assertIn('BlockEllipsis::String("more".to_string())', rust)
        self.assertIn("LineClamp::Lines(2)", rust)
        self.assertIn("WebkitBoxOrient::Vertical", rust)
        self.assertIn("legacy_webkit_box = true", rust)
        self.assertIn("TextOverflow::Ellipsis", rust)

    def test_two_no_write_generations_and_surgical_splices_are_identical(self):
        rows = closure.load_rows()
        first_outputs = closure.build_outputs(rows)
        second_outputs = closure.build_outputs(rows)
        self.assertEqual(first_outputs, second_outputs)
        targets = json.loads(closure.TARGETS.read_text(encoding="utf-8"))
        sample = [targets[0], targets[-1]]
        mapping = splice_text_port.load_mapping_rows()
        first = splice_text_port.prepare_changes(sample, mapping)
        second = splice_text_port.prepare_changes(sample, mapping)
        self.assertEqual(first, second)

    def test_closure_validator_is_deterministic_twice(self):
        closure.check()
        closure.check()


if __name__ == "__main__":
    unittest.main()
