#!/usr/bin/env python3
"""Encrypted article/media round trip using native UI and disposable Palpo users."""
import json,os,time,uuid,shutil
from pathlib import Path
from urllib.parse import quote
from native_probe import NativeApp
from seed import checked


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS='1',MAKEPAD_NO_FOCUS='1');os.environ.pop('MAKEPAD_FOCUS',None)
    fixture=json.loads(Path('lab/wechat-ux/evidence/live/fixture.json').read_text())
    assert all(u['user_id'].startswith('@robrix_ux_') for u in fixture['users'].values())
    root=Path('lab/wechat-ux/evidence/live/article-editor-v2')/('encrypted-'+uuid.uuid4().hex);root.mkdir(parents=True,mode=0o700)
    name='Article encryption '+str(int(time.time())%10000)
    room=checked(fixture['url'],'POST','createRoom',{'name':name,'preset':'private_chat','invite':[fixture['users']['emma']['user_id']],'initial_state':[{'type':'m.room.encryption','state_key':'','content':{'algorithm':'m.megolm.v1.aes-sha2'}}]},token=fixture['users']['alex']['access_token'])['room_id']
    checked(fixture['url'],'POST','join/'+quote(room,safe=''),{},token=fixture['users']['emma']['access_token'])
    report={'passed':False,'checks':[],'room':room};apps=[]
    def mark(name): report['checks'].append(name);print('PASS '+name,flush=True)
    def start(alias,port):
        path=root/alias;path.mkdir(mode=0o700);data=json.loads(json.dumps(fixture));data['users']['alex']=fixture['users'][alias];(path/'fixture.json').write_text(json.dumps(data));os.chmod(path/'fixture.json',0o600);(path/'profile').mkdir();(path/'profile/ui-language.json').write_text('"zh-CN"');app=NativeApp(path,port,size=(430,820));apps.append(app);app.start();app.wait_text('全部聊天',timeout=90);return app
    def tap(app,widget):
        for _ in range(6):
            if any(w['i']==widget for w in app.snap()): return app.click_id(widget)
            app.request('/m',k='scroll',x=210,y=440,dy=360,wait=1);time.sleep(.25)
        raise AssertionError('Control is not reachable by scrolling: '+widget)
    def fill(app,id,text):
        app.click_id(id);app.request('/k',c='A',cmd=1,wait=1);app.request('/k',c='Backspace',wait=1);app.request('/t',t=text,wait=1)
    try:
        sender=start('alex',8361);recipient=start('emma',8362)
        sender.click_id('discover_tab');sender.click_id('discover_article');sender.click_id('article_continue');sender.click_id('article_allow');sender.click_id('article_new')
        fill(sender,'article_title','加密山谷');sender.click_id('rich');sender.request('/t',t='只有这段会话的成员可以阅读图片与文字。',wait=1);sender.click_id('article_save');time.sleep(.5)
        file=next((sender.root/'profile').glob('mini-apps/**/library-v2.json'));lib=json.loads(file.read_text());asset=json.loads(Path('lab/article-editor-v2/artwork/mountains.json').read_text());lib['assets'][asset['id']]=asset;(file.parent/'assets').mkdir();shutil.copyfile('lab/article-editor-v2/artwork/mountains.png',file.parent/'assets'/asset['id']);os.chmod(file.parent/'assets'/asset['id'],0o600);file.write_text(json.dumps(lib));os.chmod(file,0o600)
        sender.click_id('article_images');sender.click_text('晨光里的山谷');sender.click_id('article_preview');sender.click_id('preview_check');tap(sender,'review_continue');fill(sender,'article_chat_search',name);sender.click_id('name');tap(sender,'article_confirm');sender.wait_text('已发布',timeout=75);sender.capture('encrypted-publication')
        lib=json.loads(file.read_text());publication=lib['publications'][0];remote=publication['assets'][asset['id']];assert 'file' in remote['source'] and 'url' not in remote['source']
        raw=checked(fixture['url'],'GET',f"rooms/{quote(room,safe='')}/event/{quote(publication['root'],safe='')}",token=fixture['users']['emma']['access_token']);assert raw['type']=='m.room.encrypted' and 'org.octosense.article' not in raw.get('content',{});mark('encrypted_room_uses_encrypted_event_and_encrypted_image_descriptor')
        recipient.wait_text(name,timeout=45);recipient.click_text(name);recipient.wait_text('加密山谷',pixels=True,timeout=60);recipient.click_text('加密山谷');recipient.wait_text('只有这段会话',pixels=True,timeout=60);time.sleep(3);recipient.capture('encrypted-reader');assert not any('integrity' in w.get('t','') or 'Unable to download' in w.get('t','') for w in recipient.snap())
        mark('recipient_decrypts_and_opens_native_article')
        tap(sender,'publication_withdraw');tap(sender,'withdraw_confirm');sender.wait_text('已撤回',timeout=60);mark('encrypted_article_withdrawal')
        report['passed']=True
    finally:
        for app in apps:
            if not report['passed'] and app.process and app.process.poll() is None:
                try:app.capture('failure');(app.output/'failure-widgets.json').write_text(json.dumps(app.request('/snap'),ensure_ascii=False))
                except Exception:pass
            app.stop()
        (root/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':report['passed'],'evidence':str(root),'checks':report['checks']}),flush=True)

if __name__=='__main__':main()
