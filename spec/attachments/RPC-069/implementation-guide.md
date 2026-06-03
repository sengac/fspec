# RPC-069 Implementation Guide

> **Card scope**: Route `ProviderType::Custom("stub")` through the in-memory `LlmProvider` registry in the post-RPC-072 agent loop, so the four `#[ignore]`'d cross-frontend parity tests in `codelet/fspec/tests/cross_frontend_parity.rs` (lines 58, 139, 369, 646) flip green.

> **Originally written** assuming dispatch lived in `codelet/providers/src/manager.rs`. **After RPC-072**, dispatch moved into `codelet/agent-loop/src/agent_loop.rs` + `codelet/agent-loop/src/dispatch.rs`. This guide is anchored to the **current** code shape.

---

## TL;DR — Four mechanical edits + fixture record

| # | File | What to add | Lines |
|---|------|-------------|-------|
| 1 | `codelet/agent-loop/src/agent_loop.rs` | New `"stub" =>` arm in the provider `match` at line 880 | ~15 |
| 2 | `codelet/agent-loop/src/dispatch.rs` | Add `"stub"` to the `matches!` list in `agent_loop_dispatch_supports_provider` (line 122) | 1 |
| 3 | `codelet/fspec/Cargo.toml` | (Already wired ✅ — verify only) | 0 |
| 4 | `codelet/fspec/tests/cross_frontend_parity.rs` | Remove the four `#[ignore]` markers (lines 58, 139, 369, 646) | -4 |
| 5 | `codelet/fspec/tests/fixtures/cross_frontend_run.jsonl` | Record via `FSPEC_RPC_066_REGENERATE=1` after edits 1+2 pass | n/a |

Net diff: ~16 added LOC + 1 generated fixture file.

---

## What already works (verified by DeepSearch against HEAD)

✅ `codelet/providers/src/stub_provider.rs` — the `StubProvider` struct, `impl LlmProvider for StubProvider`, and the in-memory registry (`STUB_REGISTRY: OnceLock<RwLock<HashMap<...>>>`) all exist. Helpers shipped:

- `pub fn register_stub_provider()` — line 131 (idempotent via `std::sync::Once`)
- `pub fn is_stub_registered(slug: &str) -> bool` — line 145
- `pub fn get_stub_provider(slug: &str) -> Option<Arc<dyn LlmProvider>>` — line 156
- Entire file gated by `test-support` feature on `codelet-providers`

✅ `codelet/providers/src/manager.rs:131-146` — `custom_provider_registered("stub")` consults `is_stub_registered` under `#[cfg(feature = "test-support")]`, so `ProviderType::from_str("stub")` resolves to `Ok(Custom("stub"))` at session-creation time.

✅ `codelet/fspec/src/common.rs:122-126` — under `#[cfg(feature = "test-stub-provider")]`, `build_service` calls `register_stub_provider()` then `manager.set_default_model("stub/canned")`. So `WebSocketFspecBackend::create_session(None)` already returns a session pinned to the stub provider when the feature is on.

✅ `codelet/fspec/Cargo.toml:93-109` — `test-stub-provider` feature already propagates to `codelet-agent-loop/test-support`:

```toml
test-stub-provider = [
    "dep:codelet-providers",
    "codelet-providers/test-support",
    "codelet-agent-loop/test-support",
]
```

✅ `codelet/agent-loop/Cargo.toml:50-67` — `test-support = ["codelet-providers/test-support"]`. The Cargo plumbing is complete; only the code arm is missing.

## What breaks today (the gap RPC-069 closes)

In `codelet/agent-loop/src/agent_loop.rs:880`, the provider match has arms for `claude | openai | gemini | zai | codex | github-copilot | copilot`. `"stub"` falls through to the `_` arm at line 966 which calls `codelet_providers::custom::CustomProvider::create_rig_agent(...)`. That function scans `~/.fspec/providers/*.json` on disk for a `stub.json` config — none exists, so it returns `Err`, and the agent loop emits `Err(anyhow!("Unsupported provider: stub"))` at line 1027.

Result: `send_input("hello")` on a stub-backed session times out at 5s with **zero chunks emitted** — no `Text`, no `Done`.

---

## Edit 1 — Add the `"stub"` arm in `agent_loop.rs:880`

