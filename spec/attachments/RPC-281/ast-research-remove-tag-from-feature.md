# RPC-281 — `remove-tag-from-feature` AST research

## TS source: `src/commands/remove-tag-from-feature.ts`

Signature:
```ts
export async function removeTagFromFeature(
  featureFilePath: string,
  tags: string[],
  options: RemoveTagFromFeatureOptions = {}
): Promise<RemoveTagFromFeatureResult>
```

```ts
interface RemoveTagFromFeatureOptions { cwd?: string }
interface RemoveTagFromFeatureResult {
  success: boolean;
  valid: boolean;
  message?: string;
  error?: string;
}
```

### Observed behaviour

1. `cwd = options.cwd || process.cwd()`. Resolve `filePath = join(cwd, featureFilePath)`.
2. Read file. ENOENT → `{success:false, valid:false, error:"File not found: <relPath>"}`.
3. **Parse Gherkin**. Throw → `error: "Invalid Gherkin syntax: <message>"`. Missing `feature` → `"File does not contain a valid Feature"`.
4. **Existence check**: `existingTags = gherkinDocument.feature.tags.map(t=>t.name)` (includes leading `@`). For each input tag, if NOT in `existingTags` → `error: "Tag <tag> not found on this feature"`. Whole call fails on first miss.
5. **Removal** — line-based mutation, NOT AST round-trip:
   - Split on `\n`.
   - For each line, if `trim().startsWith('@')` AND the FULL trimmed line is in `Set(tags)` → drop the line.
   - Otherwise keep it.
   - Important: a tag line that ALSO contains other tags (multi-tag-on-one-line like `@a @b`) is NOT split. The trimmed full line must equal the input tag exactly. This is an effective parity quirk because most fspec-managed feature files put one tag per line.
6. Re-join with `\n` and validate via another parse pass (purely advisory `valid` boolean). Always write file regardless of `valid`.
7. Returns `{success:true, valid, message: "Removed <tag-list> from <path>"}`. No system reminders.

### CLI surface

```ts
program
  .command('remove-tag-from-feature')
  .description('Remove one or more tags from a feature file')
  .argument('<file>', 'Feature file path …')
  .argument('<tags...>', 'Tag(s) to remove …')
```

- 2 positional args (`<file>`, `<tags...>` variadic), NO flags.
- On error: `output.error('Error:', result.error)` + `process.exit(1)`.
- On success: `output.log("✓ <result.message>")`, exit 0.

### Help (`src/commands/remove-tag-from-feature-help.ts`)

- name: `remove-tag-from-feature`
- description: "Remove one or more tags from a feature file"
- usage: `fspec remove-tag-from-feature <file> <tags...>`
- arguments: file (required), tags... (required)
- no options
- 1 example, 2 related commands, no notes.

### Rust port plan

- Stub at `codelet/fspec-core/src/commands/remove_tag_from_feature.rs` returns `NotYetPorted`.
- Reuse `crate::io::gherkin::parse_feature_lenient` for the Gherkin parse.
- Reuse the same line-based scanner pattern from `add_tag_to_feature` (NOT a full AST round-trip). Whole-line equality match against the trimmed tag.
- File write via `std::fs::write` (TS uses no lock).
- CLI bridge: marshal positional `file`, variadic `tags` array into JSON `{file, tags}`. NO flags.

### Edge cases

- **Multi-tag-on-one-line** is NOT supported by the removal walker — the trimmed line must equal exactly one input tag. The `add-tag-to-feature` insertion path always writes one tag per line so this is fine for fspec-managed files, but a hand-edited `@a @b` line will NOT have `@b` removed.
- **Scenario-level tags** are NOT addressed; the removal walks every line, so a scenario tag whose text exactly matches an input tag WILL be removed. The TS implementation explicitly checks `gherkinDocument.feature.tags` for the existence pre-check (so the error path tolerates only feature-level tags), but the removal pass doesn't differentiate. Documented divergence (we mirror TS).

### Two-front-doors invariant

- Dispatcher entry: `commands::remove_tag_from_feature::run(args_json, project_root)`.
- CLI bridge marshals `CliArgs { file, tags }` into JSON.

### File ownership

OWN:
- `spec/features/remove-tag-from-feature-rust-port.feature`
- `spec/features/remove-tag-from-feature-cli-subcommand.feature`
- `codelet/fspec-core/src/commands/remove_tag_from_feature.rs`
- `codelet/fspec-core/src/help/configs/remove_tag_from_feature.rs`
- `codelet/fspec/src/remove_tag_from_feature.rs`
- `codelet/fspec-core/tests/remove_tag_from_feature.rs`
- `codelet/fspec/tests/cli_remove_tag_from_feature.rs`
- `codelet/fspec/tests/fixtures/help/remove-tag-from-feature.txt`

SHARED (request from supervisor):
- `codelet/fspec-core/src/canonical.rs` (add to PORTED_COMMANDS)
- `codelet/fspec-core/src/dispatch.rs` (wire up run_ported arm)
- `codelet/fspec-core/src/help/configs/mod.rs` (add `pub mod remove_tag_from_feature;`)
- `codelet/fspec/src/main.rs` (add `Mode::RemoveTagFromFeature` variant + intercept arm)
