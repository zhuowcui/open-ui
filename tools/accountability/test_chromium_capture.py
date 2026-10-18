"""Keep Chromium profile cleanup behind browser and child-process shutdown."""

import base64
import io
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

from tools.accountability import run_all_pixel_comparisons as capture


class CaptureLifecycleTests(unittest.TestCase):
    def exercise(self, case="success"):
        events = []
        children_running = [True]
        png = io.BytesIO()
        Image.new("RGBA", (1, 1), (0, 0, 0, 255)).save(png, format="PNG")
        encoded = base64.b64encode(png.getvalue()).decode()

        class Process:
            running = True
            pid = 123456789

            def terminate(self):
                events.append("terminate")
                if case != "hung":
                    self.running = False

            def wait(self, timeout=None):
                events.append("wait")
                if self.running:
                    raise subprocess.TimeoutExpired("chrome", timeout)
                return 0

            def kill(self):
                events.append("kill")
                self.running = False

        process = Process()

        def signal_group(group, requested_signal):
            self.assertEqual(group, process.pid)
            if requested_signal == signal.SIGTERM:
                process.terminate()
                if case != "hung-child":
                    children_running[0] = False
            elif requested_signal == signal.SIGKILL:
                process.kill()
                children_running[0] = False
            else:
                self.fail(f"Unexpected shutdown signal: {requested_signal}")

        class Profile:
            def __enter__(self):
                return "/tmp/mock-profile"

            def __exit__(self, *args):
                events.append("profile-cleanup")
                if process.running or children_running[0]:
                    raise OSError(39, "Directory not empty: 'Default'")

        class Client:
            def command(self, name, params=None):
                if name == "Page.enable" and case == "aborted":
                    raise RuntimeError("CDP failed")
                if name == "Runtime.evaluate":
                    return {
                        "result": {
                            "value": {
                                "width": 2 if case == "bad-metrics" else 1,
                                "height": 1,
                                "dpr": 1,
                            }
                        }
                    }
                if name == "Page.captureScreenshot":
                    return {"data": encoded}
                return {}

            def close(self):
                events.append("client-close")

        scratch = tempfile.TemporaryDirectory(prefix="openui-capture-unit-")
        self.addCleanup(scratch.cleanup)
        output = Path(scratch.name) / "capture.png"
        with (
            patch.object(capture, "chrome_environment", return_value={}),
            patch.object(capture.tempfile, "TemporaryDirectory", return_value=Profile()),
            patch.object(capture.subprocess, "Popen", return_value=process) as launch,
            patch.object(capture, "_wait_for_devtools_endpoint", return_value=123),
            patch.object(capture, "_page_websocket_url", return_value="ws://fake"),
            patch.object(capture, "_CdpWebSocket", return_value=Client()),
            patch.object(capture.os, "killpg", side_effect=signal_group),
            patch.object(
                capture, "_chromium_group_has_live_processes",
                side_effect=lambda group: children_running[0],
            ),
        ):
            result = capture.render_chrome(
                "/tmp/frozen.html", str(output), "chrome", "chrome-dir",
                logical_width=1, logical_height=1, device_scale=1,
            )
        self.assertTrue(launch.call_args.kwargs["start_new_session"])
        self.assertFalse(process.running)
        self.assertFalse(children_running[0])
        self.assertLess(events.index("client-close"), events.index("terminate"))
        self.assertLess(events.index("wait"), events.index("profile-cleanup"))
        return result, events, output

    def test_stable_capture_succeeds_before_profile_cleanup(self):
        result, _, output = self.exercise()
        self.assertTrue(result)
        self.assertTrue(output.exists())

    def test_cdp_failure_stops_browser_before_cleanup(self):
        result, _, output = self.exercise("aborted")
        self.assertFalse(result)
        self.assertFalse(output.exists())

    def test_incorrect_viewport_still_fails(self):
        result, _, output = self.exercise("bad-metrics")
        self.assertFalse(result)
        self.assertFalse(output.exists())

    def test_stalled_shutdown_is_killed_before_cleanup(self):
        result, events, output = self.exercise("hung")
        self.assertTrue(result)
        self.assertTrue(output.exists())
        self.assertLess(events.index("kill"), events.index("profile-cleanup"))

    def test_orphaned_writer_is_killed_before_cleanup(self):
        result, events, output = self.exercise("hung-child")
        self.assertTrue(result)
        self.assertTrue(output.exists())
        self.assertLess(events.index("kill"), events.index("profile-cleanup"))


if __name__ == "__main__":
    unittest.main()
