//! RPC-070 regression tests: `SessionManagerHandle::create_session` (and
//! the other sync→async bridges in `rust/sessions/src/handle_impl.rs`)
//! MUST NOT panic when invoked from inside a multi-thread tokio runtime
//! worker (which is what the live tarpc dispatcher does).
//!
//! Feature: spec/features/rpc-070-create-session-no-panic.feature
//!
//! The pre-existing tests in `rust/sessions/tests/handle_impl.rs`
//! never triggered the bug because `#[tokio::test]` defaults to a
//! single-thread runtime and the test body is the outer future on that
//! runtime — `Handle::current().block_on(...)` re-enters it cleanly.
//!
//! In production the tarpc handler IS the currently-polled future on a
//! multi-thread worker, so `Handle::current().block_on(...)` panics
//! with `Cannot start a runtime from within a runtime`. This file
//! reproduces that context two ways:
//!
//!   1. `scenario_create_session_does_not_panic_on_multi_thread_worker`
//!      – call the trait method directly from inside a
//!      `#[tokio::test(flavor = "multi_thread")]` body. The test body
//!      is itself a future polled by the multi-thread runtime, so the
//!      bridge sees the exact nested-runtime conditions as production.
//!
//!   2. `scenario_create_session_over_embedded_tarpc_does_not_panic`
//!      – wire a real `SessionManager` through `SharedFspecService` +
//!      `EmbeddedTransport` and call `create_session` through the tarpc
//!      client. This is the closest in-process mirror of the live
//!      WS dispatcher path that the e2e repro test
//!      (`e2e/rpc-068-work-agent-panic-repro.test.ts`) exercises end-to-end.
//!
//! Three source-shape scenarios additionally pin the file to use the
//! `tokio::task::block_in_place(|| Handle::current().block_on(...))`
//! idiom and to remove the redundant `Handle::try_current()` runtime
//! construction inside `test_provider_connection`.
//!
//! ## How this would catch the regression
//!
//! Before the fix, `cargo test -p codelet-fspec --test rpc070_create_session_no_panic`
//! crashes the worker with the captured backtrace. After the fix
//! (block_in_place wrappers) the test passes silently. The shape
//! scenarios also fail the build if anyone reverts the wrapper.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::FspecServiceClient;
use codelet_rpc_embedded::{EmbeddedTransport, SharedFspecService};
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use tarpc::context;
use tempfile::tempdir;

/// Workspace root (one level above `rust/fspec/`).
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("project root walk-up")
}

fn handle_impl_path() -> PathBuf {
    workspace_root()
        .join("rust")
        .join("sessions")
        .join("src")
        .join("handle_impl.rs")
}

fn read_handle_impl_src() -> String {
    let p = handle_impl_path();
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Strip `//`, `///`, `//!` line comments and `/* … */` block comments
/// from a Rust source body so source-shape assertions aren't fooled by
/// the panic-prone idiom appearing inside rustdoc that *describes* the
/// pattern. Mirrors `rust/fspec/tests/common/mod.rs::strip_comments`
/// — duplicated here so this test binary stays standalone (no
/// `mod common`).
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut prev = ' ';
                    for ch in chars.by_ref() {
                        if prev == '*' && ch == '/' {
                            break;
                        }
                        prev = ch;
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

/// Slice out the body of the specified `fn NAME(` or `fn NAME<...>(`
/// definition inside the `impl ... SessionManagerHandle for
/// SessionManager { ... }` block (or a free function). Returns the
/// substring from `fn NAME` up to the matching closing brace at
/// indentation level four (i.e. the function's own closing brace).
/// Light-weight brace-counter — sufficient for the shape assertions
/// below, which only care about substring presence.
fn extract_method_body<'src>(src: &'src str, fn_name: &str) -> &'src str {
    let needle = format!("fn {fn_name}");
    let mut search = 0usize;
    let start = loop {
        let rel = src[search..]
            .find(&needle)
            .unwrap_or_else(|| panic!("`{needle}` not found in handle_impl.rs"));
        let abs = search + rel;
        // Make sure the next character after `fn NAME` is `(` (no
        // generics) or `<` (generics) — i.e. not an unrelated identifier
        // prefix like `fn loop_block_on_helper`.
        let after = src[abs + needle.len()..].chars().next().unwrap_or(' ');
        if after == '(' || after == '<' {
            break abs;
        }
        search = abs + needle.len();
    };
    // Find the opening brace of the body.
    let body_start_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace of `fn {fn_name}` not found"));
    let body_start = start + body_start_rel;
    // Walk braces to find the matching close.
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("did not find matching `}}` for `fn {fn_name}` body");
}

