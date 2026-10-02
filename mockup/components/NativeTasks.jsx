'use client';

/*
 * Native operator tasks (board #23, TS parity: backend-v2.js:13194-13332 over
 * lib/task-store.js).
 *
 * The retained console drove /api/tasks through the operator bearer; the
 * native console serves the same operations under its session, so this page is
 * the retained page's shape with the wire keys it now gets.
 *
 * Three rules, all carried from the alerts page's own precedent:
 *
 *  - The move controls render ONLY from each row's served `next` array
 *    (hagency_store::operator_transitions), so a transition the store refuses
 *    is never offered as a control and no client-side copy of the transition
 *    map exists to drift.
 *  - The write controls render only when the SERVED session holds the
 *    configuration scope (`permissions.configureResource`); a read-only
 *    session sees the list and no button it would be refused.
 *  - Every mutation refetches afterwards, so the table shows the RESULT rather
 *    than the intention; a refusal leaves the prior row on screen.
 *
 * Status and priority words are WIRE VALUES the store matches on, so they are
 * rendered as text and never translated — the same discipline as role keys.
 */
import { useCallback, useEffect, useState } from 'react';
import Link from 'next/link';
import TechnicalDetails from '@/components/TechnicalDetails';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { Toast, useToast } from '@/components/Toast';
import { useT } from '@/components/Prefs';
import {
  fetchTasks, createTask, updateTask, deleteTask, transitionTask, commentTask,
} from '@/lib/native-api';

const STATUSES = ['created', 'accepted', 'in_progress', 'blocked', 'done'];
const PRIORITIES = ['p0', 'p1', 'p2', 'p3'];
const GRANULARITIES = ['epic', 'task', 'subtask'];

const emptyDraft = () => ({
  title: '', description: '', priority: 'p2', granularity: 'task', assignee: '', labels: '',
});

