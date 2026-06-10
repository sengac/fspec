# AST Research — RPC-260 query-example-mapping-stats

## TypeScript Source: `src/commands/query-example-mapping-stats.ts` (~179 LOC)

### Function signatures discovered

```ts
export async function queryExampleMappingStats(
  options: QueryExampleMappingStatsOptions = {}
): Promise<QueryExampleMappingStatsResult>

export function registerQueryExampleMappingStatsCommand(program: Command): void

function calculateCompletenessScore(workUnit: WorkUnit): number
```

### Public interfaces (data contract)

```ts
interface QueryExampleMappingStatsOptions {
  workUnitId?: string;
  hasQuestions?: boolean;
  questionsFor?: string;
  cwd?: string;
}

interface ExampleMappingStats {
  workUnitId: string;
  title?: string;
  status: string;
  rules: number;
  examples: number;
  questions: number;
  assumptions: number;
  completenessScore: number;
}

interface QueryExampleMappingStatsResult {
  workUnits?: ExampleMappingStats[];
  workUnitsWithRules?: number;
  workUnitsWithExamples?: number;
  workUnitsWithQuestions?: number;
  workUnitsWithAssumptions?: number;
  avgRulesPerWorkUnit?: number;
  avgExamplesPerWorkUnit?: number;
  avgQuestionsPerWorkUnit?: number;
  avgAssumptionsPerWorkUnit?: number;
}
```

### Algorithm

1. `ensureWorkUnitsFile(cwd)` → `WorkUnitsData`
2. Filter by `workUnitId` (throw if absent) / `hasQuestions` / `questionsFor` (matches `@<name>` mention)
3. Map each WU → `ExampleMappingStats` row
4. Aggregate counts + averages
5. `calculateCompletenessScore`: 33 if rules>0, 34 if examples>0, 33 if questions==0

### CLI surface

```ts
program
  .command('query-example-mapping-stats')
  .description('Show example mapping coverage statistics')
  .option('--format <format>', 'Output format: text or json', 'text')
```

NOTE: The CLI action currently only passes `format`. `workUnitId`/`hasQuestions`/`questionsFor` are part of the programmatic API but NOT wired to CLI flags in the TS surface.

### Rust port plan

- **fspec_core/src/commands/query_example_mapping_stats.rs** — handler with JSON dispatch
- **fspec_core/src/help/configs/query_example_mapping_stats.rs** — help text config
- **fspec/src/query_example_mapping_stats.rs** — CLI bridge (marshalling only)
- **fspec/tests/fixtures/help/query-example-mapping-stats.txt** — TS help byte-fixture

### Shared-file change requests for supervisor

- `fspec_core/src/canonical.rs`: register `query-example-mapping-stats` canonical name
- `fspec_core/src/dispatch.rs`: route `query-example-mapping-stats` to handler
- `fspec_core/src/help/configs/mod.rs`: re-export help config
- `fspec/src/main.rs`: add clap subcommand
