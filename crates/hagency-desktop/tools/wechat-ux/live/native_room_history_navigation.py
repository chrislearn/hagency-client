#!/usr/bin/env python3
"""Focused desktop navigation, mobile draft, and original-photo checks."""
import json, os, time
from pathlib import Path
from native_probe import NativeApp
from PIL import Image, ImageChops, ImageStat


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS='1', MAKEPAD_NO_FOCUS='1')
    os.environ.pop('MAKEPAD_FOCUS',None)
    root=Path('lab/wechat-ux/evidence/live/room-history')
    seed=json.loads((root/'seed.json').read_text())
    result={'passed':False,'checks':[],'runs':[]}
    app=None
    def start(size):
        nonlocal app
        app=NativeApp(root,port=8299,size=size); app.start(); result['runs'].append(str(app.output))
        app.wait_text('Emma Wilson',timeout=90)
    def text_click(text):
        row=[r for r in app.ocr() if text in r['text']][-1]
        x,y,w,h=row['box']; width,height=app.size; app.click((x+w/2)*width,(y+h/2)*height)
    def room_click(button=0):
        app.wait_text(seed['room_name'],timeout=60)
        row=next(w for w in app.snap() if w.get('t')==seed['room_name']); x,y,w,h=row['r']; x,y=x+w/2,y+h/2
        if button:
            app.request('/m',k='down',x=x,y=y,b=button,wait=1); app.request('/m',k='up',x=x,y=y,b=button,wait=1)
        else: app.click(x,y)
    def query():
        app.click_id('history_query'); app.request('/t',t='History needle',wait=1)
        app.click_id('search_history'); app.wait_text('All available history checked.',timeout=90)
        app.wait_text('1 result'); text_click('History needle'); app.wait_text('Message Details',pixels=True)
    def passed(name): result['checks'].append(name); print('PASS',name,flush=True)
    try:
        start((1050,800)); room_click(1)
        app.wait_text('Search Chat History',pixels=True); app.click_id('search_history_button')
        app.wait_text('Search Chat History',pixels=True); query()
        app.click_id('view_in_chat'); app.wait_text('History needle',pixels=True,timeout=60)
        app.capture('desktop-room-search-jump'); passed('desktop_room_context_search_and_jump')
        app.stop(); start((375,812)); room_click()
        app.wait_text('Message (unencrypted)',pixels=True,timeout=60)
        text_click('Message (unencrypted)'); app.request('/t',t='History keeps my draft',wait=1)
        app.click(350,54); app.wait_text('Chat Info',pixels=True); text_click('Search Chat History')
        query(); app.click_id('view_in_chat'); app.wait_text('History needle',pixels=True,timeout=60)
        app.wait_text('History keeps my draft',pixels=True)
        app.capture('search-preserves-draft'); passed('mobile_search_jump_preserves_composer_draft')
        text_click('History keeps my draft'); app.request('/k',c='A',cmd=1,wait=1); app.request('/k',c='Backspace',wait=1)
        app.click(350,54); app.wait_text('Chat Info',pixels=True); text_click('Shared Attachments')
        app.wait_text('Room photo',pixels=True,timeout=60); text_click('Room photo'); app.wait_text('Message Details',pixels=True)
        # Compare visible preview pixels with the original mock fixture; no
        # native image is fabricated or substituted into the app capture.
        original=Image.open('/tmp/robrix-wechat-reference/resources/img/post2.jpg').convert('RGB').resize((48,30))
        deadline=time.monotonic()+30
        while True:
            path=app.capture('original-photo-preview')
            screen=Image.open(path).convert('RGB')
            best=min(sum(ImageStat.Stat(ImageChops.difference(screen.crop((32,y,718,y+419)).resize((48,30)),original)).mean)/3 for y in range(380,480,2))
            if best<12: break
            if time.monotonic()>deadline: raise AssertionError('Original photo was not visible')
            time.sleep(.5)
        result['photo_mean_rgb_error']=best
        passed('original_photo_preview_matches_source_pixels')
        app.request('/k',c='Escape',wait=1); app.wait_text('Shared Attachments',pixels=True)
        app.request('/k',c='Escape',wait=1); app.wait_text('Chat Info',pixels=True)
        passed('escape_returns_from_detail_to_results_then_chat_info')
        result['passed']=True
    finally:
        if app and app.process and app.process.poll() is None and not result['passed']: app.capture('navigation-failure')
        if app: app.stop()
        result['native_errors']=sum(any(m in line for m in ['[E]','panicked at','Assertion failed:']) for run in result['runs'] for line in (Path(run)/'native.log').read_text(errors='replace').splitlines())
        result['passed']=result['passed'] and result['native_errors']==0
        (root/'native-room-history-navigation.json').write_text(json.dumps(result,indent=2)); print(json.dumps(result),flush=True)
    assert result['passed']


if __name__=='__main__': main()
