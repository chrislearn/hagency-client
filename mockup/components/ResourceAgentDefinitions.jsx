'use client';

import { useState } from 'react';
import { useT } from '@/components/Prefs';
import { send } from '@/lib/api';

/*
 * Agent definitions editor — task #19's fourth console control. The offer caps
 * editor (capability page), the whitelist (engagements page) and resource
 * delete (config page) already existed; definitions had a mapper in api.js
 * (`agentDefinitions: p.agentDefinitions ?? []`) and no control anywhere, so
 * a definition could only be created by talking to the API directly.
 *
 * The three TS routes are one verb per row (backend-v2.js:15822-15833):
 * POST   framework-presets/:id/agents               create
 * PUT    framework-presets/:id/agents/:definitionId update
 * DELETE framework-presets/:id/agents/:definitionId remove
 * All three answer { ok: true, definition } — the TS handler shares one body.
 */
export default function ResourceAgentDefinitions({ preset, roles, live, refresh }) {
  const t = useT();
  const [name, setName] = useState('');
  const [role, setRole] = useState('');
  const [busy, setBusy] = useState(null);
  const [error, setError] = useState(null);
  const defs = preset.agentDefinitions ?? [];

  async function mutate(path, method, body) {
    setBusy(path);
    setError(null);
    const result = await send(path, { method, body });
    setBusy(null);
    if (!result.ok) { setError(result.error); return false; }
    await refresh();
    return true;
  }

  return <div data-testid={`resource-definitions-${preset.id}`}>
    <p className="note">{t('rad.help')}</p>
    {defs.length > 0 && (
      <table className="tbl" style={{ marginTop: 8 }}>
        <thead>
          <tr>
            <th>{t('col.agent')}</th>
            <th>{t('col.role')}</th>
            <th>{t('rad.enabled')}</th>
            <th>{t('rad.status')}</th>
            <th>{t('rad.active')}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {defs.map((d) => (
            <tr key={d.id}>
              <td className="mono-s">{d.name}</td>
              <td className="dim">{d.role}</td>
              <td>
                <button
                  className="btn"
                  disabled={!live || busy === `${d.id}`}
                  onClick={() => mutate(`framework-presets/${encodeURIComponent(preset.id)}/agents/${encodeURIComponent(d.id)}`, 'PUT', { enabled: !d.enabled })}
                >
                  {t(d.enabled ? 'rad.disable' : 'rad.enable')}
                </button>
              </td>
              <td><span className={`badge ${d.status === 'provisioned' ? 'ok' : d.status === 'reserved' ? 'warn-b' : ''}`}>{t(`rad.${d.status}`)}</span></td>
              <td>{d.activeEngagements}</td>
              <td>
                <button
                  className="btn"
                  disabled={!live || busy === `${d.id}`}
                  onClick={() => mutate(`framework-presets/${encodeURIComponent(preset.id)}/agents/${encodeURIComponent(d.id)}`, 'DELETE')}
                >
                  {t('act.delete')}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    )}
    {defs.length === 0 && <p className="dim">{t('rad.none')}</p>}
    <div className="btn-row" style={{ marginTop: 8 }}>
      <input
        placeholder={t('rad.name')}
        value={name}
        disabled={!live}
        onChange={(e) => setName(e.target.value)}
        style={{ width: 180 }}
      />
      <select value={role} disabled={!live} onChange={(e) => setRole(e.target.value)}>
        <option value="">{t('rad.pickRole')}</option>
        {roles.map((r) => <option key={r} value={r}>{r}</option>)}
      </select>
      <button
        className="btn"
        disabled={!live || busy === 'create' || !name || !role}
        onClick={async () => {
          const okDone = await mutate(`framework-presets/${encodeURIComponent(preset.id)}/agents`, 'POST', { name, role });
          if (okDone) { setName(''); setRole(''); }
        }}
      >
        {t('rad.add')}
      </button>
    </div>
    {error && <p role="alert" className="warn-text">{error}</p>}
  </div>;
}
