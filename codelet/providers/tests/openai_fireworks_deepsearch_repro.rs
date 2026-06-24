//! Reproduction tests for DeepSearch "Failed to get tool definitions" error
//! when using Qwen (or any model) via OpenAI-compatible Fireworks API.
//!
//! Previously reported as: `API Error: Streaming error: RequestError: Failed to get tool definitions`
//!

#![allow(
    clippy::panic,
    clippy::doc_overindented_list_items,
    clippy::needless_borrows_for_generic_args
)]
//! These tests connect to the REAL Fireworks API using an inline API key fixture
//! and progressively break down the DeepSearch code path to localize where the
//! failure happens. The original agent (qwen) hit this in the TUI when calling
//! DeepSearch — it surfaced via rig-core's `completion.rs` where the
//! `map_err(|_| ...)` swallows the underlying tool-server error into the
//! generic "Failed to get tool definitions" string.
//!
//! Strategy (layered, each layer strictly adds one thing over the previous):
//!
//!   1. raw_http         — prove Fireworks itself works with chat completions
//!                         at the exact URL we will configure in rig.
//!   2. rig_client       — prove rig's patched OpenAI `CompletionsClient` can
//!                         POST /chat/completions against Fireworks (no tools).
//!   3. agent_no_tools   — prove a rig `Agent` built via OpenAIProvider works
//!                         end-to-end for a plain prompt (no tool server).
//!   4. agent_one_tool   — single trivial tool, assert the model is prompted
//!                         to call it and the multi-turn loop finishes.
//!   5. agent_many_tools — reproduce the exact 7-tool sub-agent DeepSearch
//!                         builds (Read, Grep, AstGrep, Glob, Ls, Bash,
//!                         SessionSearch) and force a tool call.
//!   6. agent_streaming  — same as 5 but using `.stream_prompt().multi_turn()`
//!                         (the path the TUI actually uses).
//!
//! Ignored by default (require real network + API key). Run with:
//!
//!   OPENAI_API_KEY=fw_... OPENAI_BASE_URL=https://api.fireworks.ai/inference \
//!     cargo test -p codelet-providers --test openai_fireworks_deepsearch_repro \
//!     -- --ignored --nocapture --test-threads=1
//!
//! The env-var fallback defaults below are the values the user's TUI session
//! used (from ~/.fspec/fspec-config.json "fireworks" profile).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    dead_code
)]

use codelet_providers::OpenAIProvider;
use rig::client::CompletionClient;
use rig::completion::{Prompt, ToolDefinition};
use rig::streaming::StreamingPrompt;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Fireworks fixture values — mirror the `fireworks` profile in
/// ~/.fspec/fspec-config.json. Overridable via env for CI.
fn fireworks_api_key() -> String {
    // Fixture-first: use the `fireworks` profile key from fspec-config.json.
    // Override via FIREWORKS_API_KEY only (NOT OPENAI_API_KEY — which in this
    // shell currently points at a localhost vLLM dummy key).
    std::env::var("FIREWORKS_API_KEY").unwrap_or_else(|_| "fw_UVPY68joXb6wmAs4csPabr".to_string())
}

fn fireworks_base_url_raw() -> String {
    // This is what ends up in fspec-config.json. OpenAIProvider's
    // `normalize_base_url` appends `/v1` if not present.
    std::env::var("FIREWORKS_BASE_URL")
        .unwrap_or_else(|_| "https://api.fireworks.ai/inference".to_string())
}

fn fireworks_model() -> String {
    std::env::var("FIREWORKS_MODEL")
        .unwrap_or_else(|_| "accounts/fireworks/models/qwen3p6-plus".to_string())
}

// =============================================================================
// Layer 1: raw HTTP — Fireworks API shape validation, no rig involvement.
// =============================================================================
//
// This proves the URL `<base>/v1/chat/completions` + Bearer auth actually
// returns 200. If this fails, the whole stack is moot and the tests below
// will fail for a reason unrelated to rig/codelet.

