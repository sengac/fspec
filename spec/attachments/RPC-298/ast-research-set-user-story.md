# AST Research — set-user-story (RPC-298)

## TS Source

- Primary: `src/commands/set-user-story.ts` (81 lines)
- Help: `src/commands/set-user-story-help.ts`

## Public exports

```ts
interface SetUserStoryOptions { role, action, benefit, cwd? }
async function setUserStory(workUnitId, options)
async function setUserStoryCommand(workUnitId, options)
function registerSetUserStoryCommand(program)
```

## Behaviour observations

1. Reads via `ensureWorkUnitsFile(cwd)` (auto-creates `spec/work-units.json` if missing).
2. Errors with `Work unit '<id>' does not exist` when the work unit is missing.
3. Constructs `UserStory = { role, action, benefit }` from the supplied flags.
4. Assigns `data.workUnits[workUnitId].userStory = userStory` (replaces any prior story).
5. Updates `workUnit.updatedAt = ISO`.
6. Updates `data.meta.lastUpdated = ISO` when present.
7. Persists via `fileManager.transaction(workUnitsPath, ...)` (atomic write).
8. CLI wrapper prints success block:
   - `✓ User story set for <workUnitId>`
   - `  As a <role>`
   - `  I want to <action>`
   - `  So that <benefit>`
9. CLI wrapper exits 0 on success.
10. CLI wrapper catches Error → writes `Error: <msg>` to stderr, exits 1.
11. All three flags (`--role`, `--action`, `--benefit`) are required by Commander; missing them produces Commander's standard "missing required option" error.

## Field-order on disk

`userStory` object preserves the literal `{role, action, benefit}` order (TS object-literal insertion order = serde_json::Map insertion order with `preserve_order` feature).

## Rust port plan

- New core file: `codelet/fspec-core/src/commands/set_user_story.rs`.
- Use `ensure_work_units_file` + `write_json_atomic` + `iso8601_now()`.
- Build a `serde_json::Map` with keys inserted in the literal order `role, action, benefit` and store under `extra["userStory"]` of the matched `WorkUnit`.
- Update `updated_at` on the work unit + `meta.lastUpdated` on the top-level data.
- Return rendered success block as the dispatcher data string.

## Help fixture

Captured via `node dist/index.js set-user-story --help` — standard CommandHelpConfig with `whenToUse`, `whenNotToUse`, `prerequisites`, etc.
