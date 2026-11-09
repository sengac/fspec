@BUG-582
@done
@BUG-583
@BUG-490
@REFAC-020
@cli
@integration
@validation
@coverage-tracking
@code-quality
@phase-1
@critical
@BUG-457
Feature: Preserve CLI contracts while integrating precise coverage
  """
  BUG-457: the --token field BUG-429 added to client and status (and its
  dispatch bindings and call arguments) is an approved addition
  (tests/refac020/source.rs ADDED_TOKEN_FIELDS / StripAddedTokenRouting).
  BUG-583: three later cards are recorded the same way. BUG-537's
  show-foundation --section field and its routing (section.or(section_flag));
  the functions added after the freeze (ADDED_FUNCTIONS: HOOK-018's
  command_hook_target and run_cli, CLI-016's ts_help_text and ts_help_config);
  HOOK-018's main, compared as the frozen main once its recorded ArgMatches
  parse and command-hook wrapper are replaced by Cli::try_parse and run_cli's
  body; and CLI-016's help interception, compared as its exact table of
  command name to help config or static text. Any change not recorded fails.
  Root approved the bounded REFAC-020 CLI registration decomposition and temp-fixture subprocess tests. Freeze all 169 Mode variants, parser attributes/defaults, ordered dispatch arms, startup/error/help contracts and compiled Clap command tree before production changes. Keep original module declarations at crate root and all touched maintained Rust sources strictly below 300 physical lines. Whole-item composition must assemble the complete enum or match before expansion and explicitly forward caller expressions. The four thin CLI bridges marshal testMappingIndex, expectedScenarioHash, featureName/dryRun and opt-in mappingJson to the existing shared core run entrypoints; no selector, canonical-hash, mutation, generation or locking business logic belongs in the CLI. Preserve legacy format=json and CWD resolution, including non-global top-level workspace. The core owner manages help configs and REFAC-020 status; parent exclusively runs Cargo and reviews causal RED before production. Original help fixtures remain immutable; additive help expectations are explicit. No installed binary, real repository migration, daemon restart or rust/cli BUG-284 changes.
  """

  Background: User Story
    As a maintainer of the standalone fspec CLI
    I want precise coverage operations without changing existing command contracts
    So that validated coverage repairs preserve unrelated workflows and metadata

  Scenario: Ordered command and argument declarations retain their frozen contract
    Given the frozen original parser and all 169 ordered command variants
    When the actual maintained command declarations are resolved
    Then all original tokens and order remain with only the approved coverage options added

  Scenario: Dispatch startup errors and help retain their frozen contract
    Given the frozen ordered dispatch arms startup shell error helpers and help interception
    When the actual maintained registration composition is resolved
    Then every original destination argument expression and output contract remains unchanged

  Scenario: Maintained CLI integration sources remain below the physical ceiling
    Given the CLI root four coverage bridges and reachable registration sources
    When their raw maintained physical source lines are counted
    Then every targeted maintained source has fewer than 300 lines without hiding oversized source

  Scenario: Compiled Clap registration retains its complete ordered tree
    Given a compiled parser harness using frozen original declarations
    When the actual CLI declarations are compiled and their command argument tree is traversed
    Then ordered commands aliases defaults actions arity requiredness and scope match the frozen tree except approved additions

  Scenario: A later change the contract does not record still fails it
    Given the maintained CLI source with one unrecorded change: an added function, a statement added to main, or a help interception entry pointing at another config
    When the frozen dispatch, startup and help contract compares it with the frozen original
    Then the comparison fails and names the function or the help entry that changed
