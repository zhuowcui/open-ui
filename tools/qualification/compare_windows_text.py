#!/usr/bin/env python3
"""Compare Windows Open UI with Windows Chrome on the Ahem text-port tests.

The Linux renderer census compares Linux Open UI with Linux Chrome. On
Windows both sides rasterize text with DirectWrite, so this tool measures the
same expanded-suite pages against Chrome-for-Testing 147.0.7727.50 `win64`:

* pages are built from the checked-in templates with the census harness's
  `build_html_document`;
* the bundled faces that the Linux oracle installs through Fontconfig are
  supplied with `@font-face` from the repository bytes, never installed;
* Chrome runs with `NoFontAntialiasing` (Open UI's deterministic aliased
  policy), plus the census's SP18 line-clamp features for SP18 targets;
* Open UI renders with the census's raster configuration for each test.

Only the registered-Ahem font profile is selected; real-font tests use a
Linux FreeType policy with no Windows counterpart yet. The report records
per-test RGBA hashes and, when a Linux census summary is given, the Linux
status of every test so platform-specific outcomes can be separated.
"""
from __future__ import annotations

import argparse
import base64
import json
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path


QUALIFICATION_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(QUALIFICATION_DIR))
import run_renderer_matrix as matrix  # noqa: E402
import windows_chrome  # noqa: E402

ROOT = matrix.ROOT
FONTS = ROOT / "bindings/rust/openui-text/fonts"
# Family, file and weight of every face the Linux oracle's Fontconfig profile
# provides from the same repository bytes.
BUNDLED_FACES = (
    ("Ahem", "Ahem.ttf", 400),
    ("Droid Sans Fallback", "DroidSansFallback-reduced.ttf", 400),
    ("Noto Sans Devanagari", "NotoSansDevanagari-Regular.ttf", 400),
    ("Noto Color Emoji", "NotoColorEmoji.ttf", 400),
    ("DejaVu Sans", "DejaVuSans.ttf", 400),
    ("DejaVu Sans", "DejaVuSans-Bold.ttf", 700),
    ("DejaVu Sans Mono", "DejaVuSansMono.ttf", 400),
    ("DejaVu Sans Mono", "DejaVuSansMono-Bold.ttf", 700),
    ("DejaVu Serif", "DejaVuSerif.ttf", 400),
    ("DejaVu Serif", "DejaVuSerif-Bold.ttf", 700),
)
ALIASED_FEATURE = "NoFontAntialiasing"
SCHEMA_VERSION = 1


def bundled_font_faces_css(fonts: Path = FONTS) -> str:
    rules = []
    for family, name, weight in BUNDLED_FACES:
        data = base64.b64encode((fonts / name).read_bytes()).decode("ascii")
        rules.append(
            f"@font-face{{font-family:'{family}';font-weight:{weight};"
            f"src:url(data:font/truetype;base64,{data})}}"
        )
    return "<style>" + "".join(rules) + "</style>"


def windows_page(html_document: str, faces_css: str) -> str:
    """Insert the bundled faces before any author style."""
    match = re.search(r"<head\b[^>]*>", html_document, re.IGNORECASE)
    if not match:
        raise ValueError("census page has no <head>")
    return html_document[: match.end()] + faces_css + html_document[match.end():]


def chrome_features(sp18: bool, sp18_features: str) -> str:
    return ALIASED_FEATURE + ("," + sp18_features if sp18 else "")


def select_ids(manifest_ids: list[str], text_ids: set[str], real_font_ids: set[str]) -> list[str]:
    return [test_id for test_id in manifest_ids if test_id in text_ids and test_id not in real_font_ids]


def linux_statuses(summary_path: Path | None, profile: str) -> dict[str, str]:
    if summary_path is None:
        return {}
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    entry = next(item for item in summary["profiles"] if item["profile"] == profile)
    return {test["id"]: test["status"] for test in entry["tests"]}


def cross_tabulate(results: list[dict[str, object]], linux: dict[str, str]) -> dict[str, int]:
    counts = Counter(
        f"linux-{linux.get(str(result['id']), 'unmeasured')}:windows-{result['status']}"
        for result in results
    )
    return dict(sorted(counts.items()))


