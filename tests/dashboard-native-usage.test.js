import { describe, expect, test } from 'vitest';
import { renderDashboard } from './helpers/dashboard-render.js';
import { selection, fetchFleetUsage, validateAllocationReply, validateEngagements, validateReport, validateSideBudget } from '../mockup/lib/native-api.js';

const counts = { input: 4, output: 1, cacheWrite: 0, cacheRead: 1 };
const report = { engagement_id: 'created_after_build', at_ms: 2000, summary: { sources: 1, latest_counts: counts, known_high_water_lower_bound: { ...counts, input: 7 }, latest_incomplete_sources: 0, historically_incomplete_sources: 1, regression_observations: 1, evidence: 'host_attributed_untrusted_usage' }, daily: null, monthly: null, ceiling: { tokens_drawn: 0, tokens_used: null, remaining_tokens: null } };
describe('retained usage native mode', () => {
  test('native observations preserve nulls lower bounds and evidence', async () => {
    expect(validateReport(report, 'created_after_build')).toBe(report);
    for (const bad of [{ ...report, engagement_id: 'wrong' }, { ...report, billingVerified: true }, { ...report, summary: { ...report.summary, latest_counts: { ...counts, input: NaN } } }]) expect(() => validateReport(bad, 'created_after_build')).toThrow();
    const html = await renderDashboard('mockup/app/usage/page.jsx', { data: { nativeConsole: true, phase: 'ready', report, engagements: [], selected: report.engagement_id } });
    expect(html).toContain('Historical high-water lower bounds');
    expect(html).toContain('No observation for this period');
    expect(html).not.toContain('NaN');
  });
  test('unknown native state cannot become a fixture or zero response', async () => {
    const html = await renderDashboard('mockup/app/usage/page.jsx', { data: { nativeConsole: true, phase: 'access', report: null, engagements: [], selected: null } });
    expect(html).toContain('Console access required');
    expect(html).not.toContain('data-kind');
    expect(() => selection({ search: '?data=fixture' })).toThrow();
    expect(() => selection({ search: '?engagement_id=a&engagement_id=b' })).toThrow();
    expect(selection({ search: '?engagement_id=created_after_build' })).toBe('created_after_build');
    expect(() => validateEngagements({ engagements: [], next_after: null, token: 'not_allowed' })).toThrow();
  });
  /*
   * Board #108 (a): the side budget is served TWO ways and the client pinned
   * only one. The GET spreads the budget FLAT beside `ok`/`sideId`
   * (backend-v2.js:9571) — ten keys — while `budgetFields` demanded an object
   * of exactly eight, so `validateSideBudget` threw on EVERY budget read.
   * `fetchSideBudget` therefore never resolved, `fetchFleetUsage` rejected as a
   * whole, and the usage page could only ever render "Fleet usage could not be
   * read". The value check must be key-count independent; the exact SET is
   * pinned per form (ten on the flat read, eight on the nested write).
   */
  test('the side budget GET is validated flat, the PUT nested', () => {
    const commitments = [{ id: 'en_one', agent: 'UsageWorker', role: 'coding', project: 'project-one', projectName: null, allocatedTokens: 100, agentExists: true }];
    const fields = { allocated: 500, committed: 200, remaining: 300, commitments, poolCommitments: [], poolCommitted: 0, totalCommitted: 200, orphanedCommitted: 0 };
    const flat = { ok: true, sideId: 'example.test', ...fields };
    expect(validateSideBudget(flat, 'example.test')).toBe(flat);
    expect(() => validateSideBudget({ ...flat, sideId: 'other.test' }, 'example.test')).toThrow();
    expect(() => validateSideBudget({ ...flat, extra: 1 }, 'example.test')).toThrow();
    expect(() => validateSideBudget({ ...flat, committed: 'many' }, 'example.test')).toThrow();
    const nested = { ok: true, side: { id: 'example.test', representative: '@r:example.test', generation: 1, registered: true, reception_room_id: '!r:example.test', projects: [] }, budget: { ...fields, allocated: null, remaining: null } };
    expect(validateAllocationReply(nested)).toBe(nested);
    expect(() => validateAllocationReply({ ...nested, budget: { ...nested.budget, extra: 1 } })).toThrow();
    /*
     * A commitment's projectName is bounded in Unicode SCALAR values, like
     * validateEngagements' own bound and the server's `trim().chars().take(255)`
     * (authority.rs:286). The live fleet carries a 255-astral-character name
     * (the fixture's AlertWorker), which is 510 UTF-16 units — a `.length`
     * bound refused every budget read and took the whole fleet panel down.
     */
    const astral = { ...commitments[0], projectName: '𝕏'.repeat(255) };
    const withAstral = { ok: true, sideId: 'example.test', ...fields, commitments: [astral] };
    expect(validateSideBudget(withAstral, 'example.test')).toBe(withAstral);
    expect(() => validateSideBudget({ ...withAstral, commitments: [{ ...astral, projectName: '𝕏'.repeat(256) }] }, 'example.test')).toThrow();
  });
  /*
   * Bug 4, the one that hid behind the others: `fetchFleetUsage` returned
   * `sideBudgets` while the panel spreads that object into its state and then
   * indexes `state.budgets[side.id]`. On a SUCCESSFUL load the spread left
   * `budgets` undefined and the fleet table threw
   * `TypeError: Cannot read properties of undefined` — a whole-page crash no
   * fast test saw, because the budget read above always failed first and drove
   * the panel down its error branch instead. This pins the returned key SET, so
   * the wire shape and the consumer cannot drift apart again.
   */
  test('the fleet composition returns the key its panel indexes', async () => {
    const totals = { ok: true, totals: { agents: 1, tokensDrawn: 5, tokensUsed: 9, tokensMeasuredFor: 1, tokensPartial: false }, unavailable: [] };
    const sides = { at_ms: 1, unavailable: [], sides: [{ id: 'example.test', representative: '@r:example.test', generation: 1, registered: true, reception_room_id: '!r:example.test', projects: [] }] };
    const budget = { ok: true, sideId: 'example.test', allocated: null, committed: 200, remaining: null, commitments: [], poolCommitments: [], poolCommitted: 0, totalCommitted: 200, orphanedCommitted: 0 };
    const routes = { '/console/api/usage/totals': totals, '/console/api/project-sides': sides, '/console/api/project-sides/example.test/budget': budget };
    const reply = (body) => ({ ok: true, status: 200, headers: { get: (h) => (h === 'content-type' ? 'application/json' : null) }, body: { getReader: () => { let sent = false; return { read: async () => sent ? { done: true } : (sent = true, { done: false, value: new TextEncoder().encode(JSON.stringify(body)) }), cancel() {} }; } } });
    const original = globalThis.fetch;
    globalThis.fetch = async (url) => {
      const path = String(url).replace(/^\/console/, '/console');
      if (!(path in routes)) throw new Error(`unexpected request ${path}`);
      return reply(routes[path]);
    };
    try {
      const value = await fetchFleetUsage();
      expect(Object.keys(value).sort()).toEqual(['budgets', 'sides', 'totals']);
      expect(value.budgets['example.test'].committed).toBe(200);
      expect(value.sides.map((s) => s.id)).toEqual(['example.test']);
    } finally {
      globalThis.fetch = original;
    }
  });
});
