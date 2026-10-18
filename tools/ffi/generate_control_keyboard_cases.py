#!/usr/bin/env python3
"""Generate native C keyboard operations, without scripts or expected states."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
ROOT=Path(__file__).resolve().parents[2]
INPUT=ROOT/'tools/qualification/reproducers/native-control-keyboard-cases.json'
OUTPUT=ROOT/'examples/c_v02/control_keyboard_cases.h'
LABELS=['body','fa','fb','a','b','c']
EVENTS={x:y for x,y in [('keydown','OUI_EVENT_KEY_DOWN'),('keyup','OUI_EVENT_KEY_UP'),('click','OUI_EVENT_CLICK'),('focus','OUI_EVENT_FOCUS'),('blur','OUI_EVENT_BLUR'),('focusin','OUI_EVENT_FOCUS_IN'),('focusout','OUI_EVENT_FOCUS_OUT')]}
KINDS={'attribute':'KEY_ATTRIBUTE','checked':'KEY_CHECKED','focus':'KEY_FOCUS','detach':'KEY_DETACH','style':'KEY_STYLE','key':'KEY_DISPATCH'}
def output():
 fixture=json.loads(INPUT.read_bytes())
 lines=['/* Generated native operations; no expected states or script interpreter. */','#ifndef OPENUI_CONTROL_KEYBOARD_CASES_H_','#define OPENUI_CONTROL_KEYBOARD_CASES_H_','#include <stddef.h>','#include "openui.h"',
 'enum KeyboardOperationKind { KEY_ATTRIBUTE, KEY_CHECKED, KEY_FOCUS, KEY_DETACH, KEY_STYLE, KEY_DISPATCH };',
 'typedef struct KeyboardOperation { int kind, target, checked, down, code, modifiers; const char *name, *value, *json; } KeyboardOperation;',
 'typedef struct KeyboardCase { const char *name, *control; const KeyboardOperation *pre, *steps, *handler_ops; size_t pre_count, step_count, handler_count; unsigned handler_event; int handler_target, prevent; } KeyboardCase;']
 entries=[]
 for i,case in enumerate(fixture['scenarios']):
  handler=case['handler'];phases={'pre':case['pre'],'steps':case['steps'],'handler':handler['operations'] if handler else []}
  for phase,ops in phases.items():
   if not ops:continue
   lines.append(f'static const KeyboardOperation keyboard_{i}_{phase}[] = {{')
   for op in ops:
    target=LABELS.index(op['target']) if 'target' in op else -1
    name=op.get('name') if op['op']!='key' else op['key']
    value=op.get('value') if isinstance(op.get('value'),str) else None
    literal=json.dumps(json.dumps(op,sort_keys=True,separators=(',',':')))
    lines.append('  { '+', '.join([KINDS[op['op']],str(target),str(int(op.get('value') is True)),str(int(op.get('phase')=='down')),str(op.get('code',0)),str(op.get('modifiers',0)),json.dumps(name) if name is not None else 'NULL',json.dumps(value) if value is not None else 'NULL',literal])+' },')
   lines.append('};')
  arrays=[f'keyboard_{i}_{phase}' if ops else 'NULL' for phase,ops in phases.items()]
  entries.append('  { '+', '.join([json.dumps(case['name']),json.dumps(case['kind']),*arrays,*[str(len(ops)) for ops in phases.values()],EVENTS[handler['event']] if handler else '0',str(LABELS.index(handler['target']) if handler else -1),str(int(handler['prevent']) if handler else 0)])+' },')
 lines+=['static const KeyboardCase keyboard_cases[] = {',*entries,'};','#endif','']
 formatter=os.environ.get('OPENUI_CLANG_FORMAT') or shutil.which('clang-format-18') or shutil.which('clang-format')
 if not formatter or 'version 18.' not in subprocess.check_output([formatter,'--version'],text=True):raise SystemExit('clang-format 18 is required')
 return subprocess.check_output([formatter,'--assume-filename='+str(OUTPUT)],input='\n'.join(lines).encode(),cwd=ROOT)
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args();data=output()
 if args.check:
  if not OUTPUT.exists() or OUTPUT.read_bytes()!=data:raise SystemExit('Native keyboard C operations are stale')
 else:OUTPUT.write_bytes(data)
if __name__=='__main__':main()
