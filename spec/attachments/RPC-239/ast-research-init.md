# AST Research — RPC-239 init port

Tool: AstGrep (language=rust). Performed during ACDD discovery to ground the
Rust port of `src/commands/init.ts` against the existing TS reference helpers
and the ported-command conventions.

## TS reference surface analysed
- `src/commands/init.ts` — executeInit / installAgents / installAgentFiles /
  installFullDoc / installSlashCommand / generateSlashCommandContent.
- `src/utils/agentRegistry.ts` — AGENT_REGISTRY (19 agents) + getAgentById.
- `src/utils/templateGenerator.ts` — generateAgentDoc, stripSystemReminders,
  transformToVisibleInstruction, removeMetaCognitivePrompts, replacePlaceholders.
- `src/utils/slashCommandSections/header.ts` — getHeaderSection (markdown slash).
- `src/utils/agentRuntimeConfig.ts` — writeAgentConfig (read-modify-write merge).
- `src/utils/activationMessage.ts` — getActivationMessage.

## Reference port pattern compared
- `codelet/fspec-core/src/commands/list_prefixes.rs` — 2-arg
  `run(args_json: &str, project_root: &Path)` dispatched via `poll_sync_future`.
- `codelet/fspec-core/src/commands/remove_init_files.rs` — precedent for an
  inlined local agent const table (no shared agents.rs / lib.rs touch).

## AstGrep query — function inventory of the ported init.rs
Pattern: `fn $NAME($$$ARGS) -> $RET { $$$BODY }`

| fn | line | role |
|----|------|------|
| get_agent_by_id | 86 | registry lookup |
| activation_message | 92 | port of getActivationMessage |
| run | 126 | 2-arg dispatcher entry (validation + install + config) |
| install_agent_files | 186 | doc + slash per agent |
| install_slash_command | 209 | toml vs md; codex $HOME path |
| generate_slash_command_content | 236 | header section / inline toml |
| generate_agent_doc | 264 | transform pipeline driver |
| strip_system_reminders | 277 | paired-tag strip loop |
| transform_to_visible_instruction | 300 | visible IMPORTANT rendering |
| strip_severity_prefix | 341 | CRITICAL/WARNING/NOTE/IMPORTANT trim |
| cut_from | 353 | TS `.replace(/X.+/gs,'')` parity |
| capture_section | 362 | TS lookahead-free match |
| remove_meta_cognitive_prompts | 371 | ultrathink/deeply-consider removal |
| remove_ci | 393 | case-insensitive removal |
| replace_placeholders | 417 | {{AGENT_NAME}} etc. |
| write_agent_config | 428 | read-modify-write 2-space JSON |
| home_dir | 455 | injectable HOME (std::env::var_os) |
| fs_create_dir_all / fs_write | 462 / 470 | blocking std::fs scaffolding |

## Findings driving the port
1. All filesystem work maps to blocking `std::fs::create_dir_all` + `write`
   (no async, no child process) so it runs under `poll_sync_future`.
2. The 19-agent registry is inlined as a local const table per the
   remove-init-files precedent — no shared module touch.
3. Headless interactivity collapses to: empty agent list → TTY-guard error;
   agent-switch auto-confirmed.
4. Codex/codex-cli slash command resolves HOME from `std::env::var_os("HOME")`
   (injectable; tests redirect to a tempdir) — never a hard-coded path.
5. KNOWN GAP (see RPC-335): the embedded doc template is a behaviour-faithful
   stand-in, not the verbatim ~2069-line TS projectManagementTemplate.
