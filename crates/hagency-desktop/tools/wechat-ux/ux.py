#!/usr/bin/env python3
"""WeChat UX reference inventory and evidence gate, not a Rinx renderer.

All commands are local/read-only except explicitly named output files. Capture
reads the existing Makepad localhost bridge; it never clicks or sends messages.
The gate validates evidence supplied by a native runner and a visual reviewer.
It cannot establish the authenticity of an external review or backend log.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
PROJECT = ROOT / 'lab/wechat-ux'
CRITERIA = ('layout', 'typography', 'colors', 'icons', 'density', 'states')
LOCALES = ('en', 'cn')


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        stream.write(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + '\n')


def local(root, name):
    if not isinstance(name, str) or not name or Path(name).is_absolute():
        raise ValueError('Evidence path must be relative to its receipt')
    path = (root / name).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError('Evidence path escapes its receipt directory')
    return path


def source_fingerprint(root=ROOT):
    paths = [root / n for n in ('Cargo.toml', 'Cargo.lock', 'build.rs')]
    for name in ('src', 'resources', '.cargo'):
        paths.extend(p for p in (root / name).rglob('*') if p.is_file())
    files = {str(p.relative_to(root)): sha(p) for p in sorted(paths) if p.is_file()}
    return hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()


def validate(project=PROJECT, root=ROOT):
    catalog = read(project / 'flow-catalog.json')
    api = read(project / 'api-map.json')
    manifest = read(project / 'image-to-appcard-flow.json')
    errors = []
    for group, rows in (('screens', catalog['screens']), ('journeys', catalog['journeys']),
                        ('operations', api['operations'])):
        ids = [r['id'] for r in rows]
        if len(set(ids)) != len(ids):
            errors.append('Duplicate ' + group + ' IDs')
    operations = {o['id']: o for o in api['operations']}
    screens = {s['id'] for s in catalog['screens']}
    for row in catalog['screens'] + catalog['journeys']:
        for op in row['operations']:
            if op not in operations:
                errors.append(row['id'] + ': unknown operation ' + op)
        for state in row.get('states', []):
            if state not in screens:
                errors.append(row['id'] + ': unknown screen ' + state)
    for op in operations.values():
        if op['source']:
            path = local(root, op['source'])
            if not path.is_file() or op['symbol'].split('::')[-1] not in path.read_text():
                errors.append(op['id'] + ': source symbol missing; review API map')
    if {s['id'] for s in manifest['scenes']} != {s['id'] for s in catalog['screens'] if s['tier'] == 'core'}:
        errors.append('Atlas scenes must cover exactly the core storyboard')
    return {'valid': not errors, 'errors': errors, 'screens': len(screens),
            'journeys': len(catalog['journeys']), 'operations': len(operations),
            'capabilities': {status: [o['id'] for o in operations.values() if o['status'] == status]
                             for status in ('existing', 'partial', 'missing', 'planned')},
            'scope': 'Source-symbol validation only; no runtime or visual acceptance.'}


def evidence_file(base, record):
    if not isinstance(record, dict):
        raise ValueError('Missing evidence file record')
    path = local(base, record.get('path'))
    if not path.is_file() or record.get('sha256') != sha(path):
        raise ValueError('Missing or changed evidence: ' + str(record.get('path')))
    return path


def gate(receipt_path, project=PROJECT, current_source=None):
    """Full catalog gate. Missing rows, unsupported capabilities and stale data fail."""
    from PIL import Image
    catalog = read(project / 'flow-catalog.json')
    api = read(project / 'api-map.json')
    current_source = current_source or source_fingerprint()
    receipt_path = Path(receipt_path)
    errors = []
    scores = []
    if not catalog.get('screens') or not catalog.get('journeys'):
        return {'accepted': False, 'score': None, 'errors': ['Acceptance requires nonempty screen and journey catalogs']}
    if not receipt_path.is_file():
        return {'accepted': False, 'score': None, 'errors': ['No native/review evidence receipt: ' + str(receipt_path)],
                'required_screens': len(catalog['screens']) * len(LOCALES),
                'required_journeys': len(catalog['journeys']) * len(LOCALES)}
    receipt = read(receipt_path)
    base = receipt_path.parent
    if type(receipt.get('schema_version')) is not int:
        errors.append('schema_version must be an integer')
    for key, expected in (('schema_version', 1), ('catalog_sha256', sha(project / 'flow-catalog.json')),
                          ('api_map_sha256', sha(project / 'api-map.json')),
                          ('source_sha256', current_source), ('fixture', catalog['fixture']['id'])):
        if receipt.get(key) != expected:
            errors.append('Missing/stale receipt field: ' + key)
    claim = receipt.get('claim')
    if claim not in ('generated-target', 'wechat'):
        errors.append('Claim must be generated-target or wechat')
    mode = receipt.get('execution_mode')
    if mode not in ('fixture', 'live'):
        errors.append('Execution mode must explicitly be fixture or live')
    unsupported = [o['id'] for o in api['operations'] if o['status'] != 'existing']
    if unsupported and mode == 'live':
        errors.append('Unimplemented or semantically incomplete capabilities: ' + ', '.join(unsupported))
    expected_screens = {(s['id'], lang) for s in catalog['screens'] for lang in LOCALES}
    expected_journeys = {(j['id'], lang) for j in catalog['journeys'] for lang in LOCALES}
    seen_screens, seen_journeys = set(), set()
    for entry in receipt.get('screens', []):
        key = (entry.get('screen'), entry.get('locale'))
        label = '/'.join(str(v) for v in key)
        if key in seen_screens or key not in expected_screens:
            errors.append(label + ': duplicate or unknown screen/locale')
        seen_screens.add(key)
        try:
            reference = evidence_file(base, entry.get('reference'))
            native = evidence_file(base, entry.get('native'))
            inspection = read(evidence_file(base, entry.get('inspection')))
            # Hash binding prevents a review for a different pair being recycled.
            ref_hash, native_hash = sha(reference), sha(native)
            ref = entry['reference']
            if ref_hash == native_hash:
                raise ValueError('Reference reused as native screenshot')
            if ref.get('kind') not in ('generated_target', 'wechat_capture', 'makepad_wechat_capture'):
                raise ValueError('Unknown reference origin')
            if claim == 'wechat' and ref.get('kind') != 'wechat_capture':
                raise ValueError('Actual WeChat claim requires a real WeChat capture')
            if ref.get('locale') != entry['locale']:
                raise ValueError('Reference locale mismatch')
            if ref['kind'] == 'wechat_capture' and not all(ref.get(k) for k in ('app_version', 'device', 'os_version')):
                raise ValueError('WeChat version/device/OS provenance is required')
            with Image.open(reference) as ref_image, Image.open(native) as native_image:
                if ref_image.size != native_image.size or list(native_image.size) != entry.get('pixels'):
                    raise ValueError('Screenshot dimensions differ; no implicit resizing allowed')
            review = entry.get('review', {})
            score = review.get('design_match')
            if type(score) not in (int, float) or not math.isfinite(score) or not 9 <= score <= 10:
                raise ValueError('Every screen needs a finite visual score >= 9/10')
            if not review.get('reviewer', '').strip() or not review.get('notes', '').strip():
                raise ValueError('Named reviewer and written comparison required')
            if review.get('reference_sha256') != ref_hash or review.get('native_sha256') != native_hash:
                raise ValueError('Visual review refers to a different image pair')
            if review.get('verdict') != 'pass' or not all(review.get('criteria', {}).get(k) is True for k in CRITERIA):
                raise ValueError('Incomplete or failed visual criteria')
            if inspection.get('source_sha256') != current_source or inspection.get('native_sha256') != native_hash:
                raise ValueError('Native inspection is stale or refers to another screenshot')
            if inspection.get('screen') != key[0] or inspection.get('locale') != key[1]:
                raise ValueError('Native inspection scene/locale mismatch')
            if inspection.get('input_method') != 'native' or not inspection.get('run_id'):
                raise ValueError('Native input and run identity required')
            for artifact in ('snapshot', 'tree', 'layout', 'input_trace', 'build_receipt'):
                evidence_file(base, inspection.get(artifact))
            build = read(evidence_file(base, inspection['build_receipt']))
            if build.get('source_sha256') != current_source or build.get('run_id') != inspection['run_id']:
                raise ValueError('Build/run receipt mismatch')
            if not re.fullmatch('[0-9a-f]{64}', str(build.get('binary_sha256', ''))):
                raise ValueError('Missing binary hash')
            checks = inspection.get('checks', {})
            required = ('native_controls', 'no_screen_raster', 'no_clipping', 'text_legible', 'enabled_state', 'hit_targets')
            if not all(checks.get(k) is True for k in required):
                raise ValueError('Native structure or interaction check failed/missing')
            scores.append(float(score))
        except (ValueError, KeyError, TypeError, OSError) as error:
            errors.append(label + ': ' + str(error))
    for entry in receipt.get('journeys', []):
        key = (entry.get('journey'), entry.get('locale'))
        label = '/'.join(str(v) for v in key)
        if key in seen_journeys or key not in expected_journeys:
            errors.append(label + ': duplicate or unknown journey/locale')
        seen_journeys.add(key)
        try:
            trace = read(evidence_file(base, entry.get('evidence')))
            if trace.get('source_sha256') != current_source or trace.get('journey') != key[0] or trace.get('locale') != key[1]:
                raise ValueError('Journey trace identity/source mismatch')
            if trace.get('input_method') != 'native' or trace.get('mock_backend') is not (mode == 'fixture'):
                raise ValueError('Native input and explicit matching fixture/live backend required')
            definition = next(j for j in catalog['journeys'] if j['id'] == key[0])
            if trace.get('visited_states') != definition['states']:
                raise ValueError('Journey omitted or reordered required states')
            for name in ('input_trace', 'assertion_log', 'build_receipt'):
                evidence_file(base, trace.get(name))
            build = read(evidence_file(base, trace['build_receipt']))
            if not trace.get('run_id') or build.get('run_id') != trace['run_id'] or build.get('source_sha256') != current_source:
                raise ValueError('Journey build/run mismatch')
            # Backend proof is required even for operations routed through a UI modal.
            pure_ui = {'local_navigation', 'filter_chats', 'open_chat', 'view_media', 'settings'}
            if set(definition['operations']) - pure_ui:
                evidence_file(base, trace.get('backend_log'))
            if not all(trace.get('checks', {}).get(k) is True for k in
                       ('expected_outcome', 'back_state', 'error_paths', 'no_duplicate_effects')):
                raise ValueError('Journey assertions incomplete or failed')
        except (ValueError, KeyError, TypeError, OSError, StopIteration) as error:
            errors.append(label + ': ' + str(error))
    missing_screens = sorted('/'.join(k) for k in expected_screens - seen_screens)
    missing_journeys = sorted('/'.join(k) for k in expected_journeys - seen_journeys)
    if missing_screens:
        errors.append('Missing screen reviews: ' + ', '.join(missing_screens))
    if missing_journeys:
        errors.append('Missing journey evidence: ' + ', '.join(missing_journeys))
    return {'accepted': not errors, 'claim': claim, 'execution_mode': mode,
            'production_ready': not errors and mode == 'live',
            'remaining_capabilities': unsupported,
            'score': min(scores) if scores and not errors else None,
            'screen_reviews': len(scores), 'required_screens': len(expected_screens),
            'required_journeys': len(expected_journeys), 'errors': errors,
            'note': 'Score is the minimum reviewed visual score; behavior must pass independently. '
                    'This validates evidence completeness/integrity, not the honesty of external reviewers.'}


def capture(args):
    """Read-only baseline; intentionally insufficient for visual acceptance."""
    from PIL import Image
    from io import BytesIO
    parsed = urllib.parse.urlparse(args.bridge)
    if parsed.scheme != 'http' or parsed.hostname not in ('127.0.0.1', 'localhost', '::1') or parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise ValueError('Use an HTTP localhost Makepad bridge without credentials')
    base = args.bridge.rstrip('/')
    def get(path):
        with urllib.request.urlopen(base + path, timeout=30) as response:
            return response.read()
    status = json.loads(get('/s'))
    if not any(w.get('i') == args.window for w in status.get('w', [])):
        raise ValueError('Requested window missing from bridge status')
    snapshot = json.loads(get('/snap?all=1&w=' + str(args.window)))
    png = get('/g?raw=1&w=' + str(args.window))
    with Image.open(BytesIO(png)) as im:
        pixels = list(im.size)
        im.verify()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / 'native.png').write_bytes(png)
    write_new(out / 'snapshot.json', snapshot)
    write_new(out / 'bridge-status.json', status)
    record = {'schema_version': 1, 'capture_id': str(uuid.uuid4()),
              'captured_at': datetime.now(timezone.utc).isoformat(),
              'screen': args.screen, 'locale': args.locale, 'pixels': pixels,
              'source_sha256': source_fingerprint(), 'declared_binary_sha256': sha(args.binary),
              'process_binding': 'declared, not verified by this read-only capture',
              'status': 'baseline_only_not_acceptance',
              'files': {name: sha(out / name) for name in ('native.png', 'snapshot.json', 'bridge-status.json')}}
    write_new(out / 'capture.json', record)
    return record


def report(output, project=PROJECT):
    catalog = read(project / 'flow-catalog.json')
    api = read(project / 'api-map.json')
    manifest = read(project / 'image-to-appcard-flow.json')
    mocks = read(project / 'source/mock-reference-index.json') if (project / 'source/mock-reference-index.json').is_file() else {}
    payload = json.dumps({'catalog': catalog, 'api': api, 'manifest': manifest, 'mocks': mocks}, ensure_ascii=False).replace('<', '\\u003c')
    template = (ROOT / 'tools/wechat-ux/report.html').read_text()
    # Inline data works through file:// without a server or cross-origin fetch.
    result = template.replace('__UX_DATA__', payload)
    with Path(output).open('x') as stream:
        stream.write(result)
    return {'report': str(output), 'scope': 'Interactive specification; not a functioning Rinx client.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('validate')
    sub.add_parser('fingerprint')
    plan = sub.add_parser('appcard-plan')
    plan.add_argument('--appcard', type=Path, required=True)
    plan.add_argument('--stages', default='intake,prepare')
    check = sub.add_parser('gate')
    check.add_argument('--evidence', type=Path, default=PROJECT / 'evidence/acceptance.json')
    check.add_argument('--output', type=Path)
    cap = sub.add_parser('capture')
    cap.add_argument('--bridge', default='http://127.0.0.1:8099')
    cap.add_argument('--binary', type=Path, required=True)
    cap.add_argument('--screen', choices=[s['id'] for s in read(PROJECT / 'flow-catalog.json')['screens']], required=True)
    cap.add_argument('--locale', choices=LOCALES, required=True)
    cap.add_argument('--window', type=int, default=0)
    cap.add_argument('--output', type=Path, required=True)
    rep = sub.add_parser('report')
    rep.add_argument('--output', type=Path, default=PROJECT / 'index.html')
    args = parser.parse_args()
    try:
        if args.command == 'validate':
            value = validate()
        elif args.command == 'fingerprint':
            value = {'source_sha256': source_fingerprint(), 'catalog_sha256': sha(PROJECT / 'flow-catalog.json'),
                     'api_map_sha256': sha(PROJECT / 'api-map.json')}
        elif args.command == 'gate':
            value = gate(args.evidence)
            if args.output:
                write_new(args.output, value)
        elif args.command == 'capture':
            value = capture(args)
        elif args.command == 'report':
            value = report(args.output)
        else:
            return subprocess.run([sys.executable, str(args.appcard.resolve() / 'lab/image-to-appcard-flow/flow.py'),
                                   'plan', '--project', str(PROJECT), '--manifest', str(PROJECT / 'image-to-appcard-flow.json'),
                                   '--stages', args.stages], check=False).returncode
        print(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False))
        return 1 if value.get('accepted') is False or value.get('valid') is False else 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print('Error: ' + str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
