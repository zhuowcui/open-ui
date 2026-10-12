#!/usr/bin/env python3
"""Audit the reviewed Rust lockfiles without changing release inputs.

Install cargo-audit 0.22.2 separately with Rust 1.94.1. This checker never
installs tools, changes dependencies, or accepts advisory exemptions.
"""

from __future__ import annotations

import argparse
import base64
import concurrent.futures
import datetime as dt
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import tomllib
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
AUDIT_VERSION = "0.22.2"
AUDIT_TOOLCHAIN = "1.94.1"
ADVISORY_URL = "https://github.com/RustSec/advisory-db"
REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
LOCKFILES = ("bindings/rust/Cargo.lock", "bindings/rust/fuzz/Cargo.lock")
CONFIG = """[advisories]
ignore = []

[database]
url = "https://github.com/RustSec/advisory-db"
fetch = true
stale = false

[output]
deny = ["warnings"]
format = "json"
quiet = false
show_tree = false

[target]

[yanked]
enabled = true
update_index = true
"""


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def write_json(path: Path, value: object) -> None:
    temporary = path.with_suffix(path.suffix + ".pending")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def command(argv: list[str], cwd: Path, timeout: int = 600) -> dict:
    """Own, wait for, and reap a command, including a timed-out process group."""
    child = subprocess.Popen(
        argv,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
        env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"},
    )
    timed_out = False
    try:
        stdout, stderr = child.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        stdout, stderr = child.communicate()
    except BaseException:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        child.communicate()
        raise
    return {
        "argv": argv,
        "cwd": str(cwd),
        "pid": child.pid,
        "exit_code": child.returncode,
        "reaped": True,
        "timed_out": timed_out,
        "stdout": stdout.decode("utf-8", errors="replace"),
        "stderr": stderr.decode("utf-8", errors="replace"),
        "stdout_base64": base64.b64encode(stdout).decode("ascii"),
        "stderr_base64": base64.b64encode(stderr).decode("ascii"),
        "stdout_sha256": sha256(stdout),
        "stderr_sha256": sha256(stderr),
    }


def git_value(root: Path, args: list[str], commands: list[dict]) -> str:
    result = command(["git", "--no-optional-locks", "-c", "core.filemode=false", *args], root, 120)
    commands.append(result)
    if result["exit_code"] != 0 or result["timed_out"]:
        raise ValueError(f"source identity command failed: {args}")
    return result["stdout"]


def source_identity(root: Path, commands: list[dict]) -> dict:
    head = git_value(root, ["rev-parse", "HEAD"], commands).strip()
    diff = git_value(root, ["diff", "--no-ext-diff", "--binary", "HEAD"], commands)
    names = git_value(root, ["ls-files", "--others", "--exclude-standard", "-z"], commands)
    untracked = []
    for name in sorted(filter(None, names.split("\0"))):
        path = root / name
        if path.is_symlink():
            untracked.append({"path": name, "symlink": os.readlink(path)})
        else:
            untracked.append({"path": name, "sha256": sha256(path.read_bytes())})
    return {"head": head, "tracked_diff_sha256": sha256(diff.encode()), "untracked": untracked}


def read_locks(root: Path) -> tuple[list[dict], list[dict]]:
    identities = []
    packages = {}
    for name in LOCKFILES:
        path = root / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"reviewed lockfile is missing or not a regular file: {name}")
        raw = path.read_bytes()
        stat = path.stat()
        lock = tomllib.loads(raw.decode("utf-8"))
        if not isinstance(lock.get("package"), list) or not lock["package"]:
            raise ValueError(f"empty or invalid lockfile: {name}")
        identities.append({"path": name, "sha256": sha256(raw), "mtime_ns": stat.st_mtime_ns})
        for package in lock["package"]:
            source = package.get("source")
            if source is None or source.startswith("git+"):
                continue  # Cargo-audit still inspects every package in each lock.
            if source != REGISTRY_SOURCE:
                raise ValueError(f"unqualified registry source: {source}")
            entry = {key: package[key] for key in ("name", "version", "checksum")}
            if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_-]{0,63}", entry["name"]):
                raise ValueError("invalid registry package name")
            if not re.fullmatch(r"[0-9a-f]{64}", entry["checksum"]):
                raise ValueError("invalid registry package checksum")
            key = (entry["name"], entry["version"])
            if key in packages and packages[key] != entry:
                raise ValueError("conflicting checksums across reviewed lockfiles")
            packages[key] = entry
    return identities, sorted(packages.values(), key=lambda p: (p["name"], p["version"]))


def registry_url(name: str) -> str:
    lowered = name.lower()
    if len(lowered) == 1:
        prefix = "1"
    elif len(lowered) == 2:
        prefix = "2"
    elif len(lowered) == 3:
        prefix = "3/" + lowered[0]
    else:
        prefix = lowered[:2] + "/" + lowered[2:4]
    return f"https://index.crates.io/{prefix}/{lowered}"


