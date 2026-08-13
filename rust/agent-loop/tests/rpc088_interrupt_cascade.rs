//! Feature: spec/features/agent-loop-interrupt-cascade.feature
//!
//! RPC-088 (RPC-072 family): interrupt cascade parity. The canonical
//! NAPI agent loop consults `session.is_interrupted` (an `AtomicBool`)
//! and selects against `session.interrupt_notify.notified()` inside
//! the stream loop so Esc aborts the active provider call and emits
//! `StreamChunk::Interrupted`.
//!
//! After the RPC-072/RPC-080/RPC-081/RPC-082 ports the implementation
//! already lives in the NAPI-free `codelet-agent-loop` crate and
//! `rust/cli/src/interactive/stream_loop.rs`. This test file pins
//! the contract via:
//!
//!   1. Structural source-string assertions over the
//!      `BackgroundSession` struct + `new` constructor in
//!      `rust/sessions/src/background_session.rs`.
//!   2. An async integration test against a real `BackgroundSession`
//!      that registers a `notified()` future BEFORE calling
//!      `interrupt()` and asserts the wake within 100ms.
//!   3. Structural assertions over the pre-turn `reset_interrupt()`
//!      call in `rust/agent-loop/src/agent_loop.rs`.
//!   4. Structural assertions over the `run_with_provider!` macro
//!      body in `rust/agent-loop/src/dispatch.rs` forwarding both
//!      `$session.is_interrupted.clone()` (positional arg 5) and
//!      `$session.interrupt_notify.clone()` (positional arg 7) to
//!      `run_agent_stream_with_images`.
//!   5. Structural assertions over the inlined `"openai" =>` arm and
//!      the `_ =>` custom-provider fallthrough in `agent_loop.rs`
//!      forwarding both handles.
//!   6. Structural assertions over the
//!      `StreamEvent::Interrupted(queued)` arm of
//!      `BackgroundOutput::emit` in `background_output.rs`.
//!   7. A census of `codelet_rpc_types::StreamChunk::Interrupted`
//!      variant + `interrupted` constructor.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

// ===========================================================================
// Helpers
// ===========================================================================

fn agent_loop_src(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file)
}

fn read_source(file: &str) -> String {
    let path = agent_loop_src(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn read_workspace_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = rust/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Extract a balanced-brace block beginning at `start_marker` inside
/// `src`. The returned slice includes the opening `{`, the matching
/// closing `}`, and everything in between (brace-balanced — sufficient
/// for Rust match arms and macro bodies which always use `{` for their
/// bodies).
fn extract_brace_block_after<'a>(src: &'a str, start_marker: &str) -> &'a str {
    let arm_start = src
        .find(start_marker)
        .unwrap_or_else(|| panic!("source must contain `{start_marker}`"));
    let bytes = src.as_bytes();
    let mut i = arm_start + start_marker.len();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    assert!(
        i < bytes.len() && bytes[i] == b'{',
        "expected `{{` after `{start_marker}`"
    );
    let body_start = i;
    let mut depth: i32 = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unterminated brace block starting at `{start_marker}`");
}

/// Extract the body of the `run_with_provider!` macro_rules! macro from
/// `dispatch.rs`. Returns the substring spanning the outer `{ ... }` of
/// the single matcher arm.
fn extract_run_with_provider_macro_body(src: &str) -> &str {
    let macro_start = src
        .find("macro_rules! run_with_provider")
        .expect("dispatch.rs must define `macro_rules! run_with_provider`");
    let tail = &src[macro_start..];
    let arrow = tail
        .find("=> {")
        .expect("run_with_provider! must have a single `=> {` arm");
    let body_start = macro_start + arrow + "=> ".len();
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    let mut started = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find matching `}}` for run_with_provider! macro body");
}

/// Extract the inlined `"openai" => { ... }` match arm body from
/// `agent_loop.rs`. The arm is inlined (not a macro expansion) because
/// `get_openai` requires `session.id` for cache-optimisation headers.
fn extract_openai_match_arm(src: &str) -> &str {
    let arm_start = src
        .find("\"openai\" => {")
        .expect("agent_loop.rs must have an inlined `\"openai\" => {` arm");
    let body_start = arm_start + "\"openai\" => ".len();
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    let mut started = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find matching `}}` for \"openai\" => arm body");
}

