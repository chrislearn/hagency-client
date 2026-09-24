'use client';

/*
 * The native agent detail (board #22, TS backend-v2.js:12155
 * GET /api/agents/:name): one agent the service knows, fetched from
 * /console/api/agents/<name> and rendered as identity, resource, rooms,
 * current dispatch and recent tasks. Every section renders exactly what
 * the route serves — null last_seen is unknown, never zero, and a null
 * dispatch renders as "no live dispatch", never invented. A 404 from the
 * route means the native service knows no agent by this name; that is
 * said plainly, with the way back to the roster.
 */
import { useEffect, useState } from 'react';
import Link from 'next/link';
import { useT } from '@/components/Prefs';
import { fmtTokens } from '@/lib/mock-data';
import { fetchAgentDetail } from '@/lib/native-api';

export default function NativeAgentDetail({ name }) {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [detail, setDetail] = useState(null);

  useEffect(() => {
    let live = true;
    fetchAgentDetail(name)
      .then((value) => {
        if (!live) return;
        setDetail(value);
        setPhase('ready');
      })
      .catch((error) => {
        if (!live) return;
        setPhase(error?.message === 'not_found' ? 'missing' : 'error');
      });
    return () => { live = false; };
  }, [name]);

  if (phase === 'loading') {
    return (
      <section className="panel" role="status">
        <p>{t('na.detailLoading')}</p>
      </section>
    );
  }
  if (phase === 'missing') {
    return (
      <div className="empty">
        <div className="big">{t('na.detailNotFound', { name })}</div>
        <Link className="btn" href="/agents">{t('na.detailBack')}</Link>
      </div>
    );
  }
  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <p>{t('na.detailFailed')}</p>
        <Link className="btn" href="/agents">{t('na.detailBack')}</Link>
      </section>
    );
  }

  return (
    <div data-agent-name={detail.name}>
      <h2 style={{ marginTop: 0 }}>{detail.name}<span className="note"> {t('na.readonly')}</span></h2>

      <section className="panel" aria-label={t('na.detailIdentity')}>
        <h3>{t('na.detailIdentity')}</h3>
        <table>
          <tbody>
            <tr><th>{t('col.framework')}</th><td>{detail.framework}</td></tr>
            <tr><th>{t('col.role')}</th><td>{detail.role}</td></tr>
            <tr><th>{t('col.state')}</th><td>{detail.state}</td></tr>
            <tr><th>{t('na.online')}</th><td>{detail.online ? t('na.online') : t('na.offline')}</td></tr>
            <tr><th>{t('na.lastSeen')}</th><td className="dim">{detail.last_seen_ms === null ? t('nu.unknown') : new Date(detail.last_seen_ms).toISOString()}</td></tr>
            <tr><th>{t('na.detailEngagements')}</th><td className="num">{detail.engagements}</td></tr>
            <tr><th>{t('na.detailProject')}</th><td className="dim">{detail.project_id}</td></tr>
          </tbody>
        </table>
      </section>

      <section className="panel" aria-label={t('na.detailResource')}>
        <h3>{t('na.detailResource')}</h3>
        <table>
          <tbody>
            <tr><th>{t('na.detailResource')}</th><td className="dim">{detail.resource_id}</td></tr>
            <tr><th>{t('na.engagement')}</th><td className="dim">{detail.engagement_id}</td></tr>
            <tr><th className="num">{t('col.requested')}</th><td className="num dim">{fmtTokens(detail.requested_tokens)}</td></tr>
          </tbody>
        </table>
      </section>

      <section className="panel" aria-label={t('na.detailRooms')}>
        <h3>{t('na.detailRooms')}</h3>
        {detail.rooms.length === 0 ? (
          <p className="dim">{t('na.noRooms')}</p>
        ) : (
          <table>
            <thead>
              <tr><th>{t('na.detailRooms')}</th><th>{t('col.state')}</th></tr>
            </thead>
            <tbody>
              {detail.rooms.map((room) => (
                <tr key={room.session_id} data-session-id={room.session_id}>
                  <td className="dim">{room.room_id}</td>
                  <td>{room.dispatch_state ?? '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel" aria-label={t('na.detailDispatch')}>
        <h3>{t('na.detailDispatch')}</h3>
        {detail.dispatch === null ? (
          <p className="dim">{t('na.detailNoDispatch')}</p>
        ) : (
          <table>
            <tbody>
              <tr><th>{t('col.state')}</th><td>{detail.dispatch.dispatch_state}</td></tr>
              <tr><th>{t('na.detailDispatch')}</th><td className="dim">{detail.dispatch.dispatch_id}</td></tr>
              <tr><th>{t('na.detailRooms')}</th><td className="dim">{detail.dispatch.room_id}</td></tr>
            </tbody>
          </table>
        )}
      </section>

      <section className="panel" aria-label={t('na.detailTasks')}>
        <h3>{t('na.detailTasks')}</h3>
        {detail.tasks.length === 0 ? (
          <p className="dim">{t('na.detailNoTasks')}</p>
        ) : (
          <table>
            <thead>
              <tr><th>{t('na.detailTasks')}</th><th>{t('col.state')}</th><th>{t('na.lastActivity')}</th></tr>
            </thead>
            <tbody>
              {detail.tasks.map((task) => (
                <tr key={task.id} data-task-id={task.id}>
                  <td>{task.title}</td>
                  <td>{task.status}</td>
                  <td className="dim">{new Date(task.updated_at).toISOString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <div className="btn-row" style={{ marginTop: 14 }}>
        <Link className="btn" href="/agents">{t('na.detailBack')}</Link>
      </div>
    </div>
  );
}
