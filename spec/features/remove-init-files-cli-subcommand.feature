@done
@feature-management
@cli
@RPC-276
Feature: remove-init-files clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `remove-init-files` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
  - Shell argv         → clap → codelet/fspec/src/remove_init_files.rs → fspec_core::commands::remove_init_files::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::remove_init_files::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes `--keep-config` / `--no-keep-config` (boolean keepConfig). The headless Rust port does NOT render the interactive Ink prompt; an unspecified keepConfig defaults to false.
  Success → stdout '✓ Successfully removed fspec init files' plus per-file lines, exit 0; error → stderr prefixed 'Error:' exit 1.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/remove-init-files.txt.
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
  #   Q: @supervisor: No Rust port of AGENT_REGISTRY exists in fspec-core (init.rs is still a stub). I will create a local const agent table inside commands/remove_init_files.rs covering the needed fields (id, docTemplate, slashCommandPath, slashCommandFormat, detectionPaths) for the 20 agents — confirm this is acceptable vs. a new shared module codelet/fspec-core/src/agents.rs (which would require touching lib.rs/mod). Also confirm the headless default for an unspecified keepConfig should be false (remove config).
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

  Scenario: Clap exposes remove-init-files as a subcommand and prints --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-init-files --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'remove-init-files'

  Scenario: CLI removes claude agent files and prints the success summary
    Given a workspace with spec/fspec-config.json containing agent='claude' and the files spec/CLAUDE.md and .claude/commands/fspec.md
    When I run `./codelet/target/release/fspec remove-init-files --no-keep-config` from that workspace
    Then the command exits 0
    And stdout contains the substring '✓ Successfully removed fspec init files'
    And stdout contains the substring 'spec/CLAUDE.md'
    And spec/CLAUDE.md no longer exists

  Scenario: CLI --keep-config preserves spec/fspec-config.json
    Given a workspace with spec/fspec-config.json containing agent='claude' and the files spec/CLAUDE.md and .claude/commands/fspec.md
    When I run `./codelet/target/release/fspec remove-init-files --keep-config` from that workspace
    Then the command exits 0
    And spec/fspec-config.json still exists

  Scenario: CLI exits 1 when no agent installation is detected
    Given a workspace with no spec/fspec-config.json and no agent detection directories
    When I run `./codelet/target/release/fspec remove-init-files --no-keep-config` from that workspace
    Then the command exits with code 1
    And stderr contains the substring 'No fspec agent installation detected. Nothing to remove.'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a workspace with spec/fspec-config.json containing agent='claude' and the files spec/CLAUDE.md and .claude/commands/fspec.md
    When I dispatch remove-init-files through fspec_core::dispatch::dispatch_command with keepConfig=true against that workspace
    Then the dispatcher returns JSON whose filesRemoved includes 'spec/CLAUDE.md'
    And the CLI bridge module codelet/fspec/src/remove_init_files.rs contains NO inline detection or deletion logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: remove-init-files --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-init-files --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-init-files.txt
    And stdout starts with a blank line followed by 'REMOVE-INIT-FILES'