/// Extract the custom-provider fallthrough arm body — the `_ => { ... }`
/// inside the provider match in `agent_loop.rs`. Narrowed to the region
/// that contains `CustomProvider::create_rig_agent` so unrelated `_ =>`
/// branches elsewhere in the file do not match.
fn extract_custom_provider_fallthrough_arm(src: &str) -> &str {
    let create_call = src
        .find("CustomProvider::create_rig_agent")
        .expect("agent_loop.rs must call CustomProvider::create_rig_agent");
    let prefix = &src[..create_call];
    let arm_marker = prefix
        .rfind("_ => {")
        .expect("CustomProvider::create_rig_agent must live inside a `_ => {` arm");
    let body_start = arm_marker + "_ => ".len();
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    let mut started = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find matching `}}` for `_ =>` fallthrough arm body");
}

// ===========================================================================
// Scenario: BackgroundSession owns the AtomicBool + Notify interrupt handles
// ===========================================================================

#[test]
fn background_session_declares_interrupt_handle_fields() {
    // @step Given the source of `rust/sessions/src/background_session.rs`
    let src = read_workspace_source("rust/sessions/src/background_session.rs");

    // @step When I inspect the `BackgroundSession` struct
    assert!(
        src.contains("pub struct BackgroundSession"),
        "background_session.rs must declare `pub struct BackgroundSession`"
    );

    // @step Then it declares a `pub is_interrupted: Arc<AtomicBool>` field
    assert!(
        src.contains("pub is_interrupted: Arc<AtomicBool>"),
        "BackgroundSession must declare `pub is_interrupted: Arc<AtomicBool>` \
         field so Esc can flip a flag visible to the stream loop"
    );

    // @step And it declares a `pub interrupt_notify: Arc<Notify>` field
    assert!(
        src.contains("pub interrupt_notify: Arc<Notify>"),
        "BackgroundSession must declare `pub interrupt_notify: Arc<Notify>` \
         field so the stream loop can `select!` on `notified()`"
    );

    // @step And the `new` constructor initialises both fields with `Arc::new(AtomicBool::new(false))` and `Arc::new(Notify::new())`
    assert!(
        src.contains("is_interrupted: Arc::new(AtomicBool::new(false))"),
        "BackgroundSession::new must initialise `is_interrupted` with \
         `Arc::new(AtomicBool::new(false))`"
    );
    assert!(
        src.contains("interrupt_notify: Arc::new(Notify::new())"),
        "BackgroundSession::new must initialise `interrupt_notify` with \
         `Arc::new(Notify::new())`"
    );
}

