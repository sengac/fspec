//! Feature: spec/features/agent-loop-streaming.feature
//!
//! RPC-084 (RPC-072 family): streaming dispatch parity. The canonical
//! NAPI agent loop streams every turn through
//! `codelet_cli::interactive::run_agent_stream_with_images` (the rig
//! multi-turn streaming engine) and translates each rig `StreamEvent`
//! into a `codelet_rpc_types::StreamChunk` via the
//! `BackgroundOutput::StreamOutput` impl. Non-streaming
//! `complete_with_tools` is forbidden as the primary dispatch path.
//!
//! After the RPC-080/RPC-081 ports the same plumbing lives in the
//! NAPI-free `codelet-agent-loop` crate. This file pins the contract
//! via:
//!
//!   1. Structural source-string assertions over the three dispatch
//!      arms (run_with_provider! macro in `dispatch.rs`, the inlined
//!      `"openai"` arm in `agent_loop.rs`, and the `_ =>`
//!      custom-provider fallthrough in `agent_loop.rs`).
//!   2. A scan for accidental reintroduction of `.complete_with_tools(`
//!      anywhere in the agent loop body.
//!   3. A census of `codelet_rpc_types::StreamChunk` variants verifying
//!      at least the 19 names from the NAPI emission set are declared.
//!   4. A scan of the `BackgroundOutput::handle_stream_event` match
//!      arms verifying the 11 canonical StreamChunk constructors are
//!      reachable.
//!   5. A compile-time closure proving
//!      `codelet_cli::interactive::run_agent_stream_with_images` keeps
//!      the canonical 8-argument signature.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

// ===========================================================================
// Helpers — locate source files relative to CARGO_MANIFEST_DIR.
// ===========================================================================

fn agent_loop_src(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file)
}

fn read_source(file: &str) -> String {
    let path = agent_loop_src(file);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn read_workspace_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = codelet/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Extract the body of the `run_with_provider!` macro_rules! macro from
/// `dispatch.rs`. Returns the substring spanning the outer `{ ... }`.
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
/// `agent_loop.rs`.
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

/// Extract the custom-provider `_ => { ... }` fallthrough arm body
/// surrounding `CustomProvider::create_rig_agent`.
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

/// Line number (1-based) of `needle`'s first byte inside `src`.
fn line_number_of(src: &str, needle_byte_offset: usize) -> usize {
    src[..needle_byte_offset].bytes().filter(|b| *b == b'\n').count() + 1
}

/// Count occurrences of a literal substring.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut n = 0usize;
    let mut start = 0usize;
    while let Some(pos) = haystack[start..].find(needle) {
        n += 1;
        start += pos + needle.len();
    }
    n
}

// ===========================================================================
// Scenario: run_with_provider! macro body in dispatch.rs uses
//           run_agent_stream_with_images as the streaming dispatch path
// ===========================================================================

