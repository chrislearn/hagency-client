'use client';

/*
 * The register-a-project-side control (board #45, parity row #33: TS
 * `mockup/app/projects/new/page.jsx:246` `createSide` → `POST /api/project-sides`).
 * The native route the task names (`native/hagency/src/console/project_sides.rs:23`)
 * already exists and is gated by the agent-lifecycle scope; the page was
 * refresh-only, so nothing reached it.
 *
 * PARITY DIVERGENCE, stated plainly (RULES: relay it, do not redesign):
 * the retained `POST /api/project-sides` takes `{server_name, api_base_url,
 * label}` (`backend-v2.js:9861-9874`, a `ProjectSide` with a caller-supplied
 * base URL and credential). The native route deserializes the fleet
 * `Registration` instead — `{fleetId, generation, serverName,
 * receptionRoomId, representativeMxid, approvalBotMxid}`, deny_unknown_fields
 * (`hagency-core/src/authority.rs:16-25`, as `tests/console/registration.rs:17-26`
 * proves). That is the route's real contract, so the form collects THAT: a
 * form shaped to the retained body would answer `invalid` (400) and "work"
 * only in the sense of always refusing. The base URL and label have no
 * native source on this route and are not invented here.
 *
 * The representative mxid is DERIVED, not typed: the store requires exactly
 * `@{fleet_id}_representative:{server_name}` (`authority.rs:47-49`), so
 * deriving it removes the one field an operator cannot get wrong.
 */
import { useState } from 'react';
import { useT } from '@/components/Prefs';
import { registerProjectSide } from '@/lib/native-api';

/* The store's own validation, mirrored so the form refuses before the wire
 * (authority.rs:36-56). A value that fails here would be a 400 there. */
const FLEET = /^hf_[0-9a-f]{32}$/;
const SERVER = /^[a-z0-9.-]+(\.[a-z0-9-]+)+$/i;
const ROOM = /^![^:\s]+:[^\s:]+$/;

export default function RegisterSideControl() {
  const t = useT();
  const [fleet, setFleet] = useState('');
  const [server, setServer] = useState('');
  const [reception, setReception] = useState('');
  const [bot, setBot] = useState('');
  const [generation, setGeneration] = useState('1');
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);
  const [saved, setSaved] = useState(null);

  const representative = FLEET.test(fleet) && SERVER.test(server)
    ? `@${fleet}_representative:${server}`
    : '';
  const gen = Number(generation);
  const invalid = !FLEET.test(fleet.trim()) ? 'np.reg.invalidFleet'
    : !SERVER.test(server.trim()) ? 'np.reg.invalidServer'
      : !ROOM.test(reception.trim()) ? 'np.reg.invalidReception'
        : !bot.trim().startsWith('@') || !bot.includes(':') ? 'np.reg.invalidBot'
          : bot.trim() === representative ? 'np.reg.sameRepresentative'
            : !Number.isSafeInteger(gen) || gen < 1 ? 'np.reg.invalidGeneration'
              : null;

  async function save() {
    if (busy || invalid) return;
    setBusy(true);
    setNote(null);
    try {
      const side = await registerProjectSide({
        fleetId: fleet.trim(),
        generation: gen,
        serverName: server.trim(),
        receptionRoomId: reception.trim(),
        representativeMxid: representative,
        approvalBotMxid: bot.trim(),
      });
      setSaved(side);
      setConfirming(false);
    } catch (error) {
      setConfirming(false);
      setNote(error.message === 'agent_lifecycle_scope_required'
        ? t('np.reg.scope')
        : error.message === 'stale_generation' ? t('np.reg.stale')
          : error.message === 'invalid_side_query' ? t('np.reg.invalid')
            : `${t('np.reg.saveFail')} (${error.message})`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel" data-side-register>
      <h2 style={{ marginTop: 0 }}>{t('np.new.title')}</h2>
      <p className="dim" style={{ fontSize: 12 }}>{t('np.new.help')}</p>

      <div className="btn-row" style={{ flexWrap: 'wrap', gap: 8 }}>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.new.fleet')}{' '}
          <input value={fleet} onChange={(e) => setFleet(e.target.value)}
            placeholder="hf_00000000000000000000000000000000" style={{ minWidth: 300 }} />
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.new.server')}{' '}
          <input value={server} onChange={(e) => setServer(e.target.value)}
            placeholder="example.test" style={{ minWidth: 160 }} />
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.new.generation')}{' '}
          <input value={generation} onChange={(e) => setGeneration(e.target.value)}
            inputMode="numeric" style={{ width: 72 }} />
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.new.reception')}{' '}
          <input value={reception} onChange={(e) => setReception(e.target.value)}
            placeholder="!reception:example.test" style={{ minWidth: 220 }} />
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('np.new.bot')}{' '}
          <input value={bot} onChange={(e) => setBot(e.target.value)}
            placeholder="@approval:example.test" style={{ minWidth: 220 }} />
        </label>
      </div>

      {/* The derived field is shown, never typed: the store fixes its shape. */}
      <p className="dim" style={{ fontSize: 12 }}>
        {t('np.new.representative')}: <span className="mono-s">{representative || '—'}</span>
      </p>

      {invalid && (fleet || server || reception || bot) && (
        <p role="alert" className="warn-text">{t(invalid)}</p>
      )}
      {note && <p role="alert" className="warn-text">{note}</p>}
      {saved && (
        <p role="status">
          {t('np.new.saved', { id: saved.id, generation: saved.generation })}
        </p>
      )}

      <div className="btn-row">
        {confirming ? (
          <span className="btn-row tight">
            <span className="dim">{t('np.new.confirm', { server: server.trim() })}</span>
            <button type="button" className="btn danger" disabled={busy} onClick={save}>{t('np.confirm')}</button>
            <button type="button" className="btn" disabled={busy} onClick={() => setConfirming(false)}>{t('np.cancel')}</button>
          </span>
        ) : (
          <button type="button" className="btn primary" disabled={busy || Boolean(invalid)}
            onClick={() => { setNote(null); setSaved(null); setConfirming(true); }}>
            {t('np.new.register')}
          </button>
        )}
      </div>
    </section>
  );
}
