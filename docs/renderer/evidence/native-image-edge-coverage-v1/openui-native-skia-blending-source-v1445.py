import base64
import hashlib
import json
import urllib.request
from pathlib import Path

RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-skia-blending-source-v1445'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
files = ['src/core/SkColorPriv.h', 'src/core/SkBlitter_ARGB32.cpp',
         'src/opts/SkRasterPipeline_opts.h', 'src/core/SkRasterPipelineBlitter.cpp']
report = dict(schema_version=1, chromium_tag='147.0.7727.50', skia_commit='abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0', release_qualification=False,
    all_commands_terminal=False, cargo_commands_run=0, screenshots_generated=0,
    canvas_pixels_generated=0, files=[])
save = lambda: (OUT / 'receipt.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
for source in files:
    url = f'https://skia.googlesource.com/skia/+/abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0/{source}?format=TEXT'
    row = dict(source=source, url=url)
    try:
        with urllib.request.urlopen(url, timeout=20) as response:
            data = base64.b64decode(response.read(), validate=True)
        destination = OUT / Path(source).name
        assert not destination.exists()
        destination.write_bytes(data)
        row.update(path=str(destination), bytes=len(data), sha256=hashlib.sha256(data).hexdigest(), fetched=True)
    except Exception as error:
        row.update(fetched=False, error=str(error))
    report['files'].append(row)
    save()
    print(json.dumps({k:v for k,v in row.items() if k != 'url'}), flush=True)
report['all_commands_terminal'] = True
report['all_requested_files_fetched'] = all(row['fetched'] for row in report['files'])
save()
raise SystemExit(int(not report['all_requested_files_fetched']))