export default function NativeTasks() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [error, setError] = useState(null);
  const [refreshing, setRefreshing] = useState(false);
  const [rows, setRows] = useState([]);
  const [permissions, setPermissions] = useState({ configureResource: false });
  const [unavailable, setUnavailable] = useState([]);
  const [filters, setFilters] = useState({ assignee: '', status: '', priority: '' });
  const [selectedId, setSelectedId] = useState(null);
  const [busy, setBusy] = useState(null);
  const [toast, say] = useToast();
  const [draft, setDraft] = useState(null);
  const [editingId, setEditingId] = useState(null);
  const [comment, setComment] = useState('');
  const [waiting, setWaiting] = useState({ status: null, reason: '', until: '' });

  const load = useCallback(async () => {
    try {
      const value = await fetchTasks();
      setRows(value.tasks);
      setPermissions(value.permissions);
      setUnavailable(value.unavailable);
      setError(null);
      setPhase('ready');
    } catch (err) {
      setError(err.message);
      setPhase('error');
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const refresh = async () => {
    setRefreshing(true);
    try { await load(); } finally { setRefreshing(false); }
  };

  async function act(name, work, ok) {
    setBusy(name);
    try {
      await work();
      await load();
      say('ok', ok);
    } catch (err) {
      say('fail', err.message);
    } finally {
      setBusy(null);
    }
  }

  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('tk.failed')}</h2>
        <p>{t('tk.retryHelp')}</p>
        <button className="btn" onClick={refresh}>{t('common.refresh')}</button>
      </section>
    );
  }

  const canWrite = permissions.configureResource === true;
  // The filters the retained route accepts; an empty value is "any".
  const visible = rows
    .filter((r) => !filters.assignee || r.assignee === filters.assignee)
    .filter((r) => !filters.status || r.status === filters.status)
    .filter((r) => !filters.priority || r.priority === filters.priority);
  const selected = visible.find((r) => r.id === selectedId) ?? visible[0] ?? null;
  const assignees = [...new Set(rows.map((r) => r.assignee).filter(Boolean))].sort();
  const counts = Object.fromEntries(STATUSES.map((s) => [s, rows.filter((r) => r.status === s).length]));

  function submitDraft() {
    if (!draft) return;
    const body = {
      title: draft.title,
      description: draft.description,
      priority: draft.priority,
      granularity: draft.granularity,
      assignee: draft.assignee.trim() ? draft.assignee.trim() : null,
      labels: draft.labels.split(',').map((l) => l.trim()).filter(Boolean),
    };
    if (editingId) {
      act('draft', () => updateTask(editingId, body), t('tk.didUpdate', { id: editingId }))
        .then(() => { setDraft(null); setEditingId(null); });
    } else {
      act('draft', () => createTask(body), t('tk.didCreate', { id: '…' }))
        .then(() => { setDraft(null); setEditingId(null); });
    }
  }

  return (
    <div data-native-state={phase} aria-busy={refreshing === true}>
      {refreshing && <p role="status">{t('tk.refreshing')}</p>}
      <Toast toast={toast} />

      <PageHead title={t('tk.title')} sub={t('tk.note')}>
        <Link className="btn" href="/project-board">{t('tk.boardLink')}</Link>
        {canWrite && !draft && (
          <button className="btn primary" onClick={() => { setDraft(emptyDraft()); setEditingId(null); }}>{t('tk.create')}</button>
        )}
        <NativeStatusStrip />
      </PageHead>

      <div className="cards">
        {STATUSES.map((s) => (
          <div className="card" key={s}>
            <div className="cap">{s.replaceAll('_', ' ')}</div>
            <div className={`val${s === 'blocked' && counts[s] > 0 ? ' warn' : ''}`}>{counts[s]}</div>
          </div>
        ))}
      </div>

      {!canWrite && <p className="note" style={{ marginTop: 12 }}>{t('tk.scopeRequired')}</p>}
      {unavailable.length > 0 && <TechnicalDetails><p>{t('tk.unavailableNote', { list: unavailable.join(', ') })}</p></TechnicalDetails>}

      <div className="btn-row" style={{ margin: '22px 0 12px' }}>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('tk.filter.assignee')}{' '}
          <select value={filters.assignee} onChange={(e) => { setFilters((f) => ({ ...f, assignee: e.target.value })); setSelectedId(null); }}>
            <option value="">{t('common.all')}</option>
            {assignees.map((a) => <option key={a} value={a}>{a}</option>)}
          </select>
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('tk.filter.status')}{' '}
          <select value={filters.status} onChange={(e) => { setFilters((f) => ({ ...f, status: e.target.value })); setSelectedId(null); }}>
            <option value="">{t('common.all')}</option>
            {STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </label>
        <label style={{ fontSize: 12, color: 'var(--ink-dim)' }}>
          {t('tk.filter.priority')}{' '}
          <select value={filters.priority} onChange={(e) => { setFilters((f) => ({ ...f, priority: e.target.value })); setSelectedId(null); }}>
            <option value="">{t('common.all')}</option>
            {PRIORITIES.map((p) => <option key={p} value={p}>{p}</option>)}
          </select>
        </label>
        <span style={{ flex: 1 }} />
        <span className="sub dim" style={{ fontSize: 12 }}>{t('common.shown', { a: visible.length, b: rows.length })}</span>
      </div>

      {draft && canWrite && (
        <div className="panel">
          <h3>{editingId ? editingId : t('tk.create')}</h3>
          <div className="btn-row" style={{ alignItems: 'flex-end' }}>
            <label>{t('tk.field.title')}<br />
              <input value={draft.title} maxLength={255} onChange={(e) => setDraft({ ...draft, title: e.target.value })} />
            </label>
            <label>{t('tk.field.priority')}<br />
              <select value={draft.priority} onChange={(e) => setDraft({ ...draft, priority: e.target.value })}>
                {PRIORITIES.map((p) => <option key={p} value={p}>{p}</option>)}
              </select>
            </label>
            <label>{t('tk.field.granularity')}<br />
              <select value={draft.granularity} onChange={(e) => setDraft({ ...draft, granularity: e.target.value })}>
                {GRANULARITIES.map((g) => <option key={g} value={g}>{g}</option>)}
              </select>
            </label>
            <label>{t('tk.field.assignee')}<br />
              <input value={draft.assignee} maxLength={128} onChange={(e) => setDraft({ ...draft, assignee: e.target.value })} />
            </label>
            <label>{t('tk.field.labels')} <span className="faint">({t('tk.field.labelsHint')})</span><br />
              <input value={draft.labels} onChange={(e) => setDraft({ ...draft, labels: e.target.value })} />
            </label>
          </div>
          <p style={{ marginTop: 10 }}>
            <label>{t('tk.field.description')}<br />
              <textarea rows={3} style={{ width: '100%' }} value={draft.description} maxLength={4096}
                onChange={(e) => setDraft({ ...draft, description: e.target.value })} />
            </label>
          </p>
          <div className="btn-row">
            <button className="btn primary" disabled={busy === 'draft' || !draft.title.trim()} onClick={submitDraft}>
              {editingId ? t('tk.edit') : t('tk.save')}
            </button>
            <button className="btn" onClick={() => { setDraft(null); setEditingId(null); }}>{t('tk.cancel')}</button>
          </div>
        </div>
      )}

      {visible.length === 0 ? (
        <div className="empty">
          <div className="big">{t('tk.none')}</div>
          <p className="small">{t('tk.noneNote')}</p>
        </div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>{t('col.id')}</th>
              <th>{t('tk.field.title')}</th>
              <th>{t('col.state')}</th>
              <th>{t('tk.field.priority')}</th>
              <th>{t('col.agent')}</th>
              <th className="num">⌬</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((r) => (
              <tr key={r.id} aria-selected={selected?.id === r.id} style={{ cursor: 'pointer' }}
                onClick={() => setSelectedId(r.id)}>
                <td className="faint" style={{ fontSize: 11 }}>{r.id}</td>
                <td>{r.title}</td>
                <td>{r.status}</td>
                <td>{r.priority}</td>
                <td className="dim">{r.assignee ?? '—'}</td>
                <td className="num faint">{r.comments.length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {selected && (
        <div className="panel">
          <h3>{selected.title}</h3>
          <dl className="kv">
            <dt>{t('col.state')}</dt><dd>{selected.status}</dd>
            <dt>{t('tk.field.priority')}</dt><dd>{selected.priority}</dd>
            <dt>{t('tk.field.granularity')}</dt><dd>{selected.granularity}</dd>
            <dt>{t('tk.field.assignee')}</dt><dd>{selected.assignee ?? '—'}</dd>
            <dt>{t('col.id')}</dt><dd className="faint" style={{ fontSize: 11 }}>{selected.id}</dd>
            {selected.labels.length > 0 && (<><dt>{t('tk.field.labels')}</dt><dd>{selected.labels.join(', ')}</dd></>)}
            {selected.waiting_reason && (<><dt>{t('tk.waitingReason')}</dt><dd>{selected.waiting_reason}</dd></>)}
            {selected.waiting_until && (<><dt>{t('tk.waitingUntil')}</dt><dd>{selected.waiting_until}</dd></>)}
          </dl>
          {selected.description && <p style={{ fontSize: 12.5, color: 'var(--ink-2)' }}>{selected.description}</p>}

          {canWrite && (
            <div className="btn-row" style={{ marginTop: 12 }}>
              <button className="btn" onClick={() => { setDraft({ ...emptyDraft(), ...selected, labels: selected.labels.join(', ') }); setEditingId(selected.id); }}>
                {t('tk.edit')}
              </button>
              {selected.next.map((to) => (
                <button key={to} className="btn" disabled={busy === `move:${to}`}
                  onClick={() => {
                    // `blocked` needs both metadata fields; the store refuses
                    // a transition without them, so the form is shown first.
                    if (to === 'blocked') { setWaiting({ status: to, reason: '', until: '' }); return; }
                    act(`move:${to}`, () => transitionTask(selected.id, to), t('tk.didTransition', { id: selected.id, status: to }));
                  }}>
                  → {to}
                </button>
              ))}
              <span style={{ flex: 1 }} />
              <button className="btn danger" disabled={busy === 'delete'}
                onClick={() => { if (window.confirm(t('tk.confirmDelete', { id: selected.id }))) act('delete', () => deleteTask(selected.id), t('tk.didDelete', { id: selected.id })).then(() => setSelectedId(null)); }}>
                {t('tk.delete')}
              </button>
            </div>
          )}

          {waiting.status === 'blocked' && (
            <div className="btn-row" style={{ marginTop: 10, alignItems: 'flex-end' }}>
              <label>{t('tk.waitingReason')}<br />
                <input value={waiting.reason} maxLength={1024} onChange={(e) => setWaiting({ ...waiting, reason: e.target.value })} />
              </label>
              <label>{t('tk.waitingUntil')}<br />
                <input value={waiting.until} maxLength={64} placeholder="2026-01-01T00:00:00.000Z"
                  onChange={(e) => setWaiting({ ...waiting, until: e.target.value })} />
              </label>
              <button className="btn" disabled={!waiting.reason.trim() || !waiting.until.trim()}
                onClick={() => act('move:blocked', () => transitionTask(selected.id, 'blocked', { waiting_reason: waiting.reason, waiting_until: waiting.until }), t('tk.didTransition', { id: selected.id, status: 'blocked' })).then(() => setWaiting({ status: null, reason: '', until: '' }))}>
                {t('tk.edit')}
              </button>
              <button className="btn" onClick={() => setWaiting({ status: null, reason: '', until: '' })}>{t('tk.cancel')}</button>
            </div>
          )}

          <h4 style={{ marginTop: 16 }}>{t('tk.field.comment')}</h4>
          {selected.comments.length === 0
            ? <p className="small dim">{t('tk.noneNote')}</p>
            : (
              <dl className="kv">
                {selected.comments.map((c, i) => (
                  <div key={`${c.ts}-${i}`}>
                    <dt>{c.author}</dt><dd>{c.text}<span className="faint"> · {c.ts}</span></dd>
                  </div>
                ))}
              </dl>
            )}
          {canWrite && (
            <div className="btn-row" style={{ marginTop: 10, alignItems: 'flex-end' }}>
              <input style={{ minWidth: 320 }} maxLength={4096} value={comment}
                placeholder={t('tk.commentPlaceholder')} onChange={(e) => setComment(e.target.value)} />
              <button className="btn" disabled={busy === 'comment' || !comment.trim()}
                onClick={() => act('comment', () => commentTask(selected.id, { text: comment, author: 'console' }), t('tk.didComment', { id: selected.id })).then(() => setComment(''))}>
                {t('tk.comment')}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
