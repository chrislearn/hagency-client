'use client';

/*
 * The native console front door (`/console/`). The retained console's root
 * IS 我的资源 (mockup/app/page.jsx), and this page plays the same role for
 * the native console: an operator home that reads the fleet state and links
 * to every native page, so no destination depends on knowing a deep URL —
 * which is what parity findings #1–#3 recorded (`/console/` 404s; accounts,
 * approvals and project-sides reachable only by typing the URL).
 *
 * Fleet state comes from the existing unauthenticated GET /ready — the same
 * observation NativeStatusStrip renders — so the home page is truthful on a
 * host with no console session, and there is no second readiness vocabulary
 * here. It never claims session-scoped facts it did not read.
 *
 * The link list enumerates exactly the pages the native build stages and
 * the server serves (build-native-console.mjs, console/assets.rs): usage,
 * resources, resources/new, alerts, engagements, agents, project-sides,
 * approvals, accounts. A route not staged by the build is not linked; the
 * greyed rail rows (capability, projects, config) name the workflows the
 * native console does not yet have.
 */
import { useEffect, useState } from 'react';
import { useT } from '@/components/Prefs';
import { fetchReadiness } from '@/lib/native-api';

const LINKS = [
  { key: 'usage', href: '/console/usage/' },
  { key: 'resources', href: '/console/resources/' },
  { key: 'resourceNew', href: '/console/resources/new/' },
  { key: 'alerts', href: '/console/alerts/' },
  { key: 'engagements', href: '/console/engagements/' },
  { key: 'agents', href: '/console/agents/' },
  { key: 'projectSides', href: '/console/project-sides/' },
  { key: 'approvals', href: '/console/approvals/' },
  { key: 'accounts', href: '/console/accounts/' },
];

export default function NativeHome() {
  const t = useT();
  const [answer, setAnswer] = useState(null);
  const [unreachable, setUnreachable] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    fetchReadiness(controller.signal)
      .then((value) => setAnswer(value))
      .catch((error) => { if (error?.name !== 'AbortError') setUnreachable(true); });
    return () => controller.abort();
  }, []);
  // Same derivation as NativeStatusStrip: unreachable is unknown, never ready.
  const state = unreachable || (answer !== null && answer.status !== 'ok')
    ? (unreachable ? 'unknown' : 'not-ready')
    : answer === null ? 'checking' : 'ready';
  const word = state === 'ready' ? t('ns.ready') : state === 'not-ready' ? t('ns.notReady')
    : state === 'unknown' ? t('ns.unknown') : t('ns.checking');
  return (
    <section className="panel" data-native-state={state}>
      <h2>{t('nh.title')}</h2>
      <p>{t('nh.sub')}</p>
      <p data-native-status-cell="readiness">{t('ns.title')}: {word}</p>
      {state === 'not-ready' && answer !== null && (
        <p className="note">{answer.components.map((c) => `${c.name}=${c.state}`).join(' ')}</p>
      )}
      <ul className="rail-list" data-native-home="links">
        {LINKS.map((link) => (
          <li key={link.key}>
            <a className="fleet-row" href={link.href}>
              <span className="grow">{t(`nh.link.${link.key}`)}</span>
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}
