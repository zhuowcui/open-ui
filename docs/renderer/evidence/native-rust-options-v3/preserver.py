"""Freeze every measured umbrella job and local source check, keeping source identity explicit."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1';OUT=ROOT/'docs/renderer/evidence/native-rust-options-v3';INDEX=ROOT/'docs/renderer/generated/native-rust-options-v3.json';assert not OUT.exists() and not INDEX.exists();OUT.mkdir();rows=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 data=p.read_bytes();digest=hashlib.sha256(data).hexdigest()
 if expected:assert digest==expected
 target=OUT/name;target.parent.mkdir(parents=True,exist_ok=True);assert not target.exists();target.write_bytes(gzip.compress(data,mtime=0) if compress else data)
 row=dict(path=str(target.relative_to(ROOT)),sha256=sha(target),bytes=target.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(target.read_bytes())==data;row.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(data))
 rows.append(row);return json.loads(data) if p.suffix=='.json' else None
head='16187f4f54e1b62ea317a155f620c42f91a38c72'
checks_path=RAW/'native-rust-integrated-current-checks-v1818/receipt.json';checks=keep(checks_path,'checks.json');assert checks['source']==checks['source_after'] and checks['source']['clean'] and checks['source']['commit']==head and len(checks['checks'])==15 and all(r['observed_exit_code']==0 for r in checks['checks'])
for c in checks['checks']:keep(checks_path.parent/(c['name']+'.log'),'check-'+c['name']+'.log.gz',c['log_sha256'],True)
hostpath=RAW/'native-rust-integrated-hosted-v1819/receipt.json';host=keep(hostpath,'hosted.json','bb7840eac8c458ceb02f5792dc1365154deede8073e39e9cbc3f7b1d5612f3e1');assert host['source']==head and host['all_commands_terminal'] and (host['successful_jobs'],host['skipped_jobs'],host['failed_jobs'])==(13,5,0) and host['all_six_native_guards_executed_and_passed_in_hosted_parity']
for w in host['workflows']:
 d=keep(Path(w['path']),'hosted-'+str(w['run_id'])+'.json',w['sha256']);assert d['headSha']==head and d['status']=='completed' and d['conclusion']=='success'
for j in host['job_logs']:
 if 'path' in j:assert j['capture_actual_exit']==0;keep(Path(j['path']),'hosted-'+str(j['job_id'])+'.log.gz',j['sha256'],True)
for name,receipt,digest in [('checks','/tmp/openui-native-rust-integrated-current-checks-v1818.py',checks['probe_sha256']),('collector','/tmp/openui-native-rust-integrated-hosted-collect-v1819.py',host['probe_sha256'])]:keep(Path(receipt),'probes/'+name+'.py',digest)
watchpath=RAW/'native-rust-integrated-hosted-watch-v1823/receipt.json';watch=keep(watchpath,'watch-terminal.json');assert watch['all_commands_terminal'] and watch['all_hosted_workflows_terminal'] and watch['collector_observed_exit_code']==0
for number,snapshot in enumerate(watch['snapshots']):
 for w in snapshot:keep(Path(w['path']),'watch/'+str(number)+'-'+str(w['run_id'])+'.json',w['sha256'])
keep(Path('/tmp/openui-native-rust-integrated-hosted-watch-v1823.py'),'probes/watch.py',watch['probe_sha256'])
keep(watchpath.parent/'collector.log','collector.log.gz',compress=True);keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=checks['source'],source_after=checks['source_after'],all_commands_terminal=True,all_15_read_only_checks_passed=True,all_four_hosted_workflows_complete=True,successful_hosted_jobs=13,skipped_hosted_jobs=5,failed_hosted_jobs=0,full_hardening_passed=7,all_six_native_api_and_ffi_guards_executed_and_passed_in_hosted_parity=True,native_guard_names=list(host['native_guard_log_observations']),skips_are_not_release_passes=True,own_complete_renderer_matrices_unexecuted=True,all_configuration_field_rendering_effects_unqualified=True,release_qualification=False,javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'artifacts':len(rows),'sha256':sha(INDEX)}))
