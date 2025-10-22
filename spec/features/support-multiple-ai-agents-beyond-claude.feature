@INIT-004
@cli
@initialization
@multi-agent-support
@templates
@agent-registry
Feature: Support multiple AI agents beyond Claude
  """
  Create agent registry in spec/agents.json mapping agent names to configuration (doc template, slash command path, system-reminder support, etc.)
  Refactor init command: add --agent flag, read agent config from registry, copy appropriate templates, generate agent-specific documentation
  Create template system: templates/{agent-name}/AGENT.md and templates/{agent-name}/slash-command.md with variable substitution
  Add agent detection: check for .claude/, .cursor/, .continue/ directories or environment variables to auto-detect active agent
  Update vite.config.ts to copy spec/templates/{agent}/**/*.md to dist/spec/templates/{agent}/ for bundled distribution
  Template matrix challenge: 20-30 fspec commands × 18 agents = 360-540 template files. Consider shared template body + agent-specific wrappers to reduce duplication
  Init command must locate templates from bundled distribution (handle both dev: spec/templates/ and production: dist/spec/templates/ paths)
  ink/react Integration Architecture: Refactor fspec to use ink/react for ALL interactive commands (not just init). Create shared component library in src/components/ (AgentSelector, Spinner, ErrorMessage, SuccessMessage, ConfirmPrompt, etc.) and shared hooks in src/hooks/ (useKeyboardNavigation, useSafeInput, useAgentSelection). Follow DRY principles - reuse cage patterns (MainMenu navigation, keyboard input handling, visual feedback). Commands like 'fspec reverse', 'fspec board', future interactive commands should all use shared ink/react components. Vite build handles React/JSX compilation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each AI agent must have its own documentation template (e.g., CLAUDE.md, AIDER.md, CURSOR.md)
  #   2. System-reminder tags are Claude Code specific and must be abstracted or made optional for other agents
  #   3. fspec init command must support --agent flag to specify which AI agent to configure
  #   4. Slash command directory paths vary by agent (e.g., .claude/commands/ vs .continue/commands/)
  #   5. Documentation must avoid hardcoded references to 'Claude' or 'Claude Code' in agent-agnostic sections
  #   6. Yes, support multiple agents: Allow 'fspec init --agent=aider --agent=cursor' to install configurations for multiple agents simultaneously. Generate spec/AGENT.md for each and install slash commands to respective directories
  #   7. Agent-specific prompting vocabulary (like 'ultrathink' for Claude) must be translated or removed for each agent based on their capabilities
  #   8. fspec init must create slash commands for ALL major fspec CLI commands (validate, format, list-features, create-work-unit, etc.), not just documentation
  #   9. All agent templates must be bundled in fspec distribution using Vite copy plugin
  #   10. Template directory structure must support both flat (cursor) and nested (claude) slash command paths
  #   11. Generate on-the-fly using TemplateManager pattern: shared command body + agent-specific wrappers (frontmatter, path, argument placeholders). Avoids 360-540 template files, enables consistency across agents
  #   12. Remove meta-cognitive prompts for CLI-only agents (Aider, Gemini CLI, Qwen CLI). Keep for IDE/extension-based agents (Cline, Windsurf, Cursor). Agent registry should have 'supportsMetaCognition' flag to control this
  #   13. Support both: 'fspec init' (no flags) shows interactive selector by default. 'fspec init --agent=X' runs non-interactive mode for automation/scripts
  #   14. Yes, auto-detect installed agents by checking for directories (.claude/, .cursor/, etc.). Sort list with: 1) Detected agents at top (pre-selected), 2) Popular agents (Claude Code, Codex, Copilot), 3) Remaining agents alphabetically
  #   15. Support all 18 agents in v1. Use dynamic generation - ONE base template that gets transformed based on agent capabilities (system-reminder support, meta-cognition support, slash command format). No separate template files per agent.
  #   16. Install THREE files per agent: 1) Root stub (AGENTS.md or AGENT_NAME.md) for auto-loading, 2) Full doc (spec/AGENT_NAME.md) for comprehensive workflow, 3) Slash command (.agent/commands/fspec.md) for manual trigger. Most agents (Claude, Cursor, Cline, Windsurf, Copilot, etc.) auto-load root stubs; CLI-only tools need slash commands.
  #   17. fspec init is idempotent and supports agent switching. Running 'fspec init --agent=cursor' after 'fspec init --agent=claude' should remove Claude-specific files (CLAUDE.md, spec/CLAUDE.md, .claude/commands/) and install Cursor-specific files (CURSOR.md or AGENTS.md, spec/CURSOR.md, .cursor/commands/). All files are auto-generated, safe to replace.
  #   18. Interactive selector is SINGLE-SELECT: arrow keys to navigate/highlight, Enter to confirm the currently highlighted agent. No multi-select, no checkboxes. For multiple agents, user must run 'fspec init --agent=X --agent=Y' using CLI flags.
  #   19. Default highlighted agent should be the FIRST DETECTED agent (if any were auto-detected). If no agents detected, default to first agent in sorted list.
  #   20. No --no-interactive flag needed. Presence of --agent flags is sufficient to determine mode: if --agent present, use CLI mode; if no --agent, use interactive mode. A --no-interactive flag without --agent makes no sense.
  #   21. fspec init is NON-DESTRUCTIVE: if .cursor/commands/ already exists with custom files, create fspec.md alongside them. Never delete or overwrite user's custom files. Only manage fspec-specific files (AGENTS.md, spec/AGENT.md, .agent/commands/fspec.md).
  #   22. Validate agent IDs against registry before installation. Check file permissions before writing. Create directories recursively (mkdir -p). Handle missing templates gracefully with error messages. Wrap blocking hook failures in <system-reminder> tags for AI visibility.
  #   23. Generate ONE comprehensive fspec.md per agent (1010-line pattern like current .claude/commands/fspec.md). NOT multiple smaller files. This file contains complete ACDD workflow, Example Mapping process, coverage tracking, etc. Format varies by agent: Markdown with YAML frontmatter (Claude, Cursor, Cline, etc.) or TOML (Gemini CLI, Qwen Code).
  #   24. Invalid agent ID: Show error with list of valid agents, exit code 1. Missing templates: Check dist/spec/templates/ and fallback to spec/templates/. If both missing, show error explaining distribution may be corrupted. Suggest reinstalling fspec. Permission errors: Show clear message about directory permissions, suggest chmod/chown fixes.
  #   25. Research complete - see spec/attachments/INIT-004/agent-instruction-mechanisms-research.md. Finding: Claude Code's <system-reminder> tags are UNIQUE. All other agents use visible Markdown patterns: bold text (**IMPORTANT:**), headers (###), code blocks, blockquotes. Transformation required: <system-reminder> → **⚠️ IMPORTANT:** (IDE agents) or **IMPORTANT:** (CLI agents). No equivalent invisible mechanism exists for other agents.
  #
  # EXAMPLES:
  #   1. User runs 'fspec init --agent=aider' and gets Aider-specific documentation template copied to spec/AIDER.md
  #   2. User runs 'fspec init --agent=cursor' and slash command is installed to .cursor/commands/fspec.md
  #   3. User runs 'fspec init --agent=cline' and gets Cline-specific docs without system-reminder references
  #   4. User runs 'fspec init --agent=cursor' and gets 20+ slash commands created (.cursor/commands/fspec-validate.md, .cursor/commands/fspec-format.md, etc.) plus spec/CURSOR.md
  #   5. After 'npm install -g fspec', global installation contains bundled templates at node_modules/fspec/dist/spec/templates/cursor/CURSOR.md
  #   6. CLAUDE.md contains 'ultrathink your next steps' but AIDER.md (CLI-based) removes meta-cognitive prompts entirely
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should fspec init auto-detect which AI agent is being used, or always require explicit --agent flag?
  #   A: true
  #
  #   Q: For agents without system-reminder support, should we strip those tags or replace with agent-specific patterns?
  #   A: true
  #
  #   Q: Should 'fspec init' support multiple agents simultaneously (e.g., team with mixed Cursor + Aider users)?
  #   A: true
  #
  #   Q: Should we bundle ALL 18 agents' templates (increasing package size significantly) or allow dynamic generation?
  #   A: true
  #
  #   Q: Should slash command templates be generated on-the-fly from shared logic or pre-built and bundled?
  #   A: true
  #
  #   Q: How do we handle agents that don't support meta-cognitive prompts like 'ultrathink' (e.g., CLI-only Aider)?
  #   A: true
  #
  #   Q: Should fspec init use interactive mode by default, or require explicit flags?
  #   A: true
  #
  #   Q: Should interactive selector auto-detect installed agents and how should the list be sorted?
  #   A: true
  #
  #   Q: Should we support all 18 agents in v1, or start with a smaller set? Should we use separate template files or dynamic generation?
  #   A: true
  #
  #   Q: Should we install both AGENT.md (for auto-load on startup) AND a slash command (for manual trigger), or just one? Which agents support auto-loading project context files?
  #   A: true
  #
  #   Q: Should fspec init be idempotent? If user runs 'fspec init --agent=cursor' after already running 'fspec init --agent=claude', what should happen?
  #   A: true
  #
  #   Q: Is the interactive selector single-select (arrow to highlight, enter to confirm) or multi-select (space to toggle checkboxes, enter to confirm multiple)?
  #   A: true
  #
  #   Q: In interactive selector, which agent should be highlighted by default when the UI opens?
  #   A: true
  #
  #   Q: Do we need a --no-interactive flag, or is presence of --agent flags sufficient to disable interactive mode?
  #   A: true
  #
  #   Q: If target directories already exist with user's custom files (e.g., .cursor/commands/my-command.md), should fspec init be destructive or non-destructive?
  #   A: true
  #
  #   Q: How do we transform the base template for different agents? Single base template with dynamic transformations based on agent.supportsSystemReminders, agent.supportsMetaCognition flags? Or multiple template tiers?
  #   A: true
  #
  #   Q: What exact patterns need transformation? 'ultrathink' → 'carefully consider'? '<system-reminder>' → '<\!-- IMPORTANT: -->'? Any others? Should we use regex, string replacement, or AST parsing?
  #   A: true
  #
  #   Q: Current .claude/commands/fspec.md is 1010 lines (comprehensive workflow guide). Should we generate ONE comprehensive fspec.md per agent (same pattern), or MULTIPLE smaller commands (fspec-validate.md, fspec-format.md, etc.)?
  #   A: true
  #
  #   Q: Error handling: What should happen if user runs 'fspec init --agent=invalid-id'? Show list of valid agents? Exit with error code? What if template files missing from dist/?
  #   A: true
  #
  #   Q: Vite bundling: Should we bundle base template files (spec/templates/base/AGENT.md) in dist/, or bundle transformation logic as compiled code? What's the dist/ structure after build?
  #   A: true
  #
  #   Q: Agent registry structure: Confirm complete AgentConfig interface fields (id, name, description, category, rootStubFile, fullDocFile, slashCommandPath, slashCommandFormat, detectionPaths, supportsSystemReminders, supportsMetaCognition, popularity for sorting). Missing any fields?
  #   A: true
  #
  #   Q: Claude Code uses <system-reminder> tags for invisible-to-user workflow guidance. What equivalent mechanisms do other agents have for getting attention and providing hidden instructions? Need research on all 18 agents' attention/instruction mechanisms.
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Auto-detect with fallback to explicit flag: Check for agent directories (.claude/, .cursor/, .continue/) and environment variables first, but allow --agent flag to override. Default to Claude Code if no detection
  #   2. Strip system-reminder tags for non-Claude agents: Use agent registry to check 'supportsSystemReminders' flag. For agents without support, strip tags but preserve content as comments or agent-specific annotations
  #   3. Bundle ALL 18 agents in v1 for simplicity and feature completeness. Future optimization: lazy-download popular agents on first use to reduce package size
  #   4. Research which agents support auto-loading project context files (AGENT.md pattern) vs requiring slash commands. Install both for v1 to maximize compatibility.
  #   5. Single base template with dynamic transformations based on agent.supportsSystemReminders, agent.supportsMetaCognition, and agent.category flags. Use TemplateManager pattern: ONE base template transformed via regex replacement and formatting functions.
  #   6. System-reminders: <system-reminder> → **⚠️ IMPORTANT:** (IDE agents with emoji) or **IMPORTANT:** (CLI agents). Meta-cognitive prompts: 'ultrathink', 'deeply consider' → removed for CLI-only agents. Use regex patterns. Agent placeholders: {{AGENT_NAME}}, {{SLASH_COMMAND_PATH}}, {{AGENT_ID}} replaced via string.replace().
  #   7. ONE comprehensive fspec.md file per agent (like current 1010-line pattern), NOT multiple smaller files. Different formats: Markdown with YAML frontmatter (Claude, Cursor, Cline) or TOML (Gemini CLI, Qwen Code). Generate using TemplateManager pattern: shared body + agent-specific wrapper.
  #   8. Use viteStaticCopy plugin to copy base template files (spec/templates/base/*.md) into dist/ during build. Templates bundled into dist/spec/templates/base/. Component code bundled normally by Vite. Template resolver tries production path first, falls back to dev path.
  #   9. id: string, name: string, description: string, slashCommandPath: string, slashCommandFormat: 'markdown' | 'toml', supportsSystemReminders: boolean, supportsMetaCognition: boolean, docTemplate: string, rootStubFile: string, detectionPaths: string[], available: boolean, category: 'ide' | 'cli' | 'extension'
  #   10. Transform 'ultrathink' → removed (CLI agents) or kept (IDE agents). System-reminder: '<system-reminder>' → '**⚠️ IMPORTANT:**' (IDE with emoji) or '**IMPORTANT:**' (CLI). Also transform: 'deeply consider', 'take a moment to reflect' → removed for CLI agents. Use regex replacement: /<system-reminder>([\s\S]*?)<\/system-reminder>/g. Placeholders: {{AGENT_NAME}}, {{SLASH_COMMAND_PATH}} via string.replace().
  #   11. Bundle base template files using viteStaticCopy plugin. Structure: dist/spec/templates/base/AGENT.md (ONE base template with placeholders). Runtime: templateGenerator.ts reads from dist/spec/templates/base/, transforms based on agent config, outputs to spec/AGENT.md. Dev mode: reads from spec/templates/base/. Template resolver tries production path first, falls back to dev.
  #   12. AgentConfig fields confirmed: id (string), name (string), description (string), slashCommandPath (string), slashCommandFormat ('markdown' | 'toml'), supportsSystemReminders (boolean), supportsMetaCognition (boolean), docTemplate (string, e.g. 'CLAUDE.md'), rootStubFile (string, e.g. 'CLAUDE.md' or 'AGENTS.md'), detectionPaths (string[], e.g. ['.claude/', '.claude/commands/']), available (boolean), category ('ide' | 'cli' | 'extension'). Optional: popularity (number) for sorting.
  #
  # ========================================
  Background: User Story
    As a developer using any AI coding agent
    I want to initialize fspec in my project with agent-specific configuration
    So that I get properly formatted documentation and setup tailored to my agent without manual editing

  Scenario: Install Aider-specific documentation
    Given I am in a project directory
    When I run "fspec init --agent=aider"
    Then a file "spec/AIDER.md" should be created
    And the file should contain Aider-specific documentation
    And the file should not contain Claude-specific references
    And meta-cognitive prompts should be removed

  Scenario: Install Cursor slash command
    Given I am in a project directory
    When I run "fspec init --agent=cursor"
    Then a file ".cursor/commands/fspec.md" should be created
    And the file should contain comprehensive ACDD workflow documentation
    And the file should use Markdown format with YAML frontmatter

  Scenario: Transform system-reminders for Cline
    Given I am in a project directory
    When I run "fspec init --agent=cline"
    Then a file "spec/CLINE.md" should be created
    And the file should not contain "<system-reminder>" tags
    And system-reminder content should be transformed to "**⚠️ IMPORTANT:**" blocks
    And the transformed content should be visible to the agent

  Scenario: Install comprehensive slash command documentation
    Given I am in a project directory
    When I run "fspec init --agent=cursor"
    Then a file ".cursor/commands/fspec.md" should be created
    And the file should be at least 1000 lines long
    And the file should contain ACDD workflow documentation
    And the file should contain Example Mapping documentation
    And the file should contain coverage tracking documentation
    And a file "spec/CURSOR.md" should be created

  Scenario: Bundle templates in distribution
    Given fspec is installed globally via "npm install -g fspec"
    When I check the installation directory
    Then a file "dist/spec/templates/base/AGENT.md" should exist
    And the file should contain template placeholders like "{{AGENT_NAME}}"
    And the file should contain "<system-reminder>" tags for transformation

  Scenario: Remove meta-cognitive prompts for CLI agents
    Given I am in a project directory
    When I run "fspec init --agent=aider"
    Then a file "spec/AIDER.md" should be created
    And the file should not contain "ultrathink"
    And the file should not contain "deeply consider"
    And the file should not contain "take a moment to reflect"

  Scenario: Interactive mode with auto-detection
    Given I am in a project directory
    And a directory ".cursor/" exists
    When I run "fspec init" without --agent flags
    Then an interactive agent selector should appear
    And "Cursor" should be highlighted by default
    And "Cursor" should be marked as "auto-detected"
    And I can navigate with arrow keys
    And I can confirm selection with Enter

  Scenario: Install multiple agents simultaneously
    Given I am in a project directory
    When I run "fspec init --agent=cursor --agent=aider"
    Then a file "spec/CURSOR.md" should be created
    And a file "spec/AIDER.md" should be created
    And a file ".cursor/commands/fspec.md" should be created
    And both documentation files should be agent-specific

  Scenario: Agent switching (idempotent behavior)
    Given I am in a project directory
    And I have previously run "fspec init --agent=claude"
    And files "CLAUDE.md", "spec/CLAUDE.md", ".claude/commands/fspec.md" exist
    When I run "fspec init --agent=cursor"
    Then Claude-specific files should be removed
    And Cursor-specific files should be created
    And no Claude-specific content should remain

  Scenario: Non-destructive installation
    Given I am in a project directory
    And a file ".cursor/commands/my-custom-command.md" exists
    When I run "fspec init --agent=cursor"
    Then a file ".cursor/commands/fspec.md" should be created
    And the file ".cursor/commands/my-custom-command.md" should still exist
    And no user files should be deleted or modified

  Scenario: Invalid agent ID error
    Given I am in a project directory
    When I run "fspec init --agent=invalid-agent"
    Then the command should exit with code 1
    And the output should show an error message
    And the output should list all valid agent IDs
    And no files should be created

  Scenario: Transform system-reminders for IDE agents with emoji
    Given I am in a project directory
    When I run "fspec init --agent=cursor"
    Then a file "spec/CURSOR.md" should be created
    And "<system-reminder>" tags should be transformed to "**⚠️ IMPORTANT:**"
    And emoji should be included in the transformed output

  Scenario: Transform system-reminders for CLI agents without emoji
    Given I am in a project directory
    When I run "fspec init --agent=aider"
    Then a file "spec/AIDER.md" should be created
    And "<system-reminder>" tags should be transformed to "**IMPORTANT:**"
    And no emoji should be included in the transformed output

  Scenario: Generate TOML format for Gemini CLI
    Given I am in a project directory
    When I run "fspec init --agent=gemini"
    Then a file ".gemini/commands/fspec.toml" should be created
    And the file should use TOML format
    And the file should contain workflow documentation

  Scenario: Agent registry with all capabilities
    Given the agent registry is loaded
    When I query the configuration for "claude"
    Then it should have "id" as "claude"
    And it should have "supportsSystemReminders" as true
    And it should have "supportsMetaCognition" as true
    And it should have "category" as "cli"
    And it should have "slashCommandFormat" as "markdown"

  Scenario: Template transformation with placeholders
    Given I am in a project directory
    When I run "fspec init --agent=cursor"
    Then a file "spec/CURSOR.md" should be created
    And "{{AGENT_NAME}}" placeholders should be replaced with "Cursor"
    And "{{SLASH_COMMAND_PATH}}" placeholders should be replaced with ".cursor/commands/"
    And no unresolved placeholders should remain
