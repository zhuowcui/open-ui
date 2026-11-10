#!/usr/bin/env python3
"""Create apps with `openui new` and check that they build, render and test.

Each app is generated from this checkout (`--framework-path`), so the check
covers the template, the patch table, the seeded lockfile and the Skia build
profile in `.cargo/config.toml` together. Nothing else sets the Skia
variables: the app must get them from its own configuration.

For each template:

1. Run `openui new` with the CLI built from this checkout.
2. `openui build` (release) and a plain `cargo build` (dev profile).
3. Run both executables with OPENUI_RENDER_PNG and require a PNG that is
   byte for byte the same in dev and release, because the template keeps
   Skia's opt-level equal in both profiles.
4. If the template has UI tests: `cargo test` must fail on the missing
   snapshot, `OPENUI_UPDATE_SNAPSHOTS=1 cargo test` must record it, and a
   plain `cargo test` must then pass against it.

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
UPDATE_SNAPSHOTS = "OPENUI_UPDATE_SNAPSHOTS"


def run(command: list[str], cwd: Path, env: dict[str, str]) -> float:
    print("+", " ".join(command), flush=True)
    started = time.monotonic()
    subprocess.run(command, cwd=cwd, env=env, check=True)
    return time.monotonic() - started


def host() -> tuple[str, str]:
    system = platform.system().lower()
    machine = {"amd64": "x86_64"}.get(platform.machine().lower(), platform.machine().lower())
    return system, machine


def check_template(template: str, cli: str, work: Path, env: dict[str, str]) -> dict:
    app = work / "demo"
    if app.exists():
        shutil.rmtree(app)
    # Apps share one target directory, so later templates reuse Skia.
    app_target = work / "app-target"
    run(
        [cli, "new", "demo", "--template", template, "--framework-path", str(REPO), "--path", str(app)],
        work,
        env,
    )

    app_env = dict(env, CARGO_TARGET_DIR=str(app_target))
    release_seconds = run([cli, "build"], app, app_env)
    dev_seconds = run(["cargo", "build"], app, app_env)

    system, machine = host()
    executables = {
        "release": app / "dist" / f"demo-0.1.0-{system}-{machine}" / f"demo{EXE}",
        "dev": app_target / "debug" / f"demo{EXE}",
    }
    renders = {}
    for profile, executable in executables.items():
        png = work / f"{template}-{profile}.png"
        png.unlink(missing_ok=True)
        subprocess.run([str(executable)], cwd=app, env=dict(env, OPENUI_RENDER_PNG=str(png)), check=True)
        renders[profile] = png.read_bytes()
        if not renders[profile].startswith(b"\x89PNG\r\n\x1a\n"):
            raise SystemExit(f"{template}: the {profile} render is not a PNG")

    report = {
        "template": template,
        "release_build_seconds": round(release_seconds, 1),
        "dev_build_seconds": round(dev_seconds, 1),
        "render_bytes": len(renders["release"]),
        "dev_and_release_identical": renders["dev"] == renders["release"],
    }
    if (app / "tests" / "ui.rs").exists():
        report.update(check_snapshots(app, app_env))
    return report


def check_snapshots(app: Path, env: dict[str, str]) -> dict:
    """Run the app's tests without, while recording, and against snapshots."""
    env = {key: value for key, value in env.items() if key != UPDATE_SNAPSHOTS}
    print("+ cargo test (no snapshot yet, so it must fail)", flush=True)
    started = time.monotonic()
    first = subprocess.run(["cargo", "test"], cwd=app, env=env, capture_output=True, text=True)
    test_build_seconds = time.monotonic() - started
    output = first.stdout + first.stderr
    refused = first.returncode != 0 and "no accepted snapshot" in output
    if not refused:
        print(output)
    run(["cargo", "test"], app, dict(env, **{UPDATE_SNAPSHOTS: "1"}))
    recorded = sorted(path.name for path in (app / "tests" / "snapshots").glob("*.png"))
    run(["cargo", "test"], app, env)
    return {
        "test_build_seconds": round(test_build_seconds, 1),
        "missing_snapshot_refused": refused,
        "snapshots_recorded": recorded,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--work", type=Path, help="scratch directory (default: a temporary one)")
    parser.add_argument("--report", type=Path, help="write a JSON report here")
    parser.add_argument(
        "--template",
        action="append",
        help="a template to check; repeat for several (default: minimal and default)",
    )
    args = parser.parse_args()

    env = {key: value for key, value in os.environ.items() if key not in SKIA_VARS}
    work = args.work or Path(tempfile.mkdtemp(prefix="openui-cli-template-"))
    work.mkdir(parents=True, exist_ok=True)
    cli_target = work / "cli-target"
    run(
        ["cargo", "build", "--locked", "--package", "openui-cli", "--target-dir", str(cli_target)],
        WORKSPACE,
        env,
    )
    cli = str(cli_target / "debug" / f"openui{EXE}")

    system, machine = host()
    report = {"platform": f"{system}-{machine}", "templates": []}
    for template in args.template or ["minimal", "default"]:
        report["templates"].append(check_template(template, cli, work, env))
    print(json.dumps(report, indent=2))
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    failed = False
    for result in report["templates"]:
        if not result["dev_and_release_identical"]:
            print(f"{result['template']}: dev and release renders differ", file=sys.stderr)
            failed = True
        if result.get("missing_snapshot_refused") is False:
            print(f"{result['template']}: a missing snapshot did not fail the tests", file=sys.stderr)
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
