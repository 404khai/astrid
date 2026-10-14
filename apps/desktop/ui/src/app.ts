type Text = {value:string;omitted_bytes:number};
type Permission = {id:string;run_id:string;tool_call_id:string;command:Text;workspace:Text;can_approve:boolean;authority:string};
type Run = {request_id:string;session_id:string;run_id:string|null;status:string;cancellation_requested:boolean;permission:Permission|null};
type Session = {id:string;label:string;runs:number;messages:number;outcome:string|null};
type Snapshot = {version:number;workspace:Text|null;model:string;provider:string;workflow:string;policy:{read:string;write:string;execute:string};selected_session:string|null;sessions:Session[];run:Run|null;save_error:string|null;quitting:boolean;auth_phase:string;client_error:string|null;restoring:boolean};
type Message = {index:number;role:string;text:Text};
type Page = {version:number;messages:Message[];next:number|null};
type Activity = {ordinal:number;session_id:string;run_id:string;sequence:number;kind:string;text:Text;tool_call_id:string|null;model_call_id:string|null;turn_id:string|null};
type Batch = {batch_id:number;coverage_gap:boolean;events:Activity[]};
export type Invoke = (name:string,args?:Record<string,unknown>)=>Promise<unknown>;
export class WorkspaceUI {
  snapshot:Snapshot|null=null;subscription:string|null=null;ack:number|null=null;cursor=0;historyOffset=0;nextOffset:number|null=null;
  private displayedSession:string|null=null;private historySequence=0;private fetchSequence=0;private appliedSequence=0;private sessionKey="";private generation=0;private subscriptionPromise:Promise<string>|null=null;private historyKey="";private polling=false;private pending:{request_id:string;task:string}|null=null;private streamBytes=0;
  constructor(readonly document:Document,readonly invoke:Invoke,readonly requestId:()=>string=()=>crypto.randomUUID(),readonly channel:(receive:(batch:Batch)=>void)=>unknown=()=>{throw new Error("Native channel unavailable");}) {
    this.el("folder").addEventListener("click",()=>void this.action("pick_folder"));
    this.el("new-session").addEventListener("click",()=>void this.action("new_session"));
    this.el("workflow").addEventListener("change",event=>{const input=event.target as HTMLInputElement;if(input.checked)void this.action("set_workflow",{workflow:input.value});});
    this.el("composer").addEventListener("submit",e=>{e.preventDefault();void this.submit();});
    this.el("task").addEventListener("input",()=>this.controls());
    this.el("cancel").addEventListener("click",()=>{if(this.snapshot?.run)void this.action("cancel_run",{request:this.snapshot.run.request_id});});
    this.el("login").addEventListener("click",()=>void this.action("sign_in"));
    this.el("refresh-view").addEventListener("click",()=>this.document.defaultView?.location.reload());
    this.el("quit").addEventListener("click",()=>void this.action("quit_application"));
    this.el("older").addEventListener("click",()=>{this.historyOffset=Math.max(0,this.historyOffset-16);void this.loadHistory();});
    this.el("newer").addEventListener("click",()=>{if(this.nextOffset!==null){this.historyOffset=this.nextOffset;void this.loadHistory();}});
    const toggleSidebar=()=>this.document.body.classList.toggle("sidebar-hidden");
    this.el("sidebar-toggle").addEventListener("click",toggleSidebar);this.el("sidebar-return").addEventListener("click",toggleSidebar);
    this.el("details-toggle").addEventListener("click",()=>{const details=this.el("activity-panel") as HTMLDetailsElement;details.open=!details.open;if(details.open)details.scrollIntoView?.({block:"nearest"});});
    this.el("session-search").addEventListener("input",()=>this.filterSessions());
    const settings=this.el("settings") as HTMLDialogElement;
    this.el("settings-button").addEventListener("click",()=>settings.showModal());
    this.el("settings-close").addEventListener("click",()=>settings.close());
    document.addEventListener("keydown",e=>{if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==="k"){e.preventDefault();this.document.body.classList.remove("sidebar-hidden");this.el("session-search").focus();}if((e.metaKey||e.ctrlKey)&&e.key===","){e.preventDefault();settings.showModal();}});
  }
  el(id:string):HTMLElement {const node=this.document.getElementById(id);if(!node)throw new Error(`Missing control ${id}`);return node;}
  set(id:string,value:string) {if(this.el(id).textContent!==value)this.el(id).textContent=value;}
  notice(message:string) {this.set("notice",message);this.el("notice").hidden=!message;}
  private filterSessions(){const query=(this.el("session-search") as HTMLInputElement).value.toLocaleLowerCase();for(const node of Array.from(this.el("sessions").children)) (node as HTMLElement).hidden=!(node.textContent??"").toLocaleLowerCase().includes(query);}
  async action(command:string,args?:Record<string,unknown>) {
    try {await this.invoke(command,args);this.historyKey="";this.notice("");await this.refresh();}catch(error){this.notice(String(error));}
  }
  private active() {return Boolean(this.snapshot?.run&&["starting","running","waiting_permission"].includes(this.snapshot.run.status));}
  private controls() {
    const active=this.active();const disabled=active||Boolean(this.snapshot?.quitting)||Boolean(this.snapshot?.restoring)||this.snapshot?.auth_phase==="signing_in";
    for(const id of ["folder","new-session","workflow","login"]){const control=this.el(id) as HTMLButtonElement;if(control.disabled!==disabled)control.disabled=disabled;}
    (this.el("send") as HTMLButtonElement).disabled=disabled||!this.snapshot?.workspace||!(this.el("task") as HTMLTextAreaElement).value.trim();
    this.el("cancel").hidden=!active;
  }
  async submit() {
    const task=(this.el("task") as HTMLTextAreaElement).value;
    if(!task.trim()||this.active()||!this.snapshot?.workspace)return;
    if(!this.pending||this.pending.task!==task)this.pending={request_id:this.requestId(),task};
    try {await this.invoke("submit_task",{intent:this.pending});this.pending=null;(this.el("task") as HTMLTextAreaElement).value="";this.set("stream","");this.streamBytes=0;this.notice("");await this.refresh();}
    catch(error){this.notice(`${String(error)}. If receipt is uncertain, retry keeps the same request ID.`);}
  }
  async refresh() {
    const sequence=++this.fetchSequence;const s=await this.invoke("snapshot") as Snapshot;if(sequence<this.appliedSequence)return;this.appliedSequence=sequence;
    if(this.displayedSession!==s.selected_session){this.displayedSession=s.selected_session;this.historySequence++;this.el("current").hidden=true;this.set("stream","");this.streamBytes=0;this.el("activity").replaceChildren();this.set("activity-count","0");this.set("coverage","");}
    this.snapshot=s;
    this.set("workspace",s.workspace?.value??"No folder selected");
    this.set("provider",`${s.provider==="openai_codex_subscription"?"ChatGPT subscription":s.provider} · ${s.model}`);
    this.set("policy",`Read ${s.policy.read} · Write ${s.policy.write} · Execute ${s.policy.execute}`);
    const workflow=this.el("workflow");if(!workflow.contains(this.document.activeElement)){for(const input of Array.from(workflow.querySelectorAll<HTMLInputElement>("input"))){const checked=input.value===s.workflow;if(input.checked!==checked)input.checked=checked;}}
    this.set("authority-note",s.workflow==="notes"?"Notes are read only. Model-requested writes and shell commands are denied.":"Native paths remain workspace-validated. Allowed shell commands have broad local-account authority.");
    this.set("auth-status",s.auth_phase==="signed_in"?"Sign-in completed; tool permissions are unchanged.":s.auth_phase==="signing_in"?"Complete sign-in in the browser. Credentials stay in Astrid’s private Rust storage.":s.auth_phase==="failed"?"Sign-in failed or was cancelled. Retry sign-in; no credentials were exported.":"Uses existing Astrid sign-in. If requests fail, sign in here.");
    const selected=s.sessions.find(e=>e.id===s.selected_session);
    this.document.body.classList.toggle("has-history",Boolean(selected?.messages));this.document.body.classList.toggle("run-active",this.active());
    this.set("conversation-title",selected?.label??"New conversation");
    this.set("run-status",s.quitting?"Quitting — waiting for cleanup and idle save":s.run?.status==="completed"?"Run completed · verify task success from supporting evidence":s.run?.status??"Ready");
    this.el("welcome").hidden=Boolean(selected?.messages||this.active());this.el("conversation").hidden=!selected?.messages;
    if(s.save_error||s.client_error)this.notice(s.save_error??s.client_error??"");
    const sessions=this.el("sessions");const sessionKey=JSON.stringify([s.sessions,s.selected_session,this.active()]);
    if(sessionKey!==this.sessionKey){this.sessionKey=sessionKey;sessions.replaceChildren();
    for(const entry of s.sessions){const button=this.document.createElement("button");button.className="session";button.textContent=entry.label;button.setAttribute("aria-current",String(entry.id===s.selected_session));button.disabled=this.active();const meta=this.document.createElement("small");meta.textContent=`${entry.runs} runs · ${entry.outcome??"new"}`;const model=this.document.createElement("small");model.className="session-model";model.textContent=s.model;button.prepend(model);button.append(meta);button.onclick=()=>void this.action("select_session",{id:entry.id});sessions.append(button);}this.filterSessions();}
    const permission=s.run?.permission;this.el("approval").hidden=!permission;
    if(permission){this.set("approval-authority",permission.authority);this.set("approval-workspace",`Workspace: ${permission.workspace.value}`);this.set("approval-command",permission.command.value);this.set("approval-limit",permission.can_approve?"Grant applies only to this pending run/tool request.":"Command or workspace is truncated. Approval is unavailable; deny or cancel this run.");(this.el("allow") as HTMLButtonElement).disabled=!permission.can_approve;const args={id:permission.id,run:permission.run_id,tool:permission.tool_call_id};this.el("allow").onclick=()=>void this.action("decide_permission",{...args,allow:true});this.el("deny").onclick=()=>void this.action("decide_permission",{...args,allow:false});}
    this.controls();
    const key=`${s.selected_session}:${selected?.messages}`;
    if(selected&&key!==this.historyKey){this.historyKey=key;this.historyOffset=Math.max(0,selected.messages-16);await this.loadHistory();}
    if(this.pending&&s.run?.request_id===this.pending.request_id)this.pending=null;
  }
  private markdown(target:HTMLElement,text:string){
    // Bounded presentation subset. Never parse HTML, links, images or executable URLs.
    const inline=(node:HTMLElement,line:string)=>{for(const part of line.split(/(\*\*[^*]+\*\*|`[^`]+`)/g)){if(part.startsWith("**")&&part.endsWith("**")){const strong=this.document.createElement("b");strong.textContent=part.slice(2,-2);node.append(strong);}else if(part.startsWith("`")&&part.endsWith("`")){const code=this.document.createElement("code");code.textContent=part.slice(1,-1);node.append(code);}else node.append(this.document.createTextNode(part));}};
    let code:HTMLElement|null=null;let list:HTMLElement|null=null;
    for(const line of text.split("\n")){if(line.startsWith("```")){if(code){code=null;}else{code=this.document.createElement("pre");target.append(code);}list=null;continue;}if(code){code.textContent+=(code.textContent?"\n":"")+line;continue;}
      const heading=line.match(/^#{1,4} (.+)$/);const bullet=line.match(/^[-*] (.+)$/);
      if(heading){const h=this.document.createElement("h3");inline(h,heading[1]);target.append(h);list=null;}
      else if(bullet){if(!list){list=this.document.createElement("ul");target.append(list);}const item=this.document.createElement("li");inline(item,bullet[1].replace(/^\[ \] /,"☐ ").replace(/^\[x\] /i,"☑ "));list.append(item);}
      else{list=null;if(line){const paragraph=this.document.createElement("p");inline(paragraph,line);target.append(paragraph);}}
    }
  }
  async loadHistory() {
    if(!this.snapshot?.selected_session)return;
    const session=this.snapshot.selected_session;const offset=this.historyOffset;const sequence=++this.historySequence;
    const page=await this.invoke("history",{offset}) as Page;if(sequence!==this.historySequence||session!==this.snapshot?.selected_session)return;this.nextOffset=page.next;
    const container=this.el("messages");container.replaceChildren();
    for(const message of page.messages){if(message.role==="assistant"&&!message.text.value&&!message.text.omitted_bytes)continue;const node=this.document.createElement("article");node.className=`message ${message.role}`;const role=this.document.createElement("strong");role.textContent=message.role==="assistant"?"Meister":message.role;const content=this.document.createElement("div");if(message.role==="assistant")this.markdown(content,message.text.value);else content.textContent=message.text.value;
      if(message.role==="tool"){const details=this.document.createElement("details");const summary=this.document.createElement("summary");summary.textContent=`${message.text.value.split(" ",1)[0]} · source / tool result`;content.className="tool-content";details.append(summary,content);node.append(details);}else node.append(role,content);if(message.text.omitted_bytes){const omitted=this.document.createElement("small");omitted.textContent=`${message.text.omitted_bytes} bytes omitted from this visible projection.`;node.append(omitted);}container.append(node);}
    (this.el("older") as HTMLButtonElement).disabled=this.historyOffset===0;(this.el("newer") as HTMLButtonElement).disabled=page.next===null;
    this.set("page-label",page.messages.length?`Messages ${this.historyOffset+1}–${this.historyOffset+page.messages.length} · paginated history`:"No committed messages");
  }
  private renderBatch(batch:Batch) {
    if(batch.coverage_gap)this.set("coverage","Activity coverage gap: older projection data was omitted. Current state and committed history remain available.");
    for(const event of batch.events){this.cursor=event.ordinal;if(event.session_id!==this.snapshot?.selected_session)continue;
      const node=this.document.createElement("li");node.textContent=`${event.kind.replaceAll("_"," ")}\n${event.text.value}${event.text.omitted_bytes?`\n${event.text.omitted_bytes} bytes omitted`:""}`;const correlation=this.document.createElement("small");correlation.textContent=`Run ${event.run_id} · seq ${event.sequence} · tool ${event.tool_call_id??"—"}`;node.append(correlation);this.el("activity").append(node);while(this.el("activity").childElementCount>80)this.el("activity").firstElementChild?.remove();
      if(event.kind==="model_text_delta"){this.el("current").hidden=false;const old=this.el("stream").textContent??"";const incoming=event.text.value;this.streamBytes+=incoming.length;this.set("stream",(old+incoming).slice(0,32768));if(this.streamBytes>32768||event.text.omitted_bytes)this.set("coverage","Visible streaming text was truncated; inspect paginated committed history and source coverage.");}
      if(event.kind==="model_call_started"){this.set("stream","");this.streamBytes=0;}
      if(event.kind==="model_call_completed")this.el("current").hidden=true;
    }
    this.set("activity-count",String(this.el("activity").childElementCount));
  }
  async poll() {
    if(this.polling)return;this.polling=true;
    try {
      await this.refresh();
      if(this.subscription&&!await this.invoke("subscription_status",{id:this.subscription}))this.subscription=null;
      if(!this.subscription){
        const generation=++this.generation;
        const channel=this.channel(batch=>{void(async()=>{
          try {const id=await this.subscriptionPromise;
          if(!id||generation!==this.generation)return;
          this.renderBatch(batch);
          await this.invoke("acknowledge",{id,batch:batch.batch_id});}catch(error){this.subscription=null;this.notice(String(error));}
        })();});
        this.subscriptionPromise=this.invoke("subscribe",{after:this.cursor,channel}) as Promise<string>;
        this.subscription=await this.subscriptionPromise;
      }
      const material=await this.invoke("material_status") as {mode:string};this.document.body.dataset.material=material.mode;this.set("material",material.mode==="liquid_glass"?"Native Liquid Glass":material.mode==="opaque_reduced"?"Opaque · Reduce Transparency":"Opaque material fallback");
    }catch(error){this.subscription=null;this.notice(String(error));}finally{this.polling=false;}
  }
}

const global=(typeof window==="undefined"?{}:window) as unknown as {__TAURI__?:{core:{invoke:Invoke;Channel:new(receive:(batch:Batch)=>void)=>unknown}}};
if(global.__TAURI__){const ui=new WorkspaceUI(document,global.__TAURI__.core.invoke,()=>crypto.randomUUID(),receive=>new global.__TAURI__!.core.Channel(receive));void ui.poll();setInterval(()=>void ui.poll(),350);}
