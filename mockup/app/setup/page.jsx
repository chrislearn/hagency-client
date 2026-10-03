'use client';

/*
 * ADR-189: the setup page. Three steps, each showing its state and what the
 * user does next:
 *   1. Coding agents — Hagency detects them and whether they are signed in.
 *      It never signs anyone in: the user runs the agent's own login, then
 *      clicks Check again. A signed-in agent is configured automatically.
 *   2. Connect Palpo — the existing import control.
 *   3. Offer a resource — the first resource, from the Resources page.
 */
import { useCallback, useEffect, useState } from 'react';
import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import { errorText } from '@/lib/i18n';
import { fetchSetup, checkSetup } from '@/lib/native-api';
import ImportPalpoControl from '../project-sides/import-palpo';

function Step({ n, title, done, children }) {
  return <section className="panel setup-step" aria-labelledby={`setup-step-${n}`}>
    <h2 id={`setup-step-${n}`}><span className="setup-n">{done ? '✓' : n}</span> {title}</h2>
    {children}
  </section>;
}

function AgentCard({ agent }) {
  const t = useT();
  const name = agent.kind === 'codex' ? 'Codex' : agent.kind;
  return <div className="setup-agent">
    <p><b>{name}</b>{agent.version ? ` · ${agent.version}` : ''}</p>
    {!agent.found && <p>{t('st.notFound', { name })}</p>}
    {agent.found && <p className="dim mono">{agent.path}</p>}
    {agent.found && agent.signedIn && <p>{t('st.signedIn', { kind: agent.signInKind === 'api_key' ? t('st.kindApiKey') : agent.signInKind === 'chatgpt' ? t('st.kindChatgpt') : '—' })}</p>}
    {agent.found && agent.signedIn && agent.signInKind === 'chatgpt' && <p className="note">{t('st.planNote')}</p>}
    {agent.found && !agent.signedIn && <p>{t('st.notSignedIn', { name })} <code>{agent.kind === 'codex' ? 'codex login' : ''}</code></p>}
    {agent.problem && <p className="dim">{agent.problem}</p>}
  </div>;
}

export default function SetupPage() {
  const t = useT();
  const data = useData();
  const [setup, setSetup] = useState(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);

  const load = useCallback(async () => {
    try { setSetup(await fetchSetup()); setNote(null); } catch (error) { setNote(errorText(t, error.message)); }
  }, [t]);
  useEffect(() => { if (data.nativeConsole) load(); }, [data.nativeConsole, load]);

  async function check() {
    if (busy) return;
    setBusy(true); setNote(null);
    try {
      const value = await checkSetup();
      setSetup(value);
      if (value.configured === false && value.problem) setNote(value.problem);
    } catch (error) {
      setNote(error.message === 'setup_not_fleet' ? t('st.notFleet') : errorText(t, error.message));
    } finally {
      setBusy(false);
    }
  }

  if (!data.nativeConsole) return <PageHead title={t('nav.setup')} sub={t('st.nativeOnly')} />;
  const agents = setup?.agents ?? [];
  const ready = agents.some((a) => a.found && a.signedIn);
  return <>
    <PageHead title={t('nav.setup')} sub={t('st.sub')} />
    {note && <p role="status" className="note">{note}</p>}
    <Step n={1} title={t('st.agentsTitle')} done={setup?.runtimeConfigured}>
      <p>{t('st.agentsHelp')}</p>
      {setup === null ? <p className="dim">{t('st.loading')}</p> : agents.map((agent) => <AgentCard key={agent.kind} agent={agent} />)}
      <p>{setup?.runtimeConfigured ? t('st.runtimeReady') : ready ? t('st.runtimePending') : t('st.runtimeWaiting')}</p>
      <button type="button" className="btn" disabled={busy} onClick={check}>{busy ? t('st.checking') : t('st.checkAgain')}</button>
    </Step>
    <Step n={2} title={t('st.palpoTitle')} done={setup?.palpo?.imported}>
      {setup?.palpo?.imported
        ? <p>{t('st.palpoConnected', { state: setup.palpo.transport?.state ?? '—' })}</p>
        : <ImportPalpoControl />}
    </Step>
    <Step n={3} title={t('st.resourceTitle')} done={false}>
      <p>{t('st.resourceHelp')}</p>
      <a className="btn" href="/console/resources/">{t('nav.resources')}</a>
    </Step>
  </>;
}