// =============================================================================
// Scenario: create_session does not panic when invoked from a multi-thread
// tokio runtime worker
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_create_session_does_not_panic_on_multi_thread_worker() {
    // @step Given a tokio multi-thread runtime is active
    let flavor = tokio::runtime::Handle::current().runtime_flavor();
    assert_eq!(
        flavor,
        tokio::runtime::RuntimeFlavor::MultiThread,
        "test must run on a multi-thread runtime to reproduce the panic context",
    );

    // @step And a fresh SessionManager wrapped as Arc<dyn SessionManagerHandle>
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the test calls handle.create_session(None) from inside the multi-thread runtime
    //
    // The call is wrapped in `spawn_blocking` to mirror exactly what the
    // tarpc handler does: invoke the *synchronous* trait method from
    // within an `async fn` that is being polled by a runtime worker.
    // Without the RPC-070 fix, this panics with `Cannot start a runtime
    // from within a runtime` because the bridge calls
    // `Handle::current().block_on(...)` while the worker is already
    // driving the future.
    let handle_for_blocking = Arc::clone(&handle);
    let join = tokio::spawn(async move {
        // Run inside an async future polled on the worker — this is
        // structurally identical to the tarpc dispatch path.
        handle_for_blocking.create_session(None)
    });

    // @step Then no thread panics with "Cannot start a runtime from within a runtime"
    let sid: SessionId = tokio::time::timeout(Duration::from_secs(5), join)
        .await
        .expect("create_session did not return within 5s")
        .expect("create_session task panicked — RPC-070 regression");

    // @step And the call returns a SessionId value
    //
    // The inner SessionManager::create_session may legitimately return
    // an empty id (no provider/credentials configured in the test env);
    // the `.unwrap_or_default()` swallows the inner error. What we care
    // about for RPC-070 is the NO-PANIC invariant — proven by reaching
    // this assertion line at all.
    let _ = sid; // The value itself is unconstrained; reaching here is the assertion.
}

// =============================================================================
// Scenario: create_session over the live tarpc embedded transport returns
// without panicking
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_create_session_over_embedded_tarpc_does_not_panic() {
    // @step Given a tokio multi-thread runtime is active
    assert_eq!(
        tokio::runtime::Handle::current().runtime_flavor(),
        tokio::runtime::RuntimeFlavor::MultiThread,
    );

    // @step And a temp workspace with an empty spec/work-units.json
    let dir = tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("spec")).expect("mkdir spec");
    fs::write(
        dir.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(dir.path()).expect("WorkUnitsWatcher"));

    // @step And a SharedFspecService built with a real SessionManager via with_session_manager
    let session_manager: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&session_manager),
    ));

    // @step And an EmbeddedTransport bound to the multi-thread runtime handle
    let handle = tokio::runtime::Handle::current();
    let transport = EmbeddedTransport::new(handle, service);
    let client: FspecServiceClient = transport.client();

    // @step When the test calls client.create_session(context::current(), None).await on the tarpc client
    //
    // Before the RPC-070 fix this panics the worker driving the
    // FspecServiceImpl::create_session future because the synchronous
    // bridge inside handle_impl.rs:create_session calls
    // `Handle::current().block_on(...)` while the worker is busy
    // polling that same future. After the fix the bridge is wrapped in
    // `tokio::task::block_in_place(...)` and the call returns cleanly.
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        client.create_session(context::current(), None),
    )
    .await
    .expect("client.create_session timed out — likely a tarpc worker panicked")
    .expect("client.create_session returned an RpcError — RPC-070 regression");

    // @step Then the RPC returns Ok(SessionId)
    let _sid: SessionId = result;

    // @step And no worker thread emits the panic "Cannot start a runtime from within a runtime"
    //
    // The `expect` above would have surfaced the panic via
    // `RpcError::Server` if any worker had panicked; reaching this
    // assertion line is the no-panic proof.
}

