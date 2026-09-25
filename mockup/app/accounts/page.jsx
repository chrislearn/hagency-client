'use client';

import { useEffect, useState } from 'react';
import NativeAccounts from '@/components/NativeAccounts';
import { enrollAccountResource, exchangeAccess, fetchAccounts, prepareAccount, retireAccount } from '@/lib/native-api';

/* The native-only accounts page: no retained counterpart exists. A
 * non-document with no query string, deliberately outside the console's
 * five-document exception — a foreign origin cannot even navigate here.
 *
 * The three account mutations live here, not in the row renderer: the page
 * owns the account list, so every mutation reply (the same one-row envelope
 * as the single read) replaces its row in place. A mutation whose reply
 * fails the validator leaves the page listing the server's current state —
 * the next load re-reads; the write itself is never retried. */
export default function AccountsPage() {
  const [state, setState] = useState({ phase: 'loading', accounts: null });
  const [action, setAction] = useState(null);

  const load = async () => {
    const value = await fetchAccounts();
    setState({ phase: 'ready', accounts: value.accounts });
  };

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await exchangeAccess(window.location, window.history);
        const value = await fetchAccounts();
        if (!cancelled) setState({ phase: 'ready', accounts: value.accounts });
      } catch {
        if (!cancelled) setState({ phase: 'access', accounts: null });
      }
    })();
    return () => { cancelled = true; };
  }, []);

  /* One shared runner: pending → saved/refused word, the returned row
   * spliced into the list by id (prepare appends). The refused kinds mirror
   * the server's refusal words; an unexpected failure maps to `unknown` —
   * the outcome cannot be inferred locally. */
  const run = async (label, act) => {
    setAction({ kind: 'pending', label });
    try {
      const account = await act();
      setState((s) => ({
        phase: 'ready',
        accounts: s.accounts.some((a) => a.id === account.id)
          ? s.accounts.map((a) => (a.id === account.id ? account : a))
          : [...s.accounts, account],
      }));
      setAction({ kind: 'saved', label });
    } catch (error) {
      const known = { account_revision_conflict: 'conflict', account_state_conflict: 'conflict', busy: 'busy' };
      setAction({ kind: known[error.message] ?? (['invalid_account_command', 'not_found', 'account_scope_required', 'console_access_required'].includes(error.message) ? 'refused' : 'unknown'), label, error: error.message });
      try { await load(); } catch { /* the read error surfaces on the next interaction */ }
    }
  };

  return (
    <NativeAccounts
      phase={state.phase}
      accounts={state.accounts}
      action={action}
      onPrepare={() => run('prepare', prepareAccount)}
      onEnroll={(account, model, reasoning) => run(`enroll ${account.ordinal}`, () => enrollAccountResource(account.id, model, reasoning, account.revision))}
      onRetire={(account) => run(`retire ${account.ordinal}`, () => retireAccount(account.id))}
    />
  );
}
