# BUG-345 AST research

`fspec astgrep --lang rust --pattern '$B.tool(DeepSearchTool::new(session_id))' --path rust/providers/src`
matches one builder chain per provider (chain starts: claude.rs:537, openai/agent.rs:103,
gemini.rs:194, codex/agent.rs:73, zai.rs:265, copilot/rig_agent.rs:81, custom_provider.rs:300 and
:326). The `.tool(...)` calls inside those chains sit at:

| Provider | File | DeepSearch line | GenerateCompaction line |
|---|---|---|---|
| Claude | rust/providers/src/claude.rs | 560 | 562 |
| OpenAI | rust/providers/src/openai/agent.rs | 122 | 124 |
| Gemini | rust/providers/src/gemini.rs | 215 | 217 |
| Codex | rust/providers/src/codex/agent.rs | 92 | 94 |
| Z.AI | rust/providers/src/zai.rs | 285 | 287 |
| GitHub Copilot | rust/providers/src/copilot/rig_agent.rs | 101 | 103 |
| Custom | rust/providers/src/custom/custom_provider.rs | 305, 331 | 307, 333 |

`rust/providers/src/openai.rs` no longer contains `create_rig_agent` (REFAC-031 moved it to
`openai/agent.rs`), so it has no match. That is the only stale entry in the guard's file list.
