# RPC-020 — AST research summary

## Existing FspecBackend surface (AST-scanned)

`async fn ...` method declarations in
`codelet/fspec-tui/src/transport/embedded.rs` and
`codelet/fspec-tui/src/transport/websocket.rs` — each backend currently
implements 12 methods:

- list_work_units / list_sessions / create_session / send_input / interrupt
- health / checkpoint_counts
- move_work_unit_up / move_work_unit_down
- get_model_info / get_thinking_level / get_workspace_info

RPC-020 adds a 13th: `search_files(prefix, limit) -> Result<Vec<String>>`.

Both transports follow the SAME one-line-tarpc-delegate pattern (per
RPC-005 architecture rule "service impl written ONCE in a shared
module"). The WebSocket variant additionally guards on
`BackendError::Disconnected` when the inner client slot is None.

## Existing AgentView widget surface (AST-scanned)

`codelet/fspec-tui/src/views/agent/multiline_input.rs` declares the
sole `pub struct MultiLineInput` field on the AgentView orchestrator.
RPC-020 introduces sibling widget modules `slash_command_popup.rs` and
`file_search_popup.rs` and a backing registry module
`slash_commands.rs`. The orchestrator (`views/agent.rs`) is currently
247 lines so adding popup ownership without splitting risks crossing
the 300-LoC ceiling — accordingly `views/agent.rs` keeps the bare
Option<...> field declarations + the small `sync_popups()` helper and
delegates render/event-handling to the popup widget modules.

## Existing slash command + file search TS reference (read)

- `src/tui/components/SlashCommandPalette.tsx` — 172-line Ink popup with
  three-tier filter matching.
- `src/tui/components/FileSearchPopup.tsx` — 170-line Ink popup.
- `src/tui/utils/slashCommands.ts` — `SLASH_COMMANDS` registry +
  `filterCommands()` three-tier matcher.
- `src/tui/hooks/useSlashCommandInput.ts` — hosts visibility/filter
  state, handles arrow keys / Tab / Enter / Esc.
- `src/tui/hooks/useFileSearchInput.ts` — calls
  `callGlobTool(pattern, searchPath, true)` to fetch file matches
  asynchronously.

## Glob backend (AST-scanned)

`codelet/tools/src/glob.rs::GlobTool::call` uses `ignore::WalkBuilder`
+ `globset::GlobBuilder` (case-insensitive) and returns paths sorted by
modification time desc. RPC-020 ships a smaller analogue in
`codelet/core/src/file_search.rs::search(cwd, prefix, limit)` so the
shared FspecService can call it without dragging codelet_tools into
codelet_rpc's dep graph.
