'use client';
import { useEffect, useState } from 'react';
import { useT } from '@/components/Prefs';

export default function ServerLoginControl() {
  const t = useT();
  const [server, setServer] = useState('');
  const [name, setName] = useState('Hagency Client');
  const [info, setInfo] = useState(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(null);
  useEffect(() => {
    let alive = true;
    async function poll() {
      try {
        const response = await fetch('/console/server-login', { credentials: 'same-origin', cache: 'no-store', signal: AbortSignal.timeout(10000) });
        if (!response.ok) return;
        const value = await response.json();
        if (!alive) return;
        setInfo(value);
        setServer(current => current || value.server || '');
        setName(current => value.name || current);
      } catch { /* An unavailable local host does not imply a connected server. */ }
    }
    poll();
    const timer = setInterval(poll, 3000);
    return () => { alive = false; clearInterval(timer); };
  }, []);
  async function login(event) {
    event.preventDefault();
    if (busy) return;
    setBusy(true); setError(null);
    try {
      const response = await fetch('/console/server-login/start', {
        method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ server: server.trim(), name: name.trim() }), signal: AbortSignal.timeout(45000),
      });
      const value = await response.json();
      if (!response.ok) throw new Error(value.code || 'server_request_failed');
      window.location.assign(value.url);
    } catch (failure) { setError(failure.message); setBusy(false); }
  }
  const status = info?.status;
  const issue = error || (status?.state === 'failed' ? status.code : null);
  const known = ['local_access_required', 'owner_mismatch', 'self_service_disabled', 'hafleet_limit', 'invalid_server', 'configuration_import_failed', 'transport_unavailable'];
  return <section className="panel" data-server-login>
    <h2>{t('sl.title')}</h2><p className="dim">{t('sl.help')}</p>
    <form onSubmit={login}>
      <div className="field"><label htmlFor="hagency-server">{t('sl.server')}</label>
        <input id="hagency-server" type="url" required value={server} disabled={busy || info?.configured}
          placeholder="https://hagency.example.org" onChange={event => setServer(event.target.value)} /></div>
      <div className="field"><label htmlFor="hafleet-name">{t('sl.name')}</label>
        <input id="hafleet-name" required maxLength={128} value={name} disabled={busy || info?.configured} onChange={event => setName(event.target.value)} /></div>
      {issue && <p role="alert">{t(known.includes(issue) ? `sl.${issue}` : 'sl.failure')}</p>}
      {status && ['connected', 'verifying', 'saved'].includes(status.state) && <p role="status">{t(`sl.${status.state}`)}</p>}
      <button className="btn" type="submit" disabled={busy || !server.trim() || !name.trim()}>{t(busy ? 'sl.opening' : 'sl.login')}</button>
    </form>
  </section>;
}
