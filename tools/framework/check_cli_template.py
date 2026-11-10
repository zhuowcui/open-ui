#!/usr/bin/env python3
"""Create an app with `openui new` and check that it builds and renders.

The app is generated from this checkout (`--framework-path`), so the check
covers the template, the patch table, the seeded lockfile and the Skia build
profile in `.cargo/config.toml` together. Nothing else sets the Skia
variables: the app must get them from its own configuration.

1. Build the CLI and run `openui new`.
2. `openui build` (release) and a plain `cargo build` (dev profile).
3. Run both executables with OPENUI_RENDER_PNG and require a PNG that is
   byte for byte the same in dev and release, because the template keeps
   Skia's opt-level equal in both profiles.

Writes a JSON report with the build times.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
WORKSPACE = REPO / "bindings" / "rust"
EXE = ".exe" if os.name == "nt" else ""
# Variables the app's .cargo/config.toml must supply on its own.
SKIA_VARS = ("FORCE_SKIA_BUILD", "SKIA_GN_ARGS", "SKIA_NINJA_COMMAND")


def run(command: list[str], cwd: Path, env: dict[str, str]) -> float:
    print("+", " ".join(command), flush=True)
    started = time.monotonic()
    subprocess.run(command, cwd=cwd, env=env, check=True)
    return time.monotonic() - started


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--work", type=Path, help="scratch directory (default: a temporary one)")
    parser.add_argument("--report", type=Path, help="write a JSON report here")
    args = parser.parse_args()

    env = {key: value for key, value in os.environ.items() if key not in SKIA_VARS}
    work = args.work or Path(tempfile.mkdtemp(prefix="openui-cli-template-"))
    app = work / "demo"
    if app.exists():
        shutil.rmtree(app)
    cli_target = work / "cli-target"
    app_target = work / "app-target"

    run(
        ["cargo", "build", "--locked", "--package", "openui-cli", "--target-dir", str(cli_target)],
        WORKSPACE,
        env,
    )
    cli = str(cli_target / "debug" / f"openui{EXE}")
    run([cli, "new", "demo", "--framework-path", str(REPO), "--path", str(app)], work, env)

    app_env = dict(env, CARGO_TARGET_DIR=str(app_target))
    release_seconds = run([cli, "build"], app, app_env)
    dev_seconds = run(["cargo", "build"], app, app_env)

    system = platform.system().lower()
    machine = {"amd64": "x86_64"}.get(platform.machine().lower(), platform.machine().lower())
    executables = {
        "release": app / "dist" / f"demo-0.1.0-{system}-{machine}" / f"demo{EXE}",
        "dev": app_target / "debug" / f"demo{EXE}",
    }
    renders = {}
    for profile, executable in executables.items():
        png = work / f"{profile}.png"
        png.unlink(missing_ok=True)
        subprocess.run([str(executable)], cwd=app, env=dict(env, OPENUI_RENDER_PNG=str(png)), check=True)
        renders[profile] = png.read_bytes()
        if not renders[profile].startswith(b"\x89PNG\r\n\x1a\n"):
            print(f"{profile} render is not a PNG", file=sys.stderr)
            return 1

    identical = renders["dev"] == renders["release"]
    report = {
        "platform": f"{system}-{machine}",
        "release_build_seconds": round(release_seconds, 1),
        "dev_build_seconds": round(dev_seconds, 1),
        "render_bytes": len(renders["release"]),
        "dev_and_release_identical": identical,
    }
    print(json.dumps(report, indent=2))
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if not identical:
        print("dev and release renders differ", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
