# RPC-414 — Fspec Tool Help Unreachable in the Codelet TUI

## Summary

Inside the **codelet TUI**, the rig `Fspec` tool cannot produce command help. Both of
these agent invocations fail today:

| Invocation (rig Fspec tool) | Current result | Expected |
| --- | --- | --- |
| `command: "create-prefix --help"` | ❌ `Unknown fspec command: create-prefix --help` | Usage doc for `create-prefix` |
| `command: "help", args: {"command": "create-prefix"}` | ❌ `Unknown fspec command: help` | Usage doc for `create-prefix` |
| `command: "help"` | ❌ `Unknown fspec command: help` | General Fspec tool help |

This is a **defect against the tool's own advertised contract**: the `Fspec` tool
definition (`codelet/tools/src/fspec.rs:80`) tells the LLM *"Use command=\"help\" to get
detailed documentation on available commands"* — but that path returns
`UnknownCommand` in the native Rust dispatch path.

## Why it happens (root cause)

There are **two separate execution paths** for the Fspec tool:

1. **TypeScript CLI callback path** (`src/utils/fspec-callback.ts`) — used only when a JS
   chunk callback is registered. It has a regex interceptor
   (`/^(.+?)\s+--help$/`, line 856) and a `command === 'help'` handler (lines 798–840).
   **This path is NOT used by the codelet TUI.**

2. **Native Rust dispatch path** (`codelet/agent-loop/src/agent_loop.rs:542–576` →
   `codelet_fspec_core::dispatch_command`) — used whenever
   `!is_global_chunk_callback_registered()`, which is exactly the codelet standalone/TUI
   host. **This is the path the user hits.**

In the native path, `dispatch_command` (`codelet/fspec-core/src/dispatch.rs:102–136`)
does:

```rust
let canonical = match lookup(&req.command) {   // exact whole-string match
    Some(c) => c,
    None => return DispatchResult::from_error(FspecCoreError::UnknownCommand {
        command: req.command.clone(),
    }),
};
```

`lookup` (`codelet/fspec-core/src/canonical.rs:838`) is literal equality:

```rust
CANONICAL_COMMANDS.iter().find(|c| c.name == name)
```

So `"create-prefix --help"` is treated as a single opaque command name, never matches,
and returns `UnknownCommand`. There is **no `--help` parsing and no `help` command** in
this path. (`help` is not in `CANONICAL_COMMANDS` or `PORTED_COMMANDS`.)

## Existing infrastructure to reuse (do NOT rebuild)

The `fspec-core` crate **already contains a complete help registry** — the fix should
route into it, not duplicate it:

- **Per-command config**: `codelet/fspec-core/src/help/configs/<snake>.rs` exposes
  `pub const CONFIG: CommandHelpConfig` (e.g. `create_prefix::CONFIG`), registered in
  `codelet/fspec-core/src/help/configs/mod.rs` via `pub mod <name>;`.
- **Formatter**: `codelet/fspec-core/src/help/mod.rs::format_command_help(&CommandHelpConfig) -> String`
  (byte-for-byte port of the TS `help-formatter.ts` non-TTY path).
- **Note**: a handful of commands intentionally have **no `CONFIG`** (`register-tag`,
  `board`, `delete-features`, `delete-scenarios`, `list-foundation-sections`) because
  their TS reference ships no custom help. These must degrade gracefully (see rules).

## Required behavior (the fix)

Add a **help-routing pre-step to `dispatch_command`, BEFORE the canonical lookup**. Pseudo:

```
fn dispatch_command(req):
    if let Some(help_result) = try_dispatch_help(&req.command, &req.args_json):
        return help_result            # success=true with rendered doc, or a clear error
    # ... existing canonical lookup + run_ported/run_stub unchanged ...
```

`try_dispatch_help` recognises three shapes and returns `Some(DispatchResult)` only when
the input is a help request (otherwise `None` so normal dispatch proceeds):

1. **`command == "help"`, no `args.command`** → render **general Fspec tool help**
   (overview + how to get per-command help). Return `success = true`.
2. **`command == "help"`, `args.command == "<name>"`** → render per-command help.
3. **`command` matches `^(?<name>.+?)\s+(--help|-h)$`** → extract `<name>`, render its
   per-command help.

Per-command rendering resolves `<name>` to its `CONFIG` and calls
`format_command_help`. Resolution outcomes:

