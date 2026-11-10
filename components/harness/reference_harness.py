#!/usr/bin/env python3
"""Compare Open UI components with shadcn/ui rendered by pinned Chromium.

The shadcn side is the static pages in components/reference/pages, built
from pinned sources by components/reference/build.mjs. The Open UI side is
the `openui-components-reference` binary, which renders the same fixture
IDs with the public `openui` crate. Both are rasterized at the census
profiles and compared pixel for pixel.

    reference_harness.py tokens --output components/evidence/theme-tokens-v1.json
    reference_harness.py fonts --output components/evidence/fonts-v1.json
    reference_harness.py compare --renderer PATH --evidence components/evidence/reference-v1.json

Run it on Linux (WSL here) with the pinned Chrome-for-Testing 147. Chromium
uses the census real-font environment (`sp16_real_font.conf` and the
FreeType font path), and Open UI the matching `chromium-linux-lcd` raster
configuration with the pinned FreeType library.
"""

from __future__ import annotations

import argparse
import base64
import concurrent.futures
import datetime
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[2]
REFERENCE = ROOT / "components/reference"
PAGES = REFERENCE / "pages"
MANIFEST = PAGES / "manifest.json"
CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
DEFAULT_CHROME = Path.home() / (
    "openui-oracle/chrome/linux-147.0.7727.50/chrome-linux64/chrome"
)
EXPECTED_CHROMIUM = "147.0.7727.50"
RASTER_CONFIGURATION = "chromium-linux-lcd"
FONT_PROFILE = "registered-real-font"
RESULTS_SCHEMA = "openui-components-reference-results-v1"
TOKENS_SCHEMA = "openui-components-theme-tokens-v1"
FONTS_SCHEMA = "openui-components-fonts-v1"
ASSETS_MANIFEST = ROOT / "components/assets/manifest.json"

sys.path.insert(0, str(ROOT / "tools/accountability"))
import run_all_pixel_comparisons as pixel_runner  # noqa: E402


@dataclass(frozen=True)
class Profile:
    name: str
    logical_width: int
    logical_height: int
    device_scale: float

    @property
    def physical_size(self) -> tuple[int, int]:
        return (
            int(self.logical_width * self.device_scale + 0.5),
            int(self.logical_height * self.device_scale + 0.5),
        )


def census_profiles() -> list[Profile]:
    """The four required census profiles, from the renderer contract."""
    contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
    profiles = []
    for entry in contract["qualification_profiles"]:
        logical = entry["logical_size_css_px"]
        profiles.append(
            Profile(
                name=entry["name"],
                logical_width=int(logical["width"]),
                logical_height=int(logical["height"]),
                device_scale=float(entry["device_scale"]),
            )
        )
    return profiles


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_manifest() -> dict:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    for name, digest in manifest["files"].items():
        if sha256_file(PAGES / name) != digest:
            raise ValueError(f"{name} does not match the reference manifest; rebuild the pages")
    return manifest


def repository_identity(commit: str | None = None) -> dict[str, object]:
    """The measured commit and any uncommitted paths.

    A Windows git worktree cannot be read by git inside WSL; there the caller
    passes the commit it checked out (`--repository-commit`), and the dirty
    state is recorded as unknown.
    """
    def git(*args: str) -> str:
        return subprocess.run(
            ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
        ).stdout.strip()

    try:
        head = git("rev-parse", "HEAD")
        dirty: object = sorted(
            line[3:] for line in git("status", "--porcelain").splitlines() if line
        )
    except (OSError, subprocess.CalledProcessError):
        if commit is None:
            raise SystemExit("git cannot read this checkout; pass --repository-commit")
        return {"commit": commit, "dirty_paths": "unknown: git unavailable to the harness"}
    if commit is not None and commit != head:
        raise SystemExit(f"--repository-commit {commit} is not HEAD {head}")
    return {"commit": head, "dirty_paths": dirty}


def chromium_version(chrome: Path) -> str:
    env = pixel_runner.chrome_environment(str(chrome.parent))
    output = subprocess.run(
        [str(chrome), "--version"], env=env, check=True, capture_output=True, text=True
    ).stdout
    match = re.search(r"\b(\d+\.\d+\.\d+\.\d+)\b", output)
    if not match:
        raise ValueError(f"cannot parse Chromium version from {output!r}")
    return match.group(1)


# ── Theme tokens ────────────────────────────────────────────────────────

