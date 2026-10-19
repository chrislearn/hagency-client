#!/usr/bin/env python3
"""Exercise Chats' joined Space tree against isolated Palpo fixture accounts."""
import json, os, shutil, time, uuid
from pathlib import Path
from native_probe import NativeApp
from seed import checked


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS='1', MAKEPAD_NO_FOCUS='1')
    os.environ.pop('MAKEPAD_FOCUS', None)
    source = Path('lab/wechat-ux/evidence/live')
    root = source / 'joined-spaces'
    root.mkdir(mode=0o700, exist_ok=True)
    shutil.copyfile(source/'fixture.json', root/'fixture.json')
    os.chmod(root/'fixture.json', 0o600)
    if not (root/'profile').exists():
        shutil.copytree(source/'room-history/profile', root/'profile')
    fixture = json.loads((root/'fixture.json').read_text())
    alex, emma = (fixture['users'][key] for key in ('alex', 'emma'))
    assert all(user['user_id'].startswith('@robrix_ux_') for user in (alex, emma))
    base = fixture['url']
    def api(method, path, data=None, user=alex):
        return checked(base, method, path, data, token=user['access_token'])
    saved = root/'seed.json'
    if saved.exists():
        rooms = json.loads(saved.read_text())
    else:
        rooms = {}
        for key, name, space in [('team', 'UX Spaces Team', True), ('projects', 'UX Spaces Projects', True),
                                 ('general', 'UX Spaces General', False), ('design', 'UX Spaces Design', False),
                                 ('unjoined', 'UX Spaces Unjoined', False)]:
            body = {'name': name, 'preset': 'public_chat'}
            if space: body['creation_content'] = {'type': 'm.space'}
            else: body['invite'] = [emma['user_id']]
            rooms[key] = api('POST', 'createRoom', body)['room_id']
            if not space: api('POST', 'join/'+rooms[key], {}, emma)
        saved.write_text(json.dumps(rooms)); os.chmod(saved, 0o600)
    for key in ['team', 'projects', 'general', 'design']:
        api('POST', 'join/'+rooms[key], {})
    server = alex['user_id'].split(':', 1)[1]
    def link(parent, child, present=True):
        api('PUT', 'rooms/'+rooms[parent]+'/state/m.space.child/'+rooms[child], {'via': [server]} if present else {})
    for parent, child in [('team','projects'), ('team','general'), ('projects','design'), ('projects','general'), ('team','unjoined')]:
        link(parent, child)
    api('POST', 'rooms/'+rooms['unjoined']+'/leave', {})
    for key in ['general','design']:
        api('PUT', 'rooms/'+rooms[key]+'/send/m.room.message/'+uuid.uuid4().hex,
            {'msgtype':'m.text','body':'Space navigation fixture '+key}, emma)
    result = {'passed': False, 'checks': [], 'runs': []}
    app = None
    def passed(name):
        result['checks'].append(name); print('PASS', name, flush=True)
    def start(size):
        nonlocal app
        app = NativeApp(root, port=8299, size=size); app.start()
        result['runs'].append(str(app.output))
        app.wait_text('All Chats', timeout=90)
        if size[0] < 600:
            app.wait_text('Search', pixels=True)
    def click_text(text):
        rows = [w for w in app.snap() if w.get('t') == text]
        if rows:
            x,y,w,h = rows[-1]['r']; app.click(x+w/2,y+h/2)
        else: app.click_text(text)
    def visible(text): return any(w.get('t') == text for w in app.snap())
    def query(text):
        # Root search is the only visible TextInput in the Chats sidebar.
        row = next(w for w in app.snap() if w['i'] == 'input' and w['r'][1] < 180)
        x,y,w,h = row['r']; app.click(x+w/2,y+h/2)
        app.request('/k',c='A',cmd=1,wait=1); app.request('/k',c='Backspace',wait=1)
        if text: app.request('/t',t=text,wait=1)
        time.sleep(.5)
    try:
        start((375,812))
        app.wait_text('UX Spaces Design',timeout=90)
        assert not visible('UX Spaces Team')
        app.capture('all-chats-switch'); passed('all_chats_default_flat_rooms_without_space_containers')
        app.click_id('joined_spaces_tab'); app.wait_text('UX Spaces Team',timeout=45)
        assert not visible('UX Spaces General')
        click_text('UX Spaces Team'); app.wait_text('UX Spaces Projects'); app.wait_text('UX Spaces General')
        assert not visible('UX Spaces Unjoined')
        click_text('UX Spaces Projects'); app.wait_text('UX Spaces Design')
        app.wait_text('Search',pixels=True)
        app.capture('joined-space-tree'); passed('named_spaces_expand_to_joined_rooms_and_nested_spaces')
        click_text('UX Spaces Design'); app.wait_text('Space navigation fixture design',pixels=True,timeout=60)
        app.click(24,54); app.wait_text('UX Spaces Team'); app.wait_text('UX Spaces Design')
        passed('room_opens_and_back_preserves_expanded_tree')
        query('Design'); app.wait_text('UX Spaces Team'); app.wait_text('UX Spaces Projects'); app.wait_text('UX Spaces Design')
        assert not visible('UX Spaces General')
        app.capture('space-room-search'); passed('name_search_preserves_ancestor_context')
        query(''); click_text('UX Spaces Projects'); assert not visible('UX Spaces Design')
        passed('collapse_hides_nested_rooms')
        click_text('Browse rooms ›'); app.wait_text('UX Spaces Unjoined',pixels=True,timeout=45)
        app.capture('browse-space-rooms'); app.click(24,54); app.wait_text('UX Spaces Team')
        passed('browse_rooms_opens_existing_space_lobby_and_returns')
        app.click_id('discover_tab'); app.wait_text('Explore Groups & Spaces')
        assert not visible('UX Spaces Team') and not visible('All Chats')
        click_text('Explore Groups & Spaces'); app.wait_text('Explore Rooms')
        assert not visible('UX Spaces Team')
        app.capture('discover-no-membership-rail'); app.click_id('back'); app.wait_text('Explore Groups & Spaces')
        passed('discover_explores_without_joined_space_strip')
        app.click_id('chats_tab'); app.wait_text('UX Spaces Team')
        app.click_id('all_chats'); app.wait_text('UX Spaces General')
        assert sum(w.get('t') == 'UX Spaces General' for w in app.snap()) == 1
        assert not visible('UX Spaces Team')
        passed('all_chats_deduplicates_room_with_multiple_space_parents')
        app.click_id('joined_spaces_tab'); app.wait_text('UX Spaces Team')
        link('team','general',False)
        deadline=time.monotonic()+30
        while visible('UX Spaces General') and time.monotonic()<deadline: time.sleep(.5)
        assert not visible('UX Spaces General')
        link('team','general'); app.wait_text('UX Spaces General',timeout=30)
        passed('live_space_child_updates_refresh_tree')
        app.stop(); start((1050,800))
        app.click_id('joined_spaces_tab'); app.wait_text('UX Spaces Team',timeout=45)
        click_text('UX Spaces Team'); app.wait_text('UX Spaces General')
        app.capture('desktop-joined-spaces'); click_text('UX Spaces General')
        app.wait_text('Space navigation fixture general',pixels=True,timeout=60)
        passed('wide_mac_sidebar_switch_and_room_navigation')
        result['passed'] = True
    finally:
        if app and app.process and app.process.poll() is None and not result['passed']: app.capture('failure')
        if app: app.stop()
        result['native_errors'] = sum(any(marker in line for marker in ['[E]', 'panicked at', 'Assertion failed:'])
            for run in result['runs'] for line in (Path(run)/'native.log').read_text(errors='replace').splitlines())
        result['passed'] = result['passed'] and result['native_errors'] == 0
        (root/'native-joined-spaces.json').write_text(json.dumps(result,indent=2))
        print(json.dumps(result),flush=True)
    assert result['passed']


if __name__ == '__main__': main()
