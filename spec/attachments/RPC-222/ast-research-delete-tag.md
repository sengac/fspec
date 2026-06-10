# AST Research — `delete-tag` (RPC-222)

## TS source: `src/commands/delete-tag.ts` (207 LOC)

Reads `spec/tags.json`, locates `<tag>` across ALL categories via case-sensitive
`findIndex(t => t.name === tag)`, optionally scans `spec/features/**/*.feature`
for usage references, and either:

- Errors with the usage list (default path) unless `--force` or `--dry-run`
- Emits a warning prefix and proceeds (when `--force` and tag is in use)
- Returns a "Would delete tag X from category Y" message (when `--dry-run`)
- Removes the tag from its category, validates updated JSON (Ajv — skipped in
  Rust port), atomic-writes `spec/tags.json` via `LockedFileManager.transaction`,
  and regenerates `spec/TAGS.md`.

## Args (CLI surface, `registerDeleteTagCommand`, L199-206)

| Position / flag | Type   | Required | Default       | Description                                    |
|-----------------|--------|----------|---------------|------------------------------------------------|
| `<tag>`         | string | yes      | —             | Tag name (e.g. `"@deprecated"`)                |
| `--force`       | bool   | no       | `false`       | Delete even if used in feature files           |
| `--dry-run`     | bool   | no       | `false`       | Report what would change without writing       |

Internal `cwd?: string` (L17) — defaults to `process.cwd()` in the TS
function. The Rust port receives `project_root: &Path` from the dispatcher
or the CLI bridge (see RPC-003 §7 two-front-doors).

## Result shape (`DeleteTagResult`, L20-25)

```ts
{
  success: boolean
  message?: string   // success path: "Successfully deleted tag X from registry"
  warning?: string   // force + in-use:  "Warning: Tag X is still used in N file(s):\n  ..."
  error?: string     // error path: see Errors below
}
```

## Control flow (numbered against TS line numbers)

1. **L35-40 — file-existence gate.** If `spec/tags.json` is absent, return
   `{success:false, error:"spec/tags.json not found"}`. NO auto-create.
2. **L44-45 — load + JSON parse.** Read into `tagsData: Tags`. Failure
   bubbles into the outer `try/catch` (L157-162) which surfaces
   `error.message`.
3. **L48-58 — tag lookup.** Linear scan via `findIndex(t.name === tag)`
   across ALL categories; first match wins. Stores `currentCategory` +
   `tagIndex`.
4. **L60-65 — not-found error.** `Tag {tag} not found in registry`. No
   suggestion list.
5. **L68-92 — pre-delete usage scan (default path).** When neither
   `--force` nor `--dry-run`: glob `spec/features/**/*.feature`, read each
   file, check `fileContent.includes(tag)`. If any matches: return
   `{success:false, error:"Tag {tag} is used in N feature file(s):\n  {csv}\n\nUse --force to delete anyway"}`.
   Errors from glob/read are SWALLOWED — best-effort scan.
6. **L94-117 — force + usage warning.** When `--force` is set, scan the
   same way; if files match, set `warning = "Warning: Tag {tag} is still used in N file(s):\n  {csv}"`.
   Does NOT block deletion.
7. **L119-125 — dry-run early return.** If `--dry-run`, return
   `{success:true, message:"Would delete tag {tag} from category \"{cat.name}\""}`.
   NO disk mutation, NO TAGS.md regeneration.
8. **L128 — splice.** `currentCategory.tags.splice(tagIndex, 1)` removes
   the tag from the in-memory category.
9. **L131-140 — Ajv schema validation.** Skipped in Rust port (upstream
   gates: tag-found + serde shape preserve invariants).
10. **L143-145 — atomic write via `fileManager.transaction()`.** Use
    `write_json_atomic` in Rust.
11. **L148-150 — regenerate TAGS.md.** Plain `writeFile` (no atomic).
12. **L152-156 — success result.** `{success:true, message:"Successfully deleted tag {tag} from registry", warning?}`.
13. **L165-197 — CLI wrapper `deleteTagCommand`.** On failure prints
    `Error: {error}` via `output.error` then `process.exit(1)`. On
    success prints optional warning (no prefix) and `✓ {message}`; when
    NOT dry-run also prints two trailing lines `  Updated: spec/tags.json`
    and `  Regenerated: spec/TAGS.md`.

## Statistics
- `statistics.lastUpdated` is NOT bumped (unlike `register-tag`). Only
  the category array is mutated.

