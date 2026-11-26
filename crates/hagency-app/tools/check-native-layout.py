#!/usr/bin/env python3
"""Render production Makepad widgets with disposable data and check layout bounds.

Run from the workspace root after building workspace_preview and project_board_preview.
Original screenshots, widget rectangles and logs are retained for visual review.
This is UI verification, not authentication, backend or device qualification.
"""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import urllib.parse
import urllib.request
import urllib.error


def visible(rows, name):
    return [r for r in rows if r["i"] == name and r.get("v", 1) != 0
            and r["r"][2] > 0 and r["r"][3] > 0]


def inside(row, width, height):
    x, y, w, h = row["r"]
    assert x >= -1 and y >= -1 and x+w <= width+1 and y+h <= height+1, row


def run_case(root, scene, width, dark=False, scale=1.0, chinese=False, board=False):
    name = f'{scene}-{width}-{"dark" if dark else "light"}-text{scale}' + ('-zh' if chinese else '')
    output = root / name
    output.mkdir(parents=True, exist_ok=False)
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    binary = Path('target/debug/examples') / ('project_board_preview' if board else 'workspace_preview')
    args = [str(binary), f'--scene={scene}', f'--width={width}', f'--text-scale={scale}']
    if dark:
        args.append('--dark')
    if chinese:
        args.append('--zh')
    if board and width < 1000:
        args += ['--narrow', '--detail']
    log = (output / 'native.log').open('w')
    process = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, env=dict(
        os.environ, HAGENCY_APP_DATA_DIR=str(output / 'profile'), HAGENCY_NATIVE_CREDENTIAL_IO='deny', MAKEPAD_HIDE_WINDOWS='1',
        MAKEPAD_NO_FOCUS='1', MAKEPAD_REMOTE=str(port)))
    base = f'http://127.0.0.1:{port}'
    sequence = None

    def request(route, **params):
        if sequence is not None:
            params['if_user_seq'] = sequence
        with urllib.request.urlopen(base + route + '?' + urllib.parse.urlencode(params), timeout=15) as response:
            if sequence is not None:
                for header in ('X-Makepad-User-Seq-Start', 'X-Makepad-User-Seq'):
                    assert response.headers.get(header) == str(sequence), 'Native input changed; stopping automation'
            return json.load(response)

    result = {'case': name, 'passed': False, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    try:
        deadline = time.monotonic() + 40
        while True:
            assert process.poll() is None, f'{name} exited during startup'
            try:
                status = request('/s')
                assert status['pid'] == process.pid, 'Refusing a different process'
                if status['w']:
                    break
            except (OSError, ValueError):
                pass
            assert time.monotonic() < deadline, f'{name} did not start'
            time.sleep(.2)
        try:
            activity = request('/activity')
            assert not activity.get('user_active') and not activity.get('held'), 'Native input is active'
            sequence = activity['user_seq']
        except urllib.error.HTTPError as error:
            if error.code != 404:
                raise
            result['activity_guard'] = 'legacy_sdk_unavailable'
        time.sleep(.6)
        if scene == 'settings':
            menu = visible(request('/snap', all=1)['s'], 'category_interface_button')[0]
            x, y, w, h = menu['r']
            request('/click', x=x+w/2, y=y+h/2, wait=1)
            time.sleep(.3)
        # Window creation can precede the first painted frame on a cold font cache.
        request('/m', k='move', x=0, y=0, wait=1)
        screenshot = request('/g')
        shutil.copyfile(screenshot['png'], output / 'screen.png')
        snapshot = request('/snap', all=1)
        (output / 'widgets.json').write_text(json.dumps(snapshot, ensure_ascii=False, indent=2))
        rows = snapshot['s']
        w, h = request('/s')['w'][0]['sz']
        assert abs(w-width) <= 1, f'Expected a {width}px window, got {w}px'
        if not board:
            if scene in ('agents-ledger', 'agents-pending'):
                target = 'ledger_viewport' if scene == 'agents-ledger' else 'agent_recovery'
                viewport = visible(rows, target)[0]
                inside(viewport, w, h)
                assert viewport['r'][3] > 200
                inside(visible(rows, 'back')[0], w, h)
                if width >= 900:
                    assert visible(rows, 'agent_sidebar')
                else:
                    assert not visible(rows, 'agent_sidebar')
                if scene == 'agents-pending':
                    for button in ('check_request', 'retry_request'):
                        inside(visible(rows, button)[0], w, h)
            elif scene in ('projects', 'agents') or scene.startswith('legacy-'):
                agents = 'agents' in scene
                wide = width >= 900
                if wide:
                    welcome = visible(rows, 'workspace_welcome')[0]
                    inside(welcome, w, h)
                    title = visible(rows, 'welcome_title')[0]
                    inside(title, w, h)
                    expected = ('Choose an Agent' if agents else 'Select a Project')
                    assert title.get('t') in (expected, '选择 Agent', '选择一个 Agent', '选择项目'), title
                    inside(visible(rows, 'welcome_new')[0], w, h)
                    if agents:
                        sidebar = visible(rows, 'agent_sidebar')[0]
                        main = visible(rows, 'main_content')[0]
                        assert abs(sidebar['r'][2]-240) <= 1, sidebar
                        assert abs(main['r'][0]-sidebar['r'][0]-sidebar['r'][2]-1) <= 1
                        inside(visible(rows, 'agent_search')[0], w, h)
                        for badge in visible(rows, 'audience_badge') + visible(rows, 'device_badge'):
                            assert badge['r'][0] >= sidebar['r'][0] and badge['r'][0]+badge['r'][2] <= sidebar['r'][0]+sidebar['r'][2], badge
                    else:
                        assert visible(rows, 'project_context_review')
                elif agents:
                    sidebar = visible(rows, 'agent_sidebar')[0]
                    assert abs(sidebar['r'][2]-w) <= 1
                    inside(visible(rows, 'sidebar_new')[0], w, h)
                    inside(visible(rows, 'agent_search')[0], w, h)
                else:
                    header = visible(rows, 'panel_header')[0]
                    inside(header, w, h)
                    for control in ('new_item', 'refresh'):
                        inside(visible(rows, control)[0], w, h)
                if scene.startswith('legacy-'):
                    if agents:
                        empty = visible(rows, 'empty_message')[0]
                        inside(empty, w, h)
                        assert ('could not be loaded' in empty.get('t', '')) == scene.endswith('-error'), empty
                    result['legacy_dock_reaches_management'] = True
                elif agents:
                    assert any(row.get('t') == 'Research assistant' for row in visible(rows, 'title'))
            elif scene in ('overview', 'create'):
                panel = visible(rows, 'panel')[0]
                header = visible(rows, 'panel_header')[0]
                inside(header, w, h)
                inside(visible(rows, 'heading')[0], w, h)
                title = visible(rows, 'title_group')[0]
                expected_gutter = 16 if panel['r'][2] < 640 else 24
                assert abs(title['r'][0]-panel['r'][0]-expected_gutter) <= 1
                if scene == 'create':
                    inside(visible(rows, 'creation_name')[0], w, h)
                else:
                    for metric in ('metric_tasks', 'metric_done', 'metric_overdue'):
                        inside(visible(rows, metric)[0], w, h)
            elif scene == 'usage':
                toggles = visible(rows, 'agent_usage_toggle')
                assert len(toggles) == 3, 'Measured, answer-only and mobile Agents retain their entry; humans do not'
                for toggle in toggles:
                    inside(toggle, w, h)
                    assert toggle['r'][2] >= 24 and toggle['r'][3] >= 24
                assert not visible(rows, 'details'), 'Usage starts collapsed'
                for index, expected in [(0, '50'), (1, '未上报' if chinese else 'not reported'), (2, '未上报' if chinese else 'not reported')]:
                    # Each expansion changes later rows' vertical positions.
                    toggle = visible(request('/snap', all=1)['s'], 'agent_usage_toggle')[index]
                    x, y, tw, th = toggle['r']
                    request('/click', x=x+tw/2, y=y+th/2, wait=1)
                    expanded = request('/snap', all=1)
                    (output / f'expanded-{index}.json').write_text(json.dumps(expanded, ensure_ascii=False, indent=2))
                    details = visible(expanded['s'], 'details')
                    assert len(details) == 1 and expected in details[0].get('t', ''), details
                    inside(details[0], w, h)
                    screenshot = request('/g')
                    shutil.copyfile(screenshot['png'], output / f'expanded-{index}.png')
                    request('/click', x=x+tw/2, y=y+th/2, wait=1)
                    assert not visible(request('/snap', all=1)['s'], 'details'), 'Second click collapses usage'
                result['usage_entries_and_clicks'] = True
            elif scene.startswith('chats'):
                pane = visible(rows, 'chat_list_pane')[0]
                assert abs(pane['r'][2] - (280 if width >= 900 else w)) <= 1, pane
                sections = [row for row in visible(rows, 'name')
                            if row.get('t') in ('Private chats', 'Group chats', '私聊', '群聊')]
                assert len(sections) == (2 if scene == 'chats' else 1), sections
                for row in sections:
                    inside(row, w, h)
                private = next(row for row in sections if row.get('t') in ('Private chats', '私聊'))
                if scene == 'chats':
                    group = next(row for row in sections if row.get('t') in ('Group chats', '群聊'))
                    assert private['r'][1] < group['r'][1]
                    assert not visible(rows, 'notice'), 'Ready chats must not imply an outstanding request'
                else:
                    notice = visible(rows, 'notice')[0]
                    inside(notice, w, h)
                    assert notice['r'][3] <= 90, notice
                    status = visible(rows, 'status')[0]
                    assert 'classification' not in status.get('t', '')
                    if scene == 'chats-error':
                        assert visible(rows, 'retry') and not visible(rows, 'loading')
                        assert status.get('t') in ('Could not load group chats. Try again.', '群聊暂时无法加载，请重试。')
                    else:
                        assert visible(rows, 'loading') and not visible(rows, 'retry')
                        assert status.get('t') in ('Loading group chats…', '正在加载群聊…')
                assert any(row.get('t') == 'Alice Chen' for row in visible(rows, 'name'))
                if width >= 900:
                    for control in ('new_private_chat', 'new_group_chat', 'explore'):
                        inside(visible(rows, control)[0], w, h)
            elif scene == 'contacts':
                master = visible(rows, 'master_pane')[0]
                detail = visible(rows, 'contact_profile')[0]
                inside(master, w, h)
                inside(detail, w, h)
                assert abs(master['r'][2] - 300) < 1
                assert detail['r'][0] >= master['r'][0] + master['r'][2]
                inside(visible(rows, 'message')[0], w, h)
                assert not visible(rows, 'contact_chat'), 'Contacts must only own profile details'
                bob = next(row for row in visible(rows, 'name') if row.get('t') == 'Bob Wang')
                x, y, bw, bh = bob['r']
                request('/click', x=x+bw/2, y=y+bh/2, wait=1)
                time.sleep(.2)
                updated = request('/snap', all=1)['s']
                assert any('@bob:review.invalid' in row.get('t', '')
                           and row['r'][0] >= detail['r'][0] for row in visible(updated, 'user_id'))
                capture = request('/g')
                shutil.copyfile(capture['png'], output/'bob-profile.png')
                result['contact_click_updates_details'] = True
            elif scene == 'login':
                title = next(row for row in visible(rows, 'title') if row.get('t'))
                inside(title, w, h)
                assert title['r'][2] < 450, title
            else:
                inside(visible(rows, 'settings_header_title')[0], w, h)
        else:
            inside(visible(rows, 'toolbar')[0], w, h)
            lane = visible(rows, 'todo')[0]
            assert abs(lane['r'][2] - 280) <= 1, lane
        if scene == 'projects' and width == 1280 and not dark:
            before = visible(rows, 'welcome_content')[0]['r']
            request('/event', data='appearance:dark')
            time.sleep(.3)
            request('/m', k='move', x=0, y=0, wait=1)
            capture = request('/g')
            shutil.copyfile(capture['png'], output/'restyled-dark.png')
            after = visible(request('/snap', all=1)['s'], 'welcome_content')[0]['r']
            assert all(abs(a-b) <= 1 for a,b in zip(before,after)), (before,after)
            result['theme_reapply_preserves_geometry'] = True
        log.flush()
        errors = [line for line in (output/'native.log').read_text().splitlines()
                  if '[E]' in line or 'panicked at' in line or 'Template not found:' in line]
        assert not errors, errors[:5]
        result.update(passed=True, viewport=[w, h], screenshot=str(output/'screen.png'))
        return result
    except Exception as error:
        result['error'] = str(error)
        raise
    finally:
        (output / 'result.json').write_text(json.dumps(result, indent=2))
        if process.poll() is None:
            # The loopback guard refuses shutdown if a person used the window.
            try:
                request('/quit')
            except (http.client.RemoteDisconnected, ConnectionResetError):
                # A graceful quit can close the bridge before its response is
                # flushed. Only accept this after observing this owned process
                # exit successfully; a lost live bridge still fails the check.
                assert process.wait(timeout=15) == 0, 'Preview crashed during shutdown'
            process.wait(timeout=15)
        log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--only', help='Optional comma-separated scene names for focused checks')
    args = parser.parse_args()
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=False)
    cases = [
        ('projects',1280), ('agents',1280), ('agents',1280,True,1.3,True), ('overview',1280), ('create',960),
        ('agents-ledger',1280), ('agents-ledger',430), ('agents-pending',1280), ('agents-pending',430),
        ('legacy-projects-empty',1280), ('legacy-projects-error',960),
        ('legacy-agents-empty',1280), ('legacy-agents-error',960,True),
        ('chats',1280), ('chats',430,True,1.0,True), ('chats-loading',1280), ('chats-error',1280), ('chats-error',430,True,1.0,True), ('contacts',1280), ('contacts',1280,True),
        ('usage',1280), ('usage',430), ('usage',430,True,1.3,True),
        ('settings',1100), ('settings',430), ('login',1280), ('projects',430), ('agents',430), ('create',430),
        ('overview',430), ('login',430), ('projects',1280,True),
        ('overview',1280,True), ('projects',430,False,1.3,True),
        ('board',1360,False,1.0,False,True), ('board',1360,True,1.0,False,True),
        ('board',660,False,1.0,False,True),
    ]
    if args.only:
        cases = [case for case in cases if case[0] in args.only.split(',')]
    assert cases, 'No matching native layout cases'
    results = []
    for case in cases:
        result = run_case(root, *case)
        results.append(result)
        (root/'results.json').write_text(json.dumps(results, indent=2))
        print('PASS ' + result['case'], flush=True)
    print(json.dumps({'passed':True, 'cases':len(results)}), flush=True)


if __name__ == '__main__':
    main()
