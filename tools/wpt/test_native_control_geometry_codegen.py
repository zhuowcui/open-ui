"""Typed native geometry and the frozen-template surgical transaction."""
import contextlib
import copy
import io
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools.wpt import engine_fixture_codegen as lower
from tools.wpt import port_wpt as port
from tools.wpt import splice_text_port as splice


class NativeControlGeometry(unittest.TestCase):
    def setUp(self):
        self.previous = (port.ACTIVE_PORTER_PROFILE, port.RETAIN_TEXT, port.EMIT_TEXT_NODES)
        port.set_porter_profile(port.PorterProfile.LEGACY_BOX_ONLY, retain_text=False)

    def tearDown(self):
        port.ACTIVE_PORTER_PROFILE, port.RETAIN_TEXT, port.EMIT_TEXT_NODES = self.previous

    def generated(self, markup):
        parsed = port._parse_wpt_markup(markup, "/tmp")
        new = port.generate_rust_fn("native_geometry", copy.deepcopy(parsed.root), parsed.html_styles)
        old = port.generate_rust_fn("native_geometry", copy.deepcopy(parsed.root), parsed.html_styles,
                                    _legacy_control_geometry_projection=True)
        return old, new

    def test_real_source_roles_request_native_geometry_without_dimensions(self):
        old, new = self.generated("<body><textarea cols='3' rows='4'></textarea><input size='5'><input type='button'></body>")
        requests = lower.native_control_geometry_requests(new)
        self.assertEqual([role for _, role in requests], ["TextArea", "TextInput"])
        projected, rows = splice.project_native_control_geometry(old, old, new)
        self.assertEqual(len(rows), 2)
        self.assertEqual(projected, new)
        # Button/resource handling belongs to its existing distinct route.
        self.assertEqual(new.count("ReplacedResourceKind::TransparentCanvas"), 1)
        again, _ = splice.project_native_control_geometry(projected, old, new)
        self.assertEqual(again, projected)

    def test_projection_preserves_unrelated_source_and_rejects_explicit_metadata(self):
        old, new = self.generated("<body><textarea></textarea></body>")
        unrelated = "\n// stable unrelated source\nlet retained_width = 123.0;\n"
        projected, _ = splice.project_native_control_geometry(old + unrelated, old, new)
        self.assertTrue(projected.endswith(unrelated))
        # An explicit renderer dimension/resource is not the UA statement.
        with self.assertRaisesRegex(ValueError, "old metadata differs"):
            splice.project_native_control_geometry(old.replace("Some(176.0)", "Some(9.0)"), old, new)
        with self.assertRaisesRegex(ValueError, "old metadata differs"):
            splice.project_native_control_geometry(old.replace("ReplacedResourceKind::TransparentCanvas", "ReplacedResourceKind::Image(ImageResourceId::new(1))"), old, new)

    def test_missing_duplicate_and_mismatched_roles_fail_closed(self):
        old, new = self.generated("<body><textarea></textarea></body>")
        node, _ = lower.native_control_geometry_requests(new)[0]
        with self.assertRaisesRegex(ValueError, "source role differs"):
            splice.project_native_control_geometry(old.replace("FormControlRole::TextArea", "FormControlRole::TextInput"), old, new)
        with self.assertRaisesRegex(ValueError, "source (tag/identity|ancestry) differs"):
            splice.project_native_control_geometry(old.replace("ElementTag::TextArea", "ElementTag::Input"), old, new)
        with self.assertRaisesRegex(ValueError, "ambiguous or missing"):
            splice.project_native_control_geometry(old + f"doc.set_node_state({node}, RendererNodeState::Replaced(None));", old, new)
        with self.assertRaisesRegex(ValueError, "missing or duplicate"):
            splice.project_native_control_geometry(old, old, new + lower.native_control_geometry_request(node, "TextArea"))

    def test_typed_directives_validate_handles_roles_and_multiline_rust(self):
        with self.assertRaises(ValueError):
            lower.native_control_geometry_request("node()", "TextArea")
        with self.assertRaises(ValueError):
            lower.native_control_geometry_request("n1", "Range")
        with self.assertRaisesRegex(ValueError, "unsupported"):
            lower.engineify_module("doc.request_native_control_geometry(n1, openui_dom::FormControlRole::Range);")
        self.assertEqual(lower.native_control_geometry_requests(
            "doc.request_native_control_geometry(\nn1,\nopenui_dom::FormControlRole::TextArea,\n);"),
            [("n1", "TextArea")])

    def test_historical_source_is_checksum_bound_and_reverse_projection_is_lossless(self):
        old, new = self.generated("<body><textarea></textarea></body>")
        node, role = lower.native_control_geometry_requests(new)[0]
        statement = old[slice(*splice._native_control_state_spans(old, node, "Replaced")[0])]
        proof = [{"node": node, "role": role, "statement_sha256": hashlib.sha256(statement.encode()).hexdigest()}]
        self.assertEqual(splice.reverse_native_control_geometry(new, old, proof), old)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.json"
            raw = json.dumps({"schema_version": 1, "source_only": True, "node_count": 0, "nodes": [], "modules": []}).encode()
            path.write_bytes(raw)
            self.assertEqual(splice.load_native_control_source_inventory(str(path), hashlib.sha256(raw).hexdigest())['schema_version'], 1)
            path.write_bytes(raw + b" ")
            with self.assertRaisesRegex(ValueError, "checksum differs"):
                splice.load_native_control_source_inventory(str(path), hashlib.sha256(raw).hexdigest())

    def test_check_reports_drift_without_publishing_any_path(self):
        args = ["splice", "--native-control-geometry-only", "--preserve-templates", "--native-control-source-inventory", "source.json", "--native-control-source-sha256", "0" * 64, "--check", "wpt/css_flexbox/sample"]
        with patch.object(sys, "argv", args), \
             patch.object(splice, "load_mapping_rows", return_value={}), \
             patch.object(splice, "load_native_control_source_inventory", return_value={}), \
             patch.object(splice, "prepare_native_control_geometry_changes", return_value=([{"requests": [{"node": "n1"}]}], {"owned.rs": "old"}, {"owned.rs": "new"})), \
             patch.object(splice, "commit_changes") as commit, \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(splice.main(), 1)
            commit.assert_not_called()
        with patch.object(sys, "argv", args), \
             patch.object(splice, "load_mapping_rows", return_value={}), \
             patch.object(splice, "load_native_control_source_inventory", return_value={}), \
             patch.object(splice, "prepare_native_control_geometry_changes", return_value=([{"requests": []}], {}, {})), \
             patch.object(splice, "commit_changes") as commit, \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(splice.main(), 0)
            commit.assert_not_called()


if __name__ == "__main__":
    unittest.main()
