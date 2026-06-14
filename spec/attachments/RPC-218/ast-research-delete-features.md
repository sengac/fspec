# AST Research — delete-features (RPC-218)

## TS source: `src/commands/delete-features-by-tag.ts` (CLI name = `delete-features`)

Exported API:
- `deleteFeaturesByTag(options: { tags: string[], dryRun?, cwd? }): Promise<{ success, deletedCount, message?, files?, error? }>`
- `deleteFeaturesByTagCommand(options: { tag?: string|string[], dryRun? })` — Commander action
- `registerDeleteFeaturesCommand(program)` — `delete-features --tag <tag> [--dry-run]` (repeatable --tag → array, AND logic)

## Behaviour walk-through

1. **Validate tags** (30-36): empty/no tags → `success=false, deletedCount=0, error="At least one --tag is required"`.
2. **Glob** (40-51): `spec/features/**/*.feature` relative paths. Empty → `success=true, deletedCount=0, message="No feature files found"`.
3. **Match loop** (54-85): for each file, read+parse (lenient). Invalid syntax → skip (`continue`). No feature → skip. Feature-level tags = `gherkinDocument.feature.tags.map(t => t.name)` (TS tag names INCLUDE leading `@`). AND logic: `tags.every(tag => featureTags.includes(tag))`. If all present → push to `matchingFiles`.
4. **No matches** (88-94): `success=true, deletedCount=0, message="No feature files found matching tags"`.
5. **Dry run** (97-104): `success=true, deletedCount=matchingFiles.length, message="Would delete N feature file(s)", files=matchingFiles`.
6. **Delete** (107-117): unlink each matching file → `success=true, deletedCount=N, message="Deleted N feature file(s)", files=matchingFiles`.
7. **Catch** (118-124): `success=false, deletedCount=0, error=<msg>`.

### CLI rendering (`deleteFeaturesByTagCommand`, 127-176)
- normalise `tag` (string|string[]) → array.
- !success → `output.error('Error:', error)`, exit 1.
- dryRun && files → prints:
  ```
  Dry run mode - no files modified
  <cyan>\nWould delete N feature file(s):\n</cyan>
    - <file>
  ```
- else if files.length>0 → `✓ <message>\n\nDeleted files:\n  - <file>`
- else → prints `message` (e.g. "No feature files found", "No feature files found matching tags").
- exit 0.

## Rust mapping

- Reuse `crate::io::feature_glob::glob_feature_files` for the recursive walk + relative forward-slash paths (alphabetical sort — TS tinyglobby order may differ but list comparison uses `.every`; matchingFiles order only matters for rendering. NOTE potential ordering divergence — glob_feature_files sorts alphabetically; tinyglobby default order is also typically directory-walk. Acceptable: render in sorted order).
- Feature-level tags from gherkin-0.16: `feature.tags` strips leading `@`; re-prepend `@` (same as add_tag_to_feature does at line 128) before comparing to caller tags which carry `@`.
- Missing `spec/features/` dir: `glob_feature_files` returns `DirectoryNotFound`. TS `glob` returns `[]` (no dir → empty). To match TS "No feature files found", catch DirectoryNotFound → treat as empty list. **Decision: map DirectoryNotFound → empty Vec** to preserve TS message parity.
- Parse failures skipped (lenient parse Err → continue).
- Output envelope JSON for dispatcher: `{ success, deletedCount, message?, files?, error? }`.

## Shared-file needs
- None new. dispatch arm `commands::delete_features::run(args_json)` → 2-arg `(args_json, project_root)` (supervisor wiring).
- Possible question: glob ordering parity (alphabetical vs tinyglobby). Flag to supervisor but proceed with sorted.
