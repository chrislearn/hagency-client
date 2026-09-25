'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useMemo, useState } from 'react';
import { runtimeLabel } from '@/lib/agent-detail';
import { useData } from '@/components/Data';
import { PrefsSwitch, useT } from '@/components/Prefs';

/*
 * The rail. Present on every route, so what I am lending never leaves the screen.
 *
 * Decisions carried over from the previous console, each of which answered a
 * review finding:
 *  - agents are <Link>, not <button>: they have real URLs, so middle-click,
 *    bookmarking and keyboard navigation come free
 *  - nav is a pinned grid row, so a long agent list cannot push it below the fold
 *  - counts are labelled ("3 pending"), never bare numbers whose denominator the
 *    reader has to guess
 *  - aria-current="page" marks the destination exactly once
 *
 * What changed is the tag under each agent. It used to read the transport, then
 * the role it had been allocated. For a contributor the useful fact is **what it
 * contributes** — the model, and the tier that model qualifies for. An agent with
 * no preset says so, because that is the state which makes it useless.
 */
const SECTIONS = [
  {
    head: 'rail.secResource',
    rows: [
      { href: '/resources', key: 'resources', icon: '▦', count: 'resourcesConfigured', unit: 'configured', also: ['/'] },
      /*
       * The roster sits under 资源 rather than getting a heading of its own,
       * although it joins all four layers. The four headings ARE the four layers, so
       * a fifth for one row would imply a fifth layer; and the row's subject is the
       * agent — the resource itself — seen across the layers rather than a new kind
       * of thing. The count is AGENTS lent out, not engagements: the row below
       * already counts those, and one agent serving three projects is one agent
       * whose capacity is spoken for.
       */
      { href: '/workforce', key: 'workforce', icon: '☰', count: 'lent', unit: 'lent' },
    ],
  },
  {
    head: 'rail.secCapability',
    rows: [
      { href: '/capability', key: 'capability', icon: '◫', count: 'rolesOffered', unit: 'offered' },
    ],
  },
  {
    head: 'rail.secEngagement',
    rows: [
      /*
       * Projects sits above engagements because it is upstream of them: a project has to
       * have invited me before it can ask for anything. `hot` for the same reason
       * /engagements is — an unanswered invitation is a person waiting on me, and ADR-014
       * exists because that used to be invisible.
       */
      { href: '/projects', key: 'projects', icon: '⌂', count: 'invitesPending', unit: 'waiting', hot: true },
      { href: '/engagements', key: 'engagements', icon: '⇄', count: 'pending', unit: 'pending', hot: true },
      { href: '/usage', key: 'usage', icon: '◎', count: 'active', unit: 'active' },
    ],
  },
  {
    head: 'rail.secFleet',
    rows: [
      { href: '/alerts', key: 'alerts', icon: '◉', count: 'alertsOpen', unit: 'open', hot: true },
      { href: '/task-graphs', key: 'taskGraphs', icon: '⧉', count: null },
      /*
       * Operator tasks (board #23) sit under FLEET beside alerts: both are the
       * operator's own triage surfaces over a work queue, and the kept row is
       * what makes the page reachable by clicking rather than by typing a URL.
       * No count: the store read is the page's own, and a rail number would be
       * a second, staler copy of it.
       */
      { href: '/tasks', key: 'tasks', icon: '☑', count: null },
      { href: '/config', key: 'config', icon: '⚙', count: null },
    ],
  },
];

// A dynamic segment marks its parent, so /resources/new and /agents/<name> never
// render a rail with nothing current — which the invariant suite rejects and a
// reader experiences as being lost.
const isUnder = (pathname, href) => pathname.startsWith(`${href}/`);

/*
 * The native console is served under basePath /console, and usePathname()
 * reports whatever path the deployment uses. Comparing a raw pathname against
 * '/console/agents' only worked for the rows whose check happened to be a
 * suffix test; the Agents row — a full-match test — was therefore unmarked on
 * every page it was supposed to mark. Strip the base once, then compare against
 * route paths, so the marking does not depend on whether usePathname() includes
 * it (both forms are handled).
 */
