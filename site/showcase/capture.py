"""Draw the browser side of the home page's comparison with the pinned Chromium.

    python3 site/showcase/capture.py

Run it on Linux (WSL here) with the pinned Chrome-for-Testing that the
components harness uses (components/harness/reference_harness.py). It
renders site/showcase/reference/example.html at 440x220 CSS pixels and
device scale 2, the showcase renderer's `example` scene, with the harness's
own capture function and real-font environment, and writes
site/src/assets/showcase/example-chromium-light.png.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "components/harness"))

import reference_harness as harness  # noqa: E402


def main() -> int:
    chrome = Path(sys.argv[1]) if len(sys.argv) > 1 else harness.DEFAULT_CHROME
    version = harness.chromium_version(chrome)
    page = ROOT / "site/showcase/reference/example.html"
    output = ROOT / "site/src/assets/showcase/example-chromium-light.png"
    profile = harness.Profile("site-example", 440, 220, 2.0)
    if not harness.render_chromium(chrome, page, profile, output):
        print("Chromium did not render the page", file=sys.stderr)
        return 1
    print(f"{output} ({version})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
