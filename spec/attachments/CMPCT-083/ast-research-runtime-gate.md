# AST research: cold-process gate and runtime publication

AST searches (2026-10-17):

- `emit(StreamChunk::compaction_complete($RESULT))` in
  `rust/agent-loop/src/inject_summary_handler.rs` matches the legacy event
  producer at line 119. It publishes completion without a durable identity.
- `fn resume_session(&self, $SESSION: &SessionId) -> Result<(), String> { $$$BODY }`
  matches `rust/sessions/src/handle_impl.rs:173`. This real entry loads a manifest,
  creates the background session, restores persisted messages, then restores
  counters. The acceptance gate invokes this entry from a distinct OS process.

The first gate checks the physical Anthropic adapter with real temporary stores,
deep original payload retrieval, exact DAG and suffix tool call/result identities,
and generation-bound context snapshots. It is deliberately not a source-string
wiring assertion or a synthetic request JSON test. Other provider rows and the
negative matrix must be integrated before CMPCT-083 can be completed.

Concurrent compile blockers are recorded separately from behavioral red evidence:
missing compactor module during another owner's extraction and patched Gemini
usage trait/doc compilation failures. Neither counts as a failing acceptance test.
