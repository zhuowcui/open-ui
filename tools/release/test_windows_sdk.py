from __future__ import annotations

import importlib.util
import tempfile
import unittest
import zipfile
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("build_v02_windows.py")
SPEC = importlib.util.spec_from_file_location("build_v02_windows", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
sdk = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sdk)


class WindowsSdkTests(unittest.TestCase):
    def test_zip_is_byte_identical_sorted_and_time_normalized(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "stage"
            (source / "lib/cmake").mkdir(parents=True)
            (source / "include").mkdir()
            (source / "lib/cmake/b.txt").write_text("b\n", encoding="utf-8")
            (source / "include/a.h").write_text("a\n", encoding="utf-8")
            first, second = root / "first.zip", root / "second.zip"
            sdk.deterministic_zip(source, first, "sdk", 1_700_000_000)
            (source / "include/a.h").touch()
            sdk.deterministic_zip(source, second, "sdk", 1_700_000_000)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                names = archive.namelist()
                self.assertEqual(names, sorted(names))
                self.assertIn("sdk/include/a.h", names)
                self.assertEqual({info.date_time for info in archive.infolist()},
                                 {(2023, 11, 14, 22, 13, 20)})

    def test_zip_times_start_at_the_zip_epoch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "stage").mkdir()
            (root / "stage/file").write_bytes(b"x")
            sdk.deterministic_zip(root / "stage", root / "out.zip", "sdk", 0)
            with zipfile.ZipFile(root / "out.zip") as archive:
                self.assertEqual(archive.infolist()[0].date_time, (1980, 1, 1, 0, 0, 0))

    def test_library_location_requires_the_dll_and_its_import_library(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "openui_ffi.dll").write_bytes(b"dll")
            with self.assertRaises(SystemExit):
                sdk.locate_libraries(directory)
            (directory / "openui_ffi.dll.lib").write_bytes(b"lib")
            dll, import_library, pdb = sdk.locate_libraries(directory)
            self.assertEqual((dll.name, import_library.name, pdb), ("openui_ffi.dll", "openui_ffi.dll.lib", None))


if __name__ == "__main__":
    unittest.main()
