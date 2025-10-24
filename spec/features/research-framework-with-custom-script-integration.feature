@done
@example-mapping
@discovery
@research
@cli
@phase-1
@RES-002
Feature: Research framework with custom script integration
  """
  Integration points: (1) Manual invocation via 'fspec research' command, (2) Optional prompt during 'fspec add-question' for Example Mapping, (3) Results can be attached to work units as attachments in spec/attachments/{work-unit-id}/ directory, (4) Results referenced in architecture notes and example mapping data.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CLI-based with flags. Running 'fspec research' without flags displays instructional output for AI agents: lists available tools, their descriptions, and usage examples with --tool= flag. Example: 'fspec research --tool=perplexity --query="question"'. The instructional output guides AI to use the correct tool and syntax.
  #   2. Yes, use similar structure to hooks. Research scripts stored in spec/research-scripts/ directory. Each tool is a script file (e.g., spec/research-scripts/perplexity.sh, spec/research-scripts/jira.sh, spec/research-scripts/confluence.sh).
  #   3. Yes, automatically attach research results to work units. Results saved as attachments and referenced in example mapping data. Architecture notes in feature files should reference research findings where appropriate. Example: 'Research via Perplexity on 2025-10-24: [summary]' in architecture notes section.
  #   4. Prompt user to attach research results. After research completes, ask: 'Attach research results to work unit RES-XXX? (y/n)'. If yes, save as attachment and optionally add to architecture notes/example mapping data. If no, display results only (user can refine query or run additional research).
  #   5. Automatically prompt during Example Mapping. When user adds a question with 'fspec add-question', system asks: 'Would you like to research this question? (y/n)'. If yes, display available research tools and guide user to run 'fspec research --tool=<name> --query="question"'. User can also manually invoke 'fspec research' anytime.
  #   6. Research scripts are standalone CLI tools (like fspec). Each script defines its own interface, flags, and output format. When 'fspec research' lists tools, it shows how to get help (e.g., 'perplexity --help'). AI agents learn tool usage from tool's own documentation/help output. Tools return results in their chosen format (JSON, text, markdown, etc.), and AI interprets the response. No mandated stdin/stdout contract.
  #   7. Auto-discover tools from spec/research-scripts/ directory. No registry file needed. fspec scans directory for executable scripts, lists them when running 'fspec research' without flags. Tool names derived from filenames (e.g., perplexity.sh → 'perplexity'). Discovery happens dynamically at runtime.
  #   8. Auto-discover research tools by scanning spec/research-scripts/ for ANY executable files (not just .sh - could be .py, .js, compiled binaries, etc.). Check executable bit, not file extension.
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec research' without flags, sees list of available tools: perplexity, jira, confluence, with descriptions and usage examples for each
  #   2. Developer runs 'fspec research --tool=perplexity --query="How does OAuth2 work?"', tool executes and returns formatted results, system asks 'Attach research results to work unit? (y/n)'
  #   3. During Example Mapping, developer runs 'fspec add-question AUTH-001 "@human: Should we support OAuth2?"', system prompts 'Would you like to research this question? (y/n)'
  #   4. Developer chooses to attach results, research output saved to spec/attachments/AUTH-001/perplexity-oauth2-research-2025-10-24.md and referenced in work unit
  #   5. Research directory contains perplexity.py (Python), jira (compiled binary), confluence.js (Node). fspec auto-discovers all three by executable bit, derives names: 'perplexity', 'jira', 'confluence'.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the research command be interactive (prompts user step-by-step) or CLI-based with flags (e.g., fspec research --tool=perplexity --query='question')?
  #   A: true
  #
  #   Q: How should research scripts be stored? Should we use a similar structure to hooks (e.g., spec/research-scripts/)?
  #   A: true
  #
  #   Q: Should research results be automatically attached to work units, or should the user manually decide what to attach?
  #   A: true
  #
  #   Q: Should fspec prompt to research during Example Mapping automatically, or should it be manually invoked with 'fspec research'?
  #   A: true
  #
  #   Q: What should the research script interface look like? Should scripts receive JSON context via stdin (like hooks) and return JSON results via stdout?
  #   A: true
  #
  #   Q: Should there be a registry of available research tools (similar to tags.json), or should tools be auto-discovered from the research-scripts directory?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec during Example Mapping
    I want to research questions using external tools before defining acceptance criteria
    So that I make informed decisions based on external knowledge sources

  Scenario: List available research tools without executing any
    Given I have research scripts in spec/research-scripts/ directory
    And the directory contains "perplexity.sh", "jira.py", and "confluence" (compiled binary)
    When I run "fspec research" without any flags
    Then I should see a list of available research tools
    And each tool should display its name derived from filename
    And each tool should show usage example with --tool flag
    And each tool should show how to get help for that tool
    And no research should be executed

  Scenario: Execute research tool and prompt for attachment
    Given I have a research tool "perplexity.sh" in spec/research-scripts/
    When I run "fspec research --tool=perplexity --query='How does OAuth2 work?'"
    Then the perplexity tool should execute with the query
    And I should see the formatted research results
    And I should be prompted "Attach research results to work unit? (y/n)"

  Scenario: Auto-discover research tools by executable bit
    Given I have research scripts with different extensions
    And spec/research-scripts/ contains "perplexity.py" (Python script)
    And spec/research-scripts/ contains "jira" (compiled binary)
    And spec/research-scripts/ contains "confluence.js" (Node script)
    And all three files have executable bit set
    When I run "fspec research" without flags
    Then all three tools should be auto-discovered
    And tool names should be "perplexity", "jira", "confluence"
    And discovery should happen dynamically at runtime

  Scenario: Attach research results to work unit
    Given I have executed "fspec research --tool=perplexity --query='OAuth2'"
    And the research completed successfully with results
    When I choose "yes" to attach results to work unit AUTH-001
    Then research output should be saved to "spec/attachments/AUTH-001/perplexity-oauth2-research-YYYY-MM-DD.md"
    And the attachment should be referenced in work unit AUTH-001 metadata
    And I should be able to view attachments with "fspec list-attachments AUTH-001"

  Scenario: Prompt to research during Example Mapping
    Given I am adding a question during Example Mapping
    When I run "fspec add-question AUTH-001 '@human: Should we support OAuth2?'"
    Then the question should be added successfully
    And I should see a prompt "Would you like to research this question? (y/n)"
    And if I choose "yes", I should see available research tools
    And I should be guided to run "fspec research --tool=<name> --query='question'"
