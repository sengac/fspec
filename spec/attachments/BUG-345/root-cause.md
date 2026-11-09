# BUG-345 root cause: the provider tool-chain guard reads a file that no longer holds the chain

## Symptom

`cargo test --profile ci-test -p codelet-agent-loop --test cmpct045_generate_compaction_behavioral`
fails one test, `generate_compaction_tool_wired_into_every_provider_tool_chain`
(now in `rust/agent-loop/tests/cmpct045/part_02.rs` after REFAC-033), with
`CMPCT-045: rust/providers/src/openai.rs must wire DeepSearchTool`.

## Cause

REFAC-031 (f167f1f9) split `rust/providers/src/openai.rs` into modules. `create_rig_agent`, and with
it the `.tool(DeepSearchTool::new(session_id))` / `.tool(GenerateCompactionTool::new(session_id))`
chain, moved to `rust/providers/src/openai/agent.rs` (lines 122 and 124). The guard keeps a
hard-coded list of provider source files and still names `rust/providers/src/openai.rs`, so its
`find("DeepSearchTool::new(session_id)")` returns `None` there and the test panics. The OpenAI
wiring itself is intact. REFAC-031's regression set ran the providers crate only and missed this
cross-crate source read from agent-loop.

## Fix

Point the guard's OpenAI entry at `rust/providers/src/openai/agent.rs`, as BUG-311 did for Codex
(`codex/agent.rs`). The feature step names the files the guard reads, and the scenario's coverage
implementation mappings cover every provider's chain (it mapped only claude.rs).

## Not changed

No production code changes. The guard keeps its rule (GenerateCompactionTool within 400 chars after
DeepSearchTool in each file).
