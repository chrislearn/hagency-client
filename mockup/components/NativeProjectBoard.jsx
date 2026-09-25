'use client';

/*
 * Native project board (board #23, TS parity: GET /api/project-board,
 * backend-v2.js:16025-16051 over lib/project-board.js).
 *
 * The retained snapshot carries `generatedAt`, `staleAfterMs`,
 * `activityLimit`, `totals` and `projects`, each project with its agents and
 * its five task lanes. Native serves the same envelope and adds
 * `unavailable`: the retained columns with no native source (repositories,
 * worktrees, specs, issues, activity, health, binding, createdAt) are NAMED
 * by the server and rendered as unknown here, never as zero — the same
 * discipline the roster (ADR-126) and project-sides (ADR-132) reads use.
 *
 * Lane statuses are wire values, so they are rendered as text.
 */
import { useCallback, useEffect, useState } from 'react';
import Link from 'next/link';
import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { fetchProjectBoard } from '@/lib/native-api';

const STATUSES = ['created', 'accepted', 'in_progress', 'blocked', 'done'];

export default function NativeProjectBoard() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [error, setError] = useState(null);
  const [board, setBoard] = useState(null);
  const [refreshing, setRefreshing] = useState(false);

  const load = useCallback(async () => {
    try {
      setBoard(await fetchProjectBoard());
      setError(null);
      setPhase('ready');
    } catch (err) {
      setError(err.message);
      setPhase('error');
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const refresh = async () => {
    setRefreshing(true);
    try { await load(); } finally { setRefreshing(false); }
  };

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('tk.boardFailed')}</h2>
        <p>{t('tk.retryHelp')}</p>
        <button className="btn" onClick={refresh}>{t('common.refresh')}</button>
      </section>
    );
  }
  if (!board) return null;

  return (
    <div data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('tk.refreshing')}</p>}

      <PageHead title={t('tk.board')}>
        <Link className="btn" href="/tasks">{t('tk.title')}</Link>
      </PageHead>
      <h2 style={{ marginTop: 0 }}>{t('tk.board')}</h2>

      <div className="cards">
        <div className="card">
          <div className="cap">{t('tk.boardProjects')}</div>
          <div className="val">{board.totals.projects}</div>
        </div>
        <div className="card">
          <div className="cap">{t('tk.boardAgents')}</div>
          <div className="val">{board.totals.agents}</div>
        </div>
        {STATUSES.map((s) => (
          <div className="card" key={s}>
            <div className="cap">{s}</div>
            <div className={`val${s === 'blocked' && board.totals.tasks[s] > 0 ? ' warn' : ''}`}>
              {board.totals.tasks[s]}
            </div>
          </div>
        ))}
      </div>

      <p className="note faint" style={{ marginTop: 10, fontSize: 12 }}>
        {t('tk.boardGenerated')}: {board.generatedAt}
        {board.unavailable.length > 0 && ` · ${t('tk.unavailableNote', { list: board.unavailable.join(', ') })}`}
      </p>

      {board.projects.length === 0 ? (
        <div className="empty">
          <div className="big">{t('tk.boardEmpty')}</div>
        </div>
      ) : (
        board.projects.map((p) => (
          <div className="panel" key={p.id}>
            <h3>{p.name}</h3>
            <p className="small dim">{p.agents.length > 0 ? p.agents.join(', ') : '—'}</p>
            <h4 style={{ marginTop: 10 }}>{t('tk.boardLanes')}</h4>
            <div className="cards">
              {STATUSES.map((s) => (
                <div className="card" key={s}>
                  <div className="cap">{s}</div>
                  <div className={`val${s === 'blocked' && p.taskLanes[s] > 0 ? ' warn' : ''}`}>{p.taskLanes[s]}</div>
                </div>
              ))}
            </div>
          </div>
        ))
      )}
    </div>
  );
}
