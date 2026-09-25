'use client';

/*
 * Native agent roster (ADR-126, widened by board #22): a READ-ONLY
 * observation of the engagement projections, one row per AGENT — the TS
 * roster's shape, so every agent the service knows appears. The columns
 * render exactly what /console/api/agents serves; the columns the SERVER
 * names in `unavailable` render as unknown — never zero, never invented —
 * and the list is server-owned, so a future source turns a column on by
 * removing its name server-side, not by editing this page. `online` and
 * `last_seen_ms` are real worker state served per agent. There is
 * deliberately no work item, progress, queue, task count or utilisation
 * column: the retained roster withdrew them on principle and native does
 * not widen what it narrowed. The lifecycle scope exposes stop and a
 * separate private stopped-task review. Start and preset rebinding remain
 * absent because their server routes fail closed; a button that can only
 * refuse would lie. Clicking an agent's name opens its detail IN PLACE: the
 * packaged native console serves one static HTML per route with no fallback
 * (the manifest carries only agents/index.html), so a link to /agents/<name>
 * would 404 in production — the href stays only for the dev route and for
 * open-in-new-tab.
 */
import { useState } from 'react';
import Link from 'next/link';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { NativeAccessNotice } from '@/components/NativeUsage';
import { useT } from '@/components/Prefs';
import { errorText } from '@/lib/i18n';
import { useData } from '@/components/Data';
import { fmtTokens } from '@/lib/mock-data';
import { stopAgent } from '@/lib/native-api';
import NativeStoppedWork from '@/components/NativeStoppedWork';
import NativeAgentDetail from '@/components/NativeAgentDetail';

