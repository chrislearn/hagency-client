#!/usr/bin/env python3
"""Verify hash-bound design mappings and real native fixture receipts.

This checks implementation and behavior, not image similarity. No visual score
is emitted. The native screenshot remains available for human comparison.
"""
import argparse, hashlib, json
from pathlib import Path
HERE=Path(__file__).resolve().parent
REPO=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
parser=argparse.ArgumentParser();parser.add_argument('--evidence',type=Path);parser.add_argument('--output',type=Path)
args=parser.parse_args()
manifest=json.loads((HERE/'image-to-appcard-flow.json').read_text())
checks=[]
for scene in manifest['scenes']:
    directory=HERE/scene['directory']; mapping=json.loads((directory/'semantic-map.json').read_text())
    assert mapping['reference_sha256']==sha(directory/'reference.png')
    assert mapping['contract_sha256']==sha(directory/'contract.json')
    for element in mapping['elements']:
        source=(REPO/element['source']).read_text()
        assert element['id'] in source, element['id']
        assert element['native_widget'] not in ('WebView','Image'), element['id']
    checks.append(scene['id'])
source=(REPO/'src/article_app/ui.rs').read_text()
assert ' = Html {' in source and 'octoscript' in (REPO/'src/article_app/model.rs').read_text()
assert all(name not in source for name in ['system_browser','WebView','robius_open'])
result={'adapter':'robrix-native-article-v1','mapping_checks':checks,'visual_acceptance':False,'similarity_score':None,
        'inputs':{str(p.relative_to(REPO)):sha(p) for p in [REPO/'Cargo.lock',REPO/'resources/mini_apps/article-editor/app.card',REPO/'src/article_app/model.rs',REPO/'src/article_app/ui.rs']}}
if args.evidence:
    report=json.loads((args.evidence/'result.json').read_text());assert report['passed']
    result['native_checks']=report['checks'];result['native_evidence']=str(args.evidence)
    captures={}
    for png in sorted(args.evidence.glob('*/native-runs/*/*.png')):
        captures[str(png.relative_to(args.evidence))]=sha(png)
    assert len(captures)>=10
    traces=[json.loads(p.read_text()) for p in args.evidence.glob('*/native-runs/*/trace.json')]
    assert len(traces)>=3
    assert all(any(e.get('native_input') for e in trace) for trace in traces)
    result['captures']=captures
    result['native_binary_hashes']=sorted({e['binary_sha256'] for trace in traces for e in trace if 'binary_sha256' in e})
    result['behavior_passed']=True
else:result['behavior_passed']=None
if args.output:args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'mappings':len(checks),'behavior_passed':result['behavior_passed'],'visual_acceptance':False}))
