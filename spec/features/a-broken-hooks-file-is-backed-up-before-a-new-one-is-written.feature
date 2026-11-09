@done
@validation
@BUG-611
@hooks
@safety
Feature: A broken hooks file is backed up before a new one is written
  """
  rust/fspec-core/src/command_hooks/config_file.rs (new): load_for_write(project_root, command) reads spec/fspec-hooks.json under the caller's spec lock and returns the config map plus an optional backup notice; a broken file (rule 0) is renamed to its .broken-<timestamp> name (never over an existing file) before the caller's atomic write. add_hook.rs and remove_hook.rs use it and return the notice as their output (the CLI bridges print it). list_hooks.rs and validate_hooks.rs use the same broken-file check read-only.
  Rule 6 also covers use-native-serde-json-errors-with-good-diagnostics-for-malformed-json-remove-v8-style-emulation's 'Lenient parse sites stay silent on malformed input' (list-hooks on a malformed fspec-hooks.json returned an empty list): found while implementing, deleted with its test in json_parse_diagnostics.rs, and that feature's rule/example text no longer lists list-hooks or validate-hooks as lenient
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. spec/fspec-hooks.json is broken when it is not valid JSON, its top-level value is not an object, its 'hooks' value is not an object, or an event's value under 'hooks' is not an array (each of these was silently replaced before)
  #   2. A hook-config writer (add-hook, remove-hook) that finds a broken spec/fspec-hooks.json moves it to spec/fspec-hooks.json.broken-<timestamp> (UTC, %Y%m%dT%H%M%S.%3fZ, e.g. 20261108T013000.123Z), prints the notice, then writes a new configuration holding only its own change; the backup keeps the broken bytes exactly (product owner, 2026-11-07)
  #   3. The notice is exactly: spec/fspec-hooks.json could not be parsed (<reason>); it was moved to spec/fspec-hooks.json.broken-<timestamp> and a new hook configuration was written. <reason> is the JSON parser's message or the shape problem ('the top-level value is not an object', 'hooks is not an object', 'hooks.<event> is not an array')
  #   4. If the backup name already exists the writer fails with 'spec/fspec-hooks.json could not be parsed (<reason>), and its backup spec/fspec-hooks.json.broken-<timestamp> already exists; nothing was changed' and changes nothing (a backup is never overwritten)
  #   5. list-hooks and validate-hooks never change a broken spec/fspec-hooks.json: list-hooks fails with 'spec/fspec-hooks.json could not be parsed (<reason>); the file was not changed' and validate-hooks reports valid=false, exitCode 1 with that message (product owner, 2026-11-07)
  #   6. The earlier scenarios that silently replaced or ignored a broken file (add-hook-rust-port, list-hooks-rust-port, list-hooks-cli-subcommand, validate-hooks-rust-port) and remove-hook's refuse-to-write scenarios (remove-hook-rust-port, remove-hook-cli-subcommand) are deleted with their tests and coverage, never tagged @deprecated (product owner, 2026-11-08)
  #
  # EXAMPLES:
  #   1. spec/fspec-hooks.json holds '{ not json'; fspec add-hook pre-implementing lint --command lint.sh prints the notice naming the backup, spec/fspec-hooks.json.broken-<ts> holds exactly '{ not json', and spec/fspec-hooks.json holds only the lint hook
  #   2. spec/fspec-hooks.json holds {"hooks": []}; add-hook backs it up with reason 'hooks is not an object'
  #   3. spec/fspec-hooks.json holds '{ not json'; fspec remove-hook pre-implementing lint backs it up, prints the notice and writes {"hooks": {}}
  #   4. A backup with the same timestamp already exists: add-hook fails naming it, and both files are unchanged
  #   5. spec/fspec-hooks.json holds '{ not json'; fspec list-hooks fails with the not-changed message and validate-hooks reports valid=false, exitCode 1 with it; the file still holds '{ not json' and no backup exists
  #
  # ========================================
  Background: User Story
    As a developer whose spec/fspec-hooks.json got corrupted
    I want to have fspec keep the broken file and tell me exactly what it did before it writes a new hook configuration
    So that a hook command never silently throws away every hook I configured

  Scenario: add-hook backs up an unparseable hooks file and writes a new one
    Given spec/fspec-hooks.json holds "{ not json"
    When I run "fspec add-hook pre-implementing lint --command lint.sh"
    Then it prints "spec/fspec-hooks.json could not be parsed (<reason>); it was moved to spec/fspec-hooks.json.broken-<timestamp> and a new hook configuration was written" naming the backup and the JSON parser's message
    And the backup holds exactly "{ not json"
    And spec/fspec-hooks.json holds only the hook "lint" under "pre-implementing"

  Scenario: A hooks file of the wrong shape is backed up like an unparseable one
    Given spec/fspec-hooks.json holds {"hooks": []}
    When I run "fspec add-hook pre-implementing lint --command lint.sh"
    Then it prints the backup notice with the reason "hooks is not an object"
    And the backup holds exactly {"hooks": []}

  Scenario: remove-hook backs up an unparseable hooks file and writes an empty configuration
    Given spec/fspec-hooks.json holds "{ not json"
    When I run "fspec remove-hook pre-implementing lint"
    Then it prints the backup notice naming the backup
    And the backup holds exactly "{ not json"
    And spec/fspec-hooks.json holds {"hooks": {}}

  Scenario: An existing backup is never overwritten
    Given spec/fspec-hooks.json holds "{ not json"
    And its backup name for the current time already exists
    When I run "fspec add-hook pre-implementing lint --command lint.sh"
    Then it fails with "spec/fspec-hooks.json could not be parsed (<reason>), and its backup spec/fspec-hooks.json.broken-<timestamp> already exists; nothing was changed"
    And spec/fspec-hooks.json and the existing backup are unchanged

  Scenario: list-hooks and validate-hooks report a broken hooks file without changing it
    Given spec/fspec-hooks.json holds "{ not json"
    When I run "fspec list-hooks"
    Then it fails with "spec/fspec-hooks.json could not be parsed (<reason>); the file was not changed"
    When I run "fspec validate-hooks"
    Then it reports valid=false and exitCode 1 with that message
    And spec/fspec-hooks.json still holds "{ not json" and no backup exists
