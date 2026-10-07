'use client';

import { useCallback, useEffect, useState } from 'react';

async function request(path, method = 'GET', body, signal) {
  const response = await fetch(`/console/api/${path}`, { method, credentials: 'same-origin', cache: 'no-store', signal: signal || AbortSignal.timeout(45000), headers: body === undefined ? {} : { 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  const value = await response.json();
  if (!response.ok) throw new Error(value.code || 'provider_request_failed');
  return value;
}
export default function OwnerRuntimeControl({ agentId, bindingId, bindingState, assignedHere, profile, text, onProviderReference }) {
  const [provider, setProvider] = useState(null);
  const [runtime, setRuntime] = useState(null);
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);
  const [optIn, setOptIn] = useState(false);
  const [reservation, setReservation] = useState('1000');
  const [effort, setEffort] = useState('medium');
  const [takeover, setTakeover] = useState(false);
  const [hostFiles, setHostFiles] = useState(false);
  const [approvals, setApprovals] = useState([]);
  const runtimePath = `owned-agents/${encodeURIComponent(agentId)}/runtime`;
  const recoveryRequired = ['ledger_recovery_required', 'reconciliation_only'].includes(runtime?.phase) || [error, runtime?.lastError].includes('ledger_recovery_required');
  const active = Boolean(runtime?.phase && !['stopped', 'idle', 'failed', 'unknown'].includes(runtime.phase));
  const refresh = useCallback(async signal => {
    const replies = await Promise.allSettled([request('owner-provider', 'GET', undefined, signal), request(runtimePath, 'GET', undefined, signal), request(`${runtimePath}/approvals`, 'GET', undefined, signal)]);
    if (replies[0].status === 'fulfilled') setProvider(replies[0].value);
    if (replies[1].status === 'fulfilled') {const summary=replies[1].value.runtime;setRuntime(summary?.bindings?.find(item=>item.bindingId===bindingId) || (summary?.bindingId===bindingId?summary:{phase:'stopped',lastError:null}));}
    if (replies[2].status === 'fulfilled') setApprovals((replies[2].value.approvals || []).filter(item=>item.proposal?.scope?.binding===bindingId));
    for (const reply of replies) if (reply.status === 'rejected') throw reply.reason;
  }, [runtimePath, bindingId]);
  useEffect(() => {
    setOptIn(false); setTakeover(false); setHostFiles(false); setRuntime(null); setApprovals([]);
    const controller = new AbortController();
    refresh(controller.signal).catch(failure => { if (failure.name !== 'AbortError') setError(failure.message); });
    return () => controller.abort();
  }, [refresh, bindingId]);
  useEffect(() => {
    if (provider?.state !== 'signing_in' && !active) return;
    const controller = new AbortController();
    let stopped = false;
    let timer;
    const poll = async () => {
      try { await refresh(controller.signal); }
      catch (failure) { if (failure.name !== 'AbortError') setError(failure.message); }
      if (!stopped) timer = setTimeout(poll, 3000);
    };
    timer = setTimeout(poll, 3000);
    return () => { stopped = true; clearTimeout(timer); controller.abort(); };
  }, [provider?.state, active, refresh]);
  async function mutate(path, method, body) {
    if (busy) return;
    setBusy(true); setError(null);
    try { await request(path, method, body); await refresh(); }
    catch (failure) { setError(failure.message); }
    finally { setBusy(false); }
  }
  const configured = provider?.authenticated && profile?.model && profile?.workspace_root && profile?.credential_ref === provider?.credentialRef;
  const validReservation = /^\d+$/.test(reservation) && Number.isSafeInteger(Number(reservation)) && Number(reservation) > 0;
  return <section className="panel" data-owner-runtime>
    <h3>{text('本机 Codex 登录与运行', 'Local Codex login and runtime')}</h3>
    <p>{text('独立的 owner 凭据保存在系统 Keychain，由 Codex 自行管理。不会复用你日常 ~/.codex 的登录，也不会上传到 Hagency 服务器。', 'Codex manages this owner’s separate credentials in the OS keyring. Your regular ~/.codex login is not reused, and credentials stay off the Hagency server.')}</p>
    <p role="status">{text('提供方登录', 'Provider login')}: {provider?.state || text('待核实', 'Unchecked')} · {text('运行状态', 'Runtime')}: {runtime?.phase === 'reconciliation_only' ? text('仅恢复已知回复，额度账本待恢复', 'Known reply recovery only; budget ledger recovery required') : runtime?.phase === 'ledger_recovery_required' ? text('额度账本待恢复', 'Budget ledger recovery required') : runtime?.phase || text('待核实', 'Unchecked')}</p>
    {recoveryRequired && <section role="alert" data-ledger-recovery>
      <h4>{text('需要恢复同一账号的完整额度账本', 'Restore this account’s complete budget ledger')}</h4>
      <p>{text('服务器上已有你的 agent 执行历史，但本机无法证明费用记录完整。新的模型请求已被阻止；已经持久化的确知回复仍可按恢复流程投递。', 'Your agents have execution history on the server, but this device cannot prove its cost records are complete. New model requests are blocked; durable known replies may still be delivered through recovery.')}</p>
      <p>{text('请从自己的备份恢复同一 Matrix 账号的完整本地账本，包括已结算费用、未结算预留和执行记录。只能恢复当前版本的账本到当前账号的数据目录，不能导入旧 Fleet/Engagement 数据；不要新建空账本、删除预留或复制其他用户的账本。', 'Restore the complete local ledger for the same Matrix account from your own backup, including settled usage, unresolved reservations, and execution records. Restore a current-version ledger into this account’s data directory; importing old Fleet/Engagement data is unsupported. Do not create an empty ledger, remove reservations, or copy another user’s ledger.')}</p>
      <p>{text('恢复后停止所选 Room（若仍在运行）并重新启动，让客户端重新验证。勾选 Estimated 模式或接管租约不能解除此限制；无需服务器管理员审批，agent 的创建者归属也不会改变。', 'After restoring, stop the selected room if it is still running and start it again so the client can revalidate. Estimated mode or lease takeover cannot bypass this check. Server administrator approval is not needed, and the agent stays with its original owner.')}</p>
    </section>}
    {!recoveryRequired && (error || runtime?.lastError) && <p role="alert">{text('未完成', 'Failed')}: {error || runtime.lastError}</p>}
    <div className="btn-row">
      <button className="btn" disabled={busy || provider?.authenticated || provider?.state === 'signing_in'} onClick={() => mutate('owner-provider/login', 'POST')}>{text('登录我的 ChatGPT / Codex 账号', 'Sign in to my ChatGPT / Codex account')}</button>
      {provider?.authUrl && <a className="btn" href={provider.authUrl} target="_blank" rel="noopener noreferrer">{text('打开官方登录页面', 'Open official sign-in page')}</a>}
      {provider?.state === 'signing_in' && <button className="btn" disabled={busy} onClick={() => mutate('owner-provider/cancel', 'POST')}>{text('取消登录', 'Cancel login')}</button>}
      <button className="btn" disabled={busy || !provider?.authenticated} onClick={() => { if (window.confirm(text('退出 Codex 会停止本机运行器并删除此 owner 的提供方登录。继续？', 'Signing out stops local runtimes and removes this owner’s provider login. Continue?'))) mutate('owner-provider/logout', 'POST'); }}>{text('退出 Codex', 'Sign out of Codex')}</button>
      <button className="btn" disabled={busy} onClick={() => { setError(null); refresh().catch(failure => setError(failure.message)); }}>{text('刷新登录与状态', 'Refresh login and status')}</button>
    </div>
    {provider?.authenticated && <p>{text('已核实的凭据引用', 'Verified credential reference')}: <code>{provider.credentialRef}</code> <button className="btn" disabled={busy} onClick={() => onProviderReference(provider.credentialRef)}>{text('用于下方配置', 'Use in configuration below')}</button></p>}
    <p className="dim">{text('原生 shell、MCP 和现有文件修改仍关闭。硬 token 上限（Strict）不可用。Estimated 是预留估算和事后记账，不能保证模型调用不会超出预算。', 'Native shell, MCP, and editing existing files remain disabled. Strict token limits are unavailable. Estimated mode reserves an estimate and accounts for actual usage afterward; it cannot guarantee a call stays within budget.')}</p>
    <label><input data-estimated-opt-in type="checkbox" checked={optIn} disabled={busy || active} onChange={event => setOptIn(event.target.checked)} />{text('我明确同意使用 Estimated 模式及其超额风险', 'I explicitly accept Estimated mode and its overrun risk')}</label>
    <label><input data-host-files type="checkbox" checked={hostFiles} disabled={busy || active} onChange={event => setHostFiles(event.target.checked)} />{text('开启所选 Room 的受限文件工具（默认关闭）', 'Enable restricted file tools for the selected room (off by default)')}</label>
    <p className="dim">{text('开启后仅可列目录、读取和新建当前 Room 私有工作区的文件；不能覆盖已有文件、访问其他 Room 文件或使用任意网络。每次调用仍受当前发言者策略约束；需创建者确认的提案会停在下方等待审批。', 'When enabled, tools can only list, read, and create files in this room’s private workspace. They cannot overwrite files, access other rooms, or use arbitrary networking. Each call follows the requester’s current policy; proposals requiring owner confirmation wait below.')}</p>
    <label>{text('每次调用预留 token 估算', 'Estimated token reservation per call')}<input inputMode="numeric" value={reservation} disabled={busy || active} onChange={event => setReservation(event.target.value)} /></label>
    <label>{text('推理强度', 'Reasoning effort')}<select value={effort} disabled={busy || active} onChange={event => setEffort(event.target.value)}>{['low', 'medium', 'high', 'xhigh'].map(value => <option key={value} value={value}>{value}</option>)}</select></label>
    <label><input type="checkbox" checked={takeover} disabled={busy || active || !assignedHere} onChange={event => setTakeover(event.target.checked)} />{text('恢复本设备的旧租约（不能代替执行实例设备分配）', 'Recover this device’s previous lease (does not replace instance assignment)')}</label>
    <p className="dim">{text('先保存下方模型、工作区和额度策略，再显式启动所选 Room。登录或保存配置都不会自动开始推理。', 'Save the model, workspace, and budget policies below, then explicitly start the selected room. Signing in or saving does not start inference.')}</p>
    {!assignedHere && <p role="status">{text('先把此 Agent 的执行实例分配到本机，才能在此运行。', 'Assign this agent’s execution instance to this device before starting.')}</p>}
    <div className="btn-row"><button className="btn" disabled={busy || active || !assignedHere || !configured || !bindingId || bindingState !== 'active' || !optIn || !validReservation} onClick={() => mutate(`${runtimePath}/start`, 'POST', { bindingId, mode: 'estimated', reservation: Number(reservation), estimatedOptIn: true, effort, takeover, hostFiles })}>{text('启动所选 Room 的 Codex', 'Start Codex for the selected room')}</button><button className="btn" disabled={busy || !active} onClick={() => mutate(`${runtimePath}/stop`, 'POST', {bindingId})}>{text('停止所选 Room', 'Stop selected room')}</button></div>
    <section data-tool-approvals>
      <h4>{text('等待我的请求 / 工具审批', 'Requests / tool calls awaiting my approval')}</h4>
      <p className="dim">{text('这是 agent 创建者的本机决定，无需服务器管理员确认。批准只适用于以下完整参数的一次调用；执行前仍会重新核对 Room 权利、当前策略和有效期。', 'This is the agent owner’s local decision and needs no server administrator approval. Approval applies to one call with these exact arguments; room authority, current policy, and expiry are checked again before execution.')}</p>
      {approvals.length === 0 && <p>{text('暂无等待审批的请求或调用', 'No requests or calls awaiting approval')}</p>}
      {approvals.map(item => {
        const proposal = item.proposal;
        const valid = proposal?.scope?.agent === agentId && typeof item.proposalId === 'string' && /^[a-f0-9]{64}$/.test(item.argsDigest || '') && Number.isFinite(proposal.expires);
        const expired = !valid || proposal.expires * 1000 <= Date.now();
        return <article key={item.proposalId} data-tool-proposal className="panel">
          <p>{proposal?.risk === 'model_request' ? text('模型请求', 'Model request') : text('文件工具调用', 'File tool call')}: <strong>{proposal?.tool || text('无效提案', 'Invalid proposal')}</strong> · {proposal?.risk}</p>
          <dl>
            <dt>Room</dt><dd>{proposal?.scope?.room}</dd>
            <dt>{text('发言者', 'Requester')}</dt><dd>{proposal?.scope?.requester}</dd>
            <dt>{text('讨论线程', 'Room thread')}</dt><dd>{proposal?.scope?.thread}</dd>
            <dt>{text('工作目录', 'Working directory')}</dt><dd>{proposal?.canonical_directory}</dd>
            <dt>{text('策略版本', 'Policy revisions')}</dt><dd>{JSON.stringify(proposal?.policy_revision)}</dd>
            <dt>{text('有效期', 'Expires')}</dt><dd>{valid ? new Date(proposal.expires * 1000).toLocaleString() : '—'}{expired && ` (${text('已过期或无效', 'expired or invalid')})`}</dd>
            <dt>{text('参数摘要', 'Arguments digest')}</dt><dd><code>{item.argsDigest}</code></dd>
          </dl>
          <p>{text('完整调用参数', 'Exact call arguments')}</p><pre style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', maxHeight: 360, overflow: 'auto' }}>{JSON.stringify(proposal?.arguments, null, 2)}</pre>
          <div className="btn-row">{[true, false].map(approved => <button key={String(approved)} className="btn" disabled={busy || expired} onClick={() => mutate(`${runtimePath}/approvals/${encodeURIComponent(item.proposalId)}/decision`, 'POST', { argsDigest: item.argsDigest, approved })}>{approved ? text('批准这一次调用', 'Approve this call once') : text('拒绝这一次调用', 'Reject this call')}</button>)}</div>
        </article>;
      })}
    </section>
  </section>;
}