#[tokio::test]
#[ignore = "hits real Fireworks API — run with --ignored"]
async fn layer1_raw_http_fireworks_responds_to_chat_completions() {
    let url = format!("{}/v1/chat/completions", fireworks_base_url_raw());
    let body = serde_json::json!({
        "model": fireworks_model(),
        "messages": [{"role": "user", "content": "say hi in three words"}],
        "max_tokens": 20,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(fireworks_api_key())
        .json(&body)
        .send()
        .await
        .expect("HTTP request succeeds");

    let status = resp.status();
    let text = resp.text().await.expect("body readable");

    println!("[layer1] POST {url}");
    println!("[layer1] status = {status}");
    println!(
        "[layer1] body (first 400): {}",
        text.chars().take(400).collect::<String>()
    );

    assert!(
        status.is_success(),
        "Fireworks returned non-2xx status {status}: {text}"
    );
    assert!(
        text.contains("\"choices\""),
        "Response missing choices array: {text}"
    );
}

// =============================================================================
// Layer 2: rig OpenAI CompletionsClient — confirm rig's URL construction
// against Fireworks using the same base_url normalization the OpenAIProvider
// applies in production. Builds a completion model directly with no agent,
// no tools, no tool server.
// =============================================================================

#[tokio::test]
#[ignore = "hits real Fireworks API — run with --ignored"]
async fn layer2_rig_client_completion_against_fireworks() {
    let provider = OpenAIProvider::from_api_key_with_options(
        &fireworks_api_key(),
        &fireworks_model(),
        Some(&format!(
            "{}/v1",
            fireworks_base_url_raw().trim_end_matches('/')
        )),
        None,
    )
    .expect("OpenAIProvider constructs");

    // Build a bare agent with no tools — this exercises the rig_client
    // URL construction but never touches the tool server.
    let agent = provider
        .client()
        .agent(&fireworks_model())
        .max_tokens(64)
        .preamble("You are a terse assistant. Answer in at most 5 words.")
        .build();

    let out = agent
        .prompt("What is 2 plus 2?")
        .await
        .expect("rig prompt succeeds against Fireworks");

    println!("[layer2] reply = {out:?}");
    assert!(!out.trim().is_empty(), "expected non-empty reply");
}

// =============================================================================
// Shared test tools — minimal rig::tool::Tool impls with schemars parameters.
// Used by layers 4–6 to exercise the tool server, get_tool_defs(), and the
// multi-turn loop.
// =============================================================================

#[derive(Debug, thiserror::Error)]
#[error("ReproTool error: {0}")]
struct ReproToolError(String);

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct AddArgs {
    /// First addend
    x: i64,
    /// Second addend
    y: i64,
}

#[derive(Default)]
struct AddTool;

impl Tool for AddTool {
    const NAME: &'static str = "add";
    type Error = ReproToolError;
    type Args = AddArgs;
    type Output = i64;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description:
                "Add two integers together. ALWAYS use this tool when asked to add numbers."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "First addend" },
                    "y": { "type": "integer", "description": "Second addend" }
                },
                "required": ["x", "y"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.x + args.y)
    }
}

/// Seven no-op tools that mimic (by name only) the tool set DeepSearch builds
/// for its sub-agent: Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch.
/// The bodies are stubs — we are reproducing *tool-definition* serialization
/// and multi-turn dispatch, not the real tool behavior.

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct StubArgs {
    /// Free-form argument string. Present so the tool has a schema.
    query: String,
}

fn stub_definition(name: &str, description: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "query" }
            },
            "required": ["query"]
        }),
    }
}

#[derive(Default)]
struct StubRead;
#[derive(Default)]
struct StubGrep;
#[derive(Default)]
struct StubAstGrep;
#[derive(Default)]
struct StubGlob;
#[derive(Default)]
struct StubLs;
#[derive(Default)]
struct StubBash;
#[derive(Default)]
struct StubSessionSearch;

macro_rules! impl_stub_tool {
    ($t:ty, $name:literal, $descr:literal) => {
        impl Tool for $t {
            const NAME: &'static str = $name;
            type Error = ReproToolError;
            type Args = StubArgs;
            type Output = String;
            async fn definition(&self, _p: String) -> ToolDefinition {
                stub_definition($name, $descr)
            }
            async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
                Ok(format!("[stub {}] query={}", $name, args.query))
            }
        }
    };
}

impl_stub_tool!(StubRead, "Read", "Read a file from disk");
impl_stub_tool!(StubGrep, "Grep", "Search file contents with ripgrep");
impl_stub_tool!(StubAstGrep, "AstGrep", "AST-based code search");
impl_stub_tool!(StubGlob, "Glob", "Filename pattern matching");
impl_stub_tool!(StubLs, "Ls", "List directory contents");
impl_stub_tool!(StubBash, "Bash", "Execute a bash command");
impl_stub_tool!(StubSessionSearch, "SessionSearch", "Search session history");

// =============================================================================
// Layer 3: OpenAIProvider + bare rig Agent (no tools).
// This path uses OpenAIProvider::from_api_key_with_options but still builds
// the agent via provider.client().agent() — same call shape as
// deep_search_handler.rs build_and_run!.
// =============================================================================

#[tokio::test]
#[ignore = "hits real Fireworks API — run with --ignored"]
async fn layer3_provider_bare_agent_no_tools() {
    let provider = OpenAIProvider::from_api_key_with_options(
        &fireworks_api_key(),
        &fireworks_model(),
        Some(&format!(
            "{}/v1",
            fireworks_base_url_raw().trim_end_matches('/')
        )),
        None,
    )
    .expect("OpenAIProvider constructs");

    let agent = provider
        .client()
        .agent(&fireworks_model())
        .preamble("Answer with exactly: pong")
        .max_tokens(32)
        .build();

    let out = agent.prompt("ping").await.expect("prompt should succeed");
    println!("[layer3] reply = {out:?}");
    assert!(!out.trim().is_empty());
}

