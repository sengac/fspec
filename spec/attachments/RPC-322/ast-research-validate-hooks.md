# AST Research — `validate-hooks` (RPC-322)

## TS source
- `src/commands/validate-hooks.ts` — `validateHooks({cwd})` + `registerValidateHooksCommand`.
- `src/hooks/types.ts` — `HookConfig { global?, hooks: Record<string, HookDefinition[]> }`, `HookDefinition { name, command, blocking?, timeout?, condition? }`.
- Help: `validate-hooks-help.ts` (custom, auto-discovered via help-registry import.meta.glob → needs help config module).

## Behaviour summary (validateHooks)
- configPath = `<cwd>/spec/fspec-hooks.json`.
- try:
  - read+JSON.parse config as HookConfig.
  - For each [event, hooks] in config.hooks: for each hook: hookPath = join(cwd, hook.command); access(hookPath) — if not accessible push `Hook command not found: <hook.command>`.
  - if errors.length>0 → `{ exitCode:1, valid:false, errors: errors.join('\n') }`.
  - else → `{ exitCode:0, valid:true }`.
- catch (any: ENOENT, parse error) → `{ exitCode:1, valid:false, errors:'Failed to load hook configuration' }`.

## CLI registration (registerValidateHooksCommand)
- NO arguments, NO options.
- action: `await validateHooks(options)` — **NOTE: the action DISCARDS the result. It does NOT print anything and does NOT call process.exit.** This is the classic Framing A "broken TS CLI" pattern (action ignores the returned result/exitCode).

## Framing A IMPLICATION
- The TS shell `validate-hooks` produces NO output and always exits 0 (the action awaits validateHooks but never uses exitCode/valid/errors).
- The rich help (`validate-hooks-help.ts`) documents the INTENDED output:
  - All valid: '✓ All hooks are valid'
  - Missing scripts: '✗ Hook validation failed\n\nHook command not found: ...\n...\n\nFix these issues before using hooks.'
  - No hooks configured: 'No hooks configured (nothing to validate)'
  - Exit code 0 = valid, non-zero = errors.
- **Per command-port.md §10 Framing A: the help doc is canon.** The Rust binary should implement what the help PROMISES (print result + meaningful exit code), even though that diverges from the broken TS CLI which prints nothing.
- Architecture note required on RPC-322: "Framing A — TS shell is broken (action discards validateHooks result); Rust implements help-doc canon: print status + exit 0/1."

## Help-doc canon output mapping (Rust target)
- valid (hooks exist, all scripts found) → stdout '✓ All hooks are valid', exit 0.
- invalid (missing scripts) → stdout/stderr '✗ Hook validation failed\n\n<each "Hook command not found: ..." line>\n\nFix these issues before using hooks.', exit 1.
- no hooks configured (config exists, hooks empty/{}) → 'No hooks configured (nothing to validate)', exit 0.
- config missing or unparseable → 'Failed to load hook configuration', exit 1 (the core validateHooks returns this for ENOENT+parse).
- DECISION NEEDED: distinguish "no hooks configured" (config exists with empty hooks) from "config missing". The core validateHooks treats missing config as catch → 'Failed to load hook configuration' exit 1. The help promises 'No hooks configured (nothing to validate)' as a separate example. Need to map: empty hooks object → 'No hooks configured' message; missing/invalid file → 'Failed to load'. Decide rendering layer in the CLI bridge / core run.

## Result shape (core run returns String — for dispatcher)
- Mirror help-doc canon as the rendered string. For dispatcher structured output, return a JSON-or-text per format? TS validateHooks returns `{exitCode, valid, errors?}`. Plan: core run returns the rendered TEXT (help-doc canon lines). The two-front-doors `run(args_json, project_root) -> Result<String>` returns the text; CLI bridge prints it and maps exit code from a leading status. CONSULT supervisor: whether run should return a structured JSON `{exitCode, valid, errors?}` for the dispatcher and text for CLI, OR text-only. Given no --format flag and Framing A, propose: run returns the rendered text; exit-code derivation handled by CLI bridge by inspecting valid/invalid. Simpler: have run return text and a sentinel; but signature is String only. PROPOSAL: core run returns text; non-valid cases are still Ok(String) (not Err) since these are validation results not errors; CLI bridge greps the text to decide exit code OR core returns JSON the bridge parses. RESOLVE with supervisor — likely return JSON `{valid, exitCode, message}` and have the bridge print message + use exitCode (matches RPC-247 list-hooks Framing A precedent).

## Files
- core impl codelet/fspec-core/src/commands/validate_hooks.rs
- help config codelet/fspec-core/src/help/configs/validate_hooks.rs
- CLI bridge codelet/fspec/src/validate_hooks.rs
- core test codelet/fspec-core/tests/validate_hooks.rs
- CLI test codelet/fspec/tests/cli_validate_hooks.rs
- help fixture codelet/fspec/tests/fixtures/help/validate-hooks.txt

## SHARED-FILE REQUESTS to supervisor
- New type maybe: a HookConfig struct OR just parse spec/fspec-hooks.json as raw Value and iterate hooks. Propose raw Value parse in-command (only need hook.command strings). No new shared type needed.
- No new io/ensure helper strictly required: read spec/fspec-hooks.json directly (ENOENT/parse → 'Failed to load'). Mirror RPC-247 list-hooks reading pattern if it exists.
- Supervisor wires canonical.rs, dispatch.rs, help/configs/mod.rs, main.rs Mode+intercept+forward.
- Reference RPC-247 list-hooks (Framing A precedent) for how list-hooks reads spec/fspec-hooks.json and how the dispatcher/CLI split was done.