# Chromium converts each token to sRGB through relative color syntax, so the
# stand-in values come from Chromium's own oklch conversion.
TOKENS_SCRIPT = r"""
new Promise(resolve => {
  const read = () => [...document.querySelectorAll('[data-token]')].map(el => {
    const name = el.dataset.token;
    const probe = document.createElement('span');
    probe.style.color = `color(from var(--${name}) srgb r g b / alpha)`;
    el.appendChild(probe);
    const style = getComputedStyle(el);
    return {
      theme: el.dataset.theme,
      token: name,
      specified: style.getPropertyValue(`--${name}`).trim(),
      srgb: getComputedStyle(probe).color,
    };
  });
  if (document.readyState === 'complete') resolve(read());
  else addEventListener('load', () => resolve(read()), {once: true});
})
"""

SRGB = re.compile(
    r"^color\(srgb (?P<r>[-0-9.e]+) (?P<g>[-0-9.e]+) (?P<b>[-0-9.e]+)"
    r"(?: / (?P<a>[-0-9.e]+))?\)$"
)


def parse_srgb(text: str) -> tuple[float, float, float, float]:
    match = SRGB.match(text)
    if not match:
        raise ValueError(f"unexpected sRGB serialization {text!r}")
    alpha = match.group("a")
    return (
        float(match.group("r")),
        float(match.group("g")),
        float(match.group("b")),
        1.0 if alpha is None else float(alpha),
    )


def to_u8(channel: float) -> int:
    return max(0, min(255, int(channel * 255.0 + 0.5)))


def token_entry(row: dict[str, str]) -> dict[str, object]:
    r, g, b, a = parse_srgb(row["srgb"])
    return {
        "theme": row["theme"],
        "token": row["token"],
        "specified": row["specified"],
        "chromium_srgb": row["srgb"],
        # Chromium paints with these unclamped floats: a translucent token
        # blends from them, so rounding them to 8 bits first changes pixels.
        "stand_in_rgba": [r, g, b, a],
        "rgb8": [to_u8(r), to_u8(g), to_u8(b)],
        "in_gamut": all(0.0 <= value <= 1.0 for value in (r, g, b)),
    }


