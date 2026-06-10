# AST research — `create-prefix` (RPC-213)

## TS source map

- `src/commands/create-prefix.ts` (87 LOC)
- `src/commands/create-prefix-help.ts` (42 LOC)

### Interfaces

```ts
interface Prefix {
  prefix: string;
  description: string;
  epicId?: string;
  createdAt: string;
}
interface PrefixesData {
  prefixes: Record<string, Prefix>;
}
```

### Validation

- Regex: `PREFIX_REGEX = /^[A-Z]{2,6}$/`.
- Rejects: lowercase (`auth`), too short (`A`), too long (`ABCDEFG`), digits (`AB1`), special chars (`AU-TH`).
- Throws `'Prefix must be 2-6 uppercase letters (e.g., AUTH, DASH)'` BEFORE any read.

### Side effects

1. `ensurePrefixesFile(cwd)` — load OR auto-create `spec/prefixes.json`. (Distinct from `list-prefixes`'s read-or-empty!)
2. If `data.prefixes[options.prefix]` truthy → throw `'Prefix ${options.prefix} already exists'`.
3. Build `newPrefix = { prefix, description, createdAt: new Date().toISOString() }`.
4. `data.prefixes[options.prefix] = newPrefix`.
5. `fileManager.transaction(prefixesFile, ...)` → atomic write of updated object.
6. Returns `{ success: true }`.

### Error wrapping

```ts
catch (error: unknown) {
  if (error instanceof Error) {
    throw new Error(`Failed to create prefix: ${error.message}`);
  }
  throw error;
}
```

Net result: validation errors surface as `'Failed to create prefix: Prefix must be ...'`.
Duplicate errors surface as `'Failed to create prefix: Prefix AUTH already exists'`.

### Commander.js registration

```ts
program
  .command('create-prefix')
  .description('Register a new work unit prefix')
  .argument('<prefix>',  'Prefix code (2-6 uppercase letters, e.g., AUTH, DASH)')
  .argument('<description>', 'Prefix description')
  .action(async (prefix, description) => {
    try {
      await createPrefix({ prefix, description });
      output.log(`✓ Prefix ${prefix} created successfully`);
    } catch (error: any) {
      output.error('✗ Failed to create prefix:', error.message);
      process.exit(1);
    }
  });
```

NO `.option(...)` calls. NO `--format`. Stdout success line:
`✓ Prefix AUTH created successfully`.

Stderr error line (from `output.error`, which prefixes with chalk.red and joins args):
`✗ Failed to create prefix: <message>`.

Exit: 0 on success, 1 on any error.

### Help config (`create-prefix-help.ts`)

```ts
{
  name: 'create-prefix',
  description: 'Register a new work unit prefix for organizing work by component or area',
  usage: 'fspec create-prefix <prefix> <description>',
  whenToUse: 'Use when starting work on a new component or area that needs its own work unit namespace.',
  arguments: [
    { name: 'prefix',      description: 'Prefix code (e.g., AUTH, UI, API) - uppercase, short', required: true },
    { name: 'description', description: 'Description of what this prefix represents',         required: true },
  ],
  examples: [{
    command: 'fspec create-prefix AUTH "Authentication features"',
    description: 'Register new prefix',
    output: '✓ Created prefix AUTH\n  Description: Authentication features',
  }],
  relatedCommands: ['list-prefixes','update-prefix','create-story','create-bug','create-task'],
  notes: ['Prefix must be uppercase','Required before creating work units with that prefix'],
}
```

## Rust port plan

### Shared infrastructure needed

- **`io::ensure::ensure_prefixes_file`** — already exists (returns `PrefixesData` with auto-create).
- **`io::locked_file::write_json_atomic`** — already exists. Use this for the mutating write (LOCK-002 equivalent).
- **`types::prefix::Prefix`** — exists; we add `epicId: Option<String>` round-trip via `extra` map (Prefix struct already has `created_at`).
- **`types::work_unit::PrefixesData`** — keyed `IndexMap<String, Prefix>`. Mutate by `.insert(...)`.

NEW request — none needed; existing helpers cover create-prefix.

### Command signature

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

Args JSON shape (CLI marshalled):
```json
{ "prefix": "AUTH", "description": "Auth features" }
```

### Algorithm

1. Parse args (`prefix` and `description` required).
2. Validate prefix against regex `^[A-Z]{2,6}$`. On fail → `InvalidArgs { command: "create-prefix", reason: "Failed to create prefix: Prefix must be 2-6 uppercase letters (e.g., AUTH, DASH)" }`.
3. `ensure_prefixes_file(project_root)` → `PrefixesData`. ENOENT → auto-creates empty file (TS parity).
4. If `data.prefixes.contains_key(&prefix)` → `InvalidArgs { reason: "Failed to create prefix: Prefix <X> already exists" }`.
5. Build `Prefix { prefix, description, created_at: Some(now_iso), extra: empty }`.
6. `data.prefixes.insert(prefix, new_prefix)`.
7. `write_json_atomic(spec_path/prefixes.json, &data)`.
8. Return JSON: `{"success": true, "prefix": "AUTH", "description": "Auth features", "createdAt": "..."}` (dispatcher path) OR text `"✓ Prefix AUTH created successfully"` (CLI path).

We'll mirror the TS pattern: return a JSON object with the canonical field set so the dispatcher can present it; the CLI bridge prints the success line.

### CLI bridge

- Required positional args: `<prefix>` and `<description>`.
- Exit 0 on success, 1 on error.
- Success stdout: `✓ Prefix <X> created successfully`.
- Error stderr: `Error: <message>` (parity with list-prefixes bridge contract).

### Dispatcher output

Use a `#[derive(Serialize)] struct CreatePrefixResult { success: bool, prefix: String, description: String, created_at: String }`. Returned as pretty-JSON for `format=json` parity though TS has no `--format`. We default to JSON in the dispatcher path; the CLI bridge ignores body and prints its own success line keyed off success.

Decision: dispatcher returns the JSON shape regardless of `format`. CLI bridge expects success and prints the canonical message — but reads `prefix` field out of the JSON to construct the message.

### Edge cases inventoried

| Input | Result |
|-------|--------|
| `prefix=lower` | error: `Failed to create prefix: Prefix must be 2-6 uppercase letters ...` |
| `prefix=A` | same |
| `prefix=ABCDEFG` | same |
| `prefix=AB1` | same |
| `prefix=""` | same |
| Missing `prefix` arg | clap-level error (or `InvalidArgs` on JSON parse) |
| Valid `AUTH` on empty dir | success; creates `spec/prefixes.json` |
| Valid `UI`, existing file | success; appends to existing `prefixes` IndexMap preserving order |
| Duplicate `AUTH` | error: `Failed to create prefix: Prefix AUTH already exists` |
| Malformed `spec/prefixes.json` | escalated `ParseJson { file: "prefixes.json", ... }` |

### Test surface (cli_create_prefix.rs)

- Help intercept exits 0 and matches TS fixture.
- Help fixture has `ARGUMENTS` block with both `<prefix>` and `<description>` required.
- Empty workspace + valid prefix → exit 0 + file created with prefix entry + stdout = success message.
- Valid prefix + existing file → entry appended preserving prior entries' order.
- Invalid format (lowercase) → exit 1 + stderr contains `Prefix must be 2-6 uppercase letters`.
- Duplicate prefix → exit 1 + stderr contains `Prefix AUTH already exists`.
- Missing positional → clap error (exit 2).
- Two-front-doors parity: dispatcher returns success=true with JSON body, CLI exits 0 with matching message.
