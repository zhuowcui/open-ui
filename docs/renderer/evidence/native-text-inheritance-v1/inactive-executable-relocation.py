"""Preserve inactive old executables on the evidence volume, at the same paths."""
import hashlib
import json
import os
import shutil
from pathlib import Path

RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b/retired-artifact-relocation-v1681')
assert not STORE.exists()
assert shutil.disk_usage(STORE.parent).free > 10 * 2**30
sha = lambda p: hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()
names = ['native-scroll-inline-clean-v386', 'generated-tile-clean-v150', 'scroll-opaque-clean-v173']
rows = []
for name in names:
    receipt = RAW / name / 'build.json'
    d = json.loads(receipt.read_bytes())
    assert len(d['steps']) == {'native-scroll-inline-clean-v386': 8, 'generated-tile-clean-v150': 3, 'scroll-opaque-clean-v173': 3}[name] and all(x['observed_exit_code'] == 0 for x in d['steps'])
    assert d['source'] == d['source_after'] and d['source']['clean']
    assert d['build_identity']['source'] == d['source']
    build_row = next(x for x in d['steps'] if x['name'] == 'pixel-build')
    artifact = Path(build_row['binary'])
    assert artifact == RAW / name / 'pixel_compare' and not artifact.is_symlink()
    assert sha(artifact) == build_row['binary_sha256']
    users = []
    for pid in Path('/proc').iterdir():
        if not pid.name.isdecimal():
            continue
        try:
            if (pid / 'exe').resolve() == artifact.resolve():
                users.append(int(pid.name))
        except (OSError, PermissionError):
            pass
    assert not users
    rows.append({'original': str(artifact), 'bytes': artifact.stat().st_size,
                 'sha256': build_row['binary_sha256'], 'build_receipt_sha256': sha(receipt),
                 'source': d['source']['commit'], 'all_declared_build_steps_passed': True, 'build_steps': len(d['steps']),
                 'active_executable_pids': users})
before = shutil.disk_usage('/home/nero/code/open-ui').free
STORE.mkdir()
for row in rows:
    original = Path(row['original'])
    destination = STORE / (original.parent.name + '-pixel_compare')
    assert not destination.exists()
    shutil.copy2(original, destination)
    assert sha(original) == sha(destination) == row['sha256']
    with destination.open('rb') as stream:
        os.fsync(stream.fileno())
    link = original.with_name(original.name + '.relocated-v1681')
    assert not link.exists() and not link.is_symlink()
    link.symlink_to(destination)
    os.replace(link, original)
    assert original.is_symlink() and sha(original) == row['sha256']
    row['stored_path'] = str(destination)
    row['original_path_preserved_as_symlink'] = True
report = {'schema_version': 1, 'all_commands_terminal': True, 'probe_sha256': sha(Path(__file__)),
          'source_files_changed': False, 'old_executable_bytes_preserved': True,
          'oracle_or_reference_images_changed': False, 'active_sources_or_probes_changed': False,
          'cargo_or_raster_commands_run': 0, 'release_qualification': False,
          'root_free_bytes_before': before,
          'root_free_bytes_after': shutil.disk_usage('/home/nero/code/open-ui').free,
          'artifacts': rows}
receipt = RAW / 'retired-artifact-relocation-v1681.json'
assert not receipt.exists()
receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({'receipt': str(receipt), 'sha256': sha(receipt),
                  'preserved_bytes': sum(x['bytes'] for x in rows),
                  'root_free_bytes_before': report['root_free_bytes_before'],
                  'root_free_bytes_after': report['root_free_bytes_after']}), flush=True)
