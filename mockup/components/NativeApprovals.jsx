'use client';

/*
 * Native approvals: a READ-ONLY observation list over /console/api/approvals
 * (ADR-138, PC-C2b). Rows carry exactly seven keys — id, state, choice,
 * reusableScope, expiresAt, engagementId, projectRoomId — and no nested
 * object, so nothing here renders a card, a preview, an owner identity or a
 * tool name. An undelivered approval shows its `state` and `choice` WORDS;
 * the delivery stage is the deferred delivery route's own field and is never
 * fabricated here.
 *
 * No create, verdict, consume or delivery control: those mutate enforcement
 * and need their own reviewed decision; buttons that would 404 lie.
 */
import { useEffect, useState } from 'react';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { NativeAccessNotice } from '@/components/NativeUsage';
import { useT } from '@/components/Prefs';
import { fetchApprovals } from '@/lib/native-api';

export default function NativeApprovals() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [error, setError] = useState(null);
  const [rows, setRows] = useState([]);
  const [nextAfter, setNextAfter] = useState(null);
  const [refreshing, setRefreshing] = useState(false);
  const [stateFilter, setStateFilter] = useState('all');

  const load = async (after = '') => {
    try {
      const value = await fetchApprovals(after);
      setRows(after ? (prev) => [...prev, ...value.approvals] : value.approvals);
      setNextAfter(value.next_after);
      setError(null);
      setPhase('ready');
    } catch (err) {
      /* Item 6: an access need is an access notice, not a generic "could
       * not be read" — every other failure stays an error with a retry. */
      setError(err.message);
      setPhase(err.message === 'console_access_required' ? 'access' : 'error');
    }
  };

  useEffect(() => { load(); /* eslint-disable-line react-hooks/exhaustive-deps */ }, []);

  const refresh = async () => {
    setRefreshing(true);
    try { await load(); } finally { setRefreshing(false); }
  };

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('ap.failed')}</h2>
        <p>{t('ap.retryHelp')}</p>
        <button className="btn" onClick={refresh}>{t('common.refresh')}</button>
      </section>
    );
  }
  if (phase === 'access') return <>
    <PageHead title={t('nav.approvals')} sub={t('ap.readonly')}><NativeStatusStrip /></PageHead>
    <NativeAccessNotice />
  </>;

  const visible = rows.filter((r) => stateFilter === 'all' || r.state === stateFilter);
  const counts = Object.fromEntries(
    ['pending', 'decided', 'applying', 'uncertain', 'applied', 'invalidated', 'not_applied']
      .map((s) => [s, rows.filter((r) => r.state === s).length]),
  );

  return (
    <div data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('ap.refreshing')}</p>}

      {/* Items 2 and 1: one PageHead per page — h1, tab title and the
       * status strip — replacing the bare duplicate h2. */}
      <PageHead title={t('nav.approvals')} sub={t('ap.readonly')}><NativeStatusStrip /></PageHead>

      <div className="cards">
        {Object.entries(counts).map(([s, n]) => (
          <div className="card" key={s}>
            <div className="cap">{s}</div>
            <div className={`val${s === 'pending' && n > 0 ? ' warn' : ''}`}>{n}</div>
          </div>
        ))}
      </div>

      <div className="btn-row" style={{ margin: '22px 0 12px' }}>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('col.state')}{' '}
          <select value={stateFilter} onChange={(e) => setStateFilter(e.target.value)}>
            <option value="all">{t('common.all')}</option>
            {Object.keys(counts).map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </label>
        {/* Item 16: a Refresh in the ready state — approvals change as
         * decisions land; the page had no way to see new ones. */}
        <button className="btn" onClick={refresh} disabled={refreshing}>{t('common.refresh')}</button>
        <span className="spacer" style={{ flex: 1 }} />
        <span className="sub dim" style={{ fontSize: 12 }}>{t('common.shown', { a: visible.length, b: rows.length })}</span>
      </div>

      {/* Item 7: the list starts empty — "no approvals" is a ready-state
       * fact, not a first paint. */}
      {phase === 'loading' ? (
        <p role="status">{t('ap.loading')}</p>
      ) : visible.length === 0 ? (
        <div className="empty">
          <div className="big">{t('ap.none')}</div>
          <p className="small">{t('ap.noneNote')}</p>
        </div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>{t('col.id')}</th>
              <th>{t('col.state')}</th>
              <th>{t('ap.choice')}</th>
              <th>{t('ap.reusable')}</th>
              <th className="num">{t('ap.expires')}</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((r) => (
              <tr key={r.id}>
                <td className="faint" style={{ fontSize: 11 }}>{r.id}</td>
                <td>{r.state}</td>
                <td>{r.choice ?? '—'}</td>
                <td>{r.reusableScope ? t('ap.reusableYes') : t('ap.reusableNo')}</td>
                <td className="num dim">{new Date(r.expiresAt).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {nextAfter && (
        <div className="btn-row" style={{ marginTop: 16 }}>
          <button className="btn" onClick={() => load(nextAfter)}>{t('ng.nextPage')}</button>
        </div>
      )}
    </div>
  );
}
