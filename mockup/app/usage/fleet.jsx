'use client';

/*
 * ⑤ 用量 — the fleet half of the page (#20): the totals block with its own
 * denominator, and each side's allocation against its commitments — the
 * acceptance is "operator sets a side allocation and sees spend vs
 * allocation". NULL allocated renders as UNALLOCATED, never as unlimited
 * (lib/project-side-store.js:443-452); a null fleet figure renders as
 * unknown, never as zero (backend-v2.js:15702). The busy-time and task
 * columns the retained block carried are named by the server's own
 * `unavailable` list and stay unknown here.
 *
 * Self-contained on purpose: the engagement evidence rides Data's own load;
 * this panel owns its snapshot so the two cannot tear mid-render.
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { fetchFleetUsage, setSideAllocation } from '@/lib/native-api';
import { useData } from '@/components/Data';
import { useT } from '@/components/Prefs';

const SIDES_WITH_BUDGET = 16;

export default function FleetUsagePanel() {
  const t = useT();
  const data = useData();
  const [state, setState] = useState({ phase: 'loading', totals: null, sides: [], budgets: {}, error: null });
  const [edits, setEdits] = useState({});
  const [saving, setSaving] = useState(null);
  const [notice, setNotice] = useState(null);
  const load = useCallback(async () => {
    try {
      const value = await fetchFleetUsage();
      setState({ phase: 'ready', error: null, ...value });
    } catch (error) {
      // The page's own access notice already covers the access phase.
      if (error.message === 'console_access_required') return;
      setState((s) => ({ ...s, phase: 'error', error: error.message }));
    }
  }, []);
  /* Ride the provider's admission: before the ticket is exchanged — and
   * again after End access — the panel must not fire its reads with a
   * dead credential; the provider's own phase is the gate. */
  useEffect(() => {
    if (data.phase === 'ready' || data.phase === 'stale') void load();
  }, [load, data.phase]);
  /* Read the provider's phase when the event fires, not the phase this
   * listener was registered with: between End access and the next effect
   * pass, a listener holding the old 'ready' would send a dead credential. */
  const phase = useRef(data.phase);
  phase.current = data.phase;
  useEffect(() => {
    const refresh = () => { if (document.visibilityState === 'visible' && (phase.current === 'ready' || phase.current === 'stale')) void load(); };
    window.addEventListener('focus', refresh);
    document.addEventListener('visibilitychange', refresh);
    return () => { window.removeEventListener('focus', refresh); document.removeEventListener('visibilitychange', refresh); };
  }, [load]);
  if (data.phase === 'access') return null;
  if (state.phase === 'loading') return <section className="panel" data-fleet-state="loading"><p role="status">{t('nu.loading')}</p></section>;
  if (state.phase === 'error') return <section className="panel" role="alert" data-fleet-state="error"><h2 className="sec" style={{ marginTop: 0 }}>{t('us.fleetFailed')}</h2>
    <p>{t('nu.retryHelp')}</p><button className="btn" onClick={() => void load()}>{t('nu.refresh')}</button></section>;
  const totals = state.totals?.totals;
  const fmt = (v) => v == null ? t('us.unknown') : v.toLocaleString();
  const save = async (side, raw) => {
    const text = String(raw ?? '').trim();
    let value = null;
    if (text !== '') {
      if (!/^\d+$/.test(text) || !Number.isSafeInteger(Number(text))) { setNotice({ side, kind: 'invalid' }); return; }
      value = Number(text);
    }
    setSaving(side);
    setNotice(null);
    try {
      /* Absent allocated_tokens means the CLEAR — null is unallocated,
       * which is not unlimited; zero is a real allocation of nothing. */
      const reply = await setSideAllocation(side, value);
      setState((s) => ({ ...s, budgets: { ...s.budgets, [side]: reply.budget } }));
      setNotice({ side, kind: 'saved' });
    } catch (error) {
      setNotice({
        side,
        kind: error.message === 'resource_configuration_scope_required' ? 'scope'
          : error.message === 'not_found' ? 'not_found'
            : error.message === 'busy' ? 'busy' : 'failed',
      });
    } finally { setSaving(null); }
  };
  return <>
    <section className="panel" data-fleet-state="ready">
      <h2 className="sec" style={{ marginTop: 0 }}>{t('us.fleet')}</h2>
      {totals ? <dl>
        <div className="kv"><dt>{t('us.fleetAgents')}</dt><dd>{totals.agents.toLocaleString()}</dd></div>
        <div className="kv"><dt>{t('us.fleetDrawn')}</dt><dd data-fleet="drawn">{fmt(totals.tokensDrawn)}</dd></div>
        <div className="kv"><dt>{t('us.fleetUsed')}</dt><dd data-fleet="used">{fmt(totals.tokensUsed)}</dd></div>
        <div className="kv"><dt>{t('us.fleetMeasured')}</dt><dd data-fleet="measured">{t('us.fleetMeasuredFor', { n: totals.tokensMeasuredFor, agents: totals.agents })}</dd></div>
        <div className="kv"><dt>{t('us.fleetPartial')}</dt><dd data-fleet="partial">{t(totals.tokensPartial ? 'us.partialYes' : 'us.partialNo')}</dd></div>
        {state.totals?.unavailable?.map((name) => <div className="kv" key={name}><dt>{t(`us.gap.${name}`)}</dt><dd>{t('us.unknown')}</dd></div>)}
      </dl> : <p>{t('us.unknown')}</p>}
    </section>
    <section className="panel">
      <h2 className="sec" style={{ marginTop: 0 }}>{t('us.sideAlloc')}</h2>
      {state.sides.length === 0 ? <p>{t('us.noSides')}</p> : <table>
        <thead><tr><th>{t('us.side')}</th><th>{t('us.allocated')}</th><th>{t('us.committed')}</th><th>{t('us.remaining')}</th><th>{t('us.allocAction')}</th></tr></thead>
        <tbody>
          {state.sides.map((side) => {
            const budget = state.budgets[side.id];
            const row = notice?.side === side.id;
            return <tr key={side.id} data-side-row={side.id}>
              <td>{side.id}</td>
              <td data-side="allocated">{budget ? (budget.allocated == null ? t('us.unallocated') : budget.allocated.toLocaleString()) : t('us.unknown')}</td>
              <td data-side="committed">{budget ? budget.committed.toLocaleString() : t('us.unknown')}</td>
              <td data-side="remaining">{budget ? fmt(budget.remaining) : t('us.unknown')}</td>
              <td>
                <input aria-label={t('us.allocated')} inputMode="numeric" min="0" step="1" style={{ width: '10ch' }}
                  value={edits[side.id] ?? ''} onChange={(e) => setEdits((s) => ({ ...s, [side.id]: e.target.value }))}
                  disabled={saving === side.id} />
                {' '}<button className="btn" disabled={saving === side.id} onClick={() => void save(side.id, edits[side.id] ?? '')}>{t('us.setAllocation')}</button>
                {' '}<button className="btn" disabled={saving === side.id} onClick={() => void save(side.id, '')}>{t('us.clearAllocation')}</button>
                {row && notice.kind === 'saved' && <p role="status">{t('us.allocSaved')}</p>}
                {row && notice.kind === 'invalid' && <p role="alert">{t('us.allocInvalid')}</p>}
                {row && notice.kind === 'scope' && <p role="alert">{t('us.allocScope')}</p>}
                {row && notice.kind === 'not_found' && <p role="alert">{t('us.allocNotFound')}</p>}
                {row && notice.kind === 'busy' && <p role="alert">{t('nr.logoutUnresolved')}</p>}
                {row && notice.kind === 'failed' && <p role="alert">{t('nu.retryHelp')}</p>}
              </td>
            </tr>;
          })}
        </tbody>
      </table>}
      {state.sides.length > SIDES_WITH_BUDGET && <p className="dim">{t('us.budgetCap', { n: SIDES_WITH_BUDGET })}</p>}
    </section>
  </>;
}
