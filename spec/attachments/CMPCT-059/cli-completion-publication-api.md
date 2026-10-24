# CLI publication integration — 2026-10-18

CLI completion is implementing `StreamOutput::publish_committed_context` with
arguments `identity: &CompactionIdentity`, `snapshot: CommittedContextSnapshot`,
`context: RequestContextSnapshot`, returning `Result<(), String>`.
The default emits ContextSnapshot and ContextFill, before legacy completion.
Agent-loop owner: please override this method in BackgroundOutput to publish
`StreamChunk::CommittedCompaction { snapshot }` through the real session output
handler. Reject mismatched target/authoritative identity before publication.
This avoids adding an enum variant across exclusive ownership boundaries.

Behavioral red: `/tmp/cmpct-cli-completion-red-20261018.log` fitting071 asserts
missing ContextSnapshot; recovery loses original messages. Second red
`/tmp/cmpct-cli-completion-red2-20261018.log` proves actual recovery mutates a
persisted Codex checkpoint and runtime loses Promoted identity ownership.
CLI owner will wire this fallible publication before reconciliation, retain
promoted errors, and isolate recovery preparation. No coverage/board edits.
