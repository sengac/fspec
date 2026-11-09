# AST research: BUG-327

## `fields.entry($K).or_insert($V)` in rust/core/src/persistence/session_replay.rs
```
rust/core/src/persistence/session_replay.rs:36:17:fields.entry(key).or_insert(value)

```

## `json!($$$A)` in rust/core/src/persistence/session_replay.rs
```
rust/core/src/persistence/session_replay.rs:14:20:json!({"version":1,"sessionId":session_id,"revision":session.revision})
rust/core/src/persistence/session_replay.rs:28:32:json!([{"type":"text","text":stored.content}])
rust/core/src/persistence/session_replay.rs:29:28:json!({
rust/core/src/persistence/session_replay.rs:41:36:json!("compaction")
rust/core/src/persistence/session_replay.rs:42:38:json!(true)
rust/core/src/persistence/session_replay.rs:43:46:json!(true)
rust/core/src/persistence/session_replay.rs:45:52:json!({
rust/core/src/persistence/session_replay.rs:73:24:json!({"_sessionReplay":identity})

```

## `pub fn rehydrate_envelope_blobs($$$A) -> $R { $$$B }` in rust/core/src/persistence
```
rust/core/src/persistence/blob_processing.rs:144:1:pub fn rehydrate_envelope_blobs(envelope_json: &str) -> Result<String, String> {

```

## `append_message_idempotent($$$A)` in rust/agent-loop/src
```
rust/agent-loop/src/persistence_retry_commit.rs:78:5:append_message_idempotent(

```

## `get_session_message_envelopes($$$A)` in rust/sessions/src
```
No matches found
```

## `envelopes_for_manifest($$$A)` in rust/cli/src
```
rust/cli/src/session/compaction.rs:65:18:envelopes_for_manifest(&manifest)
rust/cli/src/session/compaction.rs:111:22:envelopes_for_manifest(current)
rust/cli/src/session/compaction.rs:200:20:envelopes_for_manifest(manifest)

```

