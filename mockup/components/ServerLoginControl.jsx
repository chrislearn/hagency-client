'use client';
import { useEffect, useRef, useState } from 'react';
import { usePrefs } from '@/components/Prefs';

const HISTORY_KEY = 'hagency.recent-server-origins.v1';
const DEFAULT_DEVICE_NAME = 'Hagency Client';
function serverOrigin(value) {
  if (typeof value !== 'string' || value.length > 2048) return null;
  try {
    const url = new URL(value.trim());
    const loopback = url.hostname === 'localhost' || url.hostname === '[::1]' || /^127(?:\.\d{1,3}){3}$/.test(url.hostname);
    if (url.protocol !== 'https:' && !(url.protocol === 'http:' && loopback)) return null;
    return url.origin;
  } catch { return null; }
}
function safeHistory(values) {
  return Array.isArray(values) ? [...new Set(values.map(serverOrigin).filter(Boolean))].slice(0, 8) : [];
}
function readHistory() {
  try { return safeHistory(JSON.parse(window.localStorage.getItem(HISTORY_KEY) || '[]')); }
  catch { return []; }
}
function writeHistory(values) {
  try { window.localStorage.setItem(HISTORY_KEY, JSON.stringify(safeHistory(values))); }
  catch { /* Address history is optional when browser storage is unavailable. */ }
}
export function rememberServer(value) {
  const origin = serverOrigin(value);
  const recent = readHistory();
  if (!origin) return recent;
  const values = safeHistory([origin, ...recent]);
  writeHistory(values);
  return values;
}
function notifyAuth(authorized) {
  window.dispatchEvent(new CustomEvent('hagency-owner-auth-changed', { detail: { authorized } }));
}

