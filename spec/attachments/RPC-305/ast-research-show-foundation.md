# AST Research — RPC-305 show-foundation

## TypeScript Source: `src/commands/show-foundation.ts` (~270 LOC)

### Function signatures

```ts
export async function showFoundation(
  options: ShowFoundationOptions
): Promise<ShowFoundationResult>

export async function showFoundationCommand(
  section?: string,
  options?: { section?: string; format?: string; output?: string; draft?: boolean }
): Promise<void>

export function registerShowFoundationCommand(program: Command): void

function getNestedProperty(obj: any, path: string): any
function formatFoundationAsText(foundation: any): string
```

### Public interfaces

```ts
interface ShowFoundationOptions {
  section?: string;
  format?: 'text' | 'json';
  output?: string;
  cwd?: string;
  draft?: boolean;
}

interface ShowFoundationResult {
  success: boolean;
  output?: string;
  format?: string;
  error?: string;
}
```

### FIELD_MAP (alias resolution)

```ts
projectName       -> project.name
projectVision     -> project.vision
projectType       -> project.projectType
problemTitle      -> problemSpace.primaryProblem.title
problemDescription-> problemSpace.primaryProblem.description
problemImpact     -> problemSpace.primaryProblem.impact
solutionOverview  -> solutionSpace.overview
projectOverview   -> solutionSpace.overview  (legacy)
problemDefinition -> problemSpace.primaryProblem.description  (legacy)
```

### Algorithm branches

1. **draft mode** (`--draft`): read `spec/foundation.json.draft` directly; error if absent
2. **normal mode**: `ensureFoundationFile(cwd)` loads/creates `spec/foundation.json`
3. **section selection**:
   - `section` provided → resolve via FIELD_MAP or direct path; nested property traversal; error if undefined
   - no section → entire foundation object
4. **format**:
   - `json` → `JSON.stringify(data, null, 2)`
   - `text` + section → string passthrough or JSON.stringify for objects
   - `text` + no section → `formatFoundationAsText` (renders PROJECT / PROBLEM SPACE / SOLUTION SPACE / PERSONAS / ARCHITECTURE DIAGRAMS)
5. **output file**: if `outputPath` set, `writeFile(outputPath, formatted, 'utf-8')`

### CLI surface

```ts
program
  .command('show-foundation')
  .description('Display FOUNDATION.md content')
  .argument('[section]', 'Field name or path to display (optional)')
  .option('--section <section>', 'Show specific section only')
  .option('--format <format>', 'Output format: text, markdown, or json', 'text')
  .option('--output <file>', 'Write output to file')
  .option('--draft', 'Read foundation.json.draft instead of foundation.json', false)
  .option('--list-sections', 'List section names only', false)
  .option('--line-numbers', 'Show line numbers', false)
```

NOTE: `--list-sections` and `--line-numbers` flags are declared but not actually used by the action handler in the TS source. Port preserves the flags for surface parity but mirrors TS behavior (no-op).

Positional `[section]` argument: if provided, overrides any `--section` option in action call ordering (`section || options?.section`).

### Rust port plan

- **fspec_core/src/commands/show_foundation.rs** — handler with branches for draft/section/format/list-sections
- **fspec_core/src/help/configs/show_foundation.rs** — help config
- **fspec/src/show_foundation.rs** — CLI bridge
- **fspec/tests/fixtures/help/show-foundation.txt** — help byte-fixture

### Shared-file change requests for supervisor

- `canonical.rs`: register `show-foundation`
- `dispatch.rs`: route to handler
- `help/configs/mod.rs`: re-export
- `main.rs`: add clap subcommand with optional positional `[section]` and all six flags
