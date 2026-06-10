# AST Research — `register-tag` (RPC-265)

## TS source
- `src/commands/register-tag.ts` — implementation
- `src/types/tags.ts` — `Tags` / `TagCategory` / `Tag` interfaces
- `src/utils/ensure-files.ts:98-191` — `ensureTagsFile` load-or-init with 9 canonical categories
- `src/generators/tags-md.ts` — `generateTagsMd(tags)` → markdown renderer
- `src/validators/json-schema.ts` — `validateTagsJson(path)` against `src/schemas/tags.schema.json`
- `src/utils/file-manager.ts` — `fileManager.transaction(path, mutator)` atomic JSON write
- `src/schemas/tags.schema.json` — JSON-Schema draft-07 for tags.json

## TS signature
```ts
async function registerTag(
  tag: string,
  category: string,
  description: string,
  options: { cwd?: string } = {}
): Promise<{ success: boolean; message: string; created?: boolean; converted?: boolean }>;
```

## Behavioural rules extracted line-by-line from `register-tag.ts`

1. **CWD resolution** — `options.cwd || process.cwd()`. Rust port takes `project_root: &Path` from dispatcher.
2. **Tag-format validation** (two gates):
   - L37-41: tag MUST start with `@`, else throws `Invalid tag format: "<tag>". Valid format is @lowercase-with-hyphens`.
   - L44-47: if `tag !== tag.toLowerCase()`, normalize to lowercase AND set `converted = true`.
   - L50-54: post-normalisation regex `/^@[a-z0-9-]+$/` MUST match; else throws same `Invalid tag format` error using the ORIGINAL `tag` (not the normalized one).
3. **Load tags.json via `ensureTagsFile`** (L57) — load-or-init with canonical 9-category default. Errors escalate (parse error wrapped).
   - `created` is HARD-CODED to `false` (L58) — `ensureTagsFile` handles file creation silently; the CLI never reports a "Created tags.json" line based on this flag. (Note: the CLI surface at L147-149 says "Created new tags.json and TAGS.md" if `created` is true, but `created` is ALWAYS false today — dead branch.)
4. **Duplicate detection** (L61-68) — iterate ALL categories, fail with `Tag <normalized> is already registered in <categoryName>` if `tag.name === normalizedTag` anywhere.
5. **Category match** (L71-80) — case-INSENSITIVE match: `c.name.toLowerCase() === category.toLowerCase()`. On miss: `Invalid category: "<category>". Available categories: <comma-space-joined insertion-order names>`.
6. **Tag insertion + sort** (L83-91):
   - push `{ name: normalizedTag, description }` (NO auxiliary fields)
   - sort the target category's tags alphabetically by `name.localeCompare(b.name)`
   - tag is added to the matched category, NOT necessarily the one whose `.name` was spelled exactly the same.
7. **Statistics update** (L94) — `tagsData.statistics.lastUpdated = new Date().toISOString()`. This requires the `statistics` object to round-trip via the `extra` map and be mutable — Rust port must read+set this field on the `extra` map.
8. **Rollback snapshot** (L97) — second call to `ensureTagsFile(cwd)` is captured as the pre-write state. (This is a re-read AFTER the in-memory mutation but BEFORE the atomic write — and TS captures by reference, so this is actually a same-shape snapshot of disk state.)
9. **Atomic write** (L100-102) — `fileManager.transaction(tagsJsonPath, async fileData => { Object.assign(fileData, tagsData); })`. Rust port uses `io::locked_file::write_json_atomic`.
10. **Schema validation post-write** (L105-110) — `validateTagsJson(tagsJsonPath)` against `tags.schema.json` (Ajv). On failure throws `Updated tags.json failed schema validation: <errors>`. Rust port can SKIP runtime schema validation IF the in-memory mutation already obeys the schema's `pattern: ^@[a-z0-9-]+$` (which we enforce upstream) — but to preserve the error surface we should still validate via a minimal in-Rust check or skip and trust upstream gates. **Decision: skip schema validation in Rust port** — the pattern is already enforced at step 2, duplicate gate at step 4, and the `statistics`/aux fields round-trip via `extra`. Document the decision in architecture notes.
11. **TAGS.md regeneration** (L113-125):
    - call `generateTagsMd(tagsData)` → markdown string
    - write `spec/TAGS.md` via plain `writeFile` (NOT atomic JSON path)
    - on error: ROLLBACK tags.json to `originalTagsData` via another `transaction`, then throw `Failed to regenerate TAGS.md - changes rolled back: <error>`
    - Rust port: must port `generateTagsMd` (long renderer — ~380 LOC TS). For the FIRST cut, port the minimum subset exercised by the canonical-default tags.json (the auxiliary sections degrade gracefully when their input arrays are empty). **Decision: port a minimal `generate_tags_md` covering the Tag Categories, Last Updated, and auto-generation warning header — gracefully omit empty optional sections.** This matches TS behaviour for newly-created tags.json (all aux sections empty).
