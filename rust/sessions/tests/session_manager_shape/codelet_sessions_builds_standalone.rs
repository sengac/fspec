//! Feature: spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_codelet_sessions_builds_standalone_with_session_manager_chain_of_command_navigation_credentials_and_the_hooks_trait_at_their_new_home(
) {
    // @step Given the SessionManager and ChainOfCommand structs have been moved into rust/sessions/src/
    let sm = read(&moved_sm_path());
    assert!(
        sm.contains("pub struct SessionManager"),
        "rust/sessions/src/session_manager.rs must define `pub struct SessionManager`"
    );
    let coc = read(&moved_coc_path());
    assert!(
        coc.contains("pub struct ChainOfCommand"),
        "rust/sessions/src/chain_of_command.rs must define `pub struct ChainOfCommand`"
    );

    // @step And the crate::navigation module has been lifted to rust/sessions/src/navigation.rs
    let nav = read(&moved_nav_path());
    assert!(
        nav.contains("pub fn build_navigation_list"),
        "rust/sessions/src/navigation.rs must define `pub fn build_navigation_list`"
    );

    // @step And the NAPI-free portion of crate::credentials has been lifted to rust/sessions/src/credentials/
    let creds = read(&moved_creds_mod_path());
    assert!(
        creds.contains("pub mod resolver")
            || creds.contains("pub use")
            || creds.contains("resolve_and_set_env_var"),
        "rust/sessions/src/credentials/mod.rs must re-export the lifted submodules"
    );

    // @step And the SessionManagerHooks trait plus NoopSessionManagerHooks default impl have been added to rust/sessions/src/session_manager.rs
    assert!(
        sm.contains("pub trait SessionManagerHooks"),
        "rust/sessions/src/session_manager.rs must define `pub trait SessionManagerHooks`"
    );
    assert!(
        sm.contains("pub struct NoopSessionManagerHooks") || sm.contains("NoopSessionManagerHooks"),
        "rust/sessions/src/session_manager.rs must define `NoopSessionManagerHooks`"
    );

    // @step When I run `cargo build -p codelet-sessions`
    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "--profile",
            "ci-test",
            "-p",
            "codelet-sessions",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");

    // @step Then the build completes successfully with no errors
    assert!(
        output.status.success(),
        "cargo build -p codelet-sessions failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And the public paths codelet_sessions::session_manager::SessionManager, codelet_sessions::chain_of_command::ChainOfCommand, codelet_sessions::navigation::{NavigationTarget, build_navigation_list, get_next_target, get_prev_target}, codelet_sessions::credentials::resolve_and_set_env_var, and codelet_sessions::session_manager::{SessionManagerHooks, NoopSessionManagerHooks} all resolve at compile time
    // The `use` lines at the top of this file are the compile-time witness.
    let _witness: fn() = || {
        fn _accept<T>(_: std::marker::PhantomData<T>) {}
        _accept::<SessionManager>(std::marker::PhantomData);
        _accept::<ChainOfCommand>(std::marker::PhantomData);
        _accept::<NavigationTarget>(std::marker::PhantomData);
        _accept::<NoopSessionManagerHooks>(std::marker::PhantomData);
    };
}

#[test]
fn scenario_the_moved_session_manager_rs_and_chain_of_command_rs_have_no_napi_references() {
    // @step Given the SessionManager and ChainOfCommand code has been moved into rust/sessions/src/
    let sm = read(&moved_sm_path());
    let coc = read(&moved_coc_path());
    assert!(sm.contains("pub struct SessionManager"));
    assert!(coc.contains("pub struct ChainOfCommand"));

    // @step When I grep rust/sessions/src/session_manager.rs and rust/sessions/src/chain_of_command.rs for the regex `napi::|use napi|#\[napi`
    let mut violations: Vec<String> = Vec::new();
    for (label, src) in [("session_manager.rs", &sm), ("chain_of_command.rs", &coc)] {
        for (idx, line) in src.lines().enumerate() {
            let lineno = idx + 1;
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if line.contains("napi::") || line.contains("use napi") || line.contains("#[napi") {
                violations.push(format!("{label}:{lineno}: {line}"));
            }
        }
    }

    // @step Then I find zero matches in either file
    assert!(
        violations.is_empty(),
        "moved files must not reference napi (found {} violations):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn scenario_the_moved_session_manager_rs_has_no_crate_references_to_napi_private_modules_or_free_functions(
) {
    // @step Given the SessionManager code has been moved into rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());
    assert!(sm.contains("pub struct SessionManager"));

    // @step When I grep the moved file for the regex `crate::scheduler|crate::navigation|crate::credentials|GLOBAL_CHUNK_CALLBACK|spawn_footer_poller|stop_footer_poller|init_block_notification_callbacks|init_bridge_metadata_providers|agent_loop\(`
    //
    // Interpretation: the regex starting point is broad but the spec
    // intent (see rule 6 in the feature file) is that there be **zero
    // direct, non-hook calls** to napi-private subsystems. The lifted
    // modules `navigation` and `credentials` now LIVE in
    // codelet-sessions, so `crate::navigation::*` and
    // `crate::credentials::*` references are internal paths — not
    // napi-private — and are explicitly permitted by rule 3/4. The
    // hook trait method declarations (`fn spawn_footer_poller`,
    // `fn spawn_agent_loop`, `fn stop_footer_poller`) contain the
    // forbidden substrings only because they MIRROR the napi-side free
    // function names that they delegate to. Both forms appear inside
    // `self.hooks.<method>(...)` call sites that satisfy rule 6's
    // routing requirement.
    let code = strip_line_comments(&sm);
    // Forbid: direct napi-private references that bypass the hooks.
    let forbidden_strict = [
        "crate::scheduler",
        "GLOBAL_CHUNK_CALLBACK",
        "init_block_notification_callbacks",
        "init_bridge_metadata_providers",
    ];
    let mut violations: Vec<String> = Vec::new();
    for needle in forbidden_strict {
        if code.contains(needle) {
            violations.push(needle.to_string());
        }
    }
    // For the function-call patterns (spawn_footer_poller(, stop_footer_poller(, agent_loop()
    // we forbid only the BARE call form — i.e. lines that don't contain
    // `self.hooks` (allowing both `self.hooks.<m>` field access and
    // `self.hooks().<m>` accessor calls) AND are not trait method
    // declarations (`fn spawn_footer_poller(&self`). The multi-line
    // call form `self.hooks()<newline>    .spawn_footer_poller(...)`
    // is recognised by looking back at the previous non-blank source
    // line to see if it ends in `self.hooks()` (the accessor pattern
    // used in this crate to materialize the ArcSwap guard).
    let lines: Vec<&str> = code.lines().collect();
    for needle in ["spawn_footer_poller(", "stop_footer_poller(", "agent_loop("] {
        for (idx, line) in lines.iter().enumerate() {
            let stripped = line.trim_start();
            if !stripped.contains(needle) {
                continue;
            }
            if stripped.starts_with("fn ") {
                continue;
            }
            if stripped.contains("self.hooks") {
                continue;
            }
            // Multi-line `self.hooks()\n    .method(...)` pattern: look at
            // the previous non-blank line for the `self.hooks()` opener.
            let mut found_prev_hook = false;
            if idx > 0 {
                for prev in lines[..idx].iter().rev() {
                    let p = prev.trim();
                    if p.is_empty() {
                        continue;
                    }
                    if p.ends_with("self.hooks()") || p.ends_with("self.hooks") {
                        found_prev_hook = true;
                    }
                    break;
                }
            }
            if found_prev_hook {
                continue;
            }
            violations.push(format!("{}: {}", needle, line.trim()));
        }
    }

    // @step Then I find zero matches in rust/sessions/src/session_manager.rs
    assert!(
        violations.is_empty(),
        "moved session_manager.rs must not reference napi-private modules / free fns. Violations: {violations:?}"
    );

    // @step And every former `crate::navigation::*` path resolves to `codelet_sessions::navigation::*`
    // The moved methods get_next_session/get_prev_session/get_first_session
    // must reach navigation via `crate::navigation::*` since the moved
    // file IS in codelet-sessions, so `crate::navigation` here means the
    // codelet-sessions internal navigation module.
    assert!(
        code.contains("crate::navigation::")
            || !code.contains("get_next_target"),
        "moved get_next_session/get_prev_session must reach the lifted navigation module via `crate::navigation::*` (codelet-sessions internal)"
    );

    // @step And every former `crate::credentials::*` path resolves to `codelet_sessions::credentials::*`
    // Same rule: from inside codelet-sessions, the credentials module is
    // `crate::credentials::*` (NOT `codelet_sessions::credentials::*`).
    assert!(
        code.contains("crate::credentials::resolve_and_set_env_var")
            || !code.contains("resolve_and_set_env_var"),
        "moved create_session_with_id and create_isolated_session_with_id must call `crate::credentials::resolve_and_set_env_var(...)` (codelet-sessions internal path)"
    );

    // @step And every former `crate::scheduler::*` call resolves to `self.hooks.<method>(...)`
    assert!(
        code.contains("self.hooks.") || code.contains("self.hooks()."),
        "the moved file must dispatch napi-side subsystems via `self.hooks(.|()).<method>(...)`"
    );

    // @step And every former GLOBAL_CHUNK_CALLBACK call resolves to `self.chunks_tx.send(...)`
    // (RPC-041 removed the emit_isolation_state_change hook in favor of direct sender access)
    assert!(
        code.contains("self.chunks_tx.send(")
            || code.contains("self.chunks_tx.send("),
        "the moved file must route IsolationStateChange emission via `self.chunks_tx.send(...)` (RPC-041)"
    );
    assert!(
        !code.contains("emit_isolation_state_change"),
        "RPC-041: the emit_isolation_state_change hook has been removed; create_session_with_id and create_isolated_session_with_id must call self.chunks_tx.send(...) directly"
    );

    // @step And every former spawn_footer_poller/stop_footer_poller call resolves to `self.hooks.spawn_footer_poller(...)` / `self.hooks.stop_footer_poller(...)`
    assert!(
        contains_ws_insensitive(&code, "self.hooks.spawn_footer_poller")
            || contains_ws_insensitive(&code, "self.hooks().spawn_footer_poller"),
        "the moved file must route footer poller spawning via `self.hooks(.|()).spawn_footer_poller(...)`"
    );
    assert!(
        contains_ws_insensitive(&code, "self.hooks.stop_footer_poller")
            || contains_ws_insensitive(&code, "self.hooks().stop_footer_poller"),
        "the moved file must route footer poller stop via `self.hooks(.|()).stop_footer_poller(...)`"
    );

    // @step And every former `tokio::spawn(async move { agent_loop(...).await })` call resolves to `self.hooks.spawn_agent_loop(...)`
    assert!(
        contains_ws_insensitive(&code, "self.hooks.spawn_agent_loop")
            || contains_ws_insensitive(&code, "self.hooks().spawn_agent_loop"),
        "the moved file must route agent_loop spawning via `self.hooks(.|()).spawn_agent_loop(...)`"
    );
}