**Where**: Insert immediately after the `"copilot"` arm (line 965, just before the `_ =>` Custom-provider fallback at line 966).

**Pattern**: The other arms use the `run_with_provider!` macro (`dispatch.rs:36-106`) which expects a *getter on `ProviderManager`* (e.g. `get_claude`, `get_openai`). The stub doesn't go through `ProviderManager` at all — it lives in its own `OnceLock` registry. So we **cannot** reuse `run_with_provider!` verbatim; we must inline the analogous body.

**The arm shape** (model after the `_ =>` Custom-provider block at lines 966-1033, but lookup via `get_stub_provider` instead of `CustomProvider::create_rig_agent`):

```rust
                // RPC-069: Custom("stub") dispatch — route to the in-memory
                // stub provider registered by `register_stub_provider()` in
                // common.rs::build_service under the `test-stub-provider`
                // feature. The stub yields the canned [Text("hi back"), Done]
                // stream via its LlmProvider impl with no network egress.
                //
                // Gated by `test-support` so production builds compile this
                // arm out entirely — same gate that controls whether
                // `codelet_providers::stub_provider` is even available.
                #[cfg(feature = "test-support")]
                "stub" => {
                    match codelet_providers::stub_provider::get_stub_provider(&current_provider) {
                        Some(provider) => {
                            tracing::debug!(
                                "[run_with_provider] Creating stub agent - session={}, provider={}",
                                session.id,
                                current_provider,
                            );
                            let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
                            let role_preamble = session.get_role();
                            let agent = provider.create_rig_agent(
                                session.id,
                                role_preamble.as_deref(),
                                thinking_config_value.clone(),
                            );
                            if !mcp_wrappers.is_empty() {
                                for wrapper in mcp_wrappers {
                                    if let Err(e) = agent.tool_server_handle.add_tool(wrapper).await {
                                        tracing::warn!("[MCP] Failed to add MCP tool: {}", e);
                                    }
                                }
                            }
                            codelet_tools::set_mcp_tool_server_handle(
                                session.id,
                                agent.tool_server_handle.clone(),
                            );
                            let agent = codelet_core::RigAgent::with_default_depth(agent);
                            codelet_cli::interactive::run_agent_stream_with_images(
                                agent,
                                input,
                                bridge_images.clone(),
                                &mut inner_session,
                                session.is_interrupted.clone(),
                                session.compaction_in_progress.clone(),
                                session.interrupt_notify.clone(),
                                &output,
                            )
                            .await
                        }
                        None => {
                            tracing::error!(
                                "Stub provider '{}' not in in-memory registry — was register_stub_provider() called?",
                                current_provider,
                            );
                            Err(anyhow::anyhow!(
                                "Stub provider '{}' not registered",
                                current_provider
                            ))
                        }
                    }
                }
```

### ⚠️ Open question: does `StubProvider::create_rig_agent` exist?

`LlmProvider` is the trait that `StubProvider` already implements (per `stub_provider.rs:45-101`). But `create_rig_agent` is **NOT** part of the `LlmProvider` trait — it's a per-provider inherent method (e.g. on `ClaudeProvider`, `OpenAIProvider`, `CustomProvider`). Each concrete provider implements its own `create_rig_agent`.

**Two paths to resolve this**:

**Path A — Add `create_rig_agent` to `StubProvider`** (recommended for parity with other providers):
- Add an inherent `impl StubProvider { pub fn create_rig_agent(...) -> Agent<StubModel> { ... } }`
- Requires a thin `StubModel` impl of `rig::completion::CompletionModel` that returns canned `Text("hi back") + Done`
- This puts the stub on the same dispatch shape as real providers — no special-casing in `run_agent_stream_with_images`

**Path B — Bypass `run_agent_stream_with_images` and emit chunks directly**:
- The stub arm becomes much simpler: just call `session.handle_output(StreamChunk::text("hi back"))` then `session.handle_output(StreamChunk::done())` and return `Ok(())`
- No `rig::Agent` involvement at all
- Faster to land, but the stub no longer exercises the same `RigAgent::with_default_depth` → `run_agent_stream_with_images` codepath that real providers use
- **Risk**: `scenario_scripted_run_matches_golden` (line 369) drives `set_thinking_level High` + `compact_session` + `interrupt` between turns. If these flow through `run_agent_stream_with_images`-level state but the stub arm bypasses it, the captured chunk stream may diverge from what TS Ink + a real provider would emit.

