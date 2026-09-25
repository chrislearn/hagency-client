'use client';

/*
 * Native offer book + preview (board #48): two READ-ONLY views over
 * /console/api/offer-book and /console/api/engagements/preview, and the
 * contributions list over /console/api/contributions.
 *
 * WHAT IS RENDERED, AND WHAT IS DELIBERATELY NOT. Every value here is the
 * server's; the page computes no cap, no route and no headroom of its own.
 *   - The three offer caps are `null` on the wire (the retained store's own
 *     "unset" encoding). They render as an UNSTATED cap, never as "0 tokens" —
 *     the difference between "no limit declared" and "you may not spend
 *     anything" is the whole reason the field is nullable.
 *   - `crossFamilyOk` and `runningNow` are real figures. A role with nothing
 *     able to serve it still lists, because "nothing currently qualifies" is a
 *     real answer a project needs.
 *   - The membership probe is tri-state and rendered as three states: `null` is
 *     "never checked", NOT "the agent is missing". Collapsing it would accuse
 *     every contribution of being broken before the first observation.
 *   - The preview is a DRY RUN. Its button reads; it never writes, and the
 *     route word shown is the route the server answered, not one this page
 *     picked.
 *
 * No create / verdict / revoke / whitelist control: those mutate enforcement
 * and need their own reviewed decision; buttons that would 404 lie.
 */
import { useEffect, useState } from 'react';
import { useT } from '@/components/Prefs';
import { fetchOfferBook, fetchContributions, fetchPreview } from '@/lib/native-api';
import { fmtTokens } from '@/lib/mock-data';

function Membership({ value, t }) {
  if (value === null) return <span className="dim">{t('ng.membershipUnknown')}</span>;
  return value
    ? <span className="ok">{t('ng.membershipIn')}</span>
    : <span className="stranded">{t('ng.membershipOut')}</span>;
}

