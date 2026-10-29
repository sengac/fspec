@REFAC-020
@cli
@integration
@validation
@coverage-tracking
@code-quality
@phase-1
@critical
Feature: Preserve CLI contracts while integrating precise coverage
  """
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

  Scenario: Machine mapping readback is explicit and leaves legacy JSON unchanged
    Given a real temporary feature with duplicate mapping rows and unknown metadata
    When show coverage is invoked with mapping JSON and separately with legacy format JSON
    Then explicit readback supplies ordered indices and hashes while legacy output and sidecar bytes stay unchanged

  Scenario: Selected linking reaches the same shared core mutation as native dispatch
    Given identical temporary projects with validated test steps and duplicate coverage rows
    When CLI and native dispatch add an implementation to the selected second row with its current hash
    Then complete sidecar results agree and the first row and unrelated metadata retain their lineage

  Scenario: Selected unlinking removes one occurrence through the shared core
    Given identical temporary sidecars containing duplicate mapping occurrences
    When CLI and native dispatch unlink exactly the indexed row with its current hash
    Then complete sidecars agree and all retained mapping multiplicity and unknown metadata remain intact

  Scenario: Invalid selectors and ordinary validation failures never write coverage
    Given a temporary coverage project and its exact original sidecar bytes
    When the CLI receives stale malformed missing mismatched or out-of-range selectors or invalid files and test steps
    Then each invocation reports the appropriate failure and every original sidecar byte remains unchanged

  Scenario: Scoped generation previews and applies only the named feature through shared core
    Given temporary projects with one selected feature addition and unrelated missing or drifted sidecars
    When CLI and native dispatch preview then apply generation with the explicit feature name
    Then JSON plans and complete results agree while preview writes nothing and unrelated bytes remain untouched

  Scenario: Coverage commands retain CWD and non-global workspace behavior
    Given a temporary current directory and a conflicting workspace decoy
    When coverage commands receive workspace before and after the subcommand
    Then preceding workspace never redirects coverage from CWD and trailing workspace remains an unknown option

  Scenario: Additive coverage help and legacy CLI errors preserve stream and exit contracts
    Given frozen original help fixtures and explicit approved coverage options
    When the four coverage commands receive help unknown options and missing option values
    Then help explicitly advertises additions before parsing and original stdout stderr newline and exit behavior is preserved
