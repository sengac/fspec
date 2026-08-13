@validation
@cli
@rust
@wip
@RPC-321
Feature: Port validate-foundation-schema command to Rust
  """
  Reuse the existing native validator at rust/fspec-core/src/generators/foundation_schema.rs (validate_foundation + SchemaError) which already mirrors the Ajv draft-07 subset against the bundled generic-foundation.schema.json — do NOT add a JSON-schema crate. The TS minItems special-case (instancePath stripped of leading '/' and '/' → '.') must be applied in the command layer, NOT in the shared validator, because validate_foundation/format_errors produce the generic 'instancePath: message' form used by generate-foundation-md.
  minItems parity detail: the native validator emits message 'must NOT have fewer than <limit> items' (Ajv standard) but the TS validate-foundation-schema command formats minItems errors as 'Field <dotted.path> must have at least <limit> items (found <n>)'. SchemaError carries instance_path + message only (no params.limit, no data.length). The command layer must (a) detect the message prefix 'must NOT have fewer than ', (b) parse the <limit> from it, (c) re-fetch the actual array length by re-reading the offending node from the parsed foundation via instance_path, OR re-implement a small minItems-aware error mapper. Decision: parse limit from the message and resolve the actual array length by walking the parsed JSON along instance_path — keeps the shared validator untouched.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The command MUST read spec/foundation.json relative to the project root; if the file does not exist it MUST return success=false with the error message 'foundation.json not found in spec/ directory' (parity with the ENOENT branch at src/commands/validate-foundation-schema.ts:104-110)
  #   2. The command MUST validate the parsed foundation against the bundled generic-foundation.schema.json using a hand-rolled structural validator (reuse generators::foundation_schema::validate_foundation) rather than adding a JSON-schema crate, mirroring the specific Ajv checks the TS performs
  #   3. On successful validation the command MUST emit exactly '✓ foundation.json is valid according to the schema' (parity with the success.output string at src/commands/validate-foundation-schema.ts:97-100) and exit 0
  #   4. When validation fails the command MUST return success=false with errorMessages joined by newline; each error renders as 'instancePath: message' EXCEPT (a) minItems violations which render as 'Field <dotted.path> must have at least <limit> items (found <n>)' and (b) root-level errors whose instancePath is "" which fall back to the Ajv schemaPath (e.g. a missing top-level required property renders '#/required: must have required property \'X\'') — parity with the `path = err.instancePath || err.schemaPath` and `err.keyword==='minItems'` special-cases at src/commands/validate-foundation-schema.ts:79-89, and exit 1
  #   5. If spec/foundation.json contains malformed JSON the command MUST return success=false with the error 'Failed to validate foundation schema: <message>' (parity with the generic catch branch at src/commands/validate-foundation-schema.ts:112-115) and exit 1
  #   6. The Rust dispatcher route for 'validate-foundation-schema' MUST replace the NotYetPorted stub, receive the project_root (signature changes from run(args_json) to run(args_json, project_root)), and resolve through the same poll_sync_future path
  #   7. The standalone fspec binary MUST expose 'validate-foundation-schema' as a clap subcommand with NO flags (matching the flag-less TS Commander.js registration at src/commands/validate-foundation-schema.ts:138-144) and the CLI bridge MUST delegate to the same fspec_core::commands::validate_foundation_schema::run function (two front doors, one source of truth)
  #   8. Running `fspec validate-foundation-schema --help` MUST print help byte-for-byte identical to the TS formatCommandHelp output captured from `node dist/index.js validate-foundation-schema --help` piped to non-TTY
  #
  # EXAMPLES:
  #   1. Dispatch validate-foundation-schema against a tempdir whose spec/foundation.json is a schema-valid minimal foundation (version, project, problemSpace, solutionSpace with 1 capability) → returns success=true and output '✓ foundation.json is valid according to the schema'
  #   2. Dispatch against a tempdir with NO spec/foundation.json → returns success=false with error 'foundation.json not found in spec/ directory'
  #   3. Dispatch against a tempdir whose foundation.json has solutionSpace.capabilities=[] (empty) → returns success=false with error 'Field solutionSpace.capabilities must have at least 1 items (found 0)' (minItems special-case rendering)
  #   4. Dispatch against a tempdir whose foundation.json is missing the required 'solutionSpace' property → returns success=false with error '#/required: must have required property \'solutionSpace\'' (TS Ajv renders root-level required errors via instancePath||schemaPath, and instancePath is "" at root so it falls back to schemaPath '#/required' — captured byte-exact from node dist/index.js)
  #   5. Dispatch against a tempdir whose spec/foundation.json contains the malformed bytes '{ not json' → returns success=false with error beginning 'Failed to validate foundation schema:'
  #   6. Running `./rust/target/release/fspec validate-foundation-schema` in a directory with a valid foundation.json prints '✓ foundation.json is valid according to the schema' to stdout and exits 0
  #   7. Running `./rust/target/release/fspec validate-foundation-schema` in a directory with NO foundation.json prints 'Error: foundation.json not found in spec/ directory' to stderr and exits 1
  #   8. Running `./rust/target/release/fspec validate-foundation-schema --help` prints the formatted help block (header VALIDATE-FOUNDATION-SCHEMA, description, USAGE, EXAMPLES, RELATED COMMANDS, NOTES) byte-identical to the TS fixture and exits 0
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to validate spec/foundation.json against the generic-foundation JSON schema from both the LLM dispatcher and the shell CLI
    So that I can catch malformed foundation documents without relying on Node.js, sharing one source-of-truth structural validator

  Scenario: Validates a schema-valid foundation and reports success
    Given spec/foundation.json contains a schema-valid minimal foundation with version, project, problemSpace, and solutionSpace with one capability
    When I dispatch the validate-foundation-schema command against that project root
    Then the dispatcher returns success=true with the output '✓ foundation.json is valid according to the schema'

  Scenario: Reports a friendly error when foundation.json is missing
    Given an empty project root directory with no spec/foundation.json
    When I dispatch the validate-foundation-schema command against that project root
    Then the dispatcher returns success=false with the error 'foundation.json not found in spec/ directory'

  Scenario: Renders the minItems special-case error for an empty capabilities array
    Given spec/foundation.json is valid except solutionSpace.capabilities is an empty array
    When I dispatch the validate-foundation-schema command against that project root
    Then the dispatcher returns success=false with the error 'Field solutionSpace.capabilities must have at least 1 items (found 0)'

  Scenario: Renders a required-property error when a top-level field is missing
    Given spec/foundation.json is valid except it is missing the required solutionSpace property
    When I dispatch the validate-foundation-schema command against that project root
    Then the dispatcher returns success=false with the error "#/required: must have required property 'solutionSpace'"

  Scenario: Reports a friendly error when foundation.json contains malformed JSON
    Given spec/foundation.json exists but contains the malformed bytes '{ not json'
    When I dispatch the validate-foundation-schema command against that project root
    Then the dispatcher returns success=false with an error beginning 'Failed to validate foundation schema:'
