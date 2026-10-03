'use client';

/*
 * ADR-189: until setup is complete (a configured coding agent, a Palpo
 * connection and an offered resource), every native console page shows a
 * one-line link to the Setup page. It disappears once all three are done.
 */
import { useEffect, useState } from 'react';
import { usePathname } from 'next/navigation';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import { fetchSetup } from '@/lib/native-api';

export default function SetupBanner() {
  const t = useT();
  const data = useData();
  const pathname = usePathname();
  const [incomplete, setIncomplete] = useState(false);
  useEffect(() => {
    // Only for a signed-in console: before sign-in the read would be refused.
    if (!data.nativeConsole || data.phase !== 'ready') return undefined;
    let live = true;
    fetchSetup()
      .then((s) => { if (live) setIncomplete(s.applicable === true && !(s.runtimeConfigured && s.palpo?.imported && (s.offer?.resources ?? 0) > 0)); })
      .catch(() => { /* no session yet or not a fleet: no banner */ });
    return () => { live = false; };
  }, [data.nativeConsole, data.phase, pathname]);
  if (!incomplete || /\/setup\/?$/.test(pathname ?? '')) return null;
  return <p className="note setup-banner" role="status">{t('st.banner')} <a href="/console/setup/">{t('nav.setup')}</a></p>;
}
