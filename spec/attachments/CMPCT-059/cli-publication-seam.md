# CLI completion seam — 2026-10-18

CLI owner312d7375 is repairing automatic builder publication. New planned
`StreamOutput::publish_committed_compaction(snapshot: CommittedContextSnapshot)
-> Result<(), String>` publishes before acknowledgement. The default forwards
generation-bearing context, tokens and fill through existing events; background
output must override it to forward `StreamChunk::CommittedCompaction { snapshot }`
through BackgroundSession's authoritative observer refresh. Agent-loop00c6b2ff
owns that override and a real observer regression; CLI does not edit its files.

Recovery preparation will use a separate Session builder message view exposed by
`inference_messages()`. No live messages/turns/token reset or provider reset is
allowed before durable promotion. Session source capture remains authoritative.
