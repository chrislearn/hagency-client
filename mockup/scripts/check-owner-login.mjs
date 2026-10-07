import puppeteer from 'puppeteer-core';
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';
import assert from 'node:assert/strict';

const root = process.env.HAGENCY_NATIVE_CONSOLE_ASSETS;
assert.ok(root, 'Set HAGENCY_NATIVE_CONSOLE_ASSETS from build:native');
const key = 'hagency.recent-server-origins.v1';
const errors = [], starts = [], switches = [];
let profiles = [], activeProfileId = null;
let switchRejected = false, localAccessReady = true;
let configured = false, authorized = false, serverAddress = null, deviceName = 'Hagency Client', state = 'signed_out';
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
async function open(storage) {
  const page = await browser.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await page.evaluateOnNewDocument((historyKey, saved) => {
    if (!sessionStorage.getItem('owner-login-test-seeded')) {
      localStorage.setItem(historyKey, saved);
      sessionStorage.setItem('owner-login-test-seeded', 'true');
    }
    window.addEventListener('hagency-owner-auth-changed', event => {
      const events = JSON.parse(sessionStorage.getItem('owner-login-auth-events') || '[]');
      events.push(event.detail?.authorized);
      sessionStorage.setItem('owner-login-auth-events', JSON.stringify(events));
    });
  }, key, storage);
  await page.setRequestInterception(true);
  page.on('request', async req => {
    const url = new URL(req.url());
    let body, status = 200;
    if (url.pathname === '/console/server-login') body = { localAccessReady, configured, server: serverAddress, name: deviceName, profiles, activeProfileId, status: { state, deviceAuthorized: state === 'device_authorized', transportOnline: false } };
    else if (url.pathname === '/console/api/owned-agents') {
      status = authorized ? 200 : 401;
      body = authorized ? { ownerMxid: '@owner:example.test', agents: [], projects: [] } : { code: 'sign_in_required' };
    } else if (url.pathname === '/console/server-login/start') {
      assert.equal(req.method(), 'POST');
      const value = JSON.parse(req.postData()); starts.push(value);
      body = { url: `${value.server}/_pasion/authorize?state=stub&prompt=login` };
    } else if (url.pathname === '/console/server-login/switch') {
      assert.equal(req.method(), 'POST');const value=JSON.parse(req.postData());assert.deepEqual(Object.keys(value),['profileId']);switches.push(value);
      if (switchRejected) return req.respond({status:403,contentType:'application/json',body:JSON.stringify({code:'local_access_required'})});
      const profile=profiles.find(p=>p.profileId===value.profileId);assert.ok(value.profileId===null || profile);
      activeProfileId=value.profileId;configured=activeProfileId!==null;authorized=false;state='signed_out';serverAddress=profile?.server || null;deviceName=profile?.name || 'Hagency Client';
      body={activeProfileId,needsLogin:true,server:serverAddress};
    } else if (url.pathname === '/console/server-login/sign-out') {
      assert.equal(req.method(), 'POST'); authorized = false; state = 'signed_out'; body = {};
    } else if (url.pathname === '/console/api/owner-projects') body = { projects: [] };
    else if (url.pathname === '/console/api/owner-provider') body = { state: 'signed_out', authenticated: false };
    else if (url.pathname.startsWith('/console/api/')) {
      assert.ok(!url.pathname.includes('/runtime/start'), 'sign-in never starts inference');
      status = 401; body = { code: 'sign_in_required' };
    } else if (url.origin !== base) return req.respond({ status: 200, contentType: 'text/html', body: '<p>Stub Pasion sign-in</p>' });
    if (body !== undefined) return req.respond({ status, contentType: 'application/json', body: JSON.stringify(body) });
    return req.continue();
  });
  await page.goto(`${base}/console/login/`, { waitUntil: 'networkidle0' });
  await page.waitForSelector('[data-server-login]');
  return page;
}
async function chooseServer(page, value) {
  await page.$eval('#hagency-server', node => { node.focus(); node.select(); });
  await page.keyboard.press('Backspace');
  await page.waitForResponse(response=>new URL(response.url()).pathname==='/console/server-login');
  await page.waitForNetworkIdle({idleTime:100});
  assert.equal(await page.$eval('#hagency-server',node=>node.value),'','polling cannot refill a field the user is editing');
  await page.type('#hagency-server', value);
  await page.keyboard.press('Escape');
  await page.keyboard.press('Tab');
  assert.equal(await page.$eval('#hagency-server',node=>node.value),value);
}

