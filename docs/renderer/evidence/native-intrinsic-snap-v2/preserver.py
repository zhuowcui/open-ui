"""Preserve actual native guard and integrated-build results without qualification."""
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()

class Evidence:
    def __init__(self, name):
        self.out = ROOT / 'docs/renderer/evidence' / name
        self.index = ROOT / 'docs/renderer/generated' / (name + '.json')
        assert not self.out.exists() and not self.index.exists()
        self.out.mkdir()
        self.artifacts = []

    def keep(self, source, name, expected=None, compress=False):
        source = Path(source)
        original = source.read_bytes()
        digest = sha(original)
        if expected:
            assert digest == expected, str(source)
        target = self.out / name
        assert not target.exists()
        target.write_bytes(gzip.compress(original, compresslevel=9, mtime=0) if compress else original)
        row = dict(path=str(target.relative_to(ROOT)), sha256=sha(target.read_bytes()),
                   bytes=target.stat().st_size, source_path=str(source))
        if compress:
            assert gzip.decompress(target.read_bytes()) == original
            row.update(encoding='gzip', decompressed_sha256=digest, decompressed_bytes=len(original))
        self.artifacts.append(row)
        return json.loads(original) if source.suffix == '.json' else None

    def finish(self, fields):
        self.keep(Path(__file__), 'preserver.py')
        fields.update(schema_version=1, artifacts=self.artifacts, artifact_count=len(self.artifacts),
                      artifact_bytes=sum(a['bytes'] for a in self.artifacts),
                      javascript_executed_by_openui=False, public_native_rust_apis_required=True,
                      pixel_tolerance=0, new_release_states_admitted=0, release_qualification=False)
        self.index.write_text(json.dumps(fields, sort_keys=True, indent=2) + '\n')
        for a in self.artifacts:
            b = (ROOT / a['path']).read_bytes()
            assert sha(b) == a['sha256']
            if a.get('encoding') == 'gzip':
                assert sha(gzip.decompress(b)) == a['decompressed_sha256']
        print(json.dumps(dict(index=str(self.index.relative_to(ROOT)), sha256=sha(self.index.read_bytes()),
                              artifacts=len(self.artifacts), bytes=fields['artifact_bytes'])), flush=True)

intrinsic = Evidence('native-intrinsic-snap-v2')
owner = intrinsic.keep(RAW / 'native-intrinsic-snap-guard-pipeline-v1703/receipt.json',
                       'guard-owner-terminal.json', 'b20e0c9be6a6d1dd3777486a9886714fe7777a57747a286343f5722feb4ab3df')
assert owner['all_commands_terminal'] and owner['source'] == owner['source_after']
assert [r['observed_exit_code'] for r in owner['steps']] == [0]
guards = intrinsic.keep(RAW / 'native-intrinsic-snap-guards-v1702/receipt.json',
                        'guards-terminal.json', '4c761b86cd60dcc8c08fc30aa7ce8314d7af958e03c8411636f6a9363e36baee')
assert guards['all_commands_terminal'] and guards['source'] == guards['source_after']
assert guards['source']['commit'] == '727da10e580c9439f5db83d36bfc3e2340168207'
assert guards['baseline_regression_reproduced'] and guards['observed_exit_code'] == 0
assert [r['observed_exit_code'] for r in guards['steps']] == [0, 101, 0, 0, 0, 0, 0, 0, 0]
for r in guards['steps']:
    intrinsic.keep(RAW / 'native-intrinsic-snap-guards-v1702' / (r['name'] + '.log'),
                   r['name'] + '.log.gz', r['log_sha256'], compress=True)
for r in owner['steps']:
    intrinsic.keep(RAW / 'native-intrinsic-snap-guard-pipeline-v1703' / (r['name'] + '.log'),
                   'owner-' + r['name'] + '.log.gz', r['log_sha256'], compress=True)
prepared = intrinsic.keep(RAW / 'native-intrinsic-snap-guard-prepared-v1702.json', 'guard-prepared.json')
for path, digest in prepared['scripts'].items():
    intrinsic.keep(path, Path(path).name, digest)
intrinsic.keep('/tmp/openui-native-intrinsic-snap-guard-prepare-v1702.py', 'guard-prepare.py', prepared['probe_sha256'])
fault = intrinsic.keep(RAW / 'native-intrinsic-snap-guard-preflight-v1700.json', 'preflight-failed.json')
assert fault['python_syntax_error_before_execution'] and fault['cargo_commands_run'] == fault['screenshots_generated'] == 0
intrinsic.keep('/tmp/openui-native-intrinsic-snap-guard-prepare-v1700.py', 'preflight-failed.py', fault['probe_sha256'])
hosted = intrinsic.keep(RAW / 'native-intrinsic-snap-hosted-v1707/receipt.json', 'hosted-attempt1-terminal.json',
                       '9a25b3822b941d06527e73398463e44f650f9ed0da59b55a3cc7989fbb620d3f')
assert hosted['all_commands_terminal'] and hosted['job_conclusions'] == dict(success=5, cancelled=2)
intrinsic.keep(RAW / 'native-intrinsic-snap-hosted-v1707/attempt1.json', 'hosted-attempt1.json', hosted['metadata_sha256'])
intrinsic.keep('/tmp/openui-native-intrinsic-snap-hosted-v1707.py', 'hosted-observer.py', hosted['probe_sha256'])
for job in hosted['job_logs']:
    intrinsic.keep(job['path'], str(job['job_id']) + '.log.gz', job['sha256'], compress=True)
