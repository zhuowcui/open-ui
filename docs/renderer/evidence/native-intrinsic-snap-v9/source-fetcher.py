"""Fetch exact-renderer-tag text measurement sources; never qualify pixels."""
import base64,hashlib,json,urllib.request,urllib.error
from pathlib import Path
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-width-pinned-source-v1782';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
report=dict(schema_version=1,chromium_tag='147.0.7727.50',files=[],all_commands_terminal=False,release_qualification=False,cargo_commands_run=0,raster_commands_run=0,screenshots_generated=0,probe_sha256=sha(Path(__file__)))
for name in ['shape_result.h','harfbuzz_face.cc']:
 url='https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/third_party/blink/renderer/platform/fonts/shaping/'+name+'?format=TEXT'
 try:
  with urllib.request.urlopen(url,timeout=15) as r:raw=r.read();status=r.status
  data=base64.b64decode(raw,validate=True);p=OUT/name;p.write_bytes(data)
  report['files'].append(dict(path=str(p),url=url,http_status=status,bytes=len(data),sha256=sha(p),fetched=True))
 except Exception as e:
  report['files'].append(dict(url=url,fetched=False,error_type=type(e).__name__,error=str(e)))
report.update(all_commands_terminal=True,all_requested_files_fetched=all(r['fetched'] for r in report['files']))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'receipt':str(p),'sha256':sha(p),'fetched':sum(r['fetched'] for r in report['files']),'requested':2}),flush=True)
raise SystemExit(int(not report['all_requested_files_fetched']))
