'use client';

/*
 * The native verdict surface (board #16, parity backend-v2.js:15161-15221):
 * the operator approves or refuses a pending engagement from this console.
 * Approve reaches the same store verdict the project-side Matrix approval
 * reaches — the server answers the bounded receipt and provisioning is
 * enqueued; refuse rides the existing /api/agents/{id}/refuse route. The
 * candidate read names the stored resource (the retained project-definition
 * choice); the command id is minted client-side and is the store's
 * idempotency key, never the route's.
 */
import { useEffect, useState } from 'react';
import { nativeRequest } from '@/lib/native-api';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';

const newCommand = () => `console_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;

function PendingRow({ e, onDone }) {
  const t = useT();
  const [candidates, setCandidates] = useState(null);
  const [note, setNote] = useState(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let live = true;
    nativeRequest(`/api/engagements/${encodeURIComponent(e.id)}/candidates`)
      .then((v) => { if (live) setCandidates(v); })
      .catch((error) => { if (live) setNote(error.message); });
    return () => { live = false; };
  }, [e.id]);
  const decide = async (kind) => {
    if (busy) return;
    setBusy(true);
    try {
      const path = kind === 'approve'
        ? `/api/engagements/${encodeURIComponent(e.id)}/approve`
        : `/api/agents/${encodeURIComponent(e.id)}/refuse`;
      await nativeRequest(path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ commandId: newCommand() }),
      });
      setNote(null);
      onDone(kind === 'approve' ? t('nv.approved') : t('nv.refusedMsg'));
    } catch (error) {
      setNote(error.message === 'agent_lifecycle_scope_required'
        ? t('nv.scopeRequired')
        : `${t('nv.decideFailed')} (${error.message})`);
    } finally {
      setBusy(false);
    }
  };
  const candidate = candidates?.candidates?.[0] ?? null;
  return (
    <tr>
      <td>{e.agentName}</td>
      <td className="dim">{e.role}</td>
      <td className="dim">
        {candidate
          ? `${candidate.resource} · ${candidate.framework} · ${candidate.model}`
            + (candidate.remainingTokens === null || candidate.remainingTokens === undefined
              ? '' : ` · ${t('nv.remaining')} ${candidate.remainingTokens}`)
          : (note ?? '…')}
      </td>
      <td>
        {candidates?.locked
          ? <span className="dim">{t('nv.locked')}</span>
          : (
            <div className="btn-row">
              <button className="btn-s primary" type="button" disabled={busy || !candidate} onClick={() => decide('approve')}>{t('en.approve')}</button>
              <button className="btn-s danger" type="button" disabled={busy} onClick={() => decide('refuse')}>{t('en.reject')}</button>
            </div>
          )}
        {candidates && note && <p role="alert" className="warn-text">{note}</p>}
      </td>
    </tr>
  );
}

export default function NativeVerdict() {
  const t = useT();
  const data = useData();
  const [flash, setFlash] = useState(null);
  if (data.phase === 'error' || data.phase === 'access') return null;
  const pending = (data.engagements ?? []).filter((e) => e.state === 'pending');
  return (
    <section className="panel" style={{ marginTop: 18 }}>
      <h2>{t('nv.verdict')} <span className="note">{t('nv.verdictHelp')}</span></h2>
      {pending.length === 0
        ? <p className="dim">{t('nv.nonePending')}</p>
        : (
          <div className="tbl-wrap">
            <table className="tbl">
              <thead>
                <tr>
                  <th>{t('col.agent')}</th>
                  <th>{t('col.role')}</th>
                  <th>{t('nv.candidate')}</th>
                  <th>{t('col.action')}</th>
                </tr>
              </thead>
              <tbody>
                {pending.map((e) => (
                  <PendingRow key={e.id} e={e} onDone={(message) => { setFlash(message); data.refresh(); }} />
                ))}
              </tbody>
            </table>
          </div>
        )}
      {flash && <p role="status">{flash}</p>}
    </section>
  );
}
