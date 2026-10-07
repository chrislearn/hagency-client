'use client';

import { useEffect, useRef, useState } from 'react';
import { usePrefs } from '@/components/Prefs';

export default function OwnerAccountMenu() {
  const { locale } = usePrefs();
  const zh = locale.startsWith('zh');
  const [account, setAccount] = useState(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const anchor = useRef(null);
  const trigger = useRef(null);
  const mutation = useRef(false);
  const epoch = useRef(0);

  useEffect(() => {
    let alive = true;
    let polling = false;
    async function check() {
      if (polling || mutation.current) return;
      polling = true;
      const started = epoch.current;
      try {
        const status = await fetch('/console/server-login', { credentials: 'same-origin', cache: 'no-store', signal: AbortSignal.timeout(10000) });
        if (!status.ok) throw new Error();
        const info = await status.json();
        const admission = await fetch('/console/api/owned-agents', { credentials: 'same-origin', cache: 'no-store', signal: AbortSignal.timeout(10000) });
        const active = Array.isArray(info.profiles) ? info.profiles.find(profile => profile.profileId === info.activeProfileId) : null;
        const mxid = active?.mxid;
        const owner = admission.ok ? await admission.json() : null;
        const sameServer = active && typeof active.server === 'string' && typeof info.server === 'string' && new URL(active.server).origin === new URL(info.server).origin;
        if (alive && started === epoch.current) {
          setAccount(admission.ok && typeof mxid === 'string' && owner?.ownerMxid === mxid && sameServer
            ? { mxid, server: info.server, name: info.name || 'Hagency Client' } : null);
        }
      } catch { if (alive && started === epoch.current) setAccount(null); }
      finally { polling = false; }
    }
    const changed = () => { epoch.current += 1; setAccount(null); setOpen(false); check(); };
    check();
    const timer = setInterval(check, 10000);
    window.addEventListener('hagency-owner-auth-changed', changed);
    return () => { alive = false; clearInterval(timer); window.removeEventListener('hagency-owner-auth-changed', changed); };
  }, []);

  useEffect(() => {
    if (!open) return;
    anchor.current?.querySelector('[role="menuitem"]')?.focus();
    const outside = event => { if (!anchor.current?.contains(event.target) && !mutation.current) setOpen(false); };
    const escape = event => { if (event.key === 'Escape' && !mutation.current) { setOpen(false); trigger.current?.focus(); } };
    document.addEventListener('pointerdown', outside);
    document.addEventListener('keydown', escape);
    return () => { document.removeEventListener('pointerdown', outside); document.removeEventListener('keydown', escape); };
  }, [open]);

  async function act(action) {
    if (!account || mutation.current) return;
    mutation.current = true; epoch.current += 1; setBusy(true); setError(false);
    try {
      const reauthenticate = action === 'reauthenticate';
      const response = await fetch(`/console/server-login/${reauthenticate ? 'start' : action === 'switch' ? 'switch' : 'sign-out'}`, {
        method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' },
        ...(action === 'signout' ? {} : { body: JSON.stringify(reauthenticate ? { server: account.server, name: account.name } : { profileId: null }) }),
        signal: AbortSignal.timeout(45000),
      });
      if (!response.ok) throw new Error();
      if (reauthenticate) {
        const value = await response.json();
        const url = new URL(value.url);
        if (url.origin !== new URL(account.server).origin) throw new Error();
        window.location.assign(url.href);
      } else {
        if (action === 'switch') {
          const value = await response.json();
          if (value.activeProfileId !== null || value.needsLogin !== true) throw new Error();
        }
        setAccount(null);
        window.dispatchEvent(new CustomEvent('hagency-owner-auth-changed', { detail: { authorized: false } }));
        window.location.assign('/console/login/');
      }
    } catch { setError(true); setBusy(false); mutation.current = false; }
  }
  function menuKeys(event) {
    const items = [...anchor.current.querySelectorAll('[role="menuitem"]')];
    const index = items.indexOf(document.activeElement);
    const next = event.key === 'ArrowDown' ? (index + 1) % items.length : event.key === 'ArrowUp' ? (index + items.length - 1) % items.length : event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : null;
    if (next !== null) { event.preventDefault(); items[next]?.focus(); }
  }
  if (!account) return null;
  return <div ref={anchor} className="owner-account" data-owner-account-menu>
    <button ref={trigger} type="button" className="btn owner-account-trigger" aria-haspopup="menu" aria-expanded={open} disabled={busy} onClick={() => setOpen(value => !value)}>
      <span className="owner-account-avatar" aria-hidden="true">{account.mxid.slice(1, 2).toUpperCase()}</span>
      <span className="owner-account-identity"><strong title={account.mxid}>{account.mxid}</strong><small title={account.server}>{new URL(account.server).host}</small></span>
    </button>
    {open && <div role="menu" aria-label={zh ? 'Matrix 账号' : 'Matrix account'} onKeyDown={menuKeys} className="owner-account-menu">
      <div className="owner-account-card"><strong title={account.mxid}>{account.mxid}</strong><small title={account.server}>{account.server}</small></div>
      <div>
      {[
        ['reauthenticate', zh ? '重新登录当前账号' : 'Sign in again'],
        ['switch', zh ? '切换账号' : 'Switch account'],
        ['signout', zh ? '退出登录' : 'Sign out'],
      ].map(([action, text]) => <button key={action} role="menuitem" type="button" data-account-action={action} className="btn owner-account-item" disabled={busy} onClick={() => act(action)}>{text}</button>)}
      </div>
    </div>}
    {busy && <p role="status">{zh ? '正在处理账号操作…' : 'Updating account…'}</p>}
    {error && <p role="alert">{zh ? '账号操作失败，请重试。' : 'Account action failed. Please retry.'}</p>}
  </div>;
}
