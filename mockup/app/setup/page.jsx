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
import { fetchSetup, checkSetup, offerResource } from '@/lib/native-api';
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

function OfferStep({ setup, onDone }) {
  const t = useT();
  const choices = setup?.offer?.choices ?? [];
  const [pick, setPick] = useState(0);
  const [tokens, setTokens] = useState('20000000');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);
  if (!setup?.runtimeConfigured) return <p>{t('st.resourceNeedsAgent')}</p>;
  async function offer() {
    const choice = choices[pick];
    const ceiling = Number.parseInt(tokens, 10);
    if (busy || !choice || !(ceiling > 0)) return;
    setBusy(true); setNote(null);
    try {
      await offerResource(choice.model, choice.reasoning ?? null, ceiling);
      setNote(t('st.offered'));
      onDone();
    } catch (error) {
      setNote(error.message === 'setup_unqualified_model' ? t('st.unqualified') : errorText(t, error.message));
    } finally {
      setBusy(false);
    }
  }
  return <>
    <p>{setup.offer.resources > 0 ? t('st.resourcesExist', { n: setup.offer.resources }) : t('st.resourceHelp')}</p>
    <label>{t('st.model')}{' '}
      <select value={pick} onChange={(e) => setPick(Number(e.target.value))}>
        {choices.map((c, i) => <option key={`${c.model}/${c.reasoning}`} value={i}>{c.model}{c.reasoning ? ` · ${c.reasoning}` : ''}</option>)}
      </select>
    </label>{' '}
    <label>{t('st.ceiling')}{' '}
      <input inputMode="numeric" value={tokens} onChange={(e) => setTokens(e.target.value.replace(/[^0-9]/g, ''))} />
    </label>{' '}
    <button type="button" className="btn" disabled={busy || choices.length === 0} onClick={offer}>{busy ? t('st.offering') : t('st.offer')}</button>
    {note && <p role="status" className="note">{note}</p>}
    <p className="dim">{t('st.moreResources')} <a href="/console/resources/">{t('nav.resources')}</a></p>
  </>;
}

export default function SetupPage() {
  const t = useT();
  const data = useData();
  const [setup, setSetup] = useState(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);

  const load = useCallback(async () => {
    try {
      // Wait for the provider's sign-in, like the other pages that read on their own.
      await data.ready;
      setSetup(await fetchSetup());
      setNote(null);
    } catch (error) { setNote(errorText(t, error.message)); }
  }, [t, data.ready]);
  useEffect(() => { if (data.nativeConsole) load(); }, [data.nativeConsole, load]);
  // ADR-189: a signed-in agent is configured without a click.
  const autoConfigure = setup && !setup.runtimeConfigured && (setup.agents ?? []).some((a) => a.found && a.signedIn);
  useEffect(() => { if (autoConfigure) check(); }, [autoConfigure]); // eslint-disable-line react-hooks/exhaustive-deps

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
  if (setup && setup.applicable === false) return <><PageHead title={t('nav.setup')} sub={t('st.sub')} /><p>{t('st.notFleet')}</p></>;
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
    <Step n={3} title={t('st.resourceTitle')} done={(setup?.offer?.resources ?? 0) > 0}>
      <OfferStep setup={setup} onDone={load} />
    </Step>
  </>;
}