// =============================================================================
// Layer 4: One trivial tool. This is the smallest possible exercise of the
// tool server + get_tool_defs() + multi-turn loop.
// =============================================================================

#[tokio::test]
#[ignore = "hits real Fireworks API — run with --ignored"]
async fn layer4_provider_agent_one_tool_forced_call() {
    let provider = OpenAIProvider::from_api_key_with_options(
        &fireworks_api_key(),
        &fireworks_model(),
        Some(&format!(
            "{}/v1",
            fireworks_base_url_raw().trim_end_matches('/')
        )),
        None,
    )
    .expect("OpenAIProvider constructs");

    let agent = provider
        .client()
        .agent(&fireworks_model())
        .preamble(
            "You are an arithmetic assistant. You MUST call the `add` tool to \
             compute sums — never compute them yourself. After the tool returns, \
             state the final numeric answer.",
        )
        .max_tokens(256)
        .tool(AddTool)
        .build();

    // Sanity check: the tool server can enumerate the tool definitions. This
    // is the exact code path that reports "Failed to get tool definitions"
    // (rig-core/src/agent/completion.rs:163/177 swallows the real error).
    let defs = agent
        .tool_server_handle
        .get_tool_defs(None)
        .await
        .expect("get_tool_defs(None) works for a freshly-built agent");
    println!("[layer4] tool def count = {}", defs.len());
    assert_eq!(defs.len(), 1, "expected exactly the `add` tool");
    assert_eq!(defs[0].name, "add");

    // Real call. Multi-turn depth of 5 is plenty for a single tool call.
    let reply = agent
        .prompt("What is 137 plus 246? Use the add tool.")
        .multi_turn(5)
        .await
        .expect("multi-turn prompt succeeds");

    println!("[layer4] reply = {reply:?}");
    assert!(
        reply.contains("383"),
        "expected the tool-assisted answer 383 in reply: {reply}"
    );
}

// =============================================================================
// Layer 5: Seven stub tools named exactly like DeepSearch's sub-agent tools.
// Forces a tool call and exercises `get_tool_defs` through the full multi-turn
// path.
// =============================================================================

#[tokio::test]
#[ignore = "hits real Fireworks API — run with --ignored"]
async fn layer5_provider_agent_seven_tools_forced_call() {
    let provider = OpenAIProvider::from_api_key_with_options(
        &fireworks_api_key(),
        &fireworks_model(),
        Some(&format!(
            "{}/v1",
            fireworks_base_url_raw().trim_end_matches('/')
        )),
        None,
    )
    .expect("OpenAIProvider constructs");

    let agent = provider
        .client()
        .agent(&fireworks_model())
        .preamble(
            "You are a research assistant. You have file-system tools \
             (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch). \
             When the user asks you to LOOK SOMETHING UP, you MUST call \
             exactly one of these tools. Do not answer from memory.",
        )
        .max_tokens(512)
        .tool(StubRead)
        .tool(StubGrep)
        .tool(StubAstGrep)
        .tool(StubGlob)
        .tool(StubLs)
        .tool(StubBash)
        .tool(StubSessionSearch)
        .build();

    // Sanity: exactly 7 tool defs available via the tool server channel.
    let defs = agent
        .tool_server_handle
        .get_tool_defs(None)
        .await
        .expect("get_tool_defs(None) works with 7 tools");
    println!("[layer5] tool def count = {}", defs.len());
    assert_eq!(defs.len(), 7, "expected exactly 7 stub tools");

    let reply = agent
        .prompt("Please look up the contents of /etc/hostname using the Read tool. Then summarize.")
        .multi_turn(5)
        .await;

    match reply {
        Ok(text) => {
            println!("[layer5] reply = {text:?}");
            assert!(!text.is_empty(), "empty reply from multi-turn loop");
        }
        Err(e) => {
            // This is the interesting failure. If we land here, we've likely
            // reproduced the TUI bug. Print everything so the error chain
            // (which normally gets masked by map_err(|_|)) is visible.
            panic!(
                "[layer5] multi-turn prompt FAILED — \
                 full error chain: {e:?}\n\
                 display: {e}"
            );
        }
    }
}

// =============================================================================
// Layer 6: Streaming multi-turn — the path the TUI actually uses.
// This is the one that surfaces "Streaming error: RequestError: Failed to
// get tool definitions" in production.
// =============================================================================