retry = intrinsic.keep(RAW / 'native-intrinsic-snap-hosted-retry-observed-v1713.json', 'hosted-attempt2-at-observation.json',
                       '9d65f9aeb88570fd38496173ba6d7a97ac9f5c38d62452612bfb42d2e95bdb3e')
assert retry['attempt'] == 2 and retry['headSha'] == hosted['source'] and retry['status'] == 'in_progress'
intrinsic.finish(dict(source=guards['source'], source_after=guards['source_after'],
                      baseline_actual_exit=101, fixed_guard_actual_exit=0, native_guard_stages=9,
                      public_rust_callback_five_scales_passed=True, native_conformance_scenarios=58,
                      named_inherited_style_guards=9, text_storage_updates=10000,
                      candidate_applied_to_umbrella=False, workspace_and_native_app_matrix_unexecuted=True,
                      full_renderer_matrices_unexecuted=True, hosted_attempt1_success=5,
                      hosted_attempt1_cancelled=2, hosted_attempt2_pending_at_observation=True,
                      prior_full_and_glyph_preparations_unlaunched_and_require_fresh_owner_preparation=True))

integration = Evidence('native-text-inheritance-v6')
head = '6def29f8ec6e10d87b5778df9dd1ed0157cf0cb4'
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == head
assert not subprocess.check_output(['git', 'diff', '41b616c3be5224b074e9d06ba6aa963731a1b265', head,
                                   '--', 'bindings/rust', 'tools/style', 'examples/c_v02', 'include', '.github'], cwd=ROOT)
owner = integration.keep(RAW / 'native-text-style-integration-pipeline-v1710/receipt.json', 'integration-owner-terminal.json',
                         'fa18ec3199b811bb091bd6d482f528113f979d5fd6bef585c56b5ef0ccc4fa28')
assert owner['all_commands_terminal'] and owner['source'] == owner['source_after']
assert [r['observed_exit_code'] for r in owner['steps']] == [0]
for r in owner['steps']:
    integration.keep(RAW / 'native-text-style-integration-pipeline-v1710' / (r['name'] + '.log'),
                     'owner-' + r['name'] + '.log.gz', r['log_sha256'], compress=True)
build = integration.keep(RAW / 'native-text-style-integration-clean-v1709/build.json', 'integration-build-terminal.json',
                         '480b05f6b842c4aebb77a82610edcea8f4e252a94603a0eb653ea327ce79bed2')
assert build['all_commands_terminal'] and build['source'] == build['source_after']
assert build['source']['clean'] and build['source']['commit'] == head
assert len(build['steps']) == 13 and all(r['observed_exit_code'] == 0 for r in build['steps'])
workspace = next(r for r in build['steps'] if r['name'] == 'workspace')
assert (workspace['passed'], workspace['failed'], workspace['ignored']) == (8551, 0, 13)
for r in build['steps']:
    integration.keep(RAW / 'native-text-style-integration-clean-v1709' / (r['name'] + '.log'),
                     'build-' + r['name'] + '.log.gz', r['log_sha256'], compress=True)
prepared = integration.keep(RAW / 'native-text-style-integration-prepared-v1709.json', 'integration-prepared.json',
                            '8ce41acde9c70e2616e724a4f472aa0b3fb2ad158bfb44e68cf3d4d64eb5df66')
for path, digest in prepared['scripts'].items():
    integration.keep(path, Path(path).name, digest)
integration.keep('/tmp/openui-native-text-style-integration-prepare-v1709.py', 'integration-prepare.py', prepared['probe_sha256'])
fact = integration.keep(RAW / 'native-text-style-integration-v1708.json', 'integration-fact.json',
                        'da0e255790993e3f85089b28c7b63fc023f8f6abe967899c5efa4ff36fb7580a')
integration.keep('/tmp/openui-native-text-style-integration-v1708.log', 'cherry-pick.log')
patch = Path('/tmp/openui-native-text-style-integrated-v1715.patch')
assert not patch.exists()
patch.write_bytes(subprocess.check_output(['git', 'diff', 'f873f5a7', head, '--', 'bindings/rust', 'tools/style', 'examples/c_v02'], cwd=ROOT))
integration.keep(patch, 'integrated-native-text-style.patch')
integration.finish(dict(source=build['source'], source_after=build['source_after'],
                        native_api_source_integrated_into_umbrella=True, integration_code_identical_to_tested_41=True,
                        generated_properties_regenerated_without_difference=True, clean_integration_build_passed=True,
                        clean_workspace_packages=18, build_stages=13, workspace_passed=8551,
                        workspace_failed=0, workspace_ignored=13, c_abi_exports=113, c_abi_layouts=30,
                        chromium_pixels_measured_on_clean_source_41_not_relabelled_as_6def=True,
                        all_48252_source41_comparison_rows_nine_invariants_unchanged=True,
                        full_original_exact=21334, full_original_total=22924, expanded_exact=22137,
                        expanded_total=23728, full_pixel_gates_still_failed=True,
                        native_app_exact_images=0, native_app_images=600,
                        native_geometry_exact=34560, native_geometry_total=38400))
