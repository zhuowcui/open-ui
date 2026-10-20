"""Failure-path guards; synthetic tool reports never qualify real dependencies."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.release import build_v02_linux as release
from tools.release import check_dependency_security as security


FAKE_TOOL = r'''import datetime as dt,json,os,subprocess,sys,tomllib
from pathlib import Path
sys.stdout.reconfigure(newline='\n')
case=os.environ.get('OPENUI_FAKE_AUDIT_CASE','success')
if '--version' in sys.argv:
 name='cargo-audit-audit' if sys.argv[1:2]==['audit'] else 'cargo-audit'
 print(name+' '+('0.0.0' if case=='version' else '0.22.2'));sys.exit(0)
args=sys.argv[1:]
lock=Path(args[args.index('--file')+1]);database=Path(args[args.index('--db')+1])
cfg=tomllib.loads(Path('.cargo/audit.toml').read_text())
(database.parent/'observed-config.json').write_text(json.dumps(cfg))
if not database.exists():
 database.mkdir()
 def git(*args):
  subprocess.run(['git','-c','commit.gpgsign=false','-c','core.hooksPath=/dev/null',*args],cwd=database,check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 git('init','-q');git('remote','add','origin','https://github.com/RustSec/advisory-db')
 (database/'sample').write_text('synthetic fixture, not real advisories')
 git('add','sample');git('-c','user.name=SecurityTest','-c','user.email=security@example.invalid','commit','-qm','fixture')
revision=subprocess.run(['git','rev-parse','HEAD'],cwd=database,check=True,capture_output=True,text=True).stdout.strip()
report={'database':{'advisory-count':1,'last-commit':revision,'last-updated':dt.datetime.now(dt.timezone.utc).isoformat()},
 'lockfile':{'dependency-count':len(tomllib.loads(lock.read_text())['package'])},
 'settings':{'ignore':[],'target_arch':[],'target_os':[],'severity':None,'informational_warnings':['unmaintained','unsound','notice']},
 'vulnerabilities':{'found':False,'count':0,'list':[]},'warnings':{}}
if case=='finding':report['vulnerabilities']={'found':True,'count':1,'list':[{'advisory':{'id':'RUSTSEC-TEST'}}]}
if case=='warning':report['warnings']={'unsound':[{'package':'fixture'}]}
if case=='ignored':report['settings']['ignore']=['RUSTSEC-TEST']
if case=='filter':report['settings']['target_os']=['linux']
if case=='severity':report['settings']['severity']='critical'
if case=='warning-coverage':report['settings']['informational_warnings']=[]
if case=='stale':report['database']['last-updated']='2000-01-01T00:00:00Z'
if case=='future':report['database']['last-updated']='9999-01-01T00:00:00Z'
if case=='missing-db':report['database']['last-commit']=None
if case=='count':report['lockfile']['dependency-count']+=1
if case=='lock-write':lock.write_text(lock.read_text()+'\n')
if case=='config-write':Path('.cargo/audit.toml').write_text('')
if case=='invalid-json':print('{');sys.exit(0)
if case=='duplicate-json':print('{"settings":{},'+json.dumps(report)[1:]);sys.exit(0)
print(json.dumps(report))
if case=='self-tail':print('synthetic tool advisory tail')
sys.exit(1 if case in ('status','finding','warning') else 0)
'''


class DependencySecurityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        # Resolve 8.3 short names (Windows) so recorded argv paths compare equal.
        self.base = Path(self.temporary.name).resolve()
        self.root = self.base / "source"
        self.root.mkdir()
        if os.name == "nt":
            # Windows cannot execute a shebang script; a batch wrapper runs it.
            script = self.base / "fixture-audit.py"
            script.write_text(FAKE_TOOL)
            self.binary = self.base / "fixture-audit.cmd"
            self.binary.write_text(f'@"{sys.executable}" "{script}" %*\n')
        else:
            self.binary = self.base / "fixture-audit"
            self.binary.write_text(f"#!{sys.executable}\n" + FAKE_TOOL)
            self.binary.chmod(0o755)
        self.package = {"name": "demo", "version": "1.0.0", "checksum": "a" * 64}
        for name in security.LOCKFILES:
            path = self.root / name
            path.parent.mkdir(parents=True)
            path.write_text('version = 4\n[[package]]\nname = "demo"\nversion = "1.0.0"\n'
                            f'source = "{security.REGISTRY_SOURCE}"\nchecksum = "{"a" * 64}"\n')
        (self.root / ".cargo").mkdir()
        (self.root / ".cargo/audit.toml").write_text('[advisories]\nignore=["RUSTSEC-TEST"]\n')
        for args in (["init", "-q"], ["add", "."], ["-c", "user.name=SecurityTest", "-c", "user.email=security@example.invalid", "commit", "-qm", "fixture"]):
            result = security.command(["git", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", *args], self.root)
            self.assertEqual(result["exit_code"], 0, result["stderr"])
        self.output_count = 0

    def registry_response(self, url: str) -> dict:
        row = {"name": "demo", "vers": "1.0.0", "cksum": "a" * 64, "yanked": False}
        return {"url": url, "final_url": url, "status": 200, "response_exceeds_bound": False,
                "headers": {}, "raw": json.dumps(row).encode() + b"\n"}

    def run_audit(self, case: str = "success", fetch=None) -> dict:
        self.output_count += 1
        self.output = self.base / f"evidence-{self.output_count}"
        with patch.dict(os.environ, {"OPENUI_FAKE_AUDIT_CASE": case}), patch.object(
            security, "fetch_registry", side_effect=fetch or self.registry_response
        ):
            result = security.audit(self.root, self.binary, self.output)
        self.assertTrue(all(row["reaped"] for row in result["commands"]))
        self.assertEqual(json.loads((self.output / "receipt.json").read_text()), result)
        return result

    def test_full_check_ignores_ambient_exemptions_and_retains_actual_inputs(self) -> None:
        result = self.run_audit()
        self.assertTrue(result["passed"])
        version = next(row for row in result["commands"] if "--version" in row["argv"])
        self.assertEqual(version["argv"], [str(self.binary), "--version"])
        self.assertEqual(version["stdout"], "cargo-audit 0.22.2\n")
        self.assertEqual(result["locks_before"], result["locks_after"])
        self.assertEqual(result["source_before"], result["source_after"])
        config = json.loads((self.output / "observed-config.json").read_text())
        self.assertEqual(config["advisories"]["ignore"], [])
        self.assertEqual(config["output"]["deny"], ["warnings"])
        self.assertTrue(config["database"]["fetch"])
        self.assertFalse(config["database"]["stale"])
        self.assertEqual(result["registry_completed_versions"], 1)
        self.assertTrue((self.output / "production.stdout").is_file())
        self.assertTrue((self.output / "fuzz.stdout").is_file())

    def test_findings_and_nonzero_tool_status_remain_failures(self) -> None:
        for case in ("finding", "warning", "status"):
            with self.subTest(case=case):
                result = self.run_audit(case)
                self.assertTrue(result["advisory_complete"])
                self.assertFalse(result["passed"])
                self.assertTrue(all(not row["passed"] for row in result["locks"]))

    def test_filtered_partial_and_invalid_reports_never_qualify(self) -> None:
        for case in ("ignored", "filter", "severity", "warning-coverage", "stale", "future", "missing-db", "count", "invalid-json", "duplicate-json", "self-tail"):
            with self.subTest(case=case):
                result = self.run_audit(case)
                self.assertFalse(result["passed"])
                self.assertFalse(result["advisory_complete"])
                self.assertEqual(len(result["locks"]), 2)

    def test_version_mismatch_stops_before_any_registry_or_lock_audit(self) -> None:
        def unexpected(_):
            self.fail("invalid tool must not start registry requests")
        result = self.run_audit("version", unexpected)
        self.assertFalse(result["passed"])
        self.assertFalse(result["execution_complete"])
        self.assertEqual(result["locks"], [])

    def test_missing_lockfile_does_not_generate_or_replace_it(self) -> None:
        path = self.root / security.LOCKFILES[1]
        path.unlink()
        result = self.run_audit()
        self.assertFalse(result["passed"])
        self.assertFalse(path.exists())
        self.assertEqual(result["locks"], [])

    def test_lockfile_or_config_mutation_prevents_success(self) -> None:
        result = self.run_audit("config-write")
        self.assertFalse(result["passed"])
        self.assertTrue(any("configuration changed" in error for error in result["errors"]))
        result = self.run_audit("lock-write")
        self.assertFalse(result["passed"])
        self.assertNotEqual(result["locks_before"], result["locks_after"])

    def test_missing_registry_check_cannot_hide_behind_successful_tool(self) -> None:
        def unavailable(_):
            raise OSError("synthetic registry unavailable")
        result = self.run_audit(fetch=unavailable)
        self.assertTrue(all(row["passed"] for row in result["locks"]))
        self.assertFalse(result["yanked_complete"])
        self.assertFalse(result["passed"])

    def test_yanked_checksum_missing_duplicate_and_invalid_metadata_fail(self) -> None:
        for case in ("yanked", "checksum", "missing", "duplicate", "nonboolean", "malformed", "status", "redirect", "bound"):
            def response(url, case=case):
                item = self.registry_response(url)
                row = json.loads(item["raw"])
                if case == "yanked": row["yanked"] = True
                if case == "checksum": row["cksum"] = "b" * 64
                if case == "missing": row["vers"] = "2.0.0"
                if case == "nonboolean": row["yanked"] = 0
                raw = json.dumps(row).encode() + b"\n"
                if case == "duplicate": raw += raw
                if case == "malformed": raw = b"{"
                if case == "status": item["status"] = 503
                if case == "redirect": item["final_url"] = "https://example.invalid"
                if case == "bound": item["response_exceeds_bound"] = True
                item["raw"] = raw
                return item
            with self.subTest(case=case):
                result = self.run_audit(fetch=response)
                self.assertFalse(result["passed"])
                self.assertEqual(result["yanked_complete"], case == "yanked")

    def test_unknown_registry_and_lock_symlink_are_unqualified(self) -> None:
        path = self.root / security.LOCKFILES[0]
        original = path.read_text()
        path.write_text(original.replace(security.REGISTRY_SOURCE, "registry+https://example.invalid/index"))
        self.assertFalse(self.run_audit()["passed"])
        path.unlink()
        path.symlink_to(self.root / security.LOCKFILES[1])
        self.assertFalse(self.run_audit()["passed"])

    def test_failed_audit_stops_real_builder_before_any_artifact_even_with_local_flags(self) -> None:
        artifact = self.base / "artifacts"
        with patch.object(release, "verify_source", return_value={}), patch.object(
            release, "verify_dependency_security", side_effect=subprocess.CalledProcessError(1, ["audit"])
        ), patch.object(release, "build_libraries") as build, patch.object(
            sys, "argv", ["build", "--allow-dirty", "--library-dir", str(self.base), "--output", str(artifact)]
        ):
            with self.assertRaises(subprocess.CalledProcessError):
                release.main()
            build.assert_not_called()
        self.assertFalse(artifact.exists())

    def test_offline_source_check_remains_separate(self) -> None:
        with patch.object(release, "verify_source", return_value={}), patch.object(
            release, "verify_dependency_security"
        ) as check, patch.object(sys, "argv", ["build", "--verify-source"]):
            release.main()
            check.assert_not_called()

    def test_command_timeout_reaps_owned_process(self) -> None:
        result = security.command([sys.executable, "-c", "import time; time.sleep(30)"], self.base, timeout=0.02)
        self.assertTrue(result["timed_out"])
        self.assertTrue(result["reaped"])
        self.assertNotEqual(result["exit_code"], 0)
        if Path("/proc").is_dir():
            self.assertFalse(Path(f"/proc/{result['pid']}").exists())


if __name__ == "__main__":
    unittest.main()
