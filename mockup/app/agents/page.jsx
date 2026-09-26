'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import NativeAgents from '@/components/NativeAgents';

/*
 * The native roster route (ADR-126). The retained console's roster lives at
 * /workforce — /agents/<name> is the single-agent detail page — so the
 * retained arm of this route sends the reader there rather than rendering a
 * second, divergent roster.
 *
 * #27 adds the per-agent execution policy here. It is rendered by THIS page
 * rather than on /agents/<name>, for a build reason: the native console build
 * stages only app/agents/page.jsx, so the retained agent-detail page (and the
 * editor inside it) is unreachable in the native console. The routes existed
 * and no reachable UI called them.
 */
export default function AgentsPage() {
  const data = useData();
  return data.nativeConsole ? (
    <>
      <NativeAgents />
      <ExecutionPolicy />
    </>
  ) : (
    <RetainedRoster />
  );
}

/*
 * The per-agent execution policy (#27; TS parity: backend-v2.js:10865-10885).
 *
 * It talks to the console's OWN route (`/console/api/agents/:id/execution-policy`,
 * server: native/hagency/src/console/exec_policy.rs), never the retained
 * proxy `/api/hagency/*`: the two are different products and only the native
 * one is served by the Rust console. The session cookie is Path=/console, so
 * it rides along.
 *
 * `yolo` is offered only for Codex agents — the server's own rule
 * (`normalize_policy`, hagency-core/src/execution.rs:28 requires
 * framework == "codex"), so no control is drawn that could only be refused.
 * The whole panel needs a console sign-in (one access link grants every
 * console action), the same grant the lifecycle buttons beside it need.
 */
function ExecutionPolicy() {
  const t = useT();
  const data = useData();
  const { agents = [], permissions = {}, provenance } = data;
  const live = provenance?.agents === 'live';
  const manage = permissions.manageLifecycle === true;
  const codex = agents.filter((a) => a.framework === 'codex');
  const [selected, setSelected] = useState('');
  const [snapshot, setSnapshot] = useState(null);
  const [yolo, setYolo] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const agent = codex.find((a) => a.engagement_id === selected) || null;
  const endpoint = agent
    ? `/console/api/agents/${encodeURIComponent(agent.engagement_id)}/execution-policy`
    : null;

  useEffect(() => {
    setSnapshot(null);
    setNotice('');
    if (!endpoint) return undefined;
    let current = true;
    (async () => {
      try {
        const res = await fetch(endpoint, {
          credentials: 'same-origin', cache: 'no-store', redirect: 'error',
          headers: { Accept: 'application/json' },
        });
        const body = await res.json();
        if (!current) return;
        if (!res.ok) { setNotice(body?.code || `HTTP ${res.status}`); return; }
        setSnapshot(body);
        setYolo(body.executionPolicy?.yolo === true);
      } catch (error) {
        if (current) setNotice(String(error?.message || error));
      }
    })();
    return () => { current = false; };
  }, [endpoint]);

  if (!manage || !live || codex.length === 0) return null;

  return (
    <section className="panel" aria-label={t('exec.title')} data-execution-policy>
      <h3 className="sub">{t('exec.title')}</h3>
      <label className="field-row">
        <span>{t('exec.agent')}</span>
        <select aria-label={t('exec.agent')} value={selected} disabled={busy}
          onChange={(event) => setSelected(event.target.value)}>
          <option value="">{t('exec.selectAgent')}</option>
          {codex.map((a) => (
            <option key={a.engagement_id} value={a.engagement_id}>{a.name}</option>
          ))}
        </select>
      </label>
      {agent && !snapshot && !notice && <p role="status">{t('exec.loading')}</p>}
      {agent && snapshot && (
        <>
          <div>
            <label className="field-row">
              <input type="checkbox" aria-label="YOLO" checked={yolo === true} disabled={busy}
                onChange={(event) => setYolo(event.target.checked)} />
              <b>{t('exec.yolo')}</b>
            </label>
            <p className="small">{t(yolo ? 'exec.yoloHelp' : 'exec.sandboxHelp')}</p>
          </div>
          <p className="small dim">{t('exec.nextRun')}</p>
          <button className="btn" data-execution-policy-save
            disabled={busy || yolo === (snapshot.executionPolicy?.yolo === true)}
            onClick={async () => {
              setBusy(true);
              setNotice('');
              try {
                const res = await fetch(endpoint, {
                  method: 'PUT', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
                  headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
                  body: JSON.stringify({ executionPolicy: { yolo } }),
                });
                const body = await res.json();
                if (!res.ok) { setNotice(body?.code || `HTTP ${res.status}`); return; }
                setSnapshot(body);
                setYolo(body.executionPolicy?.yolo === true);
                setNotice(t('exec.saved'));
              } catch (error) {
                setNotice(String(error?.message || error));
              } finally {
                setBusy(false);
              }
            }}>{t('exec.save')}</button>
        </>
      )}
      {notice && <p role="status" className="small">{notice}</p>}
    </section>
  );
}

function RetainedRoster() {
  const t = useT();
  const router = useRouter();
  useEffect(() => { router.replace('/workforce'); }, [router]);
  return (
    <section className="panel" role="status">
      <h2>{t('nav.workforce')}</h2>
      <p>{t('na.retiredRoute')}</p>
      <p><Link className="btn" href="/workforce">{t('nav.workforce')}</Link></p>
    </section>
  );
}
