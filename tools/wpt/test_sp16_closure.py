#!/usr/bin/env python3
"""Focused SP16 ledger, real-font porter, runner, and audit regressions."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
ACCOUNTABILITY = ROOT / "tools" / "accountability"
UPSTREAM_FIXTURES = HERE / "fixtures" / "upstream"
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ACCOUNTABILITY))

import generate_sp16_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import splice_text_port  # noqa: E402
import run_all_pixel_comparisons as runner  # noqa: E402
import shared_detectors  # noqa: E402

_audit_spec = importlib.util.spec_from_file_location(
    "sp16_audit", ACCOUNTABILITY / "audit.py"
)
audit = importlib.util.module_from_spec(_audit_spec)
_audit_spec.loader.exec_module(audit)


class LedgerTests(unittest.TestCase):
    def test_frozen_ledgers_are_sorted_disjoint_and_complete(self):
        baseline, targets, residuals, manifest = closure.load_ledgers()
        self.assertEqual(len(baseline), 2804)
        self.assertEqual(len(targets), 226)
        self.assertEqual(len(residuals), 550)
        self.assertEqual(manifest, targets)
        residual_ids = {item["test_id"] for item in residuals}
        self.assertFalse(set(targets) & residual_ids)
        self.assertEqual(len(set(targets) | residual_ids), 776)
        self.assertFalse(set(baseline) & (set(targets) | residual_ids))

    def test_residuals_have_reason_backed_non_font_owners(self):
        _, _, residuals, _ = closure.load_ledgers()
        for item in residuals:
            self.assertTrue(item["chromium_test_path"])
            self.assertTrue(item["rejection_reason"])
            owners = set(item["owner_categories"])
            self.assertNotIn("needs_font_metrics", owners)
            self.assertFalse(owners & closure.FALLBACK_CATEGORIES)
            self.assertTrue(owners - closure.METADATA_CATEGORIES)


class RealFontPorterTests(unittest.TestCase):
    def setUp(self):
        self.old_profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        self.upstream_patch = mock.patch.object(
            splice_text_port, "WPT_ROOT", str(UPSTREAM_FIXTURES)
        )
        self.upstream_patch.start()

    def tearDown(self):
        self.upstream_patch.stop()
        port_wpt.ACTIVE_PORTER_PROFILE = self.old_profile
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain

    def test_profiles_are_explicit_and_isolated(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.LEGACY_BOX_ONLY)
        self.assertFalse(port_wpt.EMIT_TEXT_NODES)
        self.assertFalse(port_wpt.RETAIN_TEXT)
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        self.assertTrue(port_wpt.EMIT_TEXT_NODES)
        self.assertTrue(port_wpt.RETAIN_TEXT)
        port_wpt.set_porter_profile(
            port_wpt.PorterProfile.REAL_FONT, retain_text=False
        )
        self.assertFalse(port_wpt.EMIT_TEXT_NODES)
        port_wpt.set_porter_profile(
            port_wpt.PorterProfile.REAL_FONT, retain_text=True
        )
        self.assertTrue(port_wpt.EMIT_TEXT_NODES)

    def test_font_shorthand_parses_corpus_grammar_and_resets(self):
        parsed = port_wpt.parse_font_shorthand(
            'italic small-caps 700 condensed 20px/150% "DejaVu Serif", serif'
        )
        self.assertEqual(parsed["font-style"], "italic")
        self.assertEqual(parsed["font-variant-caps"], "small-caps")
        self.assertEqual(parsed["font-weight"], "700")
        self.assertEqual(parsed["font-stretch"], "condensed")
        self.assertEqual(parsed["font-size"], "20px")
        self.assertEqual(parsed["line-height"], "150%")
        self.assertEqual(parsed["font-family"], '"DejaVu Serif", serif')
        initial = port_wpt.parse_font_shorthand("initial")
        self.assertEqual(initial["font-weight"], "normal")
        self.assertEqual(initial["line-height"], "normal")
        self.assertEqual(initial["font-family"], "sans-serif")
        inherit = port_wpt.parse_font_shorthand("inherit")
        self.assertTrue(all(value == "inherit" for value in inherit.values()))

    def test_unsupported_shorthand_rejects_without_partial_longhands(self):
        port_wpt.set_porter_profile(
            port_wpt.PorterProfile.REAL_FONT, retain_text=True
        )
        with self.assertRaises(port_wpt.UnsupportedFontShorthand):
            port_wpt.parse_inline_styles("color:red;font:caption;width:2ch")

    def test_real_splice_is_idempotent_and_does_not_edit_text_manifest(self):
        test_id = "wpt/css2_floats/float-nowrap-1"
        mapping = splice_text_port.load_mapping_rows()
        generated, originals, changes = splice_text_port.prepare_changes(
            [test_id], mapping, profile=port_wpt.PorterProfile.REAL_FONT
        )
        self.assertEqual([item.test_id for item in generated], [test_id])
        self.assertNotIn(splice_text_port.TEXT_PORTED_LIST, changes)
        for path, content in changes.items():
            self.assertEqual(content, Path(path).read_bytes().decode("utf-8"))
            self.assertEqual(originals[path], content)


class RunnerAndOwnershipTests(unittest.TestCase):
    def test_real_profile_precedes_ahem_and_pins_freetype(self):
        env = runner.openui_environment(use_ahem_noaa=True, use_real_font=True)
        self.assertEqual(env["OPENUI_EDGING"], "subpixel")
        self.assertEqual(env["OPENUI_HINTING"], "slight")
        self.assertTrue(
            env["LD_LIBRARY_PATH"].startswith(runner.REAL_FONT_FREETYPE_DIR + ":")
        )
        chrome = runner.chrome_environment(
            "/tmp/chrome", use_ahem_noaa=True, use_real_font=True
        )
        self.assertEqual(chrome["FONTCONFIG_FILE"], runner.REAL_FONTCONFIG)

    def test_pinned_freetype_provenance_hash(self):
        path = Path(runner.REAL_FONT_FREETYPE_DIR) / "libfreetype.so.6"
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        self.assertEqual(
            digest,
            "accea5cff7580ef0be412ac566ad2bd825ec699947d23a1a1a6602b58adba0ec",
        )

    def test_font_metric_category_is_retired_from_registry(self):
        keys = {item[0] for item in shared_detectors.DEPENDENCY_DEFS}
        self.assertNotIn("font_metrics", keys)
        self.assertNotIn("font_metrics", shared_detectors.CATEGORY_FOR_DEP)
        categories, _ = shared_detectors.classify_failure_categories(
            "<style>div{width:2ch;font:20px monospace}</style><div></div>"
        )
        self.assertNotIn("needs_font_metrics", categories)

    def test_newly_exposed_owners_are_functional_and_source_specific(self):
        fixtures = (
            (
                "<div>1111<br>2222 3333</div>",
                "wpt/css2_floats/floats-line-wrap-shifted-001-ref",
                "needs_float_line_wrap_rewind",
            ),
            (
                "<style>span{float:right}div{white-space:nowrap}</style>",
                "wpt/css2_floats/float-nowrap-2",
                "needs_float_line_wrap_rewind",
            ),
            (
                "<style>.x{display:inline-flex}</style>",
                "wpt/css_flexbox/dynamic-isize-change-001-ref",
                "needs_flex_intrinsic_sizing",
            ),
            (
                "<style>.x{display:flex;align-items:baseline}</style>",
                "wpt/css_flexbox/flexbox-align-self-baseline-horiz-004",
                "needs_flex_multiline_baseline",
            ),
            (
                "<style>.x{display:inline-flex;order:1;z-index:0}</style>",
                "wpt/css_flexbox/flexbox-paint-ordering-002",
                "needs_flex_item_paint_order",
            ),
        )
        for html, test_id, expected in fixtures:
            categories, _ = shared_detectors.classify_failure_categories(
                html, test_id=test_id
            )
            self.assertIn(expected, categories.split(","))


class SupersessionTests(unittest.TestCase):
    def test_sp14_residual_can_be_superseded_by_sp16_disposition(self):
        test_id = "wpt/css_flexbox/example"
        disposition = {
            "test_id": test_id,
            "chromium_test_path": "css-flexbox/example.html",
            "rejection_reason": "uses_javascript",
            "owner_categories": ["needs_javascript"],
        }
        row = {
            "sp_area": "css_flexbox",
            "test_name": "example",
            "ported": "no",
            "our_test_id": "",
            "chromium_test_path": disposition["chromium_test_path"],
            "failure_category": "needs_javascript",
            "notes": "Porter deferred: uses_javascript",
        }
        historical = [{
            **disposition,
            "rejection_owner": "needs_font_metrics",
            "owner_categories": ["needs_font_metrics", "needs_javascript"],
        }]
        errors = audit.sp14_text_closure_errors(
            [row], {}, {}, set(), [], [], historical,
            enforce_frozen_counts=False,
            superseded_residuals={test_id: disposition},
        )
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
