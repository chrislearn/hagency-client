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

export function snapshotRequest() {
  return '/console/api/stream/snapshot';
}

/** Subscribe to the live stream. `onChange(category)` fires on the first
 * event for a category whose feed version moved since the last event.
 * Returns nothing; the subscription is tied to the component's lifetime
 * and re-created when `active` flips. */
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
    for (const [category, handler] of handlers) source.addEventListener(category, handler);
    return () => {
      for (const [category, handler] of handlers) source.removeEventListener(category, handler);
      source.close();
    };
  }, [active]);
}
