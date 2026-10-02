'use client';

import { useCallback, useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import { nativeRequest } from '@/lib/native-api';

/*
 * Task #12's console surface: pending invitations with accept/decline —
 * the retained /projects pending-invites panel, on the native console.
 *
 * A row is a DECISION, not a notification: joining spends the
 * contributor's tokens, which is why an invitation from anyone but the
 * room's recorded owner lands here at all (ADR-014's 2026-08-11
 * amendment). Accepting establishes ownership from the inviter
 * (ADR-002); it does NOT whitelist the project (ADR-013 decision 4).
 *
 * "Queued", never "joined": only the invite poller can join a Matrix
 * room, and it retries refusals (ADR-183) — the response the decide
 * route returns says exactly that, and so does the toast. The inviter is
 * NULL when the invite state named no sender; that renders as its own
 * word, never a guess, because the inviter IS the owner.
 */
export default function InvitesPage() {
  const data = useData();
  return data.nativeConsole ? <NativeInvites /> : <RetainedInvites />;
}

function RetainedInvites() {
  const t = useT();
  const router = useRouter();
  useEffect(() => { router.replace('/projects'); }, [router]);
  return (
    <section className="panel" role="status">
      <h2>{t('ni.title')}</h2>
      <p>{t('ni.retainedRoute')}</p>
      <p><Link className="btn" href="/projects">{t('ni.retainedLink')}</Link></p>
    </section>
  );
}

function NativeInvites() {
  const t = useT();
  const data = useData();
  const [rows, setRows] = useState(null);
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(null);
  const [note, setNote] = useState(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const body = await nativeRequest('/api/matrix/pending-invites');
      setRows(Array.isArray(body.invites) ? body.invites : []);
    } catch (e) {
      setRows([]);
      setError(e.message);
    }
  }, []);

  useEffect(() => { if (data.phase !== 'access') load(); }, [load, data.phase]);

  async function decide(invite, accept) {
    setBusy(`${invite.projectRoomId} ${invite.agent} ${accept}`);
    setNote(null);
    try {
      await nativeRequest('/api/matrix/pending-invites/decide', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          projectRoomId: invite.projectRoomId,
          agent: invite.agent,
          accept,
        }),
      });
      setNote(t(accept ? 'ni.didAccept' : 'ni.didDecline', { agent: invite.agent }));
      await load();
    } catch (e) {
      setNote(
        e.message === 'agent_lifecycle_scope_required'
          ? t('ni.scope')
          : t('ni.decideFailed', { message: e.message }),
      );
    } finally {
      setBusy(null);
    }
  }

  if (data.phase === 'access') return null;

  return (
    <div data-native-state={data.phase} aria-busy={data.refreshing === true}>
      <PageHead title={t('ni.title')} sub={t('ni.subtitle')}><NativeStatusStrip /></PageHead>

      {error && (
        <section className="panel" role="alert">
          <h2>{t('nu.failed')}</h2>
          <p>{t('nu.retryHelp')}</p>
          <button className="btn" onClick={load}>{t('nu.refresh')}</button>
        </section>
      )}

      {rows !== null && (
        <>
          {rows.length === 0 ? (
            <div className="empty"><div className="big">{t('ni.none')}</div><div className="small">{t('ni.queuedNote')}</div></div>
          ) : (
            <div className="cards">
              {rows.map((invite) => (
                <div className="card" key={`${invite.projectRoomId} ${invite.agent}`}>
                  <div className="cap mono-s" title={invite.projectRoomId}>{invite.projectRoomId}</div>
                  <div className="val">{invite.agent}</div>
                  <div className="TechnicalDetails" style={{ marginTop: 8, fontSize: 12 }}>
                    <div className="sub">{t('ni.inviter')}: {invite.inviter ?? t('ni.noInviter')}</div>
                    <div className="sub">{t('ni.server')}: {invite.projectServer}</div>
                  </div>
                  <div className="btn-row" style={{ marginTop: 10 }}>
                    <button
                      type="button"
                      className="btn"
                      disabled={busy !== null || !invite.inviter}
                      title={invite.inviter ? undefined : t('ni.noInviter')}
                      data-invite-action="accept"
                      onClick={() => decide(invite, true)}
                    >
                      {busy?.endsWith('true') ? t('ni.busy') : t('ni.accept')}
                    </button>
                    <button
                      type="button"
                      className="btn"
                      disabled={busy !== null}
                      data-invite-action="decline"
                      onClick={() => decide(invite, false)}
                    >
                      {t('ni.decline')}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
          {note && <p role="status">{note}</p>}
          {rows.length > 0 && <p className="dim" style={{ fontSize: 13 }}>{t('ni.queuedNote')}</p>}
        </>
      )}
    </div>
  );
}
