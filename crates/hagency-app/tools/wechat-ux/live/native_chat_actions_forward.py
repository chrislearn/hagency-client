#!/usr/bin/env python3
"""Swipe, local clear, and actual forwarding regression checks on isolated Palpo fixtures."""
import json, os, shutil, time, urllib.parse, uuid
from pathlib import Path
from native_probe import NativeApp
from seed import checked, api


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS='1', MAKEPAD_NO_FOCUS='1')
    os.environ.pop('MAKEPAD_FOCUS', None)
    source = Path('lab/wechat-ux/evidence/live')
    root = source / 'chat-actions-forward'
    root.mkdir(mode=0o700, exist_ok=True)
    shutil.copyfile(source / 'fixture.json', root / 'fixture.json')
    os.chmod(root / 'fixture.json', 0o600)
    fixture = json.loads((root / 'fixture.json').read_text())
    alex, emma = (fixture['users'][key] for key in ('alex', 'emma'))
    base = fixture['url']
    rooms_file = root / 'rooms.json'
    rooms = json.loads(rooms_file.read_text()) if rooms_file.exists() else {}
    names = {'source': 'UX Forward Source', 'individual': 'UX Individual Target', 'bundle': 'UX Bundle Target', 'encrypted': 'UX Encrypted Target'}
    for key, name in names.items():
        if key in rooms:
            continue
        body = {'name': name, 'preset': 'private_chat', 'invite': [emma['user_id']]}
        if key == 'encrypted':
            body['initial_state'] = [{'type': 'm.room.encryption', 'state_key': '', 'content': {'algorithm': 'm.megolm.v1.aes-sha2'}}]
        rooms[key] = checked(base, 'POST', 'createRoom', body, token=alex['access_token'])['room_id']
        checked(base, 'POST', 'join/' + rooms[key], {}, token=emma['access_token'])
        rooms_file.write_text(json.dumps(rooms)); os.chmod(rooms_file, 0o600)
    run = uuid.uuid4().hex[:7].translate(str.maketrans('0123456789', 'abcdefghij'))
    first, second = 'Forward alpha ' + run, 'Forward beta ' + run + ' 中文'
    def send(text):
        return checked(base, 'PUT', 'rooms/' + rooms['source'] + '/send/m.room.message/' + uuid.uuid4().hex,
                       {'msgtype': 'm.text', 'body': text}, token=emma['access_token'])['event_id']
    source_ids = [send(first), send(second)]
    membership_before = checked(base, 'GET', 'joined_rooms', token=alex['access_token'])['joined_rooms']
    app = None
    result = {'passed': False, 'checks': [], 'runs': []}
    def passed(name):
        result['checks'].append(name); print('PASS', name, flush=True)
    def start():
        nonlocal app
        app = NativeApp(root, port=8299, size=(375, 812)); app.start()
        result['runs'].append(str(app.output))
        app.wait_text('Emma Wilson', timeout=90)
    def stop():
        if app: app.stop()
    def text_click(text, button=0):
        row = [row for row in app.ocr() if text in row['text']][-1]
        x, y, w, h = row['box']; x, y = (x+w/2)*375, (y+h/2)*812
        if button:
            app.request('/m', k='down', x=x, y=y, b=button, wait=1)
            app.request('/m', k='up', x=x, y=y, b=button, wait=1)
        else: app.click(x, y)
    def swipe(name):
        row = next(row for row in app.ocr() if name in row['text'])
        _, y, _, h = row['box']; y = (y+h/2)*812
        app.request('/m', k='down', x=315, y=y, wait=1)
        app.request('/m', k='move', x=205, y=y, wait=1)
        app.request('/m', k='up', x=205, y=y, wait=1)
        app.wait_text('Hide', pixels=True)
    def replace(widget, text):
        app.click_id(widget); app.request('/k', c='A', cmd=1, wait=1); app.request('/t', t=text, wait=1)
    def history(key):
        return checked(base, 'GET', 'rooms/' + rooms[key] + '/messages?dir=b&limit=40', token=emma['access_token'])['chunk']
    def open_source():
        app.click_id('chats_tab')
        app.wait_text(names['source'], pixels=True, timeout=60)
        text_click(names['source']); app.wait_text(second.split(' 中文')[0], pixels=True, timeout=60)
    def select_pair():
        text_click(second.split(' 中文')[0], button=1)
        app.wait_text('Select / Forward', pixels=True)
        app.click_id('forward_button'); app.wait_text('Select Messages', pixels=True)
        app.wait_text(first, pixels=True); text_click(first)
        app.wait_text('2 selected', pixels=True)
        app.capture('forward-two-selected')
        app.click_id('proceed'); app.wait_text('Forward To', pixels=True)
    def choose(key, count=1):
        replace('recipient_search', names[key]); app.wait_text(names[key], pixels=True)
        text_click(names[key]); app.wait_text(str(count) + ' recipient', pixels=True)
    try:
        start()
        app.wait_text(names['source'], pixels=True, timeout=60)
        swipe(names['source']); app.capture('swipe-actions')
        app.click_id('swipe_unread')
        # First action may mark a newly seeded unread chat read; opening also clears it.
        app.wait_text(names['source'], pixels=True)
        text_click(names['source']); app.wait_text(second.split(' 中文')[0], pixels=True); app.click(24,54)
        swipe(names['source']); app.wait_text('Unread', pixels=True, timeout=30); app.click_id('swipe_unread')
        path = 'user/' + urllib.parse.quote(alex['user_id'], safe='') + '/rooms/' + urllib.parse.quote(rooms['source'], safe='') + '/account_data/m.marked_unread'
        for _ in range(60):
            status, data = api(base, 'GET', path, token=alex['access_token'])
            if status == 200 and data.get('unread'): break
            time.sleep(.25)
        assert status == 200 and data.get('unread'), 'Unread flag was not stored in Matrix'
        passed('swipe_marks_unread_in_matrix')
        open_source(); select_pair(); choose('individual')
        app.capture('forward-individual-confirm'); app.click_id('proceed')
        app.wait_text('Messages forwarded', pixels=True, timeout=60)
        sent = [e['content']['body'] for e in reversed(history('individual')) if e['type']=='m.room.message' and e['content'].get('body') in [first, second]]
        assert sent == [first, second], sent
        passed('individual_forward_preserves_selected_order_and_no_duplicates')
        app.click(24,54); open_source(); select_pair(); choose('bundle')
        app.click_id('bundle'); app.capture('forward-bundle-confirm'); app.click_id('proceed')
        app.wait_text('Messages forwarded', pixels=True, timeout=60)
        bundles = [e['content'] for e in history('bundle') if e['type']=='m.room.message' and e['content'].get('msgtype')=='rs.robius.robrix.forwarded_chat' and run in e['content'].get('body','')]
        assert len(bundles)==1
        assert [m['event_id'] for m in bundles[0]['forwarded_chat']['messages']]==source_ids
        assert first in bundles[0]['body'] and second in bundles[0]['body']
        passed('bundled_forward_has_selected_messages_and_plaintext_fallback')
        app.click(24,54); app.wait_text(names['bundle'], pixels=True); text_click(names['bundle'])
        app.wait_text('Chat history', pixels=True); app.capture('forward-received-card'); text_click('Chat history')
        app.wait_text('Chat History', pixels=True); app.wait_text(first, pixels=True); app.wait_text(second.split(' 中文')[0], pixels=True)
        app.capture('forward-open-history'); app.click_id('back'); app.click(24,54)
        passed('received_bundle_opens_readable_history')
        encrypted_before = {e['event_id'] for e in history('encrypted')}
        open_source(); select_pair(); choose('encrypted'); app.click_id('proceed')
        app.wait_text('Messages forwarded', pixels=True, timeout=90)
        encrypted_after = history('encrypted')
        assert len([e for e in encrypted_after if e['type']=='m.room.encrypted' and e['event_id'] not in encrypted_before]) == 2
        assert not any(e['content'].get('body') in [first, second] for e in encrypted_after)
        app.click(24,54); app.wait_text(names['encrypted'], pixels=True); text_click(names['encrypted'])
        app.wait_text(first, pixels=True, timeout=60); app.wait_text(second.split(' 中文')[0], pixels=True)
        app.capture('forward-encrypted-received'); app.click(24,54)
        passed('encrypted_destination_uses_encrypted_matrix_events_and_renders_plaintext')
        # Freeze a two-recipient send, reject its second destination, and retry.
        open_source(); select_pair(); choose('individual'); choose('bundle', count=2)
        denied_key = max(('individual', 'bundle'), key=lambda key: rooms[key])
        denied_room = rooms[denied_key]
        levels_path = 'rooms/' + denied_room + '/state/m.room.power_levels/'
        original_levels = checked(base, 'GET', levels_path, token=alex['access_token'])
        before_counts = {key: sum(e['type']=='m.room.message' and e['content'].get('body') in [first,second] for e in history(key)) for key in ('individual','bundle')}
        levels_changed = False
        try:
            restricted = {**original_levels, 'users': {**original_levels.get('users', {}), alex['user_id']: 0, emma['user_id']: 100},
                          'events': {**original_levels.get('events', {}), 'm.room.message': 50}}
            checked(base, 'PUT', levels_path, restricted, token=alex['access_token'])
            levels_changed = True
            app.click_id('proceed'); app.wait_text('Retry Remaining', pixels=True, timeout=60)
            app.wait_text('Sent 2 of 4', pixels=True)
            app.capture('forward-partial-failure')
        finally:
            if levels_changed:
                checked(base, 'PUT', levels_path, original_levels, token=emma['access_token'])
        app.click_id('proceed'); app.wait_text('Messages forwarded', pixels=True, timeout=90)
        for key in ('individual','bundle'):
            after = sum(e['type']=='m.room.message' and e['content'].get('body') in [first,second] for e in history(key))
            assert after == before_counts[key]+2
        app.click(24,54)
        passed('multiple_recipients_partial_failure_retry_does_not_duplicate_confirmed_sends')
        swipe(names['source']); app.click_id('swipe_hide'); time.sleep(.5)
        assert not any(names['source'] in r['text'] for r in app.ocr())
        app.capture('chat-hidden'); stop(); start()
        assert not any(names['source'] in r['text'] for r in app.ocr())
        passed('hidden_chat_stays_hidden_after_restart')
        replace('input', 'UX Forward Source'); app.wait_text(names['source'], pixels=True)
        text_click(names['source']); app.wait_text(second.split(' 中文')[0], pixels=True)
        app.click(24,54)
        app.click_id('clear_button'); app.wait_text(names['source'], pixels=True)
        passed('search_finds_hidden_chat_and_open_restores_it')
        swipe(names['source']); app.click_id('swipe_delete')
        app.wait_text('Delete Chat', pixels=True); app.capture('delete-chat-confirm')
        app.click_id('cancel_button'); app.wait_text(names['source'], pixels=True)
        passed('delete_cancel_keeps_chat')
        swipe(names['source']); app.click_id('swipe_delete'); app.click_id('accept_button')
        time.sleep(.5)
        assert not any(names['source'] in r['text'] for r in app.ocr())
        stop(); start()
        assert not any(names['source'] in r['text'] for r in app.ocr())
        new_text = 'After clear ' + run; send(new_text)
        app.wait_text(names['source'], pixels=True, timeout=60)
        text_click(names['source']); app.wait_text(new_text, pixels=True, timeout=60)
        assert not any(first in r['text'] or second.split(' 中文')[0] in r['text'] for r in app.ocr())
        app.capture('deleted-chat-new-message')
        membership_after = checked(base, 'GET', 'joined_rooms', token=alex['access_token'])['joined_rooms']
        assert set(membership_before)==set(membership_after)
        assert all(any(e['event_id']==id for e in history('source')) for id in source_ids)
        passed('delete_survives_restart_keeps_membership_and_server_history_new_messages_restore_chat')
        result['passed'] = True
    finally:
        if app and app.process and app.process.poll() is None and not result['passed']:
            app.capture('failure')
        stop()
        result['native_errors'] = 0
        for run_path in result['runs']:
            log = Path(run_path) / 'native.log'
            if log.exists():
                result['native_errors'] += sum(any(m in line for m in ['[E]', 'panicked at', 'Assertion failed:']) for line in log.read_text(errors='replace').splitlines())
        result['passed'] = result['passed'] and result['native_errors']==0
        (root / 'native-chat-actions-forward.json').write_text(json.dumps(result, indent=2))
        print(json.dumps(result), flush=True)
    assert result['passed'], 'Native errors were recorded'


if __name__ == '__main__':
    main()
