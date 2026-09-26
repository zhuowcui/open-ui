#!/usr/bin/env python3
"""Re-render the frozen SP20 manifest and byte-compare archived OpenUI PNGs."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import re
import subprocess
import tempfile
from pathlib import Path

import run_all_pixel_comparisons as runner


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "tools/wpt/sp20_focused_ids.json"
RESULTS = ROOT / "tools/accountability/data/pixel_comparison/results"
WPT_TEMPLATES = ROOT / "tools/accountability/data/wpt_ported/all_wpt_templates.json"


def load_ids(path: Path) -> list[str]:
    values = json.loads(path.read_text(encoding="utf-8"))
    if (
        not isinstance(values, list)
        or any(not isinstance(value, str) or not value for value in values)
        or len(values) != len(set(values))
    ):
        raise ValueError(f"invalid exact-ID manifest: {path}")
    return values


def load_set(path: str) -> set[str]:
    return set(load_ids(Path(path)))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--pixel-compare", type=Path, default=Path(runner.PIXEL_COMPARE))
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    if args.jobs < 1:
        raise SystemExit("--jobs must be positive")
    if not args.pixel_compare.is_file():
        raise SystemExit(f"missing release pixel renderer: {args.pixel_compare}")

    ids = load_ids(args.manifest)
    text_ids = load_set(runner.TEXT_PORTED_LIST)
    real_font_ids = load_set(runner.REAL_FONT_LIST)
    templates = dict(runner.HTML_TEMPLATES)
    templates.update(json.loads(WPT_TEMPLATES.read_text(encoding="utf-8")))

    registered = set(
        subprocess.run(
            [args.pixel_compare, "list"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
    )
    unknown = sorted(set(ids) - registered)
    if unknown:
        raise SystemExit(f"manifest contains {len(unknown)} unregistered IDs")

    def verify(item: tuple[int, str], temporary: Path) -> tuple[int, str, str | None]:
        index, test_id = item
        expected = RESULTS / test_id / "openui.png"
        if not expected.is_file():
            return index, test_id, "missing restored frozen openui.png"
        template = templates.get(test_id, "")
        preserve_subpixel = bool(
            re.search(
                r'<input\b[^>]*\btype\s*=\s*["\']?range\b',
                template,
                re.IGNORECASE,
            )
        )
        actual = temporary / f"{index}.png"
        use_real_font = test_id in real_font_ids
        use_ahem_noaa = test_id in text_ids and not use_real_font
        raster_profile = "default"
        if use_real_font:
            raster_profile = "legacy-chromium-linux-lcd"
        elif use_ahem_noaa:
            raster_profile = (
                "legacy-deterministic-alias-subpixel"
                if preserve_subpixel
                else "legacy-deterministic-alias"
            )
        environment = runner.openui_environment(use_real_font=use_real_font)
        completed = subprocess.run(
            [
                args.pixel_compare,
                "render",
                test_id,
                actual,
                "--raster-config",
                raster_profile,
            ],
            env=environment,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            timeout=120,
        )
        if completed.returncode != 0:
            detail = completed.stderr.decode("utf-8", errors="replace").strip()
            return index, test_id, f"render failed: {detail}"
        if actual.read_bytes() != expected.read_bytes():
            return index, test_id, "PNG bytes differ from frozen result"
        return index, test_id, None

    failures: list[tuple[int, str, str]] = []
    completed_count = 0
    with tempfile.TemporaryDirectory(prefix="openui-sp20-replay-") as directory:
        temporary = Path(directory)
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futures = [executor.submit(verify, item, temporary) for item in enumerate(ids)]
            for future in concurrent.futures.as_completed(futures):
                index, test_id, error = future.result()
                completed_count += 1
                if error is not None:
                    failures.append((index, test_id, error))
                if completed_count % 500 == 0:
                    print(f"replayed {completed_count}/{len(ids)} frozen pixels")

    if failures:
        failures.sort()
        for _, test_id, error in failures[:50]:
            print(f"FAIL {test_id}: {error}")
        raise SystemExit(f"{len(failures)}/{len(ids)} frozen pixel replays changed")
    print(f"frozen pixel replay: {len(ids)}/{len(ids)} byte-identical")


if __name__ == "__main__":
    main()