#[tokio::test]
#[ignore = "hits real Fireworks API — run with --ignored"]
async fn layer6_provider_agent_streaming_seven_tools_multi_turn() {
    use futures::StreamExt;
    use rig::agent::MultiTurnStreamItem;

    let provider = OpenAIProvider::from_api_key_with_options(
        &fireworks_api_key(),
        &fireworks_model(),
        Some(&format!(
            "{}/v1",
            fireworks_base_url_raw().trim_end_matches('/')
        )),
        None,
    )
    .expect("OpenAIProvider constructs");

    let agent = provider
        .client()
        .agent(&fireworks_model())
        .preamble(
            "You are a research assistant. You have file-system tools \
             (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch). \
             When the user asks you to LOOK SOMETHING UP, you MUST call \
             exactly one of these tools. Do not answer from memory.",
        )
        .max_tokens(512)
        .tool(StubRead)
        .tool(StubGrep)
        .tool(StubAstGrep)
        .tool(StubGlob)
        .tool(StubLs)
        .tool(StubBash)
        .tool(StubSessionSearch)
        .build();

    let mut stream = agent
        .stream_prompt("Please read /etc/hostname using the Read tool and tell me what's there.")
        .multi_turn(5)
        .await;

    let mut final_text: Option<String> = None;
    let mut tool_calls_seen = 0usize;
    let mut errors_seen: Vec<String> = vec![];

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::FinalResponse(fr)) => {
                final_text = Some(fr.response().to_string());
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(inner)) => {
                if let rig::streaming::StreamedAssistantContent::ToolCall(tc) = inner {
                    tool_calls_seen += 1;
                    println!(
                        "[layer6] tool_call: {}({})",
                        tc.function.name, tc.function.arguments
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                let es = format!("{e}");
                println!("[layer6] stream error: {es}");
                errors_seen.push(es);
            }
        }
    }

    println!(
        "[layer6] tool_calls_seen = {tool_calls_seen}, \
         errors = {:?}, final = {:?}",
        errors_seen, final_text
    );

    assert!(
        errors_seen.is_empty(),
        "Streaming multi-turn against Fireworks produced errors (this is the bug): {:#?}",
        errors_seen
    );
    assert!(final_text.is_some(), "no final response from stream");
}

// =============================================================================
// Layer 7: Isolated reproduction of the root cause (FIXED)
//
// Originally this test asserted that rig-core's OpenAI Completions adapter
// PANICS when an assistant message in chat history contains
// `AssistantContent::Reasoning`. That panic — at completion/mod.rs ~line 665 —
// was the "last line of defense" (PROV-081) and it made any multi-turn
// conversation against Fireworks' Qwen3 models crash on turn 2, because
// Qwen always emits `reasoning_content` which the inbound parser captures
// into chat history.
//
// The fix: the outbound conversion now silently DROPS reasoning (and image)
// assistant-content blocks instead of panicking. Reasoning remains
// inbound-only — the explicit `reasoning: None` on the outbound
// `Message::Assistant` plus `skip_serializing_if` still guarantees the
// outbound wire never carries a `reasoning` / `reasoning_content` key.
//
// This layer is synchronous and requires NO network.
// =============================================================================

#[test]
fn layer7_outbound_conversion_drops_reasoning_instead_of_panicking() {
    use rig::message::AssistantContent;
    use rig::providers::openai::completion::Message as OpenAiWireMessage;
    use rig::OneOrMany;

    // Build an assistant content block that looks like what Qwen returned on
    // turn 1 (reasoning text + a text answer). This is structurally what ends
    // up in chat_history.
    let items = OneOrMany::many(vec![
        AssistantContent::reasoning("Here's a thinking process: the user asked to add 2 and 2."),
        AssistantContent::text("The answer is 4."),
    ])
    .expect("two items");

    // This is the exact call rig makes on turn 2 before sending the next
    // request. Previously panicked; now returns cleanly.
    let converted: Vec<OpenAiWireMessage> = items.try_into().expect("conversion should not panic");

    assert_eq!(converted.len(), 1, "one assistant message");
    match &converted[0] {
        OpenAiWireMessage::Assistant {
            content,
            tool_calls,
            reasoning,
            ..
        } => {
            assert!(
                reasoning.is_none(),
                "outbound reasoning field must be None (never sent over wire)"
            );
            assert!(tool_calls.is_empty(), "no tool calls in this fixture");
            // The visible text content should be preserved; the reasoning
            // block should have been silently dropped.
            let joined = content
                .iter()
                .map(|c| format!("{c:?}"))
                .collect::<Vec<_>>()
                .join(" | ");
            assert!(
                joined.contains("The answer is 4."),
                "text content preserved; got: {joined}"
            );
            assert!(
                !joined.contains("thinking process"),
                "reasoning content must NOT appear in outbound message; got: {joined}"
            );
        }
        other => panic!("expected Assistant variant, got {other:?}"),
    }
}
