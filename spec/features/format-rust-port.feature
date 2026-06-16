@formatting
@done
@formatter
@parser
@RPC-230
Feature: Port format command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/format.rs parses each target feature file with crate::io::gherkin and re-emits it through a hand-ported AST-based Gherkin formatter (new module codelet/fspec-core/src/io/gherkin_format.rs) that reproduces src/utils/gherkin-formatter.ts byte-for-byte: 2-space scenario / 4-space step indentation, per-column-aligned tables, preserved doc strings and tags, blank line before each feature child and before each Examples block, single trailing newline. With no file argument it globs spec/features/**/*.feature (empty → formattedCount=0, no error) and skips unparseable files with a warning; with a file argument it formats only that file and a missing file surfaces 'File not found: <file>'. Returns the JSON envelope {formattedCount}. The CLI bridge owns rendering: 'No feature files found to format' / '✓ Formatted <file>' / '✓ Formatted N feature files' / 'Error: <message>'. format has a rich -help.ts → a normal CommandHelpConfig module renders byte-for-byte parity.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. With no file argument, all spec/features/**/*.feature files are formatted; with a file argument only that file is formatted (a missing file throws 'File not found: <file>')
  #   2. Each file is parsed to a Gherkin AST and re-emitted by the formatter (2-space scenario / 4-space step indentation, aligned tables, preserved doc strings and tags, single trailing newline); formattedCount counts successfully formatted files
  #   3. When no feature files exist (no file argument), formattedCount=0 and the command exits 0; in all-files mode, files that fail to parse are skipped with a warning and do not abort the run
  #   4. CLI: no files → 'No feature files found to format' (exit 0); file argument → '✓ Formatted <file>'; otherwise green '✓ Formatted N feature files'; a thrown error (e.g. missing single file) → 'Error: <message>' to stderr, exit 1
  #
  # EXAMPLES:
  #   1. Dispatcher formatting a workspace with two well-formed feature files returns formattedCount=2 and both files are rewritten in canonical formatter layout
  #   2. Dispatcher formatting a single supplied file rewrites only that file and returns formattedCount=1
  #   3. Dispatcher formatting a workspace with no feature files returns formattedCount=0
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to format feature files with the AST-based Gherkin formatter via both the LLM dispatcher and the shell CLI
    So that I can keep my feature files consistently formatted with byte-for-byte parity to the TypeScript implementation without relying on Node.js

  Scenario: Dispatcher formats all feature files in a workspace
    Given a project root tempdir with two well-formed feature files under spec/features
    When I dispatch format with no file argument
    Then the dispatcher returns formattedCount=2
    And both feature files are rewritten in the canonical formatter layout

  Scenario: Dispatcher formats a single supplied file
    Given a project root tempdir with two feature files under spec/features
    When I dispatch format with file=spec/features/one.feature
    Then the dispatcher returns formattedCount=1
    And only that file is rewritten

  Scenario: Dispatcher returns zero when no feature files exist
    Given a project root tempdir with no feature files under spec/features
    When I dispatch format with no file argument
    Then the dispatcher returns formattedCount=0

  Scenario: Dispatcher errors when the supplied file does not exist
    Given a project root tempdir with no spec/features/missing.feature file
    When I dispatch format with file=spec/features/missing.feature
    Then the dispatcher returns an error mentioning 'File not found'

  Scenario: Dispatcher output is idempotent
    Given a project root tempdir with one feature file that is already canonically formatted
    When I dispatch format twice over that file
    Then the file content is identical after both runs