def fetch_registry(url: str) -> dict:
    request = urllib.request.Request(
        url, headers={"User-Agent": "open-ui-dependency-security/1", "Cache-Control": "no-cache"}
    )
    try:
        response = urllib.request.urlopen(request, timeout=45)
    except urllib.error.HTTPError as error:
        response = error  # Retain the actual HTTP failure body and headers too.
    with response:
        raw = response.read(32 * 1024 * 1024 + 1)
        return {
            "url": url,
            "final_url": response.url,
            "status": response.status,
            "response_exceeds_bound": len(raw) > 32 * 1024 * 1024,
            "received_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
            "headers": {key: response.headers.get(key) for key in ("Date", "ETag", "Last-Modified", "Age")},
            "raw": raw,
        }


def check_registry(packages: list[dict], output: Path) -> list[dict]:
    grouped = {}
    for package in packages:
        grouped.setdefault(package["name"], []).append(package)

    def inspect(item: tuple[str, list[dict]]) -> dict:
        name, entries = item
        result = {"name": name, "url": registry_url(name), "requested": entries, "complete": False, "passed": False}
        try:
            response = fetch_registry(result["url"])
            raw = response.pop("raw")
            result.update(response)
            result["sha256"] = sha256(raw)
            result["raw_file"] = f"registry/{name}.jsonl"
            (output / result["raw_file"]).write_bytes(raw)
            if response["status"] != 200 or response["final_url"] != result["url"] or response["response_exceeds_bound"]:
                raise ValueError("unexpected registry status/redirect or response exceeds bound")
            versions = [json.loads(line) for line in raw.decode("utf-8").splitlines() if line]
            matched = []
            for entry in entries:
                rows = [row for row in versions if row.get("name") == name and row.get("vers") == entry["version"]]
                if len(rows) != 1:
                    raise ValueError(f"missing or duplicated registry version: {name}@{entry['version']}")
                row = rows[0]
                if row.get("cksum") != entry["checksum"] or type(row.get("yanked")) is not bool:
                    raise ValueError(f"invalid checksum/yank metadata: {name}@{entry['version']}")
                matched.append({**entry, "yanked": row["yanked"]})
            result["versions"] = matched
            result["complete"] = True
            result["passed"] = not any(row["yanked"] for row in matched)
            if not result["passed"]:
                result["error"] = "locked registry version is yanked"
        except Exception as error:
            result["error"] = str(error)
        return result

    (output / "registry").mkdir()
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        return list(pool.map(inspect, sorted(grouped.items())))


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate report key: {key}")
        result[key] = value
    return result


def validate_report(raw: str, packages: int, now: dt.datetime) -> dict:
    report = json.loads(raw, object_pairs_hook=unique_object)  # Trailing diagnostics also fail.
    if not isinstance(report, dict):
        raise ValueError("cargo-audit did not return one object")
    settings = report["settings"]
    if any(type(settings[key]) is not list or settings[key] for key in ("ignore", "target_arch", "target_os")) or settings["severity"] is not None:
        raise ValueError("audit report contains exemptions or filters")
    if set(settings["informational_warnings"]) != {"unmaintained", "unsound", "notice"}:
        raise ValueError("informational warning coverage changed")
    database = report["database"]
    if type(database["advisory-count"]) is not int or database["advisory-count"] <= 0:
        raise ValueError("empty advisory database")
    if not re.fullmatch(r"[0-9a-f]{40}", database["last-commit"] or ""):
        raise ValueError("missing advisory database revision")
    updated = dt.datetime.fromisoformat(database["last-updated"].replace("Z", "+00:00"))
    if updated.tzinfo is None or not dt.timedelta(0) <= now - updated <= dt.timedelta(days=90):
        raise ValueError("future or stale advisory database")
    if type(report["lockfile"]["dependency-count"]) is not int or report["lockfile"]["dependency-count"] != packages:
        raise ValueError("audit lockfile coverage is incomplete")
    vulnerabilities = report["vulnerabilities"]
    if type(vulnerabilities["found"]) is not bool or type(vulnerabilities["count"]) is not int:
        raise ValueError("invalid vulnerability counts")
    if vulnerabilities["count"] != len(vulnerabilities["list"]) or vulnerabilities["found"] != bool(vulnerabilities["list"]):
        raise ValueError("inconsistent vulnerability counts")
    if not isinstance(report["warnings"], dict) or any(not isinstance(value, list) for value in report["warnings"].values()):
        raise ValueError("invalid warnings report")
    return report


