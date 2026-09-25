'use client';

/*
 * Task #13: the project-side 'Generate registration' action — the TS
 * dashboard behaviour (mockup/app/projects/new/page.jsx step 3 and
 * engagements/page.jsx's CredentialRow): POST project-sides/{side}/
 * registration-file with the operator-supplied url, then render the
 * written file's host path, mode, fingerprints, staged note and next
 * steps. The TOKENS never pass through the browser — that is the whole
 * design of the TS `-file` endpoint (ADR-016 decision 8), so "download"
 * here means exactly what the TS dashboard means: the YAML is written on
 * the Hagency host at the shown 0600 path, and the console shows where.
 *
 * The route lives under the agent-lifecycle console scope; a session
 * without it gets the refusal named as such, never a silent no-op.
 */
import { useState } from 'react';
import { useT } from '@/components/Prefs';
import { nativeRequest } from '@/lib/native-api';

export default function SideRegistrationControl({ sides }) {
  const t = useT();
  const [side, setSide] = useState(sides[0]?.id ?? '');
  const [url, setUrl] = useState('');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);
  const [issued, setIssued] = useState(null);

  if (!sides.length) return null;

  async function generate() {
    if (!url.trim()) {
      setIssued(null);
      setNote(t('np.reg.urlRequired'));
      return;
    }
    setBusy(true);
    setNote(null);
    try {
      const body = await nativeRequest(
        `/api/project-sides/${encodeURIComponent(side)}/registration-file`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ url: url.trim() }),
        },
      );
      setIssued(body);
    } catch (error) {
      setIssued(null);
      setNote(
        error.message === 'agent_lifecycle_scope_required'
          ? t('np.reg.scope')
          : t('np.reg.fail', { message: error.message }),
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel" data-side-registration>
      <h2 style={{ marginTop: 0 }}>{t('np.reg.title')}</h2>

      <div className="btn-row">
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.reg.side')}{' '}
          <select value={side} onChange={(e) => setSide(e.target.value)}>
            {sides.map((s) => <option key={s.id} value={s.id}>{s.id}</option>)}
          </select>
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.reg.urlLabel')}{' '}
          <input
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="http://host.docker.internal:13443"
            style={{ minWidth: 280 }}
          />
        </label>
        <button type="button" className="btn" disabled={busy} onClick={generate}>
          {busy ? t('np.reg.busy') : t('np.reg.generate')}
        </button>
      </div>
      {note && <p role="alert" style={{ color: 'var(--warn, inherit)' }}>{note}</p>}

      {issued && (
        <div style={{ marginTop: 12 }}>
          {issued.staged === true
            ? <p className="notice">{t('np.reg.staged')}</p>
            : <p className="dim">{t('np.reg.written', { mode: issued.mode ?? '' })}</p>}
          <p className="mono-s" style={{ wordBreak: 'break-all' }}>{issued.path}</p>
          <dl className="kv">
            <dt>{t('np.reg.representative')}</dt><dd className="mono-s">{issued.representative}</dd>
            <dt>{t('np.reg.namespace')}</dt><dd className="mono-s">{issued.namespace}</dd>
            <dt>{t('np.reg.urlLabel')}</dt><dd className="mono-s">{issued.url}</dd>
            <dt>as_token</dt><dd className="mono-s">{issued.asTokenFingerprint}</dd>
            <dt>hs_token</dt><dd className="mono-s">{issued.hsTokenFingerprint}</dd>
          </dl>
          <p className="dim">{t('np.reg.fingerprintNote')}</p>
          <h3>{t('np.reg.next')}</h3>
          <ul className="steps">
            {(issued.nextSteps ?? []).map((s, i) => (
              <li key={s}><span className="stg">{i + 1}</span><div>{s}</div></li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
