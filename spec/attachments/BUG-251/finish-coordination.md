# BUG-251 / BUG-252 finish coordination

Owner session: `10e330e7-bb1c-4402-8d48-7241d39898d7`.

The prior executable RED evidence is retained. The structured input writer,
media converter and discovery classifier are already integrated. Scoped
release-slim GREEN attempts use two build jobs and no target cleanup.
Both initial finish attempts stopped before test execution with E0599:
`conversation_commit.rs:20` calls `MessageStore::store_with_id`, which is not
implemented yet. The core owner is actively testing that prerequisite. Logs:
`/tmp/bug251-finish-green.log` and `/tmp/bug252-finish-green.log`. Neither is
behavioral RED or GREEN. BUG-246 received this blocker notice successfully.

BUG-249/250 owner `bc2c91d4-3bec-46f2-b35e-1a32328d3022` may add the narrow
`_transcript` display-only envelope hook in `sessions/src/replay_content.rs`.
No BUG-251 replay edits are in flight. Preserve the existing image/document
decoder, and keep transcript events out of model messages. Please send the
schema and completion notice for integrated audit. BUG-251 will not edit that
shared file while the hook is in progress.

BUG-244/245 must retain independent archive and reduced-model projections;
media blocks pass through the common replay decoder without flattening them
into text. BUG-243 owns CLI/provider-state integration. BUG-252 owns only the
OpenAI discovery helper, its existing delegate and typed classification errors,
plus profile enumeration classification; no CLI edits are planned.

Initial coordination messages to the parent and workers were rejected as busy,
not queued. This file records the coordination without claiming delivery.
Both cards are now validating after actual integrated GREEN, never done.

`/tmp/bug251-252-finish-green2.log` records 22 passing main-process tests and
two passing fresh-process accepted-image replay invocations. Durable copies
are attached to both cards. The sequential validation pipeline, shell PID
`94551`, is still alive; its scoped library check is PID `2612` and is queued
behind shared builds. Strict library/test Clippy follows only if predecessors
succeed. The command list and exact log paths are in `finish-audit.md`.

Available disk declined to about 11 GiB during concurrent builds. This owner
does not clean or delete shared artifacts. No more Cargo commands are being
added to the queue by this owner.
