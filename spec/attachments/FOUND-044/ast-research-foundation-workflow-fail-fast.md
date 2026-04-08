# AST Research: FOUND-044 Fail-Fast Foundation Workflow

## Research Methodology

Searched via AstGrep and direct Read/Grep across the fspec codebase to map every
code path touched by the proposed changes. Focus areas: `projectType` enum →
freeform conversion, length validation at write time, `show-foundation --draft`
flag, improved error formatting at finalization, and the new
`list-foundation-sections` standalone command.

## 1. Schema Definition — `projectType` enum (MUST CHANGE)

**File:** `src/schemas/generic-foundation.schema.json` lines 36–50

```json
"projectType": {
  "type": "string",
  "description": "Type of software project",
  "enum": [
    "web-app", "cli-tool", "library", "sdk", "mobile-app",
    "desktop-app", "service", "api", "other"
  ]
}
```

**Change to:**

```json
"projectType": {
  "type": "string",
  "description": "Short descriptor for the project type (freeform, e.g. cli-tool, web-app, saas-platform, browser-extension)",
  "minLength": 1,
  "maxLength": 30
}
```

## 2. TypeScript Union Type (MUST CHANGE)

**File:** `src/types/generic-foundation.ts` lines 89–98

```typescript
export type ProjectType =
  | 'web-app' | 'cli-tool' | 'library' | 'sdk' | 'mobile-app'
  | 'desktop-app' | 'service' | 'api' | 'other';
```

**Change to:**

```typescript
export type ProjectType = string;
```

The interface at line 77–83 (`ProjectIdentity.projectType: ProjectType`) stays
the same because `ProjectType` is now a string alias.

## 3. `update-foundation` switch case (ADD LENGTH VALIDATION)

**File:** `src/commands/update-foundation.ts` lines 163–166

```typescript
case 'projectType':
  foundation.project = foundation.project || {};
  foundation.project.projectType = content;
  return true;
```

**Change to:**

```typescript
case 'projectType':
  if (content.length < 1 || content.length > 30) {
    return false; // caller handles actionable error
  }
  foundation.project = foundation.project || {};
  foundation.project.projectType = content;
  return true;
```

The `updateJsonField` helper currently returns a boolean; the caller at line
83 maps `false` → generic "Unknown section" error. Needs a second return type
(result discriminator) so length errors emit a specific actionable message
distinct from unknown section name.

## 4. `problemImpact` enum validation (ALREADY EXISTS)

**File:** `src/commands/update-foundation.ts` lines 184–187

```typescript
case 'problemImpact':
  if (!['high', 'medium', 'low'].includes(content)) {
    return false;
  }
```

Current behavior returns `false`. The error at the call site (line 87) is:
`Unknown section: "problemImpact"`. **This is misleading** — the section name
IS valid, it's the value that's wrong. Work unit must distinguish these two
failure modes.

## 5. Ajv Validation Error Formatter (MUST EXTEND)

**File:** `src/commands/discover-foundation.ts` lines 336–365

Current code only handles `missingProperty` errors:

```typescript
const errorMessages = errors.map(err => {
  let field = err.instancePath.replace(/^\//, '').replace(/\//g, '.');
  if (err.params && 'missingProperty' in err.params) {
    const missingProp = err.params.missingProperty as string;
    field = field ? `${field}.${missingProp}` : missingProp;
  }
  return `Missing required: ${field}`;
});
```

**MUST EXTEND** to handle `err.keyword === 'enum'` (for `problemImpact`) and
`err.keyword === 'maxLength'`/`'minLength'` (for new `projectType` constraint).
Output format must distinguish them:

- `missingProperty` → `Missing required: <field>`
- `enum` → `Invalid value at <field>: "<value>" is not in enum. Valid values: <list>. Fix: fspec update-foundation <field> "<valid-value>"`
- `maxLength` → `Invalid value at <field>: maxLength exceeded (must be 1-30 characters, got <N>). Fix: fspec update-foundation <field> "<short-descriptor>"`
- `minLength` → `Invalid value at <field>: must be at least <N> characters. Fix: fspec update-foundation <field> "<value>"`

## 6. `show-foundation` options (MUST ADD --draft)

**File:** `src/commands/show-foundation.ts` lines 221–236

```typescript
export function registerShowFoundationCommand(program: Command): void {
  program
    .command('show-foundation')
    .argument('[section]', 'Field name or path to display (optional)')
    .option('--section <section>', 'Show specific section only')
    .option('--format <format>', 'Output format...', 'text')
    .option('--output <file>', 'Write output to file')
    .option('--list-sections', 'List section names only', false)
    .option('--line-numbers', 'Show line numbers', false)
    .action(showFoundationCommand);
}
```

**MUST ADD** `--draft` option. Implementation:
- Read `spec/foundation.json.draft` instead of `spec/foundation.json`
- Error clearly if draft doesn't exist (not fallback to final)
- Use same `ensureFoundationFile` / rendering pipeline

**File:** `src/commands/show-foundation.ts` line 49