class BrowserSession:
    """One browser at a time; a failed page restarts it and retries once.

    A browser that stops answering (a CDP timeout or a lost connection) is
    replaced rather than ending the run. A test that fails on a fresh browser
    too is recorded as an error, never as a comparison result.
    """

    FAILURES = (OSError, RuntimeError, TimeoutError)

    def __init__(self, factory):
        self.factory = factory
        self.context = None
        self.browser = None
        self.restarts = 0

    def _start(self):
        self.context = self.factory()
        self.browser = self.context.__enter__()

    def close(self) -> None:
        if self.context is not None:
            context, self.context, self.browser = self.context, None, None
            try:
                context.__exit__(None, None, None)
            except self.FAILURES:
                pass

    def run(self, test_id: str, action) -> dict[str, object]:
        for attempt in range(2):
            try:
                if self.browser is None:
                    self._start()
                return action(self.browser)
            except self.FAILURES as error:
                print(f"browser failure on {test_id} (attempt {attempt + 1}): {error!r}", flush=True)
                self.close()
                self.restarts += 1
        return {"id": test_id, "status": "error", "mismatched_pixels": -1}


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--pixel-compare", type=Path, required=True)
    parser.add_argument("--chrome", type=Path, default=windows_chrome.DEFAULT_CHROME)
    parser.add_argument("--profile", default="legacy-800x600@1")
    parser.add_argument("--linux-summary", type=Path,
                        help="expanded-summary.json of a Linux census for cross-tabulation")
    parser.add_argument("--test-id", action="append", default=[],
                        help="restrict to these selected IDs (repeatable)")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--keep-mismatches", type=Path,
                        help="directory for Chrome and Open UI PNGs of mismatched tests")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    chrome = windows_chrome.verify_chrome(args.chrome)
    contract = json.loads(matrix.CONTRACT.read_text(encoding="utf-8"))
    profiles = {profile.name: profile for profile in matrix.contract_profiles(contract, "expanded")}
    profile = profiles[args.profile]
    runner = matrix.pixel_runner
    runner.PIXEL_COMPARE = str(args.pixel_compare)

    templates = matrix.load_templates()
    text_ids = set(matrix.load_ids(Path(runner.TEXT_PORTED_LIST)))
    real_font_ids = set(matrix.load_ids(Path(runner.REAL_FONT_LIST)))
    sp18_ids = set(matrix.load_ids(Path(runner.SP18_TARGET_LIST)))
    ids = select_ids(matrix.load_ids(matrix.EXPANDED_IDS), text_ids, real_font_ids)
    if args.test_id:
        unknown = sorted(set(args.test_id) - set(ids))
        if unknown:
            raise SystemExit("not selected registered-Ahem tests: " + ", ".join(unknown))
        ids = [test_id for test_id in ids if test_id in set(args.test_id)]
    linux = linux_statuses(args.linux_summary, profile.name)
    faces_css = bundled_font_faces_css()

    results: list[dict[str, object]] = []
    restarts = 0
    with tempfile.TemporaryDirectory(prefix="openui-windows-text-") as temporary:
        work = Path(temporary)
        for sp18 in (False, True):
            group = [test_id for test_id in ids if (test_id in sp18_ids) == sp18]
            if not group:
                continue
            flags = (f"--enable-blink-features={chrome_features(sp18, runner.SP18_BLINK_FEATURES)}",)
            session = BrowserSession(lambda: windows_chrome.WindowsChrome(
                chrome, profile.logical_width, profile.logical_height, profile.scale, flags))
            try:
                for test_id in group:
                    results.append(session.run(
                        test_id,
                        lambda browser: compare_one(
                            test_id, templates, faces_css, profile, browser, work,
                            args.keep_mismatches)))
                    print(f"{results[-1]['status']:9} {results[-1]['mismatched_pixels']:7} {test_id}",
                          flush=True)
            finally:
                session.close()
                restarts += session.restarts

    results.sort(key=lambda result: str(result["id"]))
    status_counts = Counter(str(result["status"]) for result in results)
    report = {
        "schema_version": SCHEMA_VERSION,
        "kind": "windows_text_comparison",
        "profile": profile.name,
        "font_profile": "registered-ahem",
        "chrome_sha256": windows_chrome.sha256(chrome.read_bytes()),
        "pixel_compare_sha256": windows_chrome.sha256(args.pixel_compare.read_bytes()),
        "source_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
        ).stdout.strip(),
        "selection": "expanded manifest, text-port IDs minus real-font IDs"
        + (" (restricted by --test-id)" if args.test_id else ""),
        "selected": len(results),
        "status_counts": dict(sorted(status_counts.items())),
        "browser_restarts": restarts,
        "linux_cross_tabulation": cross_tabulate(results, linux) if linux else None,
        "tests": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ("selected", "status_counts", "linux_cross_tabulation")},
                     indent=1))
    return 0


def compare_one(test_id, templates, faces_css, profile, browser, work: Path,
                keep: Path | None) -> dict[str, object]:
    runner = matrix.pixel_runner
    template = re.sub(r"<!\[CDATA\[|\]\]>", "", templates[test_id])
    page = work / "test.html"
    page.write_text(windows_page(runner.build_html_document(template), faces_css), encoding="utf-8")
    preserve_subpixel = bool(re.search(
        r'<input\b[^>]*\btype\s*=\s*["\']?range\b', template, re.IGNORECASE))
    configuration = matrix.raster_configuration_name(True, False, preserve_subpixel)
    browser.load(page)
    chrome_png = work / "chromium.png"
    chrome_png.write_bytes(browser.screenshot())
    openui_png = work / "openui.png"
    openui_png.unlink(missing_ok=True)
    result: dict[str, object] = {"id": test_id, "raster_configuration": configuration}
    if not matrix.render_openui_with_configuration(test_id, openui_png, configuration, profile):
        return {**result, "status": "error", "mismatched_pixels": -1}
    mismatched, chrome_rgba, openui_rgba = matrix.compare_images(chrome_png, openui_png)
    result.update({
        "status": "exact" if mismatched == 0 else ("different" if mismatched > 0 else "size-mismatch"),
        "mismatched_pixels": mismatched,
        "chromium_rgba_sha256": chrome_rgba,
        "openui_rgba_sha256": openui_rgba,
    })
    if mismatched != 0 and keep is not None:
        target = keep / test_id
        target.mkdir(parents=True, exist_ok=True)
        (target / "chromium.png").write_bytes(chrome_png.read_bytes())
        (target / "openui.png").write_bytes(openui_png.read_bytes())
    return result


if __name__ == "__main__":
    raise SystemExit(main())
