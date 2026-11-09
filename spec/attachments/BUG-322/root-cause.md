# BUG-322 root cause

`MessagePayload` (`rust/core/src/persistence/message_envelope.rs`) is `#[serde(untagged)]`, with variants
`User(UserMessage)` and `Assistant(AssistantMessage)`. Serde tries them in order, and `UserMessage { role, content }`
accepts any role and ignores unknown fields. So an assistant envelope whose content blocks are all valid user content
(any text-only reply) deserializes as `User`, and its `model`, `stop_reason` and `usage` are dropped. Assistant messages
containing `tool_use` blocks fail the user content parse and fall through to `Assistant`, which is why the loss only
showed for some messages.

The fix is to deserialize by the `role` field (`"assistant"` → `AssistantMessage`, otherwise `UserMessage`) instead of
by shape. Serialization and the on-disk format are unchanged, so existing manifests now read back correctly.
