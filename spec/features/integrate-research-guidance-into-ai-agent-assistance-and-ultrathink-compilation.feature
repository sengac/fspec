@validation
@high
@research
@cli
@discovery
@RES-008
Feature: Integrate research guidance into AI agent assistance and ULTRATHINK compilation
  """
  Integrates research guidance into multiple touchpoints: system-reminders (src/commands/update-work-unit-status.ts), bootstrap content (src/commands/bootstrap.ts), spec/CLAUDE.md template. Adds compile-research command using agentRegistry.ts for ULTRATHINK detection, markdown-it for rendering, mermaid validation from existing add-diagram command. Auto-attaches compiled research using add-attachment flow.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System-reminders during SPECIFYING phase must mention available research tools (fspec research)
  #   2. Bootstrap content (fspec bootstrap) must include RESEARCH section documenting when and how to use research tools
  #   3. spec/CLAUDE.md must document research workflow with concrete examples
  #   4. Research compilation must use agent detection (agentRegistry.ts) to determine ULTRATHINK support
  #   5. Compiled research must be markdown format with front matter metadata (work unit ID, timestamp, tool used)
  #   6. Compiled research must generate mermaid diagrams for visual concepts (flowcharts, architecture, relationships)
  #   7. Compiled research must auto-attach to work unit using add-attachment command
  #
  # EXAMPLES:
  #   1. Agent moves AUTH-001 to specifying → system-reminder shows: 'Use fspec research --tool=ast or --tool=stakeholder to answer questions'
  #   2. Agent runs 'fspec bootstrap' → output includes RESEARCH section with tool examples and workflow
  #   3. spec/CLAUDE.md contains: 'During specifying, use fspec research --tool=ast --query="pattern" to analyze code'
  #   4. Agent runs 'fspec compile-research AUTH-001' → creates markdown with ULTRATHINK analysis (Claude) or deep analysis (other agents)
  #   5. Compiled research includes: mermaid flowchart showing authentication flow, markdown summary, auto-attached to spec/attachments/AUTH-001/
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec during discovery
    I want to receive guidance on using research tools and compile findings
    So that I can answer questions with high-quality research that includes diagrams and gets attached to work units

  Scenario: System-reminder mentions research tools during specifying phase
    Given I have a work unit AUTH-001 in backlog
    When I run "fspec update-work-unit-status AUTH-001 specifying"
    Then the system-reminder output should contain "fspec research"
    And the output should mention "--tool=ast or --tool=stakeholder"
    And the guidance should explain when to use research tools

  Scenario: Bootstrap content includes research section
    Given I am in a project directory
    When I run "fspec bootstrap"
    Then the output should include a RESEARCH section
    And the section should document available research tools
    And the section should show research workflow examples
    And the section should explain tool integration with Example Mapping

  Scenario: CLAUDE.md contains research workflow documentation
    Given I run "fspec init --agent=claude"
    When I read the file "spec/CLAUDE.md"
    Then it should contain research tool documentation
    And it should include example: "fspec research --tool=ast --query=\"pattern\""
    And it should explain when to use research during specifying phase
    And it should show how to attach research results to work units

  Scenario: Compile research with ULTRATHINK for Claude agent
    Given I am using Claude agent
    And I have work unit AUTH-001 with research data
    When I run "fspec compile-research AUTH-001"
    Then it should use agent detection to identify Claude
    And the compiled markdown should include "ULTRATHINK" terminology
    And the markdown should have front matter with work unit ID and timestamp
    And the file should be auto-attached to spec/attachments/AUTH-001/

  Scenario: Compile research without ULTRATHINK for other agents
    Given I am using Cursor agent
    And I have work unit AUTH-001 with research data
    When I run "fspec compile-research AUTH-001"
    Then it should use agent detection to identify Cursor
    And the compiled markdown should use "deep analysis" terminology
    And the markdown should have front matter with work unit ID and timestamp
    And the file should be auto-attached to spec/attachments/AUTH-001/

  Scenario: Compiled research includes mermaid diagrams
    Given I have work unit AUTH-001 with authentication flow research
    When I run "fspec compile-research AUTH-001"
    Then the compiled markdown should include a mermaid flowchart
    And the diagram should visualize the authentication flow
    And the markdown should include summary sections
    And the mermaid syntax should be validated before inclusion
