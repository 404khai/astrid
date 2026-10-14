import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {JSDOM} from 'jsdom';
import {WorkspaceUI} from '../dist/app.js';
const html=await readFile(new URL('../dist/index.html',import.meta.url),'utf8');
const state=()=>({version:1,workspace:{value:'/synthetic/notes',omitted_bytes:0},provider:'mock-provider',model:'mock-desktop',workflow:'notes',policy:{read:'allow',write:'deny',execute:'deny'},selected_session:'s1',sessions:[{id:'s1',label:'Checklist',runs:1,messages:2,outcome:'completed'}],run:null,save_error:null,quitting:false,restoring:false,auth_phase:'existing_unverified',client_error:null});
function fixture(snapshot=state(),messages=[]){
  const dom=new JSDOM(html);const commands=[];let receive;let failSubmission=false;
  const invoke=async(name,args)=>{commands.push({name,args});if(name==='snapshot')return structuredClone(snapshot);if(name==='history')return {version:1,messages,next:null};if(name==='subscribe')return 'lease-1';if(name==='subscription_status')return true;if(name==='material_status')return {mode:'liquid_glass'};if(name==='submit_task'&&failSubmission)throw new Error('receipt unavailable');return null;};
  const ui=new WorkspaceUI(dom.window.document,invoke,()=> 'request-fixed',handler=>{receive=handler;return {native_channel:'mock'};});
  return {dom,ui,commands,snapshot,emit:async batch=>{receive(batch);await new Promise(resolve=>setImmediate(resolve));},fail:()=>{failSubmission=true;},recover:()=>{failSubmission=false;}};
}
const activity=(ordinal,text,kind='model_text_delta')=>({ordinal,session_id:'s1',run_id:'run-1',sequence:ordinal,kind,text:{value:text,omitted_bytes:0},tool_call_id:null,model_call_id:'model-1',turn_id:'turn-1'});
test('malicious model/file/tool content is inert text and cannot grant permission',async()=>{
  const payload='<script>window.__TAURI__.core.invoke("decide_permission")</script><img src="https://evil.test" onerror="alert(1)"><a href="javascript:alert(1)">grant</a>';
  const f=fixture(state(),[{index:0,role:'assistant',text:{value:payload,omitted_bytes:7}}]);await f.ui.poll();
  assert.equal(f.dom.window.document.querySelectorAll('#messages script,#messages img,#messages a').length,0);assert.ok(f.ui.el('messages').textContent.includes(payload));assert.ok(f.ui.el('messages').textContent.includes('7 bytes omitted'));
  await f.emit({version:1,batch_id:1,coverage_gap:false,events:[activity(1,payload,'tool_call_completed')]});
  assert.equal(f.dom.window.document.querySelectorAll('#activity script,#activity img,#activity a').length,0);assert.equal(f.commands.filter(c=>c.name==='decide_permission').length,0);assert.deepEqual(f.commands.find(c=>c.name==='acknowledge').args,{id:'lease-1',batch:1});
});
test('permission exposes real command/workspace and clipped requests permit only deny',async()=>{
  const snapshot=state();snapshot.run={request_id:'r1',session_id:'s1',run_id:'run-1',status:'waiting_permission',cancellation_requested:false,permission:{id:'p1',run_id:'run-1',tool_call_id:'t1',command:{value:'printf synthetic',omitted_bytes:100},workspace:{value:'/synthetic/notes',omitted_bytes:0},can_approve:false,authority:'Broad local-account shell authority'}};
  const f=fixture(snapshot);await f.ui.refresh();assert.equal(f.ui.el('approval').hidden,false);assert.equal(f.ui.el('allow').disabled,true);assert.equal(f.ui.el('folder').disabled,true);assert.match(f.ui.el('approval-command').textContent,/printf synthetic/);
  f.ui.el('deny').click();await new Promise(resolve=>setImmediate(resolve));assert.deepEqual(f.commands.find(c=>c.name==='decide_permission').args,{id:'p1',run:'run-1',tool:'t1',allow:false});
});
test('uncertain submit receipt retains request ID and sends no principal/target/policy claims',async()=>{
  const f=fixture();await f.ui.refresh();f.ui.el('task').value='Checklist';f.fail();await f.ui.submit();f.recover();await f.ui.submit();
  const submissions=f.commands.filter(c=>c.name==='submit_task');assert.equal(submissions.length,2);assert.deepEqual(submissions[0].args.intent,submissions[1].args.intent);assert.deepEqual(Object.keys(submissions[0].args.intent).sort(),['request_id','task']);
});
test('visible activity and provisional output remain bounded; gaps are disclosed',async()=>{
  const f=fixture();await f.ui.poll();await f.emit({version:1,batch_id:1,coverage_gap:true,events:Array.from({length:150},(_,i)=>activity(i+1,'x'.repeat(2048)))});
  assert.equal(f.ui.el('activity').childElementCount,80);assert.ok(f.ui.el('stream').textContent.length<=32768);assert.match(f.ui.el('coverage').textContent,/truncated|coverage gap/);
});
test('effective policy, failures and idle session switching are truthful',async()=>{
  const snapshot=state();snapshot.sessions.push({id:'s2',label:'Second',runs:0,messages:0,outcome:null});snapshot.save_error='Session store unavailable; save failed';
  const f=fixture(snapshot);await f.ui.refresh();assert.match(f.ui.el('policy').textContent,/Write deny/);assert.match(f.ui.el('notice').textContent,/save failed/);assert.equal(f.ui.el('material').textContent,'Checking material');
  f.ui.el('sessions').children[1].click();await new Promise(resolve=>setImmediate(resolve));assert.deepEqual(f.commands.find(c=>c.name==='select_session').args,{id:'s2'});
});
test('polling preserves focused controls and session changes clear provisional content',async()=>{
  const f=fixture();await f.ui.poll();const button=f.ui.el('sessions').firstElementChild;button.focus();await f.ui.refresh();assert.equal(f.ui.el('sessions').firstElementChild,button);assert.equal(f.dom.window.document.activeElement,button);
  await f.emit({batch_id:1,coverage_gap:false,events:[activity(1,'provisional previous conversation')]});assert.equal(f.ui.el('current').hidden,false);
  f.snapshot.selected_session='s2';f.snapshot.sessions.push({id:'s2',label:'Fresh',runs:0,messages:0,outcome:null});await f.ui.refresh();assert.equal(f.ui.el('current').hidden,true);assert.equal(f.ui.el('stream').textContent,'');assert.equal(f.ui.el('activity').childElementCount,0);
});
test('reference sidebar controls filter real sessions and preserve access after collapse',async()=>{
 const f=fixture();await f.ui.refresh();const search=f.ui.el('session-search');search.value='missing';search.dispatchEvent(new f.dom.window.Event('input'));assert.equal(f.ui.el('sessions').firstElementChild.hidden,true);search.value='Checklist';search.dispatchEvent(new f.dom.window.Event('input'));assert.equal(f.ui.el('sessions').firstElementChild.hidden,false);
 f.ui.el('sidebar-toggle').click();assert.equal(f.dom.window.document.body.classList.contains('sidebar-hidden'),true);f.ui.el('sidebar-return').click();assert.equal(f.dom.window.document.body.classList.contains('sidebar-hidden'),false);
 f.ui.el('details-toggle').click();assert.equal(f.ui.el('activity-panel').open,true);
});
test('polling does not reset a focused native workflow choice before change commits',async()=>{
 const f=fixture();await f.ui.refresh();f.ui.el('settings').setAttribute('open','');const workflow=f.ui.el('workflow').querySelector('[value=coding_ask]');workflow.focus();assert.equal(f.dom.window.document.activeElement,workflow);workflow.checked=true;await f.ui.refresh();assert.equal(workflow.checked,true);workflow.dispatchEvent(new f.dom.window.Event('change',{bubbles:true}));await new Promise(resolve=>setImmediate(resolve));assert.deepEqual(f.commands.find(c=>c.name==='set_workflow').args,{workflow:'coding_ask'});
});
test('assistant Markdown formats actual content without interpreting HTML or executable links',async()=>{
 const text='### Outstanding\n\n- [ ] Read **source** `notes.md`\n\n<script>steal()</script> [approve](javascript:grant())\n\n```text\n<svg onload="steal()">\n```';const f=fixture(state(),[{index:0,role:'assistant',text:{value:text,omitted_bytes:0}}]);await f.ui.refresh();assert.equal(f.ui.el('messages').querySelector('h3').textContent,'Outstanding');assert.equal(f.ui.el('messages').querySelector('code').textContent,'notes.md');assert.equal(f.ui.el('messages').querySelectorAll('script,a,svg').length,0);assert.match(f.ui.el('messages').textContent,/<script>steal/);
});
