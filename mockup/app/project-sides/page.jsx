'use client';

import PageHead from '@/components/PageHead';
import { useT } from '@/components/Prefs';
import { useData } from '@/components/Data';
import NativeProjectSides from '@/components/NativeProjectSides';
import ConnectionControl from './connection-control';
import SideRegistrationControl from './registration-control';
import RegisterSideControl from './register-side';

/*
 * The native project-sides route (ADR-132). Deliberately a separate path
 * from the retained /projects: that page also renders invites, whitelist
 * and contributions, which a native console has no source for, so folding
 * a native branch into it would hide most of the page.
 *
 * Task #13: the read-only observation stays read-only; the registration
 * issuer below it is the one action the TS dashboard exposes for a side,
 * carried by its own component in this directory.
 *
 * Task #45: the register-a-side form (parity row #33) joins them — the
 * server POST existed with no control reaching it.
 *
 * Task #44 item 18: this route exists in BOTH builds, so the retained build
 * rendered NOTHING — a blank page with a working URL, which reads as a crash.
 * It now states what the page is and that native serves it, the same arm the
 * approvals page carries for the same reason. The native arm renders all
 * three controls unconditionally: the guards the other side carried were
 * vestigial inside an arm that only runs when nativeConsole is true.
 */
export default function ProjectSidesPage() {
  const data = useData();
  const t = useT();
  if (data.nativeConsole) {
    return (
      <>
        {/* The page reads top-down: its header and the registered projects
            first, then the controls that add or test a project. */}
        <NativeProjectSides />
        <RegisterSideControl />
        <SideRegistrationControl sides={data.sides ?? []} />
        <ConnectionControl sides={data.sides ?? []} />
      </>
    );
  }
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
