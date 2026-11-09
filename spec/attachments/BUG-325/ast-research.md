# AST research: BUG-325

## `provider_key: $E` in rust/fspec-tui/src/views/model_selector
```
No matches found
```

## `let provider_id = row.provider_key.clone()` in rust/fspec-tui/src/views/model_selector
```
rust/fspec-tui/src/views/model_selector/crud.rs:25:9:let provider_id = row.provider_key.clone();
rust/fspec-tui/src/views/model_selector/crud.rs:45:9:let provider_id = row.provider_key.clone();

```

## `let _ = backend.$M($$$A).await` in rust/fspec-tui/src/app
```
rust/fspec-tui/src/app/dispatch_slash_submit.rs:163:17:let _ = backend.send_input(session_for_send, text_for_send).await;
rust/fspec-tui/src/app/dispatch_stream_chunks.rs:203:17:let _ = backend.interrupt(session).await;
rust/fspec-tui/src/app/dispatch_stream_chunks.rs:260:13:let _ = backend.send_fspec_result(session_id, result).await;
rust/fspec-tui/src/app/dispatch_history_recall.rs:174:13:let _ = backend.persistence_add_history(session, text).await;
rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:185:17:let _ = backend.set_debug_enabled(sid.clone(), true).await;
rust/fspec-tui/src/app/dispatch_agent_exit.rs:133:25:let _ = backend.destroy_session(session_for_destroy).await;
rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:142:13:let _ = backend.set_thinking_level(session_id, level).await;
rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:166:13:let _ = backend.set_thinking_level_default(session_id, level).await;
rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:187:13:let _ = backend.set_session_role(session_id, role).await;
rust/fspec-tui/src/app/dispatch_esc_cascade.rs:56:17:let _ = backend.interrupt(session).await;
rust/fspec-tui/src/app/merge_worktree_success.rs:54:17:let _ = backend.destroy_session(session_id).await;
rust/fspec-tui/src/app/dispatch_session_chrome.rs:85:13:let _ = backend.set_thinking_level(sid.clone(), level).await;
rust/fspec-tui/src/app/dispatch_model_selector.rs:101:21:let _ = backend
rust/fspec-tui/src/app/dispatch_model_selector.rs:111:21:let _ = backend
rust/fspec-tui/src/app/dispatch_model_selector.rs:125:21:let _ = backend

```

## `format!("{provider_id}:{profile_name}")` in rust/sessions/src
```
rust/sessions/src/profile_sections.rs:73:14:format!("{provider_id}:{profile_name}")

```

## `return Ok(())` in rust/sessions/src/profile_sections.rs
```
rust/sessions/src/profile_sections.rs:345:9:return Ok(())
rust/sessions/src/profile_sections.rs:368:9:return Ok(())
rust/sessions/src/profile_sections.rs:404:9:return Ok(())
rust/sessions/src/profile_sections.rs:407:9:return Ok(())
rust/sessions/src/profile_sections.rs:445:9:return Ok(())
rust/sessions/src/profile_sections.rs:448:9:return Ok(())

```

## `None => Ok(())` in rust/rpc/src/service_implementation/providers.rs
```
No matches found
```

