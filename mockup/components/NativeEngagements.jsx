'use client';

/*
 * Native engagements: a READ-ONLY triage list over the same
 * /console/api/engagements read the usage page selects from — no new data
 * path, the page just renders the list slice `Data.jsx` already carries
 * (`fetchNative` returns `engagements` + `next_after`; pagination rides
 * `nextPage`/`firstPage`). No create, verdict, revoke or whitelist action:
 * those mutate enforcement and need their own reviewed decision; buttons
 * that would 404 lie (the same rule as the alerts page).
 *
 * The retained page's route reasons (notWhitelisted / overOffer /
 * overCeiling) have no native counterpart — the whitelist is the retained
 * fleet model's admission surface. The native analogue of "awaiting my
 * decision" is the engagement STATE column (pending → reserved/active …),
 * which is exactly what renders here.
 */
import { useMemo, useState } from 'react';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import { fmtTokens } from '@/lib/mock-data';
import { retireEngagement, retryEngagementCleanup } from '@/lib/native-api';

const NATIVE_STATES = ['pending', 'reserved', 'active', 'rejected', 'revoked', 'failed'];
/* The command id is the store's idempotency key: minted here, never by the
 * route. One per operator act, so a double-submit replays rather than acts. */
const newCommand = () => `console_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;

export default function NativeEngagements() {
  const t = useT();
  const data = useData();
  const { phase, error, refreshing, engagements = [], next_after: nextAfter } = data;
  const [state, setState] = useState('all');
  const [agent, setAgent] = useState('all');
  // The exile rule (AgentActions.jsx): a destructive control asks first and
  // says what happened. `confirming` holds `{id, kind}` — never a bare flag,
  // so a confirmation can never be carried onto a different row.
  const [confirming, setConfirming] = useState(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);

  const agentNames = useMemo(
    () => [...new Set(engagements.map((e) => e.agentName))].filter(Boolean),
    [engagements],
  );
  const counts = useMemo(() => {
    const out = Object.fromEntries(NATIVE_STATES.map((s) => [s, 0]));
    for (const e of engagements) out[e.state] = (out[e.state] ?? 0) + 1;
    return out;
  }, [engagements]);
  const rows = useMemo(
    () => engagements
      .filter((e) => state === 'all' || e.state === state)
      .filter((e) => agent === 'all' || e.agentName === agent),
    [engagements, state, agent],
  );

  /* Retire is the store's `end(..., revoke = true)` guard (domain.rs:1525-1530):
   * pending, reserved or active. Anything terminal is refused with
   * `engagement_not_live`, so no button is offered for it. */
  const retirable = (e) => ['pending', 'reserved', 'active'].includes(e.state);
  /* The cleanup retry's precondition (domain.rs:1505-1510): the engagement is
   * revoked AND its `retire` effect is failed. The list read exposes that as
   * `cleanup` — TS's own `['failed','pending'].includes(withdrawal.state)`
   * arm, with native's four-value column in place of the withdrawal record.
   * A retirement that failed waits for this operator act: there is no sweeper
   * and no timer (engagements.rs:12). */
  const retryable = (e) => e.state === 'revoked' && ['pending', 'uncertain'].includes(e.cleanup);

  async function act(id, kind) {
    if (busy) return;
    setBusy(true);
    setNote(null);
    try {
      const receipt = kind === 'retire'
        ? await retireEngagement(id, newCommand())
        : await retryEngagementCleanup(id, newCommand());
      setConfirming(null);
      setNote(kind === 'retire'
        ? t('ng.retiredMsg', { state: receipt.state, cleanup: receipt.cleanup })
        : t('ng.retriedMsg', { state: receipt.state, cleanup: receipt.cleanup }));
      await data.refresh();
    } catch (error) {
      setNote(error.message === 'agent_lifecycle_scope_required'
        ? t('ng.scopeRequired')
        : `${t('ng.actionFailed')} (${error.message})`);
    } finally {
      setBusy(false);
    }
  }

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('nu.failed')}</h2>
        <p>{t(error === 'not_found' ? 'nu.notFound' : 'nu.retryHelp')}</p>
        <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>
      </section>
    );
  }
  if (phase === 'access') return null;

  return (
    <div data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('nu.refreshing')}</p>}

      <PageHead title={t('nav.engagements')}><NativeStatusStrip /></PageHead>
      <h2 style={{ marginTop: 0 }}>{t('nav.engagements')}<span className="note"> {t('ng.readonly')}</span></h2>

      {/* One strip: the state split of this page. */}
      <div className="cards">
        {NATIVE_STATES.map((s) => (
          <div className="card" key={s}>
            <div className="cap">{s}</div>
            <div className={`val${s === 'pending' && counts[s] > 0 ? ' warn' : ''}`}>{counts[s]}</div>
          </div>
        ))}
      </div>

      <div className="btn-row" style={{ margin: '22px 0 12px' }}>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('col.state')}{' '}
          <select value={state} onChange={(e) => setState(e.target.value)}>
            <option value="all">{t('common.all')}</option>
            {NATIVE_STATES.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('col.agent')}{' '}
          <select value={agent} onChange={(e) => setAgent(e.target.value)}>
            <option value="all">{t('common.all')}</option>
            {agentNames.map((a) => <option key={a} value={a}>{a}</option>)}
          </select>
        </label>
        <span className="spacer" style={{ flex: 1 }} />
        <span className="sub dim" style={{ fontSize: 12 }}>
          {t('common.shown', { a: rows.length, b: engagements.length })}
        </span>
      </div>

      {rows.length === 0 ? (
        <div className="empty">
          <div className="big">{t('ng.none')}</div>
        </div>
      ) : (
        <div className="list">
          <table>
            <thead>
              <tr>
                <th>{t('col.state')}</th>
                <th>{t('col.agent')}</th>
                <th>{t('col.project')}</th>
                <th>{t('col.role')}</th>
                <th className="num">{t('col.requested')}</th>
                <th>{t('col.action')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((e) => (
                <tr key={e.id}>
                  <td>{e.state}</td>
                  <td>{e.agentName}</td>
                  <td>{e.projectName ?? '—'}</td>
                  <td>{e.role}</td>
                  <td className="num dim">{fmtTokens(e.requestedTokens)}</td>
                  <td>
                    {/* Exile + confirm (AgentActions.jsx): the confirmation
                        names THIS row's id, so a slip on one row can never
                        retire another. */}
                    {confirming?.id === e.id ? (
                      <span className="btn-row tight">
                        <span className="dim">{confirming.kind === 'retire' ? t('ng.confirmRetire') : t('ng.confirmRetry')}</span>
                        <button type="button" className="btn-s danger" disabled={busy}
                          onClick={() => act(e.id, confirming.kind)}>{t('ng.confirm')}</button>
                        <button type="button" className="btn-s" disabled={busy}
                          onClick={() => setConfirming(null)}>{t('ng.cancel')}</button>
                      </span>
                    ) : (
                      <>
                        {retirable(e) && (
                          <button type="button" className="btn-s danger" disabled={busy}
                            onClick={() => { setNote(null); setConfirming({ id: e.id, kind: 'retire' }); }}>
                            {t('ng.retire')}
                          </button>
                        )}
                        {retryable(e) && (
                          <button type="button" className="btn-s" disabled={busy}
                            onClick={() => { setNote(null); setConfirming({ id: e.id, kind: 'cleanup-retry' }); }}>
                            {t('ng.retryCleanup')}
                          </button>
                        )}
                      </>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="btn-row" style={{ marginTop: 14 }}>
        <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>
        <button className="btn" disabled={nextAfter === null} onClick={data.nextPage}>{t('ng.nextPage')}</button>
        <button className="btn" onClick={data.firstPage}>{t('nu.firstPage')}</button>
      </div>
    </div>
  );
}
