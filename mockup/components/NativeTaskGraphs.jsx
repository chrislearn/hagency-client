'use client';

/*
 * Operator task graphs (board #47): the native console's task-graph page,
 * over /console/api/task-graphs (TS parity: backend-v2.js:15974-16020).
 *
 * The page is self-contained (its own fetch/refresh state, like the other
 * native panels): a status-filtered list, a create form with a dynamic
 * node list, and a detail view whose node rows carry the TS node PATCH
 * (status/result/error) and whose header carries DELETE-as-cancel. The
 * control stays in-flight until the server answers; a failure leaves the
 * prior state on screen (the same rule the alerts page keeps).
 */
import { useCallback, useEffect, useState } from 'react';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { NativeAccessNotice } from '@/components/NativeUsage';
import { Toast, useToast } from '@/components/Toast';
import { useT } from '@/components/Prefs';
import {
  fetchTaskGraphs,
  createTaskGraph,
  deleteTaskGraph,
  updateTaskGraphNode,
} from '@/lib/native-api';

const GRAPH_STATUSES = ['active', 'complete', 'failed', 'cancelled'];
const NODE_STATUSES = ['pending', 'dispatched', 'active', 'complete', 'failed', 'skipped', 'cancelled'];
const EMPTY_NODE = { id: '', assignee: '', description: '', depends_on: '' };

