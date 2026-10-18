"""Compare reviewed upstream raster files with the already pinned Skia source."""
import base64
import hashlib
import json
import urllib.request
from pathlib import Path

RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-skia-coverage-match-v1448'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
LOCAL = Path('/home/nero/.cargo/git/checkouts/rust-skia-b4d79b6a888cdb4c/a31b86b/skia-bindings/skia')
COMMIT = 'abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
report = dict(schema_version=1, chromium_tag='147.0.7727.50', upstream_skia_commit=COMMIT,
    rust_skia_commit='a31b86ba3b767344d39af3b8c30043003d8fc991',
    compared_files=[], whole_source_equivalence_claimed=False, pin_changed=False,
    cargo_commands_run=0, screenshots_generated=0, release_qualification=False,
    all_commands_terminal=False, probe_sha256=sha(Path(__file__)))
save = lambda: (OUT / 'receipt.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
try:
    prior = json.loads((RAW / 'native-skia-blending-source-v1445/receipt.json').read_bytes())
    assert prior['all_commands_terminal'] and prior['all_requested_files_fetched'] and prior['skia_commit']==COMMIT
    rows = list(prior['files'])
    source = 'src/core/SkColorData.h'
    url = f'https://skia.googlesource.com/skia/+/{COMMIT}/{source}?format=TEXT'
    with urllib.request.urlopen(url, timeout=20) as response:
        data = base64.b64decode(response.read(), validate=True)
    destination = OUT / Path(source).name
    destination.write_bytes(data)
    rows.append(dict(source=source, url=url, path=str(destination), sha256=sha(destination), bytes=len(data), fetched=True))
    for row in rows:
        upstream = Path(row['path'])
        local = LOCAL / row['source']
        assert sha(upstream)==row['sha256']
        local_sha = sha(local)
        assert local_sha==row['sha256'], row['source']
        report['compared_files'].append(dict(source=row['source'], official_url=row['url'],
            upstream_sha256=row['sha256'], local_sha256=local_sha, byte_identical=True))
        save()
    report['all_compared_files_byte_identical'] = True
except Exception as error:
    report.update(all_compared_files_byte_identical=False, error=str(error))
    raise
finally:
    report['all_commands_terminal']=True
    save()
print(json.dumps({'compared_files':len(report['compared_files']), 'all_byte_identical':True, 'pin_changed':False, 'whole_source_equivalence_claimed':False}), flush=True)