def audit(root: Path, binary: Path, output: Path) -> dict:
    output.mkdir(parents=True, exist_ok=False)
    receipt = {"schema_version": 1, "required_audit_version": AUDIT_VERSION, "required_installation_toolchain": AUDIT_TOOLCHAIN,
               "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(), "locks": [], "commands": [],
               "errors": [], "execution_complete": False, "advisory_complete": False,
               "yanked_complete": False, "passed": False}
    write_json(output / "receipt.json", receipt)
    before_source = before_locks = None
    try:
        before_source = source_identity(root, receipt["commands"])
        receipt["source_before"] = before_source
        before_locks, packages = read_locks(root)
        receipt["locks_before"] = before_locks
        binary = binary.resolve(strict=True)
        receipt["binary"] = {"path": str(binary), "sha256": sha256(binary.read_bytes())}
        with tempfile.TemporaryDirectory(prefix="audit-sandbox-", dir=output) as temporary:
            sandbox = Path(temporary)
            (sandbox / ".cargo").mkdir()
            (sandbox / ".cargo/audit.toml").write_text(CONFIG)
            receipt["config"] = {"text": CONFIG, "sha256": sha256(CONFIG.encode())}
            version = command([str(binary), "audit", "--version"], sandbox, 60)
            receipt["commands"].append(version)
            if version["exit_code"] != 0 or version["timed_out"] or not re.fullmatch(r"cargo-audit(?: audit)? " + re.escape(AUDIT_VERSION) + r"\s*", version["stdout"]):
                raise ValueError("cargo-audit version does not match the exact required release")
            for identity in before_locks:
                name = identity["path"]
                row = {"path": name, "complete": False, "passed": False, "errors": []}
                receipt["locks"].append(row)
                result = command([str(binary), "audit", "--file", str(root / name), "--db", str(output / "advisory-db"),
                                  "--url", ADVISORY_URL, "--json", "--deny", "warnings"], sandbox)
                receipt["commands"].append(result)
                row["command_index"] = len(receipt["commands"]) - 1
                prefix = "production" if name == LOCKFILES[0] else "fuzz"
                (output / f"{prefix}.stdout").write_bytes(base64.b64decode(result["stdout_base64"]))
                (output / f"{prefix}.stderr").write_bytes(base64.b64decode(result["stderr_base64"]))
                try:
                    count = len(tomllib.loads((root / name).read_text())["package"])
                    report = validate_report(result["stdout"], count, dt.datetime.now(dt.timezone.utc))
                    row["report"] = report
                    revision = git_value(output / "advisory-db", ["rev-parse", "HEAD"], receipt["commands"]).strip()
                    origin = git_value(output / "advisory-db", ["remote", "get-url", "origin"], receipt["commands"]).strip()
                    if revision != report["database"]["last-commit"] or origin.rstrip("/").removesuffix(".git") != ADVISORY_URL:
                        raise ValueError("advisory database does not match the reported official revision")
                    row["complete"] = not result["timed_out"]
                    row["passed"] = (row["complete"] and result["exit_code"] == 0 and not report["vulnerabilities"]["found"]
                                     and not any(report["warnings"].values()))
                    if not row["passed"]:
                        row["errors"].append("cargo-audit reported findings or failed")
                except Exception as error:
                    row["errors"].append(str(error))
                write_json(output / "receipt.json", receipt)
            receipt["registry"] = check_registry(packages, output)
            receipt["registry_requested_versions"] = len(packages)
            receipt["registry_completed_versions"] = sum(len(row.get("versions", [])) for row in receipt["registry"] if row["complete"])
            receipt["execution_complete"] = True
            receipt["advisory_complete"] = len(receipt["locks"]) == len(LOCKFILES) and all(row["complete"] for row in receipt["locks"])
            receipt["yanked_complete"] = all(row["complete"] for row in receipt["registry"])
            if sha256(binary.read_bytes()) != receipt["binary"]["sha256"]:
                raise ValueError("audit executable changed during execution")
            if (sandbox / ".cargo/audit.toml").read_bytes() != CONFIG.encode():
                raise ValueError("audit configuration changed during execution")
    except Exception as error:
        receipt["errors"].append(str(error))
    finally:
        try:
            receipt["source_after"] = source_identity(root, receipt["commands"])
            receipt["locks_after"], _ = read_locks(root)
            if before_source != receipt["source_after"] or before_locks != receipt["locks_after"]:
                receipt["errors"].append("source or lockfile bytes/mtime changed during audit")
        except Exception as error:
            receipt["errors"].append(f"final source identity failed: {error}")
        receipt["passed"] = (not receipt["errors"] and receipt["execution_complete"] and receipt["advisory_complete"]
                             and receipt["yanked_complete"] and all(row["passed"] for row in receipt["locks"])
                             and all(row["passed"] for row in receipt.get("registry", [])))
        receipt["finding_errors"] = [f"{row['path']}: {error}" for row in receipt["locks"] for error in row["errors"]]
        receipt["finding_errors"].extend(f"{row['name']}: {row['error']}" for row in receipt.get("registry", []) if "error" in row)
        receipt["completed_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        write_json(output / "receipt.json", receipt)
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cargo-audit", type=Path, default=os.environ.get("OPENUI_CARGO_AUDIT") or shutil.which("cargo-audit"))
    parser.add_argument("--output", type=Path, required=True, help="new evidence directory outside the source checkout")
    args = parser.parse_args()
    output = args.output.resolve()
    if output == ROOT or ROOT in output.parents:
        parser.error("audit evidence must be outside the source checkout")
    if args.cargo_audit is None:
        parser.error(f"install cargo-audit{AUDIT_VERSION} separately and pass --cargo-audit")
    result = audit(ROOT, args.cargo_audit, output)
    print(json.dumps({"passed": result["passed"], "receipt": str(output / "receipt.json"), "errors": result["errors"] + result["finding_errors"]}))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
