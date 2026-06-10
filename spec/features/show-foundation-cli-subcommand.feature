@done
@querying
@cli
@RPC-305
Feature: show-foundation clap subcommand on the standalone fspec Rust binary

  """
  CLI surface for the `show-foundation` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (architecture note [7] on RPC-253, reused for RPC-305):
    - Shell argv         → clap → codelet/fspec/src/show_foundation.rs → fspec_core::commands::show_foundation::run
    - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::show_foundation::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes one optional positional `[section]` argument and the following flags: `--section <section>` (alias for the positional), `--format <text|json>` (default 'text'), `--output <file>`, `--draft`, `--list-sections`, `--line-numbers`. The latter two are advertised but no-op (parity with the TS source which advertises them but does not implement them in showFoundationCommand).
  Text format prints the formatted content to stdout. JSON format prints 2-space pretty-printed JSON. When --output <file> is provided, the formatted content is written to that file and stdout prints '✓ Output written to <file>'.
  Exit-code contract: 0 on success, 1 on any error (field-not-found, missing draft, parse error) with stderr prefixed 'Error:'.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/show-foundation.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a show-foundation clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that foundation rendering is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes show-foundation as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-foundation --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'show-foundation'
    Then stdout advertises the optional positional <section> argument
    Then stdout advertises the '--format' flag
    Then stdout advertises the '--output' flag
    Then stdout advertises the '--draft' flag
    Then stdout advertises the '--list-sections' flag
    Then stdout advertises the '--line-numbers' flag

  Scenario: CLI default render prints PROJECT section to stdout
    Given spec/foundation.json contains project.name='fspec', project.vision='V', project.projectType='cli-tool'
    When I run `./codelet/target/release/fspec show-foundation` from that directory
    Then the command exits 0
    Then stdout contains the exact line '=== PROJECT ==='
    Then stdout contains the line 'Name: fspec'

  Scenario: CLI positional section emits raw string in text format
    Given spec/foundation.json contains project.name='fspec'
    When I run `./codelet/target/release/fspec show-foundation projectName` from that directory
    Then the command exits 0
    Then stdout equals exactly 'fspec' (with a trailing newline)

  Scenario: CLI --format=json emits JSON
    Given spec/foundation.json contains project.name='fspec'
    When I run `./codelet/target/release/fspec show-foundation projectName --format json` from that directory
    Then the command exits 0
    Then stdout starts with the bytes '"fspec"'

  Scenario: CLI exits 1 when section is unknown
    Given spec/foundation.json contains project.name='fspec'
    When I run `./codelet/target/release/fspec show-foundation nonexistent` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring "Field 'nonexistent' not found"

  Scenario: CLI --draft surfaces missing-draft error
    Given spec/foundation.json.draft does NOT exist in the working directory
    When I run `./codelet/target/release/fspec show-foundation --draft` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'No draft found at spec/foundation.json.draft'

  Scenario: CLI --output writes file and stdout prints success line
    Given spec/foundation.json contains project.name='fspec'
    When I run `./codelet/target/release/fspec show-foundation projectName --output out/name.txt` from that directory
    Then the command exits 0
    Then the file out/name.txt exists with the exact bytes 'fspec'
    Then stdout contains the substring '✓'
    Then stdout contains the substring 'Output written to out/name.txt'

  Scenario: CLI exits 1 and writes to stderr when foundation.json is malformed
    Given spec/foundation.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec show-foundation` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'

  Scenario: CLI --list-sections is parsed but ignored (no-op parity)
    Given spec/foundation.json contains project.name='fspec'
    When I run `./codelet/target/release/fspec show-foundation --list-sections` from that directory
    Then the command exits 0
    Then stdout contains the exact line '=== PROJECT ==='

  Scenario: CLI --line-numbers is parsed but ignored (no-op parity)
    Given spec/foundation.json contains project.name='fspec'
    When I run `./codelet/target/release/fspec show-foundation --line-numbers` from that directory
    Then the command exits 0
    Then stdout contains the exact line '=== PROJECT ==='

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/foundation.json contains project.name='fspec'
    When I dispatch show-foundation through fspec_core::dispatch::dispatch_command with section='projectName' and format='json'
    Then the DispatchResult.data equals exactly '"fspec"'
    Then the CLI bridge module codelet/fspec/src/show_foundation.rs contains NO inline FIELD_MAP, formatter, or filesystem logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: show-foundation --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-foundation --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-foundation.txt
    Then stdout starts with a blank line followed by 'SHOW-FOUNDATION'
