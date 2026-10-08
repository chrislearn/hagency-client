'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { usePrefs } from '@/components/Prefs';
import OwnerRuntimeControl from '@/components/OwnerRuntimeControl';
import Link from 'next/link';

const BASE = '/console/api/owned-agents';
async function api(path = '', method = 'GET', body, signal) {
  const response = await fetch(`${BASE}${path}`, {
    method, credentials: 'same-origin', cache: 'no-store',
    headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: signal || AbortSignal.timeout(45000),
  });
  const value = await response.json();
  if (!response.ok) throw new Error(value.code || 'server_request_failed');
  return value;
}
function CommandForm({ projects, onSubmit, busy, binding, text }) {
  const [projectId, setProject] = useState('');
  const [roomId, setRoom] = useState('');
  const [displayName, setName] = useState('');
  const [rooms,setRooms]=useState([]);
  const [roomError,setRoomError]=useState(null);
  useEffect(()=>{setRoom('');setRooms([]);setRoomError(null);if(!projectId)return;const controller=new AbortController();fetch(`/console/api/owner-projects/${encodeURIComponent(projectId)}/rooms`,{credentials:'same-origin',cache:'no-store',signal:controller.signal}).then(async response=>{const value=await response.json();if(!response.ok)throw new Error(value.code||'room_discovery_failed');return value;}).then(value=>setRooms(value.rooms||[])).catch(failure=>{if(failure.name!=='AbortError')setRoomError(failure.message);});return()=>controller.abort();},[projectId,projects]);
  const command = useRef(null);
  const submit = async event => {
    event.preventDefault();
    const input = binding ? { projectId, roomId: roomId.trim() } : { displayName: displayName.trim() };
    const fingerprint = JSON.stringify(input);
    if (command.current?.fingerprint !== fingerprint) command.current = { fingerprint, key: crypto.randomUUID() };
    if (await onSubmit({ ...input, idempotencyKey: command.current.key })) {
      setRoom(''); setName(''); command.current = null;
    }
  };
  return <form onSubmit={submit} className="panel">
    <h2>{text(binding ? '增加 Room 绑定' : '创建我拥有的 Agent', binding ? 'Bind another room' : 'Create an agent I own')}</h2>
    {binding && <><label>{text('Project（Matrix Space）', 'Project (Matrix Space)')}
      <select required value={projectId} disabled={busy} onChange={e => setProject(e.target.value)}>
        <option value="">{text('选择 Project', 'Choose a project')}</option>
        {projects.map(project => <option key={project.id} value={project.id}>{project.name || project.spaceId}</option>)}
      </select>
    </label>
    <label>{text('Room（独立成员）', 'Room (independent members)')}<select required value={roomId} disabled={busy||!projectId} onChange={e=>setRoom(e.target.value)}><option value="">{text('选择已登记且可见的 Room','Choose a visible registered room')}</option>{rooms.filter(room=>room.active).map(room=><option key={room.roomId} value={room.roomId}>{room.name || room.roomId}</option>)}</select></label>
    {roomError&&<p role="alert">{roomError}</p>}
    {projectId&&!rooms.length&&<p role="status">{text('此 Project 暂无我可见的已登记 Room。Space 成员不自动获得 Room 成员资格。','No registered rooms are visible to me in this project. Space membership does not grant room membership.')}</p>}</>}
    {!binding && <label>{text('Agent 名称', 'Agent name')}<input required maxLength={64} disabled={busy} value={displayName} onChange={e => setName(e.target.value)} /></label>}
    <p className="dim">{binding ? text('只能加入已登记且你有 Agent 接入权限的 Room。', 'Choose a registered room where you may invite your agent.') : text('创建服务器范围的傀儡账号，永久归你所有。默认由本机执行，之后配置模型、额度和加入 Room。', 'Create a server-wide puppet account that you permanently own. It is assigned to this device; configure its model, budgets, and rooms afterward.')}</p>
    {binding && !projects.length && <Link className="btn" href="/projects/new/">{text('创建 Project', 'Create project')}</Link>}
    <button type="submit" className="btn" disabled={busy || (binding ? !projectId || !roomId : !displayName.trim())}>{text(binding ? '绑定 Room' : '创建 Agent', binding ? 'Bind room' : 'Create agent')}</button>
  </form>;
}
function RoomRoster({binding,text}) {
  const [roster,setRoster]=useState(null);const [error,setError]=useState(null);const [refresh,setRefresh]=useState(0);
  useEffect(()=>{setRoster(null);setError(null);if(!binding)return;const controller=new AbortController();fetch(`/console/api/owner-projects/${encodeURIComponent(binding.projectId)}/rooms/${encodeURIComponent(binding.roomId)}/agents`,{credentials:'same-origin',cache:'no-store',signal:controller.signal}).then(async response=>{const value=await response.json();if(!response.ok)throw new Error(value.code||'room_roster_failed');return value;}).then(setRoster).catch(failure=>{if(failure.name!=='AbortError')setError(failure.message);});return()=>controller.abort();},[binding,refresh]);
  return <section data-room-roster><h3>{text('此 Room 的 Agents','Agents in this room')}</h3><button className="btn" type="button" disabled={!binding} onClick={()=>setRefresh(value=>value+1)}>{text('刷新 Room 名单','Refresh room roster')}</button>{error&&<p role="alert">{error}</p>}{roster&&<ul>{roster.agents.map(agent=><li key={agent.agentId}>{agent.displayName} · <code>{agent.puppetMxid}</code> · {text('创建者','Owner')}: {agent.ownerMxid} · {agent.bindingState}</li>)}</ul>}</section>;
}
function PolicyEditor({ version, layer, usage, onSave, onReset, busy, text }) {
  const [draft, setDraft] = useState(version.policy);
  const [tools, setTools] = useState('');
  const [directories, setDirectories] = useState('');
  useEffect(() => {
    setDraft(version.policy);
    setTools(version.policy.high_risk?.AllowWithRules?.tools.join('\n') || '');
    setDirectories(version.policy.high_risk?.AllowWithRules?.directories.join('\n') || '');
  }, [version]);
  const limit = typeof draft.budget.limit === 'string' ? draft.budget.limit : 'Tokens';
  const risk = typeof draft.high_risk === 'string' ? draft.high_risk : 'AllowWithRules';
  const save = event => {
    event.preventDefault();
    const policy = { ...draft, high_risk: risk === 'AllowWithRules' ? { AllowWithRules: { tools: tools.split('\n').map(v => v.trim()).filter(Boolean), directories: directories.split('\n').map(v => v.trim()).filter(Boolean) } } : risk };
    onSave(policy, version.revision);
  };
  const budget = update => setDraft(current => ({ ...current, budget: { ...current.budget, ...update } }));
  return <form onSubmit={save} className="panel">
    <h3>{text(...({ agent: ['Agent 总配额', 'Agent total budget'], room: ['此 Room 配额', 'This room budget'], requester: ['此 Room 中该用户的配额', 'This requester in this room'] }[layer]))}</h3>
    <p className="dim">{text('已消耗', 'Spent')}: {usage.spent} · {text('预留', 'Reserved')}: {usage.held} · {text('版本', 'Revision')}: {version.revision}</p>
    <label>{text('Token 上限', 'Token limit')}<select value={limit} disabled={busy} onChange={e => budget({ limit: e.target.value === 'Tokens' ? { Tokens: 0 } : e.target.value })}>
      <option value="Unset">{text('未配置（拒绝执行）', 'Unset (execution blocked)')}</option>
      <option value="Tokens">{text('限定数量', 'Token count')}</option>
      <option value="Unlimited">{text('不限量', 'Unlimited')}</option>
    </select></label>
    {limit === 'Tokens' && <label>{text('Token 数量', 'Tokens')}<input type="number" required min={0} max={Number.MAX_SAFE_INTEGER} step={1} disabled={busy} value={draft.budget.limit.Tokens} onChange={e => budget({ limit: { Tokens: Number(e.target.value) } })} /></label>}
    <label>{text('计费周期', 'Accounting period')}<select disabled={busy} value={draft.budget.period} onChange={e => budget({ period: e.target.value })}>
      <option value="Lifetime">{text('累计', 'Lifetime')}</option><option value="UtcDay">{text('UTC 自然日', 'UTC calendar day')}</option><option value="UtcMonth">{text('UTC 自然月', 'UTC calendar month')}</option>
    </select></label>
    <label>{text('处理请求', 'Handle requests')}<select disabled={busy} value={draft.requests} onChange={e => setDraft(current => ({ ...current, requests: e.target.value }))}>
      <option value="Allow">{text('允许', 'Allow')}</option><option value="Deny">{text('拒绝', 'Deny')}</option><option value="AskOwner">{text('逐请求询问我', 'Ask me per request')}</option>
    </select></label>
    <label>{text('高风险工具调用', 'High-risk tool calls')}<select disabled={busy} value={risk} onChange={e => setDraft(current => ({ ...current, high_risk: e.target.value === 'AllowWithRules' ? { AllowWithRules: { tools: [], directories: [] } } : e.target.value }))}>
      <option value="Deny">{text('拒绝', 'Deny')}</option><option value="AskOwner">{text('逐调用询问我', 'Ask me per call')}</option><option value="AllowWithRules">{text('仅放行指定工具与目录', 'Allow specified tools and directories')}</option>
    </select></label>
    {risk === 'AllowWithRules' && <><label>{text('允许的工具名称（每行一个）', 'Allowed tool names (one per line)')}<textarea required value={tools} disabled={busy} onChange={e => setTools(e.target.value)} /></label><label>{text('允许的本机绝对目录（每行一个）', 'Allowed absolute local directories (one per line)')}<textarea required value={directories} disabled={busy} onChange={e => setDirectories(e.target.value)} /></label></>}
    <div className="btn-row"><button className="btn" type="submit" disabled={busy}>{text('保存本地策略', 'Save local policy')}</button><button className="btn" type="button" disabled={busy} onClick={() => onReset(version.revision)}>{text('恢复安全默认值', 'Reset safe defaults')}</button></div>
  </form>;
}
function ModelEditor({ profile, credentialRef, onSave, busy, text }) {
  const [draft, setDraft] = useState(profile || { model: '', credential_ref: '', workspace_root: '' });
  useEffect(() => setDraft(profile || { model: '', credential_ref: '', workspace_root: '' }), [profile]);
  return <form className="panel" onSubmit={event => { event.preventDefault(); onSave({ ...draft, credential_ref: credentialRef || draft.credential_ref }); }}>
    <h3>{text('本机 Codex 资源', 'Local Codex resources')}</h3>
    <label>{text('模型名称', 'Model name')}<input required value={draft.model} disabled={busy} onChange={e => setDraft(current => ({ ...current, model: e.target.value }))} /></label>
    <label>{text('本机凭据存储引用', 'Local credential-store reference')}<input required readOnly value={credentialRef || draft.credential_ref} placeholder={text('先登录提供方并选用已核实引用', 'Sign in and use the verified provider reference')} /></label>
    <label>{text('本机工作区绝对目录', 'Absolute local workspace directory')}<input required value={draft.workspace_root} disabled={busy} placeholder="/path/to/workspace" onChange={e => setDraft(current => ({ ...current, workspace_root: e.target.value }))} /></label>
    <p className="dim">{text('这里保存凭据引用，不接收模型密钥。资源和策略仅保存在本机，无需服务器管理员批准。保存配置不代表执行器已经上线。', 'This stores a credential reference and accepts no model secret. Resources and policies stay on this machine without administrator approval. Saving does not mean the runtime is online.')}</p>
    <button className="btn" type="submit" disabled={busy}>{text('保存 Codex 配置', 'Save Codex configuration')}</button>
  </form>;
}
function ExecutionDeviceControl({ agentId, busy, text, onAssignment, onChanged }) {
  const [devices, setDevices] = useState([]);
  const [currentDeviceId, setCurrentDevice] = useState('');
  const [agent, setAgent] = useState(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState(null);
  const inFlight = useRef(false);
  const refresh = useCallback(async signal => {
    const [known, assigned] = await Promise.all([api('/devices', 'GET', undefined, signal), api(`/${agentId}`, 'GET', undefined, signal)]);
    if (signal?.aborted) return;
    if (assigned.agent?.id !== agentId || typeof known.currentDeviceId !== 'string') throw new Error('invalid_server_response');
    setDevices(known.devices); setCurrentDevice(known.currentDeviceId); setAgent(assigned.agent);
    onAssignment(assigned.agent.executionDeviceId === known.currentDeviceId);
  }, [agentId, onAssignment]);
  useEffect(() => {
    const controller = new AbortController(); onAssignment(false);
    refresh(controller.signal).catch(failure => { if (failure.name !== 'AbortError') setError(failure.message); });
    return () => controller.abort();
  }, [refresh]);
  const assigned = devices.find(device => device.id === agent?.executionDeviceId);
  const assignedHere = agent?.executionDeviceId === currentDeviceId && !!currentDeviceId;
  return <section className="panel" data-execution-device>
    <h3>{text('执行设备', 'Execution device')}</h3>
    <p>{text('新建 Agent 默认由本机执行。每个 Agent 只有一个执行设备，只有这台设备的模型、工作区和账本会用于处理请求。切换到本机会停止原设备的执行权；不会转让 Agent。', 'New agents run on this device by default. Each agent has one execution device; only its model, workspace, and ledger handle requests. Moving execution here revokes the previous device’s execution rights without transferring ownership.')}</p>
    <p>{text('当前设备', 'Assigned device')}: {assigned ? `${assigned.name}${assigned.revoked ? text('（已撤销）', ' (revoked)') : ''}` : agent?.executionDeviceId || text('未分配', 'Unassigned')}{assignedHere ? text('（本机）', ' (this device)') : ''}</p>
    {error && <p role="alert">{error}</p>}
    {!assignedHere && <button className="btn" type="button" disabled={busy || pending || !agent || !currentDeviceId} onClick={async () => {
      if (busy || inFlight.current || !agent || !window.confirm(text('将此 Agent 的执行设备改为本机？原设备会停止执行。请先恢复完整账本，避免遗漏既有费用。', 'Move this agent’s execution to this device? The previous device will stop. Restore the complete ledger first so earlier costs are retained.'))) return;
      inFlight.current = true; setPending(true); setError(null);
      try { await api(`/${agentId}/execution-device`, 'PUT', { expectedGeneration: agent.generation }); await refresh(); await onChanged(); }
      catch (failure) { setError(failure.message); }
      finally { inFlight.current = false; setPending(false); }
    }}>{text('改为在本机执行', 'Run on this device instead')}</button>}
  </section>;
}
export default function OwnedAgentControl() {
  const { locale } = usePrefs();
  const text = (zh, en) => locale === 'zh' || locale.startsWith('zh-') ? zh : en;
  const [roster, setRoster] = useState(null);
  const [credentialRef, setCredentialRef] = useState('');
  const [agentId, selectAgent] = useState('');
  const [bindings, setBindings] = useState([]);
  const [bindingId, selectBinding] = useState('');
  const [requester, setRequester] = useState('');
  const [config, setConfig] = useState(null);
  const [agentConfig, setAgentConfig] = useState(null);
  const [assignedHere, setAssignedHere] = useState(false);
  const [ownerDirect, setOwnerDirect] = useState(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(null);
  const [note, setNote] = useState(null);
  const load = useCallback(async () => {
    const value = await api(); setRoster(value); setRequester(current => current || value.ownerMxid);
    selectAgent(current => value.agents.some(agent => agent.id === current) ? current : value.agents[0]?.id || '');
  }, []);
  useEffect(() => { load().catch(failure => setError(failure.message)); }, [load]);
  useEffect(() => {
    setBindings([]); selectBinding(''); setConfig(null); setAgentConfig(null); setAssignedHere(false); setOwnerDirect(null);
    if (!agentId) return;
    const controller = new AbortController();
    api(`/${encodeURIComponent(agentId)}/bindings`, 'GET', undefined, controller.signal).then(value => {
      setBindings(value.bindings); selectBinding(value.bindings[0]?.id || '');
    }).catch(failure => { if (failure.name !== 'AbortError') setError(failure.message); });
    api(`/${encodeURIComponent(agentId)}/agent-policy`, 'GET', undefined, controller.signal).then(setAgentConfig).catch(failure => { if (failure.name !== 'AbortError') setError(failure.message); });
    api(`/${encodeURIComponent(agentId)}/owner-direct`, 'GET', undefined, controller.signal).then(value => setOwnerDirect({ roomId: value.ownerDirectRoomId, state: value.binding?.state })).catch(failure => { if (failure.name !== 'AbortError') setError(failure.message); });
    return () => controller.abort();
  }, [agentId]);
  const configPath = bindingId && requester.trim() ? `/${encodeURIComponent(agentId)}/local-policy?${new URLSearchParams({ bindingId, requester: requester.trim() })}` : null;
  useEffect(() => {
    setConfig(null);
    if (!configPath) return;
    const controller = new AbortController();
    api(configPath, 'GET', undefined, controller.signal).then(setConfig).catch(failure => { if (failure.name !== 'AbortError') setError(failure.message); });
    return () => controller.abort();
  }, [configPath]);
  async function change(path, method, body, after) {
    if (busy) return false;
    setBusy(true); setError(null); setNote(null);
    try { const value = await api(path, method, body); await after?.(value); setNote(value.commandState === 'pending' ? `${text('创建/绑定请求已保存，尚待服务器完成加入。', 'Creation/binding saved; server joining is still pending.')}${value.pendingReason ? ` (${value.pendingReason})` : ''}` : text('操作已保存。', 'Saved.')); return true; }
    catch (failure) { setError(failure.message); return false; }
    finally { setBusy(false); }
  }
  const selected = roster?.agents.find(agent => agent.id === agentId);
  const selection = { bindingId, requester: requester.trim() };
  return <div data-owned-agents>
    <h1 style={{ fontSize: 22 }}>{text('我拥有的 AI Agents', 'My AI agents')}</h1>
    <p className="dim">{text('永久归创建者所有。一个 Agent 可绑定不同 Project 的多个 Room，每个 Room 的上下文、配额与请求策略独立。', 'The creator permanently owns the identity. One agent can serve rooms across projects, with independent room contexts, budgets, and request policies.')}</p>
    <p role="status">{text('此页面管理身份、本地策略和显式启动的 Codex 运行器；实际运行状态以下方核实结果为准。', 'This page manages identities, local policies, and explicitly started Codex runtimes. The verified runtime status is shown below.')}</p>
    {error && <p role="alert">{text('操作未完成', 'Operation failed')}: {error}{error === 'local_policy_conflict' && ` · ${text('策略版本已变化，请刷新后重新编辑。', 'The policy revision changed. Refresh before editing again.')}`}</p>}
    {note && <p role="status">{note}</p>}
    <button className="btn" disabled={busy} onClick={() => change('', 'GET', undefined, value => { setRoster(value); if (configPath) return api(configPath).then(setConfig); })}>{text('刷新', 'Refresh')}</button>
    {roster && <><p>{text('当前 owner', 'Current owner')}: <code>{roster.ownerMxid}</code></p>
      <CommandForm projects={roster.projects} busy={busy} text={text} onSubmit={input => change('', 'POST', input, async value => {
        const created = value.creation.agent.id; await load(); selectAgent(created);
        try { await api(`/${created}/owner-direct/ensure`, 'POST'); }
        catch (failure) { setError(`${text('Agent 已创建，联系人建立待重试', 'Agent created; retry contact setup')}: ${failure.message}`); }
      })} />
      <section className="panel"><h2>{text('我的 Agents', 'My agents')}</h2>
        <select aria-label={text('选择 Agent', 'Select agent')} value={agentId} disabled={busy} onChange={e => selectAgent(e.target.value)}><option value="">{text('选择 Agent', 'Choose an agent')}</option>{roster.agents.map(agent => <option key={agent.id} value={agent.id}>{agent.displayName} · {agent.state} · {agent.executionDeviceId === roster.currentDeviceId ? text('本机', 'This device') : roster.devices?.find(device => device.id === agent.executionDeviceId)?.name || text('其他设备', 'Other device')}</option>)}</select>
        {selected && <><p><code>{selected.puppetMxid}</code> · {selected.state}</p><div className="btn-row"><button className="btn" disabled={busy || selected.state === 'retiring' || selected.state === 'retired'} onClick={() => change(`/${agentId}/pause`, 'POST', undefined, load)}>{text('暂停', 'Pause')}</button><button className="btn" disabled={busy || selected.state !== 'suspended'} onClick={() => change(`/${agentId}/resume`, 'POST', undefined, load)}>{text('恢复', 'Resume')}</button><button className="btn" disabled={busy || selected.state === 'retiring' || selected.state === 'retired'} onClick={() => { if (window.confirm(text('退役将停止此 Agent 的所有 Room 服务，身份不会释放给其他用户。继续？', 'Retiring stops every room binding and never releases the identity to another owner. Continue?'))) change(`/${agentId}`, 'DELETE', undefined, load); }}>{text('退役', 'Retire')}</button></div></>}
      </section>
      {selected && <><section className="panel"><h3>{text('主人私聊', 'Owner direct chat')}</h3><p>{ownerDirect?.roomId || text('联系人尚未建立', 'Contact setup pending')} · {ownerDirect?.state}</p><button className="btn" disabled={busy || ['retiring', 'retired'].includes(selected.state)} onClick={() => change(`/${agentId}/owner-direct/ensure`, 'POST', undefined, value => { setOwnerDirect(value.ownerDirect); return api(`/${agentId}/bindings`).then(value => setBindings(value.bindings)); })}>{text('建立 / 刷新联系人', 'Set up / refresh contact')}</button></section>
        <ExecutionDeviceControl key={agentId} agentId={agentId} busy={busy} text={text} onAssignment={setAssignedHere} onChanged={load} />
        {agentConfig && <><ModelEditor key={`model:${agentId}`} profile={agentConfig.modelProfile} credentialRef={credentialRef} busy={busy || !assignedHere} text={text} onSave={profile => change(`/${agentId}/agent-model-profile`, 'PUT', { profile }, setAgentConfig)} />
          <PolicyEditor key={`budget:${agentId}`} layer="agent" version={agentConfig.policy} usage={agentConfig.usage} busy={busy || !assignedHere} text={text} onSave={(policy, expectedRevision) => change(`/${agentId}/agent-policy`, 'PUT', { expectedRevision, policy }, setAgentConfig)} onReset={expectedRevision => change(`/${agentId}/agent-policy`, 'DELETE', { expectedRevision }, setAgentConfig)} /></>}
        <CommandForm key={agentId} projects={roster.projects} binding busy={busy} text={text} onSubmit={input => change(`/${agentId}/bindings`, 'POST', input, async () => { const value = await api(`/${agentId}/bindings`); setBindings(value.bindings); })} />
        <section className="panel"><label>{text('配置 Room', 'Configure room')}<select value={bindingId} disabled={busy} onChange={e => selectBinding(e.target.value)}><option value="">{text('选择绑定', 'Choose a binding')}</option>{bindings.map(binding => <option key={binding.id} value={binding.id}>{binding.roomId} · {binding.state}</option>)}</select></label>
        {bindings.find(binding => binding.id === bindingId) && <div className="btn-row">
          <button className="btn" disabled={busy || bindings.find(binding => binding.id === bindingId)?.state !== 'active'} onClick={() => change(`/${agentId}/bindings/${bindingId}/pause`, 'POST', undefined, async () => setBindings((await api(`/${agentId}/bindings`)).bindings))}>{text('暂停此 Room', 'Pause this room')}</button>
          <button className="btn" disabled={busy || bindings.find(binding => binding.id === bindingId)?.state !== 'suspended'} onClick={() => change(`/${agentId}/bindings/${bindingId}/resume`, 'POST', undefined, async () => setBindings((await api(`/${agentId}/bindings`)).bindings))}>{text('恢复此 Room', 'Resume this room')}</button>
          <button className="btn" disabled={busy || ['leaving', 'left'].includes(bindings.find(binding => binding.id === bindingId)?.state)} onClick={() => { if (window.confirm(text('仅退出此 Room。服务器完成 Matrix 清理前状态为 leaving。继续？', 'Leave only this room. The binding stays leaving until server Matrix cleanup completes. Continue?'))) change(`/${agentId}/bindings/${bindingId}`, 'DELETE', undefined, async () => setBindings((await api(`/${agentId}/bindings`)).bindings)); }}>{text('退出此 Room', 'Leave this room')}</button>
        </div>}
        {bindings.find(binding=>binding.id===bindingId)?.scopeKind === 'project' && <RoomRoster binding={bindings.find(binding=>binding.id===bindingId)} text={text} />}
        <label>{text('Room 中的用户 MXID（用于该用户策略）', 'Requester MXID in this room (for requester policy)')}<input disabled={busy} value={requester} onChange={e => setRequester(e.target.value)} placeholder="@user:example.org" /></label>
        <p className="dim">{text('三个层级的限额和许可共同生效；任一层拒绝即拒绝。变更立即限制后续调用，不清空已发生用量。', 'All three budget and permission layers apply together. A denial at any layer denies the request. Changes constrain future calls and preserve existing usage.')}</p></section>
        {config && <><OwnerRuntimeControl key={`${agentId}:${bindingId}`} agentId={agentId} bindingId={bindingId} bindingState={bindings.find(binding => binding.id === bindingId)?.state} assignedHere={assignedHere} profile={agentConfig?.modelProfile} text={text} onProviderReference={setCredentialRef} />
          {['room', 'requester'].map((layer, index) => <PolicyEditor key={`${bindingId}:${requester}:${layer}`} layer={layer} version={config.policies[index + 1]} usage={config.usage[index + 1]} busy={busy || !assignedHere} text={text} onSave={(policy, expectedRevision) => change(`/${agentId}/local-policy`, 'PUT', { ...selection, layer, expectedRevision, policy }, setConfig)} onReset={expectedRevision => change(`/${agentId}/local-policy`, 'DELETE', { ...selection, layer, expectedRevision }, setConfig)} />)}
        </>}
      </>}
    </>}
  </div>;
}