const CONSOLE_BASE = '/console';
const routePath = (pathname) => {
  const raw = pathname ?? '/';
  const stripped = raw === CONSOLE_BASE || raw.startsWith(`${CONSOLE_BASE}/`)
    ? raw.slice(CONSOLE_BASE.length) || '/'
    : raw;
  return stripped.length > 1 && stripped.endsWith('/') ? stripped.slice(0, -1) : stripped;
};

/** Does the current route belong to this rail row? */
function isCurrent(path, row) {
  // /agents/<name> is the roster's own detail view.
  if (row.key === 'workforce') return path === '/agents' || path.startsWith('/agents/');
  // A dynamic segment marks its parent: /resources/new is still the resources
  // row, which the wizard's cancel link returns to.
  if (row.key === 'resources') return path === '/' || path === '/resources' || path.startsWith('/resources/');
  return path === row.href || path.startsWith(`${row.href}/`);
}

export default function Rail() {
  const data = useData();
  return data.nativeConsole ? <NativeRail /> : <LegacyRail />;
}

function NativeRail() {
  const t = useT();
  const pathname = usePathname();
  const data = useData();
  const path = routePath(pathname);
  return <nav className="rail" aria-label={t('rail.nav')}>
    <div className="rail-brand"><b>HAGENCY</b><span>{t('nr.nativeConsole')}</span></div>
    <div className="rail-fleet">{SECTIONS.map((sec) => <div key={sec.head}>
      <h2 className="rail-sec">{t(sec.head)}</h2>
      <ul className="rail-list">{sec.rows.map((row) => <li key={row.key}>
        {['usage', 'resources', 'alerts', 'engagements', 'workforce', 'tasks', 'taskGraphs'].includes(row.key) ? <a className="fleet-row" href={row.key === 'workforce' ? '/console/agents/' : row.key === 'taskGraphs' ? '/console/task-graphs/' : `/console/${row.key}/`} aria-current={isCurrent(path, row) ? 'page' : undefined}><span className="ico">{row.icon}</span><span className="grow">{t(`nav.${row.key}`)}</span></a>
          : <span className="fleet-row" aria-disabled="true" title={t('nu.unavailableRoute')}><span className="ico">{row.icon}</span><span className="grow">{t(`nav.${row.key}`)}</span><span>—</span></span>}
      </li>)}</ul>
    </div>)}</div>
    {/* The session control belongs to the SHELL, not to a page: it used to sit
        inside the Resource configuration panel, so six of the nine pages
        offered no way to end access at all. Rendered here it is on every
        native page, exactly once. */}
    <div className="rail-session">
      <button className="btn" data-shell-action="end-access" onClick={data.logout}>{t('nu.logout')}</button>
    </div>
    <PrefsSwitch />
  </nav>;
}

