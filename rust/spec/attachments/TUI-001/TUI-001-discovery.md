# TUI-001 — Model name, capability badges & size badge parity in `get_model_info`

## Summary

The Rust ratatui AgentView **SessionHeader** does not reach parity with the
TypeScript Ink reference. Side-by-side of the chat screen header (top-left model
area):

| Element | TypeScript (reference) | Rust (current) | Wanted |
|---|---|---|---|
| Model name | `Claude Opus 4.8` (friendly) | `claude-opus-4-8` (raw slug) | friendly name |
| Reasoning badge | `[R]` (magenta) | *missing* | `[R]` |
| Vision badge | `[V]` (blue) | *missing* | `[V]` |
| Size badge | `[192k]` (compaction threshold) | `[200k]` (raw context window) | `[192k]` |
| Right side `tokens: 0↓ 0↑ [0%]` | present | present ✅ | — |

> The `[T:High]` thinking badge is a **separate** root cause — see **TUI-002**.
> This card covers ONLY the model name, `[R]`, `[V]`, and the size badge.

## Critical context: the renderer is ALREADY at parity

Do **not** modify the rendering layer. The Rust widget code already paints all
badges conditionally with the correct colours and ordering, mirroring
`src/tui/components/SessionHeader.tsx`:

- `rust/fspec-tui/src/views/agent/header_build.rs::build_left_line`
  - cyan-bold `prefix + work-unit + model_name` block
  - `[R]` magenta when `model.supports_reasoning`
  - `[V]` blue when `model.supports_vision`
  - `[{Nk}]` dark-gray when `model.context_window > 0` (uses `format_context_window`)
- `rust/fspec-tui/src/views/agent/header.rs` — layout / truncation.

The badges are blank **only because the `ModelInfo` reaching the widget carries
empty / false / raw data.** This is a pure **data-feed** bug.

## Data pipeline (already wired, do not rebuild)

```
FspecService::get_model_info (RPC)
  -> rust/fspec-tui/src/transport/{embedded,websocket}.rs::get_model_info
  -> Action::ModelInfoLoaded(session_id, ModelInfo)
  -> rust/fspec-tui/src/store/agent_view/chrome_state.rs::set_model_info
  -> SessionHeader widget reads ModelInfo
```

The server-side trait method is the stub that produces bad data.

## Root cause #1 — `get_model_info` is a stub

`rust/sessions/src/handle_impl.rs` (~lines 870-889):

```rust
fn get_model_info(&self, session_id: &SessionId) -> codelet_rpc_types::ModelInfo {
    use std::sync::atomic::Ordering;
    let uuid = uuid_from(session_id);
    self.get_session(&uuid.to_string())
        .map(|s| {
            let model_id = s.model_id.read().ok().and_then(|g| g.clone()).unwrap_or_default();
            codelet_rpc_types::ModelInfo {
                display_name: model_id,        // ← raw slug "claude-opus-4-8"
                supports_reasoning: false,     // ← hardcoded → no [R]
                supports_vision: false,        // ← hardcoded → no [V]
                context_window: s.cached_context_window.load(Ordering::Acquire), // 200k raw
            }
        })
        .unwrap_or_default()
}
```

It never consults the model catalog.

### How TypeScript does it (the behaviour to mirror)

