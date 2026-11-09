@done
@cli
@documentation
@help-system
@CLI-016
Feature: Guidance mentions parse with the real CLI
  """
  None of fspec's guidance is generated from the clap registry: the bootstrap text, the init docs, reminders, command output and the per-command help (CommandHelpConfig) are hand-written, and much of it was captured from the TypeScript CLI. A contract test in the fspec binary crate (rust/fspec/src/guidance_contract_tests.rs, run by cargo test -p codelet-fspec --bin fspec) collects every 'fspec <command> ...' mention from those sources with exact extraction rules and parses it with Cli::try_parse_from; nothing runs. Unknown commands, help topics and flags fail with the file and line. Help configs are checked structurally (examples parse fully, option flags and related commands exist), and the help fixtures under rust/fspec/tests/fixtures/help must equal the help rendered from their config. Deterministic exact checks only; no semantic judgement. Supersedes the check halves of BUG-531, BUG-537 and BUG-538.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Sources: bootstrap_doc.txt, the init_doc_*.md files, every non-test string literal in rust/fspec-core/src and rust/fspec/src (slash-command templates, reminders, command output, generated-file headers; files named *test*.rs, tests.rs and items under #[cfg(test)] are test code), and every CommandHelpConfig
  #   2. A mention is the word 'fspec ' (not preceded by a letter, digit, '-', '_', '/' or '.') followed by a command token that starts with a letter and contains a hyphen or is a registered subcommand. It ends at a line break (a line ending in ' \' continues), at the next 'fspec ', after punctuation glued to the command token, at the closing backtick or quote when one precedes 'fspec', and outside "..." values at '|', ' #', two spaces, ' (', ' - ', ' → ', '. ', ', ', ' > ' or ' -->'. Trailing '.', ',', ';', ':' and ')' are dropped
  #   3. Tokens are split on whitespace with "..." and '...' kept together. A <...>, {...} or __NAME__ placeholder becomes the dummy value X; [...] optional groups and ... are dropped
  #   4. Each mention is parsed against the real clap tree (parse only, nothing runs). An unknown command or help topic (InvalidSubcommand) and an unknown flag (UnknownArgument on an argument starting with '-') fail the test, naming the file, line and mention; in prose a missing required argument, a dummy value a flag rejects and words after the arguments are accepted
  #   5. Every CommandHelpConfig example command parses fully (only a help or version display may stop it); every option flag exists on that command; every related command is a registered subcommand
  #   6. A help fixture follows its config: every rust/fspec/tests/fixtures/help/<command>.txt equals the help rendered from that command's CommandHelpConfig; the TypeScript captures are no longer the canon
  #   7. The check is a unit test of the fspec binary crate (src/guidance_contract_tests.rs; cargo test -p codelet-fspec --bin fspec), because Cli lives in the binary. Every violation found is fixed in this card by correcting the text to the real command, never by loosening the check
  #
  # EXAMPLES:
  #   1. The specifying reminder says 'fspec research --tool=stakeholder --platform=teams --question=...'; research has no --platform or --question, so the test fails naming reminders/specifying.rs and the line until the text uses the real flags
  #   2. bootstrap_doc.txt tells agents to run fspec restore-rule <id> --ids 0,1; restore-rule takes a positional index, so the test fails until the text says fspec restore-rule <id> <index>
  #   3. The audit-coverage help config shows the example 'fspec audit-coverage user-login --fix'; audit-coverage has no --fix, so the test fails until the example is removed or corrected
  #   4. A prose line 'run fspec add-rule <id>' parses with a missing-argument error and passes; 'run fspec link-feature X' fails as an unknown command
  #   5. fixtures/help/show-foundation.txt differs from the help rendered from the show-foundation config, so the test fails until the fixture is regenerated from the config
  #
  # ========================================
  Background: User Story
    As a AI agent following fspec's guidance and help
    I want to run every fspec command, flag and example the guidance names
    So that no instruction ends in an unknown command or unknown option error

  Scenario: Every mention in the guidance sources parses with the real CLI
    Given the bootstrap text, the init docs and the non-test string literals of fspec-core and fspec
    When every fspec mention in them is parsed with the real CLI
    Then no mention names an unknown command, help topic or flag

  Scenario: An unknown command fails while missing arguments pass in prose
    Given a source line "run fspec add-rule <id>" and a source line that runs the removed command link-feature with fspec
    When the mentions are checked
    Then the add-rule mention passes
    And the link-feature mention fails naming the source, the line and "unknown command"

  Scenario: An unknown flag fails
    Given a source line "fspec research --tool=stakeholder --platform=teams"
    When the mention is checked
    Then it fails naming the source, the line and the flag --platform

  Scenario: Mention boundaries and placeholders are exact
    Given source text with mentions in backticks, in quotes, in a table cell, at the end of a sentence and with placeholders and optional groups
    When the mentions are extracted
    Then each mention ends at its closing backtick or quote, at the '|', or before the trailing punctuation
    And placeholders become X while optional groups and ... are dropped

  Scenario: Help configs name only real examples, options and related commands
    Given every command's CommandHelpConfig
    When its examples, option flags and related commands are checked against the CLI
    Then every example parses fully, every option flag exists on its command and every related command exists

  Scenario: Help fixtures follow their configs
    Given the help fixtures under rust/fspec/tests/fixtures/help
    When each fixture is compared with the help rendered from its command's config
    Then every fixture equals its rendered help
