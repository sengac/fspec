# RPC-226 — `discover-foundation` port research

TS source: `src/commands/discover-foundation.ts` (~859 LOC) + `src/commands/discover-foundation-help.ts`.
Core fn signature target: `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.

## TS public API

`discoverFoundation(options)` returns a large result object. Options:
`outputPath?`, `finalize?`, `draftPath?`, `scanOnly?`, `lastKnownState?`, `detectManualEdit?`, `autoGenerateMd?`, `cwd?`, `force?`.

Commander registration (`registerDiscoverFoundationCommand`) only wires FOUR flags to the action:
`--output <path>` (default `spec/foundation.json`), `--finalize`, `--draft-path <path>` (default `spec/foundation.json.draft`),
`--auto-generate-md` (default true), `--force` (default false).
NOTE: the action passes `autoGenerateMd: options.autoGenerateMd !== false` (i.e. defaults to TRUE).
`scanOnly`, `detectManualEdit`, `lastKnownState` are NOT exposed via CLI — they are used by `update-foundation`'s
internal chaining (already ported into `update_foundation.rs::scan_draft_for_next_field_reminder`).

So the CLI/dispatcher surface for the PORT is just: `finalize`, `outputPath`, `draftPath`, `autoGenerateMd`, `force`.

## Mode 1 — Draft creation (default, no --finalize)

Pre-flight (unless `--force`):
1. If draft file exists → return hard error (valid:false) with wrapped `<system-reminder>` "ERROR: foundation.json.draft already exists!" (three next-step options: --finalize / show-foundation --draft / --force). The CLI prints the systemReminder then `✗ Failed to create draft` to stderr and `process.exit(1)`.
2. Else if final foundation.json exists → return error (valid:false) with wrapped `<system-reminder>` "ERROR: foundation.json already exists!". Same CLI exit-1 path.
   - Non-ENOENT fs errors on `access` are re-thrown.

With `--force` and an existing draft → `output.warn('⚠️  Warning: Overwriting existing foundation.json.draft with --force flag')` then continue.

Draft content (`draftFoundation`, written with `JSON.stringify(..., null, 2)`):
```json
{
  "version": "2.0.0",
  "project": {
    "name": "[QUESTION: What is the project name?]",
    "vision": "[QUESTION: What is the one-sentence vision?]",
    "projectType": "[DETECTED: cli-tool]"
  },
  "problemSpace": {
    "primaryProblem": {
      "title": "[QUESTION: What problem does this solve?]",
      "description": "[QUESTION: What problem does this solve?]",
      "impact": "high"
    }
  },
  "solutionSpace": { "overview": "[QUESTION: What can users DO?]", "capabilities": [] },
  "personas": [
    { "name": "[QUESTION: Who uses this?]", "description": "[QUESTION: Who uses this?]", "goals": ["[QUESTION: What are their goals?]"] }
  ]
}
```
- `mkdir(dirname(draftPath), {recursive})` then `writeFile(draftPath, draftContent)`.
- Scan draft for first unfilled field → `firstFieldReminder` (Field 1/8: project.name body).
- `agent = getAgentConfig(cwd)` → `thinkingInstruction` = "you must ULTRATHINK the entire codebase" (meta-cognition) else "you must think a lot about the entire codebase".
- `forceOverwriteWarning` prepended when `--force`.
- systemReminder (NOT wrapped in tags — the field reminder is already wrapped):
```
${forceWarn}Draft created. To complete foundation, ${thinkingInstruction}.

Analyze EVERYTHING: code structure, entry points, user interactions, documentation.
Understand HOW it works, then determine WHY it exists and WHAT users can do.

I will guide you field-by-field.

