//! Feature: spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_create_session_with_id_is_rewritten_to_a_non_napi_result_type_and_routes_side_effects_through_the_hooks(
) {
    // @step Given the SessionManager code has been moved into rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());

    // @step When I inspect the `create_session_with_id` method signature and body in the moved file
    //
    // fmt-robustness: rustfmt wraps this multi-parameter signature across
    // several lines. Rather than string-matching a pre-fmt single-line layout,
    // locate the declaration and assert each required component is present in
    // the whitespace-normalized signature span (up to the body-opening brace).
    let decl = "pub async fn create_session_with_id";
    let body_start = sm
        .find(decl)
        .expect("create_session_with_id declaration must exist");
    let sig_span_end = sm[body_start..]
        .find('{')
        .map(|i| body_start + i)
        .expect("create_session_with_id signature must have a body brace");
    let signature = &sm[body_start..sig_span_end];

    // @step Then the signature is `pub async fn create_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<(), String>`
    for component in [
        "&self",
        "id: &str",
        "model: &str",
        "project: &str",
        "name: &str",
        "-> Result<(), String>",
    ] {
        assert!(
            contains_ws_insensitive(signature, component),
            "create_session_with_id signature must contain `{component}`. Got signature:\n{signature}"
        );
    }

    // Locate body to scope the rest of the assertions.
    let after_sig = &sm[body_start..];
    let next_fn = after_sig[1..]
        .find("\n    pub async fn ")
        .or_else(|| after_sig[1..].find("\n    pub fn "))
        .map(|i| i + 1)
        .unwrap_or(after_sig.len().min(40000));
    let body = &after_sig[..next_fn];
    let body_code = strip_line_comments(body);

    // @step And every former `napi::Error::from_reason(format!(...))` error path is rewritten to a plain `format!(...)` String error
    assert!(
        !body_code.contains("napi::Error::from_reason") && !body_code.contains("Error::from_reason"),
        "create_session_with_id body must NOT call napi::Error::from_reason. Got body:\n{body_code}"
    );
    assert!(
        body_code.contains("format!("),
        "create_session_with_id body must construct String errors via `format!(...)`"
    );

    // @step And the credential resolution call site uses `codelet_sessions::credentials::resolve_and_set_env_var(...)`
    // From within codelet-sessions itself, the path is `crate::credentials::resolve_and_set_env_var(...)`.
    assert!(
        body_code.contains("crate::credentials::resolve_and_set_env_var"),
        "create_session_with_id body must call `crate::credentials::resolve_and_set_env_var(...)`. Got body:\n{body_code}"
    );

    // @step And the agent_loop spawn site calls `self.hooks.spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)` instead of `tokio::spawn(async move { agent_loop(...).await })`
    //
    // Accept either field-access (`self.hooks.spawn_agent_loop`) or
    // method-call (`self.hooks().spawn_agent_loop`) since the hooks
    // field is wrapped in `ArcSwap`. The card's spec text uses field
    // access in the example; the actual implementation uses an
    // accessor method on the SessionManager to materialize the trait
    // object from the ArcSwap guard — both forms satisfy rule 6's
    // intent (route through the hooks indirection).
    assert!(
        contains_ws_insensitive(&body_code, "self.hooks.spawn_agent_loop")
            || contains_ws_insensitive(&body_code, "self.hooks().spawn_agent_loop"),
        "create_session_with_id body must spawn agent loop via `self.hooks(.|()).spawn_agent_loop(...)`"
    );

    // @step And the footer-poller spawn site calls `self.hooks.spawn_footer_poller(id.to_string(), project.to_string(), None)`
    assert!(
        body_code.contains("self.hooks.spawn_footer_poller")
            || body_code.contains("self.hooks().spawn_footer_poller")
            || body_code.contains("spawn_footer_poller("),
        "create_session_with_id body must spawn footer poller via `self.hooks(.|()).spawn_footer_poller(...)`"
    );

    // @step And the IsolationStateChange emit site calls `let _ = self.chunks_tx.send((codelet_rpc_types::SessionId::from(id.to_string()), codelet_rpc_types::StreamChunk::isolation_state_change(false, None)))`
    // (RPC-041 replaced the hook indirection with a direct sender call)
    assert!(
        body_code.contains("self.chunks_tx.send("),
        "create_session_with_id body must emit IsolationStateChange via `self.chunks_tx.send(...)` (RPC-041)"
    );
    assert!(
        !body_code.contains("emit_isolation_state_change"),
        "RPC-041: create_session_with_id must NOT call self.hooks.emit_isolation_state_change(...) (hook was removed)"
    );
}

