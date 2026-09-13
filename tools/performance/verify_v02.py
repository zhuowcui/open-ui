#!/usr/bin/env python3
"""Validate a v0.2 performance artifact against the frozen gate schema."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PROFILE = ROOT / "tools/performance/v02-reference-profile.json"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("artifact", type=Path, nargs="?")
    args = parser.parse_args()
    profile = json.loads(PROFILE.read_text(encoding="utf-8"))
    required = profile["requirements"]
    expected = {
        "active_scene_hz_min",
        "p95_input_to_present_ms_max",
        "compositor_animations",
        "compositor_animation_presented_fps_min",
        "ui_thread_block_ms",
        "idle_cpu_single_core_percent_max",
        "measured_interactions",
        "rss_growth_percent_max",
        "owned_engine_object_leaks",
        "unchanged_layouts",
        "unchanged_paints",
    }
    if profile.get("schema_version") != 1 or set(required) != expected:
        raise SystemExit("invalid v0.2 performance profile schema")
    if args.artifact is None:
        print("v0.2 performance profile: schema valid; qualification still required")
        return

    artifact = json.loads(args.artifact.read_text(encoding="utf-8"))
    measurements = artifact.get("measurements", artifact)
    failures = []
    if measurements["p95_input_to_present_ms"] > required["p95_input_to_present_ms_max"]:
        failures.append("p95 input-to-present latency")
    if measurements["hundred_ui_thread_animations_fps"] < required["active_scene_hz_min"]:
        failures.append("active scene throughput")
    if not measurements["unchanged_layout_and_paint_work_zero"]:
        failures.append("unchanged lifecycle work")
    if not measurements["unchanged_raster_work_zero"]:
        failures.append("unchanged raster work")
    if measurements["owned_object_leak"]:
        failures.append("owned engine objects")
    growth = measurements.get("rss_growth_percent")
    if growth is not None and growth > required["rss_growth_percent_max"]:
        failures.append("resident memory growth")
    if failures:
        raise SystemExit("performance smoke failures: " + ", ".join(failures))
    print("v0.2 local performance smoke gates passed")


if __name__ == "__main__":
    main()
