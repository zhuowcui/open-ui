#!/usr/bin/env python3
"""Promote and validate the frozen SP13-P runnable paint closure."""

from __future__ import annotations

import csv
import hashlib
import io
import json
import sys
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DATA = ROOT / "tools" / "accountability" / "data"
PORTED = DATA / "wpt_ported"
RESULTS = DATA / "pixel_comparison" / "results"
SP19_KICKOFF_MAPPING = PORTED / "sp19_kickoff_mapping.csv"
SP19_KICKOFF_SUMMARY = PORTED / "sp19_sp18_summary.json"
MAPPING = (
    SP19_KICKOFF_MAPPING
    if SP19_KICKOFF_MAPPING.is_file()
    else DATA / "wpt_mapping.csv"
)
DEFERRED = DATA / "sp12_5_deferred.csv"
TARGETS = PORTED / "sp13p_paint_targets.json"
FOCUSED = PORTED / "sp13p_paint_focused.json"
RESIDUALS = PORTED / "sp13p_paint_residuals.json"
ASSET_MANIFEST = DATA / "wpt_assets" / "sp13p_manifest.json"
ASSET_DIR = DATA / "wpt_assets" / "sp13p"
SUMMARY = SP19_KICKOFF_SUMMARY if SP19_KICKOFF_SUMMARY.is_file() else RESULTS / "summary.json"

EXPECTED_HASHES = {
    TARGETS: "322d86866abdd23d14e435cba71e7f1239d49f7ad1914ac607ac7a5b7a387324",
    FOCUSED: "4dcf84612a43d8d05d7132992bde3e7daac0504d1beafcdc7efbcfe8a95d575c",
    RESIDUALS: "90293178128bb5d7cd663b6eb6ee427f43385a31bf44927f872672fc29898f5c",
    ASSET_MANIFEST: "51f0ba380ed4308686b9d1af0c3696c5d12c33487ff212a6ca737ddc04804708",
}
EXPECTED_BASELINE_HASH = (
    "b83b5602c7e76ebae7c532e269041f2f44632c825b757e90194da56c63bc4ba8"
)
EXPECTED_NON_TARGET_PROJECTION = (
    "d64731db788ab60a311bafbbfad8a1ef68006a584eb87ee4e698eae876c7ee92"
)
EXPECTED_HISTORICAL_LEDGERS = {
    "sp13r_baseline_exact.json": "07305185d51727fca6a0b739f2c87b0da18963e93216407868d6284d89e98c1c",
    "sp13r_multicol_residuals.json": "1268c9bbdc9f36ef23fc86204ac1c34259340711fe34dfca4fcd67b3ab45de67",
    "sp13r_multicol_targets.json": "140edff40c172d06b6e2dc3d91e27adc0fddf12bf9c12c04ea9cdfa7c73dd6b9",
    "sp14_w2_targets.json": "8ef76f5f38baaaaed12542285b9716b57a4fda1a318d35ecfbb3633904fee600",
    "sp14_w3_baseline_exact.json": "6c350c67d62a2a5868fb3346851db02f16ca293ae7cd24f113df28ee6236c078",
    "sp14_w3_targets.json": "ead9d861c44ec296a99a3b0887e14d5e66f456f9de368ba8cdf89c02620ea86d",
    "sp14_w4_residuals.json": "258e001a0fbe41adfe2008899ba09d746aea8a33bbe21b0bf532711491149f4a",
    "sp15_actionable_targets.json": "210023e7342d96acc9cd491521e4f100c6fe3a2cbc4bf56a596f05b18405f9ad",
    "sp15_baseline_exact.json": "9dcebde69adc70b39660ef8fff771af6f57339c0fd4dd242109a22bf5a4d391d",
    "sp15_residual_dispositions.json": "f901127ee8ae19a150965a6edbc121185f2a47b9d6d121cc895b453ae4d430af",
    "sp16_actionable_targets.json": "fc457282f5335773d1081fa9a949aaf29d401bac6dcaf38560564a1bcf3aef86",
    "sp16_baseline_exact.json": "668387215999c79c1dbae11fd8a0e06aceaf0c497b1b3afeda1b31725aa3b668",
    "sp16_real_font_tests.json": "fc457282f5335773d1081fa9a949aaf29d401bac6dcaf38560564a1bcf3aef86",
    "sp16_residual_dispositions.json": "72df8f9e2d8c6141cf266d51a4d2114101592b58dd006494b5c8eb9196d04bcc",
    "sp17_actionable_targets.json": "9d2b53070cf206b6c37a5c66bd0d37ea757e12b7ca6d4a19a5eb98ee4579f96c",
    "sp17_baseline_exact.json": "59a514d3b76b83ecc44efd43dda5a16ec2a0203803849407f3dce035d9e9fc20",
    "sp17_freetype_text_tests.json": "cd3dad3443cd4d16684119addcb8cc6ea5b98f83d385563c55c0a6684a3ce0e5",
    "sp17_initial_runnable_results.json": "1fcc02a0e742df04cec80e06750f74e6859ef11552d5488300c590095432d00d",
    "sp17_initial_runnable_targets.json": "f82d99182bf276ddd524b2894e2de6b21e8f1f5bf7d8f3bd984c8ef0e2c040c4",
    "sp17_residual_dispositions.json": "314a7a27f250ef1fb5f65b86e69f48771e5116a4b6592bce2190d474a9338776",
    "sp17_writing_mode_inventory.json": "b72a0b0b4e74f4c1cb912ab65642dd6f5bef76a570f0a9b219f68729de6d10ad",
}