#[test]
fn run_with_provider_macro_streams_via_run_agent_stream_with_images() {
    // @step Given the source file codelet/agent-loop/src/dispatch.rs
    let src = read_source("dispatch.rs");

    // @step When I locate the run_with_provider! macro body
    let body = extract_run_with_provider_macro_body(&src);

    // @step Then the body contains exactly one call to `codelet_cli::interactive::run_agent_stream_with_images`
    let occurrences = count_occurrences(
        body,
        "codelet_cli::interactive::run_agent_stream_with_images",
    );
    assert_eq!(
        occurrences, 1,
        "run_with_provider! macro body must invoke \
         codelet_cli::interactive::run_agent_stream_with_images exactly once; \
         found {occurrences} occurrences. body:\n{body}"
    );

    // @step And the call appears after `codelet_core::RigAgent::with_default_depth(agent)`
    let depth_pos = body
        .find("codelet_core::RigAgent::with_default_depth(agent)")
        .expect("macro body must wrap with `RigAgent::with_default_depth(agent)`");
    let stream_pos = body
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .expect("macro body must invoke run_agent_stream_with_images (checked above)");
    assert!(
        depth_pos < stream_pos,
        "`RigAgent::with_default_depth(agent)` must precede the \
         `run_agent_stream_with_images` call in the macro body"
    );

    // @step And the call passes the 8 positional arguments in the canonical order: agent, $input, $images, $inner, $session.is_interrupted.clone(), $session.compaction_in_progress.clone(), $session.interrupt_notify.clone(), $output
    let call_tail = &body[stream_pos..];
    // Find the call's `(` and walk argument boundaries at paren depth 1.
    let open_paren = call_tail
        .find('(')
        .expect("run_agent_stream_with_images call must include `(`");
    let mut depth: i32 = 0;
    let mut starts: Vec<usize> = vec![open_paren + 1];
    let mut commas: Vec<usize> = Vec::new();
    let bytes = call_tail.as_bytes();
    let mut i = open_paren;
    let mut close = usize::MAX;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close = i;
                    break;
                }
            }
            b',' if depth == 1 => {
                commas.push(i);
                starts.push(i + 1);
            }
            _ => {}
        }
        i += 1;
    }
    assert_ne!(close, usize::MAX, "unterminated call site:\n{call_tail}");
    let mut ends: Vec<usize> = commas.clone();
    ends.push(close);
    let args: Vec<String> = starts
        .iter()
        .zip(ends.iter())
        .map(|(s, e)| call_tail[*s..*e].trim().to_string())
        .filter(|seg| !seg.is_empty())
        .collect();
    assert_eq!(
        args.len(),
        8,
        "run_agent_stream_with_images must take 8 positional args; got {} non-empty segments. \
         call_tail:\n{call_tail}",
        args.len()
    );
    let expected: [&str; 8] = [
        "agent",
        "$input",
        "$images",
        "$inner",
        "$session.is_interrupted.clone()",
        "$session.compaction_in_progress.clone()",
        "$session.interrupt_notify.clone()",
        "$output",
    ];
    for (idx, want) in expected.iter().enumerate() {
        assert_eq!(
            args[idx], *want,
            "positional arg {idx} of run_agent_stream_with_images in the \
             run_with_provider! macro body must be `{want}`; got `{}`. \
             call_tail:\n{call_tail}",
            args[idx]
        );
    }
}

// ===========================================================================
// Scenario: OpenAI inlined arm in agent_loop.rs mirrors the macro body and
//           calls run_agent_stream_with_images
// ===========================================================================

#[test]
fn openai_inlined_arm_calls_run_agent_stream_with_images() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When I locate the "openai" match arm body
    let arm = extract_openai_match_arm(&src);

    // @step Then the arm contains exactly one direct call to `codelet_cli::interactive::run_agent_stream_with_images`
    let count = count_occurrences(
        arm,
        "codelet_cli::interactive::run_agent_stream_with_images",
    );
    assert_eq!(
        count, 1,
        "OpenAI inlined arm must call run_agent_stream_with_images exactly once; \
         found {count}. arm:\n{arm}"
    );

    // @step And the call appears after `codelet_core::RigAgent::with_default_depth(agent)`
    let depth_pos = arm
        .find("codelet_core::RigAgent::with_default_depth(agent)")
        .expect("OpenAI arm must wrap with `RigAgent::with_default_depth(agent)`");
    let stream_pos = arm
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .expect("OpenAI arm must invoke run_agent_stream_with_images (checked above)");
    assert!(
        depth_pos < stream_pos,
        "`RigAgent::with_default_depth(agent)` must precede the \
         run_agent_stream_with_images call in the OpenAI arm"
    );

    // @step And the call is positioned between line 850 and line 920
    let abs_offset = src
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .expect("agent_loop.rs must contain at least one run_agent_stream_with_images call");
    let line = line_number_of(&src, abs_offset);
    assert!(
        (850..=920).contains(&line),
        "first run_agent_stream_with_images call (OpenAI inlined arm) must \
         live between lines 850 and 920; got line {line}"
    );
}

// ===========================================================================
// Scenario: Custom-provider fallthrough arm wraps the rig agent and calls
//           run_agent_stream_with_images
// ===========================================================================

