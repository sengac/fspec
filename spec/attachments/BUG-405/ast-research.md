# BUG-405 AST research

`grep -rn "SessionManager::instance()" rust/agent-loop/src rust/sessions/src` (non-comment lines):

```
agent-loop/src/fspec_handler.rs:12:            .unwrap_or_else(|| codelet_sessions::SessionManager::instance());
sessions/src/session_tool_callbacks.rs:91:        None => SessionManager::instance().get_session(&session_id).ok(),
agent-loop/src/agent_manager_handler.rs:56:                None => SessionManager::instance(),
agent-loop/src/agent_manager_handler.rs:852:        None => SessionManager::instance(),
agent-loop/src/agent_manager_handler.rs:1225:        let session_manager = SessionManager::instance();
agent-loop/src/agent_manager_handler.rs:1239:        let session_manager = SessionManager::instance();
sessions/src/tool_admission.rs:16:            None if singleton => SessionManager::instance().admit_tool_invocation(&id, attribution),
sessions/src/tool_admission.rs:25:            None if singleton => SessionManager::instance().get_session(&validator_id),
sessions/src/footer_poller.rs:76:            crate::session_manager::SessionManager::instance()
sessions/src/background_session/focus.rs:27:            .unwrap_or_else(|| crate::SessionManager::instance());
```