PAINT_OWNERS = {
    "needs_gradient": 55,
    "needs_image": 313,
    "needs_complex_border": 74,
    "needs_rounded_border_paint": 117,
    "needs_box_shadow": 9,
    "needs_body_canvas_background_extent": 16,
    "needs_scrollbar_paint": 18,
    "needs_special_background_clip": 2,
}


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def canonical_id(row: dict[str, str]) -> str:
    return f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"


def load_manifests() -> tuple[list[str], list[str], list[str], dict]:
    for path, expected in EXPECTED_HASHES.items():
        actual = digest_file(path)
        if actual != expected:
            raise ValueError(f"frozen SP13-P hash drift: {path.name}: {actual}")
    targets = json.loads(TARGETS.read_text(encoding="utf-8"))
    focused = json.loads(FOCUSED.read_text(encoding="utf-8"))
    residuals = json.loads(RESIDUALS.read_text(encoding="utf-8"))
    assets = json.loads(ASSET_MANIFEST.read_text(encoding="utf-8"))
    if targets != sorted(set(targets)) or len(targets) != 188:
        raise ValueError("SP13-P target manifest is not the frozen sorted 188-ID set")
    if focused != sorted(set(focused)) or len(focused) != 3807:
        raise ValueError("SP13-P focused manifest is not the frozen sorted 3,807-ID set")
    if residuals != sorted(set(residuals)) or len(residuals) != 82:
        raise ValueError("SP13-P residual manifest is not the frozen sorted 82-ID set")
    if not set(targets).issubset(focused) or set(focused) & set(residuals):
        raise ValueError("SP13-P manifests overlap or omit a target")
    if len(set(focused) | set(residuals)) != 3889:
        raise ValueError("SP13-P focused and residual manifests do not cover 3,889 IDs")
    baseline = sorted(set(focused) - set(targets))
    encoded_baseline = (json.dumps(baseline, indent=2) + "\n").encode()
    if len(baseline) != 3619 or digest_bytes(encoded_baseline) != EXPECTED_BASELINE_HASH:
        raise ValueError("SP13-P frozen 3,619-ID baseline projection changed")
    return targets, focused, residuals, assets


def validate_assets(assets: dict) -> None:
    if set(assets) != {"license", "provenance", "assets", "embedded_data_urls"}:
        raise ValueError("SP13-P asset manifest schema drift")
    if assets["license"] != "W3C 3-Clause BSD" or not assets["provenance"]:
        raise ValueError("SP13-P asset license/provenance missing")
    rows = assets["assets"]
    embedded = assets["embedded_data_urls"]
    if len(rows) != 20 or len(embedded) != 2:
        raise ValueError("SP13-P must freeze 20 files and two embedded data URLs")
    required = {"path", "source_path", "mime_type", "byte_size", "sha256"}
    if len({row.get("path") for row in rows}) != 20:
        raise ValueError("SP13-P asset paths are not unique")
    for row in rows:
        if set(row) != required or not row["source_path"]:
            raise ValueError(f"invalid SP13-P asset row: {row!r}")
        path = ASSET_DIR / row["path"]
        raw = path.read_bytes()
        if len(raw) != row["byte_size"] or digest_bytes(raw) != row["sha256"]:
            raise ValueError(f"SP13-P asset byte drift: {row['path']}")
        if row["mime_type"] not in {"image/png", "image/svg+xml"}:
            raise ValueError(f"unsupported SP13-P asset MIME: {row['mime_type']}")
    for row in embedded:
        if set(row) != {"source_path", "mime_type", "byte_size", "sha256"}:
            raise ValueError(f"invalid embedded SP13-P asset row: {row!r}")
        if row["mime_type"] != "image/svg+xml" or not row["source_path"]:
            raise ValueError("embedded SP13-P provenance/MIME drift")


