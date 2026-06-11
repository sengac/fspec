# RPC-193 — `add-tag-to-feature` AST research

## TS source: `src/commands/add-tag-to-feature.ts`

Signature:
```ts
export async function addTagToFeature(
  featureFilePath: string,
  tags: string[],
  options: AddTagToFeatureOptions = {}
): Promise<AddTagToFeatureResult>
```

```ts
interface AddTagToFeatureOptions {
  cwd?: string;
  validateRegistry?: boolean;
}
interface AddTagToFeatureResult {
  success: boolean;
  valid: boolean;
  message?: string;
  error?: string;
  systemReminders?: string[];
  systemReminder?: string;
}
```

### Observed behaviour

1. `cwd = options.cwd || process.cwd()`. Project root resolution.
2. Resolve `filePath = join(cwd, featureFilePath)`.
3. Read file. ENOENT → `{success:false, valid:false, error:"File not found: <relPath>"}`.
4. **Tag format validation** loops over `tags`:
   - Must start with `@` → `error: "Invalid tag format. Tags must start with @"`.
   - Work-unit form `@[A-Z]{2,6}-\d+` OR regular form `^@[a-z0-9-#]+$` (the regex includes literal `#`).
   - Otherwise → `error: "Invalid tag format. Regular tags must use lowercase-with-hyphens, work unit tags must match @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001)"`.
5. **Parse Gherkin** with `@cucumber/gherkin`. Throw → `error: "Invalid Gherkin syntax: <message>"`. Missing `feature` → `"File does not contain a valid Feature"`.
6. **Duplicate check**: Build `existingTags = gherkinDocument.feature.tags.map(t=>t.name)`. For each input tag, if `existingTags.includes(tag)` → `error: "Tag <tag> already exists on this feature"`. Whole call fails on first duplicate.
7. **Registry validation (only when `validateRegistry=true`)**:
   - Read `spec/tags.json` JSON.
   - Build flat `Set<string>` from `categories[].tags[].name`.
   - For each tag: if not in set → `error: "Tag <tag> is not registered in spec/tags.json"`.
   - If `readFile` throws or JSON.parse throws → `error: "Failed to validate against registry: <message>"`.
8. **Tag insertion** — line-based mutation (NOT AST round-trip):
   - Split content on `\n`. Find first line starting with `Feature:` after trim.
   - Walk backwards from `featureLineIndex - 1`. Track an `insertIndex`:
     - If line is not empty AND not starting with `@` → `insertIndex = i + 1`, break.
     - If `i === 0` and we never hit non-tag → `insertIndex = 0`.
   - **Quirk**: When `insertIndex === featureLineIndex` and `existingTags.length > 0`, re-walk from `featureLineIndex - 1` and set `insertIndex = i + 1` at the last `@` line. This means new tags are appended AFTER existing tags rather than before them.
   - `lines.splice(insertIndex, 0, ...tags)` — each tag occupies its own line.
9. Validate result with another parse pass to compute `valid` boolean. Always write file regardless of `valid`.
10. **System reminder pass** (only when `validateRegistry=false`):
    - Re-read `spec/tags.json`. For each NEW tag that is not a work-unit tag and not in the registered set, call `getUnregisteredTagReminder(tag, false)`. Silently skip on tags.json failure.
11. **Missing-required-tags pass** (always):
    - `allTags = [...existingTags, ...newTags]`.
    - `hasComponentTag` = any of `@cli, @parser, @validator, @formatter, @generator, @file-ops`.
    - `hasFeatureGroupTag` = any of `@feature-management, @tag-management, @validation, @querying, @work-unit-management, @example-mapping, @metrics, @dependency-management, @workflow`.
    - Push missing categories ("component", "feature-group") into `missingTags`. Call `getMissingRequiredTagsReminder(featureFilePath, missingTags)`.
12. Returns `{success:true, valid, message: "Added <tag-list> to <path>", systemReminders?, systemReminder?}`.

### CLI surface (`addTagToFeatureCommand` + `registerAddTagToFeatureCommand`)

```ts
program
  .command('add-tag-to-feature')
  .description('Add one or more tags to a feature file')
  .argument('<file>', 'Feature file path …')
  .argument('<tags...>', 'Tag(s) to add …')
  .option('--validate-registry', 'Validate tags against spec/tags.json')
```

- 2 positional args (`<file>`, `<tags...>` variadic), 1 long flag `--validate-registry`.
- On error: `output.error('Error:', result.error)` + `process.exit(1)`.
- On success: `output.log("✓ <result.message>")`, then if `systemReminder` exists `output.log('\n' + result.systemReminder)`, then exit 0.

