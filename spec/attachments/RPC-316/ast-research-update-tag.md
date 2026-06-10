# AST research — `update-tag` (RPC-316)

Source: `src/commands/update-tag.ts` (173 lines)
Target: `codelet/fspec-core/src/commands/update_tag.rs` (currently stub)

## Surface

- Commander.js registration (`update-tag.ts:165-172`):
  - subcommand name: `update-tag`
  - description: `"Update an existing tag in TAGS.md registry"`
  - positional `<tag>`: tag name (e.g., `"@critical"`)
  - option `--category <category>`: new category name
  - option `--description <description>`: new description
  - action: `updateTagCommand(tag, options)` → calls `updateTag({tag, category, description, cwd})`

## Algorithm (TS line-by-line)

1. **L32-38** — Validate at least one of `category` or `description` is provided. Otherwise return `{success:false, error:'No updates specified. Use --category and/or --description'}`.
2. **L41-46** — Check `existsSync(tagsJsonPath)`. If missing return `{success:false, error:'spec/tags.json not found'}` (NO auto-create — divergence from register-tag).
3. **L49-51** — Read & parse `spec/tags.json` (JSON.parse). Parse failure caught by outer try/catch → `{success:false, error: error.message}` (raw `SyntaxError` message).
4. **L53-64** — Locate `(currentCategory, tagIndex)` by linear-scanning `tagsData.categories` and using `cat.tags.findIndex(t => t.name === tag)`. **Exact-match, case-sensitive** on the tag-name string.
5. **L66-71** — If not found in any category, return `{success:false, error:'Tag {tag} not found in registry'}`. (No suggestion list.)
6. **L73** — Capture `currentTag = currentCategory.tags[tagIndex]` (reference, not clone — in TS).
7. **L76-103** — Branch on whether category is being changed:
   - **L76**: `if (category && category !== currentCategory.name)` — i.e. `--category` provided AND not equal to current name (case-sensitive `!==`).
     - **L77-85**: Find `targetCategory = tagsData.categories.find(c => c.name === category)`. **Case-sensitive** match here (unlike register-tag's case-insensitive lookup). If missing, return `{success:false, error:'Invalid category: {category}. Available categories: {csv}'}`.
     - **L88**: Splice the tag out of `currentCategory.tags` at `tagIndex`.
     - **L91-94**: Push new `{name: tag, description: description || currentTag.description}` into `targetCategory.tags`. **Uses provided description if given; otherwise reuses original description.**
     - **L97**: Sort target category's tags alphabetically via `localeCompare`.
   - **L98-103** else: tag stays in same category. If `description` provided, mutate `currentTag.description = description`. **NO re-sort** (description-only update preserves order).
8. **L106-108** — Atomic write via `fileManager.transaction(tagsJsonPath, async fileData => { Object.assign(fileData, tagsData); })`. The `fileManager.transaction` helper acquires a lock, reads current, mutates, writes atomically.
9. **L111-117** — Ajv schema validation against `src/schemas/tags.schema.json`. If invalid, return `{success:false, error:'Updated tags.json failed schema validation: {errors.join(", ")}'}`. (Rust port omits Ajv — upstream gates enforce the invariants. Documented divergence.)
10. **L120-122** — Regenerate `spec/TAGS.md` via `generateTagsMd(tagsData)` + `writeFile`.
11. **L124-127** — Return `{success:true, message:'Successfully updated {tag}'}`.
12. **L128-133** — Outer catch wraps `error.message` into `{success:false, error: ...}` so JSON-parse failure / fs IO failure / etc. all surface as a structured error rather than a crash.

## CLI wrapper (`updateTagCommand` L136-162)

- If `result.success === false`: `output.error('Error:', result.error); process.exit(1)`.
- Else: `output.log("✓ {message}"); output.log("  Updated: spec/tags.json"); output.log("  Regenerated: spec/TAGS.md"); process.exit(0)`.

## Rules extracted

R1. At-least-one-update gate: `--category` and/or `--description` required, else "No updates specified" error.
R2. tags.json must exist (NO auto-create; opposite of register-tag).
R3. Tag lookup is **exact-match, case-sensitive** across ALL categories.
R4. Tag-not-found error: `Tag {tag} not found in registry`.
R5. Category lookup for `--category` argument is **case-sensitive** (`c.name === category`).
R6. Unknown-category error: `Invalid category: {category}. Available categories: {csv-of-all-category-names-in-insertion-order}`.
R7. Cross-category move: splice from current, push to target, alphabetically sort target after insert. Use new description if provided; else reuse original.
R8. Description-only update (same category): mutate in place, no sort.
R9. Auxiliary top-level fields (`statistics`, `usageGuidelines`, `combinationExamples`, …) round-trip via `#[serde(flatten)] extra` — UNTOUCHED on update (unlike register-tag which bumps statistics.lastUpdated).
R10. **statistics.lastUpdated is NOT bumped by update-tag** (TS code has no `lastUpdated` mutation). Verified.
R11. Atomic write via `write_json_atomic`.
R12. TAGS.md regenerated from in-memory data after JSON write.
R13. CLI surface: positional `<tag>` + `--category` flag + `--description` flag (BOTH optional).
R14. Success message: `Successfully updated {tag}` (single line + Updated/Regenerated trailing lines).

## File layout (Rust port)

- `codelet/fspec-core/src/commands/update_tag.rs` — core impl (replace stub). Will share `generate_tags_md` and `iso8601_now` from `register_tag` — but per supervisor-only rule on shared files, we inline a local copy to avoid touching `commands/mod.rs`. Keeps `generate_tags_md` consistent shape; statistics.lastUpdated NOT bumped.
- `codelet/fspec-core/src/help/configs/update_tag.rs` — help config.
- `codelet/fspec/src/update_tag.rs` — CLI bridge. Two clap flags `--category`, `--description` → JSON object with optional fields.
- `codelet/fspec-core/tests/update_tag.rs` — dispatcher test, 1 fn per scenario.
- `codelet/fspec/tests/cli_update_tag.rs` — CLI shell test.
- `codelet/fspec/tests/fixtures/help/update-tag.txt` — captured help fixture.

## AST patterns probed in TS source

```typescript
// L25-28 — async fn signature
export async function updateTag(options: UpdateTagOptions): Promise<UpdateTagResult>

// L57-64 — linear category scan with findIndex
for (const cat of tagsData.categories) {
  const idx = cat.tags.findIndex(t => t.name === tag);
  if (idx !== -1) { currentCategory = cat; tagIndex = idx; break; }
}

// L76 — category-change branch predicate
if (category && category !== currentCategory.name) { ... }

// L97 — alphabetical sort after cross-category insert
targetCategory.tags.sort((a, b) => a.name.localeCompare(b.name));

// L100-102 — description-only branch
if (description) { currentTag.description = description; }
```

## Divergences from TS (documented in code comments)

- Ajv schema validation omitted — upstream gates collectively enforce schema invariants.
- Outer try/catch flattened to direct `FspecCoreError` returns for parse / IO failures.
- TS uses `fileManager.transaction` (per-file lock); Rust port uses `io::locked_file::write_json_atomic` which is the established equivalent.
- Inline minimal `generate_tags_md` helper (same as register-tag port) — promotes to shared `generators` module deferred to a later batch.
