#!/usr/bin/env python3
"""Focused regressions for the SP17 W0A kickoff evidence."""

from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import generate_sp17_closure as closure  # noqa: E402


class LedgerTests(unittest.TestCase):
    def test_frozen_ledgers_are_sorted_complete_and_disjoint(self):
        baseline, inventory, targets, evidence = closure.load_ledgers()
        self.assertEqual(len(baseline), 3267)
        self.assertEqual(len(inventory), 842)
        self.assertEqual(len(targets), 19)
        self.assertEqual([item["test_id"] for item in evidence["tests"]], targets)
        self.assertFalse(set(baseline) & set(targets))

    def test_inventory_preserves_kickoff_partition_and_complete_owners(self):
        _, inventory, targets, _ = closure.load_ledgers()
        runnable = [item for item in inventory if item["kickoff_state"] == "runnable"]
        unported = [item for item in inventory if item["kickoff_state"] == "unported"]
        self.assertEqual(len(runnable), 19)
        self.assertEqual(len(unported), 823)
        self.assertEqual([item["test_id"] for item in runnable], targets)
        self.assertTrue(all(item["first_rejection"] is None for item in runnable))
        self.assertTrue(all(item["first_rejection"] for item in unported))
        self.assertTrue(all(
            "needs_writing_mode" in item["owner_categories"] for item in inventory
        ))

    def test_direct_property_probe_pool_is_frozen(self):
        _, inventory, _, _ = closure.load_ledgers()
        rejections = [item["first_rejection"] for item in inventory]
        writing_mode = sum(value in {
            "style_block_unsupported property: writing-mode",
            "unsupported property: writing-mode",
        } for value in rejections)
        unicode_bidi = rejections.count(
            "style_block_unsupported property: unicode-bidi"
        )
        self.assertEqual(writing_mode, 337)
        self.assertEqual(unicode_bidi, 3)

    def test_initial_results_preserve_exact_pixel_evidence(self):
        _, _, targets, evidence = closure.load_ledgers()
        results = {item["test_id"]: item for item in evidence["tests"]}
        self.assertEqual(set(results), set(targets))
        self.assertTrue(all(item["status"] == "fail" for item in results.values()))
        self.assertTrue(all(item["mismatched_pixels"] > 0 for item in results.values()))
        self.assertEqual(
            results["wpt/css_flexbox/auto-height-with-flex"]["mismatched_pixels"],
            1548,
        )
        self.assertEqual(
            results["wpt/css_flexbox/flex-basis-011-ref"]["mismatched_pixels"],
            16,
        )

    def test_generation_is_byte_idempotent(self):
        with closure.MAPPING_CSV.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        summary = json.loads(closure.SUMMARY_JSON.read_text(encoding="utf-8"))
        first = closure.build_outputs(rows, summary)
        second = closure.build_outputs(rows, summary)
        self.assertEqual(first, second)

    def test_historical_sp13r_through_sp16_ledgers_are_byte_pinned(self):
        closure.validate_historical_ledgers()


if __name__ == "__main__":
    unittest.main()
