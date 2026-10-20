"""Generate a signed stable Windows feed from the collected NSIS installer."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def generate(assets, version, notes):
    if '-' in version:
        raise ValueError('The stable updater feed must not publish prereleases')
    installer = assets / 'hagency-windows-amd64.exe'
    subprocess.run(['cargo', 'packager', 'signer', 'sign', str(installer)], check=True)
    with installer.open('rb') as source:
        digest = hashlib.file_digest(source, 'sha256').hexdigest()
    notes = notes.encode('utf-8')[:12000].decode('utf-8', errors='ignore')
    manifest = {'version': version, 'notes': notes, 'platforms': {'windows-x86_64': {
        'url': f'https://github.com/hagency-org/hagency-client/releases/download/v{version}/{installer.name}',
        'signature': installer.with_suffix('.exe.sig').read_text().strip(),
        'size': installer.stat().st_size, 'sha256': digest, 'format': 'nsis',
    }}}
    output = assets / 'windows-update.json'
    output.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    subprocess.run(['cargo', 'packager', 'signer', 'sign', str(output)], check=True)
    subprocess.run(['cargo', 'run', '--locked', '-p', 'hagency-updater', '--example', 'verify', '--', str(assets)], cwd=ROOT, check=True)


if __name__ == '__main__':
    if not os.environ.get('HAGENCY_UPDATER_PUBLIC_KEY') or not os.environ.get('CARGO_PACKAGER_SIGN_PRIVATE_KEY'):
        raise SystemExit('Windows updates require the public key and signing secret')
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    generate(ROOT / 'release-assets', version, (ROOT / 'packaging/release-notes.md').read_text(encoding='utf-8'))
