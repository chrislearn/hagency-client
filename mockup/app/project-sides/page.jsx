'use client';

import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import NativeProjectSides from '@/components/NativeProjectSides';
import SideRegistrationControl from './registration-control';

/*
 * The native project-sides route (ADR-132). Deliberately a separate path
 * from the retained /projects: that page also renders invites, whitelist
 * and contributions, which a native console has no source for, so folding
 * a native branch into it would hide most of the page. This route exists
 * only in the native build (NATIVE_MODE), so there is no retained arm.
 *
 * Task #13: the read-only observation stays read-only; the registration
 * issuer below it is the one action the TS dashboard exposes for a side,
 * carried by its own component in this directory.
 */
export default function ProjectSidesPage() {
  const data = useData();
  const t = useT();
  if (data.nativeConsole) {
    return (
      <>
        <SideRegistrationControl sides={data.sides ?? []} />
        <NativeProjectSides />
      </>
    );
  }
  /*
   * The route exists in both builds, so the retained build rendered NOTHING —
   * a blank page with a working URL, which reads as a crash. It states what the
   * page is and that native serves it, the same arm the approvals page carries
   * for the same reason.
   */
  return (
    <>
      <PageHead title={t('np.title')} />
      <section className="panel">
        <h2>{t('np.title')}</h2>
        <p>{t('np.nativeOnly')}</p>
      </section>
    </>
  );
}
