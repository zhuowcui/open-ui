"""Prepare a guarded native-only verification before the longer raster qualification."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-intrinsic-snap-guard-727da10e');BRANCH='agent/native-intrinsic-snap-guard-v1700';FIXED='727da10e580c9439f5db83d36bfc3e2340168207'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old_owner=Path('/tmp/openui-native-intrinsic-snap-pipeline-v1697.py');text=old_owner.read_text();c=ast.literal_eval(ast.parse(text).body[0].value)
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
old=Path('/tmp/openui-native-intrinsic-snap-guards-v1696.py');s=old.read_text().replace(c['root'],str(ROOT)).replace('agent/native-intrinsic-snap-qualification-v1696',BRANCH).replace('native-intrinsic-snap-guards-v1696','native-intrinsic-snap-guards-v1700')
guard=Path('/tmp/openui-native-intrinsic-snap-guards-v1700.py');assert not guard.exists();ast.parse(s);guard.write_text(s)
c.update(root=str(ROOT),name='native-intrinsic-snap-guard-pipeline-v1701',scripts=[guard.name],
 selections=['native-intrinsic-snap-source-v1692.json','native-intrinsic-snap-checks-v1693/receipt.json','owner-interruption-witness-v1664.json'],
 stages=[('guards',['/usr/bin/python3',str(guard)],'native-intrinsic-snap-guards-v1700/receipt.json',True)])
body=text.split('\n',1)[1].replace("baseline_commit='e995e52f8650ff9286d6beb0618cdd4b75df789b', expected_stages=7","baseline_commit='e995e52f8650ff9286d6beb0618cdd4b75df789b', expected_stages=1")
body=body.replace("old_queue_unlaunched=True, direct_c_cpp_link_outputs_kept_without_self_copy=True)","old_queue_unlaunched=True, direct_c_cpp_link_outputs_kept_without_self_copy=True, native_guard_only=True, raster_commands_run=0)")
owner=Path('/tmp/openui-native-intrinsic-snap-guard-pipeline-v1701.py');assert not owner.exists();s='CONFIG = '+repr(c)+'\n'+body;ast.parse(s);owner.write_text(s)
assert not (RAW/'native-intrinsic-snap-pipeline-v1697').exists() and not (RAW/'native-glyph-raster-pipeline-v1690').exists()
report=dict(schema_version=1,source=source,root=str(ROOT),branch=BRANCH,prior_whole_owners=40,whole_owner_lock_preserves_gaps=True,
 baseline_restore_branch_preflighted=True,full_owner_v1697_not_started=True,full_and_glyph_queues_require_fresh_preparation_after_this_guard,
 native_owner_not_started=True,native_guard_only=True,required_matrices_after_guard=4,required_native_images_after_guard=600,
 scripts={str(p):sha(p) for p in [guard,owner]},originals={str(p):sha(p) for p in [old,old_owner]},
 release_qualification=False,javascript_executed_by_openui=False,public_native_rust_apis_required=True,probe_sha256=sha(Path(__file__)))
p=RAW/'native-intrinsic-snap-guard-prepared-v1700.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(prepared=True,source=FIXED,owner_not_started=True,receipt_sha256=sha(p))),flush=True)
