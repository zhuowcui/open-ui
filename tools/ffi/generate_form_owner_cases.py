#!/usr/bin/env python3
"""Generate native C operation data from the Rust app's JSON fixture."""
import argparse
import os
import shutil
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
INPUT=ROOT/'tools/qualification/reproducers/native-form-owner-cases.json'
OUTPUT=ROOT/'examples/c_v02/form_owner_cases.h'
LABELS=['body','wrap','fa','fb','parking','blocker','newform','a','b','c','d','e']
KINDS={'attribute':'FORM_ATTRIBUTE','remove_attribute':'FORM_REMOVE_ATTRIBUTE','detach':'FORM_DETACH','append':'FORM_APPEND','insert':'FORM_INSERT','checked':'FORM_CHECKED'}
def output():
    fixture=json.loads(INPUT.read_bytes())
    assert fixture['controls']==['radio','checkbox']
    lines=['/* Generated native operation data; contains no expected browser values. */',
           '#ifndef OPENUI_FORM_OWNER_CASES_H_', '#define OPENUI_FORM_OWNER_CASES_H_',
           'enum FormOperationKind { FORM_ATTRIBUTE, FORM_REMOVE_ATTRIBUTE, FORM_DETACH, FORM_APPEND, FORM_INSERT, FORM_CHECKED };',
           'typedef struct FormOperation { int kind, target, parent, before, checked; const char *name, *value, *json; } FormOperation;',
           'typedef struct FormCase { const char *name; const FormOperation *pre, *steps; size_t pre_count, step_count; } FormCase;']
    rows=[]
    for i,case in enumerate(fixture['cases']):
        for phase in ['pre','steps']:
            ops=case[phase]
            if not ops:continue
            lines.append(f'static const FormOperation form_case_{i}_{phase}[] = {{')
            for op in ops:
                kind=KINDS[op['op']];target=LABELS.index(op['target']);parent=LABELS.index(op['parent']) if 'parent' in op else -1;before=LABELS.index(op['before']) if 'before' in op else -1
                checked=int(op.get('value') is True)
                name=json.dumps(op['name']) if 'name' in op else 'NULL'
                value=json.dumps(op['value']) if isinstance(op.get('value'),str) else 'NULL'
                literal=json.dumps(json.dumps(op,sort_keys=True,separators=(',',':')))
                lines.append(f'  {{ {kind}, {target}, {parent}, {before}, {checked}, {name}, {value}, {literal} }},')
            lines.append('};')
        pre=f'form_case_{i}_pre' if case['pre'] else 'NULL'
        rows.append(f'  {{ {json.dumps(case["name"])}, {pre}, form_case_{i}_steps, {len(case["pre"])}, {len(case["steps"])} }},')
    lines+=['static const FormCase form_cases[] = {',*rows,'};','#endif','']
    # The fixture header follows the checked-in C formatter.
    import subprocess
    formatter=os.environ.get('OPENUI_CLANG_FORMAT') or shutil.which('clang-format-18') or shutil.which('clang-format')
    if not formatter:raise SystemExit('clang-format 18 is required to verify native C fixture data')
    if 'version 18.' not in subprocess.check_output([formatter,'--version'],text=True):raise SystemExit('native C fixture generation requires clang-format 18')
    return subprocess.check_output([formatter,'--assume-filename='+str(OUTPUT)],input='\n'.join(lines).encode(),cwd=ROOT)
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=output()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes()!=data:raise SystemExit('Native form owner C fixture data is stale')
    else:OUTPUT.write_bytes(data)
if __name__=='__main__':main()