@contextmanager
def chromium_page(chrome: Path):
    """A headless pinned Chromium with one page, driven over CDP."""
    env = pixel_runner.chrome_environment(str(chrome.parent), use_real_font=True)
    with tempfile.TemporaryDirectory(prefix="openui-components-") as profile_dir, ExitStack() as stack:
        process = subprocess.Popen(
            [
                str(chrome),
                "--headless",
                "--disable-gpu",
                "--no-sandbox",
                "--no-first-run",
                "--no-default-browser-check",
                "--enable-features=FontDataServiceLinux:typeface/Freetype",
                "--remote-debugging-port=0",
                f"--user-data-dir={profile_dir}",
                "about:blank",
            ],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        stack.callback(pixel_runner._stop_chromium_process, process)
        port = pixel_runner._wait_for_devtools_endpoint(profile_dir, process)
        client = pixel_runner._CdpWebSocket(pixel_runner._page_websocket_url(port))
        stack.callback(pixel_runner._close_chromium_client, client)
        client.command("Page.enable")
        yield client


def extract_tokens(chrome: Path) -> list[dict[str, object]]:
    page = PAGES / "theme-tokens.html"
    with chromium_page(chrome) as client:
        client.command(
            "Page.navigate", {"url": "file:" + urllib.request.pathname2url(str(page))}
        )
        result = client.command(
            "Runtime.evaluate",
            {"expression": TOKENS_SCRIPT, "awaitPromise": True, "returnByValue": True},
        )
    rows = result.get("result", {}).get("value")
    if not isinstance(rows, list) or not rows:
        raise RuntimeError(f"token extraction failed: {result!r}")
    return [token_entry(row) for row in rows]


def command_tokens(args: argparse.Namespace) -> int:
    manifest = load_manifest()
    version = chromium_version(args.chrome)
    if version != EXPECTED_CHROMIUM:
        raise SystemExit(f"Chromium {version} is not the pinned {EXPECTED_CHROMIUM}")
    tokens = extract_tokens(args.chrome)
    expected = {(theme, name) for theme in manifest["themes"] for name in manifest["tokens"]}
    if {(t["theme"], t["token"]) for t in tokens} != expected:
        raise SystemExit("token page does not list every theme token")
    evidence = {
        "schema": TOKENS_SCHEMA,
        "chromium": {"version": version, "binary_sha256": sha256_file(args.chrome)},
        "reference_manifest_sha256": sha256_file(MANIFEST),
        "method": (
            "getComputedStyle color of `color(from var(--token) srgb r g b / alpha)` "
            "on pages/theme-tokens.html; stand_in_rgba keeps the serialized "
            "floats unclamped, and rgb8 is round(clamp(channel) * 255) for reference"
        ),
        "tokens": tokens,
    }
    write_json(args.output, evidence)
    print(f"wrote {len(tokens)} tokens to {args.output}")
    return 0


# ── Fonts ───────────────────────────────────────────────────────────────

# Each font fixture and the bundled family its lines must be drawn with.
FONT_FIXTURES = {
    "font-geist": "Geist",
    "font-geist-mono": "Geist Mono",
    "font-inter": "Inter",
}

FONTS_READY_SCRIPT = r"""
new Promise(resolve => {
  const done = () => document.fonts.ready.then(() => resolve(
    [...document.fonts].map(face => ({family: face.family, status: face.status}))));
  if (document.readyState === 'complete') done();
  else addEventListener('load', done, {once: true});
})
"""


def platform_fonts(client, page: Path) -> dict[str, object]:
    """The faces Chromium used for each text line of a font fixture page."""
    client.command("Page.navigate", {"url": "file:" + urllib.request.pathname2url(str(page))})
    faces = client.command(
        "Runtime.evaluate",
        {"expression": FONTS_READY_SCRIPT, "awaitPromise": True, "returnByValue": True},
    ).get("result", {}).get("value")
    client.command("DOM.enable")
    client.command("CSS.enable")
    root = client.command("DOM.getDocument", {"depth": -1})["root"]["nodeId"]
    nodes = client.command(
        "DOM.querySelectorAll", {"nodeId": root, "selector": "main p"}
    )["nodeIds"]
    lines = []
    for node in nodes:
        used = client.command("CSS.getPlatformFontsForNode", {"nodeId": node})["fonts"]
        text = client.command("DOM.getOuterHTML", {"nodeId": node})["outerHTML"]
        lines.append({"element": text, "fonts": used})
    return {"document_fonts": faces, "lines": lines}


def font_problems(family: str, record: dict[str, object]) -> list[str]:
    """Why a page's text was not drawn entirely with `family`'s bundled face."""
    problems = []
    loaded = [f for f in record["document_fonts"] if f["family"] == family]
    if not loaded or any(f["status"] != "loaded" for f in loaded):
        problems.append(f"{family} is not loaded: {record['document_fonts']}")
    if not record["lines"]:
        problems.append("no text lines")
    for line in record["lines"]:
        used = line["fonts"]
        if len(used) != 1 or not used[0]["isCustomFont"]:
            problems.append(f"{line['element']} uses {used}")
    return problems


def command_fonts(args: argparse.Namespace) -> int:
    manifest = load_manifest()
    version = chromium_version(args.chrome)
    if version != EXPECTED_CHROMIUM:
        raise SystemExit(f"Chromium {version} is not the pinned {EXPECTED_CHROMIUM}")
    pages = {f["id"]: f for f in manifest["fixtures"] if f["component"] == "font"}
    expected_pages = {f"{base}-{theme}" for base in FONT_FIXTURES for theme in manifest["themes"]}
    if set(pages) != expected_pages:
        raise SystemExit(f"font fixtures {sorted(pages)} are not {sorted(expected_pages)}")
    records = []
    problems = []
    with chromium_page(args.chrome) as client:
        for page_id in sorted(pages):
            family = FONT_FIXTURES[page_id.rsplit("-", 1)[0]]
            record = platform_fonts(client, PAGES / pages[page_id]["page"])
            records.append({"fixture": page_id, "family": family, **record})
            problems += [f"{page_id}: {p}" for p in font_problems(family, record)]
    evidence = {
        "schema": FONTS_SCHEMA,
        "chromium": {"version": version, "binary_sha256": sha256_file(args.chrome)},
        "reference_manifest_sha256": sha256_file(MANIFEST),
        "assets_manifest_sha256": sha256_file(ASSETS_MANIFEST),
        "method": (
            "CDP CSS.getPlatformFontsForNode for each `main p` of the font fixture "
            "pages after document.fonts.ready; every line must use exactly one "
            "web font (isCustomFont) and the family's FontFace must be loaded"
        ),
        "problems": problems,
        "pages": records,
    }
    write_json(args.output, evidence)
    print(f"wrote {len(records)} pages to {args.output}; {len(problems)} problems")
    for problem in problems:
        print(f"  {problem}")
    return 1 if problems else 0


# ── Pixel comparison ────────────────────────────────────────────────────


def compare_images(chromium: Path, openui: Path) -> dict[str, object]:
    with Image.open(chromium) as expected_image, Image.open(openui) as actual_image:
        expected = expected_image.convert("RGBA")
        actual = actual_image.convert("RGBA")
    result: dict[str, object] = {
        "chromium_rgba_sha256": sha256_bytes(expected.tobytes()),
        "openui_rgba_sha256": sha256_bytes(actual.tobytes()),
    }
    if expected.size != actual.size:
        result.update(status="error", error="surface size mismatch",
                      chromium_size=list(expected.size), openui_size=list(actual.size))
        return result
    difference = ImageChops.difference(expected, actual)
    channels = difference.split()
    mask = channels[0]
    for channel in channels[1:]:
        mask = ImageChops.lighter(mask, channel)
    mismatched = sum(mask.point(lambda value: 255 if value else 0).histogram()[255:])
    result["mismatched_pixels"] = mismatched
    result["status"] = "exact" if mismatched == 0 else "different"
    if mismatched:
        result["diff_bbox"] = list(mask.getbbox())
        result["max_channel_delta"] = max(channel.getextrema()[1] for channel in channels)
    return result


def write_diff(chromium: Path, openui: Path, output: Path) -> None:
    with Image.open(chromium) as expected, Image.open(openui) as actual:
        difference = ImageChops.difference(expected.convert("RGB"), actual.convert("RGB"))
    mask = difference.convert("L").point(lambda value: 255 if value else 0)
    Image.composite(Image.new("RGB", mask.size, (255, 0, 0)),
                    Image.new("RGB", mask.size, (255, 255, 255)), mask).save(output)


def render_openui(renderer: Path, fixture: str, profile: Profile, output: Path) -> str | None:
    completed = subprocess.run(
        [
            str(renderer), "render", fixture, str(output),
            "--viewport", f"{profile.logical_width}x{profile.logical_height}",
            "--scale", f"{profile.device_scale:g}",
        ],
        env=pixel_runner.openui_environment(use_real_font=True),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if completed.returncode != 0:
        return completed.stderr.strip() or f"exit {completed.returncode}"
    return None


def render_chromium(chrome: Path, page: Path, profile: Profile, output: Path) -> bool:
    return any(
        pixel_runner.render_chrome(
            str(page), str(output), str(chrome), str(chrome.parent),
            use_real_font=True,
            logical_width=profile.logical_width,
            logical_height=profile.logical_height,
            device_scale=profile.device_scale,
        )
        for _attempt in range(3)
    )


def compare_one(
    fixture: dict, profile: Profile, renderer: Path, chrome: Path, results_dir: Path
) -> dict[str, object]:
    directory = results_dir / profile.name / fixture["id"]
    directory.mkdir(parents=True, exist_ok=True)
    chromium_png = directory / "chromium.png"
    openui_png = directory / "openui.png"
    row: dict[str, object] = {
        "fixture": fixture["id"],
        "component": fixture["component"],
        "theme": fixture["theme"],
        "profile": profile.name,
    }
    error = render_openui(renderer, fixture["id"], profile, openui_png)
    if error is not None:
        return {**row, "status": "error", "error": f"Open UI render failed: {error}"}
    if not render_chromium(chrome, PAGES / fixture["page"], profile, chromium_png):
        return {**row, "status": "error", "error": "Chromium render failed"}
    for path in (chromium_png, openui_png):
        with Image.open(path) as image:
            if image.size != profile.physical_size:
                return {**row, "status": "error",
                        "error": f"{path.name} is {image.size}, expected {profile.physical_size}"}
    comparison = compare_images(chromium_png, openui_png)
    if comparison.get("status") == "different":
        write_diff(chromium_png, openui_png, directory / "diff.png")
    return {**row, **comparison}


def summarize(results: list[dict[str, object]]) -> dict[str, object]:
    def tally(key: str) -> dict[str, dict[str, int]]:
        table: dict[str, dict[str, int]] = {}
        for row in results:
            entry = table.setdefault(str(row[key]), {"exact": 0, "different": 0, "error": 0})
            entry[str(row["status"])] += 1
        return dict(sorted(table.items()))

    exact = sum(1 for row in results if row["status"] == "exact")
    return {
        "comparisons": len(results),
        "exact": exact,
        "different": sum(1 for row in results if row["status"] == "different"),
        "error": sum(1 for row in results if row["status"] == "error"),
        "by_component": tally("component"),
        "by_profile": tally("profile"),
        "by_theme": tally("theme"),
    }


def renderer_fixtures(renderer: Path) -> list[str]:
    completed = subprocess.run(
        [str(renderer), "list"], check=True, capture_output=True, text=True,
        env=pixel_runner.openui_environment(use_real_font=True),
    )
    return completed.stdout.split()


def command_compare(args: argparse.Namespace) -> int:
    manifest = load_manifest()
    version = chromium_version(args.chrome)
    if version != EXPECTED_CHROMIUM:
        raise SystemExit(f"Chromium {version} is not the pinned {EXPECTED_CHROMIUM}")
    fixtures = manifest["fixtures"]
    reference_ids = [fixture["id"] for fixture in fixtures]
    implemented = renderer_fixtures(args.renderer)
    unknown = sorted(set(implemented) - set(reference_ids))
    if unknown:
        raise SystemExit(f"renderer fixtures missing from the reference: {unknown}")
    missing = sorted(set(reference_ids) - set(implemented))
    selected = [f for f in fixtures if f["id"] in implemented]
    if args.fixture:
        wanted = set(args.fixture)
        selected = [f for f in selected if f["id"] in wanted]
    profiles = census_profiles()
    if args.profile:
        by_name = {profile.name: profile for profile in profiles}
        profiles = [by_name[name] for name in args.profile]
    complete = not args.fixture and not args.profile and not missing
    repository = repository_identity(args.repository_commit)

    tasks = [(fixture, profile) for profile in profiles for fixture in selected]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(
            lambda task: compare_one(task[0], task[1], args.renderer, args.chrome,
                                     args.results_dir),
            tasks,
        ))
    results.sort(key=lambda row: (row["fixture"], row["profile"]))

    evidence = {
        "schema": RESULTS_SCHEMA,
        "measured_on": datetime.date.today().isoformat(),
        "complete": complete,
        "repository": repository,
        "chromium": {
            "version": version,
            "binary_sha256": sha256_file(args.chrome),
            "font_profile": FONT_PROFILE,
        },
        "reference": {
            "manifest_sha256": sha256_file(MANIFEST),
            "package_lock_sha256": manifest["package_lock_sha256"],
        },
        "openui": {
            "renderer_sha256": sha256_file(args.renderer),
            "raster_configuration": RASTER_CONFIGURATION,
        },
        "profiles": [
            {"name": p.name, "logical_size": [p.logical_width, p.logical_height],
             "device_scale": p.device_scale, "physical_size": list(p.physical_size)}
            for p in profiles
        ],
        "unimplemented_fixtures": missing,
        "summary": summarize(results),
        "results": results,
    }
    if args.evidence:
        write_json(args.evidence, evidence)
    summary = evidence["summary"]
    print(f"{summary['exact']}/{summary['comparisons']} exact, "
          f"{summary['different']} different, {summary['error']} errors")
    for row in results:
        if row["status"] != "exact":
            detail = row.get("mismatched_pixels", row.get("error"))
            print(f"  {row['status']}: {row['fixture']} {row['profile']} ({detail})")
    return 0


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    tokens = sub.add_parser("tokens", help="record Chromium's sRGB values of the theme tokens")
    tokens.add_argument("--chrome", type=Path, default=DEFAULT_CHROME)
    tokens.add_argument("--output", type=Path, required=True)
    tokens.set_defaults(func=command_tokens)

    fonts = sub.add_parser("fonts", help="record the faces Chromium uses for the font fixtures")
    fonts.add_argument("--chrome", type=Path, default=DEFAULT_CHROME)
    fonts.add_argument("--output", type=Path, required=True)
    fonts.set_defaults(func=command_fonts)

    compare = sub.add_parser("compare", help="compare Open UI fixtures with the reference pages")
    compare.add_argument("--chrome", type=Path, default=DEFAULT_CHROME)
    compare.add_argument("--renderer", type=Path, required=True)
    compare.add_argument("--results-dir", type=Path,
                         default=ROOT / "out/components-reference")
    compare.add_argument("--evidence", type=Path)
    compare.add_argument("--fixture", action="append", default=[])
    compare.add_argument("--profile", action="append", default=[])
    compare.add_argument("--jobs", type=int, default=4)
    compare.add_argument("--repository-commit",
                         help="the checked-out commit, when git cannot read the checkout")
    compare.set_defaults(func=command_compare)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
