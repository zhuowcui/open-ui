#!/usr/bin/env python3
"""Focused regressions for the SP17 kickoff and live W2A closure evidence."""

from __future__ import annotations

import csv
import json
import re
import sys
import tempfile
import unittest
from collections import OrderedDict
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import generate_sp17_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import splice_text_port  # noqa: E402
import generate_sp13r_multicol_closure as sp13r_closure  # noqa: E402


EXPECTED_LIVE_PROMOTIONS = {
    "wpt/css_flexbox/auto-height-with-flex",
    "wpt/css_flexbox/align-content-wrap-004",
    "wpt/css_flexbox/aspect-ratio-intrinsic-size-009",
    "wpt/css_flexbox/fit-content-item-002",
    "wpt/css_flexbox/fit-content-item-003",
    "wpt/css_flexbox/fit-content-item-004",
    "wpt/css_flexbox/flex-wrap-002",
    "wpt/css_flexbox/flex-wrap-003",
    "wpt/css_flexbox/flex-wrap-004",
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
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-safe-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-safe-001-ref",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-safe-002",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-safe-002-ref",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-safe-003",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-safe-003-ref",
    "wpt/css_flexbox/flexbox-safe-overflow-position-005",
} | {
    "wpt/css_position/position-absolute-center-001",
    "wpt/css_position/position-absolute-center-002",
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
} | {
    "wpt/css_break/borders-006",
    "wpt/css_break/borders-007",
    "wpt/css_break/overflow-clip-000",
    "wpt/css_break/overflow-clip-001",
    "wpt/css_break/overflow-clip-002",
    "wpt/css_break/flexbox_multi-line-row-flex-fragmentation-056",
    "wpt/css_break/flexbox_single-line-column-flex-fragmentation-044",
    "wpt/css_break/flexbox_single-line-row-flex-fragmentation-030",
    "wpt/css_break/out-of-flow-in-multicolumn-063",
    "wpt/css_break/out-of-flow-in-multicolumn-064",
    "wpt/css_break/out-of-flow-in-multicolumn-066",
    "wpt/css_break/out-of-flow-in-multicolumn-067",
    "wpt/css_break/out-of-flow-in-multicolumn-118",
    "wpt/css_break/out-of-flow-in-multicolumn-119",
    "wpt/css_multicol/multicol-fill-balance-004",
    "wpt/css_multicol/multicol-span-auto-size-in-vertical-writing-mode-001",
    "wpt/css_multicol/multicol-span-auto-size-in-vertical-writing-mode-002",
    "wpt/css_multicol/multicol-under-vertical-rl-scroll",
    "wpt/css_multicol/orthogonal-writing-mode-shrink-to-fit",
    "wpt/css_multicol/orthogonal-writing-mode-spanner",
    "wpt/css_overflow/no-scrollable-overflow-vertical-rl",
    "wpt/css_overflow/no-scrollable-overflow-vertical-rl-2",
} | {
    f"wpt/css_position/static-position_{mode}-{suffix}"
    for mode in ("vlr", "vrl")
    for suffix in (
        "ltr-ltr",
        "ltr-rtl.tentative",
        "ref",
        "rtl-ltr.tentative",
        "rtl-rtl",
    )
} | {
    "wpt/css_position/static-position_htb-rtl-ltr.tentative",
    "wpt/css_position/static-position_htb-rtl-rtl",
} | {
    f"wpt/css_position/multicol_static-position_{mode}-{suffix}"
    for mode in ("vlr", "vrl")
    for suffix in (
        "in-multicol-ref",
        "ltr-ltr-in-multicol",
        "ltr-rtl-in-multicol.tentative",
        "rtl-ltr-in-multicol.tentative",
        "rtl-rtl-in-multicol",
    )
} | {
    f"wpt/css_position/multicol_{mode}-{suffix}"
    for mode in ("vlr", "vrl")
    for suffix in (
        "in-multicols-ref",
        "ltr-ltr-in-multicols",
        "ltr-rtl-in-multicols.tentative",
        "rtl-ltr-in-multicols.tentative",
        "rtl-rtl-in-multicols",
    )
} | {
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-rtl-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-rtl-002",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-rtl-003",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-rtl-004",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-vertWM-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-vertWM-002",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-vertWM-003",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-align-self-vertWM-004",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-justify-content-rtl-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-justify-content-rtl-002",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-justify-content-vertWM-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-justify-content-vertWM-002",
} | {
    f"wpt/css_flexbox/flexbox-writing-mode-{number:03d}{reference}"
    for number in range(10, 17)
    for reference in ("", "-ref")
} | {
    "wpt/css_flexbox/flexbox-writing-mode-slr",
    "wpt/css_flexbox/flexbox-writing-mode-slr-row-mix",
    "wpt/css_flexbox/flexbox-writing-mode-slr-row-mix-ref",
    "wpt/css_flexbox/flexbox-writing-mode-slr-rtl",
    "wpt/css_flexbox/flexbox-writing-mode-srl",
    "wpt/css_flexbox/flexbox-writing-mode-srl-row-mix",
    "wpt/css_flexbox/flexbox-writing-mode-srl-row-mix-ref",
    "wpt/css_flexbox/flexbox-writing-mode-srl-rtl",
}
EXPECTED_W2A_LIVE_PROMOTIONS = frozenset(EXPECTED_LIVE_PROMOTIONS)
EXPECTED_LIVE_PROMOTIONS |= set(json.loads(
    (HERE / "sp17_w2b_w4_targets.json").read_text(encoding="utf-8")
))


