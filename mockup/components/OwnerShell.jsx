'use client';

import { usePathname } from 'next/navigation';
import OwnerRail from '@/components/OwnerRail';
import OwnerAccessGate from '@/components/OwnerAccessGate';

export default function OwnerShell({ children }) {
  const login = /\/login\/?$/.test(usePathname() || '');
  return <div className={login ? 'owner-login-shell' : 'app'}>
    {!login && <OwnerRail />}
    <main className={login ? 'owner-login-main' : 'main'}>
      <OwnerAccessGate>{children}</OwnerAccessGate>
    </main>
  </div>;
}