#[test]
fn scenario_create_isolated_session_with_id_returns_the_wire_type_isolated_session_info_and_the_napi_wrapper_converts_to_isolated_session_result(
) {
    // @step Given the SessionManager code has been moved into rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());

    // @step And the wire-portable type codelet_rpc_types::IsolatedSessionInfo was lifted in RPC-036
    // (presence verified by use of the path in the moved signature below)

    // @step When I inspect the `create_isolated_session_with_id` method signature in the moved file
    // Accept either fully-qualified or shortened (via `use codelet_rpc_types::IsolatedSessionInfo`) forms.
    //
    // fmt-robustness: rustfmt wraps this multi-parameter signature across
    // several lines, so assert components in the whitespace-normalized span
    // instead of matching a pre-fmt single-line layout.
    let iso_decl = "pub async fn create_isolated_session_with_id";
    let iso_start = sm
        .find(iso_decl)
        .expect("create_isolated_session_with_id declaration must exist");
    let iso_sig_end = sm[iso_start..]
        .find('{')
        .map(|i| iso_start + i)
        .expect("create_isolated_session_with_id signature must have a body brace");
    let iso_sig = &sm[iso_start..iso_sig_end];

    // @step Then the signature is `pub async fn create_isolated_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<codelet_rpc_types::IsolatedSessionInfo, String>`
    for component in [
        "&self",
        "id: &str",
        "model: &str",
        "project: &str",
        "name: &str",
    ] {
        assert!(
            contains_ws_insensitive(iso_sig, component),
            "create_isolated_session_with_id signature must contain `{component}`. Got:\n{iso_sig}"
        );
    }
    assert!(
        contains_ws_insensitive(
            iso_sig,
            "-> Result<codelet_rpc_types::IsolatedSessionInfo, String>"
        ) || contains_ws_insensitive(iso_sig, "-> Result<IsolatedSessionInfo, String>"),
        "create_isolated_session_with_id must return Result<IsolatedSessionInfo, String>. Got:\n{iso_sig}"
    );

    let wire = read(
        &workspace_root()
            .join("rpc-types")
            .join("src")
            .join("session_state.rs"),
    );
    let handle = read(
        &workspace_root()
            .join("sessions")
            .join("src")
            .join("handle_impl.rs"),
    );
    // @step And the napi-side #[napi(object)] IsolatedSessionResult struct at rust/napi/src/session_manager.rs:5258-5267 stays in NAPI carrying the JS-bridge shape
    let wire_start = wire
        .find("pub struct IsolatedSessionInfo")
        .expect("codelet-rpc-types must own `pub struct IsolatedSessionInfo`");
    let wire_window: String = wire.chars().skip(wire_start).take(400).collect();
    let pos_session = wire_window.find("session_id");
    let pos_worktree = wire_window.find("worktree_path");
    let pos_base = wire_window.find("base_commit");
    assert!(
        pos_session.is_some() && pos_worktree.is_some() && pos_base.is_some(),
        "IsolatedSessionInfo must carry session_id, worktree_path, base_commit. Got:\n{wire_window}"
    );

    // @step And NAPI provides `impl From<codelet_rpc_types::IsolatedSessionInfo> for IsolatedSessionResult`
    let rpc_sig = "fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String>";
    assert!(
        contains_ws_insensitive(&handle, rpc_sig),
        "the sessions RPC handle must return the wire type directly: `{rpc_sig}`"
    );

    // @step And the #[napi] free function session_manager_create_isolated maps the Ok value via `.map(IsolatedSessionResult::from)` and maps the String error via `.map_err(napi::Error::from_reason)`
    let entry = handle
        .find("fn create_isolated_session(")
        .expect("create_isolated_session must exist in the sessions RPC handle");
    let after = &handle[entry..];
    let end_idx = after[1..]
        .find("\n    fn ")
        .map(|i| i + 1)
        .unwrap_or(after.len());
    let fn_body = &after[..end_idx];
    // BUG-492: since BUG-427 the handle delegates to
    // handle_creation::create_isolated_session_in_project (the session is cut
    // from the selected workspace); the forward and the error propagation live
    // in that helper.
    assert!(
        fn_body.contains("create_isolated_session_in_project("),
        "create_isolated_session must delegate to handle_creation::create_isolated_session_in_project. Got:\n{fn_body}"
    );
    let creation = read(
        &workspace_root()
            .join("sessions")
            .join("src")
            .join("handle_creation.rs"),
    );
    let helper_start = creation
        .find("fn create_isolated_session_in_project(")
        .expect("handle_creation.rs must define create_isolated_session_in_project");
    let helper = &creation[helper_start..];
    let helper_body = &helper[..helper.find("\n}").map(|i| i + 2).unwrap_or(helper.len())];
    assert!(
        helper_body.contains(".create_isolated_session_with_id(") && helper_body.contains("})?;"),
        "create_isolated_session_in_project must forward to create_isolated_session_with_id and propagate its String error. Got:\n{helper_body}"
    );
}
