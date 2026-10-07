import puppeteer from 'puppeteer-core';
import {createServer} from 'node:http';import {readFile,stat} from 'node:fs/promises';import {join} from 'node:path';import assert from 'node:assert/strict';
const root=process.env.HAGENCY_NATIVE_CONSOLE_ASSETS;assert.ok(root,'Set HAGENCY_NATIVE_CONSOLE_ASSETS from build:native');let matrixAuthorized=true;let runtime=null;let recoveryGate=false;let rejectedStarts=0;let approvals=[];const calls=[],decisions=[],errors=[];
const proposal=(id,expires=Math.floor(Date.now()/1000)+300)=>({proposalId:id,argsDigest:id.repeat(64),proposal:{scope:{agent:"agt_test",binding:"bnd_test",room:"!room:example.test",requester:"@requester:example.test",thread:"$room-thread"},dispatch:"dispatch-test",tool:"room.create",arguments:{codexThreadId:"codex-thread",turnId:"turn-test",callId:"call-"+id,file:{operation:"create",path:"note.txt",content:"<script>window.__owner_tool_injected=true</script>",nonce:"f".repeat(64)}},canonical_directory:"/private/owner/room-workspaces/binding-test",risk:"high",policy_revision:[1,2,3],expires}});
const server=createServer(async(req,res)=>{try{const url=new URL(req.url,'http://localhost');let path=decodeURIComponent(url.pathname).replace(/^\/console\/?/,'');if(!path||path.endsWith('/'))path+='index.html';const file=join(root,path);if(!(await stat(file)).isFile())throw 0;res.setHeader('content-type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'application/javascript':file.endsWith('.css')?'text/css':'application/octet-stream');res.end(await readFile(file));}catch{res.statusCode=404;res.end();}}).listen(0,'127.0.0.1');await new Promise(r=>server.once('listening',r));const base='http://127.0.0.1:'+server.address().port;
const browser=await puppeteer.launch({executablePath:process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
try{const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));await page.setRequestInterception(true);
const ref='keychain:codex-home:0123456789abcdef';const policy={budget:{limit:'Unlimited',period:'Lifetime'},requests:'Allow',high_risk:'Deny'};
page.on('request',async req=>{const url=new URL(req.url());let result;let resultStatus=200;
if(url.pathname==='/console/session'){assert.equal(req.method(),'POST');assert.equal(JSON.parse(req.postData()).ticket,'a'.repeat(64));result={};}
else if(url.pathname==='/console/server-login'){result={configured:true,server:'https://example.test/',name:'Hagency Client',status:{state:'device_authorized',ownerMxid:'@owner:example.test',deviceAuthorized:true,transportOnline:false}};}
else if(url.pathname==='/console/api/owned-agents'){result={ownerMxid:'@owner:example.test',agents:[{id:'agt_test',displayName:'My Codex',puppetMxid:'@_hagency_test:example.test',state:'active'}],projects:[{id:'prj_test',spaceId:'!space:example.test'}]};}
else if(url.pathname==='/console/api/owner-projects/prj_test/rooms'){result={rooms:[{roomId:'!room:example.test',projectId:'prj_test',active:true,creationPolicy:'inherit',revision:1}]};}
else if(url.pathname==='/console/api/owner-projects/prj_test/rooms/!room%3Aexample.test/agents'||url.pathname==='/console/api/owner-projects/prj_test/rooms/!room:example.test/agents'){result={agents:[{agentId:'agt_test',puppetMxid:'@_hagency_test:example.test',displayName:'My Codex',ownerMxid:'@owner:example.test',bindingState:'active'}]};}
else if(url.pathname==='/console/api/matrix-creations'){assert.equal(req.method(),'POST');const data=JSON.parse(req.postData());assert.equal(data.kind,'space');assert.equal(data.name,'New Space');assert.ok(data.commandId);result={creation:{...data,phase:'complete',roomId:'!new-space:example.test'}};}
else if(url.pathname==='/console/api/owned-agents/agt_test/bindings'){result={bindings:[{id:'bnd_test',agentId:'agt_test',projectId:'prj_test',roomId:'!room:example.test',state:'active'}]};}
else if(url.pathname.endsWith('/local-policy')){result={policies:[0,1,2].map(()=>({revision:0,policy})),usage:[0,1,2].map(()=>({spent:0,held:0})),modelProfile:{model:'test-model',credential_ref:ref,workspace_root:'/tmp/test-workspace'}};}
else if(url.pathname==='/console/api/owner-provider'){result={state:'authenticated',authenticated:true,credentialRef:ref,credentialStore:'codex_os_keyring',strictTokenCap:false,nativeTools:false};}
else if(url.pathname.endsWith('/runtime/start')){const data=JSON.parse(req.postData());if(recoveryGate){rejectedStarts++;resultStatus=409;result={code:'ledger_recovery_required'};}else{calls.push(data);runtime={phase:'checking_provider',bindingId:data.bindingId,nativeTools:false,strictTokenCap:false};approvals=data.hostFiles?[proposal('a'),proposal('e',1)]:[];result={runtime};}}
else if(url.pathname.endsWith('/runtime/stop')){assert.deepEqual(JSON.parse(req.postData()),{bindingId:'bnd_test'});runtime=null;approvals=[];result={stopped:true};}
else if(url.pathname.endsWith('/runtime/approvals')){result={approvals};}
else if(url.pathname.includes('/runtime/approvals/')&&url.pathname.endsWith('/decision')){assert.equal(req.method(),'POST');const data=JSON.parse(req.postData());assert.deepEqual(Object.keys(data).sort(),['approved','argsDigest']);const id=url.pathname.split('/').at(-2);assert.equal(data.argsDigest,id.repeat(64));decisions.push({id,...data});const model=proposal('b');model.proposal.tool='model.request';model.proposal.risk='model_request';model.proposal.arguments={codexThreadId:'codex-thread',turnId:'turn-test',callId:'call-b',model:'test-model',reservation:1000,mode:'estimated'};approvals=id==='a'?[model,proposal('e',1)]:[proposal('e',1)];result={decided:true};}
else if(url.pathname.endsWith('/runtime')){result={runtime,nativeTools:false,strictTokenCap:false};}
else if(url.pathname.startsWith('/console/api/')||url.pathname.startsWith('/console/server-login/')){errors.push('unexpected legacy request '+url.pathname);result={code:'unexpected'};}
if(url.pathname==='/console/api/owned-agents'&&!matrixAuthorized){resultStatus=401;result={code:'console_access_required'};}
if(result!==undefined)return req.respond({status:resultStatus,contentType:'application/json',body:JSON.stringify(result)});return req.continue();});
await page.goto(base+'/console/#access='+'a'.repeat(64),{waitUntil:'networkidle0'});assert.equal(new URL(page.url()).hash,'');
await page.waitForSelector('select[aria-label="Select agent"]');await page.select('select[aria-label="Select agent"]','agt_test');await page.waitForSelector('[data-owner-runtime]');
await page.waitForFunction(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Start Codex for the selected room')?.disabled===true);assert.equal(calls.length,0,'rendering or sign-in must not start inference');
assert.equal(await page.$eval('[data-host-files]',input=>input.checked),false);
await page.click('[data-estimated-opt-in]');await page.waitForFunction(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Start Codex for the selected room')?.disabled===false);
await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Start Codex for the selected room').click());await page.waitForFunction(()=>document.querySelector('[data-owner-runtime]').textContent.includes('checking_provider'));
assert.equal(calls.length,1);assert.equal(calls[0].bindingId,'bnd_test');assert.equal(calls[0].mode,'estimated');assert.equal(calls[0].estimatedOptIn,true);assert.equal(calls[0].reservation,1000);assert.ok(!('executable' in calls[0]));assert.equal(calls[0].hostFiles,false);
await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Stop selected room').click());await page.waitForFunction(()=>document.querySelector('[data-owner-runtime]').textContent.includes('stopped'));
assert.equal(decisions.length,0,'no automatic owner tool approval');
await page.click('[data-host-files]');
await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Start Codex for the selected room').click());
await page.waitForSelector('[data-tool-proposal]');assert.equal(calls.length,2);assert.equal(calls[1].hostFiles,true);assert.equal(decisions.length,0);
const detail=await page.$eval('[data-tool-proposal]',node=>node.textContent);for(const text of ['!room:example.test','@requester:example.test','codex-thread','turn-test','call-a','note.txt','[1,2,3]','a'.repeat(64),'<script>'])assert.ok(detail.includes(text));
assert.equal(await page.evaluate(()=>window.__owner_tool_injected),undefined,'tool arguments are rendered as text');
assert.equal(await page.$$eval('[data-tool-proposal]',nodes=>[...nodes.at(-1).querySelectorAll('button')].every(button=>button.disabled)),true,'expired proposal cannot be decided');
await page.evaluate(()=>[...document.querySelectorAll('[data-tool-proposal] button')].find(x=>x.textContent==='Approve this call once'&&!x.disabled).click());
await page.waitForFunction(()=>document.querySelector('[data-tool-proposal] pre')?.textContent.includes('call-b'));
assert.deepEqual(decisions[0],{id:'a',argsDigest:'a'.repeat(64),approved:true});assert.ok(await page.$eval('[data-tool-proposal]',node=>node.textContent.includes('Model request')&&node.textContent.includes('model.request')));
await page.evaluate(()=>[...document.querySelectorAll('[data-tool-proposal] button')].find(x=>x.textContent==='Reject this call'&&!x.disabled).click());
await page.waitForFunction(()=>document.querySelectorAll('[data-tool-proposal]').length===1);
assert.deepEqual(decisions[1],{id:'b',argsDigest:'b'.repeat(64),approved:false});
assert.equal(await page.$('[data-owner-projects]'),null,'project creation belongs on Projects, not Agents');
await page.waitForFunction(()=>document.querySelector('[data-room-roster]')?.textContent.includes('@owner:example.test'));
runtime={phase:'ledger_recovery_required',bindingId:'bnd_test',lastError:'ledger_recovery_required'};
await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Refresh login and status').click());
await page.waitForSelector('[data-ledger-recovery]');
const recovery=await page.$eval('[data-ledger-recovery]',node=>node.textContent);for(const value of ['same Matrix account','complete local ledger','current-version ledger','old Fleet/Engagement data is unsupported','settled usage','unresolved reservations','execution records','empty ledger','Estimated mode or lease takeover cannot bypass','Server administrator approval is not needed','original owner'])assert.ok(recovery.includes(value),value);
assert.equal(await page.$eval('[data-owner-runtime]',node=>[...node.querySelectorAll('button')].find(button=>button.textContent==='Start Codex for the selected room').disabled),true,'running recovery cannot start inference');
assert.equal(await page.$eval('[data-estimated-opt-in]',node=>node.disabled),true,'estimated opt-in is not a recovery override');
assert.equal(calls.length,2,'recovery display must not start inference');assert.equal(decisions.length,2,'recovery is not an administrator or owner approval proposal');
runtime={phase:'reconciliation_only',bindingId:'bnd_test',lastError:null};await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Refresh login and status').click());
await page.waitForFunction(()=>document.querySelector('[data-owner-runtime]')?.textContent.includes('Known reply recovery only'));assert.ok(await page.$('[data-ledger-recovery]'),'reconciliation-only phase requires recovery guidance even without lastError');
recoveryGate=true;await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Stop selected room').click());
await page.waitForFunction(()=>!document.querySelector('[data-ledger-recovery]'));
await page.evaluate(()=>[...document.querySelectorAll('[data-owner-runtime] button')].find(x=>x.textContent==='Start Codex for the selected room').click());
await page.waitForSelector('[data-ledger-recovery]');assert.equal(rejectedStarts,1,'failed startup code must produce the same recovery guidance');assert.equal(calls.length,2,'failed proof check must not start inference');
const links=await page.$$eval('nav a',links=>links.map(a=>new URL(a.href).pathname));assert.deepEqual(links,['/console/agents-owned/','/console/projects/']);assert.equal(calls.length,2);assert.deepEqual(errors,[]);
matrixAuthorized=false;
await page.evaluate(()=>window.dispatchEvent(new CustomEvent('hagency-owner-auth-changed')));
await page.waitForFunction(()=>window.location.pathname==='/console/login/'&&document.querySelector('[data-server-login]'));
assert.equal(await page.$('[data-owned-agents]'),null,'expired sign-in must hide Agent controls');
assert.equal(await page.$('#device-name'),null,'default device name must not require user input');
await page.goto(base+'/console/agents-owned/',{waitUntil:'networkidle0'});
await page.waitForFunction(()=>window.location.pathname==='/console/login/'&&document.querySelector('[data-server-login]'));
assert.equal(await page.$('[data-owned-agents]'),null,'direct Agent navigation must require Matrix sign-in');
assert.equal(calls.length,2,'expired sign-in and entry gating must not run inference');
assert.deepEqual(errors,[]);
console.log('PASS owner-only browser UI with stub API: finite ticket exchange, no legacy requests, provider/runtime DTO, explicit estimated opt-in, typed start/stop, restricted-files opt-in, exact owner approve/reject, expired proposals closed, same-owner complete ledger recovery notice with no bypass, no auto inference, owner navigation');
}finally{await browser.close();await new Promise(r=>server.close(r));}
