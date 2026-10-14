"""Record process absence without rewriting incomplete owner receipts."""
import hashlib,json,os
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def missing(pid):
 try:os.kill(pid,0);return False
 except ProcessLookupError:return True
owners=[]
for name in ['native-text-loader-retry-pipeline-v1656','native-glyph-guard-pipeline-v1658']:
 p=RAW/name/'receipt.json';d=json.loads(p.read_bytes());assert not d['all_commands_terminal']
 pids=[d['owner_pid']]
 if 'current_process' in d:pids.append(d['current_process']['pid'])
 assert all(missing(pid) for pid in pids)
 owners.append(dict(name=name,receipt=str(p),receipt_sha256=sha(p),state_at_interruption=d['state'],pids_verified_missing=pids,
 source=d['source'],original_receipt_terminal=False,all_observed_owner_processes_absent=True,actual_whole_exit_unknown=True,qualification=False))
for p in Path('/proc').iterdir():
 if not p.name.isdecimal():continue
 try:
  program=Path(os.readlink(p/'exe')).name
  assert program not in ['cargo','pixel_compare','native_text_content','text_content-c','text_content-cpp'], 'owned build or native app still live'
  if program.startswith('python'):
   argv=(p/'cmdline').read_bytes().split(b'\0');name=Path(os.fsdecode(argv[1])).name if len(argv)>1 else ''
   assert not name.startswith(('openui-native-text-loader-retry','openui-native-glyph-guard-pipeline')), 'old verification process still live'
 except (FileNotFoundError,ProcessLookupError,PermissionError):pass
consumer=RAW/'native-text-loader-retry-consumer-v1655/receipt.json';d=json.loads(consumer.read_bytes());assert not d['all_commands_terminal']
images=[i for c in d['cases'] for i in c.get('images',[])];phases=[v for i in images for v in i['phases']];cross=[i for i in images if i['language']!='rust']
report=dict(schema_version=1,observed_at_utc=datetime.now(timezone.utc).isoformat(),owners=owners,
 all_owned_build_raster_and_app_processes_absent=True,all_original_receipts_unchanged=True,
 consumer=dict(receipt=str(consumer),sha256=sha(consumer),recorded_images=len(images),exact_images=sum(i['analysis']['mismatched_pixels']==0 for i in images),
 recorded_geometry=len(phases),exact_geometry=sum(v['geometry_exact'] for v in phases),cross_language_images=len(cross),
 cross_language_images_matching_rust=sum(i['rust_pixels_equal'] for i in cross),rust_self_rows_excluded=True,partial_results_are_not_qualification=True),
 release_qualification=False,failed_or_incomplete_gates_remain_open=True,probe_sha256=sha(Path(__file__)))
p=RAW/'owner-interruption-witness-v1664.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(witness=str(p),sha256=sha(p),owners_verified_absent=len(owners),partial_images=len(images),qualified=False)),flush=True)