export default function NativeOfferBook() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [error, setError] = useState(null);
  const [book, setBook] = useState(null);
  const [contributions, setContributions] = useState(null);
  const [role, setRole] = useState('');
  const [preview, setPreview] = useState(null);
  const [previewNote, setPreviewNote] = useState(null);
  const [previewing, setPreviewing] = useState(false);

  const load = async () => {
    try {
      const [nextBook, nextContributions] = await Promise.all([fetchOfferBook(), fetchContributions()]);
      setBook(nextBook);
      setContributions(nextContributions.contributions);
      setRole((current) => current || nextBook.roles[0]?.role || '');
      setError(null);
      setPhase('ready');
    } catch (err) {
      setError(err.message);
      setPhase('error');
    }
  };

  useEffect(() => { load(); /* eslint-disable-line react-hooks/exhaustive-deps */ }, []);

  const runPreview = async () => {
    if (previewing || !role) return;
    setPreviewing(true);
    try {
      setPreview(await fetchPreview(role));
      setPreviewNote(null);
    } catch (err) {
      setPreview(null);
      setPreviewNote(err.message);
    } finally {
      setPreviewing(false);
    }
  };

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('nu.failed')}</h2>
        <p>{t(error === 'not_found' ? 'nu.notFound' : 'nu.retryHelp')}</p>
        <button className="btn" onClick={load}>{t('nu.refresh')}</button>
      </section>
    );
  }
  if (phase === 'access') return null;

  const roles = book?.roles ?? [];
  return (
    <div data-native-state={phase}>
      <h2 style={{ marginTop: 24 }}>{t('ng.offerBook')}<span className="note"> {t('ng.offerBookNote')}</span></h2>

      {roles.length === 0 ? (
        <div className="empty"><div className="big">{t('ng.offerNone')}</div></div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>{t('col.role')}</th>
              <th>{t('ng.serving')}</th>
              <th>{t('ng.resources')}</th>
              <th className="num">{t('ng.runningNow')}</th>
            </tr>
          </thead>
          <tbody>
            {roles.map((r) => (
              <tr key={r.role}>
                <td>
                  <div>{r.role}</div>
                  <div className="faint" style={{ fontSize: 11 }}>
                    {r.crossFamilyOk ? t('ng.crossFamilyOk') : t('ng.crossFamilyNo')}
                  </div>
                </td>
                <td>
                  {r.serving ? (
                    <>
                      <div>{r.serving.framework} · {r.serving.model}{r.serving.reasoning ? ` (${r.serving.reasoning})` : ''}</div>
                      <div className="faint" style={{ fontSize: 11 }}>
                        {r.serving.tier ?? '—'}{r.serving.provisioningRequired ? ` · ${t('ng.newAgent')}` : ''}
                      </div>
                    </>
                  ) : (
                    <span className="dim">{t('ng.servingPrivate')}</span>
                  )}
                </td>
                <td>
                  <div className="faint" style={{ fontSize: 11 }}>{r.resources.map((x) => x.name).join(', ') || '—'}</div>
                  {/* The caps are the server's; `null` renders as unstated, never 0. */}
                  <div className="faint" style={{ fontSize: 11 }}>
                    {r.budgetCapPerEngagement === null && r.rateCap === null && r.count === null
                      ? t('ng.capUnstated')
                      : [
                        r.budgetCapPerEngagement === null ? null : t('ng.budgetCap', { n: fmtTokens(r.budgetCapPerEngagement) }),
                        r.rateCap === null ? null : t('ng.rateCap', { n: fmtTokens(r.rateCap) }),
                        r.count === null ? null : t('ng.countCap', { n: r.count }),
                      ].filter(Boolean).join(' · ')}
                  </div>
                </td>
                <td className="num dim">{r.runningNow}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <h2 style={{ marginTop: 28 }}>{t('ng.preview')}<span className="note"> {t('ng.previewNote')}</span></h2>
      <div className="btn-row">
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('ng.previewRole')}{' '}
          <select value={role} onChange={(e) => { setRole(e.target.value); setPreview(null); setPreviewNote(null); }}>
            {roles.map((r) => <option key={r.role} value={r.role}>{r.role}</option>)}
          </select>
        </label>
        <button className="btn" disabled={previewing || !role} onClick={runPreview}>{t('ng.previewRun')}</button>
      </div>
      {previewNote && <p className="why-inline">{t('ng.previewRoute')}: {previewNote}</p>}
      {preview && (
        <dl className="kv" style={{ marginTop: 8 }}>
          <dt>{t('ng.previewRoute')}</dt>
          <dd>{t(`ng.route.${preview.route}`)}</dd>
          <dt>{t('ng.previewAgent')}</dt>
          <dd>{preview.agent ?? '—'}</dd>
          <dt>{t('ng.previewRemaining')}</dt>
          <dd>{preview.agentRemainingTokens === null ? '—' : fmtTokens(preview.agentRemainingTokens)}</dd>
        </dl>
      )}

      <h2 style={{ marginTop: 28 }}>{t('ng.contributions')}<span className="note"> {t('ng.contributionsNote')}</span></h2>
      {contributions && contributions.length === 0 ? (
        <div className="empty"><div className="big">{t('ng.contributionsNone')}</div></div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>{t('col.agent')}</th>
              <th>{t('col.project')}</th>
              <th>{t('col.state')}</th>
              <th>{t('ng.membershipUnknown')}</th>
            </tr>
          </thead>
          <tbody>
            {(contributions ?? []).map((c) => (
              <tr key={`${c.agent}:${c.projectRoomId}`}>
                <td>{c.agent}</td>
                <td>
                  <div>{c.project}</div>
                  <div className="faint" style={{ fontSize: 11 }}>{c.projectRoomId}</div>
                </td>
                <td>{c.active ? 'active' : 'released'}</td>
                <td><Membership value={c.agentJoined} t={t} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div className="btn-row" style={{ marginTop: 14 }}>
        <button className="btn" onClick={load}>{t('nu.refresh')}</button>
      </div>
    </div>
  );
}