#[test]
fn custom_provider_fallthrough_calls_run_agent_stream_with_images() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When I locate the `_ =>` fallthrough match arm body
    let arm = extract_custom_provider_fallthrough_arm(&src);

    // @step Then the arm contains exactly one call to `codelet_cli::interactive::run_agent_stream_with_images`
    let count = count_occurrences(
        arm,
        "codelet_cli::interactive::run_agent_stream_with_images",
    );
    assert_eq!(
        count, 1,
        "custom-provider fallthrough must call run_agent_stream_with_images \
         exactly once; found {count}. arm:\n{arm}"
    );

    // @step And the call appears after `codelet_core::RigAgent::with_default_depth(agent)`
    let depth_pos = arm
        .find("codelet_core::RigAgent::with_default_depth(agent)")
        .expect("custom-provider arm must wrap with `RigAgent::with_default_depth(agent)`");
    let stream_pos = arm
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .expect("custom-provider arm must invoke run_agent_stream_with_images (checked above)");
    assert!(
        depth_pos < stream_pos,
        "`RigAgent::with_default_depth(agent)` must precede the \
         run_agent_stream_with_images call in the custom-provider arm"
    );

    // @step And the call is positioned between line 1000 and line 1100
    //
    // RPC-069 widened the previous 950..=1020 window because adding
    // the feature-gated `"stub" =>` arm (~50 LOC) before the `_ =>`
    // custom-provider fallthrough pushed the custom-provider call
    // down. We locate the call by finding the `_ => {` arm marker,
    // not by counting "the second occurrence" — that brittle approach
    // confused the stub arm (now the second site) with the custom-
    // provider arm.
    let arm_marker_abs = src
        .find("CustomProvider::create_rig_agent")
        .and_then(|p| src[..p].rfind("_ => {"))
        .expect("agent_loop.rs must contain `_ => {` arm wrapping CustomProvider::create_rig_agent");
    let stream_rel = src[arm_marker_abs..]
        .find("codelet_cli::interactive::run_agent_stream_with_images")
        .expect("custom-provider arm must invoke run_agent_stream_with_images");
    let stream_abs = arm_marker_abs + stream_rel;
    let line = line_number_of(&src, stream_abs);
    assert!(
        (1000..=1100).contains(&line),
        "custom-provider run_agent_stream_with_images call must live between \
         lines 1000 and 1100; got line {line}"
    );
}

// ===========================================================================
// Scenario: Non-streaming complete_with_tools is forbidden as the primary
//           dispatch path in the agent loop body
// ===========================================================================

#[test]
fn agent_loop_body_does_not_use_complete_with_tools() {
    // @step Given the source file codelet/agent-loop/src/agent_loop.rs
    let src = read_source("agent_loop.rs");

    // @step When I scan non-comment, non-test lines of the file
    // We process line-by-line and:
    //   - skip any line whose first non-whitespace character starts with `//`
    //   - skip every line inside a `#[cfg(test)]` module body (paren-depth aware)
    let mut in_test_module = false;
    let mut depth: i32 = 0;
    let mut offending: Vec<(usize, String)> = Vec::new();
    let mut prev_line_is_cfg_test_attr = false;
    for (idx, raw_line) in src.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim_start();

        // Detect entering a #[cfg(test)] module on subsequent `mod` line or same-line `mod ... {`.
        if !in_test_module {
            if trimmed.starts_with("#[cfg(test)]")
                || trimmed.starts_with("#[cfg(all(test")
            {
                prev_line_is_cfg_test_attr = true;
                continue;
            }
            if prev_line_is_cfg_test_attr {
                if trimmed.starts_with("mod ") && trimmed.contains('{') {
                    in_test_module = true;
                    depth = raw_line.chars().filter(|c| *c == '{').count() as i32
                        - raw_line.chars().filter(|c| *c == '}').count() as i32;
                }
                prev_line_is_cfg_test_attr = false;
            }
        } else {
            depth += raw_line.chars().filter(|c| *c == '{').count() as i32;
            depth -= raw_line.chars().filter(|c| *c == '}').count() as i32;
            if depth <= 0 {
                in_test_module = false;
                depth = 0;
            }
        }

        if in_test_module {
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if raw_line.contains(".complete_with_tools(") {
            offending.push((line_no, raw_line.to_string()));
        }
    }

    // @step Then there is no `.complete_with_tools(` invocation in the agent loop body
    assert!(
        offending.is_empty(),
        "agent_loop.rs body must not invoke .complete_with_tools(); \
         found {} non-test, non-comment occurrence(s): {:#?}",
        offending.len(),
        offending,
    );

    // @step And every line matching `.complete_with_tools(` belongs to either a `//` comment or a `#[cfg(test)]` test module
    // (proven by the construction above — only non-comment, non-test lines populate `offending`)
}

