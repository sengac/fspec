# AST Research — export-dependencies (RPC-227)

## TS source of truth
- `src/commands/export-dependencies.ts` (147 LOC)
- `src/commands/export-dependencies-help.ts` (help config)

## Behaviour (TS)
`exportDependencies({ format, output, cwd? })`:
1. `cwd = options.cwd || process.cwd()`.
2. `data = await ensureWorkUnitsFile(cwd)` — auto-create + escalate malformed JSON.
3. If `format === 'mermaid'` → `outputContent = generateMermaidDiagram(data)`.
4. ELSE (any other value, incl. `'json'`, `'dot'`, garbage) → JSON branch.
   - **CRITICAL**: TS type is `format: 'mermaid' | 'json'` but runtime only special-cases `'mermaid'`; everything else (including `dot`) falls into the JSON `else` branch. Verified: `export-dependencies dot deps.dot` writes JSON content.
5. `mkdir(dirname(output), { recursive: true })`.
6. `writeFile(output, outputContent)` — NO trailing newline added by writeFile.
7. Returns `{ success, outputFile }`.

### generateMermaidDiagram(data)
Lines array, joined with `\n` (no trailing newline):
```
graph TB
```
Then for each `[id, workUnit]` of `Object.entries(data.workUnits)` (insertion order):
- statusClass = `:::done` if status==='done', `:::blocked` if status==='blocked', else `''`
- `  ${id}["${workUnit.title || id}"]${statusClass}`

Then a SECOND pass over entries with a `Set` of `addedEdges`:
- For each `blocks` target: `  ${id} -->|blocks| ${targetId}`  (edgeKey `${id}-blocks-${targetId}`)
- For each `dependsOn` target: `  ${id} -.->|depends on| ${targetId}`  (edgeKey `${id}-dependsOn-${targetId}`)
- For each `relatesTo` target: `  ${id} <-.->|relates to| ${targetId}` — only added if neither `${id}-relatesTo-${targetId}` NOR reverse `${targetId}-relatesTo-${id}` already added (dedupe bidirectional)
Then:
```
<blank line>
  classDef done fill:#90EE90
  classDef blocked fill:#FFB6C1
```

### JSON branch
Build `dependencies: Record<id, {blocks, blockedBy, dependsOn, relatesTo}>` for EVERY work unit (insertion order):
```json
{
  "<id>": {
    "blocks": workUnit.blocks || [],
    "blockedBy": workUnit.blockedBy || [],
    "dependsOn": workUnit.dependsOn || [],
    "relatesTo": workUnit.relatesTo || []
  },
  ...
}
```
`JSON.stringify(dependencies, null, 2)` — 2-space indent, field order blocks/blockedBy/dependsOn/relatesTo. NO trailing newline.

## CLI registration
- `program.command('export-dependencies')`
- `.argument('<format>', 'Output format: mermaid or json')` — required positional
- `.argument('<output>', 'Output file path')` — required positional
- Action: on success `output.log(chalk.green('✓ Dependencies exported to ${result.outputFile}'))`.
- On error: `output.error(chalk.red('✗ Failed to export dependencies:'), message)` + `process.exit(1)`.

## Verified actual output (`node dist/index.js`)
- mermaid success stdout: `✓ Dependencies exported to deps.mmd\n`
- mermaid file content (verbatim, see session): nodes + edges + classDefs as above.
- json/dot both produce identical JSON file content (dot falls into else branch).
- Missing `output` arg → exit=1 Commander error.

## Mermaid edge dedupe details
- `blocks`/`dependsOn` dedupe purely on their own edgeKey (no reverse check).
- `relatesTo` dedupe on edgeKey AND reverse key (prevents A<-.->B and B<-.->A duplication).

## Rust port plan
- Read `blocks`/`blockedBy`/`dependsOn`/`relatesTo` arrays from `WorkUnit.extra` as `Vec<String>` (use `.as_str()` per entry; mirror `array_entries` helper in query_dependency_stats.rs). Title from `WorkUnit.title`, fallback to id.
- Iterate `data.work_units` (IndexMap preserves insertion order — parity with `Object.entries`).
- `format == "mermaid"` → build mermaid string; else → build JSON via `#[derive(Serialize)]` per-entry struct (blocks/blockedBy/dependsOn/relatesTo) wrapped in an `IndexMap<String, _>` to preserve insertion order (NOT BTreeMap), `serde_json::to_string_pretty`.
- `create_dir_all(dirname(output))`, write file.
- Core `run` returns success message `✓ Dependencies exported to <output>`; CLI bridge prints it.
- Error escalation: malformed JSON from ensure → FspecCoreError::ParseJson; bridge prints `✗ Failed to export dependencies: <msg>` to stderr, exit 1.

## Reference impls consulted
- `codelet/fspec-core/src/commands/query_dependency_stats.rs` (reads blocks/dependsOn/relatesTo arrays from extra via `array_entries`)
- `codelet/fspec-core/src/io/ensure.rs` (`ensure_work_units_file`)
- `codelet/fspec/src/query_work_units.rs` (CLI bridge)
- `codelet/fspec-core/src/types/work_unit.rs` (WorkUnit / IndexMap insertion order)