> **Recommendation**: Spike Path A first (1-2 hours). If `StubModel` adapter is more than ~50 LOC, fall back to Path B and accept that `scenario_scripted_run_matches_golden`'s golden fixture is "Rust-pinned baseline" (already acknowledged in RPC-066 rule [10]). See `BackgroundSession::handle_output` at `codelet/sessions/src/background_session.rs:775` for the direct-emit signature.

---

## Edit 2 — Add `"stub"` to `agent_loop_dispatch_supports_provider`

**Where**: `codelet/agent-loop/src/dispatch.rs:118-124`

**Current**:
```rust
#[must_use]
pub fn agent_loop_dispatch_supports_provider(provider_name: &str) -> bool {
    matches!(
        provider_name,
        "claude" | "openai" | "gemini" | "zai" | "codex" | "github-copilot" | "copilot"
    )
}
```

**After**:
```rust
#[must_use]
pub fn agent_loop_dispatch_supports_provider(provider_name: &str) -> bool {
    #[cfg(feature = "test-support")]
    {
        if provider_name == "stub" {
            return true;
        }
    }
    matches!(
        provider_name,
        "claude" | "openai" | "gemini" | "zai" | "codex" | "github-copilot" | "copilot"
    )
}
```

The doc comment at lines 112-117 already mandates this lock-step: *"If you add an arm to the match, add the same provider name here."* — this edit honours the contract.

---

## Edit 3 — Cargo plumbing (already wired ✅)

Verify by running:

```bash
cargo metadata --no-deps --format-version 1 | jq -r '
  .packages[] | select(.name == "codelet-agent-loop") |
  .features
'
```

Should show `"test-support": ["codelet-providers/test-support"]`. Then:

```bash
cargo metadata --no-deps --format-version 1 | jq -r '
  .packages[] | select(.name == "codelet-fspec") |
  .features
'
```

Should show `"test-stub-provider"` propagating to both `codelet-providers/test-support` and `codelet-agent-loop/test-support`.

✅ Both verified by DeepSearch against HEAD. No edits needed unless the propagation chain regressed.

---

## Edit 4 — Remove the four `#[ignore]` markers

**File**: `codelet/fspec/tests/cross_frontend_parity.rs`

| Line | Test | Reason string to remove |
|------|------|-------------------------|
| 58 | `scenario_fspec_daemon_boots_and_emits_a_port` | `"RPC-066: requires fspec binary built with --features test-stub-provider; spawns the CLI binary against a real workspace"` |
| 139 | `scenario_send_input_hello_yields_canned_stream` | `"RPC-066: requires fspec binary built with --features test-stub-provider"` |
| 369 | `scenario_scripted_run_matches_golden` | `"RPC-066: requires fspec binary built with --features test-stub-provider; full agent-loop integration"` |
| 646 | `scenario_deny_network_egress_still_yields_canned_chunks` | `"RPC-066: requires fspec binary built with --features test-stub-provider; subprocess spawn"` |

The tests already use `#[cfg_attr(not(feature = "test-stub-provider"), ignore = "...")]` patterns or unconditional `#[ignore]` — confirm and remove. Then they only need the feature flag to run.

---

## Edit 5 — Record the golden fixture

After edits 1-4 pass `cargo build`:

```bash
FSPEC_RPC_066_REGENERATE=1 cargo test \
  -p codelet-fspec \
  --features test-stub-provider \
  --test cross_frontend_parity \
  scenario_scripted_run_matches_golden -- --nocapture
```

This writes `codelet/fspec/tests/fixtures/cross_frontend_run.jsonl` and skips the assertion. Then run again without the env var to confirm the byte-comparison passes.

Commit the fixture as the regression baseline (rule [10] in RPC-066 — "Rust-pinned golden file").

---

## Acceptance bar — running order