```typescript
const foundationData: any = await ensureFoundationFile(cwd);
```

Must be replaced with a branch: if `--draft`, read draft; else use current
logic. Draft read must NOT invoke `ensureFoundationFile` (which creates a
default file if missing).

## 7. `discover-foundation` draft-exists error (IMPROVE)

**File:** `src/commands/discover-foundation.ts` lines 501–527

Currently outputs a long multi-paragraph error with 2 branches (continue vs
start over). **MUST REWRITE** to be:

1. Concise (max ~10 lines)
2. List exactly 3 actionable next steps: `--finalize`, `show-foundation --draft`, `--force`
3. No inline draft content
4. Wrapped in `wrapInSystemReminder()` (existing helper)
5. Preserve `valid: false` return to match existing hard-error convention

## 8. `update-foundation-help.ts` text (MUST REWRITE)

**File:** `src/commands/update-foundation-help.ts` lines 1–107

Changes:
- Line 5–6: description `'Update section content in foundation.json or foundation.json.draft during discovery'` → `'Update a field in foundation.json (or foundation.json.draft during discovery)'`
- Line 102: Remove `'etc.'` and enumerate full list: `projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview`
- ADD note: `'Capabilities and personas are managed via dedicated commands: add-capability, add-persona, remove-capability, remove-persona'`
- Line 60: update `commonErrors` to include new length-violation error format

## 9. New Command: `list-foundation-sections` (CREATE)

**File:** `src/commands/list-foundation-sections.ts` (NEW)

Must expose every valid section name with JSON path and constraint info.
Follows pattern of existing `list-features.ts`, `list-epics.ts`, `list-tags.ts`,
`list-prefixes.ts` (12 standalone `list-*` commands already exist vs 1
`--list-sections` flag at `show-foundation.ts:233`).

Output format (text mode):
```
projectName     | project.name                              | freeform string (required)
projectVision   | project.vision                            | freeform string (required)
projectType     | project.projectType                       | freeform string (1-30 characters), examples: cli-tool, web-app, saas-platform
problemTitle    | problemSpace.primaryProblem.title         | freeform string
problemDefinition | problemSpace.primaryProblem.description | freeform string
problemImpact   | problemSpace.primaryProblem.impact        | enum: high, medium, low
solutionOverview | solutionSpace.overview                   | freeform string
```

Must also register in command router (likely `src/index.ts` or equivalent).

## 10. Test Files That Need Updating

- `src/types/__tests__/foundation-schema.test.ts` lines 129–140: remove
  `validProjectTypes: ProjectType[]` array, replace with length-boundary tests
  (accept 1-char, 30-char, reject 0-char, 31-char).
- `src/commands/__tests__/update-foundation.test.ts`: add tests for length
  validation on projectType (empty, 1-char, 30-char, 31-char).
- `src/commands/__tests__/discover-foundation.test.ts`: add tests for:
  - Draft-exists error listing exactly 3 options
  - Finalize with overlong projectType → length error (not missing-required)
- New: `src/commands/__tests__/show-foundation-draft.test.ts` for `--draft` flag.
- New: `src/commands/__tests__/list-foundation-sections.test.ts` for new command.
- New: `src/commands/__tests__/update-foundation-help-content.test.ts` for help
  text requirements (all sections listed, no 'etc.', capability/persona note).

## 11. Test Fixtures That DO NOT Need Changes

~60 test fixtures across the codebase use `projectType: 'cli-tool'`,
`'web-app'`, `'library'`, `'other' as const`, etc. All remain valid because
these values are still valid freeform strings within the 1-30 character limit.
The `as const` assertions in 9 event-storm command files become superfluous
but remain valid TypeScript.

## 12. Consumers That Read `projectType` (NO LOGIC CHANGE)

- `src/commands/show-foundation.ts:130`: `Type: ${foundation.project.projectType || 'N/A'}` — displays verbatim.
- `src/commands/discover-foundation.ts:265`: extracts `[DETECTED: X]` placeholder via regex `/\[DETECTED:\s*(.*?)\]/` — works on any string.
- `src/generators/foundation-md.ts`: does NOT reference `projectType` at all (only renders `name` and `vision`).
- `src/utils/ensure-files.ts:208`: defaults to `'cli-tool'` for newly-created foundation — still valid.

## Conclusion: Low Blast Radius

Only **5 source files** require modification:
1. `src/schemas/generic-foundation.schema.json` (remove enum, add length)
2. `src/types/generic-foundation.ts` (collapse union to string)
3. `src/commands/update-foundation.ts` (add length + enum validation with better errors)
4. `src/commands/show-foundation.ts` (add --draft flag)
5. `src/commands/discover-foundation.ts` (improved draft-exists error + Ajv formatter)
6. `src/commands/update-foundation-help.ts` (rewrite description + enumeration)

**1 new source file** to create:
1. `src/commands/list-foundation-sections.ts` (new standalone command)

**~6 test files** to add/update (per section 10 above).

**Zero business logic changes** — no code path anywhere branches on specific
`projectType` values. The enum was purely input validation. Removing it is
safe.
