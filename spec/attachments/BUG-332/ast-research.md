# AST research: BUG-332

## `pub fn envelopes_for_manifest($$$A) -> $R { $$$B }` in rust/core/src/persistence
```
rust/core/src/persistence/session_replay.rs:10:1:pub fn envelopes_for_manifest(session: &super::SessionManifest) -> Result<Vec<String>, String> {

```

## `fields.entry($K).or_insert($V)` in rust/core/src/persistence
```
rust/core/src/persistence/archive/record.rs:42:9:fields.entry("uuid").or_insert(json!(stored.id))
rust/core/src/persistence/archive/record.rs:43:9:fields
rust/core/src/persistence/archive/record.rs:46:9:fields.entry("type").or_insert(json!(stored.role))
rust/core/src/persistence/archive/record.rs:47:9:fields.entry("provider").or_insert(json!("unknown"))
rust/core/src/persistence/session_replay.rs:35:9:fields.entry("parentUuid").or_insert(Value::Null)
rust/core/src/persistence/session_replay.rs:41:9:fields.entry("requestId").or_insert(Value::Null)

```

## `fields.entry($K).or_insert_with($F)` in rust/core/src/persistence
```
rust/core/src/persistence/session_replay.rs:36:9:fields
rust/core/src/persistence/session_replay.rs:39:9:fields.entry("type").or_insert_with(|| json!(stored.role))
rust/core/src/persistence/session_replay.rs:40:9:fields.entry("provider").or_insert_with(|| json!("unknown"))
rust/core/src/persistence/session_replay.rs:42:23:fields

```

## `pub fn append_message($$$A) -> $R { $$$B }` in rust/core/src/persistence
```
rust/core/src/persistence/manifest_messages.rs:2:1:pub fn append_message(

```

## `pub fn append_message_with_metadata($$$A) -> $R { $$$B }` in rust/core/src/persistence
```
rust/core/src/persistence/manifest_messages.rs:11:1:pub fn append_message_with_metadata(

```

## `fn envelope($$$A) -> MessageEnvelope { $$$B }` in rust/agent-loop/src
```
rust/agent-loop/src/persist.rs:177:1:fn envelope(role: &str, provider: &str, message: MessagePayload) -> MessageEnvelope {

```

## `if metadata.contains_key("message") { $$$B }` in rust/agent-loop/src
```
rust/agent-loop/src/persistence_retry_commit.rs:46:5:if metadata.contains_key("message") {

```

## `pub fn get_session_messages($$$A) -> $R { $$$B }` in rust/core/src/persistence
```
rust/core/src/persistence/manifest_messages.rs:54:1:pub fn get_session_messages(session: &SessionManifest) -> Result<Vec<StoredMessage>, String> {

```
