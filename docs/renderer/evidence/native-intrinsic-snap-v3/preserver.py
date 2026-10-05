"""Retain the fresh full-width preparation as unexecuted evidence."""
import hashlib
import json
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-intrinsic-snap-v3'
INDEX = ROOT / 'docs/renderer/generated/native-intrinsic-snap-v3.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
artifacts = []
def keep(p, name, digest=None):
    p = Path(p)
    if digest:
        assert sha(p) == digest
    q = OUT / name
    assert not q.exists()
    q.write_bytes(p.read_bytes())
    artifacts.append(dict(path=str(q.relative_to(ROOT)), sha256=sha(q), bytes=q.stat().st_size,
                          source_path=str(p)))

p = RAW / 'native-intrinsic-snap-full-prepared-v1716.json'
keep(p, 'full-prepared.json', '5ce60d8837ec162cbbc2917888788a62223ed7353666f42b0668a54590772f04')
d = json.loads(p.read_bytes())
assert d['native_owner_not_started'] and d['prior_whole_owners'] == 42
assert not (RAW / 'native-intrinsic-snap-pipeline-v1717').exists()
keep('/tmp/openui-native-intrinsic-snap-full-prepare-v1716.py', 'full-prepare.py', d['probe_sha256'])
for path, digest in d['scripts'].items():
    keep(path, Path(path).name, digest)
keep(__file__, 'preserver.py')
report = dict(schema_version=1, source=d['source'], source_unchanged=True,
              fresh_full_owner_prepared_but_not_executed=True, prior_whole_owners=42,
              immutable_actual_source_identical_guards_reused=True,
              all_prior_local_native_owners_terminal=True, whole_owner_lock_preserves_gaps=True,
              required_native_images=600, required_geometry_states=38400, required_renderer_matrices=4,
              chromium_reference_inputs_unchanged=True, candidate_applied_to_umbrella=False,
              javascript_executed_by_openui=False, public_native_rust_apis_required=True,
              release_qualification=False, pixel_tolerance=0, new_release_states_admitted=0,
              artifacts=artifacts, artifact_count=len(artifacts), artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
for a in artifacts:
    assert sha(ROOT / a['path']) == a['sha256']
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)), sha256=sha(INDEX), artifacts=len(artifacts),
                      bytes=report['artifact_bytes'])), flush=True)
