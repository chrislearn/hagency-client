'use client';

import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import NativeTaskGraphs from '@/components/NativeTaskGraphs';

/*
 * Task graphs (board #47). The native arm renders the operator task-graph
 * page over /console/api/task-graphs; the retained arm points at the
 * retained dashboard's graph view, which this console does not reimplement.
 */
export default function TaskGraphsPage() {
  const data = useData();
  return data.nativeConsole ? <NativeTaskGraphs /> : <RetainedNote />;
}

function RetainedNote() {
  const t = useT();
  return (
    <section className="panel" role="status">
      <PageHead title={t('nav.taskGraphs')} />
      <p>{t('nu.unavailableRoute')}</p>
    </section>
  );
}
