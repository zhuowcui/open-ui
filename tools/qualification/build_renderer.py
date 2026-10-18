#!/usr/bin/env python3
"""Build the offline renderer with freshly compiled local Rust dependencies.

The executable's embedded checkout identity does not identify cached libraries.
Clean local packages before building, then require Cargo's artifact records to
confirm that every linked renderer crate was compiled from this checkout.
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys


RENDERER_PACKAGES = {
    "pixel-compare", "openui-geometry", "openui-style", "openui-text",
    "openui-dom", "openui-layout", "openui-paint", "openui-compositor",
    "openui-engine",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def local_packages(metadata: dict, root: Path) -> list[dict]:
    packages = [package for package in metadata["packages"]
                if package["source"] is None]
    for package in packages:
        manifest = Path(package["manifest_path"]).resolve()
        if not manifest.is_relative_to(root.resolve()):
            raise ValueError(f"local dependency outside measured checkout: {manifest}")
    if not RENDERER_PACKAGES.issubset({p["name"] for p in packages}):
        raise ValueError("metadata is missing linked renderer packages")
    return sorted(packages, key=lambda package: package["id"])


def verify_artifacts(metadata: dict, messages: list[dict], root: Path) -> dict:
    local = {p["id"]: p for p in local_packages(metadata, root)}
    artifacts = {}
    binary = None
    for message in messages:
        if message.get("reason") != "compiler-artifact":
            continue
        package = local.get(message["package_id"])
        if package is None or message["target"]["kind"] == ["custom-build"]:
            continue
        manifest = Path(message["manifest_path"]).resolve()
        if manifest != Path(package["manifest_path"]).resolve():
            raise ValueError(f"artifact came from another checkout: {package['name']}")
        if message.get("fresh") is not False:
            raise ValueError(f"cached local artifact cannot prove its source: {package['name']}")
        if not Path(message["target"]["src_path"]).resolve().is_relative_to(root.resolve()):
            raise ValueError(f"artifact source outside measured checkout: {package['name']}")
        artifacts.setdefault(package["name"], []).append(message)
        if package["name"] == "pixel-compare" and message["target"]["name"] == "pixel_compare":
            if message.get("executable"):
                binary = Path(message["executable"])
    missing = RENDERER_PACKAGES - artifacts.keys()
    if missing:
        raise ValueError("missing fresh renderer artifacts: " + ", ".join(sorted(missing)))
    if binary is None:
        raise ValueError("missing pixel_compare executable artifact")
    return {"artifacts": artifacts, "binary": str(binary)}


def build(args: argparse.Namespace) -> dict:
    root = args.source_root.resolve()
    sys.path.insert(0, str(root / "tools/qualification"))
    from renderer_source_identity import repository_source_identity

    source = repository_source_identity(root)
    if not source["clean"] and not args.allow_dirty_diagnostics:
        raise ValueError("renderer builds require a clean measured checkout")
    owner = open("/tmp/openui-native-cargo-raster-owner.lock", "a+")
    fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
    out = args.results_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    target = args.target_dir.resolve()
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="0",
               PYTHONDONTWRITEBYTECODE="1")
    command = ["cargo"]
    for config in args.cargo_config:
        command.extend(["--config", str(config.resolve())])
    report = {"schema_version": 1, "source": source, "source_root": str(root), "steps": [],
              "dirty_diagnostic_only": not source["clean"],
              "all_commands_terminal": False, "release_qualification": False,
              "renderer_library_provenance_verified": False}

    def save():
        (out / "receipt.json").write_text(json.dumps(report, sort_keys=True, indent=2) + "\n")

    def run(name: str, tail: list[str]) -> bytes:
        stdout = out / (name + ".stdout")
        stderr = out / (name + ".stderr")
        with stdout.open("xb") as output, stderr.open("xb") as errors:
            child = subprocess.Popen(command + tail, cwd=root / "bindings/rust", env=env,
                                     stdout=output, stderr=errors, start_new_session=True)
            report["current_process"] = {"pid": child.pid, "stage": name}
            save()
            try:
                result = child.wait()
            except BaseException:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                raise
            finally:
                report.pop("current_process", None)
                report["steps"].append({"name": name, "command": command + tail,
                                        "observed_exit_code": child.returncode,
                                        "stdout_sha256": sha256(stdout),
                                        "stderr_sha256": sha256(stderr)})
                save()
        if repository_source_identity(root) != source:
            raise ValueError("checkout changed during renderer build")
        if result:
            raise RuntimeError(f"{name} failed with exit {result}; see {stderr}")
        return stdout.read_bytes()

    save()
    try:
        locked = ["--locked"] + (["--offline"] if args.offline else [])
        metadata = json.loads(run("metadata", ["metadata", *locked, "--format-version", "1"]))
        packages = local_packages(metadata, root)
        report["cleaned_local_package_ids"] = [p["id"] for p in packages]
        run("clean-local-packages", ["clean", "--locked", *[
            argument for package in packages for argument in ["-p", package["id"]]
        ]])
        data = run("build", ["build", *locked, "--package", "pixel-compare",
                             "--bin", "pixel_compare", "--message-format=json"])
        verified = verify_artifacts(metadata, [json.loads(line) for line in data.splitlines()], root)
        original = Path(verified["binary"])
        binary = out / "pixel_compare"
        shutil.copy2(original, binary)
        embedded = subprocess.run([str(binary), "build-source-identity"], check=True,
                                  capture_output=True, text=True, cwd=root)
        if json.loads(embedded.stdout).get("source") != source:
            raise ValueError("runner identity does not match measured checkout")
        files = sorted({path for records in verified["artifacts"].values()
                        for record in records for path in record["filenames"]})
        report.update(renderer_library_provenance_verified=True,
                      artifacts=verified["artifacts"],
                      compiled_artifact_sha256={p: sha256(Path(p)) for p in files},
                      binary=str(binary), binary_sha256=sha256(binary), observed_exit_code=0)
    except BaseException as error:
        report.update(observed_exit_code=1, failure=str(error))
        raise
    finally:
        report.update(source_after=repository_source_identity(root), all_commands_terminal=True)
        report["source_unchanged"] = report["source_after"] == source
        if not report["source_unchanged"]:
            report.update(observed_exit_code=1, renderer_library_provenance_verified=False,
                          failure="checkout changed during renderer build")
        save()
        owner.close()
    if not report["source_unchanged"]:
        raise ValueError(report["failure"])
    return report


def verify_build_receipt(receipt: dict, binary: Path, source: dict, *,
                         allow_dirty_diagnostics: bool = False) -> dict:
    """Reject a runner label that lacks proof of its linked local libraries."""
    if receipt.get("schema_version") != 1:
        raise ValueError("unsupported renderer library build receipt")
    if not (receipt.get("all_commands_terminal") is True
            and receipt.get("renderer_library_provenance_verified") is True
            and receipt.get("observed_exit_code") == 0):
        raise ValueError("renderer library build did not complete successfully")
    if receipt.get("source") != source or receipt.get("source_after") != source:
        raise ValueError("renderer library build source does not match checkout")
    if not source.get("clean") and not allow_dirty_diagnostics:
        raise ValueError("renderer library build source is not clean")
    if receipt.get("binary_sha256") != sha256(binary):
        raise ValueError("renderer library build receipt belongs to another executable")
    root = Path(receipt["source_root"]).resolve()
    artifacts = receipt.get("artifacts", {})
    missing = RENDERER_PACKAGES - artifacts.keys()
    if missing:
        raise ValueError("missing linked library proof: " + ", ".join(sorted(missing)))
    for name, records in artifacts.items():
        if not records:
            raise ValueError(f"missing linked library proof: {name}")
        for record in records:
            if (record.get("reason") != "compiler-artifact"
                    or record["target"]["kind"] == ["custom-build"]):
                raise ValueError(f"record does not prove a compiled linked library: {name}")
            if record.get("fresh") is not False:
                raise ValueError(f"cached linked library proof: {name}")
            for path in [record["manifest_path"], record["target"]["src_path"]]:
                if not Path(path).resolve().is_relative_to(root):
                    raise ValueError(f"linked library proof outside measured checkout: {name}")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--results-dir", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--cargo-config", type=Path, action="append", default=[])
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--allow-dirty-diagnostics", action="store_true",
                        help="record an unchanged dirty source for diagnostics; never qualifies a release")
    result = build(parser.parse_args())
    print(json.dumps({key: result[key] for key in
                      ["binary", "binary_sha256", "source", "renderer_library_provenance_verified"]}))


if __name__ == "__main__":
    main()
