'use client';

import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import NativeProjectBoard from '@/components/NativeProjectBoard';

/*
 * 项目看板 — the operator project board (board #23, TS parity:
 * GET /api/project-board, backend-v2.js:16025-16051 over lib/project-board.js).
 *
 * The retained board is built from Matrix groups, workflow bindings, agents
 * with live filesystem inspections, the task graph store and the message
 * corpus. Native has a source for the registered projects, their agents and
 * the operator tasks; every column it cannot source is NAMED by the server in
 * `unavailable` and rendered as unknown here — never as zero.
 */
export default function ProjectBoardPage() {
  const data = useData();
  const t = useT();
  return data.nativeConsole ? <NativeProjectBoard /> : <LegacyProjectBoard />;
}

function LegacyProjectBoard() {
  const t = useT();
  return (
    <>
      <PageHead title={t('tk.board')} />
      <section className="panel">
        <h2>{t('tk.board')}</h2>
        <p>{t('nu.unavailableRoute')}</p>
      </section>
    </>
  );
}