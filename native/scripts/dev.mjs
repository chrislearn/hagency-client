// Local development: the Rust host serves the real, exported console.
import { spawn } from 'node:child_process';
import { watch } from 'node:fs';
import { access, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values } = parseArgs({ options: {
  'state-dir': { type: 'string', default: '.run/dev-state' },
  listen: { type: 'string', default: '127.0.0.1:13300' },
  'font-cache': { type: 'string', default: '.run/font-cache' },
} });
const state = resolve(root, values['state-dir']);
const binary = join(root, 'target/debug/hagency');
let host, build, assets, stopping = false, busy = false, timer;
let rustChanged = true, consoleChanged = true;
const watchers = [];
process.umask(0o077);
await mkdir(join(root, '.run'), { recursive: true, mode: 0o700 });

function run(command, args) {
  return new Promise((done, reject) => {
    const child = build = spawn(command, args, { cwd: root, stdio: 'inherit' });
    child.once('error', reject);
    child.once('exit', code => {
      if (build === child) build = undefined;
      code === 0 ? done() : reject(new Error(`${command} exited with ${code}`));
    });
  });
}
async function stop(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  await new Promise(done => {
    const timeout = setTimeout(() => child.kill('SIGKILL'), 30000);
    child.once('exit', () => { clearTimeout(timeout); done(); });
    child.kill('SIGTERM');
  });
}
async function rebuild() {
  if (busy || stopping) return;
  busy = true;
  const rust = rustChanged, frontend = consoleChanged;
  rustChanged = consoleChanged = false;
  try {
    let nextAssets = assets;
    if (frontend) {
      nextAssets = join(root, '.run', `dev-console-${Date.now()}`);
      const args = ['mockup/scripts/build-native-console.mjs', '--output', nextAssets];
      const cache = resolve(root, values['font-cache']);
      try { await access(join(cache, 'static/chunks')); args.push('--font-cache', cache); }
      catch { /* On a fresh checkout, the build downloads the actual font files. */ }
      await run(process.execPath, args);
    }
    if (rust) await run('cargo', ['build', '--locked', '-p', 'hagency']);
    if (stopping) return;
    try { await access(join(state, 'operator.token')); }
    catch { await run(binary, ['init', '--state-dir', state]); }
    await stop(host);
    if (stopping) return;
    assets = nextAssets;
    host = spawn(binary, ['serve', '--state-dir', state, '--listen', values.listen,
      '--palpo-transport', '--console-assets', assets], { cwd: root, stdio: 'inherit' });
    host.once('error', error => consoleError(error));
    host.once('exit', code => {
      if (!stopping && code !== 0) consoleError(new Error(`Rust host exited with ${code}; edit source to retry.`));
    });
    consoleLog(`Client: http://${values.listen}/console/ — run just console for the initial sign-in link.`);
  } catch (error) {
    consoleError(error);
    // A failed build never replaces the running host or its validated console.
    if (!host) { await shutdown(); process.exitCode = 1; }
  } finally {
    busy = false;
    if (!stopping && (rustChanged || consoleChanged)) schedule();
  }
}
const consoleLog = message => console.log(message);
const consoleError = error => console.error(error.message);
function schedule() {
  clearTimeout(timer);
  timer = setTimeout(rebuild, 500);
}
function observe(path, recursive, changed) {
  const watcher = watch(join(root, path), { recursive }, (_, name) => {
    if (!name || changed(String(name))) schedule();
  });
  watcher.on('error', consoleError);
  watchers.push(watcher);
}
async function shutdown() {
  if (stopping) return;
  stopping = true;
  clearTimeout(timer);
  watchers.forEach(watcher => watcher.close());
  await Promise.all([stop(build), stop(host)]);
}
process.once('SIGINT', shutdown);
process.once('SIGTERM', shutdown);
observe('native', true, name => {
  if (/^[^/]+\/(src\/|Cargo\.toml$|build\.rs$|role-capacity\.json$)/.test(name)) {
    rustChanged = true;
    if (name.endsWith('role-capacity.json')) consoleChanged = true;
    return true;
  }
  return false;
});
observe('.', false, path => {
  if (!['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'].includes(path)) return false;
  rustChanged = true;
  if (path === 'Cargo.toml') consoleChanged = true;
  return true;
});
observe('mockup', false, path => {
  if (!['package.json', 'package-lock.json', 'next.config.mjs', 'jsconfig.json'].includes(path)) return false;
  consoleChanged = true;
  return true;
});
for (const path of ['mockup/app', 'mockup/components', 'mockup/lib', 'mockup/scripts']) {
  observe(path, true, () => { consoleChanged = true; return true; });
}
await rebuild();
