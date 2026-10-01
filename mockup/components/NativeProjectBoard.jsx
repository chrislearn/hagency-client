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
import TechnicalDetails from '@/components/TechnicalDetails';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
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

      <PageHead title={t('tk.board')} sub={t('tk.boardSub')}>
        <Link className="btn" href="/tasks">← {t('tk.title')}</Link>
        <NativeStatusStrip />
      </PageHead>

      <div className="cards">
        <div className="card">
          <div className="cap">{t('tk.boardProjects')}</div>
          <div className="val">{board.totals.projects}</div>
        </div>
        <div className="card">
          <div className="cap">{t('tk.boardAgents')}</div>
          <div className="val">{board.totals.agents}</div>
        </div>
      </div>

      <TechnicalDetails>
        <p>{t('tk.boardGenerated')}: {board.generatedAt}</p>
        {board.unavailable.length > 0 && <p>{t('tk.unavailableNote', { list: board.unavailable.join(', ') })}</p>}
      </TechnicalDetails>

      {board.projects.length === 0 ? (
        <div className="empty">
          <div className="big">{t('tk.boardEmpty')}</div>
        </div>
      ) : (
        board.projects.map((p) => (
          <div className="panel" key={p.id}>
            <h2 className="board-title">{p.name}</h2>
            <div className="chips">{p.agents.length > 0 ? p.agents.map((a) => <span className="chip" key={a}>{a}</span>) : <span className="dim">—</span>}</div>
            {/* One column per task state: the board itself, not a second row of counts. */}
            <div className="lanes-board" aria-label={t('tk.boardLanes')}>
              {STATUSES.map((s) => (
                <section className={`lane-col${s === 'blocked' && p.taskLanes[s] > 0 ? ' hot' : ''}`} key={s}>
                  <header><span>{s.replaceAll('_', ' ')}</span><b>{p.taskLanes[s]}</b></header>
                  <p>{p.taskLanes[s] > 0 ? <Link href="/tasks">{t('tk.boardOpenLane')}</Link> : t('tk.boardLaneEmpty')}</p>
                </section>
              ))}
            </div>
          </div>
        ))
      )}
    </div>
  );
}
