import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import port_coverage_area as COVERAGE  # noqa: E402

FONTS = 'FontFamilyList { families: vec![FontFamily::Named("Ahem".to_string())] }'


def builder(name: str, document: str = "base_doc", body: str = "") -> tuple[str, str, str]:
    fn_name = f"demo_{name}"
    rust = (
        f"fn {fn_name}(viewport: ViewportMetrics) -> Result<Engine, EngineError> {{\n"
        f"    let (mut doc, vp) = {document}(viewport);\n"
        f"    doc.set_style(vp, RendererStyleValue::FontFamily({FONTS}));\n{body}"
        "    doc.finish()\n}"
    )
    return f"wpt/demo/{name}", fn_name, rust


class PortCoverageAreaTests(unittest.TestCase):
    def test_parts_split_by_size_and_register_every_builder(self):
        builders = [builder(f"t{index}", body="    // " + "x" * 40 + "\n") for index in range(5)]
        with patch.object(COVERAGE, "PART_BYTES", 2 * len(builders[0][2])):
            outputs = COVERAGE.module_sources("demo", builders)
        directory = COVERAGE.RUST_WPT_DIR / "wpt_demo"
        self.assertEqual(
            sorted(path.name for path in outputs),
            ["mod.rs", "part_0.rs", "part_1.rs", "part_2.rs"],
        )
        module = outputs[directory / "mod.rs"]
        self.assertIn("mod part_2;", module)
        self.assertIn("all.extend(part_2::registry());", module)
        registered = "".join(
            text for path, text in outputs.items() if path.name.startswith("part_")
        )
        for test_id, fn_name, _ in builders:
            self.assertEqual(registered.count(f'("{test_id}", {fn_name} as'), 1)

    def test_font_list_is_built_once_per_part(self):
        source = COVERAGE.part_source("demo", 0, [builder("a"), builder("b")])
        self.assertEqual(source.count(FONTS), 1)
        self.assertEqual(source.count("FontFamily(font_families_0())"), 2)

    def test_part_imports_only_the_documents_it_uses(self):
        base = COVERAGE.part_source("demo", 0, [builder("a")])
        self.assertIn("use crate::base_doc;", base)
        self.assertNotIn("root_doc", base)
        both = COVERAGE.part_source(
            "demo", 0, [builder("a"), builder("b", document="root_doc")]
        )
        self.assertIn("use crate::{base_doc, root_doc};", both)

    def test_unique_names_flatten_subdirectories(self):
        with tempfile.TemporaryDirectory() as directory:
            base = pathlib.Path(directory)
            nested = base / "alignment" / "grid-001.html"
            self.assertEqual(COVERAGE.unique_name(base, nested), "alignment_grid-001")


if __name__ == "__main__":
    unittest.main()
