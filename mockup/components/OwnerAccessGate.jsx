'use client';

import { useEffect, useRef, useState } from 'react';
import { usePathname } from 'next/navigation';
import { usePrefs } from '@/components/Prefs';
import { rememberServer } from '@/components/ServerLoginControl';

export default function OwnerAccessGate({ children }) {
  const [state, setState] = useState('checking');
  const [authorization, setAuthorization] = useState('checking');
  const [retry, setRetry] = useState(0);
  const pathname = usePathname();
  const loginPage = /\/login\/?$/.test(pathname || '');
  const { locale } = usePrefs();
  const admission = useRef(null);
  useEffect(() => {
    let alive = true;
    const enter = async () => {
      const fragment = window.location.hash;
      if (fragment) {
        window.history.replaceState(window.history.state, '', `${window.location.pathname}${window.location.search}`);
        if (!/^#access=[a-f0-9]{64}$/.test(fragment)) throw new Error('console_access_required');
        const response = await fetch('/console/session', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ticket: fragment.slice(8) }), signal: AbortSignal.timeout(10000) });
        if (!response.ok) throw new Error('console_access_required');
      }
    };
    admission.current ||= enter();
    admission.current.then(() => { if (alive) setState('ready'); }).catch(() => { if (alive) setState('access_required'); });
    return () => { alive = false; };
  }, []);
  useEffect(() => {
    if (state !== 'ready' || loginPage) return;
    let alive = true;
    let current = null;
    let remembered = false;
    setAuthorization('checking');
    async function check() {
      current?.abort();
      const controller = new AbortController();
      current = controller;
      const timer = setTimeout(() => controller.abort(), 10000);
      try {
        const response = await fetch('/console/api/owned-agents', { credentials: 'same-origin', cache: 'no-store', signal: controller.signal });
        if (!alive || current !== controller) return;
        if (response.status === 401) {
          setAuthorization('login_required');
          window.location.replace('/console/login/');
        } else {
          setAuthorization(response.ok ? 'authorized' : 'unavailable');
          if (response.ok && !remembered) {
            try {
              const login = await fetch('/console/server-login', { credentials: 'same-origin', cache: 'no-store', signal: controller.signal });
              if (alive && current === controller && login.ok) {
                const info = await login.json();
                rememberServer(info.server);
                remembered = true;
              }
            } catch { /* Remembering the address cannot revoke a verified session. */ }
          }
        }
      } catch {
        if (alive && current === controller) setAuthorization('unavailable');
      } finally { clearTimeout(timer); }
    }
    const onAuthChanged = () => { setAuthorization('checking'); check(); };
    const onVisible = () => { if (document.visibilityState === 'visible') check(); };
    check();
    const interval = setInterval(check, 10000);
    window.addEventListener('hagency-owner-auth-changed', onAuthChanged);
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      alive = false; current?.abort(); clearInterval(interval);
      window.removeEventListener('hagency-owner-auth-changed', onAuthChanged);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [state, loginPage, pathname, retry]);
  if (state === 'ready' && (loginPage || authorization === 'authorized')) return children;
  if (state === 'ready') return <section data-matrix-login-gate>
    <p role="status">{locale.startsWith('zh')
      ? (authorization === 'unavailable' ? '暂时无法验证 Matrix 登录，请重试。' : '正在验证 Matrix 登录…')
      : (authorization === 'unavailable' ? 'Could not verify Matrix sign-in. Please retry.' : 'Checking Matrix sign-in…')}</p>
    {authorization === 'unavailable' && <><button className="btn" onClick={() => setRetry(value => value + 1)}>{locale.startsWith('zh') ? '重试' : 'Retry'}</button><a href="/console/login/">{locale.startsWith('zh') ? '前往 Matrix 登录' : 'Go to Matrix sign-in'}</a></>}
  </section>;
  return <p role="status">{locale.startsWith('zh') ? (state === 'checking' ? '正在建立本机控制台会话…' : '请使用本机 hagency open 命令重新打开客户端。') : (state === 'checking' ? 'Opening the local console session…' : 'Open the client again using the local hagency open command.')}</p>;
}