- **Has `CONFIG`** → `success = true`, `data = format_command_help(&CONFIG)`.
- **Valid canonical command but no `CONFIG`** → `success = true` with a short
  "no detailed help available; usage: fspec `<name>`" style message (must NOT be a hard
  error — the command is real).
- **Not a canonical command at all** (e.g. `foo --help`) → `success = false` with
  `UnknownCommand`-style message naming `<name>` (the *stripped* name, not the raw
  `"foo --help"` string).

## Design constraints

- **Keep it in `fspec-core`.** New logic belongs in a dedicated module
  (e.g. `codelet/fspec-core/src/help_dispatch.rs`) kept **under 300 LoC**; wire a single
  call into `dispatch.rs`. Do not bloat `dispatch.rs`.
- **Reuse `format_command_help` + the `configs` registry.** No new help text authored
  here; no duplication of TS content.
- **No `unwrap()`/`expect()`/`todo!()`/`unimplemented!()`** in production paths. Parse
  `args_json` defensively (missing/blank/invalid → treat as "no args.command").
- **`-h` and `--help`** must both be recognised as the trailing flag form.
- **Whitespace tolerant**: `"create-prefix --help"` and `"create-prefix   --help"` behave
  the same.
- **Do not change the existing `UnknownCommand`, `NotYetPorted`, `InvalidArgs`
  contracts** for non-help inputs. Non-help dispatch must be byte-identical to today.
- **Config-registry mapping**: `<command>` is kebab-case in the tool call; `CONFIG`
  modules are snake_case. Map kebab→snake to resolve the module (or build a static
  name→CONFIG table). Prefer an explicit static lookup table over ad-hoc string munging.

## Verification plan (ACDD)

Tests live in the `fspec-core` crate (unit/integration on `dispatch_command`), driven by
the feature file for this card. Every Gherkin step maps 1:1 to a Rust `@step` comment.

Key assertions:

1. `dispatch_command{command:"create-prefix --help"}` → `success == true`, `data`
   contains the `create-prefix` usage header and its positional arguments.
2. `dispatch_command{command:"create-prefix -h"}` → same as `--help`.
3. `dispatch_command{command:"help", args_json:"{\"command\":\"create-prefix\"}"}` →
   `success == true`, same doc as case 1.
4. `dispatch_command{command:"help"}` (no args) → `success == true`, general help text.
5. `dispatch_command{command:"nonexistent-xyz --help"}` → `success == false`, error
   names `nonexistent-xyz` (stripped), message contains `Unknown fspec command`.
6. **Regression**: `dispatch_command{command:"create-prefix"}` (no help) still routes to
   the normal ported/stub path unchanged — help routing must not intercept real commands.
7. A command with no `CONFIG` (e.g. `board --help`) → `success == true`, graceful
   "no detailed help" message (NOT `UnknownCommand`).

## Out of scope

- The TypeScript `fspec-callback.ts` path (already handles `--help` for the NAPI/Node
  host). This card only fixes the native Rust dispatcher used by the codelet TUI.
- The standalone `fspec` clap CLI `intercept_ts_help` (`codelet/fspec/src/main.rs`) — that
  is the terminal `fspec <cmd> --help` surface, a different entry point, already working.
- Authoring new per-command help content for commands that currently lack `CONFIG`.

## Key file/line references

| Concern | File | Lines |
| --- | --- | --- |
| Native path selection (no JS callback) | `codelet/agent-loop/src/agent_loop.rs` | 542–576 |
| Dispatcher entry (needs help pre-step) | `codelet/fspec-core/src/dispatch.rs` | 102–136 |
| Canonical exact-match lookup | `codelet/fspec-core/src/canonical.rs` | 838 |
| Help formatter (reuse) | `codelet/fspec-core/src/help/mod.rs` | `format_command_help` (~88) |
| Per-command CONFIG registry (reuse) | `codelet/fspec-core/src/help/configs/mod.rs` | module list |
| Example CONFIG | `codelet/fspec-core/src/help/configs/create_prefix.rs` | `CONFIG` (~42) |
| UnknownCommand contract | `codelet/fspec-core/src/error.rs` | 31–32 |
| Fspec tool advertises `command="help"` | `codelet/tools/src/fspec.rs` | 80 |