12. **Success message** (L127-129):
    - if converted: `Successfully registered <normalized> (converted from <input>) in <category>`
    - else: `Successfully registered <normalized> in <category>`
13. **Result shape**: `{ success: true, message, created: false, converted: boolean }`.

## CLI wrapper (`registerTagCommand` L139-167)
- Calls `registerTag(...)`.
- If `result.created` → `output.log('Created new tags.json and TAGS.md')` (DEAD: always false).
- If `result.converted` → yellow note: `Note: Tag converted to lowercase: <input> → <input.toLowerCase()>`.
- Always prints: `✓ <message>`, then `  Updated: spec/tags.json`, then `  Regenerated: spec/TAGS.md`.
- Exit code 0 on success, 1 on caught error (prefixed `Error:`).

## Commander.js registration (L169-177)
```
program
  .command('register-tag')
  .argument('<tag>', ...)
  .argument('<category>', ...)
  .argument('<description>', ...)
```
Three POSITIONAL arguments, no flags.

## Rust port plan

### `commands/register_tag.rs` shape
- `RegisterTagArgs { tag, category, description, format? }` (camelCase via serde).
  - `format` is dispatcher-only, accepts `"text"|"json"`; default text.
- `pub async fn run(args_json, project_root) -> Result<String, FspecCoreError>`
- Returns rendered text (parity with CLI) or 2-space JSON (`{"success":true,"message":...,"converted":...,"category":...,"tag":...}`)
- Mutation flow:
  1. Parse args; validate tag format (both gates).
  2. Read tags.json via `ensure_tags_file` (load-or-init).
  3. Iterate categories — duplicate detection.
  4. Find target category by case-insensitive name match.
  5. Push new `Tag { name, description, extra: empty map }`.
  6. Sort the matched category's tags alphabetically by `name` (byte cmp — ASCII tags only).
  7. Update `extra["statistics"]["lastUpdated"]` to ISO-8601-now (use the existing `iso8601_now` helper exposed via a tiny pub fn — OR call it locally with the same algorithm).
  8. `write_json_atomic(spec/tags.json, &tags_data)`.
  9. Generate TAGS.md via NEW helper `generators::tags_md::generate(&tags_data) -> String` covering header + Tag Categories + Last Updated (canonical-default coverage). Rollback NOT implemented in the Rust port — if markdown write fails, escalate as `Io { command: "register-tag", ... }` (architecture note: TS rollback is best-effort safety net; Rust port's `write_json_atomic` + ordered write to TAGS.md after is acceptable for v1, but DOCUMENT in arch note).
  10. Write `spec/TAGS.md` via `std::fs::write` (markdown, NOT atomic).
  11. Render success line.

### `cli/src/register_tag.rs` shape
- `CliArgs { tag: String, category: String, description: String }`
- Marshal to `{"tag":...,"category":...,"description":...}` JSON; delegate to `register_tag::run`.
- Print on success: result text (which already contains the `✓` line + status). Exit 0/1.

### Files to create
1. `codelet/fspec-core/src/commands/register_tag.rs` (replace stub) — ~250 LOC
2. `codelet/fspec-core/src/generators/mod.rs` + `tags_md.rs` (NEW module) — ~80 LOC for minimum subset
3. `codelet/fspec-core/src/help/configs/register_tag.rs` — ~30 LOC
4. `codelet/fspec-core/tests/register_tag.rs` — dispatcher tests — ~300 LOC
5. `codelet/fspec/src/register_tag.rs` — CLI bridge — ~100 LOC
6. `codelet/fspec/tests/cli_register_tag.rs` — CLI shell tests — ~250 LOC
7. `codelet/fspec/tests/fixtures/help/register-tag.txt` — captured fixture

### Shared-file CHANGE REQUESTS for SUPERVISOR
- `codelet/fspec-core/src/io/locked_file.rs` already exposes `write_json_atomic` — **no change needed**.
- `codelet/fspec-core/src/io/ensure.rs` already exposes `ensure_tags_file` — **no change needed**.
- `codelet/fspec-core/src/types/tags.rs` exposes mutable `Tag` + `TagCategory` — **no change needed**.
- `codelet/fspec-core/src/generators/mod.rs` does NOT exist yet — **NEW module needed**. Worker can add it under `src/generators/` (NOT in `src/commands/mod.rs` ownership scope). **Need supervisor confirmation** whether `generators::` module is a worker-owned subtree or supervisor-owned. Pending that decision, I will keep `tags_md` rendering as a private helper inline in `register_tag.rs` (and later promote when `delete_tag` / `update_tag` ports happen — they will need it too, so consolidation is desirable for batch 7).
- `iso8601_now` is private in `io/ensure.rs`. I will duplicate the algorithm inline (small) rather than ask supervisor to make it public — keeps `io/ensure.rs` untouched.
