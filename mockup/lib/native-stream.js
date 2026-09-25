'use client';

/* #26 live updates: the SSE client hook. TS parity: the retained dashboard
 * consumed /api/stream with the browser's EventSource (lib/backend/
 * sse-adapter.js:20 served `text/event-stream`, `:\n\n` on connect, a
 * `event:`/`data:` frame per broadcast, 30 s comment keepalive). The native
 * route serves the same wire at /console/api/stream; the snapshot and
 * events reads are the routerStore.snapshot()/eventsAfter() parity
 * (store.ts:3655/:3811) for a cold-start cursor.
 *
 * The hook only SIGNALS; it never fetches page data itself. An event says
 * "this category changed"; the page refetches its own bounded read — the
 * same division the retained dashboard used. */
import { useEffect, useRef } from 'react';

export const STREAM_CATEGORIES = ['agents', 'tasks', 'alerts'];

/* #59: the TS named-event vocabulary (backend-v2.js broadcastSSE sites).
 * Each event maps to the page whose data changed — the stream also keeps
 * the three category signals, but the named events are the parity wire:
 * the dashboard refreshes FROM them, exactly as the retained dashboard
 * consumed /api/stream. */
export const NAMED_EVENTS = {
  task_created: 'tasks',
  task_updated: 'tasks',
  task_deleted: 'tasks',
  alert_created: 'alerts',
  alert_updated: 'alerts',
  alert_resolved: 'alerts',
  alert_deleted: 'alerts',
  approval_requested: 'agents',
  approval_verdict: 'agents',
  agent_blocked: 'agents',
  agent_recovered: 'agents',
  message: 'agents',
};

export function snapshotRequest() {
  return '/console/api/stream/snapshot';
}

/** Subscribe to the live stream. `onChange(category)` fires on the first
 *  event for a category whose feed version moved since the last event.
 *  #59: the TS named events also fire it — each name maps to the page whose
 *  data changed (NAMED_EVENTS), so the dashboard refreshes FROM the named
 *  events exactly as the retained dashboard consumed /api/stream.
 *  Returns nothing; the subscription is tied to the component's lifetime
 *  and re-created when `active` flips. */
export function useLiveStream(onChange, active = true) {
  const notify = useRef(onChange);
  notify.current = onChange;
  useEffect(() => {
    if (!active || typeof window === 'undefined' || typeof EventSource === 'undefined') return undefined;
    const source = new EventSource('/console/api/stream', { withCredentials: true });
    const seen = new Map();
    const changed = (category) => (event) => {
      let version = null;
      try { version = JSON.parse(event.data).feed_version ?? null; } catch { version = null; }
      if (version !== null && seen.get(category) === version) return;
      if (version !== null) seen.set(category, version);
      notify.current?.(category);
    };
    const handlers = new Map(STREAM_CATEGORIES.map((category) => [category, changed(category)]));
    // Named events carry the entity as the payload (no feed_version): each
    // occurrence is the news, so they notify without dedupe.
    for (const name of Object.keys(NAMED_EVENTS)) {
      const handler = () => notify.current?.(NAMED_EVENTS[name]);
      handlers.set(name, handler);
    }
    for (const [category, handler] of handlers) source.addEventListener(category, handler);
    return () => {
      for (const [category, handler] of handlers) source.removeEventListener(category, handler);
      source.close();
    };
  }, [active]);
}
