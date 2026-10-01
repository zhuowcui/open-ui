#!/usr/bin/env python3
"""Measure committed native geometry inputs in separate pinned Chromium processes."""

from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "tools/accountability/run_all_pixel_comparisons.py"
INPUTS = ROOT / "docs/v02/evidence/native-geometry-v1"
CONSTRAINED_INPUTS = ROOT / "docs/v02/evidence/native-constrained-box-v1"
COLUMN_FLEX_INPUTS = ROOT / "docs/v02/evidence/native-column-flex-v1"
COLUMN_PAINT_INPUTS = ROOT / "docs/v02/evidence/native-column-paint-phases-v1"
EXPRESSION = (
    "new Promise(resolve=>{"
    "const done=()=>document.fonts.ready.then(()=>"
    "requestAnimationFrame(()=>requestAnimationFrame(resolve)));"
    "if(document.readyState==='complete')done();"
    "else addEventListener('load',done,{once:true});"
    "}).then(()=>{const e=document.querySelector('.border');"
    "const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});"
    "return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},"
    "geometry:{bounds:rect(e.getBoundingClientRect()),"
    "rects:Array.from(e.getClientRects(),rect)}};})"
)
CONSTRAINED_EXPRESSION = EXPRESSION[:EXPRESSION.index("}).then")] + (
    "}).then(()=>{const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});"
    "const nodes={};for(const name of ['columns','limit','border']){"
    "const e=document.querySelector('.'+name);nodes[name]={"
    "bounds:rect(e.getBoundingClientRect()),rects:Array.from(e.getClientRects(),rect)};}"
    "return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},"
    "geometry:{nodes}};})"
)
COLUMN_FLEX_EXPRESSION = CONSTRAINED_EXPRESSION.replace(
    "['columns','limit','border']", "['columns','limit','border','clipped','following']",
).replace(
    "const e=document.querySelector('.'+name);nodes[name]",
    "const e=document.querySelector('.'+name);if(!e)continue;nodes[name]",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results-dir", type=Path, required=True)
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--suite", choices=("native", "constrained", "column-flex", "column-paint"), default="native")
    parser.add_argument("--scale", type=float, default=1.0)
    args = parser.parse_args()
    if not math.isfinite(args.scale) or args.scale <= 0.0:
        parser.error("scale must be positive and finite")
    column_flex = args.suite in ("column-flex", "column-paint")
    constrained = args.suite in ("constrained", "column-flex", "column-paint")
    inputs = (COLUMN_PAINT_INPUTS if args.suite == "column-paint" else COLUMN_FLEX_INPUTS) if column_flex else CONSTRAINED_INPUTS if constrained else INPUTS
    height = 340 if constrained else 240
    expression = COLUMN_FLEX_EXPRESSION if column_flex else CONSTRAINED_EXPRESSION if constrained else EXPRESSION
    if args.suite == "column-paint":
        expression = expression.replace(
            "geometry:{nodes}",
            "geometry:{nodes,hit_targets:Object.fromEntries([[217,30],[227,30]].map(([x,y])=>"
            "[`${x},${y}`,document.elementFromPoint(x,y)?.className||'']))}",
        )
    spec = importlib.util.spec_from_file_location("capture", HARNESS)
    assert spec is not None and spec.loader is not None
    capture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(capture)
    if args.chrome is None:
        chrome, chrome_dir = capture.find_chrome()
    else:
        chrome = str(args.chrome.resolve(strict=True))
        chrome_dir = str(args.chrome.resolve().parent)
    if chrome is None:
        raise RuntimeError("pinned Chromium is unavailable")
    environment = capture.chrome_environment(chrome_dir, True, False)
    version = subprocess.check_output([chrome, "--version"], env=environment, text=True).strip()
    if version.split()[-1] != "147.0.7727.50":
        raise RuntimeError(f"unexpected Chromium build: {version}")
    cases = sorted(inputs.glob("*/test.html"))
    expected_count = 11 if column_flex else 13 if constrained else 5
    if len(cases) != expected_count:
        raise RuntimeError(f"{expected_count} committed {args.suite} geometry inputs required")
    output = args.results_dir.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.mkdir()  # Preserve every prior input, record and Chromium profile.
    records = []
    for source in cases:
        directory = output / source.parent.name
        directory.mkdir()
        path = directory / "test.html"
        path.write_bytes(source.read_bytes())
        profile = directory / "chromium-profile"
        profile.mkdir()
        command = [
            chrome, "--headless", "--disable-gpu", "--no-sandbox", "--no-first-run",
            "--no-default-browser-check", "--remote-debugging-port=0",
            f"--user-data-dir={profile}", f"--window-size=320,{height + 87}", "about:blank",
        ]
        process = subprocess.Popen(
            command, env=environment, start_new_session=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        client = None
        try:
            port = capture._wait_for_devtools_endpoint(profile, process)
            client = capture._CdpWebSocket(capture._page_websocket_url(port))
            client.command("Page.enable")
            client.command("Emulation.setDeviceMetricsOverride", {
                "width": 320, "height": height, "deviceScaleFactor": args.scale, "mobile": False,
            })
            client.command("Page.navigate", {
                "url": "file:" + urllib.request.pathname2url(str(path)),
            })
            observations = []
            for _ in range(2):
                result = client.command("Runtime.evaluate", {
                    "expression": expression, "awaitPromise": True, "returnByValue": True,
                })
                if "exceptionDetails" in result:
                    raise RuntimeError(result["exceptionDetails"])
                value = result["result"]["value"]
                if not capture._device_metrics_match(value["metrics"], 320, height, args.scale):
                    raise RuntimeError(f"wrong viewport metrics: {value['metrics']}")
                observations.append(value["geometry"])
            if observations[0] != observations[1]:
                raise RuntimeError(f"unstable geometry: {source.parent.name}")
            screenshot = client.command("Page.captureScreenshot", {
                "format": "png", "fromSurface": True, "captureBeyondViewport": False,
            })
            (directory / "chromium.png").write_bytes(base64.b64decode(screenshot["data"]))
            record = {
                "case": source.parent.name, "input_sha256": sha256(path),
                "png_sha256": sha256(directory / "chromium.png"),
                "geometry": observations[0], "repeated_query_equal": True,
            }
            (directory / "geometry.json").write_text(json.dumps(record, indent=2) + "\n")
            records.append(record)
            print(source.parent.name, observations[0], flush=True)
        finally:
            if client is not None:
                try:
                    client.close()
                except OSError:
                    pass
            # This process group contains only this owned oracle instance.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=10)
    payload = {
        "schema_version": 1, "release_qualification": False,
        "chromium_binary_sha256": sha256(Path(chrome)),
        "capture_harness_sha256": sha256(HARNESS), "chromium_build": version,
        "viewport": [320, height, args.scale], "cases": records,
        "suite": args.suite, "probe_sha256": sha256(Path(__file__)),
    }
    (output / "summary.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
