#!/usr/bin/env python3
"""Verify the second, local-only native article host with no Matrix profile."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import uuid
from native_probe import NativeApp


def main():
    root = Path('lab/wechat-ux/evidence/live/article-components') / uuid.uuid4().hex
    root.mkdir(parents=True, mode=0o700)
    binary = Path('crates/article-makepad/target/debug/examples/standalone').resolve()
    report = {'passed': False, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'checks': []}
    app = None

    def launch():
        app = NativeApp(root, 8366, auto_login=False)
        with socket.socket() as probe:
            if probe.connect_ex(('127.0.0.1', app.port)) == 0:
                raise RuntimeError('Standalone test port is already occupied')
        app.output.mkdir(parents=True, mode=0o700)
        app.log = (app.output / 'native.log').open('w')
        env = dict(os.environ, MAKEPAD_REMOTE=str(app.port), MAKEPAD_HIDE_WINDOWS='1', MAKEPAD_NO_FOCUS='1')
        env.pop('MAKEPAD_FOCUS', None)
        app.process = subprocess.Popen([str(binary), '--data-dir=' + str((root/'data').resolve())], env=env,
                                       stdout=app.log, stderr=subprocess.STDOUT)
        for _ in range(100):
            if app.process.poll() is not None:
                raise RuntimeError('Standalone editor exited; see private native log')
            try:
                if app.request('/s')['pid'] == app.process.pid:
                    app.wait_text('Loaded / 已打开')
                    return app
            except OSError:
                pass
            time.sleep(.2)
        raise RuntimeError('Standalone editor did not start')

    try:
        app = launch()
        app.click_id('title')
        app.request('/k', c='A', cmd=1, wait=1)
        app.request('/t', t='独立运行 · Shared editor', wait=1)
        rich = min((w for w in app.snap() if w['i'] == 'rich'), key=lambda w: w['r'][1])
        x, y, width, height = rich['r']
        app.click(x + width / 2, y + height / 2)
        app.request('/k', c='A', cmd=1, wait=1)
        app.request('/t', t='中文 English — local host only.', wait=1)
        app.request('/k', c='A', cmd=1, wait=1)
        app.click_id('bold')
        app.click_id('theme')
        app.click_id('save')
        app.wait_text('Saved locally / 已本地保存')
        path = next((root/'data').glob('mini-apps/**/library-v2.json'))
        doc = json.loads(path.read_text())['documents'][0]
        assert doc['title'] == '独立运行 · Shared editor'
        assert doc['theme'] == 'paper'
        assert doc['blocks'][0]['text'] == '中文 English — local host only.'
        assert any(mark['bold'] for mark in doc['blocks'][0]['marks'])
        app.capture('standalone-editor')
        report['checks'].append('native_chinese_english_selection_bold_theme_save_without_matrix_or_octosense')
        app.stop()
        app = launch()
        assert next(w for w in app.snap() if w['i'] == 'title').get('val') == doc['title']
        app.capture('standalone-reopened')
        report['checks'].append('separate_process_reopens_existing_schema2_draft')
        report['passed'] = True
        public = Path('lab/article-components/evidence')
        public.mkdir(parents=True, exist_ok=True)
        for name in ['standalone-editor.png', 'standalone-reopened.png']:
            shutil.copyfile(root/name, public/name)
    finally:
        if app:
            if not report['passed']:
                try:
                    app.capture('failure')
                except Exception:
                    pass
            app.stop()
        (root/'result.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
        print(json.dumps(dict(report, evidence=str(root)), ensure_ascii=False), flush=True)


if __name__ == '__main__':
    main()
