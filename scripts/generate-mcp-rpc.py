from pathlib import Path
import json,re,hashlib
root=Path(__file__).resolve().parents[1]
stage=root/"rust/rpc/src/mcp_dispatch";stage.mkdir(parents=True,exist_ok=True)
s=(root/'rust/rpc/src/lib.rs').read_text().split('pub trait FspecService {',1)[1].split('\n}\n',1)[0];s=re.sub(r'///[^\n]*','',s)
methods=[]
for n,a,r in re.findall(r'async fn (\w+)\((.*?)\)\s*(?:->\s*(.*?))?;',s,re.S):
 parts=[];depth=0;start=0
 for i,c in enumerate(a):
  depth+=(c in '<([')-(c in '>)]')
  if c==',' and depth==0:parts.append(a[start:i].strip());start=i+1
 if a[start:].strip():parts.append(a[start:].strip())
 args=[(p.split(':',1)[0].strip(),' '.join(p.split(':',1)[1].split())) for p in parts if p]
 methods.append({'name':n,'args':args,'return':' '.join(r.split())})

reads=set(['blocklist_list', 'changed_files', 'checkpoint_counts', 'checkpoint_diff_files', 'checkpoint_file_diff', 'file_diff', 'get_active_session', 'get_buffered_output', 'get_compaction_progress', 'get_continue_state', 'get_debug_enabled', 'get_effective_cwd', 'get_exec_stdin_request', 'get_goal_state', 'get_hitl_request', 'get_interaction_snapshot', 'get_model_info', 'get_pause_state', 'get_pending_input', 'get_provider_credential', 'get_session_model', 'get_session_role', 'get_session_status', 'get_session_tokens', 'get_subordinate', 'get_subordinates', 'get_supervisors', 'get_thinking_level', 'get_transcript_page', 'get_work_unit_context', 'get_work_unit_focus', 'get_workspace_info', 'health', 'inspect_session_changes', 'list_checkpoints', 'list_live_sessions', 'list_provider_credentials', 'list_provider_settings', 'list_providers', 'list_session_worktrees', 'list_sessions', 'list_work_units', 'loop_list', 'oauth_get_tokens', 'persistence_get_history', 'persistence_search_history', 'read_work_units_bounded', 'schedule_list', 'search_files'])
checked=set('pause_resume pause_confirm pause_triple send_hitl_response'.split())
policy={m['name']:{'read_only':m['name'] in reads,'destructive':m['name'] not in reads,'idempotent':m['name'] in reads,'open_world':m['name'] not in reads,'request_bound':m['name'] in checked} for m in methods}

modules=[]
for group in range((len(methods)+2)//3):
 name=f'group_{group:02}';modules.append(name);lines=['use super::*;\n']
 for m in methods[group*3:(group+1)*3]:
  fields=', '.join(f'{n}: {t}' for n,t in m['args']);vals=', '.join('args.'+n for n,_ in m['args'])
  call=f'client.{m["name"]}(tarpc::context::current(){", " if vals else ""}{vals}).await.map_err(|e| InvokeError::Transport(e.to_string()))?'
  if m['return'].startswith('Result<'): call=f'{call}.map_err(|e| InvokeError::Daemon(format!("{{e:?}}")))?'
  lines.append(f'''pub(super) mod {m['name']} {{
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {{ {fields} }}
    pub fn schema() -> Value {{ serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default() }}
    pub fn validate(value: Value) -> Result<(), InvokeError> {{ let _: Args = serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?; Ok(()) }}
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {{
        let args: Args = serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let _ = &args;
        let value = {call};
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }}
}}
''')
  m['module']=name
 (stage/(name+'.rs')).write_text(''.join(lines))
lines=['use super::*;\n','#[rustfmt::skip]\npub fn methods() -> Vec<Method> { vec![\n']
for m in methods:
 p=policy[m['name']];lines.append(f'Method {{ name: "{m["name"]}", schema: {m["module"]}::{m["name"]}::schema(), read_only: {str(p["read_only"]).lower()}, destructive: {str(p["destructive"]).lower()}, request_bound: {str(p["request_bound"]).lower()} }},\n')
lines.append('] }\n')
(stage/'catalog.rs').write_text(''.join(lines))
lines=['use super::*;\n','pub fn validate(name: &str, args: Value) -> Result<(), InvokeError> { match name {\n']
for m in methods:lines.append(f'"{m["name"]}" => {m["module"]}::{m["name"]}::validate(args),\n')
lines.append('_ => Err(InvokeError::Unknown),\n} }\n')
(stage/'validate.rs').write_text(''.join(lines))
lines=['use super::*;\n','pub async fn invoke(client: &FspecServiceClient, name: &str, args: Value) -> Result<Value, InvokeError> { match name {\n']
for m in methods:lines.append(f'"{m["name"]}" => {m["module"]}::{m["name"]}::call(client,args).await,\n')
lines.append('_ => Err(InvokeError::Unknown),\n} }\n')
(stage/'dispatch.rs').write_text(''.join(lines))
mod='''//! Typed MCP-facing dispatch. Generated method modules are audited against FspecService.
use crate::FspecServiceClient;
use codelet_rpc_types::*;
use serde_json::Value;
#[derive(Debug, thiserror::Error)]
pub enum InvokeError {
    #[error("Invalid arguments: {0}")] Arguments(String),
    #[error("Transport failed; execution may be uncertain: {0}")] Transport(String),
    #[error("Daemon rejected operation: {0}")] Daemon(String),
    #[error("Result serialization failed after execution: {0}")] Serialization(String),
    #[error("Unknown RPC method")] Unknown,
}
pub struct Method { pub name: &'static str, pub schema: Value, pub read_only: bool, pub destructive: bool, pub request_bound: bool }
'''
mod+='\n'.join(f'mod {x};' for x in modules)+'\nmod catalog;\nmod dispatch;\nmod validate;\npub use catalog::methods;\npub use dispatch::invoke;\npub use validate::validate;\n'
mod+='pub const INTERFACE: &str = env!("FSPEC_RPC_SCHEMA");\n'
(stage/'mod.rs').write_text(mod)
print('Generated',len(methods),'typed methods;',len(reads),'read-only,',len(methods)-len(reads),'state-changing')

import subprocess
subprocess.run(["rustfmt","--edition","2021",str(stage/"mod.rs")],check=True)
