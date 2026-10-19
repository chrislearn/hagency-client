#!/usr/bin/env python3
"""Capture the public makepad_wechat mock, never a user's WeChat account.

Optional dependency: playwright. Supply --chromium for an installed executable,
or install Playwright's Chromium. Coordinates come from the inspected 400x800
published demo; output always requires visual review after deployment changes.
This is reference collection, not a Rinx acceptance test.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

URL = 'https://wasm.robius.rs/makepad_wechat/'
# Each route starts in a fresh page load. These are actual canvas input events.
ROUTES = {
    'chats': [],
    'chats-menu': [(380, 65)],
    'add-contact-candidate': [(380, 65), (280, 101)],
    'contacts': [(150, 765)],
    'discover': [(250, 765)],
    'moments': [(250, 765), (150, 121)],
    'me': [(345, 765)],
    'profile-edit': [(345, 765), (380, 176)],
    'conversation': [(150, 170)],
}


def main():
    from playwright.sync_api import sync_playwright
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--chromium', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    receipt = {'kind': 'makepad_wechat_capture', 'url': URL,
               'captured_at': datetime.now(timezone.utc).isoformat(),
               'logical_viewport': [400, 800], 'device_pixel_ratio': 2, 'pixels': [800, 1600],
               'locale': 'mixed English/Chinese as shipped by mock',
               'deployed_source_commit': None, 'not_actual_wechat': True,
               'scope': 'Reference screenshots only; no native Rinx or backend acceptance.',
               'review_status': 'unreviewed', 'assets': [], 'screens': [], 'errors': []}
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(
                executable_path=str(args.chromium) if args.chromium else None,
                headless=True,
                args=['--use-angle=swiftshader', '--enable-webgl', '--enable-unsafe-swiftshader'])
            receipt['browser'] = browser.version
            receipt['renderer'] = 'Chromium SwiftShader'
            context = browser.new_context(viewport={'width': 400, 'height': 800}, device_scale_factor=2)
            page = context.new_page()
            page.on('pageerror', lambda error: receipt['errors'].append(str(error)))
            def asset(response):
                if '.wasm' in response.url:
                    try:
                        data = response.body()
                        entry = {'url': response.url, 'status': response.status,
                                 'sha256': hashlib.sha256(data).hexdigest()}
                        if entry not in receipt['assets']:
                            receipt['assets'].append(entry)
                    except Exception as error:
                        receipt['errors'].append(str(error))
            page.on('response', asset)
            for name, clicks in ROUTES.items():
                page.goto(URL, wait_until='networkidle', timeout=45000)
                page.wait_for_function('document.querySelector("canvas")?.width > 0')
                # The old published mock has no Studio readiness protocol.
                # Retain a manual-review requirement; elapsed time is not proof.
                page.wait_for_timeout(2000)
                for x, y in clicks:
                    page.mouse.click(x, y)
                    page.wait_for_timeout(500)
                path = args.output / (name + '.png')
                page.screenshot(path=str(path))
                receipt['screens'].append({'id': name, 'path': path.name,
                    'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                    'inputs': [{'type': 'canvas_pointer', 'x': x, 'y': y} for x, y in clicks],
                    'observed_state_review': 'pending'})
                print(name, flush=True)
            browser.close()
    except Exception as error:
        receipt['errors'].append(str(error))
        raise
    finally:
        (args.output / 'capture-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')


if __name__ == '__main__':
    main()
