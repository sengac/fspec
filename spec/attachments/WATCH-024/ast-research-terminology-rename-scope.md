# AST Research: WATCH-024 Terminology Rename Scope

## Rust Files (codelet/)

### session_manager.rs (~200+ occurrences)
- WatchGraph struct (lines 1696-1820) — rename to ChainOfCommand
- SessionRole struct (lines 295-330) — rename to SupervisorRole, drop authority field, rename description→brief
- RoleAuthority enum (lines 150-180) — REMOVE entirely
- WatcherState enum (line 414) — rename to SupervisorState
- WatcherInput struct (lines 334-402) — rename to SupervisorInput
- WatcherOutput struct (lines 6219-6260) — rename to SupervisorOutput
- watcher_agent_loop fn (line 5793) — rename to supervisor_agent_loop
- watcher_loop_tick fn (line 622) — rename to supervisor_loop_tick
- run_watcher_loop fn (line 729) — rename to run_supervisor_loop
- format_watcher_input fn (line 402) — rename to format_supervisor_input
- watcher_broadcast field (line 945) — rename to supervisor_broadcast
- watcher_input_tx/rx fields (lines 952-953) — rename to supervisor_input_tx/rx
- SessionManager.watch_graph field (line 4127) — rename to chain_of_command
- receive_watcher_input fn (line 1562) — rename to receive_supervisor_input
- watcher_input_sender fn (line 1574) — rename to supervisor_input_sender
- session_create_watcher NAPI fn — rename to session_create_supervisor
- session_get_watchers NAPI fn — rename to session_get_supervisors
- session_get_parent NAPI fn — rename to session_get_subordinate
- session_set_role NAPI fn — update parameter (remove authority)
- session_clear_role NAPI fn — REMOVE entirely
- watcher_inject NAPI fn — rename to supervisor_inject, REMOVE #[napi]
- SessionRoleInfo NAPI struct (line 6949) — update (remove authority field)
- All test modules within the file (~20+ test functions)

### types.rs
- WatcherInputImage struct (line 34) — rename to SupervisorInputImage
- StreamChunk::WatcherInput variant (line 321) — rename to SupervisorInput
- StreamChunk::WatcherPendingInjection variant (line 329) — rename to SupervisorPendingInjection
- WatcherPendingInjectionInfo struct (line 172) — rename to SupervisorPendingInjectionInfo
- watcher_input_with_images fn (line 459) — rename to supervisor_input_with_images
- JSON serialization (line 630) — watcherInput → supervisorInput

### navigation.rs
- Import WatchGraph (line 21) — rename to ChainOfCommand
- build_navigation_list parameter (line 42) — rename type

### Test files (5 files):
- navigation_hierarchy_test.rs — MockWatchGraph → MockChainOfCommand
- watcher_interjection_test.rs — SessionRole/RoleAuthority mocks, all watcher references
- message_duplication_test.rs — TestWatcherInput references
- session_model_validation_test.rs — watcher comment references
- watcher_broadcast_tests module in session_manager.rs

## TypeScript Files (src/)

### Production files (15 files):
1. AgentView.tsx — slash commands (/watcher, /parent), component imports, chunk processing
2. WatcherCreateView.tsx → SupervisorCreateView.tsx (rename file)
3. WatcherTemplateList.tsx → SupervisorTemplateList.tsx (rename file)
4. WatcherTemplateForm.tsx → SupervisorTemplateForm.tsx (rename file)
5. SplitSessionView.tsx — useWatcherHeaderInfo, pane types
6. TurnContentModal.tsx — 'watcher' role
7. watcherTemplate.ts → supervisorTemplate.ts (rename file + type)
8. conversation.ts — 'watcher-input' type, 'watcher' role
9. watcherTemplateStorage.ts → supervisorTemplateStorage.ts (rename file)
10. chunkProcessor.ts — parseWatcherPrefix, WatcherInput chunk, WATCHER: regex
11. conversationUtils.ts — 'watcher-input', 'watcher' role
12. lazyLineIndex.ts — 'watcher-input'
13. thinkingBlockManager.ts — 'watcher-input'
14. slashCommands.ts — '/watcher', '/parent' commands
15. useWatcherHeaderInfo.ts → useSupervisorHeaderInfo.ts (rename file)
16. correlationMapping.ts — 'parent'/'watcher' pane refs

### Test files (24 files + 2 fixtures):
All test files reference sessionGetWatchers, sessionGetParent, and various watcher types

## Terms with ZERO TypeScript consumers (confirm safe to remove):
- watcherInject — NO TS references
- sessionClearRole — NO TS references
- RoleAuthority — NO TS references (only Rust concept)
- WatcherPendingInjection — NO TS references (only in Rust StreamChunk JSON)

## DO NOT TOUCH:
- work_units_watcher.rs (filesystem watcher)
- useWorkUnitsWatcher hook
- startWorkUnitsWatcher/stopWorkUnitsWatcher NAPI bindings
- Existing feature files (done cards — don't rename)