function LegacyRail() {
  const t = useT();
  const pathname = usePathname();
  const [filter, setFilter] = useState('');
  /*
   * The rail reads the same source as the page beside it.
   *
   * It did not, briefly, and the result was the clearest possible symptom: a live
   * roster in the table and five fixture names in the rail, on the same screen. A
   * navigation surface that disagrees with the content it navigates to is worse
   * than one with no counts at all.
   */
  const { agents, presets, presetOf, railCounts, tierOf } = useData();
  const counts = { ...railCounts(), resourcesConfigured: presets.length };

  /*
   * EVERY agent, not only the running ones.
   *
   * This filtered on `alive !== false`, which was invisible against a fixture where
   * every agent is alive. Against a real backend it emptied the rail completely:
   * registered-but-not-running is the ordinary state of a contributed agent, and
   * the page beside it listed five while the rail said "AGENTS · 0".
   *
   * Hiding them is also wrong on its own terms. This is a roster of what the
   * contributor OWNS, not of what happens to be executing; an idle agent is still
   * lent capacity, and it is the one you would click to find out why it is idle.
   * The row already carries its state, so nothing is lost by showing it.
   *
   * `agents` belongs in the dependency list. When the roster was a module-level
   * import it never changed, so [filter] alone was harmless; now it arrives from a
   * fetch, and omitting it froze the rail on the fixture names while the counts
   * beside them — computed outside the memo — had already switched to live.
   */
  const shown = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return agents;
    return agents.filter((a) => a.name.toLowerCase().includes(q));
  }, [agents, filter]);

  const unconfigured = agents.filter((a) => !a.presetId).length;

  return (
    <nav className="rail" aria-label={t('rail.nav')}>
      <div className="rail-brand">
        <b>HAGENCY</b>
        <span>
          {`${agents.length} ${t('rail.agentsCount')} · ${unconfigured} ${t('rail.unconfigured')}`}
        </span>
      </div>

      <div className="rail-fleet">
        {SECTIONS.map((sec) => (
          <div key={sec.head}>
            <h2 className="rail-sec">{t(sec.head)}</h2>
            <ul className="rail-list">
              {sec.rows.map((f) => {
                const n = f.count ? counts[f.count] : null;
                const current = pathname === f.href
                  || isUnder(pathname, f.href)
                  || (f.also ?? []).includes(pathname);
                return (
                  <li key={f.href}>
                    <Link
                      href={f.href}
                      className="fleet-row"
                      aria-current={current ? 'page' : undefined}
                    >
                      <span className="ico" aria-hidden="true">{f.icon}</span>
                      <span className="grow">{t(`nav.${f.key}`)}</span>
                      {/* One interpolated string, not two adjacent expressions:
                          React separates those with a comment marker in SSR,
                          which fragments the text node and breaks copy. */}
                      {f.count && (
                        <span className={`pill${n > 0 && f.hot ? ' hot' : ''}${n === 0 ? ' zero' : ''}`}>
                          {`${n} ${t(`unit.${f.unit}`)}`}
                        </span>
                      )}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
      </div>

      <div className="rail-filter roster-filter">
        <input
          type="search"
          placeholder={t('rail.filter')}
          aria-label={t('rail.filter')}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>

      <div className="rail-scroll">
        <h2 className="rail-sec roster-head">
          <span className="grow">{`${t('rail.agents')} · ${shown.length}`}</span>
        </h2>
        <ul className="rail-list">
          {shown.map((a) => {
            const href = `/agents/${a.name}`;
            const preset = presetOf(a);
            const tier = tierOf(preset);
            return (
              <li key={a.name}>
                <Link
                  href={href}
                  className={`agent-row${a.activeNow ? ' on' : ''}`}
                  aria-current={pathname === href ? 'page' : undefined}
                >
                  <span className="glyph" aria-hidden="true">{a.activeNow ? '●' : '○'}</span>
                  <span className="id">
                    <span className="nm">{a.name}</span>
                    {/* Status is text, not colour. */}
                    <span className="st">{runtimeLabel(a, t)}</span>
                    {/* What it contributes. An unconfigured agent contributes
                        nothing, and that is the fact worth surfacing. */}
                    <span className={`job${preset ? '' : ' none'}`}>
                      {preset ? `${preset.model} · ${tier}` : t('rail.noModel')}
                    </span>
                  </span>
                </Link>
              </li>
            );
          })}
          {/* Two different empty states. `No agent matches ""` — which is what a
              fresh install used to read — describes a search nobody performed. The
              quoted term only belongs here when there IS one. */}
          {shown.length === 0 && (
            <li className="rail-empty">
              {agents.length === 0
                ? t('rail.noAgentsAtAll')
                : `${t('rail.noMatch')} “${filter}”`}
            </li>
          )}
        </ul>
      </div>

      <PrefsSwitch />
    </nav>
  );
}
