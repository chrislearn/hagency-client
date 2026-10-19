// Isolated real-Hagency backend for Rust interoperability tests; never starts a runner.
// Run only through the ignored Rust test with ROBRIX_HAGENCY_SOURCE set to the pinned checkout.
import { createInterface } from 'node:readline';
import { createServer } from 'node:net';
import { pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
const source = process.env.ROBRIX_HAGENCY_SOURCE;
const pinned = '4a8a8ac25e43e645345fc25987261e0d7a644fcd';
if (!source || execFileSync('git',['-C',source,'rev-parse','HEAD'],{encoding:'utf8'}).trim() !== pinned) throw new Error('Use the pinned Hagency test checkout');
process.chdir(source);
const { createBackendTestContext } = await import(pathToFileURL(path.join(source,'tests/helpers/backend-test-runtime.js')));
const { createLoopbackTestServer } = await import(pathToFileURL(path.join(source,'tests/helpers/loopback-test-server.js')));
const reserve = createServer(); await new Promise(resolve=>reserve.listen(0,'127.0.0.1',resolve));
const port = reserve.address().port; await new Promise(resolve=>reserve.close(resolve));
const origin = `http://127.0.0.1:${port}`;
const owner = '@owner:test', ownerRoom = '!approval:test', project = '!project:test';
const device = {matrix_device_id:'OWNERDEVICE',matrix_device_ed25519:Buffer.alloc(32,7).toString('base64url'),matrix_device_curve25519:Buffer.alloc(32,9).toString('base64url')};
const context = await createBackendTestContext('robrix-agent-ops-',{
 env:{HAGENCY_THREAD_SESSIONS:'1',HAGENCY_ROUTER_TASK_CUTOVER:'1',HAGENCY_AGENT_OPS_CLIENT:'1',HAGENCY_AGENT_OPS_LOOPBACK_ORIGIN:origin,MATRIX_BRIDGE_SECRET:'isolated-test-bridge',API_TOKEN:'isolated-test-operator'},
 agents:{worker:{name:'worker',agentId:'agent_worker',type:'codex',role:'coding',kind:'agent',workdir:source,workspaceMode:'shared',online:true}},agentTokens:{worker:'isolated-test-worker'},
});
context.internals.stopRouterPumpForTest();
const listener = await createLoopbackTestServer(context.expressApp,{port});
const binding = {agent:'worker',project:'project',project_room_id:project,owner_mxid:owner,owner_dm_room_id:ownerRoom};
async function api(method,route,body,operator=false) {
 const response = await fetch(origin+route,{method,headers:{'Content-Type':'application/json',...(operator?{Authorization:'Bearer isolated-test-operator'}:{'X-Bridge-Secret':'isolated-test-bridge'})},body:JSON.stringify(body)});
 if (!response.ok) throw new Error(`Fixture control failed: ${response.status} ${route}`);
 return response.json();
}
await api('PUT','/api/approval-bindings',binding);
await api('PUT','/api/agent-ops/v1/operator/device-enrollment',{...binding,...device},true);
const router = context.internals.routerStoreForTest;
const input = router.ingestMessage({messageId:'fixture-input',roomId:project,matrixEventId:'$fixture-input',threadRootEventId:'$fixture-thread',senderMxid:owner,senderName:'owner',recipientAgentId:'agent_worker',recipientAgentName:'worker',normalizedBody:'Review scoped contract'});
if (!input.ok) throw new Error('Fixture ingest failed');
router.registerWorkspace({resourceId:'fixture-workspace',safeLabel:'Fixture workspace',backendPath:context.runtimeDir});
const queued = router.enqueueDispatch({sessionId:input.session.sessionId,framework:'codex',localServerId:'local',workspaceResourceId:'fixture-workspace',namedResourceIds:['fixture-workspace'],mayWrite:false,payload:{}});
if (!queued.ok) throw new Error('Fixture queue failed');
const claim = router.claimDispatch({runnerId:'parked-fixture-runner',leaseMs:60000,capabilityTtlMs:60000,maxLiveRunners:8});
const capability = {dispatchId:claim.dispatchId,runnerId:claim.runnerId,fenceGeneration:claim.fenceGeneration,capability:claim.capability};
if (!router.takePayload(capability).ok || !router.parkForApproval({...capability,approvalId:'fixture-approval',operationDigest:'a'.repeat(64),maxParkedRunners:4}).ok) throw new Error('Fixture park failed');
function emit(value) { process.stdout.write('ROBRIX_FIXTURE:'+JSON.stringify(value)+'\n'); }
emit({origin,fingerprint:context.internals.agentOpsServerIdentityForTest.fingerprint});
const lines = createInterface({input:process.stdin,crlfDelay:Infinity});
try {
 for await (const line of lines) {
  const command = JSON.parse(line);
  if (command.kind === 'bootstrap') {
   emit(await api('POST','/api/agent-ops/v1/control/bootstrap',{schema:'com.hagency.agent_ops.v1',...binding,...device,matrix_event_id:'$'+command.nonce,client_nonce:command.nonce,client_public_jwk:command.jwk,was_encrypted:true,device_self_signature_verified:true,room_members_verified:true}));
  } else if (command.kind === 'dirty') {
   router.db.prepare("UPDATE resources SET dirty = 1, dirty_reason = 'fixture inspection', dirty_generation = dirty_generation + 1, dirty_dispatch_id = NULL WHERE resource_id = 'fixture-workspace'").run();
   emit({ok:true});
  } else if (command.kind === 'revoke') {
   await api('PUT','/api/approval-bindings',{...binding,owner_dm_room_id:'!changed:test'}); emit({ok:true});
  } else if (command.kind === 'stop') break;
  else throw new Error('Unsupported fixture command');
 }
} finally {
 await listener.close(); context.internals.routerStoreForTest.close(); await context.cleanup();
}