```bash
# Step 1: Build with feature on
cargo build -p codelet-fspec --features test-stub-provider

# Step 2: Sanity — predicate should now return true for "stub"
cargo test -p codelet-agent-loop --features test-support \
  agent_loop_dispatch_supports_provider -- --nocapture

# Step 3: The simplest end-to-end test should pass
cargo test -p codelet-fspec --features test-stub-provider \
  --test cross_frontend_parity \
  scenario_send_input_hello_yields_canned_stream -- --nocapture

# Step 4: The daemon spawn test
cargo test -p codelet-fspec --features test-stub-provider \
  --test cross_frontend_parity \
  scenario_fspec_daemon_boots_and_emits_a_port -- --nocapture

# Step 5: Record the golden fixture
FSPEC_RPC_066_REGENERATE=1 cargo test \
  -p codelet-fspec --features test-stub-provider \
  --test cross_frontend_parity \
  scenario_scripted_run_matches_golden -- --nocapture

# Step 6: Re-run scripted to verify golden matches
cargo test -p codelet-fspec --features test-stub-provider \
  --test cross_frontend_parity \
  scenario_scripted_run_matches_golden -- --nocapture

# Step 7: Network-deny test
cargo test -p codelet-fspec --features test-stub-provider \
  --test cross_frontend_parity \
  scenario_deny_network_egress_still_yields_canned_chunks -- --nocapture

# Step 8: Full suite green
cargo test -p codelet-fspec --features test-stub-provider \
  --test cross_frontend_parity
```

All four previously-ignored tests should pass + the seven other source-shape tests should still pass.

---

## Out of scope (deferred to sibling cards)

- **`noop_tool` registration** — RPC-066 architecture notes [B] and [L] called for a real `MessageContent::ToolUse` return shape from `StubProvider::complete_with_tools` + a registered `noop_tool` in `codelet_tools`. The stub currently returns `MessageContent::Text("hi back")` for the `trigger-tool` step too (stub_provider.rs:95-99), which is enough for `scenario_scripted_run_matches_golden` to pass against the Rust-pinned golden. A proper ToolUse exercise stays deferred — explicitly documented at `cross_frontend_parity.rs:32`.
- **TS-recorded reference fixture** — RPC-066 rule [10] said the initial golden is Rust-pinned; the TS-recorded variant is a follow-up card once the TS stub-provider boot recipe is documented (see RPC-066 README scenario at line 716).
- **Live-Anthropic smoke test for the fspec binary** — RPC-072 AC (c) deferred this to a separate follow-up (not part of RPC-069).

---

## Risk acknowledgement

- **`StubProvider::create_rig_agent` does not exist today.** Edit 1 assumes Path A (add a `StubModel` adapter) or Path B (bypass `run_agent_stream_with_images`). Decide during testing phase, not during specifying — both are viable, neither expands the card's scope materially.
- **`scenario_scripted_run_matches_golden`** is the most fragile test. If the chosen path emits a chunk-stream that doesn't match what a real provider would emit (e.g. missing `TokenUpdate`, missing `ContextFillUpdate`), the golden fixture pins the Rust-side behaviour as the baseline. That's accepted by RPC-066 rule [10].
- **Feature-gate hygiene**: `#[cfg(feature = "test-support")]` on the new arm means production builds compile it out entirely. Confirm by `cargo build --release -p codelet-fspec` (without `--features test-stub-provider`) — the stub arm should not appear in symbol output.

---

## Reference — key file:line citations

| Anchor | Path | Line |
|--------|------|------|
| Provider `match` block | `codelet/agent-loop/src/agent_loop.rs` | 880-1034 |
| `run_with_provider!` macro | `codelet/agent-loop/src/dispatch.rs` | 36-106 |
| `agent_loop_dispatch_supports_provider` | `codelet/agent-loop/src/dispatch.rs` | 118-124 |
| `StubProvider` + `LlmProvider` impl | `codelet/providers/src/stub_provider.rs` | 31-101 |
| `register_stub_provider`, `is_stub_registered`, `get_stub_provider` | `codelet/providers/src/stub_provider.rs` | 131, 145, 156 |
| `custom_provider_registered("stub")` consults registry | `codelet/providers/src/manager.rs` | 131-146 |
| `build_service` registers stub + sets default model | `codelet/fspec/src/common.rs` | 81-134 |
| `BackgroundSession::handle_output` | `codelet/sessions/src/background_session.rs` | 775 |
| Feature flag `test-stub-provider` | `codelet/fspec/Cargo.toml` | 93-109 |
| Feature flag `test-support` | `codelet/agent-loop/Cargo.toml` | 50-67 |
| `#[ignore]` markers | `codelet/fspec/tests/cross_frontend_parity.rs` | 58, 139, 369, 646 |
