#!/usr/bin/env python3
"""Focused regressions for the SP17 kickoff and live W1G closure evidence."""

from __future__ import annotations

import csv
import json
import sys
import unittest
from collections import OrderedDict
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import generate_sp17_closure as closure  # noqa: E402
import port_wpt  # noqa: E402


EXPECTED_LIVE_PROMOTIONS = {
    "wpt/css_flexbox/aspect-ratio-intrinsic-size-009",
    "wpt/css_flexbox/fit-content-item-002",
    "wpt/css_flexbox/fit-content-item-003",
    "wpt/css_flexbox/fit-content-item-004",
    "wpt/css_flexbox/flex-item-max-width-min-content",
    "wpt/css_flexbox/flex-item-min-width-min-content",
    "wpt/css_flexbox/flexbox-flex-wrap-flexing-003",
    "wpt/css_flexbox/flexbox-overflow-padding-002",
    "wpt/css_flexbox/flexbox-writing-mode-001",
    "wpt/css_flexbox/flexbox-writing-mode-002",
    "wpt/css_flexbox/flexbox-writing-mode-003",
    "wpt/css_flexbox/flexbox-writing-mode-004",
    "wpt/css_flexbox/flexbox-writing-mode-005",
    "wpt/css_flexbox/flexbox-writing-mode-006",
    "wpt/css_flexbox/flexbox-writing-mode-007",
    "wpt/css_flexbox/flexbox-writing-mode-008",
    "wpt/css_flexbox/flexbox-writing-mode-009",
    "wpt/css_flexbox/flexbox_align-items-center-3",
    "wpt/css_flexbox/flexbox_align-items-stretch-3",
    "wpt/css_flexbox/stretching-orthogonal-flows",
    "wpt/css_flexbox/css-flexbox-row",
    "wpt/css_flexbox/css-flexbox-row-reverse",
    "wpt/css_flexbox/css-flexbox-row-reverse-wrap",
    "wpt/css_flexbox/css-flexbox-row-reverse-wrap-reverse",
    "wpt/css_flexbox/css-flexbox-row-wrap",
    "wpt/css_flexbox/css-flexbox-row-wrap-reverse",
    "wpt/css_flexbox/flex-direction-row-vertical",
    "wpt/css_flexbox/flex-direction-row-vertical-ref",
    "wpt/css_flexbox/flexbox-flex-direction-default",
    "wpt/css_flexbox/flexbox-flex-direction-row",
    "wpt/css_flexbox/flexbox-flex-direction-row-reverse",
    "wpt/css_flexbox/flexbox-flex-direction-column",
    "wpt/css_flexbox/flexbox-flex-direction-column-reverse",
    "wpt/css_flexbox/flexbox-flex-wrap-wrap",
    "wpt/css_flexbox/flexbox-flex-wrap-wrap-reverse",
    "wpt/css_flexbox/intrinsic-size_col-wrap-crash",
} | {
    f"wpt/css_flexbox/gap-{number:03d}-{direction}{reference}"
    for number in range(1, 8)
    for direction in ("lr", "rl")
    for reference in ("", "-ref")
} | {
    f"wpt/css_flexbox/abspos_abspos-autopos-{mode}-{direction}"
    for mode in ("htb", "vlr", "vrl")
    for direction in ("ltr", "rtl")
} | {
    f"wpt/css_sizing/div-{keyword}-orthogonal-{case}.tentative"
    for keyword in ("fit-content", "max-content", "min-content")
    for case in (
        "auto-margin-left",
        "auto-margin-right",
        "auto-margin",
        "block-size",
    )
} | {
    f"wpt/css_sizing/div-orthogonal-{case}-ref"
    for case in (
        "auto-margin-left",
        "auto-margin-right",
        "auto-margin",
        "block-size",
    )
} | {
    "wpt/css_sizing/aspect-ratio_abspos-004",
    "wpt/css_sizing/aspect-ratio_abspos-015",
    "wpt/css_sizing/aspect-ratio_abspos-020",
    "wpt/css_sizing/div-orthogonal-left-and-non-auto-margin-ref",
    "wpt/css_sizing/div-orthogonal-left-and-non-auto-margin.tentative",
}


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

    def test_sp17_kickoff_ledgers_are_byte_pinned_after_live_promotions(self):
        for path in (
            closure.BASELINE_JSON,
            closure.INVENTORY_JSON,
            closure.INITIAL_TARGETS_JSON,
            closure.INITIAL_RESULTS_JSON,
        ):
            self.assertEqual(
                closure.hashlib.sha256(path.read_bytes()).hexdigest(),
                closure.KICKOFF_LEDGER_SHA256[path.name],
            )

    def test_live_snapshot_accepts_only_exact_actionable_promotions(self):
        baseline, inventory, _, _ = closure.load_ledgers()
        actionable, _ = closure.load_probe_ledgers(inventory)
        with closure.MAPPING_CSV.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        summary = json.loads(closure.SUMMARY_JSON.read_text(encoding="utf-8"))
        promoted = closure.validate_live_snapshot(
            rows, summary, baseline, inventory, actionable
        )
        self.assertEqual(len(EXPECTED_LIVE_PROMOTIONS), 91)
        self.assertEqual(promoted, EXPECTED_LIVE_PROMOTIONS)
        self.assertTrue(promoted.issubset(set(actionable)))

    def test_w0b_probe_is_a_disjoint_cover_of_the_frozen_inventory(self):
        _, inventory, initial, _ = closure.load_ledgers()
        actionable, residuals = closure.load_probe_ledgers(inventory)
        residual_ids = {item["test_id"] for item in residuals}
        self.assertEqual(len(actionable), 311)
        self.assertEqual(len(residuals), 531)
        self.assertFalse(set(actionable) & residual_ids)
        self.assertEqual(
            set(actionable) | residual_ids,
            {item["test_id"] for item in inventory},
        )
        self.assertTrue(set(initial).issubset(actionable))

    def test_w0b_probe_ledgers_are_byte_pinned(self):
        for path in (closure.ACTIONABLE_JSON, closure.RESIDUALS_JSON):
            self.assertEqual(
                closure.hashlib.sha256(path.read_bytes()).hexdigest(),
                closure.PROBE_LEDGER_SHA256[path.name],
            )

    def test_w0b_actionable_area_counts_and_sole_owner_partition_are_frozen(self):
        _, inventory, _, _ = closure.load_ledgers()
        actionable, residuals = closure.load_probe_ledgers(inventory)
        counts = {}
        for test_id in actionable:
            area = test_id.split("/")[1]
            counts[area] = counts.get(area, 0) + 1
        self.assertEqual(counts, {
            "css2_floats": 1,
            "css_backgrounds": 3,
            "css_break": 51,
            "css_flexbox": 145,
            "css_multicol": 11,
            "css_overflow": 22,
            "css_position": 35,
            "css_sizing": 43,
        })
        kickoff_sole_unported = {
            item["test_id"] for item in inventory
            if item["kickoff_state"] == "unported"
            and item["owner_categories"] == ["needs_writing_mode"]
        }
        self.assertEqual(len(kickoff_sole_unported), 90)
        self.assertEqual(len(kickoff_sole_unported & set(actionable)), 85)
        self.assertEqual(
            len(kickoff_sole_unported & {item["test_id"] for item in residuals}),
            5,
        )

    def test_w0b_residuals_are_reason_owned_and_font_guard_is_explicit(self):
        _, inventory, _, _ = closure.load_ledgers()
        _, residuals = closure.load_probe_ledgers(inventory)
        for item in residuals:
            self.assertTrue(item["rejection_reason"])
            self.assertIn(item["rejection_owner"], item["owner_categories"])
        sp17_owned = [
            item for item in residuals
            if "needs_writing_mode" in item["owner_categories"]
        ]
        self.assertEqual(
            {item["test_id"] for item in sp17_owned},
            {
                "wpt/css_flexbox/css-flexbox-test1",
                "wpt/css_flexbox/css-flexbox-test1-ref",
            },
        )
        self.assertTrue(all(
            item["rejection_reason"] == "text_non_ascii"
            and item["rejection_owner"] == "needs_writing_mode"
            for item in sp17_owned
        ))
        self.assertEqual(
            closure.dependency_for_portability_reason("text_non_ascii"),
            "writing_mode",
        )


