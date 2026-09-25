'use client';

import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import NativeTasks from '@/components/NativeTasks';

/*
 * 任务管理 — operator task management (board #23, TS parity:
 * backend-v2.js:13194-13332 over lib/task-store.js).
 *
 * The retained page this replaces is `/api/tasks` driven; the native console
 * serves the same operations under its own session (list/filter, create,
 * edit, comment, delete, transition, per-agent list) plus the project board.
 * The write controls render ONLY when the session holds the configure scope
 * the server reports, so a read-only session sees the list and no control it
 * would be refused.
 */
export default function TasksPage() {
  const data = useData();
  const t = useT();
  return data.nativeConsole ? <NativeTasks /> : <LegacyTasks />;
}

function LegacyTasks() {
  const t = useT();
  return (
    <>
      <PageHead title={t('nav.tasks')} />
      <section className="panel">
        <h2>{t('nav.tasks')}</h2>
        <p>{t('nu.unavailableRoute')}</p>
      </section>
    </>
  );
}