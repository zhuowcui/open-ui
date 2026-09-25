import importlib.util
import json
import pathlib
import re
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "port_wpt", ROOT / "tools" / "wpt" / "port_wpt.py"
)
PORT_WPT = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(PORT_WPT)


class RendererGeneralityTests(unittest.TestCase):
    def test_ast_mutation_ir_materializes_final_dom_before_cascade(self):
        mutation_ir = {
            "lowerable": True,
            "operations": [
                {
                    "kind": "bind-target",
                    "order": 0,
                    "binding": "target",
                    "target": {
                        "kind": "dom-query",
                        "method": "getElementById",
                        "argument": "target",
                        "constant": True,
                    },
                },
                {
                    "kind": "layout-barrier",
                    "order": 1,
                    "property": "offsetTop",
                    "target": {"kind": "binding", "name": "target"},
                },
                {
                    "kind": "class-list",
                    "order": 2,
                    "action": "add",
                    "tokens": ["active"],
                    "target": {"kind": "binding", "name": "target"},
                },
                {
                    "kind": "set-style",
                    "order": 3,
                    "target": {"kind": "binding", "name": "target"},
                    "cascade_delta": {"css_name": "width", "value": "50px"},
                },
                {
                    "kind": "create-element",
                    "order": 4,
                    "source_offset": 100,
                    "binding": None,
                    "tag_name": "span",
                },
                {
                    "kind": "append",
                    "order": 5,
                    "target": {"kind": "binding", "name": "target"},
                    "children": [{"kind": "created-node", "source_offset": 100}],
                },
            ],
        }
        parser = PORT_WPT._parse_wpt_markup(
            "<style>.active { width: 10px; color: green }</style>"
            "<body><div id='target' style='height: 5px'></div></body>",
            str(ROOT),
            mutation_ir=mutation_ir,
        )
        target = parser.root.children[0]
        self.assertEqual(target.attrs["class"], "active")
        self.assertEqual(target.styles["width"], "50px")
        self.assertEqual(target.styles["height"], "5px")
        self.assertEqual(target.styles["color"], "green")
        self.assertEqual([child.tag for child in target.children], ["span"])
        self.assertEqual(
            parser.lowered_layout_barriers,
            [{"order": 1, "property": "offsetTop", "method": None}],
        )
        self.assertTrue(PORT_WPT.analyze_portability(parser)[0])

    def test_ast_mutation_ir_emits_script_free_final_state_template(self):
        mutation_ir = {
            "lowerable": True,
            "operations": [
                {
                    "kind": "class-list",
                    "order": 0,
                    "action": "add",
                    "tokens": ["active"],
                    "target": {
                        "kind": "dom-query",
                        "method": "getElementById",
                        "argument": "target",
                    },
                },
                {
                    "kind": "set-style",
                    "order": 1,
                    "target": {
                        "kind": "dom-query",
                        "method": "getElementById",
                        "argument": "target",
                    },
                    "cascade_delta": {"css_name": "width", "value": "50px"},
                },
                {
                    "kind": "create-element",
                    "order": 2,
                    "source_offset": 10,
                    "tag_name": "span",
                },
                {
                    "kind": "append",
                    "order": 3,
                    "target": {
                        "kind": "dom-query",
                        "method": "getElementById",
                        "argument": "target",
                    },
                    "children": [{"kind": "created-node", "source_offset": 10}],
                },
            ],
        }
        previous = PORT_WPT.ACTIVE_PORTER_PROFILE
        previous_emit = PORT_WPT.EMIT_TEXT_NODES
        previous_retain = PORT_WPT.RETAIN_TEXT
        try:
            PORT_WPT.set_porter_profile(PORT_WPT.PorterProfile.DETERMINISTIC_AHEM)
            with tempfile.TemporaryDirectory() as temp:
                source = pathlib.Path(temp, "candidate.html")
                source.write_text(
                    "<style>.active { width: 10px; color: green }</style>"
                    "<body data-state='final'><div id='target'></div>"
                    "<script>target.classList.add('active')</script></body>",
                    encoding="utf-8",
                )
                parser = PORT_WPT.parse_wpt_html(
                    str(source), mutation_ir=mutation_ir
                )
                template = PORT_WPT.generate_html_template(
                    str(source), final_state_parser=parser
                )
        finally:
            PORT_WPT.ACTIVE_PORTER_PROFILE = previous
            PORT_WPT.EMIT_TEXT_NODES = previous_emit
            PORT_WPT.RETAIN_TEXT = previous_retain

        self.assertNotIn("<script", template)
        self.assertIn('class="active"', template)
        self.assertIn('style="width: 50px"', template)
        self.assertIn("<span></span>", template)
        self.assertTrue(template.startswith("<!--OPENUI_FINAL_HTML_ATTRS:"))

    def test_viewport_lengths_stay_semantic(self):
        self.assertEqual(
            PORT_WPT.parse_length("25vw"),
            "crate::fixture_viewport_length(LengthValue::ViewportWidth(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("25vh"),
            "crate::fixture_viewport_length(LengthValue::ViewportHeight(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("25vmin"),
            "crate::fixture_viewport_length(LengthValue::ViewportMin(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("25vmax"),
            "crate::fixture_viewport_length(LengthValue::ViewportMax(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("calc(60vh - 6px)"),
            "crate::fixture_viewport_calc(LengthValue::ViewportHeight(60.0), -6.000000)",
        )

    def test_static_geometry_and_transforms_never_assume_800_by_600(self):
        self.assertIsNone(PORT_WPT._css_length_px("10vw"))
        self.assertIsNone(PORT_WPT._css_length_px("10vh"))
        self.assertIsNone(PORT_WPT._parse_calc_token("10vmin"))
        self.assertIsNone(PORT_WPT._parse_calc_token("10vmax"))
        self.assertEqual(
            PORT_WPT._transform_2d_rust("translateX(10vw)", 16.0),
            "Transform2D { a: 1.0, b: 0.0, c: 0.0, d: 1.0, "
            "e: (crate::fixture_viewport_px(LengthValue::ViewportWidth(10.0))), f: 0.0 }",
        )
        self.assertEqual(
            PORT_WPT._transform_2d_rust(
                "translateY(-50%) translateY(100vh)", 16.0, (50.0, 50.0)
            ),
            "Transform2D { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, "
            "f: (-25.0) + (crate::fixture_viewport_px(LengthValue::ViewportHeight(100.0))) }",
        )
        self.assertEqual(
            PORT_WPT.parse_length("calc(50% - 100vh)"),
            "Length::calc_percent_px(50.000000, "
            "-(crate::fixture_viewport_px(LengthValue::ViewportHeight(100.0))))",
        )
        self.assertEqual(
            PORT_WPT.generate_single_style(
                "font-size", "40vh", "style", 16.0
            ),
            "style.font_size = crate::fixture_viewport_px(LengthValue::ViewportHeight(40.0));",
        )

    def test_committed_viewport_unit_fixtures_are_runtime_responsive(self):
        templates = json.loads(
            (
                ROOT
                / "tools/accountability/data/wpt_ported/all_wpt_templates.json"
            ).read_text(encoding="utf-8")
        )
        manifest = set(
            json.loads(
                (
                    ROOT / "tools/qualification/manifests/complete-5731.json"
                ).read_text(encoding="utf-8")
            )
        )
        viewport_unit = re.compile(
            r"(?<![-\w.])(?:\d*\.)?\d+(?:vw|vh|vmin|vmax)\b", re.I
        )
        modules = {}
        stale = []
        audited = 0
        for test_id, template in templates.items():
            if test_id not in manifest or not viewport_unit.search(template):
                continue
            audited += 1
            _, module, name = test_id.split("/", 2)
            source = modules.setdefault(
                module,
                (
                    ROOT
                    / f"bindings/rust/pixel-compare/src/wpt/wpt_{module}.rs"
                ).read_text(encoding="utf-8"),
            )
            function = f"{module}_{PORT_WPT.sanitize_fn_name(name)}"
            start = source.find(f"fn {function}(")
            self.assertGreaterEqual(start, 0, test_id)
            end = source.find("\nfn ", start + 1)
            body = source[start : end if end >= 0 else len(source)]
            if "crate::fixture_viewport_" not in body:
                stale.append(test_id)
        self.assertGreater(audited, 0)
        self.assertEqual(stale, [])


if __name__ == "__main__":
    unittest.main()