### Help (`src/commands/add-tag-to-feature-help.ts`)

- name: `add-tag-to-feature`
- description: "Add one or more tags to a feature file (feature-level tags)"
- usage: `fspec add-tag-to-feature <file> <tags...> [options]`
- arguments: file (required), tags... (required)
- options: `--validate-registry`
- 3 examples, 4 related commands, 3 notes.

### Rust port plan

- Stub at `codelet/fspec-core/src/commands/add_tag_to_feature.rs` returns `NotYetPorted`.
- Read file with `std::fs::read_to_string`; map `ErrorKind::NotFound` to the structured `{success:false, error:"File not found: <relPath>"}`.
- Validate each tag with two regexes (work-unit + regular). Strip `@` AFTER validation — for the FILE INSERTION we keep the `@` form because we write literal text into the file (the line-based mutation never round-trips through the gherkin AST).
- Parse with `crate::io::gherkin::parse_feature_lenient` to retrieve `feature.tags` (bare, no `@`). Build `existing_tags_with_at: Vec<String>` by re-prefixing each entry with `@`.
- For the duplicate check compare with `@` prefix to mirror the TS behaviour (it compares `t.name` which includes the leading `@` from `@cucumber/gherkin`).
- Insertion: replicate the exact line-based algorithm with split/join. Insert with NO trailing newline (each tag occupies its own line; splice maintains the `\n` join).
- Validate result with a second parse pass; only the `valid` boolean is affected, no error escalation.
- Atomic write via standard `std::fs::write` (TS uses plain `writeFile`, no lock).
- System reminders are emitted as a single consolidated `<system-reminder>` block embedded in the dispatcher output (text) or as part of the JSON shape.
- Registry validation re-uses `crate::types::tags::TagsData` deserialization; missing/malformed file in the `validateRegistry=true` path escalates as `error: "Failed to validate against registry: <message>"`.
- CLI bridge: marshal positional `file`, variadic `tags` array, optional `--validate-registry` bool into JSON `{file, tags, validateRegistry?}`.

### Edge cases

- Tags-as-variadic with **zero** tags is rejected by Commander (`<tags...>` requires at least one). The Rust clap variant uses `num_args = 1..` likewise.
- The regex `^@[a-z0-9-#]+$` accepts `#` literally (TS regex char class). For Rust port reproduce the same allowed alphabet exactly.
- The order in which we walk inserts when there are NO pre-existing tags: insertIndex starts at `featureLineIndex` and walks back; if every preceding line is empty or a tag, eventually `i === 0` clamp triggers, so we insert at position 0 — directly before the Feature header. This is the typical "fresh feature file" path.
- A feature file beginning with description/comment lines and NO existing tags: `insertIndex = (last non-tag, non-empty line index) + 1`. Tags are inserted directly between the last text line and the `Feature:` line.

### Two-front-doors invariant

- Dispatcher entry: `commands::add_tag_to_feature::run(args_json, project_root)`.
- CLI bridge: thin façade that marshals `CliArgs { file, tags, validate_registry }` into JSON.
- Both paths converge.

### File ownership

OWN:
- `spec/features/add-tag-to-feature-rust-port.feature`
- `spec/features/add-tag-to-feature-cli-subcommand.feature`
- `codelet/fspec-core/src/commands/add_tag_to_feature.rs`
- `codelet/fspec-core/src/help/configs/add_tag_to_feature.rs`
- `codelet/fspec/src/add_tag_to_feature.rs`
- `codelet/fspec-core/tests/add_tag_to_feature.rs`
- `codelet/fspec/tests/cli_add_tag_to_feature.rs`
- `codelet/fspec/tests/fixtures/help/add-tag-to-feature.txt`

SHARED (request from supervisor):
- `codelet/fspec-core/src/canonical.rs` (add to PORTED_COMMANDS)
- `codelet/fspec-core/src/dispatch.rs` (wire up run_ported arm)
- `codelet/fspec-core/src/commands/mod.rs` (already declares add_tag_to_feature — no change)
- `codelet/fspec-core/src/help/configs/mod.rs` (add `pub mod add_tag_to_feature;`)
- `codelet/fspec/src/main.rs` (add `Mode::AddTagToFeature` variant + intercept arm)
- `Cargo.toml` if any new deps (none expected — reuse gherkin via crate::io::gherkin).