export default function NativeTaskGraphs() {
  const t = useT();
  const [phase, setPhase] = useState('loading');
  const [graphs, setGraphs] = useState([]);
  const [status, setStatus] = useState('');
  const [selectedId, setSelectedId] = useState(null);
  const [busy, setBusy] = useState(false);
  const [toast, say] = useToast();
  const [form, setForm] = useState({ owner: 'operator', label: '', nodes: [{ ...EMPTY_NODE }] });

  const refresh = useCallback(async (statusFilter = status) => {
    try {
      const rows = await fetchTaskGraphs(statusFilter);
      setGraphs(rows);
      setPhase('ready');
    } catch (error) {
      setPhase(error.message === 'console_access_required' ? 'access' : 'error');
    }
  }, [status]);
  useEffect(() => { refresh(status); }, [refresh, status]);

  if (phase === 'access') return <>
    <PageHead title={t('tg.title')} sub={t('tg.nativeReadonly')}><NativeStatusStrip /></PageHead>
    <NativeAccessNotice />
  </>;
  if (phase === 'error') {
    return (
      <section className="panel" role="alert">
        <h2>{t('nu.failed')}</h2>
        <button className="btn" onClick={() => refresh()}>{t('nu.refresh')}</button>
      </section>
    );
  }

  const selected = graphs.find((g) => g.id === selectedId) ?? graphs[0] ?? null;

  async function submitCreate(event) {
    event.preventDefault();
    const nodes = {};
    for (const node of form.nodes) {
      if (!node.id.trim()) continue;
      nodes[node.id.trim()] = {
        id: node.id.trim(),
        assignee: node.assignee.trim(),
        description: node.description.trim(),
        depends_on: node.depends_on.split(',').map((d) => d.trim()).filter(Boolean),
      };
    }
    setBusy(true);
    try {
      const graph = await createTaskGraph({ owner: form.owner, label: form.label, nodes });
      say('ok', t('tg.created', { id: graph.id }));
      setSelectedId(graph.id);
      await refresh();
    } catch (error) {
      say('fail', t('tg.createFailed', { why: error.message }));
    }
    setBusy(false);
  }

  async function cancel(graph) {
    setBusy(true);
    try {
      await deleteTaskGraph(graph.id);
      say('ok', t('tg.cancelled', { id: graph.id }));
      await refresh();
    } catch (error) {
      say('fail', t('tg.cancelFailed', { why: error.message }));
    }
    setBusy(false);
  }

  async function patchNode(graphId, nodeId, patch) {
    setBusy(true);
    try {
      await updateTaskGraphNode(graphId, nodeId, patch);
      say('ok', t('tg.nodeUpdated', { id: nodeId }));
      await refresh();
    } catch (error) {
      say('fail', t('tg.nodeFailed', { why: error.message }));
    }
    setBusy(false);
  }

  return (
    <div>
      <PageHead title={t('tg.title')} sub={t('tg.nativeReadonly')}><NativeStatusStrip /></PageHead>
      <section className="panel">
        <label>
          {t('tg.status')}{' '}
          <select value={status} onChange={(e) => setStatus(e.target.value)}>
            <option value="">{t('common.all')}</option>
            {GRAPH_STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </label>
        {graphs.length === 0 ? (
          <div className="empty"><div className="big">{t('tg.empty')}</div><div className="small">{t('tg.emptyHint')}</div></div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>{t('tg.label')}</th><th>{t('tg.owner')}</th><th>{t('tg.status')}</th>
                <th>{t('tg.nodes')}</th><th>{t('tg.updated')}</th>
              </tr>
            </thead>
            <tbody>
              {graphs.map((g) => (
                <tr key={g.id}
                  onClick={() => setSelectedId(g.id)}
                  aria-current={selected && selected.id === g.id ? 'true' : undefined}>
                  <td>{g.label}</td><td>{g.owner}</td><td>{g.status}</td>
                  <td>{Object.keys(g.nodes).length}</td><td>{g.updatedAt}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {selected && (
        <section className="panel">
          <h3>{t('tg.detail')}: {selected.label} <small>({selected.id})</small></h3>
          <p>
            {t('tg.owner')}: {selected.owner} · {t('tg.status')}: {selected.status} ·{' '}
            {t('tg.createdAt')}: {selected.createdAt}
            {selected.completedAt ? <> · {t('tg.completedAt')}: {selected.completedAt}</> : null}
          </p>
          {selected.status === 'active' && (
            <button className="btn" disabled={busy} onClick={() => cancel(selected)}>
              {t('tg.cancel')}
            </button>
          )}
          <table>
            <thead>
              <tr>
                <th>{t('tg.node')}</th><th>{t('tg.assignee')}</th><th>{t('tg.nodeStatus')}</th>
                <th>{t('tg.dependsOn')}</th><th>{t('tg.messageId')}</th><th>{t('tg.dispatchedAt')}</th>
              </tr>
            </thead>
            <tbody>
              {Object.values(selected.nodes).map((n) => (
                <NodeRow key={n.id} graph={selected} node={n} busy={busy}
                  statuses={NODE_STATUSES} onPatch={patchNode} t={t} />
              ))}
            </tbody>
          </table>
        </section>
      )}

      <section className="panel">
        <h2>{t('tg.create')}</h2>
        <form onSubmit={submitCreate}>
          <label>{t('tg.owner')}{' '}
            <input value={form.owner}
              onChange={(e) => setForm({ ...form, owner: e.target.value })} required />
          </label>{' '}
          <label>{t('tg.label')}{' '}
            <input value={form.label}
              onChange={(e) => setForm({ ...form, label: e.target.value })} required />
          </label>
          {form.nodes.map((node, i) => (
            <fieldset key={i}>
              <legend>{t('tg.nodeLegend', { n: i + 1 })}</legend>
              <label>{t('tg.nodeId')}{' '}
                <input value={node.id} onChange={(e) => setNode(i, 'id', e.target.value)} required />
              </label>{' '}
              <label>{t('tg.assignee')}{' '}
                <input value={node.assignee} onChange={(e) => setNode(i, 'assignee', e.target.value)} required />
              </label>{' '}
              <label>{t('tg.description')}{' '}
                <input value={node.description} onChange={(e) => setNode(i, 'description', e.target.value)} required />
              </label>{' '}
              <label>{t('tg.dependsOn')}{' '}
                <input value={node.depends_on} onChange={(e) => setNode(i, 'depends_on', e.target.value)} />
              </label>
            </fieldset>
          ))}
          <div className="form-foot">
            <button type="button" className="btn"
              onClick={() => setForm({ ...form, nodes: [...form.nodes, { ...EMPTY_NODE }] })}>
              + {t('tg.addNode')}
            </button>
            <button type="submit" className="btn primary" disabled={busy}>{t('tg.submitCreate')}</button>
          </div>
        </form>
      </section>
      <Toast toast={toast} />
    </div>
  );

  function setNode(index, field, value) {
    const nodes = form.nodes.map((n, i) => (i === index ? { ...n, [field]: value } : n));
    setForm({ ...form, nodes });
  }
}

function NodeRow({ graph, node, busy, statuses, onPatch, t }) {
  const [status, setStatus] = useState(node.status);
  const [result, setResult] = useState('');
  const [error, setError] = useState('');
  const patch = {};
  if (status !== node.status) patch.status = status;
  if (result.trim()) {
    try { patch.result = JSON.parse(result); } catch { patch.result = result; }
  }
  if (error.trim()) patch.error = error.trim();
  return (
    <tr>
      <td>{node.id}</td>
      <td>{node.assignee}</td>
      <td>
        <select value={status} disabled={busy} onChange={(e) => setStatus(e.target.value)}>
          {statuses.map((s) => <option key={s} value={s}>{s}</option>)}
        </select>
      </td>
      <td>{(node.depends_on || []).join(', ')}</td>
      <td>{node.message_id ?? ''}</td>
      <td>{node.dispatchedAt ?? ''}</td>
      <td>
        <input placeholder={t('tg.result')} value={result}
          onChange={(e) => setResult(e.target.value)} />
        <input placeholder={t('tg.error')} value={error}
          onChange={(e) => setError(e.target.value)} />
        <button className="btn" disabled={busy || Object.keys(patch).length === 0}
          onClick={() => onPatch(graph.id, node.id, patch)}>
          {t('tg.submitNode')}
        </button>
      </td>
    </tr>
  );
}