${firstFieldReminder}
```
Returns `{ systemReminder, valid:true, draftPath, draftCreated:true, draftContent }`.

CLI success branch (non-finalize, valid:true):
- if result.systemReminder → `output.log(systemReminder)`.
- `output.log("✓ Generated <draftPath>")`.
- `output.log("\nNext steps:")`.
- `output.log(chalk.yellow("1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders"))`.
- `output.log(chalk.yellow("2. When complete, run: fspec discover-foundation --finalize"))`.

## Mode 2 — Finalize (--finalize)

`finalPath = options.outputPath || join(cwd,'spec/foundation.json')`.
1. Read+parse draft (`readFile(draftPath)` → JSON.parse). NOTE: missing draft → readFile throws (no graceful handling); becomes Io error in Rust.
2. `scanDraftForNextField(foundation)` → if `nextField` present → NOT all complete:
   - return `{ valid:false, validated:true, validationErrors: "Cannot finalize: draft still has unfilled placeholder fields...\nField '<nextField>' still contains [QUESTION:] or [DETECTED:] placeholders.\n..." , foundation }`.
3. Else validate via `validateGenericFoundationObject(foundation)` (Ajv generic-foundation schema). On invalid:
   - Map each Ajv error via `formatAjvErrorForFinalize` (required→"Missing required: <path>"; minItems→"Missing required: <field> (at least one item required)"; maxLength/minLength→length message with fix hint; enum→"... is not one of [...]"; fallback→Ajv message).
   - `validationErrors = "Schema validation failed.\n\n<joined errors>\n\nFix by running appropriate commands:\n  - For simple fields: ...\n  - For capabilities: ...\n  - For personas: ...\n\nThen re-run: fspec discover-foundation --finalize"`.
   - return `{ valid:false, validated:true, validationErrors, foundation }`.
4. On valid:
   - `mkdir(dirname(finalPath),{recursive})`; `writeFile(finalPath, JSON.stringify(foundation,null,2))`.
   - `unlink(draftPath)`.
   - Auto-create FOUND work unit (idempotent):
     - `ensureWorkUnitsFile(cwd)`; if any id starts with `FOUND-` → reuse (workUnitCreated=false, workUnitId=existing).
     - else: `createPrefix({prefix:'FOUND', description:'Foundation Event Storm tasks', cwd})` (swallow already-exists error); then `createWorkUnit('FOUND', 'Conduct Foundation Event Storm for Foundation', {cwd, type:'task', description:<long FOUND blurb>})`; workUnitCreated=true.
     - ENTIRE block wrapped in try/catch — silently swallow on any failure.
   - if `autoGenerateMd` → `generateFoundationMdCommand({cwd: dirname(dirname(finalPath))})`; `mdGenerated=mdResult.success`.
   - completionMessage = "Discovery complete!\n\nCreated: <finalPath>{, spec/FOUNDATION.md if md}\n\nFoundation is ready.".
   - return `{ valid:true, validated:true, finalPath, finalCreated:true, draftDeleted:true, allFieldsComplete:true, mdGenerated, completionMessage, workUnitCreated, workUnitId, foundation }`.

CLI finalize branch:
- if result.systemReminder → log it (none on finalize path).
- if !valid → `output.error('✗ Foundation validation failed')`; if validationErrors → `output.error('\n'+validationErrors)`; `process.exit(1)`.
- else: `output.log("✓ Generated <finalPath>")`; if mdGenerated → `output.log("✓ Generated spec/FOUNDATION.md")`; `output.log(chalk.green("✓ Foundation discovered and validated successfully"))`; if workUnitCreated && workUnitId → `output.log(chalk.green("✓ Created work unit <id>: Foundation Event Storm"))` + `output.log(chalk.dim("  Run: fspec show-work-unit <id>"))`.

## scanDraftForNextField (lines 37-93)
Ordered 8-field list (project.name, project.vision, project.projectType,
problemSpace.primaryProblem.title, problemSpace.primaryProblem.description,
solutionSpace.overview, solutionSpace.capabilities, personas).
- `field.value === undefined` → skip (NOT placeholder).
- value→string (string as-is, else JSON.stringify); hasPlaceholder = contains "[QUESTION:" or "[DETECTED:".
- first hasPlaceholder field → nextField (1-indexed fieldNumber). completedFields counts non-placeholder.

This is ALREADY ported into `update_foundation.rs::scan_draft_for_next_field_reminder` +
`field_reminder_body` + `extract_detected_value` + `agent_supports_meta_cognition`/`is_known_agent`.
**REUSE strategy**: these are private to update_foundation.rs. I will copy the field-reminder
builder into discover_foundation.rs (or ask supervisor to promote a shared module). See arch notes.

## Existing Rust reuse available
- `io::ensure::ensure_foundation_file` (load-or-init final), `ensure_work_units_file`.
- `io::locked_file::{read_or_init_json, write_json_atomic}` (write_json_atomic = 2-space indent, no trailing nl — matches JSON.stringify,null,2).
- `commands::generate_foundation_md::regenerate(project_root)` (best-effort MD regen, swallows errors).
- `generators::foundation_schema::validate_foundation(&Value) -> Result<(),Vec<SchemaError>>` (native Ajv-equivalent; SchemaError has instancePath, keyword, message, params? — MUST confirm shape to port formatAjvErrorForFinalize).
- `commands::create_prefix::run` + `commands::create_story::run` — but FOUND unit is type:task, so need create_task or createWorkUnit-equivalent. NO `create_task` core fn exists yet (only create_story). Need supervisor guidance — see flags.

## DIVERGENCE / ASYNC FLAGS (for supervisor)
1. **Dispatch arm signature**: current stub `discover_foundation::run(args_json)` (single arg). Ported fn needs `(args_json, project_root)`. SHARED-FILE CHANGE: dispatch.rs:634 + move to run_ported + canonical PORTED_COMMANDS. (supervisor)
2. **No core `create_task`**: FOUND auto-unit is `type:'task'`. Options: (a) inline-build the FOUND task object like create_story does (idempotency check for `FOUND-` prefix, createPrefix FOUND, then build task), or (b) supervisor exposes a shared create-work-unit core helper. The whole block is best-effort (TS try/catch swallows), so inline build is safe & isolated. PROPOSAL: inline in discover_foundation.rs, swallow all errors.
3. **getAgentConfig / scan-field-reminder duplication**: identical logic already private in update_foundation.rs. PROPOSAL: copy into discover_foundation.rs (isolated) OR supervisor promotes a `foundation_reminder` shared module. Ask supervisor.
4. **formatAjvErrorForFinalize** depends on SchemaError shape from foundation_schema.rs — must read that struct in Phase C to port the keyword-specific messages (required/minItems/maxLength/minLength/enum/fallback).
5. **No real async**: all fs is std::fs blocking. createPrefix/createWorkUnit/generateFoundationMd are sync in Rust. Safe under poll_sync_future. No tokio await, no child process, no network.
6. **scanOnly / detectManualEdit / lastKnownState NOT in CLI surface** — out of port scope (only finalize/output/draftPath/autoGenerateMd/force). The scanOnly chaining already lives in update_foundation. Confirm with supervisor we DON'T need to expose them.
7. **output() wrapper**: TS `output.log/warn/error` → stdout/stderr. The CLI bridge owns rendering. Core returns a structured String (JSON) the bridge decodes, OR core returns rendered text. Given the multi-branch CLI output, core should return a JSON envelope ({valid, draftCreated|finalCreated, systemReminder, draftPath, finalPath, mdGenerated, workUnitCreated, workUnitId, validationErrors, forceWarn?}) and the bridge renders the exact TS stdout/stderr lines. (Mirrors update_foundation pattern.)