class TransactionalSp17CssTests(unittest.TestCase):
    def setUp(self):
        self.old_profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        port_wpt.set_porter_profile(port_wpt.PorterProfile.LEGACY_BOX_ONLY)

    def tearDown(self):
        port_wpt.ACTIVE_PORTER_PROFILE = self.old_profile
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain

    def test_invalid_sp17_declarations_preserve_prior_valid_values(self):
        styles = port_wpt.parse_inline_styles(
            "writing-mode:vertical-rl;writing-mode:diagonal;"
            "direction:rtl;direction:auto;"
            "unicode-bidi:isolate-override;unicode-bidi:visual;"
            "text-orientation:upright;text-orientation:rotate-left;"
            "text-combine-upright:all;text-combine-upright:digits 9"
        )
        self.assertEqual(styles["writing-mode"], "vertical-rl")
        self.assertEqual(styles["direction"], "rtl")
        self.assertEqual(styles["unicode-bidi"], "isolate-override")
        self.assertEqual(styles["text-orientation"], "upright")
        self.assertEqual(styles["text-combine-upright"], "all")

    def test_css_wide_keywords_resolve_at_the_correct_inheritance_boundary(self):
        styles = port_wpt.parse_inline_styles(
            "writing-mode:unset;direction:unset;text-orientation:unset;"
            "text-combine-upright:unset;unicode-bidi:unset"
        )
        self.assertEqual(styles["writing-mode"], "inherit")
        self.assertEqual(styles["direction"], "inherit")
        self.assertEqual(styles["text-orientation"], "inherit")
        self.assertEqual(styles["text-combine-upright"], "inherit")
        self.assertEqual(styles["unicode-bidi"], "normal")
        initial = port_wpt.parse_inline_styles(
            "writing-mode:initial;direction:revert;text-orientation:revert-layer;"
            "text-combine-upright:initial;unicode-bidi:initial"
        )
        self.assertEqual(initial["writing-mode"], "horizontal-tb")
        self.assertEqual(initial["direction"], "ltr")
        self.assertEqual(initial["text-orientation"], "mixed")
        self.assertEqual(initial["text-combine-upright"], "none")
        self.assertEqual(initial["unicode-bidi"], "normal")

    def test_sp17_computed_values_emit_existing_rust_style_enums(self):
        styles = port_wpt.parse_inline_styles(
            "writing-mode:sideways-lr;direction:rtl;unicode-bidi:plaintext;"
            "text-orientation:sideways;text-combine-upright:all"
        )
        lines, _ = port_wpt.generate_style_code(styles, "n1")
        output = "\n".join(lines)
        self.assertIn("writing_mode = WritingMode::SidewaysLr", output)
        self.assertIn("direction = Direction::Rtl", output)
        self.assertIn("unicode_bidi = UnicodeBidi::Plaintext", output)
        self.assertIn("text_orientation = TextOrientation::Sideways", output)
        self.assertIn("text_combine_upright = TextCombineUpright::All", output)

    def test_logical_sides_resolve_from_writing_mode_and_direction(self):
        styles = port_wpt.parse_inline_styles(
            "writing-mode:vertical-rl;direction:ltr;inline-size:10px;"
            "block-size:20px;margin-inline-start:1px;"
            "padding-block-end:2px;inset-inline-end:3px;"
            "border-block-start-width:4px"
        )
        resolved = port_wpt.resolve_sp17_logical_properties(styles)
        self.assertEqual(resolved["height"], "10px")
        self.assertEqual(resolved["width"], "20px")
        self.assertEqual(resolved["margin-top"], "1px")
        self.assertEqual(resolved["padding-left"], "2px")
        self.assertEqual(resolved["bottom"], "3px")
        self.assertEqual(resolved["border-right-width"], "4px")

        rtl = port_wpt.resolve_sp17_logical_properties(port_wpt.parse_inline_styles(
            "writing-mode:vertical-lr;direction:rtl;"
            "margin-inline-start:5px;margin-inline-end:6px"
        ))
        self.assertEqual(rtl["margin-bottom"], "5px")
        self.assertEqual(rtl["margin-top"], "6px")

        sideways = port_wpt.resolve_sp17_logical_properties(
            port_wpt.parse_inline_styles(
                "writing-mode:sideways-lr;direction:ltr;"
                "inset-inline-start:7px;inset-block-start:8px"
            )
        )
        self.assertEqual(sideways["bottom"], "7px")
        self.assertEqual(sideways["left"], "8px")

        corners = port_wpt.resolve_sp17_logical_properties(
            port_wpt.parse_inline_styles(
                "writing-mode:vertical-rl;direction:ltr;"
                "border-start-start-radius:9px;"
                "border-end-end-radius:10px"
            )
        )
        self.assertEqual(corners["border-top-right-radius"], "9px")
        self.assertEqual(corners["border-bottom-left-radius"], "10px")

    def test_logical_physical_conflicts_use_declaration_cascade_priority(self):
        inline = port_wpt.parse_inline_styles(
            "writing-mode:vertical-rl;margin-right:1px;"
            "margin-block-start:2px;margin-right:3px"
        )
        resolved = port_wpt.resolve_sp17_logical_properties(inline)
        self.assertEqual(resolved["margin-right"], "3px")

        node = port_wpt.DomNode("div", {"id": "x", "class": "x"}, OrderedDict())
        rules = port_wpt.parse_simple_css_rules(
            "#x { margin-right: 3px; }"
            ".x { writing-mode: vertical-rl; margin-block-start: 2px; }"
        )
        port_wpt.apply_css_rules(rules, node)
        resolved = port_wpt.resolve_sp17_logical_properties(node.styles)
        self.assertEqual(resolved["margin-right"], "3px")

        important = port_wpt.DomNode(
            "div", {"id": "y", "class": "y"}, OrderedDict()
        )
        port_wpt.apply_css_rules(port_wpt.parse_simple_css_rules(
            "#y { margin-right: 3px; }"
            ".y { writing-mode: vertical-rl; "
            "margin-block-start: 2px !important; }"
        ), important)
        resolved = port_wpt.resolve_sp17_logical_properties(important.styles)
        self.assertEqual(resolved["margin-right"], "2px")

        same_property = port_wpt.parse_inline_styles(
            "writing-mode:vertical-rl!important;writing-mode:horizontal-tb"
        )
        self.assertEqual(same_property["writing-mode"], "vertical-rl")

        repeated = port_wpt.DomNode("div", {"class": "z"}, OrderedDict())
        port_wpt.apply_css_rules(port_wpt.parse_simple_css_rules(
            ".z { writing-mode: vertical-rl; margin-right: 1px; "
            "margin-block-start: 2px; margin-right: 3px; }"
        ), repeated)
        resolved = port_wpt.resolve_sp17_logical_properties(repeated.styles)
        self.assertEqual(resolved["margin-right"], "3px")

    def test_writing_properties_cross_elements_and_anonymous_text(self):
        root = port_wpt.DomNode(
            "body", {}, port_wpt.parse_inline_styles(
                "writing-mode:vertical-lr;direction:rtl;text-orientation:sideways"
            )
        )
        child = port_wpt.DomNode("div", {}, OrderedDict())
        text = port_wpt.DomNode("#text", {}, OrderedDict())
        text.is_text = True
        text.text_content = "x"
        child.children.append(text)
        root.children.append(child)
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        output = port_wpt.generate_rust_fn("sp17_inheritance", root)
        self.assertGreaterEqual(
            output.count("writing_mode = WritingMode::VerticalLr"), 3
        )
        self.assertGreaterEqual(output.count("direction = Direction::Rtl"), 3)
        self.assertGreaterEqual(
            output.count("text_orientation = TextOrientation::Sideways"), 3
        )

    def test_dir_presentational_hint_is_lower_priority_than_author_css(self):
        node = port_wpt.DomNode("div", {"dir": "rtl", "class": "x"}, OrderedDict())
        port_wpt.apply_css_rules(
            port_wpt.parse_simple_css_rules(".x { direction: ltr; }"), node
        )
        self.assertEqual(node.styles["direction"], "ltr")

        hinted = port_wpt.DomNode("div", {"dir": "rtl"}, OrderedDict())
        port_wpt.apply_css_rules([], hinted)
        self.assertEqual(hinted.styles["direction"], "rtl")
        self.assertEqual(hinted.styles["unicode-bidi"], "isolate")


if __name__ == "__main__":
    unittest.main()