// ===========================================================================
// Scenario: BackgroundSession::interrupt() flips the flag and wakes notifier
// ===========================================================================

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_session_interrupt_flips_flag_and_wakes_notifier() {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::Duration;

    use codelet_agent_loop::FspecAgentHooks;
    use codelet_sessions::session_manager::SessionManager;
    use uuid::Uuid;

    // Hermetic data dir + stub provider, mirroring the RPC-082/086 pattern.
    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());

    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp
        .path()
        .to_str()
        .expect("tempdir path is utf8")
        .to_string();
    let session_id_str = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &session_id_str,
            "stub/canned",
            &project,
            "interrupt-test-session",
        )
        .await
        .expect("create_session_with_id");

    // @step Given a `codelet_sessions::background_session::BackgroundSession` constructed via test helpers
    let session = manager
        .get_session(&session_id_str)
        .expect("session must exist after create_session_with_id");

    // Sanity: a fresh session reports is_interrupted=false.
    assert!(
        !session.is_interrupted.load(Ordering::Acquire),
        "fresh BackgroundSession must report is_interrupted=false"
    );

    // Register the notified() future BEFORE interrupt() so the
    // tokio::sync::Notify::notify_one wake-up has a registered
    // subscriber. (If we registered AFTER interrupt(), notify_one is
    // a no-op — see attachments/RPC-088/ast-research-interrupt-cascade.md.)
    let notify_handle = session.interrupt_notify.clone();
    let waiter = tokio::spawn(async move {
        notify_handle.notified().await;
    });
    // Yield so the spawned task can register its notified() future
    // before we call interrupt().
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    // @step When I call `session.interrupt()`
    session.interrupt();

    // @step Then `session.is_interrupted.load(Ordering::Acquire)` returns `true`
    assert!(
        session.is_interrupted.load(Ordering::Acquire),
        "session.interrupt() must store true into is_interrupted with \
         Release ordering"
    );

    // @step And a tokio task awaiting `session.interrupt_notify.notified()` (registered BEFORE interrupt was called) is woken within 100ms
    let wake_result = tokio::time::timeout(Duration::from_millis(100), waiter).await;
    assert!(
        wake_result.is_ok(),
        "tokio task awaiting interrupt_notify.notified() (registered before \
         interrupt) must be woken within 100ms; instead the timeout elapsed \
         which proves interrupt() did NOT call notify_one()"
    );
    wake_result
        .expect("timeout did not elapse (checked above)")
        .expect("notified() waiter task must not panic");

    // @step And calling `session.reset_interrupt()` flips `session.is_interrupted.load(Ordering::Acquire)` back to `false`
    session.reset_interrupt();
    assert!(
        !session.is_interrupted.load(Ordering::Acquire),
        "session.reset_interrupt() must store false into is_interrupted with \
         Release ordering"
    );
}

// ===========================================================================
// Scenario: Agent loop calls reset_interrupt() at the start of each turn
// ===========================================================================

#[test]
fn agent_loop_calls_reset_interrupt_immediately_after_set_status_running() {
    // @step Given the source of `rust/agent-loop/src/agent_loop.rs`
    let src = read_source("agent_loop.rs");

    // @step When I locate the pre-turn setup block
    // @step Then the body contains `session.reset_interrupt();` immediately after `session.set_status(SessionStatus::Running);`
    //
    // Pin the contract by asserting the two calls appear back-to-back
    // (allowing whitespace/newlines between them) at the top-level
    // turn entry. Mid-turn `set_status(SessionStatus::Running)` calls
    // exist (pause/HITL resume @ 492, 563) but those are NOT followed
    // by reset_interrupt — so we cannot just count occurrences.
    //
    // Anchor on `session.set_status(SessionStatus::Running);\n` —
    // i.e. the LITERAL pre-turn entry — and require the next
    // non-whitespace statement to be `session.reset_interrupt();`.
    let marker = "session.set_status(SessionStatus::Running);";
    let mut positions: Vec<usize> = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = src[search_from..].find(marker) {
        let abs = search_from + rel;
        positions.push(abs);
        search_from = abs + marker.len();
    }
    assert!(
        !positions.is_empty(),
        "agent_loop.rs must contain at least one \
         `session.set_status(SessionStatus::Running);` call"
    );

    // Find the occurrence followed immediately (skipping whitespace
    // and comment lines) by `session.reset_interrupt();`.
    let mut matched = false;
    for pos in &positions {
        let tail = &src[*pos + marker.len()..];
        // Skip whitespace-only lines; bail out if we hit a non-blank,
        // non-comment statement before finding reset_interrupt.
        let mut bytes = tail.as_bytes();
        let mut i = 0;
        // Skip whitespace.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Optional single-line comment lines.
        while i < bytes.len() && bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            // Skip until newline.
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            // Skip whitespace.
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            bytes = tail.as_bytes();
        }
        let remainder = &tail[i..];
        if remainder.starts_with("session.reset_interrupt();") {
            matched = true;
            break;
        }
    }
    assert!(
        matched,
        "at least one `session.set_status(SessionStatus::Running);` in \
         agent_loop.rs must be IMMEDIATELY followed by \
         `session.reset_interrupt();` — this is the pre-turn reset that \
         prevents a previous Esc from poisoning the next prompt"
    );
}

