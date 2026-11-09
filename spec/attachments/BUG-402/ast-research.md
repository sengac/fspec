# BUG-402 AST research

`grep -n "Err(_)\|\.ok()" rust/sessions/src/handle_impl/*.rs` (30 sites, file:line:text):

```
handle_impl_1.rs:223:                    .ok()
handle_impl_1.rs:229:                    .ok()
handle_impl_3.rs:23:            .ok()
handle_impl_3.rs:142:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_3.rs:218:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_3.rs:228:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_3.rs:238:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_9.rs:16:                Err(_) => continue, // non-repository project roots ar
handle_impl_4.rs:23:            Err(_) => Err(format!("Session not found: {}", session_id.
handle_impl_4.rs:29:            .ok()
handle_impl_4.rs:37:            .ok()
handle_impl_4.rs:53:            .ok()
handle_impl_4.rs:110:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_4.rs:120:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_4.rs:179:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_4.rs:204:                    .ok()
handle_impl_4.rs:210:                    .ok()
handle_impl_2.rs:8:            .ok()
handle_impl_2.rs:50:            Err(_) => Err(format!("Session not found: {}", session_id.
handle_impl_2.rs:185:            .ok()
handle_impl_2.rs:211:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_2.rs:238:            .ok()
handle_impl_6.rs:14:            Err(_) => return Err(format!("Session not found: {}", sess
handle_impl_6.rs:199:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_6.rs:217:            Err(_) => Err(format!("Session not found: {}", session_id
handle_impl_7.rs:19:            Err(_) => Err(format!("Session not found: {}", session_id.
handle_impl_7.rs:25:            .ok()
handle_impl_7.rs:31:            .ok()
handle_impl_7.rs:44:            Err(_) => Err(format!("Session not found: {}", session_id.
handle_impl_7.rs:232:            Err(_) => {
```

Each site still needs checking that the dropped error comes from `get_session` (some `.ok()` calls may be on other results).
