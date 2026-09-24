'use client';

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
  return (
    <>
      {data.nativeConsole && (
        <SideRegistrationControl sides={data.sides ?? []} />
      )}
      {data.nativeConsole ? <NativeProjectSides /> : null}
    </>
  );
}
