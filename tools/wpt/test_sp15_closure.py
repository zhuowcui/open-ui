"""Focused SP15 ledger, porter, and supersession regressions."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
ACCOUNTABILITY = ROOT / "tools" / "accountability"
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ACCOUNTABILITY))

import generate_sp15_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import splice_text_port  # noqa: E402

_audit_spec = importlib.util.spec_from_file_location(
    "sp15_audit", ACCOUNTABILITY / "audit.py"
)
audit = importlib.util.module_from_spec(_audit_spec)
_audit_spec.loader.exec_module(audit)


class LedgerTests(unittest.TestCase):
    def test_frozen_ledgers_are_sorted_disjoint_and_complete(self):
        baseline, targets, residuals = closure.load_ledgers()
        self.assertEqual(len(baseline), 2767)
        self.assertEqual(len(targets), 76)
        self.assertEqual(len(residuals), 54)
        residual_ids = {item["test_id"] for item in residuals}
        self.assertFalse(set(targets) & residual_ids)
        self.assertEqual(len(set(targets) | residual_ids), 130)

    def test_residuals_have_reason_backed_non_sp15_owners(self):
        _, _, residuals = closure.load_ledgers()
        for item in residuals:
            self.assertTrue(item["rejection_reason"])
            owners = set(item["owner_categories"])
            self.assertFalse(owners & closure.SP15_CATEGORIES)
            self.assertTrue(owners - closure.METADATA_CATEGORIES)

    def test_root_aware_membership_survives_mapping_category_retirement(self):
        _, targets, _ = closure.load_ledgers()
        root_aware = splice_text_port.load_root_aware_ids()
        promoted_root_body = set(targets) & root_aware
        self.assertEqual(len(promoted_root_body), 49)
        self.assertIn(
            "wpt/css_backgrounds/background-color-body-propagation-004",
            promoted_root_body,
        )


class RootAwarePorterTests(unittest.TestCase):
    def setUp(self):
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        self.temp = tempfile.TemporaryDirectory()

    def tearDown(self):
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain
        self.temp.cleanup()

    def html(self, source: str) -> Path:
        path = Path(self.temp.name) / "case.html"
        path.write_text(source, encoding="utf-8")
        return path

    def test_root_selector_is_not_applied_to_body(self):
        path = self.html(
            "<!doctype html><style>:root{width:400px;height:200px}"
            "body{background:red}</style><body></body>"
        )
        parser = port_wpt.parse_wpt_html(str(path), root_aware=True)
        self.assertEqual(parser.html_styles["width"], "400px")
        self.assertNotIn("width", parser.root.styles)
        self.assertEqual(parser.root.styles["background"], "red")

    def test_root_builder_and_template_preserve_document_nodes(self):
        path = self.html(
            "<!doctype html><html style='height:100%'><style>body{color:red}</style>"
            "<body style='overflow:hidden'><div></div></body></html>"
        )
        parser = port_wpt.parse_wpt_html(str(path), root_aware=True)
        rust = port_wpt.generate_rust_fn(
            "demo", parser.root, parser.html_styles, root_aware=True
        )
        template = port_wpt.generate_html_template(str(path), root_aware=True)
        self.assertIn("let (mut doc, html, vp) = root_doc()", rust)
        self.assertIn("doc.node_mut(html).style.height", rust)
        self.assertIn("doc.node_mut(vp).style.overflow_x", rust)
        self.assertTrue(template.startswith("<!--OPENUI_ROOT_AWARE-->"))
        self.assertIn("html {height:100%}", template)
        self.assertIn("body {overflow:hidden}", template)

    def test_visible_style_content_survives_but_contents_br_is_suppressed(self):
        path = self.html(
            "<!doctype html><style>*{display:contents}</style><body>"
            "<br><whatever>PASS</whatever></body>"
        )
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        self.assertIn("*{display:contents}", rust)
        self.assertIn("PASS", rust)
        self.assertNotIn("ElementTag::Break", rust)
        self.assertIn("doc.node_mut(vp).style.display = Display::None", rust)

    def test_display_contents_resets_non_inherited_background_boundary(self):
        path = self.html(
            "<!doctype html><style>.outer{display:contents;background:blue}"
            ".middle{display:contents}.leaf{background:inherit}</style><body>"
            "<div class='outer'><div class='middle'><span class='leaf'>X</span>"
            "</div></div></body>"
        )
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        # The intermediate unboxed element's computed transparent background,
        # rather than the outer blue value, is inherited by the leaf.
        self.assertNotIn("background_color = Color::BLUE", rust)


class SupersessionTests(unittest.TestCase):
    def test_sp14_w4_entry_can_be_superseded_by_sp15_promotion(self):
        test_id = "wpt/css_backgrounds/example"
        row = {
            "sp_area": "css_backgrounds",
            "test_name": "example",
            "ported": "yes",
            "our_test_id": test_id,
            "chromium_test_path": "css-backgrounds/example.html",
            "failure_category": "",
            "notes": "",
        }
        w4 = [{
            "test_id": test_id,
            "chromium_test_path": "css-backgrounds/example.html",
            "rejection_reason": "no_layout_content",
            "rejection_owner": "needs_root_body_layout",
            "owner_categories": ["needs_root_body_layout"],
        }]
        errors = audit.sp14_text_closure_errors(
            [row],
            {test_id: {"id": test_id, "status": "pass", "mismatch_pct": 0.0}},
            {test_id: "template"},
            {test_id},
            [],
            [],
            w4,
            enforce_frozen_counts=False,
            superseded_actionable={test_id},
        )
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
