# REFAC-018 — Dispatch File Rename Name Map

## Problem

The `codelet/fspec-tui/src/app/dispatch_rpcNNN.rs` files are named after the
work-unit / ticket IDs that *introduced* them (RPC-337, RPC-022, …) rather than
the capability they implement. Ticket numbers are point-in-time events; the code
is permanent. Filenames (and the `try_dispatch_rpcNNN` method names) should answer
"what does this do?" — not "which card birthed it?".

This is the same anti-pattern fspec warns about for feature files
(`AUTH-001.feature` BAD vs `user-authentication.feature` GOOD).

## Decisions (recorded 2026-06-19)

1. **Partition:** Rename 1:1 — keep current file boundaries, just rename to
   capability names. (No re-partition by domain.)
2. **Source-shape tests/features:** Retire the structural assertions
   (`source_shape_rpc*` tests + `*-source-shape.feature` scenarios that assert
   file names / method names exist). They are the mechanism that re-imprints the
   anti-pattern.
3. **Process:** Tracked as fspec work unit REFAC-018 on branch `rust-refactor`.

Ticket IDs survive only as a one-line `// Introduced: RPC-NNN` provenance comment
inside each renamed file — history, not identity.

## 1:1 Name Map

| Current file / method | New file / method |
|---|---|
| `dispatch_rpc018` | `dispatch_session_chrome` |
| `dispatch_rpc020` | `dispatch_slash_commands` |
| `dispatch_rpc022` / `try_dispatch_rpc022` | `dispatch_model_thinking_dialogs` / `try_dispatch_model_thinking_dialogs` |
| `dispatch_rpc024` | `dispatch_session_cycle` |
| `dispatch_rpc025` | `dispatch_history_recall` |
| `dispatch_rpc026` | `dispatch_resume_search_views` |
| `dispatch_rpc045` | `dispatch_stream_chunks` |
| `dispatch_rpc045_fspec_runner` | `dispatch_fspec_runner` |
| `dispatch_rpc046` | `dispatch_slash_clear` |
| `dispatch_rpc050` | `dispatch_work_unit_binding` |
| `dispatch_rpc051` | `dispatch_esc_cascade` |
| `dispatch_rpc052` | `dispatch_pending_input` |
| `dispatch_rpc053` / `try_dispatch_rpc053` | `dispatch_pause_hitl` / `try_dispatch_pause_hitl` |
| `dispatch_rpc054` / `try_dispatch_rpc054` | `dispatch_provider_settings` / `try_dispatch_provider_settings` |
| `dispatch_rpc055` / `try_dispatch_rpc055` | `dispatch_slash_debug` / `try_dispatch_slash_debug` |
| `dispatch_rpc056` / `try_dispatch_rpc056` | `dispatch_blocklist` / `try_dispatch_blocklist` |
| `dispatch_rpc057` / `try_dispatch_rpc057` | `dispatch_merge_worktree` / `try_dispatch_merge_worktree` |
| `dispatch_rpc058` / `try_dispatch_rpc058` | `dispatch_slash_schedule` / `try_dispatch_slash_schedule` |
| `dispatch_rpc059` / `try_dispatch_rpc059` | `dispatch_slash_loop` / `try_dispatch_slash_loop` |
| `dispatch_rpc060` / `try_dispatch_rpc060` | `dispatch_create_session_dialog` / `try_dispatch_create_session_dialog` |
| `dispatch_rpc061` / `try_dispatch_rpc061` | `dispatch_supervisor_links` / `try_dispatch_supervisor_links` |
| `dispatch_rpc063` | `dispatch_role_dialog` |
| `dispatch_rpc079` / `try_dispatch_rpc079` | `dispatch_dialog_dismiss` / `try_dispatch_dialog_dismiss` |
| `dispatch_rpc098` | `dispatch_agent_exit` |
| `dispatch_rpc337` / `try_dispatch_rpc337` | `dispatch_model_selector` / `try_dispatch_model_selector` |

## Blast radius

1. 25 `dispatch_rpc*.rs` source files (`git mv`).
2. `app/mod.rs` — `pub mod dispatch_rpcNNN;` declarations.
3. `app/dispatch.rs` — the `try_dispatch_rpcNNN(&action)` call chain.
4. `try_dispatch_rpcNNN` method names (definitions + call sites).
5. Cross-file `use super::dispatch_rpcNNN::…` imports.
6. ~30 in-code doc-comment cross-references.
7. ~10 `tests/source_shape_rpc*.rs` / `tests/*_rpcNNN.rs` structural tests — retired or de-referenced.
8. ~25 `spec/features/*source-shape*.feature` scenarios that encode the names — retired.

## Execution sequence (build-green at each gate)

1. Create work unit + git checkpoint baseline.
2. `git mv` files to new names; rewrite mod.rs + dispatch.rs chain + method names + `use super::` imports + doc cross-refs. `cargo build` passes.
3. Full `cargo test` (source-shape tests now fail — expected).
4. Retire/neutralize source-shape structural assertions + their feature scenarios.
5. `cargo test` green, `fspec validate` / `validate-tags`, close work unit.

## Completion notes (executed)

- 25 `src/app/dispatch_rpc*.rs` files renamed via `git mv` to capability names.
- `app/mod.rs` `pub mod` decls + `app/dispatch.rs` `try_dispatch_*` call chain + all
  `try_dispatch_rpcNNN` method names + `use super::` imports + in-`src` doc cross-refs updated.
- Module-header ticket IDs demoted to `Introduced: RPC-NNN` provenance; capability now leads.
- Tests: retired the naming-tripwire assertions (`*_file_has_expected_shape`,
  `*_declares_*helper`, `*_module_exists`, `*_hosts_*helpers`, "catch-all routes through
  try_dispatch_rpcNNN") per decision 2. Kept LoC-ceiling + behavioral-wiring + RPC-layer
  contract assertions, updating their hardcoded paths to the new filenames.
- Feature files + `.feature.coverage`: production source-file + method references updated to
  new names (test-file names like `app_dispatch_rpc024.rs` left intact — those files were NOT
  renamed).
- `cargo build`, `cargo test` (169 test binaries, 0 failed), `cargo test --no-run` (0 warnings),
  and `fspec validate` (1424 features valid) all green.

### Known-acceptable leftover
- `spec/features/slash-command-resume.feature` mentions `app/dispatch_rpc049.rs` — a file that
  was NEVER created (RPC-049 logic landed in dispatch_rpc026 → now `dispatch_resume_search_views`).
  Left as accurate historical "we considered creating X" narrative; nothing to rename.
