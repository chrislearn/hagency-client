'use client';

/*
 * The approval-bindings surface (board #52, TS `GET /api/approval-bindings`
 * at backend-v2.js:9051-9082 plus `DELETE
 * /api/approval-bindings/:agent/:roomId` at :9032-9046).
 *
 * The LIST is an observation: every live binding is derived from the native
 * store's own room observations (`observe_approval_room` +
 * `current_approval_bindings`), never asserted by the operator — so no "bind"
 * control exists here. The TS ASSERTING writes (`PUT /api/approval-bindings`,
 * PUT membership) assert a governance fact native derives instead, and have no
 * faithful native writer.
 *
 * The UNBIND is ported: it removes the derived binding row AND revokes the
 * approval grants that binding carried, in one transaction — the TS
 * `removeBinding` + `revokeScopesByBinding` pair. integ is SINGLE-LOGIN
 * (operator decision), so any logged-in session may unbind: the control is
 * always offered, and there is no capability word to gate it on.
 *
 * Rows carry exactly the nine keys the route serves; membership status,
 * `active` and the authority id have no native source and are never invented.
 */
import { useEffect, useState } from 'react';
import { useT } from '@/components/Prefs';
import { fetchApprovalBindings, unbindApprovalBinding } from '@/lib/native-api';

export default function NativeApprovalBindings() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [error, setError] = useState(null);
  const [rows, setRows] = useState([]);
  const [refreshing, setRefreshing] = useState(false);
  const [hold, setHold] = useState(false);
  const [action, setAction] = useState(null);

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

  const unbind = async (row) => {
    if (hold) return;
    setHold(true);
    setAction({ engagementId: row.engagementId, kind: 'pending' });
    try {
      await unbindApprovalBinding(row.agent, row.roomId);
      setAction({ engagementId: row.engagementId, kind: 'unbound' });
      await load();
    } catch (err) {
      // The refusal word is surfaced, never swallowed: a 404 and an
      // unconfirmed mutation mean different things to the operator, and an
      // unconfirmed one is reported as unknown rather than as success.
      const kind = err.message === 'not_found' ? 'notFound'
        : ['outcome_unknown', 'native_unavailable', 'invalid_native_response'].includes(err.message) ? 'unknown'
        : 'refused';
      setAction({ engagementId: row.engagementId, kind, error: err.message });
    } finally {
      setHold(false);
    }
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
      {action && ['unbound', 'unknown', 'refused', 'notFound'].includes(action.kind) && (
        <p role="status" className="small faint">
          {action.kind === 'unbound' ? t('ab.unbound')
            : action.kind === 'unknown' ? t('ab.unbindUnknown')
            : t('ab.unbindRefused')}
        </p>
      )}
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
              <th>{t('ab.controls')}</th>
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
                <td>
                  <button
                    className="btn"
                    data-bindings-action="unbind"
                    disabled={hold}
                    onClick={() => unbind(r)}
                  >
                    {action?.engagementId === r.engagementId && action.kind === 'pending'
                      ? t('ab.unbindPending') : t('ab.unbind')}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