export default function ServerLoginControl() {
  const { t, locale, setLocale, theme, setTheme } = usePrefs();
  const [server, setServer] = useState('');
  const [history, setHistory] = useState([]);
  const [info, setInfo] = useState(null);
  const [verified, setVerified] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(null);
  const currentAuth = useRef(null);
  const serverEdited = useRef(false);
  const mutation = useRef(false);
  const epoch = useRef(0);
  useEffect(() => {
    const values = readHistory();
    setHistory(values);
    writeHistory(values);
    let alive = true;
    let polling = false;
    async function poll() {
      if (polling || mutation.current) return;
      polling = true;
      const started = epoch.current;
      try {
        const response = await fetch('/console/server-login', { credentials: 'same-origin', cache: 'no-store', signal: AbortSignal.timeout(10000) });
        if (!response.ok) return;
        const value = await response.json();
        if (!alive || started !== epoch.current) return;
        setInfo(value);
        setServer(current => serverEdited.current ? current : current || serverOrigin(value.server) || '');
        let authorized = false;
        if (value.status?.state === 'device_authorized' && value.status.deviceAuthorized === true) {
          // The installation's shared status does not authorize this browser.
          const check = await fetch('/console/api/owned-agents', { credentials: 'same-origin', cache: 'no-store', signal: AbortSignal.timeout(10000) });
          if (!alive || started !== epoch.current) return;
          if (!check.ok && check.status !== 401 && check.status !== 403) return;
          authorized = check.ok;
        }
        const becameAuthorized = currentAuth.current === false && authorized;
        setVerified(authorized);
        if (currentAuth.current !== authorized) {
          currentAuth.current = authorized;
          notifyAuth(authorized);
        }
        if (authorized) {
          const origin = serverOrigin(value.server);
          if (origin) {
            setHistory(rememberServer(origin));
          }
          if (becameAuthorized && window.location.pathname !== '/console/agents-owned/') window.location.replace('/console/agents-owned/');
        }
      } catch { /* An unavailable local host does not prove an expired session. */ }
      finally { polling = false; }
    }
    poll();
    const timer = setInterval(poll, 3000);
    return () => { alive = false; clearInterval(timer); };
  }, []);
  async function login(event) {
    event.preventDefault();
    if (busy || mutation.current) return;
    if (!canLogin) { setError('local_access_required'); return; }
    const origin = serverOrigin(server);
    if (!origin) { setError('invalid_server'); return; }
    mutation.current = true; epoch.current += 1;
    setBusy(true); setError(null);
    try {
      // A server choice never pins a saved username. Pasion determines the owner.
      if (info?.configured && info.localAccessReady !== false) await clearAccount();
      const response = await fetch('/console/server-login/start', {
        method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ server: origin, name: DEFAULT_DEVICE_NAME }), signal: AbortSignal.timeout(45000),
      });
      const value = await response.json();
      if (!response.ok) throw new Error(value.code || 'server_request_failed');
      const authorization = new URL(value.url);
      if (authorization.origin !== origin) throw new Error('invalid_server');
      window.location.assign(authorization.href);
    } catch (failure) { setError(failure.message); setBusy(false); mutation.current = false; }
  }
  async function clearAccount() {
    const response = await fetch('/console/server-login/switch', {
      method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ profileId: null }), signal: AbortSignal.timeout(45000),
    });
    const value = await response.json();
    if (!response.ok) throw new Error(value.code || 'server_request_failed');
    if (value.needsLogin !== true || value.activeProfileId !== null) throw new Error('server_request_failed');
    currentAuth.current = false; setVerified(false);
    setInfo(current => ({ ...current, activeProfileId: null, configured: false,
      server: null, name: DEFAULT_DEVICE_NAME, status: { state: 'signed_out' } }));
    notifyAuth(false);
  }
  async function useAnotherAccount() {
    if (busy || mutation.current) return;
    mutation.current = true; epoch.current += 1;
    setBusy(true); setError(null);
    try { await clearAccount(); }
    catch (failure) { setError(failure.message); }
    finally { setBusy(false); mutation.current = false; }
  }
  async function signOut() {
    if (busy || mutation.current) return;
    mutation.current = true; epoch.current += 1;
    setBusy(true); setError(null);
    try {
      const response = await fetch('/console/server-login/sign-out', {
        method: 'POST', credentials: 'same-origin', signal: AbortSignal.timeout(45000),
      });
      if (!response.ok) throw new Error('server_request_failed');
      currentAuth.current = false;
      setVerified(false);
      setInfo(current => ({ ...current, status: { state: 'signed_out' } }));
      notifyAuth(false);
    } catch (failure) { setError(failure.message); }
    finally { setBusy(false); mutation.current = false; }
  }
  const status = info?.status;
  const admittedServer = (info?.profiles || []).some(profile => serverOrigin(profile.server) === serverOrigin(server));
  const canLogin = Boolean(info) && (info.localAccessReady !== false || admittedServer);
  const issue = !canLogin && info ? 'local_access_required' : error || (status?.state === 'failed' ? status.code : null);
  const known = ['local_access_required', 'owner_mismatch', 'invalid_server', 'sign_in_required', 'profile_not_found', 'unsupported_hagency_server', 'unsupported_hagency_protocol', 'unsupported_hagency_capabilities', 'invalid_hagency_metadata', 'hagency_server_unavailable'];
  const recentServers = safeHistory([...history, ...(Array.isArray(info?.profiles) ? info.profiles.map(profile => profile.server) : []), info?.server]);
  const currentAccount = verified && serverOrigin(server) === serverOrigin(info?.server);
  return <section className="owner-login" data-server-login data-login-state={status?.state || 'loading'}>
    <header className="owner-login-header">
      <svg className="owner-login-mark" viewBox="0 0 80 80" aria-hidden="true">
        <path d="M40 7 69 24v32L40 73 11 56V24Z" fill="currentColor" />
        <path d="M27 25v30m26-30v30M27 40h26" fill="none" stroke="white" strokeWidth="7" strokeLinecap="round" />
      </svg>
      <h1>{t('sl.title')}</h1>
      <p>{t('sl.help')}</p>
      <div className="owner-login-languages" role="group" aria-label={t('prefs.language')}>
        <button type="button" aria-pressed={locale === 'en'} onClick={() => setLocale('en')}>English</button>
        <button type="button" lang="zh-CN" aria-pressed={locale === 'zh'} onClick={() => setLocale('zh')}>简体中文</button>
      </div>
    </header>
    <form onSubmit={login} className="owner-login-form">
      <div className="owner-login-server">
        <div className="owner-login-server-heading"><label htmlFor="hagency-server">{t('sl.server')}</label></div>
        <input id="hagency-server" type="url" required value={server} disabled={busy} list="hagency-server-history"
          placeholder="https://matrix.example.org" autoCapitalize="none" autoCorrect="off" spellCheck={false} onChange={event => { serverEdited.current = true; setServer(event.target.value); }} />
        {recentServers.length > 0 && <datalist id="hagency-server-history" data-server-history>
          {recentServers.map(origin => <option key={origin} value={origin}>{origin}</option>)}
        </datalist>}
        <p className="owner-login-hint">{t('sl.server_hint')}</p>
      </div>
      {issue && <p className="owner-login-error" role="alert">{t(known.includes(issue) ? `sl.${issue}` : 'sl.failure')}</p>}
      {status?.state === 'device_authorized' && !verified && <p className="owner-login-hint" role="status">{t('sl.sign_in_required')}</p>}
      {status?.state === 'signed_in' && <p className="owner-login-hint" role="status">{t('sl.signed_in')}</p>}
      {currentAccount ? <a className="owner-login-primary" href="/console/agents-owned/">{t('sl.continue')}</a> :
        <button className="owner-login-primary" type="submit" disabled={busy || !canLogin || !server.trim()}>{t(busy ? 'sl.opening' : 'sl.login')}</button>}
      <p className="owner-login-hint owner-login-browser-note">{t('sl.browser_note')}</p>
      {verified && <button className="owner-login-text-button" type="button" data-add-owner-profile disabled={busy} onClick={useAnotherAccount}>{t('sl.other_account')}</button>}
      {verified && ['signed_in', 'device_authorized'].includes(status?.state) && <button className="owner-login-text-button" type="button" disabled={busy} onClick={signOut}>{t('sl.sign_out')}</button>}
      {status?.remoteRevocationPending && <p className="owner-login-hint" role="status">{t('sl.revocation_pending')}</p>}
    </form>
    <footer className="owner-login-footer">
      <span>{t('sl.account_isolation')}</span>
      <select aria-label={t('prefs.theme')} value={theme} onChange={event => setTheme(event.target.value)}>
        <option value="system">{t('prefs.system')}</option><option value="light">{t('prefs.light')}</option><option value="dark">{t('prefs.dark')}</option>
      </select>
    </footer>
  </section>;
}