// ===========================================================================
// Scenario: run_with_provider! macro forwards both interrupt handles to
//           run_agent_stream_with_images
// ===========================================================================

#[test]
fn run_with_provider_macro_forwards_both_interrupt_handles() {
    // @step Given the source of `rust/agent-loop/src/dispatch.rs`
    let src = read_source("dispatch.rs");

    // @step When I locate the `run_with_provider!` macro body
    let body = extract_run_with_provider_macro_body(&src);

    // The macro body must invoke run_agent_stream_with_images.
    assert!(
        body.contains("codelet_cli::interactive::run_agent_stream_with_images("),
        "run_with_provider! macro body must invoke \
         `codelet_cli::interactive::run_agent_stream_with_images(`; \
         body was:\n{body}"
    );

    // Extract the call site substring — from the call identifier up to
    // and including the trailing `.await`. The call uses parentheses
    // (not braces), so we walk paren-depth rather than brace-depth.
    let call_start = body
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .expect("call site exists (checked above)");
    let call_tail = &body[call_start..];
    let open_paren = call_tail.find('(').expect("call site must contain `(`");
    let bytes = call_tail.as_bytes();
    let mut depth: i32 = 0;
    let mut i = open_paren;
    let mut end_paren: Option<usize> = None;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    end_paren = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let end_paren = end_paren.expect("call site must have matching `)`");
    let call_text = &call_tail[..=end_paren];

    // @step Then the body's call to `codelet_cli::interactive::run_agent_stream_with_images` passes `$session.is_interrupted.clone()` as positional arg 5
    assert!(
        call_text.contains("$session.is_interrupted.clone()"),
        "run_with_provider! macro must forward `$session.is_interrupted.clone()` \
         into `run_agent_stream_with_images`; call site was:\n{call_text}"
    );

    // @step And the call passes `$session.interrupt_notify.clone()` as positional arg 7
    assert!(
        call_text.contains("$session.interrupt_notify.clone()"),
        "run_with_provider! macro must forward `$session.interrupt_notify.clone()` \
         into `run_agent_stream_with_images`; call site was:\n{call_text}"
    );

    // Pin positional ordering: is_interrupted (arg 5) appears BEFORE
    // interrupt_notify (arg 7) in the call site, and both appear AFTER
    // the `$inner` arg (arg 4) and BEFORE the `$output` arg (arg 8).
    let inner_pos = call_text
        .find("$inner")
        .expect("call site must include `$inner` as positional arg 4");
    let is_interrupted_pos = call_text
        .find("$session.is_interrupted.clone()")
        .expect("call site must forward is_interrupted (checked above)");
    let compaction_pos = call_text
        .find("$session.compaction_in_progress.clone()")
        .expect("call site must include compaction_in_progress as positional arg 6");
    let interrupt_notify_pos = call_text
        .find("$session.interrupt_notify.clone()")
        .expect("call site must forward interrupt_notify (checked above)");
    let output_pos = call_text
        .find("$output")
        .expect("call site must include `$output` as positional arg 8");

    assert!(
        inner_pos < is_interrupted_pos,
        "`$inner` (arg 4) must precede `$session.is_interrupted.clone()` (arg 5)"
    );
    assert!(
        is_interrupted_pos < compaction_pos,
        "`$session.is_interrupted.clone()` (arg 5) must precede \
         `$session.compaction_in_progress.clone()` (arg 6)"
    );
    assert!(
        compaction_pos < interrupt_notify_pos,
        "`$session.compaction_in_progress.clone()` (arg 6) must precede \
         `$session.interrupt_notify.clone()` (arg 7)"
    );
    assert!(
        interrupt_notify_pos < output_pos,
        "`$session.interrupt_notify.clone()` (arg 7) must precede `$output` (arg 8)"
    );
}

// ===========================================================================
// Scenario: OpenAI inlined arm and custom-provider fallthrough forward both
//           interrupt handles
// ===========================================================================

