'use client';

// Optional compatibility path for an administrator-provided configuration.

import { useState } from 'react';
import ServerLoginControl from '@/components/ServerLoginControl';
import { useData } from '@/components/Data';
import { useT } from '@/components/Prefs';
import { errorText } from '@/lib/i18n';
import { importPalpo } from '@/lib/native-api';

const FLEET = /^hf_[0-9a-f]{32}$/;

export default function ImportPalpoControl() {
  const t = useT();
  const data = useData();
  const [text, setText] = useState(null);
  const [preview, setPreview] = useState(null);
  const [homeserver, setHomeserver] = useState('');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState(null);
  const [saved, setSaved] = useState(null);

  async function pick(event) {
    setNote(null); setSaved(null); setText(null); setPreview(null);
    const file = event.target.files?.[0];
    if (!file) return;
    if (file.size > 65536) { setNote(t('pi.tooLarge')); return; }
    const raw = await file.text();
    try {
      const value = JSON.parse(raw);
      const fleetId = value?.registration?.id;
      if (!FLEET.test(fleetId ?? '') || typeof value?.serverName !== 'string') throw new Error('shape');
      setPreview({ fleetId, serverName: value.serverName, outbound: value?.transport?.mode === 'outbound' });
      setText(raw);
    } catch {
      setNote(t('pi.notDownload'));
    }
    event.target.value = '';
  }

  async function save() {
    if (busy || !text || !homeserver.trim()) return;
    setBusy(true); setNote(null);
    try {
      const result = await importPalpo(text, homeserver.trim());
      setSaved(result);
      setText(null);
    } catch (error) {
      setNote(error.message === 'palpo_import_invalid' ? t('pi.invalid', { field: error.field ?? '' })
        : error.message === 'palpo_fleet_conflict' ? t('pi.conflict')
          : `${t('pi.failed')} (${errorText(t, error.message)})`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
    {data.phase !== 'access' && <ServerLoginControl />}
    <details className="panel" data-palpo-import><summary>{t('sl.manual')}</summary>
      <h2 style={{ marginTop: 0 }}>{t('pi.title')}</h2>
      <ol className="dim" style={{ fontSize: 13, paddingLeft: 18 }}>
        <li>{t('pi.step1')}</li>
        <li>{t('pi.step2')}</li>
        <li>{t('pi.step3')}</li>
      </ol>
      <div className="btn-row" style={{ flexWrap: 'wrap', gap: 8 }}>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('pi.file')}{' '}
          <input type="file" accept="application/json,.json" disabled={busy} onChange={pick} />
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('pi.homeserver')}{' '}
          <input value={homeserver} onChange={(e) => setHomeserver(e.target.value)}
            placeholder="https://matrix.example.org" style={{ minWidth: 260 }} />
        </label>
      </div>
      {preview && (
        <p className="dim" style={{ fontSize: 12 }}>
          {t('pi.preview', { server: preview.serverName })} <span className="mono-s">{preview.fleetId}</span>
          {!preview.outbound && <span className="warn-text"> · {t('pi.notOutbound')}</span>}
        </p>
      )}
      {note && <p role="alert" className="warn-text">{note}</p>}
      {saved && (
        <div role="status">
          <p>{t('pi.saved', { server: saved.serverName })}</p>
          <p className="dim" style={{ fontSize: 12 }}>
            {saved.started ? t('pi.started') : t('pi.notStarted')}
            {' · '}{t('pi.representative')} <span className="mono-s">{saved.representative}</span>
          </p>
          <p><strong>{t('pi.next')}</strong></p>
        </div>
      )}
      <div className="btn-row">
        <button type="button" className="btn" disabled={busy || !text || !homeserver.trim()} onClick={save}>
          {busy ? t('pi.saving') : t('pi.save')}
        </button>
      </div>
    </details>
    </>
  );
}
