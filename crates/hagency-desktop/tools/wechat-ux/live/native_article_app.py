#!/usr/bin/env python3
"""Native L0 article editor: consent, sanitized Html, two-account Matrix sharing.

Uses hidden Makepad windows, real input events, disposable fixture profiles and
recipient-side Matrix history. No personal accounts and no visual score claim.
"""
import json, os, shutil, time, uuid
from pathlib import Path
from urllib.parse import quote
from native_probe import NativeApp
from seed import checked


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS='1', MAKEPAD_NO_FOCUS='1')
    os.environ.pop('MAKEPAD_FOCUS', None)
    fixture_path=Path('lab/wechat-ux/evidence/live/fixture.json')
    fixture=json.loads(fixture_path.read_text())
    assert all(u['user_id'].startswith('@robrix_ux_') for u in fixture['users'].values())
    root=fixture_path.parent/'article-editor'/uuid.uuid4().hex
    root.mkdir(parents=True, mode=0o700)
    report={'passed':False,'checks':[],'runs':[],'visual_acceptance':False}
    apps=[]
    def mark(name):report['checks'].append(name);print('PASS '+name,flush=True)
    def prepare(name,alias,lang):
        path=root/name;path.mkdir(mode=0o700)
        data=json.loads(json.dumps(fixture));data['users']['alex']=fixture['users'][alias]
        (path/'fixture.json').write_text(json.dumps(data));os.chmod(path/'fixture.json',0o600)
        (path/'profile').mkdir();(path/'profile/ui-language.json').write_text(json.dumps(lang))
        return path
    def start(path,port):
        app=NativeApp(path,port,size=(406,776));apps.append(app);app.start();report['runs'].append(str(app.output))
        app.wait_text('全部聊天',timeout=90);return app
    def fill(app,widget,text):
        app.click_id(widget);app.request('/k',c='A',cmd=1,wait=1);app.request('/k',c='Backspace',wait=1)
        if text:app.request('/t',t=text,wait=1)
    def capture(app,name):
        app.capture(name)
        (app.output/(name+'-widgets.json')).write_text(json.dumps(app.request('/snap'),ensure_ascii=False,indent=2))
    def open_app(app):app.click_id('discover_tab');app.click_id('discover_article');app.wait_text('应用详情')
    def grant(app):app.click_id('article_continue');app.wait_text('授权使用');app.click_id('article_allow');app.wait_text('文章编辑器')
    def select(app,name):
        fill(app,'article_chat_search',name);app.click_text(name);app.wait_text('确认发布' if not any(w.get('t')=='确认分享' for w in app.snap()) else '确认分享')
    def event_from(alias,predicate):
        end=time.monotonic()+40
        while time.monotonic()<end:
            events=checked(fixture['url'],'GET',f"rooms/{quote(fixture['rooms']['emma'],safe='')}/messages?dir=b&limit=30",token=fixture['users'][alias]['access_token'])['chunk']
            matches=[e for e in events if predicate(e)]
            if matches:return matches
            time.sleep(.4)
        raise AssertionError('Recipient history did not contain expected event')
    try:
        sender_path=prepare('sender','alex','zh-CN')
        sender=start(sender_path,8361)
        open_app(sender);capture(sender,'02-app-details')
        sender.click_id('article_continue');sender.wait_text(fixture['users']['alex']['user_id']);capture(sender,'03-account-consent')
        assert not list((sender_path/'profile').glob('mini-apps/**/draft.json'))
        sender.click_id('article_cancel');open_app(sender);grant(sender)
        title='周末读书笔记 '+root.name[:6]
        body='## 慢下来\n\n今天读到一句话：好的文字来自认真观察。\n\n- 阅读\n- 记录\n\n**Native HTML**\n\n<script>DO_NOT_RENDER</script><img src="https://invalid.example/spy"><a href="javascript:alert(1)">安全文字</a>'
        fill(sender,'article_title',title);fill(sender,'article_markdown',body)
        sender.click_id('article_save');sender.wait_text('草稿已保存在本设备');capture(sender,'04-markdown-editor')
        sender.click_id('article_preview');sender.wait_text('文章预览');sender.wait_text('慢下来',pixels=True);capture(sender,'05-native-preview')
        assert 'DO_NOT_RENDER' not in ' '.join(r['text'] for r in sender.ocr())
        sender.click_id('article_publish');sender.wait_text('选择聊天');capture(sender,'06-room-picker')
        fill(sender,'article_chat_search','Emma');sender.click_text('Emma Wilson');sender.wait_text('确认发布');capture(sender,'07-publish-confirmation')
        sender.click_id('article_confirm');sender.wait_text('已发布',timeout=45);capture(sender,'08-published')
        events=event_from('emma',lambda e:e['sender']==fixture['users']['alex']['user_id'] and title in e.get('content',{}).get('body',''))
        assert len(events)==1
        content=events[0]['content'];assert content['msgtype']=='m.text';html=content['formatted_body']
        assert '<strong>Native HTML</strong>' in html
        for bad in ['<script','<img','javascript:','invalid.example','DO_NOT_RENDER']:assert bad not in html
        mark('native_markdown_html_publish_and_recipient_delivery')
        sender.click_id('article_share_again');fill(sender,'article_chat_search','Emma');sender.click_text('Emma Wilson');sender.wait_text('确认分享');sender.click_id('article_confirm');sender.wait_text('小应用已发送',timeout=45)
        cards=event_from('emma',lambda e:e['sender']==fixture['users']['alex']['user_id'] and e.get('content',{}).get('msgtype')=='rs.robius.robrix.article_app')
        package=cards[0]['content']['app'];assert set(package)=={'app_id','version','source_hash'}
        assert not any(key in json.dumps(package) for key in ['token','grant','draft','password'])
        mark('shared_app_card_contains_no_sender_authority')
        sender.click_id('article_close');open_app(sender);sender.click_id('article_continue');sender.wait_text('授权使用');sender.click_id('article_allow');assert next(w for w in sender.snap() if w['i']=='article_title')['val']==title
        mark('reopening_requires_consent_and_restores_own_draft')
        recipient_path=prepare('recipient','emma','zh-CN')
        recipient=start(recipient_path,8362)
        recipient.click_text('Alex Chen');recipient.wait_text('文章编辑器',timeout=30,pixels=True);capture(recipient,'01-received-card')
        recipient.click_text('文章编辑器');recipient.wait_text('应用详情');recipient.click_id('article_continue');recipient.wait_text(fixture['users']['emma']['user_id']);capture(recipient,'recipient-account-consent')
        recipient.click_id('article_allow')
        texts={w['i']:w.get('val',w.get('t','')) for w in recipient.snap()}
        assert texts.get('article_title','')=='' and texts.get('article_markdown','')==''
        assert not list((recipient_path/'profile').glob('mini-apps/**/draft.json'))
        mark('recipient_authenticates_as_self_and_starts_without_sender_draft')
        reply='接收者自己的文章 '+root.name[:6]
        fill(recipient,'article_title',reply);fill(recipient,'article_markdown','## 我的草稿\n\n由接收者独立授权后发表。')
        recipient.click_id('article_preview');recipient.click_id('article_publish');fill(recipient,'article_chat_search','Alex');recipient.click_text('Alex Chen');recipient.click_id('article_confirm');recipient.wait_text('已发布',timeout=45)
        own=event_from('alex',lambda e:e['sender']==fixture['users']['emma']['user_id'] and reply in e.get('content',{}).get('body',''))
        assert len(own)==1;mark('recipient_publish_uses_recipient_matrix_identity')
        recipient.click_id('article_close');recipient.stop();apps.remove(recipient)
        (recipient_path/'profile/ui-language.json').write_text(json.dumps('en'))
        english=NativeApp(recipient_path,8362,size=(406,776));apps.append(english);english.start();english.wait_text('All Chats',timeout=90)
        english.click_id('discover_tab');english.click_id('discover_article');english.wait_text('App details');capture(english,'details-en')
        english.click_id('article_continue');english.wait_text('Authorize app');english.click_id('article_allow');assert next(w for w in english.snap() if w['i']=='article_title')['val']==reply;english.click_id('article_preview');english.wait_text('Article preview');capture(english,'preview-en')
        mark('english_chinese_native_ui_and_persisted_draft')
        report['passed']=True
    finally:
        if not report['passed']:
            for app in apps:
                if app.process and app.process.poll() is None:
                    try:capture(app,'failure')
                    except Exception:pass
        for app in apps:app.stop()
        (root/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps({'passed':report['passed'],'evidence':str(root),'checks':report['checks']}),flush=True)

if __name__=='__main__':main()