export default function NativeAgents() {
  const t = useT();
  const data = useData();
  const { phase, error, refreshing, agents = [], unavailable = [], permissions = {} } = data;
  const manageLifecycle = permissions.manageLifecycle === true;
  const [review, setReview] = useState(null);
  const [hold, setHold] = useState(false);
  const [selected, setSelected] = useState(null);
  /* Item 5: Stop is an awaited mutation with visible pending / success /
   * error words, never a fire-and-forget click that can reject unhandled. */
  const [stop, setStop] = useState(null);

  const stopOne = async (agent) => {
    if (stop?.engagement === agent.engagement_id) return;
    setStop({ engagement: agent.engagement_id, kind: 'pending' });
    try {
      await stopAgent(agent.engagement_id);
      setStop({ engagement: agent.engagement_id, kind: 'saved' });
      await data.refresh();
    } catch (error) {
      setStop({
        engagement: agent.engagement_id,
        kind: ['busy', 'outcome_unknown', 'native_unavailable', 'invalid_native_response'].includes(error.message) ? 'unknown' : 'refused',
        error: error.message,
      });
    }
  };

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('nu.failed')}</h2>
        <p>{t(error === 'not_found' ? 'nu.notFound' : 'nu.retryHelp')}</p>
        <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>
      </section>
    );
  }
  /* Item 6: an access need renders the page head and the notice with the
   * CLI command — never a blank screen. */
  if (phase === 'access') return <>
    <PageHead title={t('nav.workforce')} sub={t('na.rosterSub')}><NativeStatusStrip /></PageHead>
    <NativeAccessNotice />
  </>;
  if (selected) return <NativeAgentDetail name={selected} onBack={() => setSelected(null)} />;

  return (
    <div data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('nu.refreshing')}</p>}

      {/* Items 2 and 4: one PageHead per page — h1, tab title and the
       * status strip — and no heading that contradicts its buttons. The
       * observation itself IS read-only, but with the lifecycle scope the
       * page stops work, so the heading says observation. */}
      <PageHead title={t('nav.workforce')} sub={t('na.rosterSub')}><NativeStatusStrip /></PageHead>
      <NativeAccessNotice />

      {/* The server's own gap list, rendered verbatim: the page never
          decides which columns are unknown. */}
      <p className="sub dim" style={{ fontSize: 12 }}>
        {t('na.unavailable', { list: unavailable.join(', ') })}
      </p>

      {/* Item 7: the roster starts empty — "an agent appears once it is
       * lent" is a ready-state fact, not a first paint. */}
      {phase === 'loading' ? (
        <p role="status">{t('na.loadingRoster')}</p>
      ) : agents.length === 0 ? (
        <div className="empty">
          <div className="big">{t('na.none')}</div>
        </div>
      ) : (
        <div className="list">
          <table>
            <thead>
              <tr>
                <th>{t('col.agent')}</th>
                <th>{t('col.framework')}</th>
                <th>{t('col.role')}</th>
                <th>{t('col.state')}</th>
                <th>{t('na.liveness')}</th>
                <th>{t('na.engagement')}</th>
                <th className="num">{t('col.requested')}</th>
                <th className="num">{t('na.consumed')}</th>
                <th>{t('na.lastActivity')}</th>
                <th>{t('na.online')}</th>
                <th>{t('na.lastSeen')}</th>
                {manageLifecycle && <th>{t('na.controls')}</th>}
              </tr>
            </thead>
            <tbody>
              {agents.map((a) => (
                <tr key={a.name} data-engagement-id={a.engagement_id} data-agent-name={a.name}>
                  <td><Link href={`/agents/${encodeURIComponent(a.name)}`} onClick={(e) => { e.preventDefault(); setSelected(a.name); }}>{a.name}</Link></td>
                  <td className="dim">{a.framework}</td>
                  <td>{a.role}</td>
                  <td>{a.state}</td>
                  {/* Board #60 item 2: the LIVE DISPATCH's word, a separate
                      fact from the engagement lifecycle word beside it —
                      null means no live dispatch, said as unknown. */}
                  <td className="dim">{a.liveness === null ? t('nu.unknown') : t(`na.liveness.${a.liveness}`)}</td>
                  <td className="dim">{a.engagement_id}</td>
                  <td className="num dim">{fmtTokens(a.requested_tokens)}</td>
                  {/* Tokens observed consumed; null when unmeasured, never
                      rendered as a zero that would read as "used nothing". */}
                  <td className="num dim">{a.consumed === null ? t('nu.unknown') : fmtTokens(a.consumed)}</td>
                  {/* Last dispatch activity, not last seen; null is unknown,
                      rendered as the word — never a zero clock. */}
                  <td className="dim">{a.last_activity_ms === null ? t('nu.unknown') : new Date(a.last_activity_ms).toISOString()}</td>
                  {/* Real worker state: a live dispatch in one of the
                      agent's sessions. */}
                  <td>{a.online ? t('na.online') : t('na.offline')}</td>
                  {/* Newest attempt clock the agent produced; null is
                      unknown, never zero. */}
                  <td className="dim">{a.last_seen_ms === null ? t('nu.unknown') : new Date(a.last_seen_ms).toISOString()}</td>
                  {manageLifecycle && (
                    <td>
                      <button className="btn" data-lifecycle-action="stop" disabled={hold || stop?.kind === 'pending'} onClick={() => void stopOne(a)}>{t('na.stop')}</button>
                      <button className="btn" data-lifecycle-action="review" disabled={hold} onClick={() => setReview(a)}>{t('nrec.open')}</button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="btn-row" style={{ marginTop: 14 }}>
        <button className="btn" disabled={hold} onClick={data.refresh}>{t('nu.refresh')}</button>
      </div>
      {stop && (
        <section className="notice" data-stop-action={stop.kind} role={stop.kind === 'pending' || stop.kind === 'saved' ? 'status' : 'alert'}>
          <p><b>{stop.engagement}</b> · {t(`na.stop.${stop.kind}`)}{stop.error ? ` (${errorText(t, stop.error)})` : ''}</p>
          {stop.kind !== 'pending' && <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>}
        </section>
      )}
      {manageLifecycle && review && <NativeStoppedWork key={review.engagement_id} agent={review} onHold={setHold} />}
    </div>
  );
}
