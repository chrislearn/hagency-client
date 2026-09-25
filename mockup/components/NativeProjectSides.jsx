'use client';

/*
 * Native project-sides (ADR-132 + board #14): the fleet registrations read as
 * sides — the id IS the server name (ADR-016) — with the side lifecycle
 * controls the retained console offers: credential install (write-only,
 * ADR-016 decision 8), verify + verdict, deactivate/reactivate/delete, and
 * add/archive project. The list read keeps its exact six-key contract (a
 * credential can never render in a row); the DETAIL of one side — fetched
 * through the lifecycle single-side route — carries the allow-list projection
 * (`credentialKind`/`hasCredential`, never a token) plus the access verdict.
 * No credential value can appear on this page.
 */
import { useState } from 'react';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import { fetchSide, setSideCredential, verifySide, addSideProject, archiveSideProject, deactivateSide, reactivateSide, removeSide } from '@/lib/native-api';

const ACCESS_STATES = ['unverified', 'accepted', 'rejected', 'unreachable', 'blocked'];

function SideDetail({ side, onClose, onChanged }) {
  const t = useT();
  const [credential, setCredential] = useState('');
  const [busy, setBusy] = useState(null);
  const [error, setError] = useState(null);
  const [verify, setVerify] = useState(null);
  const [projectName, setProjectName] = useState('');
  const [projectRoom, setProjectRoom] = useState('');

  const run = async (label, fn) => {
    setBusy(label);
    setError(null);
    try {
      const result = await fn();
      setBusy(null);
      await onChanged?.();
      return result;
    } catch (e) {
      setBusy(null);
      setError(e.message);
      return null;
    }
  };

  const submitCredential = () => run('set', async () => {
    if (!credential.trim()) return;
    const parsed = JSON.parse(credential);
    await setSideCredential(side.id, parsed);
    setCredential('');
  });

  const checkVerify = () => run('verify', async () => {
    const result = await verifySide(side.id);
    setVerify(result);
    return result;
  });

  const addProject = () => run('project', async () => {
    const input = { name: projectName };
    if (projectRoom.trim()) input.roomId = projectRoom.trim();
    await addSideProject(side.id, input);
    setProjectName('');
    setProjectRoom('');
  });

  const toggleArchive = (project, archived) => run('archive', () => archiveSideProject(side.id, project.id, archived));

  const toggleActive = () => run(side.active ? 'deactivate' : 'reactivate', () =>
    side.active ? deactivateSide(side.id) : reactivateSide(side.id));

  const remove = () => run('delete', async () => {
    if (!window.confirm(t('sl.deleteConfirm'))) return;
    await removeSide(side.id);
    onClose?.();
  });

  return (
    <div className="panel" style={{ marginTop: 14 }}>
      <h3>{t('sl.detail')} — <span className="mono">{side.id}</span></h3>
      <dl className="kv">
        <dt>{t('sl.apiBaseUrl')}</dt><dd className="mono">{side.apiBaseUrl ?? '—'}</dd>
        <dt>{t('sl.credentialKind')}</dt><dd>{side.credentialKind ?? t('sl.noCredential')}</dd>
        <dt>{t('sl.accessState')}</dt><dd className="pill">{side.accessState}</dd>
        {side.awaitingInstall && <div className="sub dim">{t('sl.awaitingInstall')}</div>}
      </dl>

      {side.projects.map((project) => (
        <div className="sub" key={project.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>
            <span className="mono">{project.id}</span>{project.archived && <span className="pill warn"> {t('sl.archived')}</span>}
            {project.roomId && <span className="dim"> · {project.roomId}</span>}
          </span>
          <button className="btn-s" disabled={busy} onClick={() => toggleArchive(project, !project.archived)}>
            {project.archived ? t('sl.restore') : t('sl.archive')}
          </button>
        </div>
      ))}

      <div className="cred-actions" style={{ marginTop: 12 }}>
        <textarea
          rows={3}
          placeholder={t('sl.credentialJson')}
          value={credential}
          onChange={(e) => setCredential(e.target.value)}
          spellCheck={false}
        />
        <button className="btn-s primary" disabled={busy || !credential.trim()} onClick={submitCredential}>
          {busy === 'set' ? t('sl.setting') : t('sl.setCredential')}
        </button>
      </div>

      <div className="btn-row" style={{ marginTop: 10 }}>
        <button className="btn" disabled={busy} onClick={checkVerify}>{t('cr.verify')}</button>
        <button className="btn" disabled={busy} onClick={toggleActive}>{side.active ? t('sl.deactivate') : t('sl.reactivate')}</button>
        <button className="btn danger" disabled={busy} onClick={remove}>{t('sl.delete')}</button>
      </div>

      {verify && (
        <p role="status" className={verify.promoted ? 'stranded' : 'sub dim'}>
          {t('sl.verifyResult', { state: verify.side?.accessState ?? verify.promoted })}
          {verify.promoted && <span> — {t('sl.promotedNote')}</span>}
        </p>
      )}

      <div className="cred-actions" style={{ marginTop: 10 }}>
        <input placeholder={t('sl.projectName')} value={projectName} onChange={(e) => setProjectName(e.target.value)} />
        <input placeholder={t('sl.roomId')} value={projectRoom} onChange={(e) => setProjectRoom(e.target.value)} />
        <button className="btn-s" disabled={busy || !projectName.trim()} onClick={addProject}>{t('sl.addProject')}</button>
      </div>

      {error && <p className="stranded" role="alert">{error}</p>}
    </div>
  );
}
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { NativeAccessNotice } from '@/components/NativeUsage';

export default function NativeProjectSides() {
  const t = useT();
  const data = useData();
  const { phase, error, refreshing, sides = [], unavailable = [], permissions = {} } = data;
  const manageLifecycle = permissions.manageLifecycle === true;
  const [selected, setSelected] = useState(null);
  const [detail, setDetail] = useState(null);
  const [detailBusy, setDetailBusy] = useState(false);

  const open = async (side) => {
    setSelected(side.id);
    setDetailBusy(true);
    setDetail(null);
    try {
      setDetail(await fetchSide(side.id));
    } catch (e) {
      setDetail({ error: e.message });
    } finally {
      setDetailBusy(false);
    }
  };

  const refreshDetail = async () => {
    if (!selected) return;
    setDetail(await fetchSide(selected));
    await data.refresh();
  };

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
              {manageLifecycle && (
                <div className="btn-row" style={{ marginTop: 10 }}>
                  <button className="btn-s" onClick={() => open(side)} disabled={detailBusy}>{t('sl.controls')}</button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {!manageLifecycle && <p className="sub dim">{t('sl.needLifecycle')}</p>}

      {selected && (
        <SideDetail
          side={detail && !detail.error ? detail : { id: selected, active: true, projects: [] }}
          onClose={() => { setSelected(null); setDetail(null); }}
          onChanged={refreshDetail}
        />
      )}

      <div className="btn-row" style={{ marginTop: 14 }}>
        <button className="btn" onClick={data.refresh}>{t('nu.refresh')}</button>
      </div>
    </div>
  );
}
