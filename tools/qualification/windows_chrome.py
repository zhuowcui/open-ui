"""Drive pinned Windows Chrome-for-Testing for Windows reference measurements.

Windows Open UI text uses DirectWrite, so its references come from the
Windows build of the pinned Chromium release rather than the Linux oracle.
This module reuses the Linux harness's dependency-free CDP client and owns the
Windows process lifecycle: every child of the browser is ended before the
temporary profile is removed.
"""

from __future__ import annotations

import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import urllib.request


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "tools/accountability/run_all_pixel_comparisons.py"
CHROMIUM_VERSION = "147.0.7727.50"
# chrome.exe from storage.googleapis.com/chrome-for-testing-public/147.0.7727.50/win64/chrome-win64.zip
# (archive SHA-256 d014348fe444480c7490de4411fd3110420d659580a868d882bb929eca3522e9).
CHROME_SHA256 = "5a261cd9e0fa71f40fa5df4b9841da4f3311f9175d2e5868c27cb510ccd28505"
DEFAULT_CHROME = ROOT / "out/chrome-win/win-147.0.7727.50/chrome-win64/chrome.exe"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _harness():
    spec = importlib.util.spec_from_file_location("run_all_pixel_comparisons", HARNESS)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def verify_chrome(chrome: Path) -> Path:
    chrome = chrome.resolve()
    if sha256(chrome.read_bytes()) != CHROME_SHA256:
        raise SystemExit(f"{chrome} is not the pinned Windows Chrome-for-Testing build")
    if not (chrome.parent / f"{CHROMIUM_VERSION}.manifest").is_file():
        raise SystemExit(f"{chrome} lacks the {CHROMIUM_VERSION} manifest")
    return chrome


class WindowsChrome:
    """One headless browser with an emulated viewport and device scale."""

    def __init__(self, chrome: Path, width: int, height: int, scale: float,
                 extra_args: tuple[str, ...] = ()):
        self._harness = _harness()
        self.chrome = verify_chrome(chrome)
        self.width, self.height, self.scale = width, height, scale
        self.extra_args = tuple(extra_args)
        self.process = None
        self.client = None
        self.profile = tempfile.mkdtemp(prefix="openui-win-chrome-")

    def __enter__(self) -> "WindowsChrome":
        physical = (int(self.width * self.scale + 0.5), int(self.height * self.scale + 0.5))
        self.process = subprocess.Popen(
            [str(self.chrome), "--headless", "--disable-gpu", "--no-sandbox", "--no-first-run",
             "--no-default-browser-check", "--remote-debugging-port=0",
             f"--user-data-dir={self.profile}",
             f"--window-size={physical[0]},{physical[1] + 87}", *self.extra_args, "about:blank"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
        )
        h = self._harness
        port = h._wait_for_devtools_endpoint(self.profile, self.process, timeout=30)
        self.client = h._CdpWebSocket(h._page_websocket_url(port, timeout=30))
        self.client.command("Page.enable")
        self.client.command("Emulation.setDeviceMetricsOverride", {
            "width": self.width, "height": self.height, "deviceScaleFactor": self.scale,
            "mobile": False, "screenWidth": self.width, "screenHeight": self.height,
        })
        version = self.client.command("Browser.getVersion")
        if CHROMIUM_VERSION not in version.get("product", ""):
            raise SystemExit(f"unexpected browser version: {version}")
        return self

    def load(self, html: Path) -> None:
        url = "file:" + urllib.request.pathname2url(str(html.resolve()))
        self.client.command("Page.navigate", {"url": url})
        self.evaluate(
            "new Promise(resolve => { const done = () => document.fonts.ready.then(() => "
            "requestAnimationFrame(() => requestAnimationFrame(resolve)));"
            "if (document.readyState === 'complete') done();"
            "else addEventListener('load', done, {once: true}); })",
            await_promise=True,
        )
        metrics = self.evaluate("({width: innerWidth, height: innerHeight, dpr: devicePixelRatio})")
        if metrics != {"width": self.width, "height": self.height, "dpr": self.scale}:
            raise RuntimeError(f"viewport emulation failed: {metrics}")

    def evaluate(self, expression: str, await_promise: bool = False):
        result = self.client.command("Runtime.evaluate", {
            "expression": expression, "awaitPromise": await_promise, "returnByValue": True,
        })
        if "exceptionDetails" in result:
            raise RuntimeError(json.dumps(result["exceptionDetails"])[:500])
        return result.get("result", {}).get("value")

    def screenshot(self) -> bytes:
        """A stable PNG of the emulated viewport (two identical captures)."""
        previous = None
        for _ in range(8):
            data = base64.b64decode(self.client.command(
                "Page.captureScreenshot", {"format": "png", "fromSurface": True})["data"])
            if data == previous:
                return data
            previous = data
        raise RuntimeError("screenshots did not stabilize")

    def __exit__(self, *_exc) -> None:
        if self.client is not None:
            self._harness._close_chromium_client(self.client)
        if self.process is not None:
            subprocess.run(["taskkill", "/T", "/F", "/PID", str(self.process.pid)],
                           capture_output=True)
            self.process.wait(timeout=10)
        for _ in range(50):
            try:
                shutil.rmtree(self.profile)
                return
            except FileNotFoundError:
                return
            except PermissionError:
                time.sleep(0.1)
        raise RuntimeError(f"browser profile writers did not exit: {self.profile}")
