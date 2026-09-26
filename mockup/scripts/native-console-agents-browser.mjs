/*
 * Served-binary agent lifecycle walk (board #106): the operator's stop → Start
 * → start journey through the UI only.
 *
 * Why this exists. The live run (#102 step 4) stopped an agent and then could
 * not bring it back: the roster kept rendering Stop and never a Start control,
 * and the row still read the engagement's own word, so nothing on screen said
 * the agent was stopped. The console page is the only thing that talks to the
 * service here (no step calls an API), because "the route exists" and "the
 * operator can reach it" are different claims and only a real press proves the
 * second.
 *
 * Protocol, exactly the sibling walks': one JSON line on stdin
 * {base, url, engagement}. The access link is never an argv, never printed,
 * never logged — the harness that minted it owns it.
 */
import assert from 'node:assert/strict';
import { createInterface } from 'node:readline';
import { chromium } from 'playwright-core';

const line = await new Promise((resolve) => {
  const rl = createInterface({ input: process.stdin });
  rl.on('line', (value) => { rl.close(); resolve(value); });
});
const config = JSON.parse(line);

const browser = await chromium.launch({
  executablePath: process.env.HAGENCY_BROWSER_CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: true,
  args: ['--disable-background-networking', '--disable-component-update', '--no-default-browser-check'],
});
try {
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const page = await context.newPage();
  const failures = [];
  page.on('pageerror', (error) => failures.push(error.message));

  await page.goto(config.url);
  await page.locator('[data-native-state="ready"]').first().waitFor();
  await page.goto(`${config.base}/console/agents/`);
  await page.locator('[data-native-state="ready"]').first().waitFor();

  const row = page.locator(`[data-engagement-id="${config.engagement}"]`);
  await row.waitFor({ timeout: 15_000 });
  // Before the stop: the operator's one available transition is Stop.
  assert.equal(await row.locator('[data-lifecycle-action="start"]').count(), 0, 'a serving agent offers no Start');
  assert.equal(await row.locator('[data-lifecycle-action="stop"]').count(), 1, 'a serving agent offers Stop');

  // STOP.
  await row.locator('[data-lifecycle-action="stop"]').click();
  await page.locator('[data-stop-action="saved"]').waitFor({ timeout: 20_000 });

  // THE #106 DEFECT: after the accepted stop the row must offer the way BACK.
  // The roster re-read is awaited by the page itself, so `start` appearing is
  // the operator-visible proof that the stop reached the roster's own state.
  const start = row.locator('[data-lifecycle-action="start"]');
  try {
    await start.waitFor({ timeout: 20_000 });
  } catch (error) {
    const text = await row.innerText().catch(() => '(no row)');
    throw new Error(`the agent was stopped but the row offers no Start control; row says: ${JSON.stringify(text)} — ${error.message}`);
  }
  assert.equal(await row.locator('[data-lifecycle-action="stop"]').count(), 0, 'a stopped agent no longer offers Stop');
  // The Liveness column is the fifth cell (agent, framework, role, state,
  // liveness); it must say the agent is stopped rather than Unknown.
  const liveness = (await row.locator('td').nth(4).innerText()).trim();
  assert.match(liveness, /stopped|已停止/i, `the stopped row says so in Liveness, got ${JSON.stringify(liveness)}`);

  // START — the route the live operator could not reach.
  await start.click();
  await page.locator('[data-start-action="saved"]').waitFor({ timeout: 20_000 });

  // Back in service: Stop is offered again and the liveness word is no longer
  // the stopped one.
  await row.locator('[data-lifecycle-action="stop"]').waitFor({ timeout: 20_000 });
  const resumed = (await row.locator('td').nth(4).innerText()).trim();
  assert.doesNotMatch(resumed, /^\s*(Unknown|未知)\s*$/, 'a restarted agent has a real liveness word, not unknown');
  assert.doesNotMatch(resumed, /stopped|已停止/i, `the restarted row is not stopped, got ${JSON.stringify(resumed)}`);

  assert.deepEqual(failures, [], `console errors: ${failures.join(' | ')}`);
  console.log('PASS served-binary agent stop-then-start walk');
} finally {
  await browser.close();
}
process.exit(0);
