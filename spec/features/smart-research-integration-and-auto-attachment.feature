@done
@high
@cli
@research-tools
@ai-assistance
@attachment-management
@RES-013
Feature: Smart Research Integration and Auto-Attachment
  """
  Integrates with research tool system (RES-010) to capture research output. Uses AI (Claude/GPT) to analyze research results and extract business rules, examples, and questions. Provides interactive prompts for user acceptance/rejection/editing of AI suggestions. Stores attachments in spec/attachments/<work-unit-id>/ with both raw and structured formats. Supports --auto-attach flag for non-interactive mode. Updates Example Map (work-units.json) with extracted artifacts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After running a research tool, system must prompt user to save results as attachment to current work unit
  #   2. AI assistant must analyze research output and suggest extractable business rules, examples, and questions
  #   3. User must be able to accept/reject/edit AI-suggested rules and examples before adding to Example Map
  #   4. Saved attachments must be stored in spec/attachments/<work-unit-id>/ directory
  #   5. Research output must be saved as both raw format and AI-extracted structured format
  #   6. System must support --auto-attach flag to skip prompt and auto-save research results
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=perplexity --query="OAuth best practices" --work-unit=AUTH-001' and after receiving results, system prompts: 'Save research results as attachment? (y/n)'
  #   2. User saves research results, AI analyzes output and suggests: 'Found 3 rules, 5 examples. Add to AUTH-001? (y/n/edit)'
  #   3. User accepts AI suggestions, system adds rules/examples to AUTH-001 Example Map and saves raw output to spec/attachments/AUTH-001/perplexity-oauth-research.md
  #   4. User runs 'fspec research --tool=ast --query="authentication patterns" --work-unit=AUTH-001 --auto-attach' and results are automatically saved without prompts
  #   5. AI extracts rule 'JWT tokens must expire within 24 hours' from research output and adds it to Example Map after user confirmation
  #
  # ========================================
  Background: User Story
    As a developer using research tools during Example Mapping
    I want to automatically save research results as attachments and extract rules/examples with AI assistance
    So that I can quickly capture research findings and convert them into actionable specification artifacts

  Scenario: Prompt to save research results as attachment after research tool execution
    Given I have a work unit AUTH-001 in specifying state
    When I run "fspec research --tool=perplexity --query=\"OAuth best practices\" --work-unit=AUTH-001"
    Then the system should prompt "Save research results as attachment? (y/n)"
    And the research tool returns results
    And the prompt should wait for user input

  Scenario: AI analyzes research output and suggests extractable rules and examples
    Given I have saved research results for AUTH-001
    When the AI analyzes the research output
    Then the system should display "Found 3 rules, 5 examples"
    And the system should prompt "Add to AUTH-001? (y/n/edit)"

  Scenario: Accept AI suggestions and add to Example Map with attachment saved
    Given AI has suggested 3 rules and 5 examples for AUTH-001
    When I accept the AI suggestions
    Then the rules and examples should be added to AUTH-001 Example Map
    And the raw output should be saved to spec/attachments/AUTH-001/perplexity-oauth-research.md
    And the structured extraction should be saved to spec/attachments/AUTH-001/perplexity-oauth-research-extracted.json

  Scenario: Auto-attach research results without prompts using --auto-attach flag
    Given I have a work unit AUTH-001 in specifying state
    When I run "fspec research --tool=ast --query=\"authentication patterns\" --work-unit=AUTH-001 --auto-attach"
    Then the research results should be automatically saved without prompts
    And the attachment should be saved to spec/attachments/AUTH-001/

  Scenario: AI extracts specific business rule from research output
    Given I have research output about JWT token expiration
    When the AI analyzes the content
    Then the AI should extract rule "JWT tokens must expire within 24 hours"
    And the extracted rule should be presented for user confirmation
    And after confirmation the rule should be added to the Example Map
