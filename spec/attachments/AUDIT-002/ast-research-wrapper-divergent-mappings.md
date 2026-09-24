# AST Research — AUDIT-002 P1 band 2 (wrapper.rs divergent mappings)

Scope: the 21 divergent scenarios in `fixlist.json` attributed to
`rust/tools/src/facade/wrapper.rs` (features
`isolated-session-file-operations` +
`require-session-id-for-all-tools-to-support-worktree-isolation`).

## Findings

### Implementing functions (tight ranges, all < 300 lines)

| Function | File | Lines | Role |
|---|---|---|---|
| `validate_and_resolve_path` | rust/tools/src/facade/wrapper.rs | 779-789 | Per-session gateway: normalizes Unicode, looks up isolation context, delegates |
| `validate_and_resolve_path_with_isolation` | rust/tools/src/facade/wrapper.rs | 805-932 | Core allow/block logic: worktree→ALLOW, blocked_project→BLOCK, other→ALLOW |
| `validate_and_resolve_path_with_cwd` | rust/tools/src/facade/wrapper.rs | 944-966 | Legacy wrapper deriving IsolationContext from worktree path |
| `get_isolation_context` | rust/tools/src/facade/wrapper.rs | 726-732 | Resolves ctx via registered callback |

### Base tools that consume the gateway (the scenarios actually exercise these)

Each file tool calls `validate_and_resolve_path` inside `rig::tool::Tool::call`:

| Tool | `call` lines | Validation call site |
|---|---|---|
| ReadTool | rust/tools/src/read.rs 273-494 | L287 |
| WriteTool | rust/tools/src/write.rs 66-114 | L80 |
| EditTool | rust/tools/src/edit.rs 69-151 | L83 |
| LsTool | rust/tools/src/ls.rs 147-313 | L162 |
| GrepTool | rust/tools/src/grep.rs 413-494 | L428/L434 |
| GlobTool | rust/tools/src/glob.rs 91-197 | L95 |
| AstGrepTool | rust/tools/src/astgrep.rs 442-513 | L457/L463 |
| AstGrepRefactorTool | rust/tools/src/astgrep_refactor.rs 1070-1166 | L1084/L1095 |

### Test-side gap (drives new integration test)

- `isolated-session-file-operations` currently maps all 24 scenarios to the
  deleted TS file `src/tui/__tests__/isolated-session-file-blocking-e2e.test.ts`
  (P0 dead test mapping) + wrapper.rs 547-738 blob (P1 divergent).
- No Rust test invokes the REAL tools (ReadTool/WriteTool/…) against an
  isolated session — `require-session-id` scenarios test only the validation
  helper, not tool-level enforcement.
- Precedent for the test harness: `rust/sessions/tests/wt004_session_tool_callbacks.rs`
  (real SessionManager, real git worktree, tool-callback registration).
- codelet-sessions already depends on codelet-tools (sessions/Cargo.toml), so
  the tools are directly constructible in that crate's tests.

### Re-link plan

- 22 scenarios in `isolated-session-file-operations` → new Rust
  integration test (one per scenario, `@step` comments) + per-tool `call`
  impl ranges.
- 6 scenarios in `require-session-id-for-all-tools-to-support-worktree-isolation`
  flagged divergent (AstGrep/AstGrepRefactor reject-absolute, Grep search,
  AstGrep* worktree resolve) → existing `test_*` functions in
  wrapper.rs tests (2615-2665, 2765-2855).