## Auxiliary fields
- All top-level fields (`combinationExamples`, `usageGuidelines`,
  `references`, `statistics`, …) round-trip untouched via
  `#[serde(flatten)] extra` on `TagsData`.

## Glob behaviour
- TS uses `tinyglobby` with `cwd` + `absolute:false`. Rust port can use
  `walkdir` over `spec/features` filtering `*.feature`, OR `glob` crate,
  OR simple recursive scan. Decision: use `walkdir` (already a tree-dep
  of fspec-core? — if not, add hand-rolled recursion to keep deps thin).
- Glob/read failures are SWALLOWED in TS (`catch {}` at L89-91 and
  L114-116). Rust port mirrors: best-effort scan, treat any IO failure
  as "no matches" and proceed with the original branch logic.

## Error messages (verbatim — must round-trip byte-for-byte)
- `"spec/tags.json not found"`
- `"Tag {tag} not found in registry"`
- `"Tag {tag} is used in {N} feature file(s):\n  {file1}\n  {file2}\n\nUse --force to delete anyway"`
- `"Warning: Tag {tag} is still used in {N} file(s):\n  {file1}\n  {file2}"` (NOT an error, success path)
- `"Would delete tag {tag} from category \"{cat.name}\""` (dry-run success)
- `"Successfully deleted tag {tag} from registry"` (delete success)
- Parse failure: TS surfaces `error.message` from `JSON.parse`. Rust port
  emits `FspecCoreError::ParseJson{ file:"tags.json", reason }` —
  consistent with `update-tag` port.

## CLI multi-line success block (TS `deleteTagCommand` L181-190)

```text
[Warning: Tag X is still used in N file(s):
  spec/features/a.feature
  spec/features/b.feature]
✓ Successfully deleted tag X from registry
  Updated: spec/tags.json
  Regenerated: spec/TAGS.md
```

For `--dry-run` the trailing `Updated:` / `Regenerated:` lines are
suppressed (L187).

## Two-front-doors invariant (RPC-003 §7/§11)

Both invocation paths (LLM dispatcher AND CLI shell) must call the
SAME `pub async fn run(args_json:&str, project_root:&Path) -> Result<String, FspecCoreError>`
in `codelet/fspec-core/src/commands/delete_tag.rs`. The CLI bridge in
`codelet/fspec/src/delete_tag.rs` is JSON-marshalling only — clap
positional `<tag>` + `--force` + `--dry-run` → `{"tag":..., "force":..., "dryRun":...}`
JSON → `delete_tag::run(args, project_root).await`.

## Divergences from TS (documented inline in Rust impl)

1. **Ajv schema validation (L131-140) skipped** — upstream gates
   (tag-found + `TagsData` serde shape) collectively enforce invariants.
2. **Outer try/catch flattened.** Parse / IO failures surface as
   `FspecCoreError::ParseJson` / `Io` directly.
3. **`fileManager.transaction()` → `write_json_atomic`.** Behaviourally
   equivalent (atomic rename) but without cross-process write locking.
4. **TAGS.md regenerator.** Reuse the inline minimal generator pattern
   from `register_tag.rs` / `update_tag.rs`; will be promoted to a
   shared `generators` module in a follow-up batch.
5. **Glob/IO failures during usage scan are SWALLOWED** — mirrors TS
   `catch {}` (no error propagation, proceed as "no matches").
6. **Best-effort feature-file scan** — Rust port uses `walkdir`-style
   recursion (or hand-rolled `fs::read_dir`) restricted to files ending
   `.feature` under `spec/features/`. Tag-membership check uses
   `contents.contains(tag)` — same byte-substring test as TS
   `fileContent.includes(tag)`.

## File ownership map

| File                                                            | Action          |
|-----------------------------------------------------------------|-----------------|
| `codelet/fspec-core/src/commands/delete_tag.rs`                 | Replace stub    |
| `codelet/fspec-core/src/help/configs/delete_tag.rs`             | New             |
| `codelet/fspec/src/delete_tag.rs`                               | New CLI bridge  |
| `codelet/fspec-core/tests/delete_tag.rs`                        | New             |
| `codelet/fspec/tests/cli_delete_tag.rs`                         | New             |
| `codelet/fspec/tests/fixtures/help/delete-tag.txt`              | New (captured)  |
| `spec/features/delete-tag-rust-port.feature`                    | New             |
| `spec/features/delete-tag-cli-subcommand.feature`               | New             |
| `spec/attachments/RPC-222/ast-research-delete-tag.md`           | New (this file) |
