# AST Research — `list-tags` Rust Port (RPC-251)

Behaviour-of-record: `src/commands/list-tags.ts` (106 lines).

## TS surface

### `listTags(options: ListTagsOptions = {})` — pure function
Inputs:
- `options.category?: string` — optional category filter (exact match)
- `options.cwd?: string` — project root, defaults to `process.cwd()`

Returns: `Promise<ListTagsResult>` with shape
```ts
{ success: true, categories: Array<{ name, tags: Array<{ tag, description }> }> }
```

Behaviour (lines 27-59):
1. Resolves `cwd` from arg or `process.cwd()`.
2. Calls `ensureTagsFile(cwd)` → loads OR **creates** `spec/tags.json` with the canonical initial 9-category structure (`src/utils/ensure-files.ts:98-191`).
3. Maps each `cat.tags` into `{ tag: t.name, description: t.description }` records.
4. **Sorts** each category's tags alphabetically by tag name using `localeCompare`.
5. **Preserves insertion order of categories** (no category sorting).
6. If `options.category` is provided:
   - Filter to entries with `name === options.category` (exact match).
   - If filtered list is empty → throw `Error("Category not found: <name>. Available categories: <comma-joined>")`.
7. Returns `{ success: true, categories }`.

### `listTagsCommand(options)` — CLI wrapper (lines 61-98)
Inputs:
- `options.category?: string` — from `--category <category>` flag

