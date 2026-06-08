# AST Research — `tag-stats` (RPC-310)

## TypeScript Source of Truth

- **Command impl**: `src/commands/tag-stats.ts` (263 lines)
- **Help config**: `src/commands/tag-stats-help.ts`

## TS Behaviour Summary

### Inputs
- `options.cwd?: string` — defaults to `process.cwd()`
- No CLI flags (Commander.js `.action(tagStatsCommand)` with no `.option(...)`).

### Read I/O
1. `spec/tags.json` — optional. Loaded via `loadTagsJson(cwd)`:
   - If file missing → `throw 'tags.json not found'`. Caller catches → `tagsFileFound = false`, `tagsData = null`.
   - If file exists but malformed → `JSON.parse` throws → also swallowed → `tagsFileFound = false`.
2. Feature files: `glob(['spec/features/**/*.feature'], { cwd, absolute: false })`.
   - Parsed with `@cucumber/gherkin` `Parser`.
   - Each feature's `gherkinDocument.feature.tags` array contributes counts (feature-level only — scenario tags are NOT counted).

### Counting Algorithm
- `tagCounts: Map<string, number>` — count of features each tag appears on.
- Parse failure on a file → file added to `invalidFiles`, NOT counted.
- File with no `feature` block (`!gherkinDocument.feature`) → silently skipped, NOT counted as invalid.
- Outer `try/catch` (line 100-102) → any non-parse error (read failure, etc.) also pushes to `invalidFiles`.

### Output Shape (TagStatsResult)
```
{
  success: boolean,
  totalFiles: number,             // count of files matched by glob
  uniqueTags: number,             // tagCounts.size
  totalOccurrences: number,       // sum of all counts
  categories: CategoryStats[],    // [{ name, tags: [{tag, count}] }]
  unusedTags: string[],           // registered tags NOT used in any file (sorted alphabetically)
  tagsFileFound: boolean,
  invalidFiles: string[]
}
```

### Category Construction
**With tags.json:**
- For each `tagsData.categories[]` (declaration order):
  - For each tag in `category.tags[]`:
    - Add `tag.name` to `registeredTags` set.
    - If `tagCounts.get(tag.name) > 0` → push `{tag, count}` to `categoryTags`.
  - Sort `categoryTags` by count **descending**.
  - If non-empty → push `{name: category.name, tags: categoryTags}` to `categories`.
- Collect **unregistered tags** (in `tagCounts` but not in `registeredTags`):
  - Sort by count descending.
  - Push as final category `{name: "Unregistered", tags: [...]}`.

**Without tags.json:**
- All used tags go into a single category: `{name: "Unregistered", tags: <all sorted by count desc>}`.

### Unused Tags
- Only populated when `tagsData` is non-null.
- For each tag in every category: if `!tagCounts.has(tag.name)` → push to `unusedTags`.
- Sort alphabetically (`unusedTags.sort()`).

### Empty `files` array short-circuit
- Returns immediately with `totalFiles=0, uniqueTags=0, totalOccurrences=0, categories=[], unusedTags=[]` and the resolved `tagsFileFound` boolean.
- IMPORTANT: even if `tags.json` HAS unused registered tags, when there are no feature files the function short-circuits BEFORE computing `unusedTags` — those tags are reported as unused via the longer path only.

### Text Renderer (`tagStatsCommand`)
- Header: `\nTag Usage Statistics\n` + 50 `─` chars.
- Three counters: `Total feature files:`, `Unique tags used:`, `Total tag occurrences:`.
- Warning blocks (when applicable):
  - `\n⚠ Warning: spec/tags.json not found` when `!tagsFileFound`.
  - `\n⚠ Warning: <N> file(s) with invalid syntax skipped:` then bulleted `  - <file>`.
- Per-category header: `\n\nTag Counts by Category\n` + 50 `─` chars. Then for each category:
  - `\n<name> (<N> tags)`
  - For each tag: `  <tag padEnd(30)> <count>`.
- Unused tags block: `\n\nUnused Registered Tags\n` + 50 `─` chars, `<N> registered tag(s) not used in any feature file:\n`, then `  <tag>` per line.
- Final `\n` and `process.exit(0)`.
- On uncaught error: stderr `Error: <msg>` then `process.exit(2)`.

## Rust Surface Plan

