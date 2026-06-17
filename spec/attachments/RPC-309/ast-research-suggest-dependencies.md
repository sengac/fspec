# AST Research — suggest-dependencies (RPC-309)

## TS source of truth
- `src/commands/suggest-dependencies.ts` (267 LOC)
- `src/commands/suggest-dependencies-help.ts`

## Signature (TS)
```ts
export async function suggestDependencies(
  options: { cwd?: string; output?: 'json' | 'text' } = {}
): Promise<{ suggestions: DependencySuggestion[] }>
```
`DependencySuggestion = { from, to, type: 'dependsOn'|'relatesTo', reason, confidence: 'high'|'medium' }`

## Data load
- `const data = await ensureWorkUnitsFile(cwd)` — AUTO-CREATES `spec/work-units.json` if missing
  (parity: Rust `ensure_work_units_file` — escalates malformed JSON via ParseJson).
- `const workUnits = Object.values(data.workUnits)` — IndexMap insertion order.

## Algorithm (5 rules, in this evaluation order)
1. **Group by prefix** — `wu.id.split('-')[0]` into `Map<prefix, units[]>` for sequential analysis.
2. **Rule 3 (Build/Test pairs)** — HIGH confidence, evaluated FIRST (higher priority than sequential).
   - For each `wu` whose `title.toLowerCase().startsWith('test ')`: testTarget = title without leading `test `.
   - For each candidate `!== wu`: if candidateTitle `startsWith('build ')` AND `includes(testTarget)`:
     - skip if `wu.dependsOn?.includes(candidate.id) || wu.blockedBy?.includes(candidate.id)`.
     - push `{ from: wu.id, to: candidate.id, type:'dependsOn', reason:`test work depends on build work: "${wu.title}" depends on "${candidate.title}"`, confidence:'high' }`.
     - record `specificMatches.add(`${wu.id}->${candidate.id}`)`.
3. **Rule 4 (Infrastructure before features)** — HIGH confidence.
   - infrastructureKeywords = ['schema','migration','database schema','setup','infrastructure'].
   - featureKeywords = ['add','create','implement','build'].
   - For each featureWu whose title startsWith one of featureKeywords + ' ':
     - For each infraWu (`!== featureWu`) whose title `.includes` any infra keyword:
       - require SAME prefix (`featureWu.id.split('-')[0] === infraWu.id.split('-')[0]`).
       - skip if existing dependsOn/blockedBy includes infraWu.id.
       - push reason `infrastructure work (schema/migration) should complete before feature work: "${featureWu.title}" depends on "${infraWu.title}"`, confidence high.
       - record specificMatches.
4. **Rule 2 (Sequential IDs in same prefix)** — MEDIUM confidence, FALLBACK.
   - per prefix, sort units by numeric part `parseInt(id.split('-')[1] || '0')`.
   - for i in 1..sorted.len: from=sorted[i], to=sorted[i-1].
   - skip if existing dependsOn/blockedBy includes to.id.
   - skip if `specificMatches.has(`${from.id}->${to.id}`)`.
   - push reason `sequential IDs in ${prefix} prefix suggest ${from.id} depends on ${to.id}`, confidence medium.
5. **Rule 5 (Remove circular)** — filter: if a reverse suggestion exists (s.from===suggestion.to && s.to===suggestion.from), keep only `suggestion.from < suggestion.to` (string compare). Otherwise keep.

NOTE: Rule 1 ("same epic → relatesTo") documented in JSDoc but NOT implemented in TS. Mirror TS — do NOT add it.

## Output
- `output: 'json'` → `JSON.stringify(result, null, 2)` (pretty, 2-space).
- default text:
  - empty → `No dependency suggestions found.` then dim line `Suggestions are based on sequential IDs, build/test pairs, and infrastructure patterns.`
  - else bold `\nFound N dependency suggestion(s):\n` then per suggestion:
    - `${i+1}. ${from} → ${to} (${type})`
    - `   ● ${reason}` (green dot for high, yellow for medium — identity in non-TTY)
    - `   Confidence: ${confidence.toUpperCase()}\n`
  - trailing dim line `To apply a suggestion: fspec add-dependency <from-id> --depends-on=<to-id>`.

## Error path
- catch → `output.error(chalk.red('✗ Failed to suggest dependencies:'), error.message); process.exit(1)`.

## Rust port plan
- `commands/suggest_dependencies.rs`: `Args { output: Option<String> }`.
- Read via `ensure_work_units_file(project_root)`.
- WorkUnit fields used: `id`, `title`; `dependsOn`/`blockedBy` read from `extra.get(...)` as arrays.
- `Suggestion` struct `#[derive(Serialize)] #[serde(rename_all="camelCase")]` decl order from,to,type,reason,confidence.
  NOTE `type` is reserved → use `r#type` with `#[serde(rename="type")]`.
- Result `{ suggestions: Vec<Suggestion> }`.
- string `<` comparison for circular tiebreak = Rust `&str` `<` (byte/lexicographic — matches JS string compare for ASCII ids).
- Two-front-doors: CLI bridge marshals `--output`, dispatcher passes args_json verbatim.

## Shared modules reused (no new helpers needed)
- `crate::io::ensure::ensure_work_units_file`
- `crate::types::work_unit::{WorkUnit, WorkUnitsData}`
- `crate::error::FspecCoreError`
