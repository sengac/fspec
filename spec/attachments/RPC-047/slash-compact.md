# RPC-047 — `/compact` slash command + compaction progress footer

**Parent:** RPC-030 · **Phase:** 6.4 · **Estimate:** 5 pts · **Depends on:** RPC-046

## Goal

Wire `SlashCommandAction::Compact` (currently a notice-fallback at `dispatch_rpc020.rs` line 73-78) to call `backend.compact_session(session_id)`. Render compaction progress in `SessionFooter`. On completion render `CompactionResult` (compression ratio + token counts).

## Trait wiring (from RPC-037)

`FspecBackend::compact_session(&self, session_id: SessionId) -> Result<CompactionResult>`.

`SessionManagerHandle::get_compaction_progress(&self, session_id) -> Option<CompactionProgress>` returns the live phase + current/total.

`StreamChunk::CompactionComplete { compaction_result: CompactionResult }` already exists in `codelet/rpc-types/src/lib.rs`.

## Work to do

### Step 1 — Slash handler

```rust
SlashCommandAction::Compact => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("/compact: no active session");
        return;
    };
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        match backend.compact_session(session_id.clone()).await {
            Ok(result) => {
                let line = format!(
                    "[compaction] {:.1}% reduction ({} → {} tokens, {} turns summarised)",
                    (1.0 - result.compression_ratio) * 100.0,
                    result.original_tokens,
                    result.compacted_tokens,
                    result.turns_summarized,
                );
                let _ = sender.send(Action::EmitNotice { session_id, text: line });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[error] /compact failed: {e}"),
                });
            }
        }
    });
}
```

### Step 2 — SessionFooter compaction-progress widget

`AgentViewStore` already has `compaction_progress_by_session: HashMap<SessionId, CompactionProgress>` (or add it). The dispatcher updates this map whenever a chunk-driven progress update arrives (the agent loop emits incremental progress via internal channels or via a new chunk variant — confirm by reading `BackgroundSession::update_compaction_progress` at line 1175 of `session_manager.rs`).

`SessionFooter` reads the map and renders:

```
[compacting: summarising messages 12/45]
```

Bar visual: `▰▰▰▰▰▱▱▱▱▱` driven by `current / total`.

### Step 3 — `CompactionComplete` chunk handler

In RPC-045 dispatcher, add a branch for `StreamChunk::CompactionComplete`:

```rust
StreamChunk::CompactionComplete { compaction_result } => {
    self.agent_view_store.clear_compaction_progress(&session_id);
    // The notice line is also emitted from the slash-handler side, but in case
    // compaction was triggered automatically (token-threshold), emit it here.
    self.emit_compaction_notice(&session_id, compaction_result);
}
```

## Acceptance criteria

1. `/compact` triggers `backend.compact_session(session_id)`.
2. SessionFooter shows progress bar with phase + count.
3. On completion, scrollback shows `[compaction] X% reduction (Y → Z tokens)`.
4. Failure path emits `[error] /compact failed: ...`.
5. Integration test in `codelet/fspec-tui/tests/slash_compact.rs` drives stub backend through happy + sad paths.

## Out of scope

- Auto-compaction at token threshold (existing in agent loop; no UI work needed).
