'use client';

/*
 * Task #51: the "Test connection" action — the TS dashboard's probe
 * (mockup/app/projects/new/page.jsx `probeManual`, `backend-v2.js:9666`):
 * POST /api/matrix/probe with the operator-supplied address, then render the
 * verdict (reachable / reason / versions) and the callback-check's honest
 * "no appservice port" note. Read-only: no credential passes through the
 * browser, and the probe is a GET /_matrix/client/versions the backend makes,
 * so holding this panel open grants nothing.
 */
import { useState } from 'react';
import { useT } from '@/components/Prefs';
import { nativeRequest } from '@/lib/native-api';

export default function ConnectionControl({ sides }) {
  const t = useT();
  const [url, setUrl] = useState('');
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);

  async function test() {
    const target = url.trim();
    if (!target) {
      setResult(null);
      setError(t('np.conn.urlRequired'));
      return;
    }
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const probe = await nativeRequest('/api/matrix/probe', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: target }),
      });
      let callback = null;
      try {
        callback = await nativeRequest('/api/matrix/callback-check', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ homeserver_url: target }),
        });
      } catch {
        callback = null;
      }
      setResult({ probe, callback });
    } catch (err) {
      setResult(null);
      setError(t('np.conn.fail', { message: err.message }));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel" data-test-connection>
      <h2 style={{ marginTop: 0 }}>{t('np.conn.title')}</h2>
      <div className="btn-row">
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.conn.urlLabel')}{' '}
          <input
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://matrix.example.com"
            style={{ minWidth: 280 }}
            list="conn-sides"
          />
        </label>
        <datalist id="conn-sides">
          {sides.map((s) => <option key={s.id} value={`https://${s.id}`} />)}
        </datalist>
        <button type="button" className="btn" disabled={busy} onClick={test}>
          {busy ? t('np.conn.busy') : t('np.conn.test')}
        </button>
      </div>

      {error && <p role="alert" style={{ color: 'var(--warn, inherit)' }}>{error}</p>}

      {result?.probe && (
        <div style={{ marginTop: 12 }}>
          <p className={result.probe.probe?.reachable ? 'notice' : 'warn-text'} role="status">
            {result.probe.probe?.reachable
              ? t('np.conn.reachable', { origin: result.probe.origin })
              : t('np.conn.unreachable', { origin: result.probe.origin, reason: result.probe.probe?.reason ?? '' })}
          </p>
          {Array.isArray(result.probe.probe?.versions) && result.probe.probe.versions.length > 0 && (
            <p className="dim">
              {t('np.conn.versions')}: {result.probe.probe.versions.join(', ')}
            </p>
          )}
          {result.callback && result.callback.applicable === false && (
            <p className="dim">{t('np.conn.noPort', { reason: result.callback.reason })}</p>
          )}
        </div>
      )}
    </section>
  );
}