/// Within an arm body, extract the substring spanning the
/// `run_agent_stream_with_images(...)` call — from the call identifier
/// through the closing `)` — by walking paren depth.
fn extract_run_agent_stream_call(arm: &str) -> &str {
    let call_start = arm
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .or_else(|| arm.find("run_agent_stream_with_images"))
        .expect("arm must invoke `run_agent_stream_with_images`");
    let tail = &arm[call_start..];
    let open_paren = tail.find('(').expect("call site must contain `(`");
    let bytes = tail.as_bytes();
    let mut depth: i32 = 0;
    let mut i = open_paren;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return &tail[..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("call site must have matching `)`");
}

#[test]
fn openai_inlined_arm_forwards_both_interrupt_handles() {
    // @step Given the source of `rust/agent-loop/src/agent_loop.rs`
    let src = read_source("agent_loop.rs");

    // @step When I locate the inlined `"openai" =>` match arm
    let arm = extract_openai_match_arm(&src);
    let call_text = extract_run_agent_stream_call(arm);

    // @step Then the body's `run_agent_stream_with_images` call passes `session.is_interrupted.clone()` and `session.interrupt_notify.clone()`
    assert!(
        call_text.contains("session.is_interrupted.clone()"),
        "OpenAI inlined arm must forward `session.is_interrupted.clone()` into \
         `run_agent_stream_with_images`; call site was:\n{call_text}"
    );
    assert!(
        call_text.contains("session.interrupt_notify.clone()"),
        "OpenAI inlined arm must forward `session.interrupt_notify.clone()` into \
         `run_agent_stream_with_images`; call site was:\n{call_text}"
    );

    // Pin positional ordering: is_interrupted before compaction before
    // interrupt_notify, matching the canonical 8-arg signature.
    let is_interrupted_pos = call_text
        .find("session.is_interrupted.clone()")
        .expect("call site must forward is_interrupted (checked above)");
    let compaction_pos = call_text
        .find("session.compaction_in_progress.clone()")
        .expect("call site must include compaction_in_progress");
    let interrupt_notify_pos = call_text
        .find("session.interrupt_notify.clone()")
        .expect("call site must forward interrupt_notify (checked above)");
    assert!(
        is_interrupted_pos < compaction_pos && compaction_pos < interrupt_notify_pos,
        "OpenAI inlined arm must forward args in canonical order: \
         is_interrupted (5), compaction_in_progress (6), interrupt_notify (7); \
         call site was:\n{call_text}"
    );
}

#[test]
fn custom_provider_fallthrough_arm_forwards_both_interrupt_handles() {
    // @step Given the source of `rust/agent-loop/src/agent_loop.rs`
    let src = read_source("agent_loop.rs");

    // @step When I locate the `_ =>` custom-provider fallthrough arm
    let arm = extract_custom_provider_fallthrough_arm(&src);
    let call_text = extract_run_agent_stream_call(arm);

    // @step Then the body's `run_agent_stream_with_images` call also passes `session.is_interrupted.clone()` and `session.interrupt_notify.clone()`
    assert!(
        call_text.contains("session.is_interrupted.clone()"),
        "Custom-provider fallthrough arm must forward \
         `session.is_interrupted.clone()` into `run_agent_stream_with_images`; \
         call site was:\n{call_text}"
    );
    assert!(
        call_text.contains("session.interrupt_notify.clone()"),
        "Custom-provider fallthrough arm must forward \
         `session.interrupt_notify.clone()` into `run_agent_stream_with_images`; \
         call site was:\n{call_text}"
    );

    let is_interrupted_pos = call_text
        .find("session.is_interrupted.clone()")
        .expect("call site must forward is_interrupted (checked above)");
    let compaction_pos = call_text
        .find("session.compaction_in_progress.clone()")
        .expect("call site must include compaction_in_progress");
    let interrupt_notify_pos = call_text
        .find("session.interrupt_notify.clone()")
        .expect("call site must forward interrupt_notify (checked above)");
    assert!(
        is_interrupted_pos < compaction_pos && compaction_pos < interrupt_notify_pos,
        "Custom-provider fallthrough arm must forward args in canonical order: \
         is_interrupted (5), compaction_in_progress (6), interrupt_notify (7); \
         call site was:\n{call_text}"
    );
}

// ===========================================================================
// Scenario: BackgroundOutput translates StreamEvent::Interrupted into
//           StreamChunk::interrupted
// ===========================================================================

#[test]
fn background_output_interrupted_arm_persists_and_emits_interrupted_chunk() {
    // @step Given the source of `rust/agent-loop/src/background_output.rs`
    let src = read_source("background_output.rs");

    // @step When I locate the `StreamEvent::Interrupted(queued)` arm of `BackgroundOutput::emit`
    let arm = extract_brace_block_after(&src, "StreamEvent::Interrupted(queued) =>");

    // @step Then the arm body calls `self.persist_assistant_message()`
    assert!(
        arm.contains("self.persist_assistant_message()"),
        "Interrupted arm must call `self.persist_assistant_message()` to \
         flush accumulated content on interrupt; arm was:\n{arm}"
    );

    // @step And the arm body returns `StreamChunk::interrupted(queued)`
    assert!(
        arm.contains("StreamChunk::interrupted(queued)"),
        "Interrupted arm must return `StreamChunk::interrupted(queued)`; \
         arm was:\n{arm}"
    );

    // Pin ordering: persist call MUST happen BEFORE the return chunk
    // is constructed, otherwise an interrupted stream would lose the
    // in-flight assistant content.
    let persist_pos = arm
        .find("self.persist_assistant_message()")
        .expect("persist call exists (checked above)");
    let chunk_pos = arm
        .find("StreamChunk::interrupted(queued)")
        .expect("interrupted chunk exists (checked above)");
    assert!(
        persist_pos < chunk_pos,
        "`self.persist_assistant_message()` must run BEFORE \
         `StreamChunk::interrupted(queued)` is returned; arm was:\n{arm}"
    );
}

// ===========================================================================
// Scenario: codelet_rpc_types::StreamChunk declares Interrupted variant +
//           constructor
// ===========================================================================

#[test]
fn stream_chunk_declares_interrupted_variant_and_constructor() {
    // @step Given the source of `rust/rpc-types/src/lib.rs`
    let src = read_workspace_source("rust/rpc-types/src/lib.rs");

    // @step When I inspect the `StreamChunk` enum
    assert!(
        src.contains("pub enum StreamChunk"),
        "rust/rpc-types/src/lib.rs must define `pub enum StreamChunk`"
    );

    // @step Then the enum declares a `Interrupted { queued_inputs: Vec<String> }` variant
    //
    // rpc-types/src/lib.rs is verbose-formatted with variant fields on
    // their own lines; locate the variant marker and check the field
    // line as a delimited substring.
    let variant_pos = src
        .find("    Interrupted {")
        .expect("StreamChunk must declare an `Interrupted {` variant on its own line");
    let variant_tail = &src[variant_pos..];
    let variant_end = variant_tail
        .find("    },")
        .expect("`Interrupted {` variant must terminate with `    },`");
    let variant = &variant_tail[..variant_end + 6];
    assert!(
        variant.contains("queued_inputs: Vec<String>"),
        "StreamChunk::Interrupted variant must carry \
         `queued_inputs: Vec<String>`; variant was:\n{variant}"
    );

    // @step And the impl block defines a `pub fn interrupted(queued_inputs: Vec<String>) -> Self` constructor returning `Self::Interrupted { queued_inputs }`
    assert!(
        src.contains("pub fn interrupted(queued_inputs: Vec<String>) -> Self"),
        "rpc-types must expose \
         `pub fn interrupted(queued_inputs: Vec<String>) -> Self`"
    );
    let ctor = extract_brace_block_after(
        &src,
        "pub fn interrupted(queued_inputs: Vec<String>) -> Self",
    );
    assert!(
        ctor.contains("Self::Interrupted { queued_inputs }"),
        "`interrupted` ctor must return `Self::Interrupted {{ queued_inputs }}`; \
         body was:\n{ctor}"
    );
}
