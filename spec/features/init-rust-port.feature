@done
@rust
@initialization
@agent-registry
@RPC-239
Feature: Port init command to Rust
  """
  Core impl rust/fspec-core/src/commands/init.rs: replace the NotYetPorted
  stub with `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
  dispatched through the same `poll_sync_future` path as the other ported commands.
  Args shape: `{ agent: string[] }` (camelCase; the repeatable --agent flag at the
  CLI surface accumulates into this Vec). All filesystem work uses BLOCKING std::fs
  (mkdir_all / write) — parity with the TS `fs/promises` calls; NO real async, NO
  child process, NO network.

  Faithful port of src/commands/init.ts (executeInit + installAgents +
  installAgentFiles + installFullDoc + installSlashCommand) and the
  src/utils helpers it depends on:
  - agentRegistry.ts (AGENT_REGISTRY / getAgentById)
  - templateGenerator.ts (generateAgentDoc → getProjectManagementTemplate +
  stripSystemReminders + removeMetaCognitivePrompts + replacePlaceholders)
  - slashCommandTemplate.ts (getSlashCommandTemplate → getHeaderSection)
  - agentRuntimeConfig.ts (writeAgentConfig — read-modify-write merge)
  - agentDetection.ts (detectAgents — used by the switch prompt path)
  The 20-agent registry is INLINED as a local const table (SUPERVISOR DECISION:
  confirmed — same precedent as the ported remove-init-files command; no shared
  agents.rs module, no lib.rs/mod.rs touch).

  Headless interactivity (SUPERVISOR DECISION: confirmed): the TS command renders
  two Ink TUIs — the AgentSelector (interactive mode, when no --agent is given) and
  the ConfirmPrompt agent-switch dialog (when an already-installed agent differs
  from the requested one). Neither is reproducible under poll_sync_future. The Rust
  port therefore (a) errors with the exact TS TTY-guard message when no agent id is
  supplied, and (b) treats the agent-switch decision as AUTO-CONFIRMED (proceed,
  never cancelled) — mirroring how interactive mode already passes
  `promptAgentSwitch: async () => true`.

  Template content (SUPERVISOR DECISION: embed VERBATIM + apply all transforms,
  assert behaviour not byte-diff): generateAgentDoc renders the ~2069-line
  project-management template (src/utils/projectManagementTemplate.ts) per agent,
  byte-faithfully embedded, then strips <system-reminder> blocks for agents without
  supportsSystemReminders, removes meta-cognitive phrases for agents without
  supportsMetaCognition, and replaces
  {{AGENT_NAME}}/{{DOC_TEMPLATE}}/{{SLASH_COMMAND_PATH}}/{{AGENT_ID}} +
  <test-command>/<quality-check-commands> placeholders so the RUNTIME output
  matches TS. The markdown slash-command file content is getHeaderSection() (title +
  IMMEDIATELY + --sync-version with getVersion() + bootstrap block); the TOML
  slash-command file is the inline TOML literal in init.ts. Tests assert the
  placeholder/transform BEHAVIOUR, not a 2069-line byte-diff.

  Codex home-dir (SUPERVISOR DECISION: confirmed): the codex/codex-cli slash command
  is written under <home>/.codex/prompts; the core reads HOME from an injectable
  source (env HOME), never a hard-coded path, and tests override HOME to a tempdir
  rather than excluding the codex agents from parity.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `init` MUST replace the NotYetPorted stub and return a real result through the same `poll_sync_future` path the other ported commands use; the signature becomes `run(args_json, project_root)` (the current stub is the 1-arg `run(args_json)` form and the dispatch route must be updated by the supervisor)
  #   2. Args parse as `{ agent: string[] }` (camelCase, default empty Vec). An EMPTY agent list means interactive selection, which is impossible headless: the command MUST error with the TS TTY-guard text 'Interactive mode requires a TTY. Use --agent flag instead:' followed by the two `fspec init --agent=...` example lines (parity with src/commands/init.ts:310-316)
  #   3. Every requested agent id MUST be validated against the inlined AGENT_REGISTRY (20 agents, registry order). An unknown id MUST error 'Unknown agent: <id>.' followed by a blank line, 'Valid agent IDs:' and one '  - <id>: <description>' line per AVAILABLE agent (parity with installAgents at src/commands/init.ts:100-110)
  #   4. For each valid agent the command installs exactly two files: the full documentation at spec/<docTemplate> (e.g. spec/CLAUDE.md, spec/AGENTS.md for codex) and one slash-command file; both parent directories are created recursively (parity with installAgentFiles / installFullDoc / installSlashCommand)
  #   5. The doc file content is generateAgentDoc(agent): the project-management template with {{AGENT_NAME}}/{{DOC_TEMPLATE}}/{{SLASH_COMMAND_PATH}}/{{AGENT_ID}} replaced, <system-reminder> blocks stripped to visible '**IMPORTANT:**' / '**⚠️ IMPORTANT:**' instructions for agents without supportsSystemReminders, and meta-cognitive phrases (ultrathink, deeply consider, take a moment to reflect) removed for agents without supportsMetaCognition (parity with templateGenerator.ts)
  #   6. The slash-command filename is 'fspec.toml' when slashCommandFormat=='toml' (gemini, qwen) else 'fspec.md'; the TOML content is the inline literal from init.ts:265-279 and the markdown content is getHeaderSection() (parity with generateSlashCommandContent at src/commands/init.ts:262-288)
  #   7. The slash-command directory is normally <project_root>/<slashCommandPath>; for the codex and codex-cli agents it is <home>/.codex/prompts (os.homedir) and the reported install path for those agents is '~/.codex/prompts/fspec.md' (parity with src/commands/init.ts:238-256); the core reads HOME from an injectable source (env HOME) — never a hard-coded path — and tests override HOME to a tempdir (SUPERVISOR DECISION: confirmed)
  #   8. After installing, the command writes spec/fspec-config.json with { agent: agentIds[0] } using a read-modify-write merge that preserves any existing keys, 2-space indented (parity with writeAgentConfig at agentRuntimeConfig.ts:65-90)
  #   9. Multiple --agent values install every agent in the given order; filesInstalled accumulates every installed path and the config records ONLY the first agent id (parity with installAgents loop + writeAgentConfig(agentIds[0]))
  #   10. The interactive agent-switch ConfirmPrompt (shown when a different agent is already installed) is not reproducible headless; the Rust port treats the switch as AUTO-CONFIRMED (proceed with install) and never returns cancelled=true from a headless run (SUPERVISOR DECISION: confirmed)
  #   11. On success the command returns a structured result containing filesInstalled (array of installed paths), cancelled=false and success=true; the dispatcher delivers this as JSON, the CLI renders the human success summary
  #   12. The 20-agent registry and detection paths are reused via a local inlined const table (same field set as AGENT_REGISTRY: id, name, description, docTemplate, slashCommandPath, slashCommandFormat, supportsSystemReminders, supportsMetaCognition, category, detectionPaths, available) — mirroring the precedent set by the ported remove-init-files command
  #   13. All writes use BLOCKING std::fs (create_dir_all + write); the command performs NO network calls and spawns NO child processes
  #
  # EXAMPLES:
  #   1. Dispatch init with `{"agent":["claude"]}` against an empty project root → spec/CLAUDE.md and .claude/commands/fspec.md are created, spec/fspec-config.json contains {"agent":"claude"}, and the result has success=true, cancelled=false, filesInstalled=['spec/CLAUDE.md','.claude/commands/fspec.md']
  #   2. Dispatch init with `{"agent":["gemini"]}` → the slash-command file is .gemini/commands/fspec.toml (TOML format) whose content starts with '[command]' and the doc file is spec/GEMINI.md with all <system-reminder> blocks rewritten to '**IMPORTANT:**' visible instructions (gemini does not support system reminders)
  #   3. Dispatch init with `{"agent":["cursor"]}` → spec/CURSOR.md is created and its system-reminder blocks render as '**⚠️ IMPORTANT:**' (cursor is an IDE/extension category agent)
  #   4. Dispatch init with `{"agent":["claude","cursor"]}` → both spec/CLAUDE.md and spec/CURSOR.md plus both slash-command files exist, filesInstalled lists all four, and spec/fspec-config.json records only {"agent":"claude"}
  #   5. Dispatch init with `{"agent":["bogus"]}` → success=false with an error message beginning 'Unknown agent: bogus.' and listing the valid agent ids
  #   6. Dispatch init with `{"agent":[]}` (empty) → success=false with the error 'Interactive mode requires a TTY. Use --agent flag instead:' (headless selection is unsupported)
  #   7. Dispatch init with `{"agent":["claude"]}` against a project root that already has spec/fspec-config.json containing {"agent":"cursor","foo":"bar"} → install proceeds (switch auto-confirmed), spec/fspec-config.json becomes {"foo":"bar","agent":"claude"} (existing keys preserved, agent overwritten)
  #   8. Dispatch init with `{"agent":["claude"]}` then the doc file spec/CLAUDE.md contains the literal {{...}} placeholders NOWHERE — {{AGENT_NAME}} has been replaced with 'Claude Code', {{DOC_TEMPLATE}} with 'CLAUDE.md', {{SLASH_COMMAND_PATH}} with '.claude/commands/'
  #
  # QUESTIONS (ANSWERED by supervisor):
  #   1. @supervisor: Adopt the 2-arg `run(args_json, project_root)` signature? A: YES — adopt 2-arg; the supervisor updates the dispatch.rs route + run_ported/Mode registration (shared files, supervisor-owned).
  #   2. @supervisor: Headless interactivity? A: YES to both — (a) empty agent list errors with the exact TS TTY-guard text; (b) the agent-switch ConfirmPrompt is AUTO-CONFIRMED (proceed, never cancelled). Mirrors remove-init-files.
  #   3. @supervisor: Template fidelity? A: Option (b) — embed projectManagementTemplate VERBATIM (byte-faithful) AND apply all transforms (system-reminder strip, meta-cognitive removal, {{PLACEHOLDER}} + <test-command> replacement) so RUNTIME output matches TS; tests assert placeholder/transform BEHAVIOUR, not a 2069-line byte-diff. Markdown slash file = getHeaderSection() (getVersion()); TOML slash file = inline literal.
  #   4. @supervisor: Codex home-dir under test? A: Override HOME to a tempdir in tests and KEEP codex/codex-cli in parity scenarios; core reads HOME from an injectable source (env HOME), never a hard-coded path.
  #   5. @supervisor: Inlined local const agent table? A: YES — inlined local const table, NO shared agents.rs, NO mod.rs/lib.rs touch. Identical to remove-init-files.
  #
  # ASSUMPTIONS:
  #   1. Confirmed by supervisor: adopt run(args_json, project_root); inline the 20-agent registry as a local const table; empty agent list errors with the TTY-guard message; agent-switch is auto-confirmed headless; template embedded verbatim + transforms applied; HOME injectable (overridden to tempdir in tests); all writes use blocking std::fs.
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch init from the agent loop and scaffold the same agent docs, slash commands and config as the TypeScript implementation
    So that I can initialise fspec for an AI coding agent without relying on Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Installs claude agent files and writes the config
    Given an empty project root directory
    When I dispatch the init command against that project root with agent list ['claude']
    Then the dispatcher returns success=true and cancelled=false
    Then spec/CLAUDE.md exists in the project root
    Then .claude/commands/fspec.md exists in the project root
    Then spec/fspec-config.json contains the agent field 'claude'
    Then the filesInstalled array contains 'spec/CLAUDE.md' and '.claude/commands/fspec.md'

  Scenario: Installs a TOML-format agent slash command file
    Given an empty project root directory
    When I dispatch init against that project root with agent list ['gemini']
    Then the dispatcher returns success=true
    Then spec/GEMINI.md exists in the project root
    Then .gemini/commands/fspec.toml exists in the project root
    Then the file .gemini/commands/fspec.toml starts with the substring '[command]'

  Scenario: Strips system-reminder blocks to visible instructions for non-claude agents
    Given an empty project root directory
    When I dispatch init against that project root with agent list ['gemini']
    Then the doc file spec/GEMINI.md does NOT contain the substring '<system-reminder>'
    Then the doc file spec/GEMINI.md contains the substring '**IMPORTANT:**'

  Scenario: Replaces all template placeholders with agent-specific values
    Given an empty project root directory
    When I dispatch init against that project root with agent list ['claude']
    Then the doc file spec/CLAUDE.md does NOT contain the substring '{{AGENT_NAME}}'
    Then the doc file spec/CLAUDE.md does NOT contain the substring '{{DOC_TEMPLATE}}'
    Then the doc file spec/CLAUDE.md does NOT contain the substring '{{SLASH_COMMAND_PATH}}'

  Scenario: Installs multiple agents in order and records only the first in config
    Given an empty project root directory
    When I dispatch init against that project root with agent list ['claude', 'cursor']
    Then spec/CLAUDE.md and spec/CURSOR.md both exist in the project root
    Then the filesInstalled array contains all four installed paths
    Then spec/fspec-config.json contains the agent field 'claude'

  Scenario: Rejects an unknown agent id with the valid-id listing
    Given an empty project root directory
    When I dispatch init against that project root with agent list ['bogus']
    Then the dispatcher returns success=false
    Then the error message begins with the substring 'Unknown agent: bogus.'
    Then the error message contains the substring 'Valid agent IDs:'

  Scenario: Rejects an empty agent list because headless selection is unsupported
    Given an empty project root directory
    When I dispatch init against that project root with an empty agent list
    Then the dispatcher returns success=false
    Then the error message contains the substring 'Interactive mode requires a TTY. Use --agent flag instead:'

  Scenario: Preserves existing config keys when overwriting the agent field
    Given a project root whose spec/fspec-config.json contains agent='cursor' and an extra key foo='bar'
    When I dispatch init against that project root with agent list ['claude']
    Then the dispatcher returns success=true and cancelled=false
    Then spec/fspec-config.json contains the agent field 'claude'
    Then spec/fspec-config.json still contains the key foo with value 'bar'

  Scenario: Writes the codex slash command under an injectable HOME directory
    Given an empty project root directory
    Given the HOME environment variable points at a separate temporary directory
    When I dispatch init against that project root with agent list ['codex']
    Then the dispatcher returns success=true
    Then spec/AGENTS.md exists in the project root
    Then the file .codex/prompts/fspec.md exists under the injected HOME directory
    Then the filesInstalled array contains '~/.codex/prompts/fspec.md'

  Scenario: Shares one implementation between the dispatcher and the CLI bridge
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/commands/init.rs
    Then init::run scaffolds files via blocking std::fs and contains the inlined agent registry table
    Then the CLI bridge rust/fspec/src/init.rs delegates to init::run and contains no inline scaffolding or registry logic