TS resolves the friendly name + capabilities from the **models.dev catalog**:
`AgentView.tsx`'s `rustModelInfo` memo calls `findModelInProviders(providerId,
modelId)` over the catalog populated by `modelsListAll()`, then uses
`model.name` (friendly), `model.reasoning`, `model.hasVision`. Context window /
compaction threshold prefer the Rust-resolved `SessionModel` values.

### Everything needed already exists in-process (Rust)

- The session carries `provider_id` and `model_id`
  (`rust/sessions/src/background_session.rs` — `RwLock<Option<String>>`).
- `codelet_providers::models::ModelRegistry`
  (`rust/providers/src/models/registry.rs`) exposes:
  - `get_model(provider, model) -> Result<&ModelInfo, _>`
  - the catalog `ModelInfo` has `name`, `reasoning`,
    `has_capability(Capability::Vision)`, `limit.context`.
- `rust/sessions/src/cloud_models.rs` **already performs this exact mapping**
  for the model selector (`cloud_model_entries`). Reuse the same registry lookup
  + the `canonical_to_models_dev` slug mapping (`gemini` -> `google`) that lives
  in that file.

### Required behaviour for #1

`get_model_info` should:
1. Read the session's `provider_id` + `model_id`.
2. Map the provider slug via `canonical_to_models_dev`.
3. `registry.get_model(dev_id, model_id)`:
   - **Hit** → `display_name = catalog.name`,
     `supports_reasoning = catalog.reasoning`,
     `supports_vision = catalog.has_capability(Vision)`.
   - **Miss** → fall back to the CURRENT behaviour (raw `model_id`,
     `reasoning=false`, `vision=false`). This mirrors the TS fallback path and
     keeps graceful degradation when no session manager / unknown model.

## Root cause #2 — `[192k]` vs `[200k]` (compaction threshold not on the wire)

TS `SessionHeader.tsx:165`:

```ts
const badgeValue = compactionThreshold ?? contextWindow; // 192k = ~0.96 * 200k
```

The size badge shows the **compaction threshold**, not the raw window. Problems
in Rust:

1. `codelet_rpc_types::ModelInfo` (`rust/rpc-types/src/lib.rs:285`) has **no
   `compaction_threshold` field**.
2. `header_build.rs` uses `m.context_window` directly for the `[Nk]` badge.

The data already exists server-side: the session stores
`cached_compaction_threshold` (AtomicU32, see `background_session.rs`), and the
sibling `get_model` RPC already returns `compaction_threshold` in `SessionModel`
(`handle_impl.rs:206`).

### Required behaviour for #2

1. Add `compaction_threshold: u32` to `codelet_rpc_types::ModelInfo`
   (`#[cfg_attr(feature = "napi", napi_derive::napi(object))]` struct — keep the
   `Default`/derives). Document it like the existing fields.
2. Populate it in `get_model_info` from
   `s.cached_compaction_threshold.load(Ordering::Acquire)`.
3. In `header_build.rs::build_left_line`, compute the badge value as
   `if compaction_threshold > 0 { compaction_threshold } else { context_window }`
   and feed it through `format_context_window` (so `192000` → `192k`). This
   mirrors `compactionThreshold ?? contextWindow`.

## Tests to update / add (these already assert the shape — keep green)

- `rust/fspec-tui/tests/source_shape_rpc018.rs` — asserts `ModelInfo` shape &
  default impls. Adding a field requires updating any literal `ModelInfo { .. }`
  constructions in tests across `rust/fspec-tui/tests/**` and
  `rust/sessions/**` (search for `ModelInfo {`).
- New ACDD scenarios should cover:
  - friendly display name resolved from catalog for a known model;
  - `[R]`/`[V]` reflect catalog capabilities;
  - unknown model falls back to raw slug with no capability flags;
  - size badge uses compaction threshold when > 0, else context window.

## Files in scope

| Purpose | Path |
|---|---|
| Stub to fix (server) | `rust/sessions/src/handle_impl.rs` (`get_model_info`) |
| Registry lookup helper to reuse | `rust/sessions/src/cloud_models.rs` |
| Model registry API | `rust/providers/src/models/registry.rs` |
| Wire struct (add field) | `rust/rpc-types/src/lib.rs` (`ModelInfo`) |
| Size-badge selection | `rust/fspec-tui/src/views/agent/header_build.rs` |
| Renderer (reference only) | `rust/fspec-tui/src/views/agent/header.rs` |
| Shape tests | `rust/fspec-tui/tests/source_shape_rpc018.rs` + ModelInfo literals |

## Out of scope

- Any change to the rendering / colour / ordering logic (already correct).
- The `[T:High]` thinking badge (TUI-002).
- The right-hand `tokens: …` block (already at parity).