### Dispatcher entry
- `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
- Args: `{ format?: "text" | "json" }` (dispatcher-only; CLI has no flags).

### Reusable shared infrastructure
- `crate::io::feature_glob::glob_feature_files(project_root)` — already exists, returns sorted relative paths.
  - Need to handle `Err(DirectoryNotFound { path: "spec/features/" })` as "no files" → return empty totals. TS `tinyglobby` returns `[]` on missing dir. **Must NOT escalate** to error.
- `spec/tags.json` reading — must mirror TS bare-catch silent degradation: ENOENT OR malformed → `tagsFileFound=false`, `tagsData=None`. We CANNOT use `ensure_tags_file` (it auto-creates AND escalates parse errors).
  - Read inline with `std::fs::read_to_string` + `serde_json::from_str::<TagsData>` — both errors collapse to `None`.
- Gherkin parsing: there's no full gherkin parser dep in fspec-core. We need to extract **feature-level tags only** — same as the inline scanner in `list_feature_tags::parse_feature_tags`. Reuse that approach (private helper here, or extract a shared helper if supervisor approves).

### Output JSON shape (camelCase)
```
{
  "success": true,
  "totalFiles": N,
  "uniqueTags": N,
  "totalOccurrences": N,
  "categories": [{"name": "...", "tags": [{"tag": "@foo", "count": N}]}],
  "unusedTags": ["@bar"],
  "tagsFileFound": bool,
  "invalidFiles": ["spec/features/x.feature"]
}
```
- Field declaration order matters → typed struct with `#[derive(Serialize)]`.

### Text rendering
Match TS line-for-line (without chalk colours, since dispatcher delivers plain text):
```
\nTag Usage Statistics\n──...50──\nTotal feature files: N\nUnique tags used: N\nTotal tag occurrences: N\n
[\n⚠ Warning: spec/tags.json not found\n]
[\n⚠ Warning: N file(s) with invalid syntax skipped:\n  - file\n  - file\n]
[\n\nTag Counts by Category\n──...50──\n\n<name> (<N> tags)\n  <tag padEnd 30> <count>\n...]
[\n\nUnused Registered Tags\n──...50──\nN registered tag(s) not used in any feature file:\n\n  @tag\n]
\n
```

### CLI bridge
- `codelet/fspec/src/tag_stats.rs` — flag-less, parity with TS Commander.
- `pub async fn run(args: CliArgs) -> Result<u8>`:
  - Resolve project root from CWD.
  - Build `args_json = "{}"`.
  - Call core `run`.
  - Print stdout, exit 0 on success, eprintln `Error:` + exit 1 on failure.
- Exit code: TS uses 2 on uncaught error; here we follow the established RPC port convention of 1 (matches `list-prefixes` / `list-tags` Rust bridges).

### Help fixture
Captured byte-for-byte from `node dist/index.js tag-stats --help` (see EXAMPLES section in TS help config). Will be saved to `codelet/fspec/tests/fixtures/help/tag-stats.txt`.

## Scenario Coverage Map

| TS behaviour | Scenario(s) |
|---|---|
| Empty spec/ (no features dir, no tags.json) | "Returns zero-totals when no features directory exists" |
| Glob returns 0 files | covered above |
| tags.json missing | "Reports tagsFileFound=false when spec/tags.json is missing" |
| tags.json malformed | "Treats malformed tags.json as missing (silent degradation)" |
| Feature with no tags | "Counts only feature-level tags, ignores scenarios with tags" |
| Feature-level tags counted | "Counts each registered tag once per feature file" |
| tags.json + categories projection (desc by count) | "Groups tags by registered category sorted descending by count" |
| Unregistered tags bucket | "Collects unregistered tags into the 'Unregistered' category" |
| unusedTags alphabetical | "Lists registered-but-unused tags alphabetically in unusedTags" |
| Invalid gherkin file | "Records files with malformed Gherkin in invalidFiles without throwing" |
| Text rendering header counters | "Text format prints overall counters" |
| Text rendering warnings | "Text format prints warnings for missing tags.json and invalid files" |
| Text rendering category section | "Text format groups tag counts under category headers" |
| Text rendering unused section | "Text format lists unused registered tags" |
| JSON format pretty 2-space | "JSON format emits two-space indented payload with canonical fields" |
| CLI smoke | scenarios in cli-subcommand feature |
| CLI help byte-for-byte | scenario in cli-subcommand feature |
| CLI clap exposes flag-less subcommand | scenario in cli-subcommand feature |

## Open Questions for Supervisor

1. **Shared gherkin scanner**: `list_feature_tags::parse_feature_tags` is currently private inside its module. Two options:
   - (a) Duplicate the scanner inline here (small, parallel-safe).
   - (b) Promote to `crate::io::gherkin_scan::feature_level_tags(content: &str) -> Option<Vec<String>>` (shared-file; requires supervisor wire-up).
   - **Recommendation**: option (a) for this PR to keep parallel-safe; supervisor can extract later.

2. **`glob_feature_files` ENOENT behaviour**: TS treats missing `spec/features/` as empty list. The Rust helper escalates `DirectoryNotFound`. We will catch that variant inside `tag_stats::run` and convert to empty list. No shared-file change needed.

3. **Exit code on uncaught error**: TS uses `process.exit(2)`. The Rust CLI bridge convention is `exit 1`. I'll use 1 for consistency. Confirm if 2 is desired.
