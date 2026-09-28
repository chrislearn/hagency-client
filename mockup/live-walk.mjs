// Live console walkthrough: reads {base,url,shots} JSON on stdin (the login link is never an argv or a log line).
import { createInterface } from 'node:readline';
import { chromium } from 'playwright-core';
const line = await new Promise((r) => { const rl = createInterface({ input: process.stdin }); rl.on('line', (l) => { rl.close(); r(l); }); });
const cfg = JSON.parse(line);
const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' });
const page = await (await browser.newContext({ viewport: { width: 1360, height: 900 } })).newPage();
const errs = []; page.on('pageerror', (e) => errs.push(String(e).slice(0, 160))); page.on('console', (m) => { if (m.type() === 'error') errs.push('console: ' + m.text().slice(0, 160)); });
const bad = []; page.on('response', (r) => { const u = r.url(); if (u.includes('/console/api/') && r.status() >= 400) bad.push(`${r.status()} ${u.replace(cfg.base, '').split('?')[0]}`); });
await page.goto(cfg.url, { waitUntil: 'networkidle' }); await page.waitForTimeout(1500);
const out = [];
for (const p of ['', 'agents/', 'resources/', 'engagements/', 'accounts/', 'project-sides/', 'approvals/', 'usage/', 'alerts/', 'tasks/', 'task-graphs/']) {
  errs.length = 0; bad.length = 0;
  await page.goto(`${cfg.base}/console/${p}`, { waitUntil: 'networkidle' }); await page.waitForTimeout(1500);
  const h1 = (await page.locator('h1').first().innerText().catch(() => '')).trim();
  const text = (await page.locator('main, body').first().innerText()).replace(/\s+/g, ' ');
  const accessWall = /Console access required|需要控制台访问权限/.test(text);
  const rows = await page.locator('table tbody tr').count();
  await page.screenshot({ path: `${cfg.shots}/${p.replace('/', '') || 'home'}.png`, fullPage: true });
  out.push({ page: '/console/' + p, h1, rows, accessWall, apiErrors: [...bad], jsErrors: [...errs].slice(0, 3), textLen: text.length, head: text.slice(0, 140) });
}
console.log(JSON.stringify(out, null, 1));
await browser.close();
