'use client';

/*
 * The read-only approval-bindings list (board #52, TS `GET
 * /api/approval-bindings` at backend-v2.js:9051-9082, the plain-list
 * branch): every live binding, derived from the native store's own
 * observation-driven tables — never an operator assertion, so no unbind
 * control exists here. The TS write half (PUT bind, PUT membership, DELETE
 * unbind) asserts facts native derives from room observations instead, and
 * the unbind-revokes semantics already exist natively at the right moment:
 * the approval_room_retire_grants trigger revokes every grant of a binding's
 * engagement the instant its room observation goes unsafe.
 *
 * Rows carry exactly the nine keys the route serves; membership status,
 * `active` and the authority id have no native source and are never
 * invented.
 */
import { useEffect, useState } from 'react';
import { useT } from '@/components/Prefs';
import { fetchApprovalBindings } from '@/lib/native-api';

export default function NativeApprovalBindings() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [error, setError] = useState(null);
  const [rows, setRows] = useState([]);
  const [refreshing, setRefreshing] = useState(false);

  const load = async () => {
    try {
      const value = await fetchApprovalBindings();
      setRows(value.bindings);
      setError(null);
      setPhase('ready');
    } catch (err) {
      setError(err.message);
      setPhase('error');
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
        <h2>{t('ab.failed')}</h2>
        <p>{t('ab.retryHelp')}</p>
        <button className="btn" onClick={refresh}>{t('common.refresh')}</button>
      </section>
    );
  }

  return (
    <section className="panel" data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('ab.refreshing')}</p>}
      <h2 style={{ marginTop: 0 }}>
        {t('ab.title')}<span className="note"> {t('ab.readonly')}</span>
      </h2>
      {rows.length === 0 ? (
        <div className="empty">
          <div className="big">{t('ab.none')}</div>
          <p className="small">{t('ab.noneNote')}</p>
        </div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>{t('ab.agent')}</th>
              <th>{t('ab.project')}</th>
              <th>{t('ab.room')}</th>
              <th>{t('ab.owner')}</th>
              <th className="num">{t('ab.generation')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.engagementId}>
                <td>{r.agent}</td>
                <td className="faint" style={{ fontSize: 11 }}>{r.projectId}</td>
                <td className="faint" style={{ fontSize: 11 }}>{r.roomId}</td>
                <td className="dim" style={{ fontSize: 11 }}>{r.ownerMxid}</td>
                <td className="num dim">{r.roomGeneration}.{r.incarnation}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
