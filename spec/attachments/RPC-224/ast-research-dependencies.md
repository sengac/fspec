# AST Research — `dependencies` command (RPC-224)

## TS source surveyed
`src/commands/dependencies.ts` (31.7 KB). The relevant export for this port is
`showDependencies(workUnitId, options, config)` at line 793 — the other exports
(`addDependency`, `removeDependency`, `queryImpact`, graph/mermaid exporters) are
OUT OF SCOPE (separate work units).

### AST findings (grep/structural)
- `export async function showDependencies(workUnitId: string, options: { graph?: boolean }, config: { cwd: string }): Promise<string>` — line 793. Returns a rendered **string**, throws on missing unit.
- Missing-unit guard: `throw new Error(\`Work unit '${workUnitId}' does not exist\`)` — line 803.
- `loadWorkUnits(cwd)` (line 25) = `readFile` + `JSON.parse`, **no auto-create** → port reads work-units.json directly with std::fs.
- Schema fallback (lines 809–815): `workUnit.<rel> || workUnit.relationships?.<rel> || []` for blocks / blockedBy / dependsOn / relatesTo. Legacy top-level fields take precedence over the `relationships` object.
- Text render (lines 817–830): header `Dependencies for <id>:\n` then one indented line per **non-empty** relationship in fixed order Blocks / Blocked by / Depends on / Related to; empty types omitted; result ends in `\n`.
- `--graph` (lines 833–856): DFS via inner `traverse(id, indent)`; `visited: Set` prevents loops; recurses ONLY through `relationships.blocks`; each child rendered `<prefix>  blocks → <id>`, indent grows by 2 spaces per level; lines joined with `\n` (no trailing newline).
- CLI registration (line 1038): `.option('--graph', ..., false)`; error path maps `does not exist` → exit 1.

## Rust port mapping
- `commands/dependencies.rs::run(args_json, project_root)` → returns `Result<String,_>`; dispatcher maps the `does not exist` `InvalidArgs` to `{success:false,error}`.
- Reuse `crate::types::work_unit::{WorkUnitsData,WorkUnit}` (IndexMap insertion order). Relationship arrays read from typed/extra without adding new typed fields to shared `types/work_unit.rs`.
- `--graph` DFS folded into a single returned string (cleaner-than-TS, no double-print) — matches Phase B test expectations.

## Conclusion
TS behaviour fully characterised; no hidden side effects. Port is a pure read +
string render. Ready for testing/implementing phases.
