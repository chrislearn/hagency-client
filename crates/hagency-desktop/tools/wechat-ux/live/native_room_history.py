#!/usr/bin/env python3
"""Room history and attachment checks using isolated Matrix fixtures and native UI."""
import io, json, os, shutil, time, urllib.parse, urllib.request, uuid, wave
from pathlib import Path
from native_probe import NativeApp
from seed import checked


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS='1', MAKEPAD_NO_FOCUS='1')
    os.environ.pop('MAKEPAD_FOCUS', None)
    source = Path('lab/wechat-ux/evidence/live')
    root = source / 'room-history'
    root.mkdir(mode=0o700, exist_ok=True)
    shutil.copyfile(source/'fixture.json', root/'fixture.json'); os.chmod(root/'fixture.json', 0o600)
    # The forwarding fixture is stopped. Copy its test device so encrypted
    # history can be decrypted without touching the user's personal profile.
    if not (root/'profile').exists():
        shutil.copytree(source/'chat-actions-forward/profile', root/'profile')
    fixture = json.loads((root/'fixture.json').read_text())
    alex, emma = (fixture['users'][key] for key in ('alex','emma'))
    assert all(user['user_id'].startswith('@robrix_ux_') for user in (alex, emma))
    base = fixture['url']
    seed_file = root / 'seed.json'
    if os.environ.get('ROBRIX_HISTORY_REUSE') == '1' and seed_file.exists():
        saved = json.loads(seed_file.read_text())
        run, room, room_name, old_event = (saved[key] for key in ('run','room','room_name','old_event'))
        needle, corrected = 'History needle ' + run, 'Corrected label ' + run
    else:
        run = uuid.uuid4().hex[:6].translate(str.maketrans('0123456789','abcdefghij'))
        room_name = 'UX History ' + run
        room = checked(base,'POST','createRoom',{'name':room_name,'preset':'private_chat','invite':[emma['user_id']]},token=alex['access_token'])['room_id']
        checked(base,'POST','join/'+room,{},token=emma['access_token'])
        def send(content):
            return checked(base,'PUT','rooms/'+room+'/send/m.room.message/'+uuid.uuid4().hex,content,token=emma['access_token'])['event_id']
        needle = 'History needle ' + run
        old_event = send({'msgtype':'m.text','body':needle+' 中文'})
        for i in range(110): send({'msgtype':'m.text','body':'Recent spacer '+str(i)})
        edited = send({'msgtype':'m.text','body':'Obsolete label '+run})
        corrected = 'Corrected label '+run
        send({'msgtype':'m.text','body':'* '+corrected,'m.relates_to':{'rel_type':'m.replace','event_id':edited},'m.new_content':{'msgtype':'m.text','body':corrected}})
        removed = send({'msgtype':'m.text','body':'Removed label '+run})
        checked(base,'PUT','rooms/'+room+'/redact/'+removed+'/'+uuid.uuid4().hex,{},token=emma['access_token'])
        assets = json.loads((source/'media-fixture.json').read_text())['assets']
        photo = assets['resources/img/post2.jpg']
        send({'msgtype':'m.image','body':'Room photo '+run,'filename':'room-photo.jpg','url':photo['uri'],'info':photo['info']})
        def upload(name, data, mime):
            request=urllib.request.Request(base+'/_matrix/media/v3/upload?filename='+urllib.parse.quote(name),data=data,method='POST',headers={'Authorization':'Bearer '+emma['access_token'],'Content-Type':mime})
            with urllib.request.urlopen(request,timeout=30) as response: return json.load(response)['content_uri']
        data=b'Isolated Rinx room attachment test.\n'
        send({'msgtype':'m.file','body':'Room note '+run,'filename':'project-notes.txt','url':upload('project-notes.txt',data,'text/plain'),'info':{'size':len(data),'mimetype':'text/plain'}})
        wav=io.BytesIO()
        with wave.open(wav,'wb') as writer:
            writer.setnchannels(1); writer.setsampwidth(2); writer.setframerate(8000); writer.writeframes(b'\0\0'*800)
        data=wav.getvalue()
        send({'msgtype':'m.audio','body':'Room audio '+run,'filename':'room-audio.wav','url':upload('room-audio.wav',data,'audio/wav'),'info':{'size':len(data),'mimetype':'audio/wav','duration':100}})
        other = checked(base,'POST','createRoom',{'name':'UX Other '+run,'preset':'private_chat'},token=alex['access_token'])['room_id']
        checked(base,'PUT','rooms/'+other+'/send/m.room.message/'+uuid.uuid4().hex,{'msgtype':'m.text','body':'Other room only '+run},token=alex['access_token'])
        seed_file.write_text(json.dumps({'run':run,'room':room,'room_name':room_name,'old_event':old_event}))
    result={'passed':False,'checks':[], 'room':room, 'needle_event':old_event}
    app=NativeApp(root,port=8299,size=(375,812))
    def passed(name): result['checks'].append(name); print('PASS',name,flush=True)
    def click_text(text):
        row=[r for r in app.ocr() if text in r['text']][-1]
        x,y,w,h=row['box']; app.click((x+w/2)*375,(y+h/2)*812)
    def replace(text):
        app.click_id('history_query'); app.request('/k',c='A',cmd=1,wait=1)
        app.request('/k',c='Backspace',wait=1)
        if text: app.request('/t',t=text,wait=1)
    def query(text):
        replace(text); app.click_id('search_history'); app.wait_text('All available history checked.',timeout=90)
    def open_info(name):
        app.wait_text(name,timeout=60)
        row=next(w for w in app.snap() if w.get('t')==name)
        x,y,w,h=row['r']; app.click(x+w/2,y+h/2)
        app.wait_text('Message',pixels=True,timeout=60); app.click(350,54)
        app.wait_text('Chat Info',pixels=True); app.wait_text('Search Chat History',pixels=True,timeout=60)
    try:
        app.start(); app.wait_text('Emma Wilson',timeout=90)
        open_info(room_name); click_text('Search Chat History')
        app.wait_text('Older history is available.',timeout=60)
        replace(needle); app.wait_text('0 results')
        app.capture('search-older-not-yet-loaded')
        app.click_id('search_history'); app.wait_text('All available history checked.',timeout=90)
        app.wait_text('1 result'); app.wait_text('History needle',pixels=True)
        app.capture('search-older-result'); passed('search_finds_messages_beyond_initial_page')
        click_text('History needle'); app.wait_text('Message Details',pixels=True); app.wait_text('History needle',pixels=True)
        app.click_id('view_in_chat'); app.wait_text('History needle',pixels=True,timeout=90)
        app.wait_text('Message (unencrypted)',pixels=True)
        app.capture('search-jump-to-original'); passed('view_in_chat_returns_to_original_message')
        app.click(350,54); app.wait_text('Chat Info',pixels=True); click_text('Search Chat History')
        query('Obsolete label'); app.wait_text('0 results')
        query(corrected); app.wait_text('1 result')
        query('Removed label'); app.wait_text('0 results')
        passed('edited_text_replaces_original_and_redacted_messages_are_excluded')
        query('Other room only'); app.wait_text('0 results')
        passed('search_is_scoped_to_current_room')
        app.click_id('back'); app.wait_text('Chat Info',pixels=True)
        click_text('Shared Attachments'); app.wait_text('Shared Attachments',pixels=True)
        app.wait_text('Room photo',pixels=True,timeout=60); app.wait_text('1 result')
        app.capture('room-photos'); click_text('Room photo'); app.wait_text('Message Details',pixels=True)
        app.wait_text('Download',pixels=True); app.wait_text('Share',pixels=True)
        app.capture('room-photo-detail'); passed('room_photo_filter_and_detail_preview')
        app.click_id('back'); app.click_id('file_filter'); app.wait_text('Room note',pixels=True)
        replace('PROJECT-NOTES.TXT'); app.wait_text('1 result')
        click_text('Room note'); app.wait_text('project-notes.txt',pixels=True)
        app.capture('room-file-detail'); passed('file_filter_and_case_insensitive_filename_search')
        app.click_id('back'); replace(''); app.click_id('audio_filter'); app.wait_text('Room audio',pixels=True)
        app.wait_text('1 result'); app.capture('room-audio-filter'); passed('room_audio_filter')
        app.request('/k',c='Escape',wait=1); app.wait_text('Chat Info',pixels=True)
        app.click(24,54); app.wait_text('Message (unencrypted)',pixels=True); app.click(24,54)
        open_info('UX Encrypted Target'); click_text('Search Chat History')
        query('Forward alpha'); app.wait_text('Forward alpha',pixels=True)
        assert not any('0 results' in row['text'] for row in app.ocr())
        app.capture('encrypted-room-search'); passed('encrypted_history_search_uses_available_device_keys')
        app.click_id('back'); app.click(24,54); app.wait_text('Message',pixels=True); app.click(24,54)
        open_info('UX Forward Source'); click_text('Search Chat History')
        query('Forward alpha'); app.wait_text('0 results')
        app.capture('cleared-history-excluded'); passed('delete_chat_cutoff_also_applies_to_room_search')
        result['passed']=True
    finally:
        if app.process and app.process.poll() is None and not result['passed']: app.capture('failure')
        app.stop(); result['run']=str(app.output)
        log=(app.output/'native.log').read_text(errors='replace') if (app.output/'native.log').exists() else ''
        result['native_errors']=sum(any(marker in line for marker in ['[E]','panicked at','Assertion failed:']) for line in log.splitlines())
        result['passed']=result['passed'] and result['native_errors']==0
        (root/'native-room-history.json').write_text(json.dumps(result,indent=2)); print(json.dumps(result),flush=True)
    assert result['passed'], 'Native UI errors were recorded'


if __name__=='__main__': main()
