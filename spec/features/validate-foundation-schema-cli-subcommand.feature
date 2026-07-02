@validation
@cli
@rust
@wip
@RPC-321
Feature: Validate foundation schema CLI subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::validate_foundation_schema::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes NO flags — mirroring the TypeScript Commander.js registration at src/commands/validate-foundation-schema.ts:138-144 which only declares `.command('validate-foundation-schema').description('Validate foundation.json against JSON Schema').action(...)` with no `.option(...)` calls.

  The CLI bridge prints result.output to stdout and exits 0 on success; on failure it writes 'Error:' followed by the joined error messages to stderr and exits 1 (parity with validateFoundationSchemaCommand at src/commands/validate-foundation-schema.ts:119-136).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec validate-foundation-schema` directly from a shell with the same flag-less surface offered by the TypeScript Commander.js CLI
    So that I can verify my foundation document from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: CLI validates a valid foundation and exits 0
    Given spec/foundation.json contains a schema-valid minimal foundation in the working directory
    When I run `./codelet/target/release/fspec validate-foundation-schema` from that directory
    Then the command exits 0
    Then stdout contains the substring '✓ foundation.json is valid according to the schema'

  Scenario: CLI exits 1 and writes to stderr when foundation.json is missing
    Given an empty directory with no spec/foundation.json is set as the current working directory
    When I run `./codelet/target/release/fspec validate-foundation-schema` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'foundation.json not found in spec/ directory'

  Scenario: CLI exits 1 when foundation.json violates the schema
    Given spec/foundation.json has an empty solutionSpace.capabilities array in the working directory
    When I run `./codelet/target/release/fspec validate-foundation-schema` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Field solutionSpace.capabilities must have at least 1 items (found 0)'

  Scenario: Clap exposes validate-foundation-schema as a subcommand and prints help byte-identical to the TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec validate-foundation-schema --help` from a shell
    Then the command exits 0
    Then stdout is byte-for-byte identical to the captured TS formatCommandHelp reference fixture
    Then stdout does NOT contain the substring '--format'
