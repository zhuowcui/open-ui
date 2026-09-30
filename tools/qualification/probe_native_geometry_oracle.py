#!/usr/bin/env python3
"""Read the five committed geometry inputs from separate pinned Chromium processes."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "tools/accountability/run_all_pixel_comparisons.py"
INPUTS = ROOT / "docs/v02/evidence/native-geometry-v1"
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


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results-dir", type=Path, required=True)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("capture", HARNESS)
    assert spec is not None and spec.loader is not None
    capture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(capture)
    chrome, chrome_dir = capture.find_chrome()
    if chrome is None:
        raise RuntimeError("pinned Chromium is unavailable")
    environment = capture.chrome_environment(chrome_dir, True, False)
    version = subprocess.check_output([chrome, "--version"], env=environment, text=True).strip()
    if version.split()[-1] != "147.0.7727.50":
        raise RuntimeError(f"unexpected Chromium build: {version}")
    cases = sorted(INPUTS.glob("*/test.html"))
    if len(cases) != 5:
        raise RuntimeError("five committed native geometry inputs required")
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
            f"--user-data-dir={profile}", "--window-size=320,327", "about:blank",
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
                "width": 320, "height": 240, "deviceScaleFactor": 1.0, "mobile": False,
            })
            client.command("Page.navigate", {
                "url": "file:" + urllib.request.pathname2url(str(path)),
            })
            observations = []
            for _ in range(2):
                result = client.command("Runtime.evaluate", {
                    "expression": EXPRESSION, "awaitPromise": True, "returnByValue": True,
                })
                if "exceptionDetails" in result:
                    raise RuntimeError(result["exceptionDetails"])
                value = result["result"]["value"]
                if not capture._device_metrics_match(value["metrics"], 320, 240, 1.0):
                    raise RuntimeError(f"wrong viewport metrics: {value['metrics']}")
                observations.append(value["geometry"])
            if observations[0] != observations[1]:
                raise RuntimeError(f"unstable geometry: {source.parent.name}")
            record = {
                "case": source.parent.name, "input_sha256": sha256(path),
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
        "viewport": [320, 240, 1.0], "cases": records,
    }
    (output / "summary.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