class AssertionOnlyCheckLayoutPorterTests(unittest.TestCase):
    def setUp(self):
        self.old_profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        port_wpt.set_porter_profile(port_wpt.PorterProfile.LEGACY_BOX_ONLY)
        self.temp = tempfile.TemporaryDirectory()

    def tearDown(self):
        port_wpt.ACTIVE_PORTER_PROFILE = self.old_profile
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain
        self.temp.cleanup()

    def parse(self, body: str, scripts: str | None = None):
        if scripts is None:
            scripts = """
              <script src="/resources/testharness.js"></script>
              <script src="/resources/testharnessreport.js"></script>
              <script src="/resources/check-layout-th.js"></script>
            """
        path = Path(self.temp.name, "check-layout.html")
        path.write_text(
            "<!doctype html><html><head>"
            "<style>.box { width: 10px; height: 10px }</style>"
            f"{scripts}</head>{body}</html>",
            encoding="utf-8",
        )
        return path, port_wpt.parse_wpt_html(str(path))

    def test_exact_check_layout_harness_is_accepted_and_stripped(self):
        path, parser = self.parse(
            "<body onload=\"checkLayout('div > div')\">"
            "<div class=\"box\" data-offset-x=\"0\"></div></body>"
        )
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
        template = port_wpt.generate_html_template(str(path))
        rust = port_wpt.generate_rust_fn("check_layout", parser.root)
        self.assertNotIn("<script", template)
        self.assertNotIn("checkLayout", template)
        self.assertNotIn("onload", template)
        self.assertNotIn("div')", template)
        self.assertIn('<div class="box" data-offset-x="0"></div>', template)
        self.assertIn("style.width = Length::px(10.0)", rust)

    def test_mutation_unknown_scripts_handlers_and_dynamic_alignment_stay_rejected(self):
        cases = {
            "inline mutation": (
                "<body onload=\"checkLayout('.box')\"><div class=box></div></body>",
                """
                  <script src="/resources/testharness.js"></script>
                  <script src="/resources/testharnessreport.js"></script>
                  <script src="/resources/check-layout-th.js">mutate()</script>
                """,
            ),
            "unknown script": (
                "<body onload=\"checkLayout('.box')\"><div class=box></div></body>",
                """
                  <script src="/resources/testharness.js"></script>
                  <script src="/resources/testharnessreport.js"></script>
                  <script src="/resources/unknown-helper.js"></script>
                """,
            ),
            "mixed handlers": (
                "<body onload=\"checkLayout('.box')\">"
                "<div class=box onclick=\"mutate()\"></div></body>",
                None,
            ),
            "dynamic alignment": (
                "<body onload=\"target.style.alignSelf='center'; checkLayout('.box')\">"
                "<div id=target class=box></div></body>",
                None,
            ),
            "handler only": (
                "<body><div class=box onclick=\"mutate()\"></div></body>",
                "",
            ),
        }
        for label, (body, scripts) in cases.items():
            with self.subTest(label=label):
                _, parser = self.parse(body, scripts)
                self.assertEqual(
                    port_wpt.analyze_portability(parser),
                    (False, "uses_javascript"),
                )


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

    def test_live_snapshot_accepts_only_exact_authorized_promotions(self):
        baseline, inventory, _, _ = closure.load_ledgers()
        actionable, _ = closure.load_probe_ledgers(inventory)
        with closure.MAPPING_CSV.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        summary = json.loads(closure.SUMMARY_JSON.read_text(encoding="utf-8"))
        promoted = closure.validate_live_snapshot(
            rows, summary, baseline, inventory, actionable
        )
        self.assertEqual(len(EXPECTED_LIVE_PROMOTIONS), 325)
        self.assertEqual(promoted, EXPECTED_LIVE_PROMOTIONS)
        residual_admissions = set(closure.load_w1m_manifests()[0]) - set(actionable)
        residual_admissions |= set(closure.load_w2b_w4_manifests()[0]) - set(actionable)
        self.assertTrue(promoted.issubset(set(actionable) | residual_admissions))

    def test_w1m_manifests_pin_the_complete_atomic_cohort_and_guards(self):
        targets, focused = closure.load_w1m_manifests()
        _, inventory, _, _ = closure.load_ledgers()
        actionable, residuals = closure.load_probe_ledgers(inventory)
        inventory_ids = {item["test_id"] for item in inventory}
        residual_ids = {item["test_id"] for item in residuals}
        self.assertEqual(len(set(targets) & residual_ids), 12)
        self.assertEqual(len(set(targets) - inventory_ids), 17)
        self.assertFalse(set(targets) & set(actionable))
        self.assertEqual(len(set(targets) | closure.W1M_EXISTING_RUNNABLE), 39)
        self.assertTrue(
            set(targets) | closure.W1M_EXISTING_RUNNABLE <= set(focused)
        )

    def test_w1n_manifests_pin_the_four_target_cohort_and_focused_guards(self):
        targets, focused = closure.load_w1n_manifests()
        _, inventory, _, _ = closure.load_ledgers()
        actionable, _ = closure.load_probe_ledgers(inventory)
        self.assertEqual(len(targets), 4)
        self.assertEqual(len(focused), 19)
        self.assertTrue(set(targets) <= set(actionable))
        self.assertTrue(set(targets) <= set(focused))

    def test_w1o_manifests_and_target_splice_are_byte_pinned_and_idempotent(self):
        targets, focused = closure.load_w1o_manifests()
        self.assertEqual(targets, ["wpt/css_flexbox/auto-height-with-flex"])
        self.assertEqual(len(focused), 15)
        self.assertTrue(set(closure.load_w1n_manifests()[0]) <= set(focused))

        mapping = splice_text_port.load_mapping_rows()
        first = splice_text_port.prepare_changes(targets, mapping)
        second = splice_text_port.prepare_changes(targets, mapping)
        self.assertEqual(first[0], second[0])
        self.assertEqual(first[1], second[1])
        self.assertEqual(first[2], second[2])
        rust_path = str(
            ROOT / "bindings/rust/pixel-compare/src/wpt/wpt_css_flexbox.rs"
        )
        fn_name = first[0][0].fn_name
        original_start, original_end = splice_text_port._rust_function_span(
            first[1][rust_path], fn_name
        )
        changed_start, changed_end = splice_text_port._rust_function_span(
            first[2][rust_path], fn_name
        )
        original_fn = first[1][rust_path][original_start:original_end]
        changed_fn = first[2][rust_path][changed_start:changed_end]
        # W1O remains byte-pinned on disk. A no-write regeneration now plans
        # the later deterministic fallback chain, and repeated plans must be
        # byte-identical without silently rewriting the historical builder.
        self.assertIn(
            'FontFamily::Named("DejaVu Sans".to_string())',
            original_fn,
        )
        self.assertNotIn(
            'FontFamily::Named("Droid Sans Fallback".to_string())',
            original_fn,
        )
        self.assertIn(
            'FontFamily::Named("Droid Sans Fallback".to_string())',
            changed_fn,
        )
        manifest_path = splice_text_port.TEXT_PORTED_LIST
        self.assertEqual(first[2][manifest_path], first[1][manifest_path])
        self.assertEqual(second[2][manifest_path], second[1][manifest_path])

    def test_w2a_manifests_membership_and_projected_ledger_are_pinned(self):
        targets, focused = closure.load_w2a_manifests()
        expected = {
            f"wpt/css_flexbox/flexbox-writing-mode-{number:03d}{reference}"
            for number in range(10, 17)
            for reference in ("", "-ref")
        } | {
            "wpt/css_flexbox/flexbox-writing-mode-slr",
            "wpt/css_flexbox/flexbox-writing-mode-slr-row-mix",
            "wpt/css_flexbox/flexbox-writing-mode-slr-row-mix-ref",
            "wpt/css_flexbox/flexbox-writing-mode-slr-rtl",
            "wpt/css_flexbox/flexbox-writing-mode-srl",
            "wpt/css_flexbox/flexbox-writing-mode-srl-row-mix",
            "wpt/css_flexbox/flexbox-writing-mode-srl-row-mix-ref",
            "wpt/css_flexbox/flexbox-writing-mode-srl-rtl",
        }
        self.assertEqual(set(targets), expected)
        self.assertEqual(len(focused), 49)
        self.assertTrue(expected | closure.W2A_EXISTING_EXACT_GUARDS <= set(focused))
        self.assertEqual(
            (
                closure.EXPECTED_W2A_PROMOTIONS,
                closure.EXPECTED_W2A_RUNNABLE,
                closure.EXPECTED_W2A_EXACT,
                closure.EXPECTED_W2A_FAILURES,
                closure.EXPECTED_W2A_UNPORTED,
                closure.EXPECTED_W2A_LIVE_OWNED,
            ),
            (193, 3768, 3487, 281, 3905, 649),
        )

    def test_w2a_two_generations_and_two_surgical_splices_are_byte_identical(self):
        first = closure.load_w2a_manifests()
        second = closure.load_w2a_manifests()
        self.assertEqual(first, second)
        for path, values in zip(
            (closure.W2A_TARGETS_JSON, closure.W2A_FOCUSED_JSON), first
        ):
            self.assertEqual(path.read_text(encoding="utf-8"), closure.encoded(values))

    def test_w2b_w4_manifest_projection_and_area_partition_are_pinned(self):
        targets, focused = closure.load_w2b_w4_manifests()
        _, inventory, _, _ = closure.load_ledgers()
        actionable, _ = closure.load_probe_ledgers(inventory)
        old_promotions = set(focused) - set(targets)
        self.assertEqual(old_promotions, set(EXPECTED_W2A_LIVE_PROMOTIONS))
        self.assertEqual(set(actionable) - old_promotions, set(targets) & set(actionable))
        self.assertEqual(
            set(targets) - set(actionable),
            {
                "wpt/css_flexbox/css-flexbox-test1",
                "wpt/css_flexbox/css-flexbox-test1-ref",
            },
        )
        counts = {}
        for test_id in targets:
            area = test_id.split("/")[1]
            counts[area] = counts.get(area, 0) + 1
        self.assertEqual(counts, {
            "css2_floats": 1,
            "css_backgrounds": 3,
            "css_break": 37,
            "css_flexbox": 43,
            "css_multicol": 5,
            "css_overflow": 20,
            "css_position": 1,
            "css_sizing": 22,
        })
        self.assertEqual(
            (
                closure.EXPECTED_W2B_W4_PROMOTIONS,
                closure.EXPECTED_W2B_W4_RUNNABLE,
                closure.EXPECTED_W2B_W4_EXACT,
                closure.EXPECTED_W2B_W4_FAILURES,
                closure.EXPECTED_W2B_W4_UNPORTED,
                closure.EXPECTED_W2B_W4_LIVE_OWNED,
            ),
            (325, 3889, 3619, 270, 3784, 517),
        )

    def test_sp17_fallback_font_hashes_and_registration_are_symmetric(self):
        font_dir = ROOT / "bindings/rust/openui-text/fonts"
        expected = {
            "DroidSansFallback-reduced.ttf": "27db42b79d0846f6fd01b3d6a8233df9a8a5ece80b042299dc4174c48213ffd3",
            "NotoSansDevanagari-Regular.ttf": "b1dffa1fccb30dc45287111834a9db15c652b05d4d67201abe73e67717017590",
            "NotoColorEmoji.ttf": "72a635cb3d2f3524c51620cdde406b217204e8a6a06c6a096ff8ed4b5fd6e27b",
        }
        for name, digest in expected.items():
            self.assertEqual(closure.hashlib.sha256((font_dir / name).read_bytes()).hexdigest(), digest)
        families = [
            "Ahem",
            "Droid Sans Fallback",
            "Noto Sans Devanagari",
            "Noto Color Emoji",
            "DejaVu Sans",
        ]
        fontconfig = (
            ROOT / "tools/accountability/data/fonts/ahem_noaa.conf"
        ).read_text(encoding="utf-8")
        cache = (
            ROOT / "bindings/rust/openui-text/src/font/cache.rs"
        ).read_text(encoding="utf-8")
        self.assertEqual(
            families,
            re.findall(r"<string>([^<]+)</string>", fontconfig),
        )
        self.assertEqual(
            families,
            sorted(families, key=cache.index),
        )
        self.assertEqual(
            families,
            sorted(families, key=port_wpt.TEXT_TEMPLATE_OVERRIDE.index),
        )
        self.assertEqual(
            families,
            sorted(families, key=port_wpt.DETERMINISTIC_FONT_FAMILY_RUST.index),
        )
        self.assertEqual(
            json.loads((closure.PORTED_DIR / "sp17_freetype_text_tests.json").read_text()),
            [
                "wpt/css_flexbox/css-flexbox-test1",
                "wpt/css_flexbox/css-flexbox-test1-ref",
            ],
        )

    def test_full_w2_metadata_and_live_snapshot_invariants_are_exact(self):
        baseline, inventory, _, _ = closure.load_ledgers()
        actionable, _ = closure.load_probe_ledgers(inventory)
        with closure.MAPPING_CSV.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        summary = json.loads(closure.SUMMARY_JSON.read_text(encoding="utf-8"))
        promoted = closure.validate_live_snapshot(
            rows, summary, baseline, inventory, actionable
        )
        targets, focused = closure.load_w2b_w4_manifests()
        self.assertEqual(promoted, set(focused))
        self.assertTrue(set(targets) <= promoted)
        text_manifest = json.loads(
            (closure.PORTED_DIR / "text_ported_tests.json").read_text(encoding="utf-8")
        )
        self.assertEqual(len(text_manifest), 1289)
        self.assertTrue(set(targets) <= set(text_manifest))

    def test_frozen_ledgers_and_sp13r_later_promotion_allowlist_are_deterministic(self):
        closure.validate_historical_ledgers()
        self.assertEqual(len(sp13r_closure.LATER_EXACT_PROMOTIONS), 80)
        targets = set(closure.load_w2b_w4_manifests()[0])
        residuals = {
            item["test_id"] for item in sp13r_closure.load_ledgers()[2]
        }
        self.assertEqual(len(targets & residuals), 31)
        self.assertTrue(targets & residuals <= sp13r_closure.LATER_EXACT_PROMOTIONS)

    def test_w2b_w4_two_no_write_generations_and_two_splices_match(self):
        with closure.MAPPING_CSV.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        summary = json.loads(closure.SUMMARY_JSON.read_text(encoding="utf-8"))
        first_generation = closure.build_outputs(rows, summary)
        second_generation = closure.build_outputs(rows, summary)
        self.assertEqual(first_generation, second_generation)

        targets = closure.load_w2b_w4_manifests()[0]
        mapping = splice_text_port.load_mapping_rows()
        first_splice = splice_text_port.prepare_changes(targets, mapping)
        second_splice = splice_text_port.prepare_changes(targets, mapping)
        self.assertEqual(first_splice, second_splice)
        self.assertEqual(set(first_splice[1]), set(first_splice[2]))
        self.assertTrue(first_splice[2])

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

    def test_w2a_sideways_inheritance_emission_and_fragment_metadata(self):
        root = port_wpt.DomNode(
            "body",
            {},
            port_wpt.parse_inline_styles(
                "writing-mode:sideways-lr;direction:rtl;text-orientation:sideways"
            ),
        )
        child = port_wpt.DomNode("span", {}, OrderedDict())
        text = port_wpt.DomNode("#text", {}, OrderedDict())
        text.is_text = True
        text.text_content = "Ahem"
        child.children.append(text)
        root.children.append(child)
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        output = port_wpt.generate_rust_fn("w2a_sideways", root)
        self.assertGreaterEqual(output.count("writing_mode = WritingMode::SidewaysLr"), 3)
        self.assertGreaterEqual(output.count("direction = Direction::Rtl"), 3)
        self.assertGreaterEqual(
            output.count("text_orientation = TextOrientation::Sideways"), 3
        )

        fragment = (ROOT / "bindings/rust/openui-layout/src/fragment.rs").read_text()
        inline = (
            ROOT / "bindings/rust/openui-layout/src/inline/algorithm.rs"
        ).read_text()
        painter = (ROOT / "bindings/rust/openui-paint/src/painter.rs").read_text()
        self.assertIn("pub enum TextRunOrientation", fragment)
        self.assertIn("pub text_run_orientation: TextRunOrientation", fragment)
        self.assertIn("resolve_text_run_orientation(style, text_content)", inline)
        self.assertIn("fragment.text_run_orientation", painter)
        self.assertNotIn("is_homogeneous_rotated_vertical_run", painter)

    def test_w2b_w4_bidi_orientation_and_fragment_metadata_emission(self):
        style_target = "doc.node_mut(n1).style"
        expected = {
            "normal": "UnicodeBidi::Normal",
            "embed": "UnicodeBidi::Embed",
            "bidi-override": "UnicodeBidi::Override",
            "isolate": "UnicodeBidi::Isolate",
            "isolate-override": "UnicodeBidi::IsolateOverride",
            "plaintext": "UnicodeBidi::Plaintext",
        }
        for value, enum_value in expected.items():
            output = port_wpt.generate_single_style(
                "unicode-bidi", value, style_target, 16.0
            )
            self.assertIn(f"unicode_bidi = {enum_value}", output)

        builder = (
            ROOT / "bindings/rust/openui-layout/src/inline/items_builder.rs"
        ).read_text(encoding="utf-8")
        fragment = (
            ROOT / "bindings/rust/openui-layout/src/fragment.rs"
        ).read_text(encoding="utf-8")
        painter = (
            ROOT / "bindings/rust/openui-paint/src/painter.rs"
        ).read_text(encoding="utf-8")
        self.assertIn("fn bidi_open_chars", builder)
        self.assertIn("fn bidi_close_chars", builder)
        self.assertIn(
            "debug_assert_ne!(orientation, TextRunOrientation::UnresolvedMixed)",
            builder,
        )
        self.assertIn("pub enum TextRunOrientation", fragment)
        self.assertIn("fragment.text_combine", painter)
        self.assertNotIn("style.text_orientation", painter)

    def test_flex_shorthand_zero_percent_and_semantic_break_metrics(self):
        style = "doc.node_mut(n1).style"
        for value in ("1", "1 1", "1 1 0"):
            output = "\n".join(
                port_wpt.generate_single_style("flex", value, style, 16.0)
            )
            self.assertIn("flex_basis = Length::percent(0.0)", output)
            self.assertNotIn("flex_basis = Length::px(0.0)", output)
        explicit = {
            "1 1 0px": "Length::px(0.0)",
            "1 1 0%": "Length::percent(0.0)",
            "1 1 auto": "Length::auto()",
            "1 1 4px": "Length::px(4.0)",
        }
        for value, basis in explicit.items():
            output = "\n".join(
                port_wpt.generate_single_style("flex", value, style, 16.0)
            )
            self.assertIn(f"flex_basis = {basis}", output)

        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body style="font-size:22px;line-height:30px;font-family:serif;'
            'writing-mode:vertical-rl;direction:rtl;text-orientation:upright">'
            '<div>A<br><br style="display:none">B'
            '<br style="display:contents">C<br clear="both">D</div></body>'
        )
        rust = port_wpt.generate_rust_fn("semantic_breaks", parser.root)
        self.assertEqual(rust.count("ElementTag::Break"), 2)
        self.assertNotIn('Some("\\n".to_string())', rust)
        first_break = rust.split("ElementTag::Break", 1)[1].split(
            "doc.append_child", 1
        )[0]
        for line in (
            "font_size = 22.0",
            "font_family = FontFamilyList",
            "line_height = LineHeight::Length(30.0)",
            "writing_mode = WritingMode::VerticalRl",
            "direction = Direction::Rtl",
            "text_orientation = TextOrientation::Upright",
        ):
            self.assertIn(line, first_break)
        self.assertIn("style.clear = Clear::Both", rust)

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
