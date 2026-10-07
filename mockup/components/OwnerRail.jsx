'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { PrefsSwitch, usePrefs } from '@/components/Prefs';
import OwnerAccountMenu from '@/components/OwnerAccountMenu';

export default function OwnerRail() {
  const pathname = (usePathname() || '/').replace(/^\/console(?=\/|$)/, '').replace(/\/$/, '') || '/';
  const { locale } = usePrefs();
  const zh = locale.startsWith('zh');
  return <aside className="rail owner-rail">
    <div className="brand">Hagency Client</div>
    <nav className="rail-scroll" aria-label={zh ? '客户端导航' : 'Client navigation'}><ul className="rail-list">
      <li><Link className="fleet-row" href="/agents-owned/" aria-current={pathname === '/' || pathname === '/agents-owned' ? 'page' : undefined}>{zh ? '我拥有的 Agents' : 'My agents'}</Link></li>
      <li><Link className="fleet-row" href="/projects/" aria-current={pathname.startsWith('/projects') ? 'page' : undefined}>Projects</Link></li>
    </ul></nav>
    <p className="dim">{zh ? '使用自己的 Matrix 身份与本地 Codex 资源。' : 'Your Matrix identity and local Codex resources.'}</p>
    <PrefsSwitch />
    <OwnerAccountMenu />
  </aside>;
}
