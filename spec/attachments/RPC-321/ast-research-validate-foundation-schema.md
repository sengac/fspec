# AST Research — validate-foundation-schema (RPC-321)

## TS source of truth
- `src/commands/validate-foundation-schema.ts` (145 LOC)
- `src/commands/validate-foundation-schema-help.ts`
- Schema: `src/schemas/generic-foundation.schema.json`

## TS control flow (`validateFoundationSchema`)
1. `cwd = options.cwd || process.cwd()`
2. Read `join(cwd, 'spec/foundation.json')`, `JSON.parse`.
3. Read bundled `generic-foundation.schema.json` (tries `dist/schemas/` then `src/schemas/`).
4. `new Ajv({ allErrors:true, verbose:true, strictSchema:false, logger:false })`; `compile(schema)`; `validate(foundation)`.
5. On invalid: map each `err` →
   - if `err.keyword === 'minItems'`: `Field <path'/'→'.'> must have at least <params.limit> items (found <err.data?.length || 0>)`
   - else: `${err.instancePath || err.schemaPath}: ${err.message}`
   joined by `\n`. Return `{success:false, error}`.
6. On valid: `{success:true, output:'✓ foundation.json is valid according to the schema'}`.
7. catch: if message includes `ENOENT` → `{success:false, error:'foundation.json not found in spec/ directory'}`; else `{success:false, error:'Failed to validate foundation schema: <message>'}`.

`validateFoundationSchemaCommand`: on `!success` → `output.error('Error:', result.error); process.exit(1)`. Else `output.log(result.output); process.exit(0)`. Outer catch → `output.error('Error:', message); process.exit(1)`.

`registerValidateFoundationSchemaCommand`: `.command('validate-foundation-schema').description('Validate foundation.json against JSON Schema').action(...)` — NO flags.

## Existing Rust infrastructure to REUSE
- `codelet/fspec-core/src/generators/foundation_schema.rs`:
  - `pub fn validate_foundation(data: &Value) -> Result<(), Vec<SchemaError>>`
  - `pub struct SchemaError { pub instance_path: String, pub message: String }`
  - `pub fn format_errors(&[SchemaError]) -> String` (joins `instance_path: message` by `"; "`) — NOT used here (TS joins by `\n`).
  - Bundled schema embedded via `include_str!("generic-foundation.schema.json")` (identical to src/schemas copy — verified `diff` IDENTICAL).
  - Native validator emits minItems message `must NOT have fewer than <limit> items` (Ajv standard wording). It does NOT carry params.limit/data.length.
- `codelet/fspec-core/src/io/ensure.rs::ensure_foundation_file` AUTO-CREATES — must NOT use (TS reads directly, ENOENT → error). Read foundation.json directly via `std::fs::read_to_string` + `serde_json::from_str`.

## minItems parity mapping (command layer, NOT shared validator)
For each SchemaError:
- If `message` starts with `"must NOT have fewer than "` and ends with `" items"`:
  - parse `<limit>` from message.
  - `dotted = instance_path.trim_start_matches('/').replace('/', ".")`.
  - resolve actual array length: walk parsed foundation JSON along instance_path segments; if array → len, else 0.
  - render `Field {dotted} must have at least {limit} items (found {len})`.
- Else render `{instance_path}: {message}` (instance_path may be empty for root errors → `: must have required property '...'`).

Note: TS uses `err.instancePath || err.schemaPath`. Native validator always populates instance_path (empty string for root). Root-level `required` errors → instance_path = "" → renders `: must have required property 'X'` which matches the TS Ajv `instancePath` of "" exactly (Ajv emits instancePath "" for root required, falsy → falls back to schemaPath `#/required`). DIVERGENCE RISK: TS would fall back to schemaPath `#/required` when instancePath is "". CAPTURE AS QUESTION — see below.

## OPEN QUESTION / divergence risk
- TS `path = err.instancePath || err.schemaPath`: for a root-level missing-required error, Ajv sets `instancePath = ""` (falsy) → path becomes the schemaPath (e.g. `#/required`), so the TS message would be `#/required: must have required property 'solutionSpace'`, NOT `: must have required property 'solutionSpace'`. Need to confirm against captured `node dist/index.js` output before locking the expected string in the test. Flag to supervisor.

## New files this work unit produces (6 artifacts + features)
1. `codelet/fspec-core/src/commands/validate_foundation_schema.rs` (rewrite stub → real impl)
2. `codelet/fspec/src/validate_foundation_schema.rs` (CLI bridge)
3. `codelet/fspec/src/main.rs` Mode variant + arm + intercept (SUPERVISOR-owned)
4. `codelet/fspec-core/src/help/configs/validate_foundation_schema.rs`
5. `codelet/fspec/tests/fixtures/help/validate-foundation-schema.txt`
6. `codelet/fspec/tests/cli_validate_foundation_schema.rs`
+ core dispatcher test `codelet/fspec-core/tests/validate_foundation_schema.rs`

## Shared-file change requests (SUPERVISOR)
- `canonical.rs`: already lists validate-foundation-schema; add to PORTED_COMMANDS.
- `dispatch.rs`: change arm `commands::validate_foundation_schema::run(args_json)` → `run(args_json, project_root)` and move from run_stub to run_ported.
- `commands/mod.rs`: already registers module (stub exists).
- `help/configs/mod.rs`: register new config.
- `main.rs`: Mode::ValidateFoundationSchema variant, forward! arm, intercept arm, `mod validate_foundation_schema;`.
