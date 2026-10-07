import puppeteer from 'puppeteer-core';
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';
import assert from 'node:assert/strict';

const root = process.env.HAGENCY_NATIVE_CONSOLE_ASSETS;
assert.ok(root, 'Set HAGENCY_NATIVE_CONSOLE_ASSETS to the current owner console bundle');
const server = createServer(async (req, res) => {
  try {
    let path = decodeURIComponent(new URL(req.url, 'http://localhost').pathname).replace(/^\/console\/?/, '');
    if (!path || path.endsWith('/')) path += 'index.html';
    const file = join(root, path);
    assert.ok((await stat(file)).isFile());
    res.setHeader('content-type', file.endsWith('.html') ? 'text/html' : file.endsWith('.js') ? 'application/javascript' : file.endsWith('.css') ? 'text/css' : 'application/octet-stream');
    res.end(await readFile(file));
  } catch { res.statusCode = 404; res.end(); }
}).listen(0, '127.0.0.1');
await new Promise(resolve => server.once('listening', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
const browser = await puppeteer.launch({ executablePath: process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true });
try {
  const page = await browser.newPage();
  const errors = [], mutations = [];
  let authorized = true, fail = false, mismatchedOwner = false;
  page.on('pageerror', error => errors.push(error.message));
  await page.setRequestInterception(true);
  page.on('request', async req => {
    const url = new URL(req.url());
    let value, status = 200;
    if (url.origin === 'https://matrix.example.test') { await req.respond({ status: 200, contentType: 'text/html', body: '<p>Pasion sign-in</p>' }); return; }
    if (url.pathname === '/console/server-login') value = { configured: true, localAccessReady: true, activeProfileId: 'alice', profiles: [{ profileId: 'alice', mxid: '@alice:example.test', server: 'https://matrix.example.test/' }], server: 'https://matrix.example.test/', name: 'Existing desktop', status: { state: 'device_authorized', deviceAuthorized: true } };
    else if (url.pathname.startsWith('/console/server-login/')) {
      mutations.push({ path: url.pathname, body: req.postData() ? JSON.parse(req.postData()) : null });
      await new Promise(resolve => setTimeout(resolve, 120));
      if (fail) { value = { code: 'server_unavailable' }; status = 503; }
      else if (url.pathname.endsWith('/start')) value = { url: 'https://matrix.example.test/_pasion/oauth2/authorize?prompt=login' };
      else { authorized = false; value = { activeProfileId: null, needsLogin: true }; }
    } else if (url.pathname === '/console/api/owned-agents') { value = authorized ? { ownerMxid: mismatchedOwner ? '@bob:example.test' : '@alice:example.test', agents: [], projects: [] } : { code: 'sign_in_required' }; status = authorized ? 200 : 401; }
    else if (url.pathname.startsWith('/console/api/')) { assert.equal(req.method(), 'GET', 'account menu must never submit model/provider/runtime actions'); value = { projects: [], profiles: [], state: 'signed_out', runtime: null }; }
    if (value !== undefined) await req.respond({ status, contentType: 'application/json', body: JSON.stringify(value) });
    else await req.continue();
  });
  async function enter() {
    await page.goto(`${base}/console/agents-owned/`, { waitUntil: 'networkidle0' });
    try { await page.waitForSelector('[data-owner-account-menu]', { timeout: 10000 }); } catch (failure) { console.error('Account menu missing:', await page.evaluate(() => document.body.innerText), errors); throw failure; }
    assert.ok(await page.$eval('[data-owner-account-menu]', node => node.textContent.includes('@alice:example.test') && node.textContent.includes('matrix.example.test')));
    await page.click('[data-owner-account-menu] > button');
    await page.waitForSelector('[role="menu"]');
  }
  await enter();
  const links = await page.$$eval('nav a', nodes => nodes.map(node => new URL(node.href).pathname));
  assert.deepEqual(links, ['/console/agents-owned/', '/console/projects/']);
  fail = true;
  await page.evaluate(() => { const button = document.querySelector('[data-account-action="switch"]'); button.click(); button.click(); });
  await page.waitForSelector('[data-owner-account-menu] [role="alert"]');
  assert.equal(mutations.length, 1, 'concurrent account mutation must be blocked');
  assert.equal(await page.$eval('[data-account-action="reauthenticate"]', node => node.disabled), false, 'failure must leave actions retryable');
  assert.deepEqual(mutations[0].body, { profileId: null });
  fail = false;
  await page.click('[data-account-action="reauthenticate"]');
  await page.waitForFunction(() => window.location.origin === 'https://matrix.example.test');
  assert.deepEqual(mutations.at(-1).body, { server: 'https://matrix.example.test/', name: 'Existing desktop' });
  assert.equal(mutations.filter(item => item.path.endsWith('/switch')).length, 1, 'reauthentication must not clear the selected owner');
  await enter();
  await page.click('[data-account-action="switch"]');
  await page.waitForFunction(() => window.location.pathname === '/console/login/');
  assert.equal(await page.$('[data-owner-account-menu]'), null);
  authorized = true;
  mismatchedOwner = true;
  await page.goto(`${base}/console/agents-owned/`, { waitUntil: 'networkidle0' });
  assert.equal(await page.$('[data-owner-account-menu]'), null, 'mismatched owner must not display another account identity');
  mismatchedOwner = false;
  await enter();
  await page.click('[data-account-action="signout"]');
  await page.waitForFunction(() => window.location.pathname === '/console/login/');
  assert.equal(mutations.at(-1).path, '/console/server-login/sign-out');
  assert.equal(mutations.at(-1).body, null, 'signout must use the empty-body Rust contract');
  await page.goto(`${base}/console/agents-owned/`, { waitUntil: 'networkidle0' });
  await page.waitForFunction(() => window.location.pathname === '/console/login/');
  assert.equal(await page.$('[data-owner-account-menu]'), null, 'global installation status cannot expose an authenticated account menu');
  assert.deepEqual(errors, []);
  console.log('PASS Owner account menu: verified identity, Agents/Projects navigation, concurrent mutation rejection, visible failures, pinned Pasion reauthentication, account switch and signout, anonymous hidden; no model calls');
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