def non_target_projection(summary: dict, targets: set[str]) -> str:
    fields = (
        "id", "status", "mismatched_pixels", "mismatch_pct",
        "max_channel_diff", "avg_channel_diff",
    )
    projection = sorted(
        ({key: row.get(key) for key in fields}
         for row in summary.get("tests", []) if row.get("id") not in targets),
        key=lambda row: row["id"],
    )
    encoded = json.dumps(projection, sort_keys=True, separators=(",", ":")).encode()
    return digest_bytes(encoded)


def validate_historical_ledgers() -> None:
    for name, expected in EXPECTED_HISTORICAL_LEDGERS.items():
        if digest_file(PORTED / name) != expected:
            raise ValueError(f"historical ledger changed: {name}")


def validate_closed_snapshot() -> None:
    targets, focused, residuals, assets = load_manifests()
    validate_assets(assets)
    validate_historical_ledgers()
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    by_id = {row["id"]: row for row in summary.get("tests", [])}
    if (
        summary.get("total"), summary.get("passed"), summary.get("failed"),
        summary.get("errors"), len(by_id),
    ) != (4139, 4058, 81, 0, 4139):
        raise ValueError("SP13-P/SP18 final summary identity changed")
    if not set(focused) | set(residuals) <= set(by_id):
        raise ValueError("SP13-P frozen proof is absent from the live summary")
    for test_id in focused:
        row = by_id[test_id]
        result = json.loads((RESULTS / test_id / "result.json").read_text(encoding="utf-8"))
        if (
            row.get("status") != "pass"
            or row.get("mismatch_pct") != 0.0
            or result.get("status") != "pass"
            or result.get("mismatched_pixels") != 0
            or result.get("mismatch_pct") != 0.0
        ):
            raise ValueError(f"SP13-P focused proof is not exact: {test_id}")
    for test_id in residuals:
        row = by_id[test_id]
        if row.get("status") not in {"fail", "pass"} or (
            row.get("status") == "pass" and row.get("mismatch_pct") != 0.0
        ):
            raise ValueError(f"SP13-P residual changed incompatibly: {test_id}")
    if non_target_projection(summary, set(targets)) != EXPECTED_NON_TARGET_PROJECTION:
        raise ValueError("SP13-P changed a non-target status or mismatch value")

    with MAPPING.open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source))
    mapping = {canonical_id(row): row for row in rows}
    if len(rows) != 7673 or len(mapping) != 7673:
        raise ValueError("SP13-P mapping must contain 7,673 unique rows")
    if sum(row["ported"] == "yes" for row in rows) != 4139:
        raise ValueError("SP13-P/SP18 mapping runnable count is not 4,139")
    if sum(row["ported"] == "no" for row in rows) != 3534:
        raise ValueError("SP13-P/SP18 mapping unported count is not 3,534")
    for test_id in targets:
        row = mapping[test_id]
        if (
            row["ported"] != "yes" or row["our_test_id"] != test_id
            or row["pixel_result"] != "pass"
            or float(row["mismatch_pct"]) != 0.0
            or row["failure_category"] or row["dependency"]
        ):
            raise ValueError(f"SP13-P mapping target is not closed: {test_id}")
    for owner, expected in PAINT_OWNERS.items():
        owned = [row for row in rows if owner in categories(row["failure_category"])]
        if len(owned) != expected or any(row["ported"] != "no" for row in owned):
            raise ValueError(f"SP13-P remaining {owner} inventory changed")

    failed_ids = {row["id"] for row in summary["tests"] if row["status"] == "fail"}
    if SP19_KICKOFF_SUMMARY.is_file():
        deferred_ids = {
            row["test_id"]
            for row in json.loads(
                (PORTED / "sp19_initial_mismatches.json").read_text(encoding="utf-8")
            )
        }
    else:
        with DEFERRED.open(newline="", encoding="utf-8") as source:
            deferred_ids = {row["test_id"] for row in csv.DictReader(source)}
    if len(deferred_ids) != 81 or deferred_ids != failed_ids:
        raise ValueError("SP13-P/SP18 deferred CSV is not the live 81-ID failure set")
    text_ids = json.loads((PORTED / "text_ported_tests.json").read_text())
    if len(text_ids) != 1289:
        raise ValueError("SP13-P/SP19-W1B text manifest count changed from 1,289")
    writing_owners = [
        row for row in rows if "needs_writing_mode" in categories(row["failure_category"])
    ]
    if len(writing_owners) != 510 or any(row["ported"] != "no" for row in writing_owners):
        raise ValueError("SP13-P/SP18 writing-mode owner count changed from 510")
    sp13r = json.loads((PORTED / "sp13r_multicol_residuals.json").read_text())
    from generate_sp13r_multicol_closure import LATER_EXACT_PROMOTIONS
    frozen_sp13r = [row for row in sp13r if row["test_id"] not in LATER_EXACT_PROMOTIONS]
    if len(frozen_sp13r) != 938 or len(LATER_EXACT_PROMOTIONS) != 80:
        raise ValueError("SP13-P/SP18 changed the frozen SP13-R 938/80 split")