try {
  // Shared installation status never grants a browser a Matrix session.
  configured = true; authorized = false; state = 'device_authorized'; serverAddress = 'https://bound.example.test'; deviceName = 'Previously named device';
  let page = await open('[]');
  await page.waitForFunction(() => document.querySelector('[data-server-login]').textContent.includes('Authorization expired'));
  assert.equal(new URL(page.url()).pathname, '/console/login/');
  assert.equal(await page.$eval('#hagency-server', node => node.disabled), false);
  assert.equal(await page.$('#device-name'), null);
  assert.ok(await page.$('[data-server-history]'));
  assert.deepEqual(await page.evaluate(k => JSON.parse(localStorage.getItem(k)), key), [], 'no authenticated browser means no successful address recorded');
  assert.equal(await page.$('.rail'), null, 'login has its own screen without workspace navigation');
  assert.ok(await page.$eval('[data-server-login]', node => node.textContent.includes('Server address')));
  await page.setViewport({ width: 390, height: 844 });
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'phone login does not overflow');
  const bounds = await page.$eval('[data-server-login]', node => ({x:node.getBoundingClientRect().x,width:node.getBoundingClientRect().width}));
  assert.ok(Math.abs(bounds.x - (390-bounds.width)/2) < 2, 'login is centered');
  await page.setViewport({ width: 1200, height: 800 });
  assert.ok(await page.$eval('[data-server-login]', node => node.getBoundingClientRect().width <= 400));
  await page.click('[data-server-login] button[type="submit"]');
  await page.waitForFunction(() => location.hostname === 'bound.example.test');
  assert.deepEqual(starts.at(-1), { server: 'https://bound.example.test', name: 'Hagency Client' }, 'Pasion chooses the account after the saved identity is cleared');
  await page.close();

  // Read history defensively, strip all credentials/path/query/fragment, dedupe and bound it.
  configured = false; authorized = false; state = 'signed_out'; serverAddress = null;
  const saved = ['https://user:password@recent.example.test/private?token=secret#fragment', 'https://recent.example.test/', 'http://192.0.2.1', 'javascript:secret', ...Array.from({ length: 10 }, (_, n) => `https://recent-${n}.example.test/path`)];
  page = await open(JSON.stringify(saved));
  await page.waitForSelector('[data-server-history]');
  const addresses = await page.evaluate(k => JSON.parse(localStorage.getItem(k)), key);
  assert.equal(addresses.length, 8); assert.equal(addresses[0], 'https://recent.example.test');
  assert.ok(addresses.every(value => !value.includes('password') && !value.includes('secret') && new URL(value).origin === value));
  await chooseServer(page, 'https://recent.example.test');
  assert.equal(await page.$eval('#hagency-server', node => node.value), 'https://recent.example.test');
  assert.equal(await page.$('#device-name'), null);
  await page.click('[data-server-login] button[type="submit"]');
  await page.waitForFunction(() => location.hostname === 'recent.example.test');
  assert.deepEqual(starts.at(-1), { server: 'https://recent.example.test', name: 'Hagency Client' });
  await page.close();

  // Corrupt optional storage cannot prevent sign-in or permit unsafe HTTP origins.
  page = await open('{broken');
  assert.equal(await page.$('[data-server-history]'), null);
  assert.deepEqual(await page.evaluate(k => JSON.parse(localStorage.getItem(k)), key), []);
  await page.type('#hagency-server', 'http://192.0.2.1/private');
  const before = starts.length;
  await page.click('[data-server-login] button[type="submit"]');
  await page.waitForSelector('[data-server-login] [role="alert"]');
  assert.equal(starts.length, before, 'unsafe HTTP address is rejected before sending');
  await page.close();

  // Login completion requires an owner API grant; record only origin then enter the client.
  page = await open(JSON.stringify(Array.from({ length: 8 }, (_, n) => `https://old-${n}.example.test`)));
  await page.waitForFunction(() => document.querySelector('[data-server-login]').dataset.loginState === 'signed_out');
  configured = true; state = 'device_authorized'; authorized = true; serverAddress = 'https://user:password@new.example.test/private?token=secret#fragment';
  await page.waitForFunction(() => location.pathname === '/console/agents-owned/', { timeout: 15000 });
  const recent = await page.evaluate(k => JSON.parse(localStorage.getItem(k)), key);
  assert.equal(recent[0], 'https://new.example.test'); assert.equal(recent.length, 8);
  assert.ok(!JSON.stringify(recent).includes('secret') && !JSON.stringify(recent).includes('password'));
  assert.ok(await page.evaluate(() => JSON.parse(sessionStorage.getItem('owner-login-auth-events')).includes(true)), 'verified login notifies the global gate');
  await page.close();

  // Already signed-in users may visit login to sign out; initial polling must not redirect them.
  serverAddress = 'https://bound.example.test'; deviceName = 'Previously named device';
  page = await open('[]');
  await page.waitForFunction(() => [...document.querySelectorAll('[data-server-login] button')].some(node => node.textContent === 'Sign out on this device'));
  assert.equal(new URL(page.url()).pathname, '/console/login/');
  assert.equal(await page.$eval('[data-server-login] a', node => new URL(node.href).pathname), '/console/agents-owned/');
  const eventCount = await page.evaluate(() => JSON.parse(sessionStorage.getItem('owner-login-auth-events')).length);
  await page.evaluate(() => [...document.querySelectorAll('[data-server-login] button')].find(node => node.textContent === 'Sign out on this device').click());
  await page.waitForFunction(() => document.querySelector('[data-server-login]').dataset.loginState === 'signed_out');
  const events = await page.evaluate(() => JSON.parse(sessionStorage.getItem('owner-login-auth-events')));
  assert.ok(events.length > eventCount); assert.equal(events.at(-1), false);
  await page.close();

  // The selector is ONLY server origins, deduplicated even with multiple saved
  // owners on the same server. No saved username is promised to Pasion.
  profiles=[{profileId:'profile-a',server:'https://bound.example.test',issuer:'https://bound.example.test/_pasion/',mxid:'@alice:example.test',name:'Alice device'},{profileId:'profile-b',server:'https://second.example.test',issuer:'https://second.example.test/_pasion/',mxid:'@bob:second.test',name:'Bob device'},{profileId:'profile-c',server:'https://bound.example.test',issuer:'https://bound.example.test/_pasion/',mxid:'@carol:example.test',name:'Carol device'}];
  activeProfileId='profile-a';configured=true;authorized=true;state='device_authorized';serverAddress=profiles[0].server;deviceName=profiles[0].name;
  page=await open(JSON.stringify(['https://second.example.test','https://bound.example.test']));
  await page.waitForSelector('[data-server-history]');
  const choices=await page.$$eval('[data-server-history] option',options=>options.map(o=>o.textContent));
  assert.equal(choices.filter(x=>x==='https://bound.example.test').length,1);
  assert.ok(choices.includes('https://second.example.test'));
  assert.equal(await page.$('[data-owner-profile]'),null);
  assert.ok(await page.$eval('[data-server-login]',node=>!node.textContent.includes('@alice')&&!node.textContent.includes('@bob')&&!node.textContent.includes('@carol')));
  await chooseServer(page,'https://second.example.test');
  await page.click('[data-server-login] button[type="submit"]');
  await page.waitForFunction(()=>location.hostname==='second.example.test');
  assert.deepEqual(switches.at(-1),{profileId:null});assert.equal(authorized,false);
  assert.deepEqual(starts.at(-1),{server:'https://second.example.test',name:'Hagency Client'});
  assert.equal(new URL(page.url()).searchParams.get('prompt'),'login');
  assert.equal(profiles.length,3,'server choice retains all independent saved identities');
  await page.close();

  // Same-server account changes must also clear the pinned identity first;
  // actual authenticated Pasion identity decides which isolated data is opened.
  activeProfileId='profile-a';configured=true;authorized=false;state='signed_out';serverAddress=profiles[0].server;
  page=await open('[]');await page.click('[data-server-login] button[type="submit"]');
  await page.waitForFunction(()=>location.hostname==='bound.example.test');
  assert.deepEqual(switches.at(-1),{profileId:null});assert.equal(activeProfileId,null);
  assert.deepEqual(starts.at(-1),{server:'https://bound.example.test',name:'Hagency Client'});await page.close();

  // Failure to stop/revoke the old account must never begin another login.
  activeProfileId='profile-a';configured=true;authorized=false;state='signed_out';serverAddress=profiles[0].server;switchRejected=true;
  page=await open('[]');const startCount=starts.length;
  await page.click('[data-server-login] button[type="submit"]');await page.waitForSelector('[role="alert"]');
  assert.equal(starts.length,startCount);assert.equal(activeProfileId,'profile-a');assert.equal(new URL(page.url()).hostname,'127.0.0.1');
  switchRejected=false;await page.close();

  // Signed-in users can explicitly use another account on the same server.
  authorized=true;state='device_authorized';
  page=await open('[]');await page.waitForSelector('[data-add-owner-profile]');await page.click('[data-add-owner-profile]');
  await page.waitForFunction(()=>document.querySelector('[data-server-login]').dataset.loginState==='signed_out');
  assert.deepEqual(switches.at(-1),{profileId:null});assert.equal(authorized,false);
  assert.equal(await page.$eval('#hagency-server',node=>node.value),'https://bound.example.test');
  await page.click('[data-server-login] button[type="submit"]');await page.waitForFunction(()=>location.hostname==='bound.example.test');
  assert.deepEqual(starts.at(-1),{server:'https://bound.example.test',name:'Hagency Client'});await page.close();
  // A registered server allows Pasion re-authentication even after local
  // authority expires; no red warning or disabled button for normal re-login.
  localAccessReady=false;authorized=false;state='signed_out';configured=true;activeProfileId='profile-a';serverAddress=profiles[0].server;
  page=await open('[]');
  assert.equal(await page.$('[role="alert"]'),null);
  assert.equal(await page.$eval('button[type="submit"]',node=>node.disabled),false);
  const switchCount=switches.length;
  await page.click('button[type="submit"]');await page.waitForFunction(()=>location.hostname==='bound.example.test');
  assert.equal(switches.length,switchCount,'expired local cookie cannot switch or grant itself local access');
  assert.deepEqual(starts.at(-1),{server:'https://bound.example.test',name:'Hagency Client'});await page.close();
  // A new, unregistered server still needs initial local host admission.
  page=await open('[]');await chooseServer(page,'https://not-admitted.example.test');
  await page.waitForSelector('[role="alert"]');
  assert.equal(await page.$eval('button[type="submit"]',node=>node.disabled),true);
  assert.equal(await page.$$eval('#hagency-server',nodes=>nodes.length),1);
  localAccessReady=true;await page.close();
  assert.deepEqual(errors, []);
  console.log('PASS owner login browser contract: Matrix grant required, safe bounded server history, default device name, corrupt storage, success navigation and auth notifications, usable sign-out page, deduplicated server-only history without usernames, old identity cleared before same/different-server Pasion login, failed revocation blocks login; no inference');
} catch (failure) {
  console.error(JSON.stringify({starts,switches,pages:await Promise.all((await browser.pages()).map(async page=>({url:page.url(),text:await page.evaluate(()=>document.body.innerText)})))}));
  throw failure;
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