Behaviour:
1. Calls `listTags({ category })`.
2. **Display loop** (lines 67-83):
   - For each category:
     - `output.log(chalk.bold.blue(`\n${name}`) + chalk.gray(` (${tags.length} tags)`))`
       → After ANSI strip: `\n<name> (<n> tags)` (NB: chalk preserves the literal `\n` inside the bold-blue colored text; once colors are stripped this becomes the literal characters `\n<name>` followed by ` (N tags)`. In Node's `console.log` the leading `\n` becomes a real newline since chalk just wraps with escape codes around the **raw string** — i.e. the leading `\n` is part of the input string before colors are applied, so it IS a real newline.)
     - If `tags.length === 0`: `output.log('  No tags registered')`
     - Else, for each tag: `output.log(`  ${chalk.green(tag.tag)} - ${tag.description}`)`
       → After ANSI strip: `  <tag> - <description>`
   - After all categories: `output.log('')` (blank line).
3. `process.exit(0)` on success.
4. **Error handling** (lines 85-97):
   - If `error.message.includes('tags.json not found')` → `output.error(message)`, log yellow suggestion, `process.exit(2)`.
   - Otherwise → `output.error('Error:', error.message)`, `process.exit(1)`.

   NOTE: `ensureTagsFile` NEVER throws "tags.json not found" — it AUTO-CREATES the file. So in practice the exit-2 path is dead code in the current TS impl (the file always exists after `ensureTagsFile`). For Rust parity we should:
   - Keep the auto-create-via-ensure semantics (TS calls `ensureTagsFile` which is load-or-init).
   - Treat malformed `tags.json` as a parse error (exit 1) — consistent with `list-prefixes` rule [4].
   - Optionally support `--category` filter; on unknown category, return structured error.

### `registerListTagsCommand(program: Command)` — Commander registration (lines 100-105)
```ts
program
  .command('list-tags')
  .description('List all registered tags from TAGS.md')
  .option('--category <category>', 'Filter by category (e.g., "Phase Tags")')
  .action(listTagsCommand);
```

ONE flag: `--category <category>`. NO `--format`. NO `--workspace`. No `-c` short form.

## tags.json shape (from `src/types/tags.ts`)

```ts
interface Tags {
  $schema?: string;
  categories: TagCategory[];
  combinationExamples: TagExample[];
  usageGuidelines: Guidelines;
  addingNewTags: AddingProcess;
  queries: QueryExamples;
  statistics: Statistics;
  validation: ValidationRules;
  references: Reference[];
}

interface TagCategory {
  name: string;
  description: string;
  required: boolean;
  tags: Tag[];
  rule?: string;
}

interface Tag {
  name: string;
  description: string;
  // many other optional fields (usage, scope, examples, etc.) — list-tags ignores them
}
```

`list-tags` only reads `categories[].name` and `categories[].tags[].{name, description}`.

## Default-init structure (from `ensureTagsFile`)

When `spec/tags.json` is missing, it is created with 9 categories in this order, each with empty `tags` array:
1. Phase Tags (required)
2. Component Tags (required)
3. Feature Group Tags (required)
4. Technical Tags
5. Platform Tags
6. Priority Tags
7. Status Tags
8. Testing Tags
9. Automation Tags

## Text-rendering parity table

| TS source                                  | Rust text output (ANSI stripped)            |
|--------------------------------------------|---------------------------------------------|
| `chalk.bold.blue(`\n${name}`) + chalk.gray(` (${N} tags)`)` | `\n<name> (<N> tags)` (real newline at start) |
| `'  No tags registered'`                   | `  No tags registered`                      |
| ``  ${chalk.green(tag)} - ${description}``  | `  <tag> - <description>`                   |
| Final `output.log('')`                     | trailing blank line                         |

Note: each `output.log(...)` call appends `\n` (Node `console.log` behaviour).

## Two-front-doors plan (mirror RPC-248)

### Shared lib (codelet/fspec-core/src/commands/list_tags.rs)
- `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
- Accepts JSON args: `{ "category"?: string, "format"?: "text" | "json" }`
- `format` exposed at the dispatcher path (parity with how `list-prefixes` exposes it); CLI surface does NOT expose `--format` (TS Commander has no such flag).
- Reads `spec/tags.json` via a NEW helper `read_tags_or_init(project_root)` which preserves the TS load-or-init semantics:
  - ENOENT → create file with the canonical 9-category default structure, return default.
  - Parse error → escalate as `FspecCoreError::ParseJson { file: "tags.json", ... }`.
  - (Use the existing `read_or_init_json` helper from `locked_file.rs`.)

### CLI bridge (codelet/fspec/src/list_tags.rs)
- `pub struct CliArgs { pub category: Option<String> }`
- `pub async fn run(args: CliArgs) -> Result<u8>`
- Resolves CWD, marshals `{ "category": ... }` into JSON (omit when None), delegates.
- Exit 0 success / Exit 1 on error.
- Stderr prefix: `Error: <msg>`.

### Main.rs subcommand
```rust
#[command(name = "list-tags", about = "List all registered tags from TAGS.md")]
ListTags {
    #[arg(long, value_name = "CATEGORY")]
    category: Option<String>,
},
```

## New shared infrastructure needed

1. **`codelet/fspec-core/src/types/tags.rs`** (NEW type module) — Rust port of `Tags` / `TagCategory` / `Tag` structs.
   - For RPC-251 we only need fields touched by list-tags + round-trip safety:
     - `categories: Vec<TagCategory>` (insertion-ordered — `Vec` preserves order).
     - `#[serde(flatten)] extra: serde_json::Map<String, Value>` to preserve unused top-level fields (`combinationExamples`, `usageGuidelines`, etc.) when load-or-init writes the file.
     - `TagCategory { name, description, required: bool, tags: Vec<Tag>, rule?: Option<String>, #[serde(flatten)] extra }`.
     - `Tag { name, description, #[serde(flatten)] extra }`.
   - `TagsData::initial()` — constructs the 9-category canonical default to match `ensureTagsFile`.

2. **`codelet/fspec-core/src/io/ensure.rs`** — NEW helper `ensure_tags_file(cwd: &Path) -> Result<TagsData, FspecCoreError>` using `read_or_init_json`. (This is a shared-file addition; we will request it via the supervisor in Phase C.)

   Mirrors `ensure_prefixes_file` / `ensure_work_units_file` exactly.

## Error parity

TS throws "Category not found: X. Available categories: A, B, C". Rust will return `FspecCoreError::InvalidArgs { command: "list-tags", reason: "Category not found: X. Available categories: A, B, C" }` so the dispatcher emits the same message and the CLI surfaces it on stderr prefixed with `Error:`.

## Estimate (provisional)

Story points: **5** (complex)
- New type module (`types/tags.rs`) — moderate (multiple structs + initial())
- New ensure helper (`ensure_tags_file`) — small
- Command impl with category filter + text/json renderer — moderate
- Two test files with ~10 dispatcher scenarios + ~5 CLI scenarios — moderate
- Shared-file additions: `commands/mod.rs` (none needed — list_tags module already declared), `types/mod.rs` (add `pub mod tags`), `canonical.rs` PORTED_COMMANDS entry, `dispatch.rs` run_ported arm + comment-out the stub arm, `main.rs` Mode variant + arm, `cargo_shape.rs` lock-list entry (add `list_tags.rs`).

Higher than list-prefixes (3) because of:
- An additional `--category` flag with error semantics.
- A new shared type module with multiple structs and a default-init function (9 hard-coded categories).
- Insertion-order preservation across categories.
