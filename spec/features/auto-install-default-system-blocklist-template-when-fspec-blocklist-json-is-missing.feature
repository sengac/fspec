@done
@tools
@security
@auto-create
@default
@rust
@BLOCK-012
Feature: Auto-install default system blocklist template when ~/.fspec/blocklist.json is missing
  """
  Template embedded via include_str! of rust/tools/data/default-blocklist.json in a new codelet-tools module (blocklist/template.rs) mirroring the codex_allowlist.rs precedent. Check-then-write lives in middleware.rs::load_blocklist_config (the single chokepoint every check_bash_command/check_file_path/init_blocklist call funnels through): if system_config_path() exists() is false, install_default_system_blocklist() writes the template (create_dir_all on parent) BEFORE the load, so the fresh file is loaded and matched in the same synchronous call. install failures (no HOME, read-only fs) are tracing::warn! and swallowed so checking never breaks. Existing user file is never touched (guarded by !exists()). Because load_blocklist_config hot-reloads on every check, deleting the file mid-session re-triggers install on the next check (idempotent). Tests use a HomeGuard RAII (redirect HOME to a fresh tempdir; on drop restore HOME + clear_session_allowances + init_blocklist(None)) and are #[serial], mirroring blocklist_init_tests.rs (RPC-407).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The default blocklist template (the contents of ~/blocklist.json: version 1.0.0 with 68 rules covering Windows and Linux dangerous commands, sensitive file paths, and agent-loop protection) is embedded in the codelet-tools binary at compile time via include_str! of a bundled data file (rust/tools/data/default-blocklist.json), so no external file is needed at runtime
  #   2. The template is written to ~/.fspec/blocklist.json ONLY when that file does not already exist — an existing user-edited blocklist is never overwritten
  #   3. The check-then-write happens inside load_blocklist_config() (the single chokepoint every check_bash_command/check_file_path call passes through) — the moment the system path is checked and found missing, the template is written and then loaded, so the new rules are active in the very same process and the very same check call
  #   4. The embedded template must be valid BlocklistConfig JSON — a unit test asserts it deserializes via serde_json into BlocklistConfig with version "1.0.0" and exactly 68 rules, so a broken template fails the build's test suite rather than silently degrading at runtime
  #   5. Write failures (missing home dir, read-only filesystem, permission errors) degrade gracefully: the install is skipped with a tracing::warn! log and the check proceeds with whatever configs could be loaded — a failed template install must never break command checking
  #   6. Because load_blocklist_config() hot-reloads on every check, the missing-file check is re-evaluated on each call: if the user deletes ~/.fspec/blocklist.json mid-session, the next check re-installs the template — the check-then-write is idempotent and safe to repeat
  #   7. The install creates the ~/.fspec parent directory if it does not exist (create_dir_all on the parent) before writing the file, so first-run users who have never had a ~/.fspec dir still get the template
  #
  # EXAMPLES:
  #   1. First run on a fresh machine: HOME points at an empty temp dir with no ~/.fspec/blocklist.json. load_blocklist_config() is called (via init_blocklist or check_bash_command). The template is written to ~/.fspec/blocklist.json and the returned config contains all 68 default rules — a command matching 'git checkout' is blocked in that same call
  #   2. User already has a custom ~/.fspec/blocklist.json (e.g., 3 rules they edited). load_blocklist_config() is called. The file is NOT overwritten — the config loaded contains exactly the user's 3 rules, and the file on disk is byte-identical to before
  #   3. User deletes ~/.fspec/blocklist.json mid-session. The next check_bash_command() call re-detects the missing file, re-installs the template, and blocking works again without a process restart
  #
  # ========================================
  Background: User Story
    As a developer (or AI agent operator)
    I want to have a default ~/.fspec/blocklist.json auto-installed from the embedded template the first time the blocklist is loaded
    So that sensible default protections for dangerous commands and sensitive files are active on every machine without any manual setup

  # ========================================
  # TEMPLATE EMBEDDING
  # ========================================
  Scenario: Embedded template is valid and complete
    Given the codelet-tools crate compiles with the bundled default blocklist template
    When the template is parsed as a BlocklistConfig
    Then the template has version "1.0.0"
    And the template contains exactly 68 rules
    And every rule id in the template is unique

  # ========================================
  # FIRST-RUN INSTALL
  # ========================================
  Scenario: Template is installed and active on first check when no system blocklist exists
    Given a fresh environment where "~/.fspec/blocklist.json" does not exist
    When the AI runs "git stash" via Bash
    Then "~/.fspec/blocklist.json" exists on disk
    And the file on disk parses as a BlocklistConfig with 68 rules
    And the command is blocked with rule id "git-stash-block"
    And the blocked error carries the reason from the template rule

  Scenario: Install creates the ~/.fspec parent directory when it is missing
    Given a fresh environment where the "~/.fspec" directory itself does not exist
    When the blocklist is loaded
    Then the "~/.fspec" directory is created
    And "~/.fspec/blocklist.json" contains the embedded template

  # ========================================
  # EXISTING FILE SAFETY
  # ========================================
  Scenario: Existing user blocklist is never overwritten
    Given a user blocklist at "~/.fspec/blocklist.json" containing a single block rule for pattern "sentinel-block012"
    When the blocklist is loaded
    Then the file on disk is byte-identical to the user's original file
    And running a command matching "sentinel-block012" is blocked by the user rule
    And the template rule "git-stash-block" is not present in the file on disk

  # ========================================
  # IDEMPOTENT RE-INSTALL
  # ========================================
  Scenario: Deleted system blocklist is re-installed on the next check
    Given a fresh environment where the first check already installed "~/.fspec/blocklist.json"
    When the system blocklist file is deleted
    And the AI runs "git stash" via Bash again
    Then "~/.fspec/blocklist.json" exists on disk again
    And the command is blocked with rule id "git-stash-block"

  # ========================================
  # GRACEFUL DEGRADATION
  # ========================================
  Scenario: Install failure degrades gracefully without breaking command checking
    Given a fresh environment where "~/.fspec" exists as a regular file so the template write fails
    When the AI runs "echo hello" via Bash
    Then the command check completes without error
    And no panic or install failure propagates to the caller
