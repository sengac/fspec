# BUG-243 implementation coordination

The parent requested exact shared paths before BUG-245 implementation. The
archival API contract has been delivered to BUG-244/245 owner
`197a9f4a-d009-455d-af59-6cd6dab3b3cf` through AgentManager.

BUG-243 needs these bounded shared-source edits:

- `rust/core/src/persistence/manifest.rs`: one optional versioned
  `provider_state` reference and its constructor initialization.
- `rust/core/src/persistence/mod.rs`: exports for the new provider-state and
  archival APIs.
- `rust/core/src/persistence/originals.rs`: exact internal checkpoint
  classification before the genuine-envelope validation.
- `rust/core/src/persistence/session_replay.rs`: verified authoritative state
  supplied independently from conversational metadata, with no expansion of
  reduced model dialogue.

New implementation lives in focused `provider_state` and `archive` modules.
BUG-243 does not edit `conversation_commit.rs`, `manifest_commit.rs`, MessageStore,
session delivery, or sessions multimodal rendering modules. The user-approved
architecture is `spec/attachments/BUG-242/architecture-approval.md`.

At 12:25 AEST the scoped executable-RED attempt failed compilation because
`conversation_commit.rs:20` calls missing `MessageStore::store_with_id`.
Evidence: `/tmp/bug243-red-confirmed.log`. This is not claimed as behavioral RED.
The core audit owner is repairing that prerequisite. Parent and core-owner live
messages were rejected as busy, not queued; the request is also recorded in the
BUG-243 architecture notes and relayed to BUG-246 owner.
