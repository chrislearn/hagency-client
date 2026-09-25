'use client';

/*
 * Native project-sides (ADR-132): a READ-ONLY observation of the fleet
 * registrations read as sides — the id IS the server name (ADR-016) —
 * and their projects. Six keys per side, projects of exactly {id,
 * room_id}; the SERVER-OWNED unavailable list is rendered verbatim, so
 * the columns native cannot answer (the whole credential family, access
 * verdicts, per-side allocation, label, API base URL) show as unknown
 * rather than invented. No credential value can appear on this page:
 * the validator refuses any key set other than the declared one, and no
 * declared key is a credential.
 */
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { NativeAccessNotice } from '@/components/NativeUsage';

export default function NativeProjectSides() {
  const t = useT();
  const data = useData();
  const { phase, error, refreshing, sides = [], unavailable = [] } = data;

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('nu.failed')}</h2>
        <p>{t(error === 'not_found' ? 'nu.notFound' : 'nu.retryHelp')}</p>
        <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>
      </section>
    );
  }
  /* Item 6: the access notice with the CLI command, not a blank screen. */
  if (phase === 'access') return <>
    <PageHead title={t('np.title')} sub={t('np.readonly')}><NativeStatusStrip /></PageHead>
    <NativeAccessNotice />
  </>;

  return (
    <div data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('nu.refreshing')}</p>}

      {/* Items 1, 2 and 7: one PageHead (h1, tab title, status strip)
       * instead of a bare h2, and a loading state before the fetch
       * settles — "no sides" is a ready-state fact, not a first paint. */}
      <PageHead title={t('np.title')} sub={t('np.readonly')}><NativeStatusStrip /></PageHead>
      <NativeAccessNotice />

      {/* The server's own gap list, rendered verbatim: the page never
          decides which columns are unknown. */}
      <p className="sub dim" style={{ fontSize: 12 }}>
        {t('np.unavailable', { list: unavailable.join(', ') })}
      </p>

      {phase === 'loading' ? (
        <p role="status">{t('np.loading')}</p>
      ) : sides.length === 0 ? (
        <div className="empty">
          <div className="big">{t('np.none')}</div>
        </div>
      ) : (
        <div className="cards">
          {sides.map((side) => (
            <div className="card" key={side.id}>
              <div className="cap" title={side.representative}>{side.id}</div>
              <div className="val">{t('np.projects', { n: side.projects.length })}</div>
              <div className="sub">
                <span className={`pill${side.registered ? '' : ' warn'}`}>{t(side.registered ? 'np.registered' : 'np.generationDrift')}</span>
              </div>
              <div className="TechnicalDetails" style={{ marginTop: 8, fontSize: 12 }}>
                <div className="sub">{t('np.representative')}: {side.representative}</div>
                <div className="sub">{t('np.reception')}: {side.reception_room_id}</div>
                <div className="sub">{t('np.generation')}: {side.generation}</div>
                {side.projects.map((project) => (
                  <div className="sub" key={project.id}>{project.id} · {project.room_id}</div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="btn-row" style={{ marginTop: 14 }}>
        <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>
      </div>
    </div>
  );
}
