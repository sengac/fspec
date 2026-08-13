@done
@feature-management
@cli
@RPC-276
Feature: Port remove-init-files command to Rust
  """
  Core impl rust/fspec-core/src/commands/remove_init_files.rs: Args {keep_config: Option<bool>} (camelCase keepConfig). Local const AGENT table (20 agents) with id/docTemplate/slashCommandPath/slashCommandFormat/detectionPaths — inlined to avoid touching shared mod.rs/lib.rs (pending supervisor confirmation).
  detect_installed_agent(project_root): read spec/fspec-config.json as serde_json::Value, use .agent if present/parseable; else iterate AGENT table and pick first whose any detectionPaths entry exists under project_root. File deletion uses std::fs::remove_file with ErrorKind::NotFound tolerated (force:true parity). Does NOT use ensure_* or locked_file — these are unconditional rm -f.
  Result envelope {filesRemoved: Vec<String>} as JSON string. CLI bridge rust/fspec/src/remove_init_files.rs marshals --keep-config / --no-keep-config into keepConfig and renders success lines (exit 0) / error (exit 1). Help config rust/fspec-core/src/help/configs/remove_init_files.rs. Two-front-doors; dispatcher passes args_json verbatim.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Detect the installed agent: read spec/fspec-config.json and use its .agent field if present and parseable; otherwise scan each agent's detectionPaths and pick the first agent whose any detection path exists in cwd
  #   2. If no agent is detected, error 'No fspec agent installation detected. Nothing to remove.'; if the detected agent id is unknown, error 'Unknown agent: <id>'
  #   3. Remove agent files: spec/<docTemplate> (e.g. spec/CLAUDE.md) and <slashCommandPath><fspec.md|fspec.toml> (filename depends on slashCommandFormat); both use force removal so missing files are silently skipped (idempotent)
  #   4. The interactive Ink ConfirmPrompt (used when keepConfig is undefined in TS) is not reproducible in headless Rust; the Rust port treats an unspecified keepConfig as false (remove config), matching the destructive --no-keep-config default — see supervisor question
  #   5. Success output: '✓ Successfully removed fspec init files' then each removed file as '  - <path>', exit 0; error: stderr '✗ Failed to remove init files: <msg>', exit 1
  #   6. The command must NOT remove spec/features/, spec/work-units.json, or other project files — only agent docs, slash command files, and (optionally) fspec-config.json
  #
  # EXAMPLES:
  #   1. spec/fspec-config.json has agent='claude' -> removes spec/CLAUDE.md, .claude/commands/fspec.md, and spec/fspec-config.json
  #   2. No config but .gemini/ directory exists -> detects gemini, removes spec/GEMINI.md and .gemini/commands/fspec.toml (toml format)
  #   3. keepConfig=true with claude installed -> removes spec/CLAUDE.md and .claude/commands/fspec.md but NOT spec/fspec-config.json
  #   4. No agent files and no config -> error 'No fspec agent installation detected. Nothing to remove.' exit 1
  #   5. claude detected but spec/CLAUDE.md already deleted -> still succeeds, filesRemoved still lists the attempted paths (force removal is idempotent)
  #
  # QUESTIONS (ANSWERED):
  #   Q: @supervisor: No Rust port of AGENT_REGISTRY exists in fspec-core (init.rs is still a stub). I will create a local const agent table inside commands/remove_init_files.rs covering the needed fields (id, docTemplate, slashCommandPath, slashCommandFormat, detectionPaths) for the 20 agents — confirm this is acceptable vs. a new shared module rust/fspec-core/src/agents.rs (which would require touching lib.rs/mod). Also confirm the headless default for an unspecified keepConfig should be false (remove config).
  #   A: Working assumption pending supervisor confirmation: inline a local const AGENT table inside commands/remove_init_files.rs (no shared mod.rs/lib.rs changes); an unspecified keepConfig defaults to false (remove config), matching the destructive --no-keep-config default.
  #
  # ASSUMPTIONS:
  #   1. Working assumption pending supervisor confirmation: inline a local const AGENT table inside commands/remove_init_files.rs (no shared mod.rs/lib.rs changes); an unspecified keepConfig defaults to false (remove config), matching the destructive --no-keep-config default.
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of remove-init-files wired through both the LLM dispatcher and the clap subcommand
    So that the standalone Rust binary and the daemon share one agent-uninstall implementation with byte-parity to the TS exported function

  Scenario: Removes agent files and config when the config names claude
    Given a workspace with spec/fspec-config.json containing agent='claude'
    And the files spec/CLAUDE.md and .claude/commands/fspec.md exist
    When I dispatch remove-init-files with no keepConfig
    Then the dispatcher returns success=true
    And the returned JSON filesRemoved includes 'spec/CLAUDE.md'
    And the returned JSON filesRemoved includes '.claude/commands/fspec.md'
    And the returned JSON filesRemoved includes 'spec/fspec-config.json'
    And spec/CLAUDE.md no longer exists
    And spec/fspec-config.json no longer exists

  Scenario: Detects a toml agent by its detection directory when no config is present
    Given a workspace with no spec/fspec-config.json but a .gemini/ directory exists
    And the files spec/GEMINI.md and .gemini/commands/fspec.toml exist
    When I dispatch remove-init-files with no keepConfig
    Then the dispatcher returns success=true
    And the returned JSON filesRemoved includes 'spec/GEMINI.md'
    And the returned JSON filesRemoved includes '.gemini/commands/fspec.toml'

  Scenario: keepConfig=true preserves spec/fspec-config.json
    Given a workspace with spec/fspec-config.json containing agent='claude'
    And the files spec/CLAUDE.md and .claude/commands/fspec.md exist
    When I dispatch remove-init-files with keepConfig=true
    Then the dispatcher returns success=true
    And the returned JSON filesRemoved includes 'spec/CLAUDE.md'
    And the returned JSON filesRemoved does NOT include 'spec/fspec-config.json'
    And spec/fspec-config.json still exists

  Scenario: Errors when no agent installation is detected
    Given a workspace with no spec/fspec-config.json and no agent detection directories
    When I dispatch remove-init-files with no keepConfig
    Then the dispatcher returns success=false with an error message containing 'No fspec agent installation detected. Nothing to remove.'

  Scenario: Force removal is idempotent when an agent file is already absent
    Given a workspace with spec/fspec-config.json containing agent='claude'
    And spec/CLAUDE.md does NOT exist but .claude/commands/fspec.md exists
    When I dispatch remove-init-files with no keepConfig
    Then the dispatcher returns success=true
    And the returned JSON filesRemoved includes 'spec/CLAUDE.md'

  Scenario: Errors when the config names an unknown agent
    Given a workspace with spec/fspec-config.json containing agent='not-a-real-agent'
    When I dispatch remove-init-files with no keepConfig
    Then the dispatcher returns success=false with an error message containing 'Unknown agent: not-a-real-agent'

  Scenario: Does not touch project files
    Given a workspace with spec/fspec-config.json containing agent='claude'
    And spec/work-units.json and a spec/features/ directory exist
    When I dispatch remove-init-files with no keepConfig
    Then the dispatcher returns success=true
    And spec/work-units.json still exists
    And the spec/features/ directory still exists
