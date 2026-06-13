# AST Research — `update-foundation` (RPC-312)

## TS source of truth
- `src/commands/update-foundation.ts` (330 LOC)
- `src/commands/update-foundation-help.ts` (help config)

## Behaviour (TS `updateFoundation`)
Signature: `updateFoundation({ section, content, cwd?, draftPath? }) => { success, message?, error?, systemReminder? }`

### Validation order (fail-fast, BEFORE generic empty guard)
1. `section` empty/whitespace → `error: "Section name cannot be empty"`.
2. If `section === 'projectType'`: length rule (1-30 chars). Errors:
   - empty/null → `Invalid projectType: "" (must be 1-30 characters, got 0). Fix: fspec update-foundation projectType "<short-descriptor>"`
   - len>30 → `Invalid projectType: too long (must be 1-30 characters, got N). Fix: ...`
3. If `section === 'problemImpact'`: enum {high,medium,low}. Errors:
   - empty → `Invalid value for problemImpact: "". Valid values: high, medium, low. Fix: fspec update-foundation problemImpact "<valid-value>"`
   - other invalid → `Invalid value for problemImpact: "<content>". Valid values: high, medium, low. Fix: ...`
4. Generic: `content` undefined/null/whitespace → `error: "Section content cannot be empty"`.

### File selection
- `draftPath = cwd/spec/foundation.json.draft` (overridable via option).
- `foundationJsonPath = cwd/spec/foundation.json`, `foundationMdPath = cwd/spec/FOUNDATION.md`.
- If draft EXISTS → `isDraft=true`, target=draft. Load via `readFile(draft)` + `JSON.parse`.
- Else → target=foundation.json. Load via `ensureFoundationFile(cwd)` (auto-creates).

### Field mapping (`updateJsonField`) — section → nested path
- `projectName`|`name` → `project.name`
- `projectVision`|`vision` → `project.vision`
- `projectType` → `project.projectType`
- `problemTitle` → `problemSpace.primaryProblem.title`
- `problemDefinition`|`problemDescription` → `problemSpace.primaryProblem.description`
- `problemImpact` → `problemSpace.primaryProblem.impact`
- `solutionOverview`|`projectOverview` → `solutionSpace.overview`
- legacy `testingStrategy`|`developmentTools`|`architecturePattern`|`painPoints`|`methodology` → `solutionSpace.overview`
- default → returns false → `error: "Unknown section: \"<section>\". Use field names like: projectOverview, problemDefinition, etc."`
- Each branch lazily creates parent objects (`x = x || {}`).

### Write + post-write
- Write `JSON.stringify(data, null, 2)` (NO trailing newline) to target.
- DRAFT path: do NOT validate or regen MD. Chain `discoverFoundation({ scanOnly:true, draftPath, cwd })` and return its `systemReminder`. Message: `Updated "<section>" in foundation.json.draft`.
- FINAL path: `validateFoundationJson(foundationJsonPath)`. If invalid → `error: "Updated foundation.json failed schema validation: <messages>"`. Then regenerate FOUNDATION.md via `generateFoundationMd(data)` + write. Message: `Updated "<section>" section in FOUNDATION.md`.
- catch-all → `error: error.message`.

### CLI command (`updateFoundationCommand`)
- args `<section> <content>` (both required positional).
- success: `output.log('✓', message)`; if message includes 'draft' → log `  Updated: spec/foundation.json.draft` + emit chained systemReminder; else → log `  Updated: spec/foundation.json` and `  Regenerated: spec/FOUNDATION.md`. exit 0.
- failure: `output.error('Error:', error)`; exit 1.

## Rust reference patterns
- `add_command_to_foundation.rs`: `read_or_init_json` foundation default, mutate `serde_json::Value` (preserve_order), `write_json_atomic`, `generate_foundation_md::regenerate(project_root)`.
- `ensure.rs::ensure_foundation_file` — richer default for final path.
- `write_json_atomic` (NO trailing newline) matches `JSON.stringify(...,2)`.
- `show_foundation.rs` FIELD_MAP precedent for section→path.

## Shared-file / scope concerns (for supervisor)
1. Rust `discover_foundation` is still a STUB → draft-path `systemReminder` chaining cannot be reproduced. Options: (a) emit draft success message WITHOUT systemReminder, (b) supervisor provides a scan helper. NEEDS DECISION.
2. Rust `validate_foundation_schema` is still a STUB → final-path schema validation cannot run. Options: (a) skip schema validation (write+regen MD only), (b) supervisor provides validator. NEEDS DECISION. Recommend (a) skip for now, parity-note it.
3. `generate_foundation_md::regenerate` already exists — reuse on final path.

## Open questions
- Q1: Reproduce draft-chaining systemReminder now or defer? (depends on discover_foundation port)
- Q2: Reproduce final-path schema validation now or defer? (depends on validate_foundation_schema port)
