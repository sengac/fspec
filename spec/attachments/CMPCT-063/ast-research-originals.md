# CMPCT-063 discovery

AST query: Rust `pub fn get_session_messages_full($$$ARGS) -> $RET { $$$BODY }`
in `rust/core/src/persistence/manifest.rs`.

Match: `get_session_messages_full` at line 1013. It iterates manifest references
but silently skips missing stored messages. Original retrieval must instead
reject missing evidence and retain manifest positions.

The existing `get_session_message_envelopes` constructs synthetic envelopes and
rehydrates through a narrowed schema. It is suitable for existing restoration
semantics, not lossless originals. Reuse the core stores and blob reader, but
read structured metadata without conversion through MessageEnvelope.

Both agent-loop and legacy NAPI SessionSearch handlers match the shared action
enum. Add minimal dispatch to a single core reader; do not duplicate logic or
change preview behavior. User approved minimal wiring of existing oversized
files rather than broad refactoring.
