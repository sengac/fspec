@done
@rust
@initialization
@cli
@RPC-239
Feature: init CLI subcommand on the standalone fspec Rust binary

  """
  CLI surface for the `init` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
    - Shell argv         → clap → codelet/fspec/src/init.rs → fspec_core::commands::init::run
    - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::init::run
  Both call sites pass a JSON-encoded `{ agent: string[] }` args shape and a
  `project_root: &Path`; the CLI surface resolves project_root from CWD (parity
  with TS `process.cwd()` default).

  The clap subcommand exposes a single repeatable `--agent <agent>` option
  (Vec<String>) and NO other flags, mirroring the TS Commander.js registration at
  src/commands/init.ts:290-301 which declares only `.option('--agent <agent>', …)`.
  Because the CLI runs non-TTY, the interactive AgentSelector and the agent-switch
  ConfirmPrompt are never rendered: omitting --agent triggers the TS TTY-guard
  error.

  Success output (parity with the TS action handler at src/commands/init.ts:362-378):
    '✓ Installed fspec for <comma-joined agent ids>'
    one '  - <path>' line per installed file
    a blank line then 'Next steps:' then the agent-specific activation message
    (getActivationMessage), exit 0.
  Errors print '✗ Init failed: <msg>' to stderr and exit 1.
  --help is byte-for-byte identical to the captured TS fixture at
  codelet/fspec/tests/fixtures/help/init.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The standalone fspec binary MUST expose `init` as a clap v4 derive subcommand whose only option is a repeatable `--agent <agent>` collecting into Vec<String> (parity with the single Commander.js `.option('--agent <agent>')` at src/commands/init.ts:294-301)
  #   2. The clap action MUST resolve project_root from CWD, marshal the collected agents into `{ agent: [...] }` JSON, and delegate to fspec_core::commands::init::run — NO inline scaffolding, registry, detection or template logic in the CLI bridge (two-front-doors invariant)
  #   3. On success the CLI prints '✓ Installed fspec for <agent ids joined by ", ">', then one '  - <path>' line for each installed file, then a blank line, 'Next steps:' and the agent-specific activation message from getActivationMessage, and exits 0 (parity with src/commands/init.ts:362-379)
  #   4. Running `fspec init` with NO --agent flag (the CLI is non-TTY) MUST fail with '✗ Init failed: Interactive mode requires a TTY. Use --agent flag instead:' on stderr and exit 1 (parity with the TTY guard at src/commands/init.ts:310-316 surfaced through the catch at 380-383)
  #   5. An unknown agent id MUST fail with '✗ Init failed: Unknown agent: <id>.' on stderr (followed by the valid-id listing) and exit 1
  #   6. The activation message is agent-specific (getActivationMessage): claude → 'Run /fspec in Claude Code to activate'; cursor → 'Open .cursor/commands/ in Cursor to activate'; gemini → 'Add .gemini/commands/ to your Gemini CLI configuration to activate', etc.
  #   7. `fspec init --help` output MUST be byte-for-byte identical to codelet/fspec/tests/fixtures/help/init.txt captured from `node dist/index.js init --help` piped to a non-TTY
  #
  # EXAMPLES:
  #   1. `fspec init --agent claude` in an empty dir → exit 0, stdout '✓ Installed fspec for claude', lists 'spec/CLAUDE.md' and '.claude/commands/fspec.md', then 'Next steps:' and 'Run /fspec in Claude Code to activate'; spec/CLAUDE.md and spec/fspec-config.json exist afterwards
  #   2. `fspec init --agent claude --agent cursor` → exit 0, stdout '✓ Installed fspec for claude, cursor' and four '  - <path>' lines
  #   3. `fspec init` (no --agent) → exit 1, stderr contains 'Interactive mode requires a TTY. Use --agent flag instead:'
  #   4. `fspec init --agent bogus` → exit 1, stderr contains '✗ Init failed: Unknown agent: bogus.'
  #   5. `fspec init --help` → exit 0, stdout byte-for-byte equal to the fixture and starting with a blank line followed by 'INIT'
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec init --agent <agent>` directly from a shell with the same surface offered by the TypeScript Commander.js CLI
    So that I can scaffold fspec agent docs, slash commands and config from a terminal or script without going through the LLM tool-call dispatcher

  Scenario: Clap exposes init as a subcommand and prints --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec init --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'init'
    Then stdout contains the substring '--agent'

  Scenario: CLI installs the claude agent and prints the success summary
    Given an empty directory is set as the current working directory
    When I run `./codelet/target/release/fspec init --agent claude` from that directory
    Then the command exits 0
    Then stdout contains the substring '✓ Installed fspec for claude'
    Then stdout contains the substring 'spec/CLAUDE.md'
    Then stdout contains the substring 'Next steps:'
    Then stdout contains the substring 'Run /fspec in Claude Code to activate'
    Then spec/CLAUDE.md exists in the directory
    Then spec/fspec-config.json exists in the directory

  Scenario: CLI installs multiple agents from repeated --agent flags
    Given an empty directory is set as the current working directory
    When I run `./codelet/target/release/fspec init --agent claude --agent cursor` from that directory
    Then the command exits 0
    Then stdout contains the substring '✓ Installed fspec for claude, cursor'
    Then spec/CLAUDE.md exists in the directory
    Then spec/CURSOR.md exists in the directory

  Scenario: CLI without --agent fails because the shell is non-TTY
    Given an empty directory is set as the current working directory
    When I run `./codelet/target/release/fspec init` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Interactive mode requires a TTY. Use --agent flag instead:'

  Scenario: CLI rejects an unknown agent id
    Given an empty directory is set as the current working directory
    When I run `./codelet/target/release/fspec init --agent bogus` from that directory
    Then the command exits with code 1
    Then stderr contains the substring '✗ Init failed: Unknown agent: bogus.'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given an empty project root directory
    When I dispatch init through fspec_core::dispatch::dispatch_command with agent list ['claude'] against that project root
    Then the dispatcher result reports filesInstalled including 'spec/CLAUDE.md'
    Then the CLI bridge module codelet/fspec/src/init.rs contains NO inline scaffolding, registry or template logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: init --help is byte-for-byte identical to the TS reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec init --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/init.txt
    And stdout starts with a blank line followed by 'INIT'
