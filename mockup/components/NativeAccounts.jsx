'use client';
import { useState } from 'react';
import PageHead from '@/components/PageHead';
import NativeStatusStrip from '@/components/NativeStatusStrip';
import { useT } from '@/components/Prefs';

/* The console account surface (MA-S3b) plus its three mutations (HIDDEN in
 * the audit, wired here): prepare ("Add account"), enrollment (the
 * first-resource act — `enroll_account_resource` creates the account-bound
 * published resource, so an empty service onboards account → resource here)
 * and retire. Every row carries exactly the six public keys; the identity
 * triple (namespace identity, identity tuple, seat) never crosses the wire.
 * The readiness word is the observed fact — `subscription`/`api_key`/
 * `unknown`, the operator's own login, never a console check — and no
 * credential byte or probe output ever crosses. State words are wire values,
 * never translated.
 *
 * Enrollment binds the SESSION's authority server-side; the form sends only
 * model, optional reasoning and the row's current revision as
 * expectedRevision — a stale revision is refused with the conflict word, no
 * stale write is applied. */
export default function NativeAccounts({ phase, error, accounts, action, onPrepare, onEnroll, onRetire, onRetry }) {
  const t = useT();
  if (phase === 'access') {
    return (
      <>
      <PageHead title={t('na.title')} sub={t('na.sub')}><NativeStatusStrip /></PageHead>
      <section className="panel" data-native-state="access">
        <h2>{t('na.access')}</h2>
        <p>{t('na.accessHelp')}</p>
        {/* Item 6: the command, shown — and it is the SAME one link as
         * everywhere else: one login grants every console action. */}
        <code>hagency console-access --state-dir &lt;state&gt; --listen &lt;address&gt;</code>
      </section>
      </>
    );
  }
  if (phase === 'error') {
    return (
      <>
      <PageHead title={t('na.title')} sub={t('na.sub')}><NativeStatusStrip /></PageHead>
      {/* Item 6: a busy or unreachable service is a read failure with a
       * retry — never misreported as "access required". */}
      <section className="panel" data-native-state="error" role="alert">
        <h2>{t('na.failed')}</h2>
        <p>{t('nu.retryHelp')}</p>
        {onRetry && <button className="btn" onClick={onRetry}>{t('common.refresh')}</button>}
      </section>
      </>
    );
  }
  if (phase === 'loading') {
    return (
      <>
      <PageHead title={t('na.title')} sub={t('na.sub')}><NativeStatusStrip /></PageHead>
      <section className="panel" data-native-state="loading" aria-busy="true">
        <h2>{t('na.loading')}</h2>
      </section>
      </>
    );
  }
  const busy = action?.kind === 'pending';
  return (
    <>
    <PageHead title={t('na.title')} sub={t('na.sub')}><NativeStatusStrip /></PageHead>
    <section className="panel" data-native-state="ready" aria-busy="false">
      <h2>{t('na.title')}</h2>
      <p>{t('na.sub')}</p>
      {action && ['conflict', 'unknown', 'busy', 'refused'].includes(action.kind) && (
        <section className="notice" data-account-action={action.kind} role="alert">
          <p><b>{action.label}</b> · {t(`na.action.${action.kind}`)}</p>
        </section>
      )}
      {action && action.kind === 'saved' && (
        <section className="notice" data-account-action="saved" role="status">
          <p><b>{action.label}</b> · {t('na.action.saved')}</p>
        </section>
      )}
      <div className="btn-row">
        <button className="btn primary" disabled={busy} onClick={onPrepare}>{t('na.add')}</button>
      </div>
      {!accounts?.length ? (
        <p>{t('na.empty')}</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>{t('na.col.ordinal')}</th>
              <th>{t('na.col.state')}</th>
              <th>{t('na.col.readiness')}</th>
              <th>{t('na.col.profile')}</th>
              <th>{t('na.col.revision')}</th>
              <th>{t('col.action')}</th>
            </tr>
          </thead>
          <tbody>
            {accounts.map((a) => (
              <AccountRow key={a.id} account={a} busy={busy} onEnroll={onEnroll} onRetire={onRetire} />
            ))}
          </tbody>
        </table>
      )}
      <p>{t('na.opacity')}</p>
    </section>
    </>
  );
}

function AccountRow({ account: a, busy, onEnroll, onRetire }) {
  const t = useT();
  const [enrolling, setEnrolling] = useState(false);
  const [model, setModel] = useState('');
  const [reasoning, setReasoning] = useState('');
  const enrollable = ['active', 'uncertain'].includes(a.state);
  return (
    <tr key={a.id} data-account-row={a.id}>
      <td data-account="ordinal">{a.ordinal}</td>
      <td data-account="state">{a.state}</td>
      <td data-account="readiness">{t(`na.readiness.${a.readiness}`)}</td>
      <td data-account="profile">{a.profile}</td>
      <td data-account="revision" title={a.revision}>{a.revision.slice(0, 12)}…</td>
      <td>
        {enrollable && !enrolling && (
          <button className="btn" disabled={busy} onClick={() => setEnrolling(true)}>{t('na.enroll.open')}</button>
        )}
        {enrollable && enrolling && (
          <span data-account-enroll={a.id}>
            <label htmlFor={`model-${a.id}`}>{t('na.enroll.model')}</label>{' '}
            <input id={`model-${a.id}`} value={model} maxLength={256} disabled={busy}
              onChange={(e) => setModel(e.target.value)} />{' '}
            <label htmlFor={`reasoning-${a.id}`}>{t('na.enroll.reasoning')}</label>{' '}
            <input id={`reasoning-${a.id}`} value={reasoning} maxLength={128} disabled={busy}
              onChange={(e) => setReasoning(e.target.value)} />{' '}
            <button className="btn primary" disabled={busy || !model.trim()}
              onClick={() => { setEnrolling(false); onEnroll(a, model.trim(), reasoning.trim() || null); }}>
              {t('na.enroll.submit')}
            </button>{' '}
            <button className="btn" disabled={busy} onClick={() => setEnrolling(false)}>{t('na.enroll.cancel')}</button>
          </span>
        )}
        {a.state !== 'retired' && (
          <button className="btn" disabled={busy} onClick={() => onRetire(a)}>{t('na.retire')}</button>
        )}
      </td>
    </tr>
  );
}