def _encode_csv_row(fieldnames: list[str], row: dict[str, str]) -> str:
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=fieldnames, lineterminator="\n")
    writer.writerow(row)
    return output.getvalue()


def promoted_mapping_text(
    original: str, summary_by_id: dict[str, dict], targets: set[str]
) -> str:
    """Promote only target records while retaining every non-target byte."""
    lines = original.splitlines(keepends=True)
    reader = csv.reader(lines)
    fieldnames = next(reader)
    record_start = reader.line_num
    updated = ["".join(lines[:record_start])]
    seen: set[str] = set()
    for values in reader:
        record_end = reader.line_num
        original_record = "".join(lines[record_start:record_end])
        record_start = record_end
        row = dict(zip(fieldnames, values))
        test_id = canonical_id(row)
        if test_id not in targets:
            updated.append(original_record)
            continue
        result = summary_by_id.get(test_id)
        if not result or result.get("status") != "pass" or result.get("mismatched_pixels") != 0:
            raise ValueError(f"cannot promote non-exact SP13-P target: {test_id}")
        row.update(
            ported="yes", our_test_id=test_id, pixel_result="pass",
            mismatch_pct="0.0", failure_category="", dependency="", notes="",
        )
        updated.append(_encode_csv_row(fieldnames, row))
        seen.add(test_id)
    if seen != targets:
        raise ValueError("SP13-P mapping splice did not find every target")
    return "".join(updated)


def residual_deferred_text(original: str, targets: set[str]) -> str:
    """Remove only promoted records while retaining every residual byte."""
    lines = original.splitlines(keepends=True)
    reader = csv.reader(lines)
    fieldnames = next(reader)
    test_id_index = fieldnames.index("test_id")
    record_start = reader.line_num
    result = ["".join(lines[:record_start])]
    removed: set[str] = set()
    for values in reader:
        record_end = reader.line_num
        original_record = "".join(lines[record_start:record_end])
        record_start = record_end
        test_id = values[test_id_index]
        if test_id in targets:
            removed.add(test_id)
        else:
            result.append(original_record)
    if removed != targets:
        raise ValueError("SP13-P deferred splice did not find every target")
    return "".join(result)


def promote() -> None:
    targets, _, _, _ = load_manifests()
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    summary_by_id = {row["id"]: row for row in summary.get("tests", [])}
    target_set = set(targets)
    for test_id in targets:
        result = json.loads(
            (RESULTS / test_id / "result.json").read_text(encoding="utf-8")
        )
        summary_by_id.setdefault(test_id, {}).update(result)
    mapping = promoted_mapping_text(MAPPING.read_text(encoding="utf-8"), summary_by_id, target_set)
    deferred = residual_deferred_text(DEFERRED.read_text(encoding="utf-8"), target_set)
    MAPPING.write_text(mapping, encoding="utf-8")
    DEFERRED.write_text(deferred, encoding="utf-8")


def main() -> int:
    if sys.argv[1:] == ["--promote"]:
        promote()
        print("SP13-P promoted exactly 188 mapping/deferred rows")
        return 0
    if sys.argv[1:] not in ([], ["--check"]):
        print("Usage: generate_sp13p_paint_closure.py [--check|--promote]", file=sys.stderr)
        return 2
    validate_closed_snapshot()
    print("SP13-P closure: 3,889 runnable, 3,807 exact, 82 residual, 0 errors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