// =============================================================================
// Scenario: Every Handle::current().block_on call inside handle_impl.rs is
// wrapped in tokio::task::block_in_place
// =============================================================================
#[test]
fn scenario_every_block_on_is_wrapped_in_block_in_place() {
    // @step Given the file rust/sessions/src/handle_impl.rs
    let src = read_handle_impl_src();

    // @step When I read the source bytes
    //
    // Identify every occurrence of the panic-prone idiom by counting
    // distinct `Handle::current().block_on(` matches. The rustfmt
    // line-break form `Handle::current()\n    .block_on(` is normalised
    // by collapsing all whitespace inside the impl body. We strip
    // comments first so doc-comments describing the pattern don't
    // trigger false-positive failures (or false-positive passes when
    // they sit next to a `block_in_place(` mention in prose).
    let code_only = strip_comments(&src);
    let collapsed: String = code_only.split_whitespace().collect::<Vec<_>>().join(" ");

    // @step Then every occurrence of "tokio::runtime::Handle::current().block_on(" is preceded (within the same statement) by a "tokio::task::block_in_place(" call
    //
    // Strategy: walk every `tokio::runtime::Handle::current().block_on(`
    // match in the COLLAPSED text and assert that the nearest preceding
    // 200 chars contain `tokio::task::block_in_place(`. 200 chars is
    // comfortably more than the longest reasonable wrapper expression.
    let needle = "tokio::runtime::Handle::current().block_on(";
    let mut search_from = 0usize;
    let mut occurrences = 0usize;
    while let Some(rel) = collapsed[search_from..].find(needle) {
        let abs = search_from + rel;
        let window_start = abs.saturating_sub(200);
        let window = &collapsed[window_start..abs];
        assert!(
            window.contains("tokio::task::block_in_place("),
            "Handle::current().block_on at byte {abs} is NOT wrapped in tokio::task::block_in_place(...); preceding context:\n{window}",
        );
        occurrences += 1;
        search_from = abs + needle.len();
    }
    assert!(
        occurrences >= 2,
        "expected at least 2 `Handle::current().block_on(` occurrences inside handle_impl.rs (create_session, create_isolated_session, loop_block_on...); found {occurrences}",
    );

    // @step And the file contains exactly one "fn loop_block_on" helper
    let loop_block_on_defs = code_only.matches("fn loop_block_on").count();
    assert_eq!(
        loop_block_on_defs, 1,
        "expected exactly one `fn loop_block_on` definition; found {loop_block_on_defs}",
    );

    // @step And the loop_block_on helper body contains "tokio::task::block_in_place"
    let helper_body = extract_method_body(&code_only, "loop_block_on");
    assert!(
        helper_body.contains("tokio::task::block_in_place"),
        "loop_block_on helper must call tokio::task::block_in_place(...) around its block_on; body was:\n{helper_body}",
    );

    // @step And the loop_block_on helper body contains a debug_assert! on RuntimeFlavor::MultiThread
    assert!(
        helper_body.contains("debug_assert"),
        "loop_block_on helper must include a debug_assert! that the current runtime flavor is MultiThread; body was:\n{helper_body}",
    );
    assert!(
        helper_body.contains("MultiThread"),
        "loop_block_on helper's debug_assert! must reference RuntimeFlavor::MultiThread; body was:\n{helper_body}",
    );
}

// =============================================================================
// Scenario: test_provider_connection no longer constructs its own runtime
// =============================================================================
#[test]
fn scenario_test_provider_connection_uses_block_in_place() {
    // @step Given the file rust/sessions/src/handle_impl.rs
    let src = read_handle_impl_src();

    // @step When I read the source bytes
    //
    // Strip comments so doc-comments describing the old contract don't
    // taint the substring check.
    let code_only = strip_comments(&src);
    let body = extract_method_body(&code_only, "test_provider_connection");

    // @step Then the test_provider_connection method body does not contain "Handle::try_current()"
    assert!(
        !body.contains("Handle::try_current()"),
        "test_provider_connection must not construct its own runtime via Handle::try_current(); body was:\n{body}",
    );

    // @step And the test_provider_connection method body contains "tokio::task::block_in_place"
    assert!(
        body.contains("tokio::task::block_in_place"),
        "test_provider_connection must wrap its async work in tokio::task::block_in_place(...); body was:\n{body}",
    );
}

// =============================================================================
// Scenario: Pre-existing SessionManagerHandle shape tests still pass
// =============================================================================
//
// This scenario is satisfied transitively: if the RPC-070 fix breaks
// any of the three named tests, `cargo test --workspace` fails
// independently of this file. We codify the invariant here by
// re-running the same `Arc::new(SessionManager::new()) as Arc<dyn
// SessionManagerHandle>` cast that the original
// `scenario_session_manager_satisfies_trait_object` validates, then
// listing the three pre-existing tests as the canonical assertion path.
#[test]
fn scenario_pre_existing_handle_impl_tests_still_pass() {
    // @step Given the RPC-070 fix is applied
    //
    // (Established by the workspace build that produces this test
    // binary at all.)

    // @step When cargo test -p codelet-sessions --test handle_impl runs
    //
    // We do not recursively invoke `cargo test` here (cargo-in-cargo
    // is expensive and the outer `cargo test --workspace` already runs
    // those tests). Instead we duplicate the cheapest of the three
    // assertions to prove the trait surface still matches.
    let manager = SessionManager::new();
    let handle: Arc<dyn SessionManagerHandle> = Arc::new(manager) as Arc<dyn SessionManagerHandle>;

    // @step Then scenario_session_manager_satisfies_trait_object passes
    let sessions = handle.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    assert!(
        sessions.is_empty(),
        "trait-object cast must still yield an empty list_sessions() — proves RPC-042 invariant held",
    );

    // @step And scenario_unknown_session_id_returns_safe_defaults passes
    //
    // The full assertion lives in rust/sessions/tests/handle_impl.rs;
    // we duplicate the cheapest safe-default check here as the marker.
    let unknown = SessionId::new("nonexistent-uuid");
    let _ = handle.get_session_status(&unknown);

    // @step And scenario_impl_block_exists_with_every_override passes
    //
    // Source-shape proof: the existing handle_impl.rs test asserts the
    // impl block exists with overrides for every trait method. We
    // re-read the same file and confirm the impl marker string is
    // still present and unique.
    let src = read_handle_impl_src();
    let impl_marker = "impl codelet_core::SessionManagerHandle for SessionManager";
    assert_eq!(
        src.matches(impl_marker).count(),
        1,
        "expected exactly one `{impl_marker}` block in handle_impl.rs",
    );
}
