# RPC-299 — AST Research: show-acceptance-criteria

**TS Source:** `src/commands/show-acceptance-criteria.ts` (~332 LOC)

## Exported Symbols (AstGrep)

```
src/commands/show-acceptance-criteria.ts:44  export async function showAcceptanceCriteria(options = {})
src/commands/show-acceptance-criteria.ts:277 export async function showAcceptanceCriteriaCommand(options)
src/commands/show-acceptance-criteria.ts:314 export function registerShowAcceptanceCriteriaCommand(program)
```

## Behaviour Map

1. Check `spec/features` exists → if missing returns `{success:false, error:'spec/features directory not found'}`.
2. `glob spec/features/**/*.feature` → if 0 files returns `{success:true, message:'No feature files found in spec/features/'}`.
3. For each file:
   - Parse with `@cucumber/gherkin` AstBuilder + GherkinClassicTokenMatcher.
   - On parse error: continue (skip).
   - No `gherkinDocument.feature`: continue.
   - Read featureTags from `gherkinDocument.feature.tags[].name`.
   - If `tags.length > 0`: feature must contain ALL specified tags (AND filter).
   - Build `FeatureAC { name, tags, description?, background?, scenarios[] }`.
   - Background built from `child.background.steps` joined with `\n` (optionally prefixed by name + description).
   - Scenarios: only `child.scenario.keyword === 'Scenario'` (skips Scenario Outline).
4. Build message with pluralization (scenario/s, feature/s) — see lines 165-174 for exact templates.
5. Format output: `markdown` → `generateMarkdown(features)`, `json` → `JSON.stringify(features, null, 2)`, otherwise → `generateTextOutput(features)` (uses chalk colors).
6. If `--output <file>`: write formatted output to file, message becomes `Acceptance criteria written to <basename>`.
7. CLI prints `result.message` always; if no `--output`, also prints `result.output`.

## Text Output (chalk-coloured) — `generateTextOutput`

```
\n<NAME>\n
─── (N dashes matching name length)\n
Tags: @a @b\n          (if tags non-empty)
\n<description>\n     (if description)
\nBackground:\n<bg>\n  (if background)
\n  Scenario: <name>\n
    <Keyword> <text>\n  (per step)
\n  No scenarios defined\n   (if scenarios empty)
\n
```

## Markdown Output — `generateMarkdown`

```
# <name>\n\n
**Tags:** @a @b\n\n     (if tags)
<description>\n\n      (if description)
> **Background:**\n> <bg-with-newlines-replaced-by-`\n> `>\n\n   (if background)
## <scenario.name>\n\n
- **<Keyword>** <text>\n
\n
_No scenarios defined_\n\n   (if scenarios empty)
---\n\n
```

## CLI Shape (commander)

```
fspec show-acceptance-criteria
  --tag <tag>           Can be repeated (collects into array via custom parser)
  --format <format>     text | markdown | json   (default text)
  --output <file>       Write output to file
```

## Rust Port Plan

- `codelet/fspec-core/src/commands/show_acceptance_criteria.rs::run(args_json, project_root)`
- Use `gherkin_query::parse_all_features` (shared helper, already ported).
- Filter features by AND-intersection of tags.
- Build `FeatureAC` struct (Serialize) with name/tags/description/background/scenarios.
- Implement two formatters: `render_text(&[FeatureAC]) -> String` and `render_markdown(&[FeatureAC]) -> String`. JSON via `serde_json::to_string_pretty`.
- Optional file writeback when `--output` set.
- CLI bridge is thin — only stdout/exit handling.