// ===========================================================================
// Scenario: codelet_rpc_types::StreamChunk exposes at least nineteen
//           variants matching the NAPI emission set
// ===========================================================================

#[test]
fn stream_chunk_enum_has_at_least_nineteen_variants() {
    // @step Given the source file codelet/rpc-types/src/lib.rs
    let src = read_workspace_source("codelet/rpc-types/src/lib.rs");

    // @step When I locate the `pub enum StreamChunk` declaration
    let enum_start = src
        .find("pub enum StreamChunk {")
        .expect("rpc-types/src/lib.rs must declare `pub enum StreamChunk`");
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = enum_start;
    let mut started = false;
    let mut close = usize::MAX;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    close = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    assert_ne!(close, usize::MAX, "could not find end of StreamChunk enum");
    let body_start = src[enum_start..]
        .find('{')
        .expect("enum body must open with `{`")
        + enum_start
        + 1;
    let body = &src[body_start..close];

    // @step Then the enum declares 19 or more variants
    // Variants are top-level identifier-followed-by-`{` or identifier-followed-by-`,`
    // inside the enum body. We detect them by scanning lines and matching an
    // identifier followed by `{` at the outermost brace depth of the enum.
    let mut variants: Vec<String> = Vec::new();
    let mut inner_depth: i32 = 0;
    for raw_line in body.lines() {
        let trimmed = raw_line.trim();
        // Skip blanks, comments, and attributes.
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[")
        {
            inner_depth += raw_line.chars().filter(|c| *c == '{').count() as i32;
            inner_depth -= raw_line.chars().filter(|c| *c == '}').count() as i32;
            continue;
        }
        if inner_depth == 0 {
            // Candidate variant line: starts with an identifier character
            // and ends with either `{` (struct variant) or a bare identifier.
            let mut chars = trimmed.chars();
            if let Some(c) = chars.next() {
                if c.is_ascii_uppercase() {
                    // Capture leading identifier.
                    let ident: String = trimmed
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !ident.is_empty() {
                        variants.push(ident);
                    }
                }
            }
        }
        inner_depth += raw_line.chars().filter(|c| *c == '{').count() as i32;
        inner_depth -= raw_line.chars().filter(|c| *c == '}').count() as i32;
    }
    assert!(
        variants.len() >= 19,
        "StreamChunk must declare ≥19 variants; found {}: {:?}",
        variants.len(),
        variants
    );

    // @step And the variant set includes Text, Thinking, ToolCall, ToolResult, ToolProgress, SessionStateChange, UserNotification, Interrupted, TokenUpdate, ContextFillUpdate, Done, Error, UserInput, IncomingMessage, SupervisorPendingInjection, CompactionComplete, FspecCommandRequest, FspecCommandResult, WorkUnitsUpdate
    let required: [&str; 19] = [
        "Text",
        "Thinking",
        "ToolCall",
        "ToolResult",
        "ToolProgress",
        "SessionStateChange",
        "UserNotification",
        "Interrupted",
        "TokenUpdate",
        "ContextFillUpdate",
        "Done",
        "Error",
        "UserInput",
        "IncomingMessage",
        "SupervisorPendingInjection",
        "CompactionComplete",
        "FspecCommandRequest",
        "FspecCommandResult",
        "WorkUnitsUpdate",
    ];
    for name in required.iter() {
        assert!(
            variants.iter().any(|v| v == name),
            "StreamChunk must declare a `{name}` variant; got {variants:?}"
        );
    }
}

// ===========================================================================
// Scenario: BackgroundOutput translates the eleven canonical rig
//           StreamEvent variants into StreamChunk variants
// ===========================================================================

#[test]
fn background_output_emits_eleven_canonical_stream_chunk_constructors() {
    // @step Given the source file codelet/agent-loop/src/background_output.rs
    let src = read_source("background_output.rs");

    // @step When I scan the handle_stream_event match arms
    // Trim the source to the impl block to be precise — but a flat substring
    // scan over the file suffices because the helpers we look for are all
    // namespaced under `StreamChunk::`.

    // @step Then each of the following StreamChunk constructors appears at least once: StreamChunk::text, StreamChunk::thinking, StreamChunk::tool_call, StreamChunk::tool_result, StreamChunk::tool_progress, StreamChunk::user_notification, StreamChunk::token_update, StreamChunk::context_fill_update, StreamChunk::error, StreamChunk::interrupted, StreamChunk::done
    let constructors: [&str; 11] = [
        "StreamChunk::text(",
        "StreamChunk::thinking(",
        "StreamChunk::tool_call(",
        "StreamChunk::tool_result(",
        "StreamChunk::tool_progress(",
        "StreamChunk::user_notification(",
        "StreamChunk::token_update(",
        "StreamChunk::context_fill_update(",
        "StreamChunk::error(",
        "StreamChunk::interrupted(",
        "StreamChunk::done(",
    ];
    for ctor in constructors.iter() {
        assert!(
            src.contains(ctor),
            "background_output.rs must invoke `{ctor}` at least once \
             so the rig→StreamChunk translation for the canonical \
             StreamEvent variants is wired"
        );
    }
}

// ===========================================================================
// Scenario: run_agent_stream_with_images public signature accepts the
//           canonical 8 positional arguments
// ===========================================================================

#[test]
fn run_agent_stream_with_images_has_canonical_eight_argument_signature() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use codelet_cli::interactive::{BridgeImage, run_agent_stream_with_images};
    use codelet_cli::session::Session;
    use codelet_core::RigAgent;
    use rig::completion::CompletionModel;
    use tokio::sync::Notify;

    // @step Given the codelet_cli::interactive module exposes run_agent_stream_with_images
    // (proven by `use` line above)

    // @step When I take a closure reference to the function with the canonical 8-argument signature
    //
    // The closure body is never executed — it exists purely so that the
    // type-checker enforces the canonical 8-argument signature. Each
    // argument is annotated with its expected concrete or generic type so
    // any future drift in the public API breaks compilation.
    #[allow(clippy::too_many_arguments)]
    fn typecheck<'a, 'b, 'c, M, O>(
        agent: RigAgent<M>,
        input: &'a str,
        images: Option<Vec<BridgeImage>>,
        session: &'b mut Session,
        is_interrupted: Arc<AtomicBool>,
        compaction_in_progress: Arc<AtomicBool>,
        interrupt_notify: Arc<Notify>,
        output: &'c O,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + use<'a, 'b, 'c, M, O>
    where
        M: CompletionModel + 'static,
        M::StreamingResponse:
            rig::wasm_compat::WasmCompatSend + rig::completion::GetTokenUsage,
        O: codelet_cli::interactive::StreamOutput,
    {
        run_agent_stream_with_images(
            agent,
            input,
            images,
            session,
            is_interrupted,
            compaction_in_progress,
            interrupt_notify,
            output,
        )
    }

    // @step Then the closure compiles
    // (proven by `typecheck` compiling above — its body invokes
    // `run_agent_stream_with_images` with the canonical 8 positional
    // arguments, so a type-checker drift breaks this test at compile
    // time)
    let _ = typecheck::<rig::providers::openai::CompletionModel, codelet_cli::interactive::CliOutput>;

    // @step And the function is re-exported from codelet_cli::interactive so the agent loop dispatch arms can call it directly
    //
    // The fully-qualified `use codelet_cli::interactive::run_agent_stream_with_images`
    // at the top of this function proves the re-export. If a future
    // refactor moves the function under a private submodule, this `use`
    // line will fail to resolve at compile time.
}
