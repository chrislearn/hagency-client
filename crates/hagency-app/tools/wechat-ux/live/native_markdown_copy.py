#!/usr/bin/env python3
"""Verify native Markdown widget selection/copy without using the OS clipboard."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import uuid
from native_probe import NativeApp

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,default=Path('target/debug/examples/article_native_markdown'))
    args=parser.parse_args()
    root=Path('target/article-native-copy')/uuid.uuid4().hex
    root.mkdir(parents=True)
    with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
    app=NativeApp(root,port,auto_login=False)
    app.output.mkdir(parents=True)
    app.log=(app.output/'native.log').open('w')
    app.process=subprocess.Popen([str(args.binary.resolve())],stdin=subprocess.DEVNULL,stdout=app.log,stderr=subprocess.STDOUT,env=dict(os.environ,MAKEPAD_REMOTE=str(port),MAKEPAD_HIDE_WINDOWS='1',MAKEPAD_NO_FOCUS='1'))
    report={'passed':False,'checks':[],'binary_sha256':hashlib.sha256(args.binary.read_bytes()).hexdigest()}
    try:
        for _ in range(120):
            assert app.process.poll() is None
            try:
                if app.request('/s')['pid']==app.process.pid:break
            except OSError:pass
            time.sleep(.25)
        app.wait_text('Ready')
        app.capture('native-markdown')
        body=next(w for w in app.snap() if w['i']=='native_body' and w['ty']=='Html')
        x,y,w,h=body['r'];app.click(x+5,y+10)
        app.request('/k',c='A',cmd=1,wait=1)
        app.capture('native-selected')
        app.click_id('inspect')
        state=json.loads(next(w['t'] for w in app.snap() if w['i']=='result'))
        (root/'selection.json').write_text(json.dumps(state,ensure_ascii=False,indent=2))
        assert state['copy']==state['text'],state
        for text in ['Alpha 中文 bold','😃 and','E=mc^2','fn main()', '你好','Left\tRight','234','Final paragraph.', 'Andrew->China: Says Hello', 'China thinks\\nabout it', 'A[中文] --> B[Complete]']:
            assert text in state['copy'],(text,state)
        assert '<rcode>' not in state['copy'] and 'e4b8ade69687' not in state['copy'],state
        report['checks'].append('native_select_all_copy_includes_unicode_emoji_math_code_cells_and_diagram_source')
        assert 'panicked at' not in (app.output/'native.log').read_text(errors='replace')
        report['passed']=True
    finally:
        app.stop();(root/'result.json').write_text(json.dumps(report,indent=2));print(root/'result.json')

if __name__=='__main__':main()
