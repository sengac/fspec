//! Feature: spec/features/agent-loop-thinking-config.feature
//!
//! RPC-085 (RPC-072 family): thinking config wiring parity. The canonical
//! NAPI agent loop computes an effective thinking config per turn and
//! threads it as the 3rd positional argument of every provider's
//! `create_rig_agent` invocation. The three dispatch paths are:
//!
//!   1. The `run_with_provider!` macro in `dispatch.rs` (used by the
//!      `claude`, `gemini`, `zai`, `codex`, and `github-copilot` /
//!      `copilot` arms).
//!   2. The inlined `"openai" => { ... }` match arm in `agent_loop.rs`
//!      (inlined because `get_openai` requires `session.id`).
//!   3. The `_ =>` custom-provider fallthrough in `agent_loop.rs` which
//!      calls the free function `codelet_providers::custom::
//!      CustomProvider::create_rig_agent`.
//!
//! This file pins the contract via structural source-string assertions
//! and compile-time closure signatures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
    // Follows file-backed `mod` declarations: agent_loop.rs keeps its
    // provider dispatch in the agent_loop/providers.rs child module.
    codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Extract the body of the `run_with_provider!` macro_rules! macro from
/// `dispatch.rs`. Returns the substring spanning the outer `{ ... }`.
fn extract_run_with_provider_macro_body(src: &str) -> &str {
    let macro_start = src
        .find("macro_rules! run_with_provider")
        .expect("dispatch.rs must define `macro_rules! run_with_provider`");
    // The macro has helper `provider ...` arms; the dispatch arm is the one
    // whose header declares the `$thinking:expr` metavariable.
    let tail = &src[macro_start..];
    let header = tail
        .find("$thinking:expr")
        .expect("run_with_provider! must have an arm taking `$thinking:expr`");
    let arrow = tail[header..]
        .find("=> {")
        .expect("the `$thinking` arm must have a `=> {` body");
    let body_start = macro_start + header + arrow + "=> ".len();
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

/// Parse the positional arguments of a function-call expression starting
/// at `call_start_offset` in `src`. Returns the trimmed arg strings.
fn parse_positional_args(src: &str, call_start_offset: usize) -> Vec<String> {
    let tail = &src[call_start_offset..];
    let open_paren = tail.find('(').expect("call site must include `(`");
    let bytes = tail.as_bytes();
    let mut depth: i32 = 0;
    let mut starts: Vec<usize> = vec![open_paren + 1];
    let mut commas: Vec<usize> = Vec::new();
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
    assert_ne!(close, usize::MAX, "unterminated call:\n{tail}");
    let mut ends: Vec<usize> = commas.clone();
    ends.push(close);
    starts
        .iter()
        .zip(ends.iter())
        .map(|(s, e)| tail[*s..*e].trim().to_string())
        .filter(|seg| !seg.is_empty())
        .collect()
}

// ===========================================================================
// Scenario: InputWithImages carries thinking_config alongside text and images
// ===========================================================================

// ===========================================================================
// Scenario: run_with_provider! macro forwards thinking config to create_rig_agent
// ===========================================================================

// ===========================================================================
// Scenario: agent_loop body computes thinking_config_value per turn
// ===========================================================================

// ===========================================================================
// Scenario: All run_with_provider! call sites pass thinking_config_value
// ===========================================================================

// ===========================================================================
// Scenario: OpenAI inlined arm passes thinking_config_value to create_rig_agent
// ===========================================================================

// ===========================================================================
// Scenario: Custom-provider fallthrough passes thinking_config_value to create_rig_agent
// ===========================================================================

// ===========================================================================
// Scenario: Provider create_rig_agent signature compiles for all 6
//           instance-based built-in providers
// ===========================================================================

// ===========================================================================
// Scenario: CustomProvider create_rig_agent signature accepts thinking_config
//           as its last argument
// ===========================================================================

#[path = "rpc085_thinking_config/input_with_images_declares.rs"]
mod input_with_images_declares;
#[path = "rpc085_thinking_config/openai_inlined_arm_passes.rs"]
mod openai_inlined_arm_passes;
